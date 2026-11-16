//! Opening a document from inside a window: Open (Ctrl+O, the right-click menu and the empty
//! window's "Open file") and dropping a file. Either shows the document in that window, or brings
//! the window that already shows it to the front.

use std::fmt;
use std::path::{Path, PathBuf};

use downlight_core::{MARKDOWN_EXTENSIONS, is_markdown};
use tauri::{DragDropEvent, Manager as _, WebviewWindow, WindowEvent};

use crate::documents::{DocumentAt, Documents, ShowError, Target};
use crate::message::Message;
use crate::notice::{NamedFile, Notice};

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
        app.state::<Documents>()
            .give(DocumentAt::new(&document), Target::Window(window.label()))
            .map(|_| ())
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
        show_dropped(&app.state::<Documents>(), &label, &paths);
    });
}

/// Shows the first Markdown file of `paths`, dropped onto `window`, in that window, or tells its
/// page why not.
fn show_dropped(documents: &Documents, window: &str, paths: &[PathBuf]) {
    let failure = match dropped_document(paths).map(Path::to_path_buf) {
        Some(document) => {
            match documents.give(DocumentAt::new(&document), Target::Window(window)) {
                Ok(_) => return,
                Err(source) => OpenError::Show { document, source },
            }
        }
        None => OpenError::NotMarkdown,
    };
    // The notice is the only way to tell the reader about the drop, so a notice that can't be
    // sent leaves nobody to tell.
    let _ = documents.send(window, Message::Notice(failure.notice()));
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
        source: ShowError,
    },
    /// The background task that shows the open dialog panicked or was cancelled.
    DialogStopped { source: tauri::Error },
}

impl OpenError {
    fn notice(&self) -> Notice {
        match self {
            Self::NotMarkdown => {
                let extensions = MARKDOWN_EXTENSIONS.map(|extension| format!(".{extension}"));
                format!(
                    "Downlight opens Markdown files ({}).",
                    extensions.join(", ")
                )
                .into()
            }
            Self::Show { document, source } => Notice::naming(
                "Can't show ",
                NamedFile::at(document),
                format!(": {source}"),
            ),
            Self::DialogStopped { source } => format!("The open dialog stopped: {source}").into(),
        }
    }
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.notice().fmt(f)
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as a notice.
impl serde::Serialize for OpenError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.notice().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::recorder::recorded;

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
    fn drop_of_a_markdown_file_shows_it_in_the_window_it_was_dropped_on() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(
            &documents,
            &window,
            &[repo_file("app/Cargo.toml"), notes.clone()],
        );
        assert_eq!(documents.document_path(&window), Some(notes));
        assert_eq!(recorder.sent(), [(window, Message::AskForDocument)]);
    }

    #[test]
    fn drop_without_markdown_tells_the_page_which_files_downlight_opens() {
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(&documents, &window, &[repo_file("app/Cargo.toml")]);
        assert_eq!(documents.document_path(&window), None);
        assert_eq!(
            recorder.sent(),
            [(window, Message::Notice(OpenError::NotMarkdown.notice()))]
        );
    }

    #[test]
    fn drop_onto_a_window_that_has_closed_says_so_in_the_reader_s_terms() {
        // Ticket 28: it reached the reader as "window not found".
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        documents.forget(&window);
        show_dropped(&documents, &window, &[notes]);
        let sent = recorder.sent();
        let [(_, Message::Notice(notice))] = sent.as_slice() else {
            panic!("expected a notice, got {sent:?}");
        };
        assert!(
            notice.to_string().ends_with(": its window has closed"),
            "{notice}"
        );
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
