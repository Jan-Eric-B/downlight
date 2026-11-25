//! Paste (paste-markdown ticket 01): the clipboard's plain text shown as a document. The text is
//! written to a file of Downlight's own in the pasted folder, and the window is given that file as
//! Open and a drop give it one (ADR 0006), so History, the Wait note, Stop, watching, Reload, Edit
//! and the title bar need nothing of their own for a paste.
//!
//! Text dragged from another program and dropped onto a window is shown the same way, in a file
//! named `Dropped ...` (paste-markdown ticket 04).
//!
//! Reading the clipboard and the drag is the per-OS module's (`platform::clipboard_text`,
//! `platform::add_drop_target`); this module owns the pasted folder and what is written to it.

use std::fs::OpenOptions;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use downlight_core::is_text;
use tauri::{Manager as _, State, WebviewWindow};

use crate::documents::{DocumentAt, Documents, ShowError, Target};
use crate::message::Message;
use crate::notice::{NamedFile, Notice};
use crate::platform;
use crate::timing::{Stages, Timing};

/// Shows the clipboard's text in the calling window. Answers as Open does: nothing once the
/// window is given the paste, or the notice that says why it isn't, with the window as it was.
///
/// Reading the clipboard may wait for another program that holds it, and writing a large paste
/// takes a while, so both run on a blocking thread, off the main thread.
#[tauri::command]
pub async fn paste_clipboard(
    window: WebviewWindow,
    timing: State<'_, Timing>,
) -> Result<(), Notice> {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    let (pasted, stages) = tauri::async_runtime::spawn_blocking(move || {
        let mut stages = Stages::start();
        let clipboard = platform::clipboard_text();
        stages.lap("clipboard");
        let pasted = paste(
            &app.state::<Documents>(),
            &label,
            clipboard,
            platform::local_time(),
        );
        stages.lap("show");
        (pasted, stages)
    })
    .await
    .map_err(|source| PasteError::TaskFailed { source })?;
    // The time a paste takes grows with its text, so the timing switch reports it
    // (CODING_STANDARDS.md); a refused one too, whose read may have waited on a busy clipboard.
    timing.report(window.label(), "paste", &stages);
    Ok(pasted?)
}

/// What the clipboard held when Paste read it, as the per-OS module answers.
#[derive(Debug)]
pub enum Clipboard {
    Text(String),
    /// The clipboard holds no text, such as an image, copied files or nothing at all.
    NoText,
    /// Another program held the clipboard open through every try.
    Busy,
    /// This system can't read its clipboard until its port.
    #[cfg_attr(
        windows,
        allow(
            dead_code,
            reason = "the other systems' fallback answers it, Windows never does"
        )
    )]
    NotAvailable,
}

/// A moment in local time, to the second, which names a pasted or dropped file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
}

/// The folder pasted files are written to. A pasted file is any file in it, whichever way it was
/// later given to a window, which is how deleting pastes (paste-markdown ticket 02) tells them.
#[derive(Debug, Clone)]
pub struct PastedFolder {
    folder: PathBuf,
}

impl PastedFolder {
    /// `%TEMP%\Downlight\Pasted\`: the system's temporary folder, since a paste is meant to be
    /// read once and thrown away (paste-markdown spec).
    pub fn in_temp() -> Self {
        Self {
            folder: std::env::temp_dir().join(crate::APP_NAME).join("Pasted"),
        }
    }

    /// The pasted folder at `folder`, so that a test has its own.
    #[cfg(test)]
    pub const fn at(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// The folder's path, as it was given.
    pub fn path(&self) -> &Path {
        &self.folder
    }

    /// Writes `text`, which came from `origin`, to a new file named by `at`, created with the folder
    /// if it isn't there yet, and gives its path. A name already taken, as by an earlier paste in
    /// the same second, gets ` (2)`, ` (3)` and so on before its extension; the file is created
    /// only if it doesn't exist yet, so that no paste ever overwrites another, even one another
    /// Downlight process wrote.
    fn write(&self, text: &str, origin: PasteOrigin, at: LocalTime) -> Result<PathBuf, PasteError> {
        let LocalTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        } = at;
        let how = match origin {
            PasteOrigin::Pasted => "Pasted",
            PasteOrigin::Dropped => "Dropped",
        };
        let stem = format!("{how} {year:04}-{month:02}-{day:02} {hour:02}.{minute:02}.{second:02}");
        std::fs::create_dir_all(&self.folder).map_err(|source| PasteError::Write {
            file: self.folder.clone(),
            source,
        })?;
        let mut number = 1_u32;
        loop {
            let name = match number {
                1 => format!("{stem}.md"),
                _ => format!("{stem} ({number}).md"),
            };
            let file = self.folder.join(name);
            match OpenOptions::new().write(true).create_new(true).open(&file) {
                Ok(mut created) => {
                    if let Err(source) = created.write_all(text.as_bytes()) {
                        // Closed first, as Windows removes a file only once its last handle closes.
                        drop(created);
                        // A part of the paste no window was given would stay until the next
                        // startup's sweep. One already gone is as good as removed, and any other
                        // failure leaves it for that sweep: the reader is told the write failed.
                        let _ = std::fs::remove_file(&file);
                        return Err(PasteError::Write { file, source });
                    }
                    return Ok(file);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => number += 1,
                Err(source) => return Err(PasteError::Write { file, source }),
            }
        }
    }
}

/// Shows what `clipboard` held in `window`: its text, written to a new file in the pasted folder
/// named by `at`, which the window is then given as Open gives it a file. Text that is empty, only
/// whitespace or binary is refused, and so is a clipboard without text; a refused or failed paste
/// leaves the window as it was.
pub fn paste(
    documents: &Documents,
    window: &str,
    clipboard: Clipboard,
    at: LocalTime,
) -> Result<(), PasteError> {
    let text = match clipboard {
        Clipboard::Text(text) => text,
        Clipboard::NoText => return Err(PasteError::NoText),
        Clipboard::Busy => return Err(PasteError::Busy),
        Clipboard::NotAvailable => return Err(PasteError::NotAvailable),
    };
    show(documents, window, &text, PasteOrigin::Pasted, at)
}

/// Shows `text`, dropped onto `window` from another program (paste-markdown ticket 04), as Paste
/// shows the clipboard's: written to a new file in the pasted folder named by `at`, which the
/// window is then given. Paste's refusals and notices hold, but for text with nothing to show,
/// whose drag the drop target already refused with the no-drop cursor, so letting go of it does
/// nothing.
pub fn show_dropped_text(documents: &Documents, window: &str, text: &str, at: LocalTime) {
    match show(documents, window, text, PasteOrigin::Dropped, at) {
        Ok(()) | Err(PasteError::NoText) => {}
        // The notice is the only way to tell the reader about the drop, so a notice that can't be
        // sent leaves nobody to tell.
        Err(failure) => {
            let _ = documents.send(window, Message::Notice(failure.into()));
        }
    }
}

/// Where the text of a paste came from, which names its file.
#[derive(Debug, Clone, Copy)]
enum PasteOrigin {
    /// From the clipboard, by Ctrl+V or the menu.
    Pasted,
    /// Dragged from another program and dropped onto the window.
    Dropped,
}

/// Shows `text`, which came from `origin`, in `window`: written to a new file in the pasted folder
/// named by `at`, which the window is then given as Open gives it a file.
fn show(
    documents: &Documents,
    window: &str,
    text: &str,
    origin: PasteOrigin,
    at: LocalTime,
) -> Result<(), PasteError> {
    // Only spaces or empty lines would show an empty document (user story 18).
    if text.trim().is_empty() {
        return Err(PasteError::NoText);
    }
    // The same rule as for a file, however the text arrives (user story 43).
    if !is_text(text.as_bytes()) {
        return Err(PasteError::NotText);
    }
    let document = documents.pasted_folder().write(text, origin, at)?;
    documents
        .give(DocumentAt::new(&document), Target::Window(window))
        .map(|_| ())
        .map_err(|source| PasteError::Show { document, source })
}

/// Why a paste isn't shown. The page shows the reader the notice it turns into.
#[derive(Debug)]
pub enum PasteError {
    /// The clipboard held no text, or only whitespace.
    NoText,
    /// Another program kept the clipboard open through every try.
    Busy,
    /// The system has no way to read the clipboard yet.
    NotAvailable,
    /// The clipboard's text would count as a binary file.
    NotText,
    /// The folder or the file couldn't be created or written.
    Write { file: PathBuf, source: io::Error },
    Show {
        document: PathBuf,
        source: ShowError,
    },
    /// The background task that reads and writes the paste panicked or was cancelled.
    TaskFailed { source: tauri::Error },
}

impl From<PasteError> for Notice {
    fn from(error: PasteError) -> Self {
        match error {
            PasteError::NoText => {
                "The clipboard holds no text. To open a file, drop it here or use Open.".into()
            }
            PasteError::Busy => "The clipboard is in use by another program. Try again.".into(),
            PasteError::NotAvailable => "Paste isn't available on this system yet.".into(),
            PasteError::NotText => "Downlight opens text files.".into(),
            PasteError::Write { file, source } => {
                Self::naming("Can't write ", NamedFile::at(&file), format!(": {source}"))
            }
            PasteError::Show { document, source } => Self::naming(
                "Can't show ",
                NamedFile::at(&document),
                format!(": {source}"),
            ),
            PasteError::TaskFailed { source } => format!("Pasting failed: {source}").into(),
        }
    }
}
