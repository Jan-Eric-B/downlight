//! A window per document, in one process (ADR 0001): which window shows which document, opening
//! a window or bringing an open document's window to the front, and closing it. Reload
//! (`reload.rs`) keeps a window's watch and its document's next render in the table kept here.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use std::{fmt, io};

use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, Emitter as _, Manager as _, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::render::{self, RenderTask};
use crate::timing::Timing;
use crate::watch::Watch;
use crate::{APP_NAME, command_line, platform, reload};

/// The event that tells a window's page its window shows another document now.
const DOCUMENT_SHOWN: &str = "document-shown";

/// Which window shows which document, by window label. The app's managed state.
#[derive(Default)]
pub struct OpenWindows {
    table: Mutex<WindowTable>,
}

/// Every open window by label, whether it shows a document or none.
#[derive(Default)]
struct WindowTable {
    /// Labels are `document-0`, `document-1` and so on, never reused while the process runs.
    labels_issued: usize,
    /// Every open window, with the document it shows now; None while it shows none.
    shown: HashMap<String, Option<ShownDocument>>,
}

struct ShownDocument {
    /// As the launch, the dialog or the drop named it, for reading and the title.
    path: PathBuf,
    /// For finding the window when the document is opened again.
    key: PathBuf,
    /// The document's read and render, started when the window was given it and taken by its
    /// page's first request. A new window's page asks only once its webview has started, about
    /// 600 ms into a cold start, and each grammar costs up to 300 ms on its first use in the
    /// process (ticket 07), so the render runs while the webview starts. A reload starts a new
    /// one (`OpenWindows::reload_ahead`).
    render_ahead: Option<RenderTask>,
    /// The watch on the document's file, which reloads it; None until it has started (`show`), or
    /// if it couldn't. Replacing or forgetting the entry drops it, which stops it.
    watch: Option<Watch>,
    /// Whether the page has asked for this document yet (`OpenWindows::render_of`), and so
    /// listens for notices.
    page_asked: bool,
    /// A notice about this document, held for the page's first request (`hold_notice`).
    held_notice: Option<String>,
    /// The heading the window shows the document at, from a link like `guide.md#install` (ticket
    /// 11). Handed over with every render rather than taken by the first, so that of two links
    /// followed in quick succession, the page's latest request still gets it; the page uses it
    /// only when it shows the document, never on a reload.
    heading: Option<String>,
}

impl ShownDocument {
    fn given(document: DocumentAt<'_>, key: PathBuf) -> Self {
        Self {
            path: document.path.to_owned(),
            key,
            render_ahead: Some(render::start(document.path, "read-and-render")),
            watch: None,
            page_asked: false,
            held_notice: None,
            heading: document.heading.map(str::to_owned),
        }
    }

    fn start_reload(&mut self) {
        self.render_ahead = Some(render::start(&self.path, "reload"));
    }
}

/// What a window's page gets when it asks for its document.
pub struct Handover {
    /// The document's read and render.
    pub render: RenderTask,
    /// A notice about the document for the page to show, held since before the page asked.
    pub notice: Option<String>,
    /// The heading to show the document at, or None for its top.
    pub heading: Option<String>,
}

/// A document to show, and where in it.
#[derive(Debug, Clone, Copy)]
pub struct DocumentAt<'a> {
    /// As the launch, the dialog, the drop or the link named it.
    pub path: &'a Path,
    /// The heading to show it at, from a link like `guide.md#install` (ticket 11), or None.
    pub heading: Option<&'a str>,
}

impl<'a> DocumentAt<'a> {
    /// The document at `path`, without a heading: a window given it shows its top, and a window
    /// that shows it already keeps its reading position.
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            heading: None,
        }
    }
}

/// The window a document is to be shown in.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A new window, as for a launch from the file manager or the command line.
    NewWindow,
    /// The window with this label, as for Ctrl+O and dropping a file.
    Window(&'a str),
}

/// What [`show`] did with a document.
#[derive(Debug, PartialEq, Eq)]
pub enum Shown {
    /// A window was given it: a new one, or the target window.
    Given,
    /// It is open already, in `window`, which was brought to the front. That may be the target
    /// window itself. `scroll_to` is the heading for that window's page to scroll to now.
    AlreadyOpen {
        window: String,
        scroll_to: Option<String>,
    },
}

/// What showing a document changed, by window label. The caller acts on it: it opens a new
/// window, or titles a changed one and tells its page.
///
/// A window given a document comes with the document's key (None for no document), and its watch
/// is kept under that key. A key worked out again later could differ, had the file been renamed,
/// deleted or named by a short name in between, and the window would then drop its watch.
#[derive(Debug, PartialEq, Eq)]
enum Claim {
    /// A new window shows the document; the caller opens it.
    New {
        window: String,
        key: Option<PathBuf>,
    },
    /// The window shows the document now.
    Changed {
        window: String,
        key: Option<PathBuf>,
    },
    /// The document is already open in `window`. `scroll_to` is the heading its page is to scroll
    /// to now, if the document came with one and the page listens.
    AlreadyOpen {
        window: String,
        scroll_to: Option<String>,
    },
    /// The target window has closed.
    WindowGone,
}

impl OpenWindows {
    /// Records that `target` shows `document` (None: no document), unless the document is open
    /// already. Looking up and recording happen under one lock, so two requests for the same
    /// document end up in one window.
    ///
    /// The window of an open document is to scroll to the document's heading, if it has one. A
    /// page that hasn't asked for its document yet may not be listening (a new window's webview
    /// takes about 600 ms to start), so it gets the heading with its first render instead, the
    /// way `hold_notice` holds a notice (ticket 10).
    fn claim(&self, document: Option<DocumentAt<'_>>, target: Target<'_>) -> Claim {
        let key = document.map(|document| document_key(document.path));
        let mut table = self.lock();
        if let Some(key) = &key
            && let Some(window) = table.window_showing(key)
        {
            let mut scroll_to = document
                .and_then(|document| document.heading)
                .map(str::to_owned);
            if scroll_to.is_some()
                && let Some(shown) = table.shown_mut(&window, key)
                && !shown.page_asked
            {
                shown.heading = scroll_to.take();
            }
            return Claim::AlreadyOpen { window, scroll_to };
        }
        // Made only once a window takes the document, so that an open document or a closed
        // window starts no render.
        let shown = |key: &Option<PathBuf>| {
            document
                .zip(key.clone())
                .map(|(document, key)| ShownDocument::given(document, key))
        };
        match target {
            Target::NewWindow => {
                let window = table.next_label();
                table.shown.insert(window.clone(), shown(&key));
                Claim::New { window, key }
            }
            Target::Window(window) => match table.shown.get_mut(window) {
                Some(entry) => {
                    *entry = shown(&key);
                    Claim::Changed {
                        window: window.to_owned(),
                        key,
                    }
                }
                None => Claim::WindowGone,
            },
        }
    }

    /// `window`'s document read and rendered, or None while the window shows none: the render
    /// started when the window was given the document, the first time, and a new read of its
    /// file after that.
    pub fn render_of(&self, window: &str) -> Option<Handover> {
        let (path, ahead, notice, heading) = {
            let mut table = self.lock();
            let shown = table.shown.get_mut(window)?.as_mut()?;
            shown.page_asked = true;
            (
                shown.path.clone(),
                shown.render_ahead.take(),
                shown.held_notice.take(),
                shown.heading.clone(),
            )
        };
        Some(Handover {
            render: ahead.unwrap_or_else(|| render::start(&path, "read-and-render")),
            notice,
            heading,
        })
    }

    /// The file of the document `window` shows, as the launch, the dialog, the drop or the link
    /// named it, or None while it shows none. The one way to learn a window's document: commands
    /// take it from here, never from the page.
    pub fn document_path(&self, window: &str) -> Option<PathBuf> {
        self.lock()
            .shown
            .get(window)?
            .as_ref()
            .map(|shown| shown.path.clone())
    }

    /// Holds `message`, a notice about the document with `key`, for `window`'s page until the page
    /// first asks for that document. Once it has asked, the message comes back, to be shown now. A
    /// message for a document the window has left, or for a closed window, is dropped.
    ///
    /// A new window's page listens for nothing until its webview has started, about 600 ms after
    /// the window was given its document, so a notice sent before then would be lost. The page
    /// listens for notices before it first asks for its document, so one handed over with that
    /// request reaches it (ticket 10). Held on the document's entry, a notice goes when the
    /// window is given another document.
    pub fn hold_notice(&self, window: &str, key: &Path, message: String) -> Option<String> {
        let mut table = self.lock();
        let shown = table.shown_mut(window, key)?;
        if shown.page_asked {
            return Some(message);
        }
        shown.held_notice = Some(message);
        None
    }

    /// Starts reading and rendering `window`'s document again, for the page's next request (a
    /// reload by hand). False while the window shows no document, and then there is nothing to
    /// reload.
    pub fn reload_ahead(&self, window: &str) -> bool {
        let mut table = self.lock();
        table
            .shown
            .get_mut(window)
            .and_then(Option::as_mut)
            .map(ShownDocument::start_reload)
            .is_some()
    }

    /// Starts reading and rendering the document with `key` again, for `window`'s page's next
    /// request (a reload on a change to its file), while the window still shows it. False if the
    /// window has left it or closed, and then nothing is reloaded. The check and the render start
    /// under one lock, so a document the window is given in between keeps its render ahead
    /// (ticket 21).
    pub fn reload_ahead_while_shown(&self, window: &str, key: &Path) -> bool {
        self.lock()
            .shown_mut(window, key)
            .map(ShownDocument::start_reload)
            .is_some()
    }

    /// Whether `window` still shows the document with `key`. A watch can report a change after
    /// its window has left the document or closed, and a document the window has left is never
    /// reloaded.
    pub fn shows(&self, window: &str, key: &Path) -> bool {
        self.lock().shown_mut(window, key).is_some()
    }

    /// Gives `window` the watch started for the document with `key`. A watch that started after
    /// the window was given another document, or closed, is dropped instead, so a save to a
    /// document the window has left never reloads it.
    pub fn keep_watch(&self, window: &str, key: &Path, watch: Watch) {
        if let Some(shown) = self.lock().shown_mut(window, key) {
            shown.watch = Some(watch);
        }
    }

    /// Forgets a window that closed or never opened, so its document opens anew. A render started
    /// for it, and the watch on its document, go with it.
    fn forget(&self, window: &str) {
        self.lock().shown.remove(window);
    }

    fn lock(&self) -> MutexGuard<'_, WindowTable> {
        // The map stays valid even if a thread panicked while holding the lock.
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What two paths to the same document have in common: the path with links, `..` and short names
/// resolved, in the case the file system compares names in. A missing file keeps its name as
/// given, in its resolved folder; with its folder missing too, the path stays as given, absolute
/// already, as launches, the open dialog and dropping all give it.
fn document_key(document: &Path) -> PathBuf {
    let resolved = std::fs::canonicalize(document)
        .or_else(|_| {
            let (Some(folder), Some(name)) = (document.parent(), document.file_name()) else {
                return Err(io::ErrorKind::NotFound.into());
            };
            std::fs::canonicalize(folder).map(|folder| folder.join(name))
        })
        .unwrap_or_else(|_: io::Error| document.to_owned());
    platform::fold_case(resolved)
}

impl WindowTable {
    /// What `window` shows, while it shows the document with `key`.
    fn shown_mut(&mut self, window: &str, key: &Path) -> Option<&mut ShownDocument> {
        self.shown
            .get_mut(window)
            .and_then(Option::as_mut)
            .filter(|shown| shown.key == key)
    }

    fn window_showing(&self, key: &Path) -> Option<String> {
        self.shown.iter().find_map(|(label, shown)| {
            shown
                .as_ref()
                .is_some_and(|shown| shown.key == key)
                .then(|| label.clone())
        })
    }

    fn next_label(&mut self) -> String {
        let label = format!("document-{}", self.labels_issued);
        self.labels_issued += 1;
        label
    }
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
    let documents = if documents.is_empty() {
        vec![None]
    } else {
        documents.into_iter().map(Some).collect()
    };
    documents
        .into_iter()
        .filter_map(|document| {
            show(
                app,
                document.as_deref().map(DocumentAt::new),
                Target::NewWindow,
            )
            .err()
            .map(|error| LaunchFailure { document, error })
        })
        .collect()
}

/// Shows `document` (None: no document) in `target`, the one way a window gets or changes its
/// document. A document that is open already has its window brought to the
/// front instead, so a document is shown in at most one window. A new window opens titled after
/// the document; a window shown another document gets that document's title and a
/// `document-shown` event, on which its page renders the document. Either page gets the heading
/// with the render (`Handover`).
///
/// The document's render starts as soon as the window is given it, so that it runs while a new
/// window's webview starts (`ShownDocument::render_ahead`), and the window starts watching the
/// document's file, to reload it when it changes.
pub fn show(
    app: &AppHandle,
    document: Option<DocumentAt<'_>>,
    target: Target<'_>,
) -> tauri::Result<Shown> {
    let open_windows = app.state::<OpenWindows>();
    let path = document.map(|document| document.path);
    match open_windows.claim(document, target) {
        Claim::New { window, key } => {
            open_window(app, &window, &title_of(path)).inspect_err(|_| {
                open_windows.forget(&window);
            })?;
            watch(app, &window, path, key);
            Ok(Shown::Given)
        }
        Claim::Changed { window: label, key } => {
            watch(app, &label, path, key);
            let window = app
                .get_webview_window(&label)
                .ok_or(tauri::Error::WindowNotFound)?;
            window.set_title(&title_of(path))?;
            app.emit_to(label.as_str(), DOCUMENT_SHOWN, ())?;
            Ok(Shown::Given)
        }
        Claim::AlreadyOpen { window, scroll_to } => {
            bring_to_front(app, &window)?;
            Ok(Shown::AlreadyOpen { window, scroll_to })
        }
        Claim::WindowGone => Err(tauri::Error::WindowNotFound),
    }
}

/// Starts watching the file of the document `window` was given with `key`, to reload it when it
/// changes. A window given no document has nothing to watch.
fn watch(app: &AppHandle, window: &str, document: Option<&Path>, key: Option<PathBuf>) {
    if let Some((document, key)) = document.zip(key) {
        reload::watch_document(app, window, document, key);
    }
}

/// Forgets a closed window's document, so opening the document again opens a new window.
pub fn forget_when_destroyed(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::Destroyed = event {
        window.state::<OpenWindows>().forget(window.label());
    }
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
    // Decided once, so the page drops its background exactly when the window is transparent.
    let backdrop = platform::has_backdrop();
    let url = WebviewUrl::App(timing.page_url(backdrop).into());
    let page = page_origin(app);
    let builder = WebviewWindowBuilder::new(app, label, url)
        .title(title)
        .visible(false)
        // Links never navigate the webview or open one of its own windows: the page follows them
        // through the app (ticket 11). These stop whatever still tries, such as a link the page
        // missed, from loading another site or a file in the window.
        .on_navigation(move |url| url.origin().ascii_serialization() == page)
        .on_new_window(|_, _| NewWindowResponse::Deny);
    let builder = if backdrop {
        platform::with_backdrop(builder)
    } else {
        builder
    };
    let window = builder.build()?;
    platform::leave_reload_and_find_keys_to_the_page(&window);
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
    let file = document
        .file_name()
        .map_or_else(|| document.display(), |name| Path::new(name).display());
    // Launches, the open dialog and dropping all give absolute paths, so only a drive root has no
    // folder.
    match document.parent() {
        // A drive root has no name of its own, so it shows as itself.
        Some(folder) => {
            let folder = folder
                .file_name()
                .map_or_else(|| folder.display(), |name| Path::new(name).display());
            format!("{file} - {folder} - {APP_NAME}")
        }
        None => format!("{file} - {APP_NAME}"),
    }
}

#[cfg(all(test, windows))]
mod tests;
