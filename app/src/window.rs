//! A window per document, in one process (ADR 0001): opening a window for a document or bringing
//! an open document's window to the front, and closing it. Which window shows which document is
//! kept by the documents module (`documents.rs`).

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, Manager as _, State, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::documents::{DocumentAt, Documents, Given, Target, file_name_shown};
use crate::timing::Timing;
use crate::{APP_NAME, command_line, platform, title_bar};

/// What [`show`] did with a document.
#[derive(Debug, PartialEq, Eq)]
pub enum Shown {
    /// It opened in a window: a new one, or the target window.
    Opened,
    /// It is open already, in the window the request came from ([`Given::AlreadyHere`]).
    AlreadyHere,
    /// It is open already, in another window, which was brought to the front. Its page has been
    /// told to scroll to the document's heading, if it has to ([`Given::AlreadyElsewhere`]).
    AlreadyElsewhere,
}

/// A window a launch couldn't open or bring to the front. Its message names the document, so the
/// reader can tell which of a launch's documents failed.
#[derive(Debug)]
pub struct LaunchFailure {
    /// None for the window of a launch that names no document.
    document: Option<PathBuf>,
    error: tauri::Error,
}

impl fmt::Display for LaunchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.document {
            Some(document) => write!(
                f,
                "Can't open a window for {}: {}",
                document.display(),
                self.error
            ),
            None => write!(f, "Can't open a window: {}", self.error),
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
    app: &AppHandle,
    args: impl IntoIterator<Item = impl Into<OsString>>,
    launching_folder: &Path,
) -> Vec<LaunchFailure> {
    let documents = command_line::document_paths(args, launching_folder);
    if documents.is_empty() {
        return open_empty(app)
            .err()
            .map(|error| LaunchFailure {
                document: None,
                error,
            })
            .into_iter()
            .collect();
    }
    documents
        .into_iter()
        .filter_map(|document| {
            show(app, DocumentAt::new(&document), Target::NewWindow)
                .err()
                .map(|error| LaunchFailure {
                    document: Some(document),
                    error,
                })
        })
        .collect()
}

/// Opens a new window that shows no document.
fn open_empty(app: &AppHandle) -> tauri::Result<()> {
    let documents = app.state::<Documents>();
    open_or_forget(app, &documents, &documents.open_empty(), &title_of(None))
}

/// Shows `document` in `target`, the one way a window gets or changes its document
/// ([`Documents::give`]). A document that is open already has its window brought to the front
/// instead, so a document is shown in at most one window. A new window opens titled after the
/// document; a window shown another document gets that document's title, and its page is told to
/// render it. Either page gets the heading with the render. A page that can't be told fails the
/// show, so the reader learns the document isn't shown.
pub fn show(app: &AppHandle, document: DocumentAt<'_>, target: Target<'_>) -> tauri::Result<Shown> {
    let documents = app.state::<Documents>();
    let title = title_of(Some(document.path));
    match documents.give(document, target)? {
        Given::NewWindow(window) => {
            open_or_forget(app, &documents, &window, &title)?;
            Ok(Shown::Opened)
        }
        Given::Changed(label) => {
            let window = app
                .get_webview_window(&label)
                .ok_or(tauri::Error::WindowNotFound)?;
            window.set_title(&title)?;
            Ok(Shown::Opened)
        }
        Given::AlreadyHere(window) => {
            // Dropping a file onto a window from the file manager leaves the file manager in
            // front, so even the window the request came from is brought there.
            bring_to_front(app, &window)?;
            Ok(Shown::AlreadyHere)
        }
        Given::AlreadyElsewhere(window) => {
            bring_to_front(app, &window)?;
            Ok(Shown::AlreadyElsewhere)
        }
        Given::WindowGone => Err(tauri::Error::WindowNotFound),
    }
}

/// Forgets a closed window's document, so opening the document again opens a new window.
pub fn forget_when_destroyed(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::Destroyed = event {
        window.state::<Documents>().forget(window.label());
    }
}

/// Opens the window `label` the documents module has just recorded, or forgets it if it can't
/// open, so that its document opens anew.
fn open_or_forget(
    app: &AppHandle,
    documents: &Documents,
    label: &str,
    title: &str,
) -> tauri::Result<()> {
    open_window(app, label, title).inspect_err(|_| documents.forget(label))
}

/// What a window leaves for its page to draw, decided once when it opens.
#[derive(Debug, Clone, Copy)]
struct WindowLook {
    /// The window is transparent over the system's backdrop (Mica on Windows), so the page draws
    /// no background of its own.
    backdrop: bool,
    /// The window has no native frame, so the page draws the title bar and the window's buttons
    /// (ADR 0005).
    title_bar: bool,
}

/// The page and what it is told in its query: `timing` turns on its timing reports, and the
/// window's look (`backdrop`, `titleBar`) says what the page draws. The page reads them in one
/// place (`pageFlags` in `page/src/app.ts`), against this function's contract sample.
fn page_url(timing: bool, look: WindowLook) -> String {
    let flags = [
        ("timing", timing),
        ("backdrop", look.backdrop),
        ("titleBar", look.title_bar),
    ]
    .into_iter()
    .filter_map(|(flag, on)| on.then_some(flag))
    .collect::<Vec<_>>();
    if flags.is_empty() {
        "index.html".to_owned()
    } else {
        format!("index.html?{}", flags.join("&"))
    }
}

/// The page's URL with every flag on and with none, which the page's tests read its flags from
/// (`app/src/contract.rs`). A flag that is renamed on one side only would otherwise pass both
/// test suites and, for `titleBar`, open a frameless window with no title bar to close it by.
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A struct literal names every field, so a new flag stops the build here until it is sampled.
    let every = |on| WindowLook {
        backdrop: on,
        title_bar: on,
    };
    Ok(serde_json::json!({
        "allOn": page_url(true, every(true)),
        "allOff": page_url(false, every(false)),
    }))
}

/// How long a window may stay hidden waiting for its page to ask to be shown. Cold first paint
/// is about 1 s (ticket 02), so this only fires when the page never asks, and keeps a window
/// that would otherwise stay invisible, and the process with it, from staying so.
const LONGEST_HIDDEN: Duration = Duration::from_secs(5);

/// Opens a window, hidden until its page has drawn its first frame (`show_window`), so it never
/// shows WebView2's white or an empty frame while the webview starts (ticket 04).
fn open_window(app: &AppHandle, label: &str, title: &str) -> tauri::Result<()> {
    let timing = app.state::<Timing>();
    timing.window_opening(label);
    // Decided once, so the page drops its background exactly when the window is transparent, and
    // draws the title bar exactly when the window has no frame.
    let look = WindowLook {
        backdrop: platform::has_backdrop(),
        title_bar: platform::draws_own_title_bar(),
    };
    let url = WebviewUrl::App(page_url(timing.is_on(), look).into());
    let page = page_origin(app);
    let builder = WebviewWindowBuilder::new(app, label, url)
        // Alt+Tab and the taskbar still show the title, though the page's title bar doesn't.
        .title(title)
        .decorations(!look.title_bar)
        .visible(false)
        // Links never navigate the webview or open one of its own windows: the page follows them
        // through the app (ticket 11). These stop whatever still tries, such as a link the page
        // missed, from loading another site or a file in the window.
        .on_navigation(move |url| is_the_page(url, &page))
        .on_new_window(|_, _| NewWindowResponse::Deny);
    let builder = if look.backdrop {
        platform::with_backdrop(builder)
    } else {
        builder
    };
    let window = builder.build()?;
    platform::leave_reload_and_find_keys_to_the_page(&window);
    if look.title_bar {
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

fn bring_to_front(app: &AppHandle, label: &str) -> tauri::Result<()> {
    // A window that is still being built has no handle yet. One that is built but still hidden
    // ignores set_focus (tao does nothing for a window that isn't visible). Either way it comes up
    // in front when its page asks to be shown (`show_window`).
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };
    if window.is_minimized()? {
        window.unminimize()?;
    }
    window.set_focus()
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

/// `README.md - docs - Downlight`: the folder tells documents with the same name apart in Alt+Tab
/// and on the taskbar. A window that shows no document (None) is titled `Downlight`.
fn title_of(document: Option<&Path>) -> String {
    let Some(document) = document else {
        return APP_NAME.to_owned();
    };
    // The same name as the title bar's.
    let file = file_name_shown(document);
    // Launches, the open dialog and dropping all give absolute paths, so only a drive root has no
    // folder.
    match document.parent() {
        Some(folder) => format!("{file} - {} - {APP_NAME}", file_name_shown(folder)),
        None => format!("{file} - {APP_NAME}"),
    }
}

#[cfg(all(test, windows))]
mod tests;
