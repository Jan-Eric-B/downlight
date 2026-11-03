//! The document given on the command line, its window, and the command that renders it.

use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use downlight_core::RenderedDocument;
use tauri::{AppHandle, State, WebviewUrl, WebviewWindowBuilder};

use crate::platform;

/// The document named by `downlight <file>`, if any.
pub struct CommandLineDocument {
    path: Option<PathBuf>,
}

impl CommandLineDocument {
    /// Takes the first argument after the program name as the document's path.
    pub fn from_args(mut args: impl Iterator<Item = OsString>) -> Self {
        let path = args.nth(1).map(|arg| {
            let path = PathBuf::from(arg);
            // Relative to the shell's folder. If it can't be made absolute, reading the path as
            // given reports the problem to the reader.
            std::path::absolute(&path).unwrap_or(path)
        });
        Self { path }
    }
}

/// Opens the window that shows the document.
pub fn open_window(app: &AppHandle) -> tauri::Result<()> {
    let builder =
        WebviewWindowBuilder::new(app, "document", WebviewUrl::default()).title("Downlight");
    platform::with_backdrop(builder).build()?;
    Ok(())
}

/// Reads and renders the window's document.
///
/// The page passes nothing: the path comes from the app's own state, so the page can't ask for
/// other files. The work runs on a blocking thread, off the main thread, so the window keeps
/// answering while a large document renders.
#[tauri::command]
pub async fn render_document(
    document: State<'_, CommandLineDocument>,
) -> Result<RenderedDocument, DocumentError> {
    let path = document.path.clone().ok_or(DocumentError::NoDocument)?;
    tauri::async_runtime::spawn_blocking(move || read_and_render(&path))
        .await
        .map_err(|source| DocumentError::ReadAndRenderStopped { source })?
}

fn read_and_render(path: &Path) -> Result<RenderedDocument, DocumentError> {
    let bytes = std::fs::read(path).map_err(|source| DocumentError::Read {
        path: path.to_owned(),
        source,
    })?;
    Ok(downlight_core::render(&bytes, path))
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
