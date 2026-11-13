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
    let bytes = std::fs::read(path).map_err(|source| DocumentError::Read {
        path: path.to_owned(),
        source,
    })?;
    stages.lap("read");
    let document = downlight_core::render(&bytes, path, &image::origin());
    stages.lap("render");
    Ok((document, stages))
}

/// Why the window can't show its document. The page shows the message to the reader.
#[derive(Debug)]
pub enum DocumentError {
    Read {
        path: PathBuf,
        source: io::Error,
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
