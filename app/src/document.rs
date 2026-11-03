//! The command that reads and renders a window's document.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use downlight_core::RenderedDocument;
use tauri::{State, WebviewWindow};

use crate::timing::{ReadAndRenderTimes, Timing};
use crate::window::OpenWindows;

/// Reads and renders the calling window's document.
///
/// The page passes nothing: the path comes from the app's own state, so the page can't ask for
/// other files. The work runs on a blocking thread, off the main thread, so the window keeps
/// answering while a large document renders.
#[tauri::command]
pub async fn render_document(
    window: WebviewWindow,
    open_windows: State<'_, OpenWindows>,
    timing: State<'_, Timing>,
) -> Result<RenderedDocument, DocumentError> {
    let path = open_windows
        .document_of(window.label())
        .ok_or(DocumentError::NoDocument)?;
    let (document, times) = tauri::async_runtime::spawn_blocking(move || read_and_render(&path))
        .await
        .map_err(|source| DocumentError::ReadAndRenderStopped { source })??;
    timing.read_and_render(window.label(), &times);
    Ok(document)
}

fn read_and_render(path: &Path) -> Result<(RenderedDocument, ReadAndRenderTimes), DocumentError> {
    let started = SystemTime::now();
    let clock = Instant::now();
    let bytes = std::fs::read(path).map_err(|source| DocumentError::Read {
        path: path.to_owned(),
        source,
    })?;
    let read = clock.elapsed();
    let document = downlight_core::render(&bytes, path);
    let times = ReadAndRenderTimes {
        started,
        read,
        render: clock.elapsed().saturating_sub(read),
    };
    Ok((document, times))
}

/// Why the window can't show its document. The page shows the message to the reader.
#[derive(Debug)]
pub enum DocumentError {
    NoDocument,
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
            Self::NoDocument => write!(f, "No document to show. Run downlight <file>."),
            Self::Read { path, source } => write!(f, "Can't read {}: {source}", path.display()),
            Self::ReadAndRenderStopped { source } => {
                write!(f, "Reading and rendering the document stopped: {source}")
            }
        }
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as text.
impl serde::Serialize for DocumentError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
