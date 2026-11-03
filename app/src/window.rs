//! A window per document, in one process (ADR 0001): which window shows which document, opening
//! a window or bringing an open document's window to the front, and closing it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, Manager as _, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::platform;
use crate::timing::Timing;

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
        let key = platform::document_key(document);
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
    pub fn forget(&self, window: &str) {
        self.lock().documents.remove(window);
    }

    fn lock(&self) -> MutexGuard<'_, LabelledDocuments> {
        // The map stays valid even if a thread panicked while holding the lock.
        self.labelled.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl LabelledDocuments {
    fn next_label(&mut self) -> String {
        let label = format!("document-{}", self.labels_issued);
        self.labels_issued += 1;
        label
    }
}

/// Shows each document in a window of its own: a new window, or the one that already shows it,
/// brought to the front. With no documents, opens one window that shows none. A window that
/// fails doesn't keep the launch's other documents from opening; the first failure is returned.
pub fn open_documents(app: &AppHandle, documents: &[PathBuf]) -> tauri::Result<()> {
    let open_windows = app.state::<OpenWindows>();
    if documents.is_empty() {
        return open_window(app, &open_windows.claim_empty(), APP_NAME);
    }
    let mut first_failure = None;
    for document in documents {
        let shown = match open_windows.claim(document) {
            Claim::New(label) => open_window(app, &label, &title_of(document)).inspect_err(|_| {
                open_windows.forget(&label);
            }),
            Claim::AlreadyOpen(label) => bring_to_front(app, &label),
        };
        if let Err(error) = shown {
            first_failure.get_or_insert(error);
        }
    }
    first_failure.map_or(Ok(()), Err)
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
    match document
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
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
