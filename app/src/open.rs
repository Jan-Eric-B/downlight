//! Opening a document from inside a window: Open (Ctrl+O, the right-click menu and the empty
//! window's "Open file") and dropping a file. Either shows the document in that window, or brings
//! the window that already shows it to the front.

use std::fmt;
use std::path::{Path, PathBuf};

use downlight_core::{MARKDOWN_EXTENSIONS, is_markdown};
use tauri::{DragDropEvent, Manager as _, WebviewWindow, WindowEvent};

use crate::notice;
use crate::window::{self, Target};

/// Shows the open dialog, filtered to Markdown files, and shows the chosen document in the
/// calling window. Choosing nothing changes nothing.
///
/// The dialog belongs to the window, so the window waits for it; it runs on a blocking thread so
/// that the main thread and every other window keep answering.
#[tauri::command]
pub async fn open_document(window: WebviewWindow) -> Result<(), OpenError> {
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(document) = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("Markdown files", &MARKDOWN_EXTENSIONS[..])
            .pick_file()
        else {
            return Ok(());
        };
        window::show(&app, Some(&document), Target::Window(window.label()))
            .map_err(|source| OpenError::Show { document, source })
    })
    .await
    .map_err(|source| OpenError::DialogStopped { source })?
}

/// Shows the first Markdown file dropped onto a window in that window. A drop without one, or a
/// document that can't be shown, gives the page a message to show instead.
pub fn show_when_dropped(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event else {
        return;
    };
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    let paths = paths.clone();
    // Picking the dropped document and finding the window that shows it read the file system, so
    // they stay off the main thread, where window events arrive.
    tauri::async_runtime::spawn_blocking(move || {
        let failure = match dropped_document(&paths).map(Path::to_path_buf) {
            Some(document) => match window::show(&app, Some(&document), Target::Window(&label)) {
                Ok(()) => return,
                Err(source) => OpenError::Show { document, source },
            },
            None => OpenError::NotMarkdown,
        };
        notice::show_notice(&app, &label, &failure.to_string());
    });
}

/// The first Markdown file of a drop; a folder named like one doesn't count.
fn dropped_document(paths: &[PathBuf]) -> Option<&Path> {
    paths
        .iter()
        .map(PathBuf::as_path)
        .find(|path| is_markdown(path) && path.is_file())
}

/// Why a window doesn't show the document the reader chose or dropped. The page shows the
/// message to the reader.
#[derive(Debug)]
pub enum OpenError {
    /// The drop held no file with a Markdown extension (a folder named like one isn't a file).
    NotMarkdown,
    Show {
        document: PathBuf,
        source: tauri::Error,
    },
    /// The background task that shows the open dialog panicked or was cancelled.
    DialogStopped { source: tauri::Error },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMarkdown => {
                let extensions = MARKDOWN_EXTENSIONS.map(|extension| format!(".{extension}"));
                write!(
                    f,
                    "Downlight opens Markdown files ({}).",
                    extensions.join(", ")
                )
            }
            Self::Show { document, source } => {
                write!(f, "Can't show {}: {source}", document.display())
            }
            Self::DialogStopped { source } => write!(f, "The open dialog stopped: {source}"),
        }
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as text.
impl serde::Serialize for OpenError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_without_markdown_names_the_extensions_downlight_opens() {
        assert_eq!(
            OpenError::NotMarkdown.to_string(),
            "Downlight opens Markdown files (.md, .markdown, .mdown, .mkd, .mkdn, .mdwn)."
        );
    }

    fn repo_file(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)
    }

    #[test]
    fn drop_of_text_and_markdown_shows_the_markdown_file() {
        let markdown = repo_file("core/tests/fixtures/notes.md");
        let paths = [repo_file("app/Cargo.toml"), markdown.clone()];
        assert_eq!(dropped_document(&paths), Some(markdown.as_path()));
    }

    #[test]
    fn drop_without_markdown_shows_no_document() {
        let paths = [repo_file("app/Cargo.toml")];
        assert_eq!(dropped_document(&paths), None);
    }

    #[test]
    fn drop_of_folder_named_like_markdown_shows_no_document() {
        let parent = std::env::temp_dir().join(format!("downlight-open-{}", std::process::id()));
        let folder = parent.join("folder.md");
        std::fs::create_dir_all(&folder).unwrap();
        let shown = dropped_document(&[folder]).map(Path::to_path_buf);
        // Cleaned up before asserting, so a failing run leaves no folder behind.
        std::fs::remove_dir_all(&parent).unwrap();
        assert_eq!(shown, None);
    }
}
