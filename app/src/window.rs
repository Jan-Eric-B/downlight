//! A window per document, in one process (ADR 0001): which window shows which document, opening
//! a window or bringing an open document's window to the front, and closing it.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use std::{fmt, io};

use tauri::{
    AppHandle, Emitter as _, Manager as _, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::timing::Timing;
use crate::{APP_NAME, command_line, platform};

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
}

/// The window a document is to be shown in.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A new window, as for a launch from the file manager or the command line.
    NewWindow,
    /// The window with this label, as for Ctrl+O and dropping a file.
    Window(&'a str),
}

/// What showing a document changed, by window label. The caller acts on it: it opens a new
/// window, or titles a changed one and tells its page.
#[derive(Debug, PartialEq, Eq)]
enum Claim {
    /// A new window with this label shows the document; the caller opens it.
    New(String),
    /// The window with this label shows the document now.
    Changed(String),
    /// The document is already open in the window with this label.
    AlreadyOpen(String),
    /// The target window has closed.
    WindowGone,
}

impl OpenWindows {
    /// Records that `target` shows `document` (None: no document), unless the document is open
    /// already. Looking up and recording happen under one lock, so two requests for the same
    /// document end up in one window.
    fn claim(&self, document: Option<&Path>, target: Target<'_>) -> Claim {
        let shown = document.map(|path| ShownDocument {
            path: path.to_owned(),
            key: document_key(path),
        });
        let mut table = self.lock();
        if let Some(shown) = &shown
            && let Some(label) = table.window_showing(&shown.key)
        {
            return Claim::AlreadyOpen(label);
        }
        match target {
            Target::NewWindow => {
                let label = table.next_label();
                table.shown.insert(label.clone(), shown);
                Claim::New(label)
            }
            Target::Window(label) => match table.shown.get_mut(label) {
                Some(entry) => {
                    *entry = shown;
                    Claim::Changed(label.to_owned())
                }
                None => Claim::WindowGone,
            },
        }
    }

    pub fn document_of(&self, window: &str) -> Option<PathBuf> {
        self.lock()
            .shown
            .get(window)
            .and_then(|shown| shown.as_ref().map(|shown| shown.path.clone()))
    }

    /// Forgets a window that closed or never opened, so its document opens anew.
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
            show(app, document.as_deref(), Target::NewWindow)
                .err()
                .map(|error| LaunchFailure { document, error })
        })
        .collect()
}

/// Shows `document` (None: no document) in `target`, the one way a window gets or changes its
/// document. A document that is open already has its window brought to the front instead, so a
/// document is shown in at most one window. A new window opens titled after the document; a
/// window shown another document gets that document's title and a `document-shown` event, on
/// which its page renders the document.
pub fn show(app: &AppHandle, document: Option<&Path>, target: Target<'_>) -> tauri::Result<()> {
    let open_windows = app.state::<OpenWindows>();
    match open_windows.claim(document, target) {
        Claim::New(label) => open_window(app, &label, &title_of(document)).inspect_err(|_| {
            open_windows.forget(&label);
        }),
        Claim::Changed(label) => {
            let window = app
                .get_webview_window(&label)
                .ok_or(tauri::Error::WindowNotFound)?;
            window.set_title(&title_of(document))?;
            app.emit_to(label.as_str(), DOCUMENT_SHOWN, ())
        }
        Claim::AlreadyOpen(label) => bring_to_front(app, &label),
        Claim::WindowGone => Err(tauri::Error::WindowNotFound),
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
    let url = WebviewUrl::App(page_path(timing.is_on(), backdrop).into());
    let builder = WebviewWindowBuilder::new(app, label, url)
        .title(title)
        .visible(false);
    let builder = if backdrop {
        platform::with_backdrop(builder)
    } else {
        builder
    };
    let window = builder.build()?;
    std::thread::spawn(move || {
        std::thread::sleep(LONGEST_HIDDEN);
        // The window may have closed meanwhile, and then there is nothing to show.
        if window.is_visible().is_ok_and(|visible| !visible) {
            let _ = window.show();
        }
    });
    Ok(())
}

/// The page and what it is told in its query: `timing` turns on its timing reports, and
/// `backdrop` says the window is transparent over the system's backdrop (Mica on Windows), so the
/// page draws no background of its own.
fn page_path(timing: bool, backdrop: bool) -> String {
    let flags = [("timing", timing), ("backdrop", backdrop)]
        .into_iter()
        .filter_map(|(flag, on)| on.then_some(flag))
        .collect::<Vec<_>>();
    if flags.is_empty() {
        "index.html".to_owned()
    } else {
        format!("index.html?{}", flags.join("&"))
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
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A file that exists: the app crate's own manifest.
    fn manifest() -> PathBuf {
        app_file("Cargo.toml")
    }

    fn app_file(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    /// Opens a new window for `document` (None: an empty window) and returns its label.
    #[expect(
        clippy::panic,
        reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
    )]
    fn open_new(windows: &OpenWindows, document: Option<&Path>) -> String {
        match windows.claim(document, Target::NewWindow) {
            Claim::New(label) => label,
            other => panic!("expected a new window, got {other:?}"),
        }
    }

    #[test]
    fn first_claim_of_a_document_names_a_new_window_that_shows_it() {
        let windows = OpenWindows::default();
        let label = open_new(&windows, Some(&manifest()));
        assert_eq!(windows.document_of(&label), Some(manifest()));
    }

    #[test]
    fn open_document_claimed_again_gives_its_window() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&manifest()));
        assert_eq!(
            windows.claim(Some(&manifest()), Target::NewWindow),
            Claim::AlreadyOpen(first)
        );
    }

    #[test]
    fn open_document_is_found_ignoring_case() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&manifest()));
        assert_eq!(
            windows.claim(Some(&app_file("CARGO.TOML")), Target::NewWindow),
            Claim::AlreadyOpen(first)
        );
    }

    #[test]
    fn open_document_is_found_through_a_parent_folder_step() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&manifest()));
        let roundabout = app_file("src").join("..").join("Cargo.toml");
        assert_eq!(
            windows.claim(Some(&roundabout), Target::NewWindow),
            Claim::AlreadyOpen(first)
        );
    }

    #[test]
    fn missing_document_claimed_again_in_other_case_gives_its_window() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&app_file("missing.md")));
        assert_eq!(
            windows.claim(Some(&app_file("MISSING.md")), Target::NewWindow),
            Claim::AlreadyOpen(first)
        );
    }

    #[test]
    fn other_document_opens_another_window() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&manifest()));
        let other = open_new(&windows, Some(&app_file("build.rs")));
        assert_ne!(first, other);
    }

    #[test]
    fn forgotten_window_gives_its_document_a_new_window() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&manifest()));
        windows.forget(&first);
        assert!(matches!(
            windows.claim(Some(&manifest()), Target::NewWindow),
            Claim::New(_)
        ));
        assert_eq!(windows.document_of(&first), None);
    }

    #[test]
    fn empty_windows_are_never_the_same_window() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, None);
        let second = open_new(&windows, None);
        assert_ne!(first, second);
        assert_eq!(windows.document_of(&first), None);
    }

    #[test]
    fn window_without_a_document_is_titled_downlight() {
        assert_eq!(title_of(None), "Downlight");
    }

    #[test]
    fn empty_window_shows_the_document_it_is_given() {
        let windows = OpenWindows::default();
        let label = open_new(&windows, None);
        assert_eq!(
            windows.claim(Some(&manifest()), Target::Window(&label)),
            Claim::Changed(label.clone())
        );
        assert_eq!(windows.document_of(&label), Some(manifest()));
    }

    #[test]
    fn window_shows_another_document_in_place_of_its_own() {
        let windows = OpenWindows::default();
        let label = open_new(&windows, Some(&manifest()));
        let other = app_file("build.rs");
        assert_eq!(
            windows.claim(Some(&other), Target::Window(&label)),
            Claim::Changed(label.clone())
        );
        assert_eq!(windows.document_of(&label), Some(other));
    }

    #[test]
    fn document_a_window_gave_up_opens_anew() {
        let windows = OpenWindows::default();
        let label = open_new(&windows, Some(&manifest()));
        windows.claim(Some(&app_file("build.rs")), Target::Window(&label));
        assert!(matches!(
            windows.claim(Some(&manifest()), Target::NewWindow),
            Claim::New(_)
        ));
    }

    #[test]
    fn document_shown_in_a_window_is_found_there_again() {
        let windows = OpenWindows::default();
        let label = open_new(&windows, None);
        windows.claim(Some(&manifest()), Target::Window(&label));
        assert_eq!(
            windows.claim(Some(&app_file("CARGO.TOML")), Target::NewWindow),
            Claim::AlreadyOpen(label)
        );
    }

    #[test]
    fn document_open_in_another_window_stays_there() {
        let windows = OpenWindows::default();
        let first = open_new(&windows, Some(&manifest()));
        let second = open_new(&windows, Some(&app_file("build.rs")));
        assert_eq!(
            windows.claim(Some(&manifest()), Target::Window(&second)),
            Claim::AlreadyOpen(first)
        );
        assert_eq!(windows.document_of(&second), Some(app_file("build.rs")));
    }

    #[test]
    fn showing_in_a_closed_window_changes_nothing() {
        let windows = OpenWindows::default();
        let label = open_new(&windows, None);
        windows.forget(&label);
        assert_eq!(
            windows.claim(Some(&manifest()), Target::Window(&label)),
            Claim::WindowGone
        );
        assert!(matches!(
            windows.claim(Some(&manifest()), Target::NewWindow),
            Claim::New(_)
        ));
    }

    #[test]
    fn failure_names_its_document_and_the_error() {
        let failure = LaunchFailure {
            document: Some(PathBuf::from(r"C:\work\docs\README.md")),
            error: tauri::Error::WindowNotFound,
        };
        assert_eq!(
            failure.to_string(),
            r"Can't open a window for C:\work\docs\README.md: window not found"
        );
    }

    #[test]
    fn failure_of_the_window_without_a_document_names_the_error() {
        let failure = LaunchFailure {
            document: None,
            error: tauri::Error::WindowNotFound,
        };
        assert_eq!(failure.to_string(), "Can't open a window: window not found");
    }

    #[test]
    fn report_names_each_failure_on_a_line_of_its_own() {
        let failures = [
            LaunchFailure {
                document: Some(PathBuf::from(r"C:\work\a.md")),
                error: tauri::Error::WindowNotFound,
            },
            LaunchFailure {
                document: Some(PathBuf::from(r"C:\work\b.md")),
                error: tauri::Error::WindowNotFound,
            },
        ];
        assert_eq!(
            failure_report(&failures).as_deref(),
            Some(
                "Can't open a window for C:\\work\\a.md: window not found\n\
                 Can't open a window for C:\\work\\b.md: window not found"
            )
        );
    }

    #[test]
    fn launch_without_failures_has_nothing_to_report() {
        assert_eq!(failure_report(&[]), None);
    }

    #[test]
    fn page_is_told_about_timing_and_backdrop() {
        assert_eq!(page_path(false, false), "index.html");
        assert_eq!(page_path(true, false), "index.html?timing");
        assert_eq!(page_path(false, true), "index.html?backdrop");
        assert_eq!(page_path(true, true), "index.html?timing&backdrop");
    }

    #[test]
    fn title_names_the_file_then_its_folder_then_downlight() {
        assert_eq!(
            title_of(Some(Path::new(r"C:\work\docs\README.md"))),
            "README.md - docs - Downlight"
        );
    }

    #[test]
    fn title_of_a_file_at_a_drive_root_names_the_drive() {
        assert_eq!(
            title_of(Some(Path::new(r"C:\README.md"))),
            r"README.md - C:\ - Downlight"
        );
    }
}
