//! The documents given on the command line, their windows, and the command that renders them.

use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use downlight_core::RenderedDocument;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::platform;
use crate::timing::{ReadAndRenderTimes, Timing};

/// The documents named by `downlight <file>...`, the first one shown in the first window.
pub struct CommandLineDocuments {
    paths: Vec<PathBuf>,
}

impl CommandLineDocuments {
    /// Takes every argument after the program name as a document's path.
    pub fn from_args(args: impl Iterator<Item = OsString>) -> Self {
        let paths = args
            .skip(1)
            .map(|arg| {
                let path = PathBuf::from(arg);
                // Relative to the shell's folder. If it can't be made absolute, reading the path
                // as given reports the problem to the reader.
                std::path::absolute(&path).unwrap_or(path)
            })
            .collect();
        Self { paths }
    }

    fn path_for(&self, window: &str) -> Option<&Path> {
        self.paths.get(index_of(window)?).map(PathBuf::as_path)
    }
}

const WINDOW_LABEL_PREFIX: &str = "document-";

fn label_of(index: usize) -> String {
    format!("{WINDOW_LABEL_PREFIX}{index}")
}

fn index_of(window: &str) -> Option<usize> {
    window.strip_prefix(WINDOW_LABEL_PREFIX)?.parse().ok()
}

/// Opens the window that shows the first document, or says there is none.
pub fn open_first_window(app: &AppHandle) -> tauri::Result<()> {
    open_window(app, 0)
}

/// Opens the window for the document after `window`'s on the command line, if there is one.
///
/// Only the timing switch calls it, once `window` has shown its document, so that the next
/// window measures a further window in a running process. Ticket 08 replaces this with a window
/// for every launch.
pub fn open_next_window(app: &AppHandle, window: &str) -> tauri::Result<()> {
    let Some(next) = index_of(window).and_then(|index| index.checked_add(1)) else {
        return Ok(());
    };
    if next < app.state::<CommandLineDocuments>().paths.len() {
        open_window(app, next)?;
    }
    Ok(())
}

fn open_window(app: &AppHandle, index: usize) -> tauri::Result<()> {
    let label = label_of(index);
    let timing = app.state::<Timing>();
    timing.window_opening(&label);
    // The page turns on its own timing reports when it sees the query.
    let url = if timing.is_on() {
        WebviewUrl::App("index.html?timing".into())
    } else {
        WebviewUrl::default()
    };
    let builder = WebviewWindowBuilder::new(app, label, url).title("Downlight");
    platform::with_backdrop(builder).build()?;
    Ok(())
}

/// Reads and renders the calling window's document.
///
/// The page passes nothing: the path comes from the app's own state, so the page can't ask for
/// other files. The work runs on a blocking thread, off the main thread, so the window keeps
/// answering while a large document renders.
#[tauri::command]
pub async fn render_document(
    window: WebviewWindow,
    documents: State<'_, CommandLineDocuments>,
    timing: State<'_, Timing>,
) -> Result<RenderedDocument, DocumentError> {
    let path = documents
        .path_for(window.label())
        .ok_or(DocumentError::NoDocument)?
        .to_owned();
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
