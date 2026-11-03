//! A window per document, in one process (ADR 0001): which window shows which document, opening
//! a window or bringing an open document's window to the front, and closing it.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::{fmt, io};

use tauri::{
    AppHandle, Manager as _, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::timing::Timing;
use crate::{command_line, platform};

const APP_NAME: &str = "Downlight";

/// Which window shows which document, by window label. The app's managed state.
#[derive(Default)]
pub struct OpenWindows {
    labelled: Mutex<LabelledDocuments>,
}

#[derive(Default)]
struct LabelledDocuments {
    /// Labels are `document-0`, `document-1` and so on, never reused while the process runs.
    labels_issued: usize,
    documents: HashMap<String, ShownDocument>,
}

struct ShownDocument {
    /// As the launch named it, for reading and the title.
    path: PathBuf,
    /// For finding the window when the document is opened again.
    key: PathBuf,
}

/// The window a document is to be shown in.
#[derive(Debug, PartialEq, Eq)]
enum Claim {
    /// A new window with this label shows the document; the caller opens it.
    New(String),
    /// The document is already open in the window with this label.
    AlreadyOpen(String),
}

impl OpenWindows {
    /// Finds the window that shows `document`, or names a new window for it. Checking and naming
    /// happen under one lock, so two launches with the same document open one window.
    fn claim(&self, document: &Path) -> Claim {
        let key = document_key(document);
        let mut windows = self.lock();
        if let Some(label) = windows
            .documents
            .iter()
            .find_map(|(label, shown)| (shown.key == key).then(|| label.clone()))
        {
            return Claim::AlreadyOpen(label);
        }
        let label = windows.next_label();
        windows.documents.insert(
            label.clone(),
            ShownDocument {
                path: document.to_owned(),
                key,
            },
        );
        Claim::New(label)
    }

    /// Names a new window that shows no document.
    fn claim_empty(&self) -> String {
        self.lock().next_label()
    }

    pub fn document_of(&self, window: &str) -> Option<PathBuf> {
        self.lock()
            .documents
            .get(window)
            .map(|shown| shown.path.clone())
    }

    /// Forgets a window that closed or never opened, so its document opens anew.
    fn forget(&self, window: &str) {
        self.lock().documents.remove(window);
    }

    fn lock(&self) -> MutexGuard<'_, LabelledDocuments> {
        // The map stays valid even if a thread panicked while holding the lock.
        self.labelled.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What two paths to the same document have in common: the path with links, `..` and short names
/// resolved, in the case the file system compares names in. A missing file keeps its name as
/// given, in its resolved folder; with its folder missing too, the path stays as given, which the
/// command line already made absolute.
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

impl LabelledDocuments {
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
    let open_windows = app.state::<OpenWindows>();
    let documents = command_line::document_paths(args, launching_folder);
    if documents.is_empty() {
        return open_window(app, &open_windows.claim_empty(), APP_NAME)
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
            let shown = match open_windows.claim(&document) {
                Claim::New(label) => {
                    open_window(app, &label, &title_of(&document)).inspect_err(|_| {
                        open_windows.forget(&label);
                    })
                }
                Claim::AlreadyOpen(label) => bring_to_front(app, &label),
            };
            shown.err().map(|error| LaunchFailure {
                document: Some(document),
                error,
            })
        })
        .collect()
}

/// Forgets a closed window's document, so opening the document again opens a new window.
pub fn forget_when_destroyed(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::Destroyed = event {
        window.state::<OpenWindows>().forget(window.label());
    }
}

fn open_window(app: &AppHandle, label: &str, title: &str) -> tauri::Result<()> {
    let timing = app.state::<Timing>();
    timing.window_opening(label);
    // The page turns on its own timing reports when it sees the query.
    let url = if timing.is_on() {
        WebviewUrl::App("index.html?timing".into())
    } else {
        WebviewUrl::default()
    };
    let builder = WebviewWindowBuilder::new(app, label, url).title(title);
    platform::with_backdrop(builder).build()?;
    Ok(())
}

fn bring_to_front(app: &AppHandle, label: &str) -> tauri::Result<()> {
    // A window that is still being built has no handle yet, and comes up in front anyway.
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

/// `README.md - docs - Downlight`: the folder tells documents with the same name apart in Alt+Tab
/// and on the taskbar.
fn title_of(document: &Path) -> String {
    let file = document
        .file_name()
        .map_or_else(|| document.display(), |name| Path::new(name).display());
    // The command line made the path absolute, so only a drive root has no folder.
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
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn label_of(claim: &Claim) -> &str {
        match claim {
            Claim::New(label) | Claim::AlreadyOpen(label) => label,
        }
    }

    #[test]
    fn first_claim_of_a_document_names_a_new_window_that_shows_it() {
        let windows = OpenWindows::default();
        let Claim::New(label) = windows.claim(&manifest()) else {
            panic!("the document isn't open yet");
        };
        assert_eq!(windows.document_of(&label), Some(manifest()));
    }

    #[test]
    fn open_document_claimed_again_gives_its_window() {
        let windows = OpenWindows::default();
        let first = windows.claim(&manifest());
        assert_eq!(
            windows.claim(&manifest()),
            Claim::AlreadyOpen(label_of(&first).to_owned())
        );
    }

    #[test]
    fn open_document_is_found_ignoring_case() {
        let windows = OpenWindows::default();
        let first = windows.claim(&manifest());
        let upper = Path::new(env!("CARGO_MANIFEST_DIR")).join("CARGO.TOML");
        assert_eq!(
            windows.claim(&upper),
            Claim::AlreadyOpen(label_of(&first).to_owned())
        );
    }

    #[test]
    fn open_document_is_found_through_a_parent_folder_step() {
        let windows = OpenWindows::default();
        let first = windows.claim(&manifest());
        let roundabout = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("..")
            .join("Cargo.toml");
        assert_eq!(
            windows.claim(&roundabout),
            Claim::AlreadyOpen(label_of(&first).to_owned())
        );
    }

    #[test]
    fn missing_document_claimed_again_in_other_case_gives_its_window() {
        let windows = OpenWindows::default();
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        let first = windows.claim(&folder.join("missing.md"));
        assert_eq!(
            windows.claim(&folder.join("MISSING.md")),
            Claim::AlreadyOpen(label_of(&first).to_owned())
        );
    }

    #[test]
    fn other_document_opens_another_window() {
        let windows = OpenWindows::default();
        let first = windows.claim(&manifest());
        let other = windows.claim(&Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"));
        assert!(matches!(&other, Claim::New(label) if label != label_of(&first)));
    }

    #[test]
    fn forgotten_window_gives_its_document_a_new_window() {
        let windows = OpenWindows::default();
        let first = windows.claim(&manifest());
        windows.forget(label_of(&first));
        assert!(matches!(windows.claim(&manifest()), Claim::New(_)));
        assert_eq!(windows.document_of(label_of(&first)), None);
    }

    #[test]
    fn empty_windows_are_never_the_same_window() {
        let windows = OpenWindows::default();
        let first = windows.claim_empty();
        let second = windows.claim_empty();
        assert_ne!(first, second);
        assert_eq!(windows.document_of(&first), None);
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
    fn title_names_the_file_then_its_folder_then_downlight() {
        assert_eq!(
            title_of(Path::new(r"C:\work\docs\README.md")),
            "README.md - docs - Downlight"
        );
    }

    #[test]
    fn title_of_a_file_at_a_drive_root_names_the_drive() {
        assert_eq!(
            title_of(Path::new(r"C:\README.md")),
            r"README.md - C:\ - Downlight"
        );
    }
}
