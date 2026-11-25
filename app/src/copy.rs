//! Copy as Markdown (copy-as-markdown ticket 01): the document's Markdown put on the clipboard
//! instead of the text the reader sees. The page names the pieces of the file its selection
//! covers, never the text; the core turns the bytes of the file the shown render was made from
//! into their Markdown, and the per-OS module writes it to the clipboard
//! (`platform::write_clipboard_text`), so a large copy never passes through the page.

use std::{fmt, io};

use downlight_core::Piece;
use tauri::{Manager as _, State, WebviewWindow};

use crate::documents::{Documents, Showing};
use crate::notice::Notice;
use crate::platform;
use crate::timing::{Stages, Timing};

/// Copies the Markdown of `pieces` of the document the calling window's view shows as `showing`,
/// or of its whole file for no pieces, to the clipboard. Answers as Open does: nothing once it is
/// copied, or the notice that says why it isn't. A showing the window has left copies nothing
/// and says nothing: the reader has moved on.
///
/// Turning a large document into its Markdown takes a while, and writing the clipboard may wait
/// for another program that holds it, so both run on a blocking thread, off the main thread.
#[tauri::command]
pub async fn copy_markdown(
    window: WebviewWindow,
    showing: Showing,
    pieces: Vec<Piece>,
    timing: State<'_, Timing>,
) -> Result<(), Notice> {
    let app = window.app_handle().clone();
    let owner = window.clone();
    let (copied, stages) = tauri::async_runtime::spawn_blocking(move || {
        let mut stages = Stages::start();
        let mut wrote = false;
        let copied = copy(
            &app.state::<Documents>(),
            owner.label(),
            showing,
            &pieces,
            |markdown| {
                stages.lap("markdown");
                let written = platform::write_clipboard_text(&owner, markdown);
                stages.lap("clipboard");
                wrote = true;
                written
            },
        );
        (copied, wrote.then_some(stages))
    })
    .await
    .map_err(|source| CopyError::TaskFailed { source })?;
    // The time a copy takes grows with its selection, so the timing switch reports it
    // (CODING_STANDARDS.md): turning the bytes into Markdown, then writing the clipboard.
    if let Some(stages) = stages {
        timing.report(window.label(), "copy-markdown", &stages);
    }
    Ok(copied?)
}

/// A sample of each set of pieces [`copy_markdown`] takes, by name, which the page's tests check
/// the pieces it sends against (`app/src/contract.rs`): a range of lines, a span within one line,
/// and none for the whole file.
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new kind of piece stops the build here: give it a sample below, then name it here.
    let _sampled = |piece: &Piece| match piece {
        Piece::Lines { .. } | Piece::Span { .. } => {}
    };
    let lines: Vec<Piece> = vec![Piece::Lines { first: 3, last: 5 }];
    let span: Vec<Piece> = vec![Piece::Span {
        line: 4,
        first: 2,
        last: 9,
    }];
    let whole_file: Vec<Piece> = Vec::new();
    Ok(serde_json::json!({
        "lines": serde_json::to_value(lines)?,
        "span": serde_json::to_value(span)?,
        "wholeFile": serde_json::to_value(whole_file)?,
    }))
}

/// Why the clipboard wasn't written, as the per-OS module answers.
#[derive(Debug)]
pub enum ClipboardError {
    /// Another program held the clipboard open through every try.
    Busy,
    /// This system can't write its clipboard until its port.
    #[cfg_attr(
        windows,
        allow(
            dead_code,
            reason = "the other systems' fallback answers it, Windows never does"
        )
    )]
    NotAvailable,
    /// The system refused to take the text, as when it has no memory for it.
    Failed(io::Error),
}

/// Copies the Markdown of `pieces` of the document `window`'s view shows as `showing`, or of its
/// whole file for no pieces, with `write`, which puts it on the clipboard. Writes nothing while
/// the view shows another showing, or no document.
pub fn copy(
    documents: &Documents,
    window: &str,
    showing: Showing,
    pieces: &[Piece],
    write: impl FnOnce(&str) -> Result<(), ClipboardError>,
) -> Result<(), CopyError> {
    let Some(bytes) = documents.bytes_shown_as(window, showing) else {
        return Ok(());
    };
    let markdown = downlight_core::markdown_of(&bytes, pieces)
        .map_err(|source| CopyError::NotInTheFile { source })?;
    write(&markdown).map_err(CopyError::Clipboard)
}

/// Why Copy as Markdown copied nothing. The page shows the reader the notice it turns into.
#[derive(Debug)]
pub enum CopyError {
    /// The pieces aren't in the file the shown render was made from, or a span splits one of its
    /// characters, which only a stale or broken request causes.
    NotInTheFile {
        source: downlight_core::CopyError,
    },
    Clipboard(ClipboardError),
    /// The background task that copies the Markdown panicked or was cancelled.
    TaskFailed {
        source: tauri::Error,
    },
}

impl fmt::Display for CopyError {
    /// What went wrong, for a log: the reader is told the notice instead, and the cause is
    /// [`source`](std::error::Error::source).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInTheFile { .. } => f.write_str("the pieces aren't in the shown document"),
            Self::Clipboard(_) => f.write_str("the clipboard wasn't written"),
            Self::TaskFailed { .. } => f.write_str("the copy's task failed"),
        }
    }
}

impl std::error::Error for CopyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotInTheFile { source } => Some(source),
            Self::TaskFailed { source } => Some(source),
            Self::Clipboard(_) => None,
        }
    }
}

impl From<CopyError> for Notice {
    fn from(error: CopyError) -> Self {
        match error {
            CopyError::NotInTheFile { .. } => {
                "Can't copy the Markdown: the selection is no longer in the document.".into()
            }
            CopyError::Clipboard(ClipboardError::Busy) => {
                "The clipboard is in use by another program. Try again.".into()
            }
            CopyError::Clipboard(ClipboardError::NotAvailable) => {
                "Copy as Markdown isn't available on this system yet.".into()
            }
            CopyError::Clipboard(ClipboardError::Failed(source)) => {
                format!("Can't copy the Markdown: {source}").into()
            }
            CopyError::TaskFailed { source } => {
                format!("Copying the Markdown failed: {source}").into()
            }
        }
    }
}
