//! A window per document, in one process (ADR 0001): showing the documents a launch names, what
//! every new window starts with, and closing a window. Which window shows which document, and
//! carrying out a show, are the documents module's (`documents.rs`); Tauri's `AppHandle` is its
//! [`Windows`] here.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, Manager as _, Monitor, State, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::documents::{
    Area, DocumentAt, Documents, NewWindow, Point, ShowError, Target, WindowError, WindowPlace,
    WindowSize, Windows,
};
use crate::settings::{PageSettings, Theme, WindowLook};
use crate::timing::Timing;
use crate::{command_line, platform, title_bar};

/// What a launch couldn't show. Its message names the document, so the reader can tell which of
/// a launch's documents failed.
#[derive(Debug)]
pub enum LaunchFailure {
    /// A document the launch names isn't shown.
    Document { document: PathBuf, error: ShowError },
    /// The window of a launch that names no document didn't open.
    EmptyWindow(WindowError),
}

impl fmt::Display for LaunchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document { document, error } => {
                write!(f, "Can't show {}: {error}", document.display())
            }
            Self::EmptyWindow(error) => write!(f, "Can't open a window: {error}"),
        }
    }
}

/// What the reader is told about a launch's failures, one per line; None when every window
/// opened.
pub fn failure_report(failures: &[LaunchFailure]) -> Option<String> {
    (!failures.is_empty()).then(|| {
        failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Shows the documents a launch names (`downlight <file>...`), the first launch's or one handed
/// over by a second launch, which was started in `launching_folder`.
///
/// Each document gets a window of its own: a new window, or the one that already shows it,
/// brought to the front. With no documents, one window opens that shows none. A window that
/// fails doesn't keep the launch's other documents from opening; the failures come back for the
/// caller to show (ticket 19), empty when every window opened.
pub fn open_launch(
    documents: &Documents,
    args: impl IntoIterator<Item = impl Into<OsString>>,
    launching_folder: &Path,
) -> Vec<LaunchFailure> {
    let paths = command_line::document_paths(args, launching_folder);
    if paths.is_empty() {
        return documents
            .open_empty()
            .err()
            .map(LaunchFailure::EmptyWindow)
            .into_iter()
            .collect();
    }
    paths
        .into_iter()
        .filter_map(|document| {
            documents
                .give(DocumentAt::new(&document), Target::NewWindow)
                .err()
                .map(|error| LaunchFailure::Document { document, error })
        })
        .collect()
}

/// Keeps track of the windows for the documents module: which one the reader is in, for a new
/// window to open beside (ticket 16); the size of one that is closing, read while it is still
/// there to measure, for a new window to open at; and forgets a closed window's document, so
/// opening the document again opens a new window.
pub fn track_windows(window: &tauri::Window, event: &WindowEvent) {
    let documents = window.state::<Documents>();
    match event {
        WindowEvent::Focused(true) => documents.activated(window.label()),
        // Saved here, on the main thread, rather than on a background task, since closing the
        // last window ends the process right after: an exception to keeping the main thread free
        // (ticket 16's Results), one small file written as a window closes.
        WindowEvent::CloseRequested { .. } => documents.closing(window.label()),
        WindowEvent::Destroyed => documents.forget(window.label()),
        _ => {}
    }
}

/// The app's [`Windows`]: Tauri's windows.
impl Windows for AppHandle {
    fn open(&self, window: &str, new_window: &NewWindow) -> Result<(), WindowError> {
        Ok(open_window(self, window, new_window)?)
    }

    fn set_title(&self, window: &str, title: &str) -> Result<(), WindowError> {
        // The window may have closed since it was given its document, and its title is then
        // nobody's to see.
        let Some(window) = self.get_webview_window(window) else {
            return Ok(());
        };
        // Off the main thread, tauri-runtime-wry 2.12 posts the title to the event loop without
        // waiting, in the order titles are set, so the documents module's titles lock, held while
        // this runs, holds up no other thread for long and keeps titles in its order.
        Ok(window.set_title(title)?)
    }

    fn bring_to_front(&self, window: &str) -> Result<(), WindowError> {
        // A window that is still being built has no handle yet. One that is built but still
        // hidden ignores set_focus (tao does nothing for a window that isn't visible). Either way
        // it comes up in front when its page asks to be shown (`show_window`).
        let Some(window) = self.get_webview_window(window) else {
            return Ok(());
        };
        if window.is_minimized()? {
            window.unminimize()?;
        }
        Ok(window.set_focus()?)
    }

    fn place_of(&self, window: &str) -> Option<WindowPlace> {
        let window = self.get_webview_window(window)?;
        if window.is_minimized().ok()? || window.is_maximized().ok()? {
            return None;
        }
        // In the logical pixels of the window's own monitor, so that a new window opened beside
        // it lands on that monitor, at that monitor's scale (tao picks the monitor a logical
        // position lands on at its own scale).
        let scale = window.scale_factor().ok()?;
        let at = window.outer_position().ok()?.to_logical::<f64>(scale);
        let size = window.inner_size().ok()?.to_logical::<f64>(scale);
        let monitor = window.current_monitor().ok()??;
        Some(WindowPlace {
            at: Point { x: at.x, y: at.y },
            size: WindowSize {
                width: size.width.round() as u32,
                height: size.height.round() as u32,
            },
            work_area: work_area_of(&monitor, scale),
        })
    }

    fn primary_work_area(&self) -> Option<Area> {
        // At the main monitor's own scale, which a window the system places there opens at.
        let monitor = self.primary_monitor().ok()??;
        Some(work_area_of(&monitor, monitor.scale_factor()))
    }

    fn set_look(&self, window: &str, look: &WindowLook) -> Result<(), WindowError> {
        // Not built yet, or closed: the documents module gives a window still being built its
        // look when its page first asks.
        let Some(window) = self.get_webview_window(window) else {
            return Ok(());
        };
        // Both setters post to the event loop off the main thread and wait for nothing. tao
        // sets the window's dark mode, which its backdrop's tint follows, and tauri-runtime-wry
        // hands the webview the theme on tao's `ThemeChanged` as its preferred color scheme,
        // which belongs to the WebView2 profile all windows share (window design ticket 13).
        window.set_theme(tauri_theme(look.theme))?;
        Ok(platform::set_backdrop(&window, look.backdrop)?)
    }
}

/// The reader's `theme` as Tauri takes it: None follows Windows.
fn tauri_theme(theme: Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    }
}

/// `monitor`'s work area, in logical pixels at `scale`.
fn work_area_of(monitor: &Monitor, scale: f64) -> Area {
    let work_area = monitor.work_area();
    let corner = work_area.position.to_logical::<f64>(scale);
    let extent = work_area.size.to_logical::<f64>(scale);
    Area {
        left: corner.x,
        top: corner.y,
        width: extent.width,
        height: extent.height,
    }
}

/// What the OS and the process tell a window's page in its URL, decided once when the window
/// opens: the page's `PageFlags` in `page/src/app.ts`. The reader's choices are no flags: they
/// come with the settings.
#[derive(Debug, Clone, Copy)]
struct PageFlags {
    /// The timing switch is on, so the page sends its timing reports.
    timing: bool,
    /// The system can show a backdrop (Mica on Windows 11), and the window is transparent over
    /// it, so the page draws no background of its own unless the reader's Backdrop, which comes
    /// with the settings, is None (window design ticket 08).
    backdrop_supported: bool,
    /// The window has no native frame, so the page draws the title bar and the window's buttons
    /// (ADR 0005).
    title_bar: bool,
}

/// The page and what it is told in its query: the `flags` (`timing`, `backdropSupported`,
/// `titleBar`), and `settings`, the first snapshot of the settings the page shows, as JSON
/// (window design ticket 12), so first paint waits for no round trip. The page reads them in one
/// place (`pageFlags` and `firstSettings` in `page/src/app.ts`), against this function's contract
/// sample.
fn page_url(flags: PageFlags, settings: PageSettings) -> tauri::Result<String> {
    // Only the query is kept, so any URL does as the base: Tauri resolves the page's path
    // against its own origin.
    let mut url = Url::parse("tauri://localhost/").map_err(tauri::Error::InvalidUrl)?;
    {
        let mut query = url.query_pairs_mut();
        let flags = [
            ("timing", flags.timing),
            ("backdropSupported", flags.backdrop_supported),
            ("titleBar", flags.title_bar),
        ];
        for (flag, on) in flags {
            if on {
                query.append_key_only(flag);
            }
        }
        // Form-encoded, as the page's `URLSearchParams` reads it.
        query.append_pair("settings", &serde_json::to_string(&settings)?);
    }
    Ok(format!("index.html?{}", url.query().unwrap_or_default()))
}

/// The page's URL with every flag on and with none, which the page's tests read its flags and
/// its first settings from (`app/src/contract.rs`), and with the default settings, which the page
/// checks its own defaults against (`DEFAULT_SETTINGS` in `page/src/app.ts`). A flag that is
/// renamed on one side only would otherwise pass both test suites and, for `titleBar`, open a
/// frameless window with no title bar to close it by.
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    use serde::ser::Error as _;

    // A struct literal names every field, so a new flag stops the build here until it is sampled.
    let every = |on| PageFlags {
        timing: on,
        backdrop_supported: on,
        title_bar: on,
    };
    let url = |flags, settings| page_url(flags, settings).map_err(serde_json::Error::custom);
    // The snapshot of the `settings` message's sample (`message::contract_samples`).
    let sample = PageSettings::sample();
    Ok(serde_json::json!({
        "allOn": url(every(true), sample)?,
        "allOff": url(every(false), sample)?,
        "defaults": url(every(false), PageSettings::default())?,
    }))
}

/// How long a window may stay hidden waiting for its page to ask to be shown. Cold first paint
/// is about 1 s (ticket 02), so this only fires when the page never asks, and keeps a window
/// that would otherwise stay invisible, and the process with it, from staying so.
const LONGEST_HIDDEN: Duration = Duration::from_secs(5);

/// Opens a window, hidden until its page has drawn its first frame (`show_window`), so it never
/// shows WebView2's white or an empty frame while the webview starts (ticket 04). The one place
/// that decides what every new window starts with: the page and its flags and first settings, its
/// size and place, theme and backdrop as `new_window` says, its frame, the navigation guard, the
/// webview's browser arguments, closing it when its webview stops, the key setup, the maximize
/// button and the 5 s fallback.
fn open_window(app: &AppHandle, label: &str, new_window: &NewWindow) -> tauri::Result<()> {
    let timing = app.state::<Timing>();
    timing.window_opening(label);
    // Decided once, so the page drops its background exactly when the window is transparent, and
    // draws the title bar exactly when the window has no frame.
    let flags = PageFlags {
        timing: timing.is_on(),
        backdrop_supported: platform::has_backdrop(),
        title_bar: platform::draws_own_title_bar(),
    };
    let url = WebviewUrl::App(page_url(flags, new_window.settings)?.into());
    let page = page_origin(app);
    let size = new_window.size;
    let builder = WebviewWindowBuilder::new(app, label, url)
        // Alt+Tab and the taskbar still show the title, though the page's title bar doesn't.
        .title(&new_window.title)
        .theme(tauri_theme(new_window.look.theme))
        .inner_size(f64::from(size.width), f64::from(size.height));
    // Without a place the system places the window, as for the first one.
    let builder = match new_window.at {
        Some(at) => builder.position(at.x, at.y),
        None => builder,
    };
    let builder = builder
        .decorations(!flags.title_bar)
        .visible(false)
        // Links never navigate the webview or open one of its own windows: the page follows them
        // through the app (ticket 11). These stop whatever still tries, such as a link the page
        // missed, from loading another site or a file in the window.
        .on_navigation(move |url| is_the_page(url, &page))
        .on_new_window(|_, _| NewWindowResponse::Deny);
    let builder = platform::with_backdrop(builder, new_window.look.backdrop);
    // A window whose webview fails to start never shows: Tauri drops the native window it made
    // for it, and only a built window is recorded with Tauri, so it keeps no process alive
    // (ticket 37).
    let window = platform::with_browser_arguments(builder).build()?;
    let stopped = window.clone();
    platform::when_webview_stops(&window, move |why| close_stopped(stopped, why));
    platform::leave_keys_to_the_page(&window);
    if flags.title_bar {
        title_bar::add_maximize_button(&window);
    }
    std::thread::spawn(move || {
        std::thread::sleep(LONGEST_HIDDEN);
        // The window may have closed meanwhile, and then there is nothing to show.
        if window.is_visible().is_ok_and(|visible| !visible) {
            let _ = window.show();
        }
    });
    Ok(())
}

/// Closes `window`, whose webview stopped (`why`), after telling the reader, so no empty window
/// with a title bar that does nothing is left behind (ticket 37). Closing the last window ends
/// the process (ADR 0001), so a later launch starts afresh rather than being handed to a process
/// whose webviews can't start.
fn close_stopped(window: WebviewWindow, why: &'static str) {
    // Off the main thread, where WebView2 tells of the failure, so the other windows keep working
    // while the box is up.
    std::thread::spawn(move || {
        let document = window.state::<Documents>().document_path(window.label());
        platform::show_error(&stopped_report(document.as_deref(), why));
        // Only once the box is dismissed: closing the last window ends the process, and the box
        // with it. Destroyed rather than closed: the reader didn't close it, so its size isn't kept
        // for the next window (`track_windows`).
        let _ = window.destroy();
    });
}

/// What the reader is told about a window that closed because its webview stopped (`why`),
/// naming the document it showed, if any.
fn stopped_report(document: Option<&Path>, why: &str) -> String {
    match document {
        Some(document) => format!("The window of {} closed: {why}", document.display()),
        None => format!("A window that showed no document closed: {why}"),
    }
}

/// The origin the app's page loads from, as Tauri serves `WebviewUrl::App`: the `devUrl` (Vite's
/// dev server) in development, and otherwise the app's own `tauri` scheme, which Tauri also falls
/// back to in development without a `devUrl`. Always one origin, so the page always loads.
fn page_origin(app: &AppHandle) -> String {
    let dev_url = app.config().build.dev_url.as_ref().filter(|_| cfg!(dev));
    match dev_url {
        Some(dev_url) => dev_url.origin().ascii_serialization(),
        None => platform::custom_scheme_origin("tauri"),
    }
}

/// Whether `url` is the app's page on `page_origin`, the only URL a window may load. Tauri loads
/// the page at the origin's root, or at `index.html` when the page's URL has a query, and Vite's
/// reloads in development load the same URL again. Any other URL on the origin, such as a link
/// dropped onto the window, would load the page anew and lose the reading position.
fn is_the_page(url: &Url, page_origin: &str) -> bool {
    url.origin().ascii_serialization() == page_origin && matches!(url.path(), "/" | "/index.html")
}

/// Ctrl+W.
#[tauri::command]
pub fn close_window(window: WebviewWindow) -> tauri::Result<()> {
    window.close()
}

/// The page asks for this once it has drawn its first frame, with its document, the empty state
/// or the error in it. Until then the window is hidden (`open_window`).
#[tauri::command]
pub fn show_window(window: WebviewWindow, timing: State<'_, Timing>) -> tauri::Result<()> {
    window.show()?;
    timing.window_shown(window.label());
    Ok(())
}

#[cfg(all(test, windows))]
mod tests;
