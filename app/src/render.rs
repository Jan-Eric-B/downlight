//! Reading a document's file and rendering it on a blocking thread. The documents module starts
//! these and hands what they give to the window's page; this module knows nothing of windows.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use downlight_core::RenderedDocument;
use tauri::async_runtime::JoinHandle;

use crate::image;
use crate::notice::{NamedFile, Notice};
use crate::platform;
use crate::timing::Stages;

/// The rendered document, with how long reading and rendering it took for the timing switch.
pub type ReadAndRender = Result<(RenderedDocument, Stages), DocumentError>;

/// A document being read and rendered on a blocking thread, by [`start`].
pub type RenderTask = JoinHandle<ReadAndRender>;

/// Starts reading and rendering `path` on a blocking thread, off the main thread and the async
/// runtime's workers. The timing switch reports it by the place the page gets (`Place` in
/// `documents.rs`).
pub fn start(path: &Path) -> RenderTask {
    let path = path.to_owned();
    tauri::async_runtime::spawn_blocking(move || read_and_render(&path))
}

fn read_and_render(path: &Path) -> ReadAndRender {
    let mut stages = Stages::start();
    let bytes = std::fs::read(path).map_err(|source| unread(path, source))?;
    stages.lap("read");
    let document = downlight_core::render(&bytes, path, &image::origin());
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
    /// The background task that reads and renders the document panicked or was cancelled.
    ReadAndRenderStopped {
        source: tauri::Error,
    },
}

impl DocumentError {
    fn notice(&self) -> Notice {
        match self {
            Self::Read { file, source } => {
                Notice::naming("Can't read ", file.clone(), format!(": {source}"))
            }
            Self::Folder { path } => {
                Notice::naming("", NamedFile::at(path), " is a folder, not a document")
            }
            Self::ReadAndRenderStopped { source } => {
                format!("Reading and rendering the document stopped: {source}").into()
            }
        }
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.notice().fmt(f)
    }
}

// The page gets the error inside its answer, serialized, and shows it as a notice.
impl serde::Serialize for DocumentError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.notice().serialize(serializer)
    }
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
            error.to_string(),
            format!("{} is a folder, not a document", NamedFile::at(folder))
        );
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
        (folder, serde_json::to_value(&error).unwrap())
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
