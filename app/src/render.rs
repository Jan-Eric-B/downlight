//! Reading a document's file and rendering it on a blocking thread. The documents module starts
//! these and hands what they give to the window's page; this module knows nothing of windows.

use std::io;
use std::path::{Path, PathBuf};

use downlight_core::{RenderError, RenderedDocument};
use tauri::async_runtime::JoinHandle;

use crate::disk::Disk;
use crate::image;
use crate::notice::{NamedFile, Notice};
use crate::platform;
use crate::timing::Stages;
use crate::watch::ReadStamp;

/// The rendered document, with how long reading and rendering it took for the timing switch.
pub type ReadAndRender = Result<(RenderedDocument, Stages), DocumentError>;

/// A document being read and rendered on a blocking thread, by [`start`].
pub type RenderTask = JoinHandle<ReadAndRender>;

/// Starts reading and rendering `path` on a blocking thread, off the main thread and the async
/// runtime's workers. The timing switch reports it by the place the page gets (`Place` in
/// `documents.rs`).
///
/// Gives the file's stamp from just before the read too, which a watch on the file starts from,
/// so that a save after the read reloads the document, even one made before the watch started
/// (ticket 30). A reload or a render the page asks for drops it: the watch runs already.
pub fn start(path: &Path) -> (RenderTask, ReadStamp) {
    let path = path.to_owned();
    let (noter, read_stamp) = ReadStamp::pending();
    let task = tauri::async_runtime::spawn_blocking(move || {
        noter.note(&path);
        read_and_render(&path)
    });
    (task, read_stamp)
}

fn read_and_render(path: &Path) -> ReadAndRender {
    let mut stages = Stages::start();
    let bytes = std::fs::read(path).map_err(|source| unread(path, source))?;
    stages.lap("read");
    let document =
        downlight_core::render(&bytes, path, &image::origin(), Disk).map_err(|source| {
            DocumentError::NotRendered {
                path: path.to_owned(),
                source,
            }
        })?;
    stages.lap("render");
    Ok((document, stages))
}

/// Why `path` couldn't be read. Windows reads a folder as access denied, which would send the
/// reader looking at permissions, so a folder is told apart; only after the read failed, so a
/// document still takes one read. A missing file's folder opens from the notice (window design
/// ticket 11), looked at here, on the blocking thread.
fn unread(path: &Path, source: io::Error) -> DocumentError {
    if path.is_dir() {
        return DocumentError::Folder {
            path: path.to_owned(),
        };
    }
    let file = if platform::is_missing(&source) {
        NamedFile::missing(path)
    } else {
        NamedFile::at(path)
    };
    DocumentError::Read { file, source }
}

/// Why the window can't show its document. The page shows it to the reader, in the view or as a
/// notice.
#[derive(Debug)]
pub enum DocumentError {
    Read {
        file: NamedFile,
        source: io::Error,
    },
    /// The path names a folder: a launch's argument, a dropped path or a link that isn't a file.
    Folder {
        path: PathBuf,
    },
    /// The core refused the file, a binary one: from the command line, as a link's `.md` target,
    /// or a document whose file became binary since it was shown. Open and a drop refuse a binary
    /// file before the window is given it (`open.rs`).
    NotRendered {
        path: PathBuf,
        source: RenderError,
    },
    /// The background task that reads and renders the document panicked or was cancelled.
    ReadAndRenderStopped {
        source: tauri::Error,
    },
}

impl From<DocumentError> for Notice {
    fn from(error: DocumentError) -> Self {
        match error {
            DocumentError::Read { file, source } => {
                Self::naming("Can't read ", file, format!(": {source}"))
            }
            DocumentError::Folder { path } => {
                Self::naming("", NamedFile::at(&path), " is a folder, not a document")
            }
            DocumentError::NotRendered {
                path,
                source: RenderError::NotText,
            } => not_text(&path),
            DocumentError::ReadAndRenderStopped { source } => {
                format!("Reading and rendering the document stopped: {source}").into()
            }
        }
    }
}

/// What the reader is told about the binary file at `path`, however it arrived: the same words
/// for a launch, a link, Open and a drop (ticket 34).
pub fn not_text(path: &Path) -> Notice {
    Notice::naming(
        "",
        NamedFile::at(path),
        " isn't a text file. Downlight opens text files.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_is_told_apart_from_a_document() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Err(error) = read_and_render(folder) else {
            panic!("a folder rendered as a document");
        };
        assert_eq!(
            Notice::from(error).to_string(),
            format!("{} is a folder, not a document", NamedFile::at(folder))
        );
    }

    #[test]
    fn binary_file_is_refused_with_a_notice_that_downlight_opens_text_files() {
        let icon = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("icons")
            .join("icon.ico");
        let Err(error) = read_and_render(&icon) else {
            panic!("a binary file rendered as a document");
        };
        assert_eq!(
            Notice::from(error).to_string(),
            format!(
                "{} isn't a text file. Downlight opens text files.",
                NamedFile::at(&icon)
            )
        );
    }

    #[test]
    fn text_file_with_another_extension_renders_as_markdown() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let Ok((document, _)) = read_and_render(&manifest) else {
            panic!("a text file was refused");
        };
        assert!(document.as_str().contains("[package]"), "{document:?}");
    }

    /// Reads `missing.md` in a folder of its own, which is there unless `folder_gone`.
    fn read_missing(folder_gone: bool) -> (PathBuf, serde_json::Value) {
        let folder = std::env::temp_dir().join(format!(
            "downlight-render-{}-{folder_gone}",
            std::process::id()
        ));
        std::fs::create_dir_all(&folder).unwrap();
        if folder_gone {
            std::fs::remove_dir_all(&folder).unwrap();
        }
        let read = read_and_render(&folder.join("missing.md"));
        let _ = std::fs::remove_dir_all(&folder);
        let Err(error) = read else {
            panic!("a missing file rendered as a document");
        };
        (folder, serde_json::to_value(Notice::from(error)).unwrap())
    }

    #[test]
    fn missing_file_cannot_be_read_and_the_notice_opens_its_folder() {
        let (folder, error) = read_missing(false);
        assert_eq!(error["before"], "Can't read ");
        assert_eq!(
            error["file"],
            serde_json::json!({
                "name": "missing.md",
                "folder": folder.display().to_string(),
                "folderOpens": true,
            })
        );
    }

    #[test]
    fn missing_file_whose_folder_is_gone_too_names_a_folder_that_does_not_open() {
        let (_, error) = read_missing(true);
        assert_eq!(error["file"]["folderOpens"], false);
    }
}
