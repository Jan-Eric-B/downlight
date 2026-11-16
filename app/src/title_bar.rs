//! The title bar the page draws in place of the native one (ADR 0005, window design ticket 01):
//! the window's own buttons, the window behind the page's maximize button that brings back
//! Windows' snap layouts flyout, and the window menu and the reveal behind the logo and the file
//! name (ticket 04).

use std::path::{Path, PathBuf};
use std::{fmt, io};

use serde::Deserialize;
use tauri::{Manager as _, State, WebviewWindow, WindowEvent};

use crate::documents::Documents;
use crate::message::{Message, Pages as _};
use crate::platform;

/// Where the page's maximize button is, in physical pixels from the top left of the window's
/// client area, as the page measures it each time it lays the title bar out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct ButtonRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ButtonRect {
    /// Whether `point` is on the button: its right and bottom edges belong to what is beside it.
    pub fn contains(self, (x, y): (i32, i32)) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

/// How the page shows its maximize button while the pointer is on it. On Windows the pointer
/// there reaches the window behind the button, not the page, so the app tells the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaximizeButtonState {
    Normal,
    Hovered,
    Pressed,
}

/// A sample of the argument `place_maximize_button` takes, which the page's tests check the
/// button it measures against (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "maximizeButton": serde_json::to_value(ButtonRect {
            x: 1362,
            y: 0,
            width: 69,
            height: 48,
        })?,
    }))
}

/// Opens the window behind the page's maximize button, which tells the page how to show the
/// button.
pub fn add_maximize_button(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    platform::add_maximize_button(window, move |state| {
        // A page that can't be told keeps showing the button as it was, which is all it costs.
        let _ = app.send(&label, Message::MaximizeButton(state));
    });
}

/// The page laid its title bar out, and its maximize button is at `button` now.
#[tauri::command]
pub fn place_maximize_button(window: WebviewWindow, button: ButtonRect) {
    platform::place_maximize_button(&window, button);
}

/// The window's minimize button.
#[tauri::command]
pub fn minimize_window(window: WebviewWindow) -> tauri::Result<()> {
    window.minimize()
}

/// The window's maximize button, for a click that reaches the page's own button. The page draws
/// its title bar only on Windows (`platform::draws_own_title_bar`), and there the window behind
/// the button takes the clicks; this is the fallback while that window isn't there to take them
/// (not created, or not placed yet).
#[tauri::command]
pub fn toggle_maximize_window(window: WebviewWindow) -> tauri::Result<()> {
    if window.is_maximized()? {
        window.unmaximize()
    } else {
        window.maximize()
    }
}

/// The logo's click: the window menu, as Alt+Space opens it (window design spec, story 2).
#[tauri::command]
pub fn show_window_menu(window: WebviewWindow) -> tauri::Result<()> {
    Ok(platform::show_window_menu(&window)?)
}

/// The file name's click: shows the calling window's document selected in its folder (window
/// design ticket 04), or does nothing while the window shows none.
///
/// The page passes nothing: the document comes from the app's own state, so the page can't have
/// another file revealed. Revealing looks at the file system, and a network share connects only
/// now, on a blocking thread, so the window keeps answering.
#[tauri::command]
pub async fn reveal_document(
    window: WebviewWindow,
    documents: State<'_, Documents>,
) -> Result<(), RevealError> {
    let Some(document) = documents.document_path(window.label()) else {
        return Ok(());
    };
    tauri::async_runtime::spawn_blocking(move || reveal(&document))
        .await
        .map_err(|source| RevealError::Stopped { source })?
}

/// Shows the file at `path` selected in its folder. The file is looked at first: Windows' reveal
/// fails on a missing file without saying why, and the reader is to learn that the file is gone
/// rather than that Explorer failed.
fn reveal(path: &Path) -> Result<(), RevealError> {
    reveal_with(path, platform::reveal_file)
}

/// [`reveal`] with the OS's reveal passed in, so a test can give the errors only an offline share
/// gives.
fn reveal_with(
    path: &Path,
    reveal_file: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), RevealError> {
    if let Err(error) = std::fs::metadata(path)
        && platform::is_missing(&error)
    {
        return Err(RevealError::NotFound {
            path: path.to_owned(),
        });
    }
    // The look is the only one to say the file is missing: the reveal fails as not found on an
    // offline share too, where the file may well exist (`links::path_kind`).
    reveal_file(path).map_err(|source| RevealError::Reveal {
        path: path.to_owned(),
        source,
    })
}

/// Why the document's file wasn't shown in its folder. The page shows the message as a notice.
#[derive(Debug)]
pub enum RevealError {
    /// The file was deleted or moved.
    NotFound {
        path: PathBuf,
    },
    Reveal {
        path: PathBuf,
        source: io::Error,
    },
    /// The background task that reveals the file panicked or was cancelled.
    Stopped {
        source: tauri::Error,
    },
}

impl fmt::Display for RevealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(
                f,
                "Can't find {}, so it can't be shown in its folder.",
                path.display()
            ),
            Self::Reveal { path, source } => {
                write!(f, "Can't show {} in its folder: {source}", path.display())
            }
            Self::Stopped { source } => {
                write!(f, "Showing the file in its folder stopped: {source}")
            }
        }
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as text.
impl serde::Serialize for RevealError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Tells the window's page, each time the window's size changes, whether it is maximized now, so
/// the page shows maximize or restore.
pub fn tell_window_state(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::Resized(_) = event
        && let Ok(maximized) = window.is_maximized()
    {
        // Sent on every resize, so a missed one is put right by the next.
        let _ = window
            .app_handle()
            .send(window.label(), Message::WindowState { maximized });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A reveal that succeeds opens an Explorer window, so it is a hand check (ticket 04).

    #[test]
    fn missing_document_is_not_revealed_and_the_reader_is_told_why() {
        let missing = std::env::temp_dir().join(format!(
            "downlight-title-bar-{}-missing.md",
            std::process::id()
        ));
        let error = reveal(&missing).err();
        assert!(
            matches!(&error, Some(RevealError::NotFound { path }) if *path == missing),
            "{error:?}"
        );
        assert_eq!(
            error.map(|error| error.to_string()),
            Some(format!(
                "Can't find {}, so it can't be shown in its folder.",
                missing.display()
            ))
        );
    }

    #[test]
    fn file_the_reveal_finds_nothing_at_is_not_told_as_missing() {
        // On an offline share, looking at the file can't tell whether it is there, and Windows'
        // reveal then finds no shell item for it or fails with ERROR_BAD_NETPATH (53), both of
        // which Rust counts as not found. The file may well exist (`links::path_kind`).
        let there = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        for failure in [
            io::Error::new(io::ErrorKind::NotFound, "Windows has no shell item for it"),
            io::Error::from_raw_os_error(53),
        ] {
            let error = reveal_with(&there, |_| Err(failure)).err();
            assert!(
                matches!(&error, Some(RevealError::Reveal { path, .. }) if *path == there),
                "{error:?}"
            );
        }
    }

    #[test]
    fn reveal_error_message_names_the_file_and_why() {
        let error = RevealError::Reveal {
            path: PathBuf::from(r"C:\docs\a.md"),
            source: io::Error::other("Explorer didn't start."),
        };
        assert_eq!(
            error.to_string(),
            r"Can't show C:\docs\a.md in its folder: Explorer didn't start."
        );
    }
}
