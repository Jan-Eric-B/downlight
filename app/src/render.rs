//! Reading a document's file and rendering it on a blocking thread. The documents module starts
//! these and hands what they give to the window's page; this module knows nothing of windows.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use downlight_core::RenderedDocument;
use tauri::async_runtime::JoinHandle;

use crate::image;
use crate::timing::Stages;

/// The rendered document, with how long reading and rendering it took for the timing switch.
pub type ReadAndRender = Result<(RenderedDocument, Stages), DocumentError>;

/// A document being read and rendered on a blocking thread, by [`start`].
pub type RenderTask = JoinHandle<ReadAndRender>;

/// Starts reading and rendering `path` on a blocking thread, off the main thread and the async
/// runtime's workers. The timing switch reports it as `event`: `read-and-render` for a document
/// a window was given, `reload` for reading it again.
pub fn start(path: &Path, event: &'static str) -> RenderTask {
    let path = path.to_owned();
    tauri::async_runtime::spawn_blocking(move || read_and_render(&path, event))
}

fn read_and_render(path: &Path, event: &'static str) -> ReadAndRender {
    let mut stages = Stages::start(event);
    let bytes = std::fs::read(path).map_err(|source| unread(path, source))?;
    stages.lap("read");
    let document = downlight_core::render(&bytes, path, &image::origin());
    stages.lap("render");
    Ok((document, stages))
}

/// Why `path` couldn't be read. Windows reads a folder as access denied, which would send the
/// reader looking at permissions, so a folder is told apart; only after the read failed, so a
/// document still takes one read.
fn unread(path: &Path, source: io::Error) -> DocumentError {
    let path = path.to_owned();
    if path.is_dir() {
        DocumentError::Folder { path }
    } else {
        DocumentError::Read { path, source }
    }
}

/// Why the window can't show its document. The page shows the message to the reader.
#[derive(Debug)]
pub enum DocumentError {
    Read {
        path: PathBuf,
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

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "Can't read {}: {source}", path.display()),
            Self::Folder { path } => write!(f, "{} is a folder, not a document", path.display()),
            Self::ReadAndRenderStopped { source } => {
                write!(f, "Reading and rendering the document stopped: {source}")
            }
        }
    }
}

// The page gets the error inside its answer, serialized, and shows it as text.
impl serde::Serialize for DocumentError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_is_told_apart_from_a_document() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Err(error) = read_and_render(folder, "read-and-render") else {
            panic!("a folder rendered as a document");
        };
        assert_eq!(
            error.to_string(),
            format!("{} is a folder, not a document", folder.display())
        );
    }

    #[test]
    fn missing_file_cannot_be_read() {
        let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing.md");
        let Err(error) = read_and_render(&missing, "read-and-render") else {
            panic!("a missing file rendered as a document");
        };
        assert!(
            error
                .to_string()
                .starts_with(&format!("Can't read {}: ", missing.display())),
            "{error}"
        );
    }
}
