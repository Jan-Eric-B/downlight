//! Opening a document from inside a window: Open (Ctrl+O, the right-click menu and the empty
//! window's "Open file") and dropping a file. Either shows the document in that window, or brings
//! the window that already shows it to the front. A text file is a document whatever its
//! extension, and a binary file is refused before the window is given it, so the window keeps the
//! document it shows (ticket 34).

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use downlight_core::{MARKDOWN_EXTENSIONS, TEXT_CHECK_LENGTH, is_text};
use tauri::{DragDropEvent, Manager as _, WebviewWindow, WindowEvent};

use crate::documents::{DocumentAt, Documents, ShowError, Target};
use crate::message::Message;
use crate::notice::{NamedFile, Notice};
use crate::render::not_text;

/// Shows the open dialog, filtered to Markdown files with a choice of all files, and shows the
/// chosen document in the calling window. Choosing nothing changes nothing.
///
/// The dialog belongs to the window, so the window waits for it; it runs on a blocking thread so
/// that the main thread and every other window keep answering.
#[tauri::command]
pub async fn open_document(window: WebviewWindow) -> Result<(), Notice> {
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // The first filter is the one the dialog starts with. rfd writes `*` as `*.*`, which
        // Windows matches against every file, one without an extension too.
        let Some(document) = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("Markdown files", &MARKDOWN_EXTENSIONS[..])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return Ok(());
        };
        show_chosen(&app.state::<Documents>(), window.label(), document)
    })
    .await
    .map_err(|source| OpenError::DialogStopped { source })??;
    Ok(())
}

/// Shows `document`, chosen in Open's dialog, in `window`, unless it is a binary file.
fn show_chosen(documents: &Documents, window: &str, document: PathBuf) -> Result<(), OpenError> {
    if !is_text_file(&document) {
        return Err(OpenError::NotText { document });
    }
    give(documents, window, document)
}

/// Shows the first text file dropped onto a window in that window. A drop without one, or a
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

/// Shows the first text file of `paths`, dropped onto `window`, in that window, or tells its page
/// why not.
fn show_dropped(documents: &Documents, window: &str, paths: &[PathBuf]) {
    let shown = dropped_document(paths)
        .and_then(|document| give(documents, window, document.to_path_buf()));
    if let Err(failure) = shown {
        // The notice is the only way to tell the reader about the drop, so a notice that can't be
        // sent leaves nobody to tell.
        let _ = documents.send(window, Message::Notice(failure.into()));
    }
}

/// The first text file of a drop, or why the drop holds none. A shortcut is never followed, nor
/// read, and it is what the reader is told about first, since dropping the file itself is all it
/// takes; then a binary file; a drop of folders alone holds no file.
fn dropped_document(paths: &[PathBuf]) -> Result<&Path, OpenError> {
    let mut shortcut = None;
    let mut binary = None;
    for path in paths {
        if is_shortcut(path) {
            shortcut.get_or_insert(path);
        } else if !path.is_file() {
            continue;
        } else if is_text_file(path) {
            return Ok(path);
        } else {
            binary.get_or_insert(path);
        }
    }
    Err(match (shortcut, binary) {
        (Some(shortcut), _) => OpenError::Shortcut {
            shortcut: shortcut.clone(),
        },
        (None, Some(document)) => OpenError::NotText {
            document: document.clone(),
        },
        (None, None) => OpenError::NoFile,
    })
}

/// Whether `path` names a shortcut, Windows' `.lnk` file, by its extension in any case. Following
/// one would read where it points through the shell, and the reader can drop the file itself
/// (ticket 34). Other systems' links are the file system's own, which opening follows.
fn is_shortcut(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
}

/// Whether the file at `path` is text, by as much of its start as the core looks at. A file whose
/// start can't be read counts as text, so that showing it tells the reader why it can't be read,
/// as a document whose read fails does.
fn is_text_file(path: &Path) -> bool {
    let mut start = Vec::with_capacity(TEXT_CHECK_LENGTH);
    File::open(path)
        .and_then(|file| file.take(TEXT_CHECK_LENGTH as u64).read_to_end(&mut start))
        .map_or(true, |_| is_text(&start))
}

/// Shows `document`, chosen or dropped, in `window`.
fn give(documents: &Documents, window: &str, document: PathBuf) -> Result<(), OpenError> {
    documents
        .give(DocumentAt::new(&document), Target::Window(window))
        .map(|_| ())
        .map_err(|source| OpenError::Show { document, source })
}

/// Why a window doesn't show the document the reader chose or dropped. The page shows the
/// message to the reader.
#[derive(Debug)]
pub enum OpenError {
    /// The drop held no file, only folders or paths that are gone.
    NoFile,
    /// The drop held no text file, only shortcuts and binary files, and this was its first
    /// shortcut.
    Shortcut { shortcut: PathBuf },
    /// The chosen file, or the drop's first file when it held no text file, is binary.
    NotText { document: PathBuf },
    Show {
        document: PathBuf,
        source: ShowError,
    },
    /// The background task that shows the open dialog panicked or was cancelled.
    DialogStopped { source: tauri::Error },
}

impl From<OpenError> for Notice {
    fn from(error: OpenError) -> Self {
        match error {
            OpenError::NoFile => "Downlight opens text files.".into(),
            OpenError::Shortcut { shortcut } => Self::naming(
                "",
                NamedFile::at(&shortcut),
                " is a shortcut. Drop the file itself, not its shortcut.",
            ),
            OpenError::NotText { document } => not_text(&document),
            OpenError::Show { document, source } => Self::naming(
                "Can't show ",
                NamedFile::at(&document),
                format!(": {source}"),
            ),
            OpenError::DialogStopped { source } => {
                format!("The open dialog stopped: {source}").into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::history::Step;
    use crate::documents::recorder::recorded;
    use crate::render::not_text;

    fn repo_file(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)
    }

    /// A binary file in the repository.
    fn icon() -> PathBuf {
        repo_file("app/icons/icon.ico")
    }

    #[test]
    fn drop_of_a_markdown_file_shows_it_in_the_window_it_was_dropped_on() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(&documents, &window, &[icon(), notes.clone()]);
        assert_eq!(documents.document_path(&window), Some(notes));
        assert_eq!(recorder.sent(), [(window, Message::AskForDocument)]);
    }

    #[test]
    fn drop_of_a_text_file_with_another_extension_shows_it() {
        for name in ["app/Cargo.toml", "LICENSE"] {
            let file = repo_file(name);
            let (documents, _) = recorded();
            let window = documents.open_empty().unwrap();
            show_dropped(&documents, &window, std::slice::from_ref(&file));
            assert_eq!(documents.document_path(&window), Some(file), "{name}");
        }
    }

    #[test]
    fn drop_of_another_document_leaves_a_step_back_to_the_one_the_window_showed() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let readme = repo_file("core/tests/fixtures/readme.md");
        let (documents, _) = recorded();
        let window = documents.open_empty().unwrap();
        // Each document joins the history once it has arrived (ticket 38).
        show_dropped(&documents, &window, std::slice::from_ref(&notes));
        documents.page_asks(&window);
        show_dropped(&documents, &window, &[readme]);
        documents.page_asks(&window);
        documents.step(&window, Step::Back).unwrap();
        assert_eq!(documents.document_path(&window), Some(notes));
    }

    #[test]
    fn drop_of_a_binary_file_keeps_the_document_and_says_downlight_opens_text_files() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(&documents, &window, std::slice::from_ref(&notes));
        show_dropped(&documents, &window, &[icon()]);
        assert_eq!(documents.document_path(&window), Some(notes));
        assert_eq!(
            recorder.sent().last(),
            Some(&(window, Message::Notice(not_text(&icon()))))
        );
        assert_eq!(
            not_text(&icon()).to_string(),
            format!(
                "{} isn't a text file. Downlight opens text files.",
                NamedFile::at(&icon())
            )
        );
    }

    #[test]
    fn drop_of_a_shortcut_is_not_followed_and_says_to_drop_the_file_itself() {
        // Told by its name alone, so the shortcut needn't exist, and nothing is read.
        let shortcut = Path::new(env!("CARGO_MANIFEST_DIR")).join("notes.md.LNK");
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(&documents, &window, std::slice::from_ref(&shortcut));
        assert_eq!(documents.document_path(&window), None);
        let sent = recorder.sent();
        let [(_, Message::Notice(notice))] = sent.as_slice() else {
            panic!("expected a notice, got {sent:?}");
        };
        assert_eq!(
            notice.to_string(),
            format!(
                "{} is a shortcut. Drop the file itself, not its shortcut.",
                NamedFile::at(&shortcut)
            )
        );
    }

    #[test]
    fn drop_of_a_shortcut_beside_a_text_file_shows_the_text_file() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let shortcut = repo_file("core/tests/fixtures/notes.md.lnk");
        let (documents, _) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(&documents, &window, &[shortcut, notes.clone()]);
        assert_eq!(documents.document_path(&window), Some(notes));
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
    fn drop_of_a_folder_shows_no_document_and_says_downlight_opens_text_files() {
        let parent = std::env::temp_dir().join(format!("downlight-open-{}", std::process::id()));
        let folder = parent.join("folder.md");
        std::fs::create_dir_all(&folder).unwrap();
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        show_dropped(&documents, &window, &[folder]);
        // Cleaned up before asserting, so a failing run leaves no folder behind.
        std::fs::remove_dir_all(&parent).unwrap();
        assert_eq!(documents.document_path(&window), None);
        let sent = recorder.sent();
        let [(_, Message::Notice(notice))] = sent.as_slice() else {
            panic!("expected a notice, got {sent:?}");
        };
        assert_eq!(notice.to_string(), "Downlight opens text files.");
    }

    #[test]
    fn open_of_a_text_file_with_another_extension_shows_it() {
        let file = repo_file("app/Cargo.toml");
        let (documents, _) = recorded();
        let window = documents.open_empty().unwrap();
        show_chosen(&documents, &window, file.clone()).unwrap();
        assert_eq!(documents.document_path(&window), Some(file));
    }

    #[test]
    fn open_of_a_binary_file_keeps_the_document_and_says_downlight_opens_text_files() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (documents, _) = recorded();
        let window = documents.open_empty().unwrap();
        show_chosen(&documents, &window, notes.clone()).unwrap();
        let refused = show_chosen(&documents, &window, icon()).map_err(Notice::from);
        assert_eq!(refused, Err(not_text(&icon())));
        assert_eq!(documents.document_path(&window), Some(notes));
    }
}
