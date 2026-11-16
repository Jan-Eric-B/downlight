//! Carrying out a show (ticket 28): opening the new window, titling a window given another
//! document, bringing a window to the front, and undoing the show when part of it fails. What is
//! done to a window itself goes through the [`Windows`] port, beside what its page is told
//! ([`Pages`]), so a whole show is tested without a webview. Kept apart from the table, which only
//! records who shows what, since the port grows with what a new window starts with (ticket 16).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use super::{Claimed, Documents, Given, WindowRecord};
use crate::APP_NAME;
use crate::message::Message;
#[cfg(doc)]
use crate::message::Pages;
use crate::notice::file_name_shown;

/// What is done to windows themselves, beside what their pages are told ([`Pages`]): Tauri's in
/// the app (`window.rs`), a recorder in the tests. A trait with two adapters rather than code
/// generic over Tauri's runtime, as ticket 22 decided for [`Pages`].
pub trait Windows: Send + Sync + 'static {
    /// Opens a new window with the label `window`, as `new_window` says. It stays hidden until
    /// its page has drawn its first frame.
    fn open(&self, window: &str, new_window: &NewWindow) -> Result<(), WindowError>;

    /// Sets the title Alt+Tab and the taskbar show for `window`. A window that has closed gets
    /// nothing, and that is no failure.
    fn set_title(&self, window: &str, title: &str) -> Result<(), WindowError>;

    /// Brings `window` to the front, restored if it was minimized. A window that has closed, or
    /// is still being built or hidden, gets nothing, and that is no failure: a hidden one comes up
    /// in front when its page asks to be shown.
    fn bring_to_front(&self, window: &str) -> Result<(), WindowError>;
}

/// What a new window starts with that depends on its document: its title. What every window
/// starts with alike (the page and its flags, the navigation guard, the backdrop, the key setup
/// and the 5 s fallback for showing it) is decided in one place, `open_window` in `window.rs`.
/// Ticket 16 adds the size and the offset here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewWindow {
    /// What Alt+Tab and the taskbar show.
    pub title: String,
}

/// Why the system couldn't open, change or tell a window or its page, in its own words: what
/// [`Pages`] and [`Windows`] fail with. The reader sees it after what failed ([`ShowError`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowError(String);

impl WindowError {
    /// For the recorder, which fails as a window or page may.
    #[cfg(test)]
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<tauri::Error> for WindowError {
    fn from(error: tauri::Error) -> Self {
        Self(error.to_string())
    }
}

/// Why a document isn't shown, in the reader's terms (ticket 28). The reader sees it after the
/// document's name, as in `Can't show a.md in C:\docs: its window has closed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowError {
    /// The window it was to be shown in closed first.
    WindowClosed,
    /// A new window for it didn't open, and it opens anew next time.
    NotOpened(WindowError),
    /// The window's title or page couldn't be changed to it, so the window keeps the document it
    /// showed (ticket 22's open item 6).
    NotTaken(WindowError),
    /// The window that shows it already didn't come to the front.
    NotBroughtToFront(WindowError),
}

impl fmt::Display for ShowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowClosed => write!(f, "its window has closed"),
            Self::NotOpened(error) => write!(f, "its window didn't open: {error}"),
            Self::NotTaken(error) => write!(f, "the window couldn't change to it: {error}"),
            Self::NotBroughtToFront(error) => {
                write!(f, "its window didn't come to the front: {error}")
            }
        }
    }
}

/// `README.md - docs - Downlight`: the folder tells documents with the same name apart in Alt+Tab
/// and on the taskbar. A window that shows no document (None) is titled `Downlight`.
pub(super) fn title_of(document: Option<&Path>) -> String {
    let Some(document) = document else {
        return APP_NAME.to_owned();
    };
    // The same name as the title bar's.
    let file = file_name_shown(document);
    // Launches, the open dialog and dropping all give absolute paths, so only a drive root has no
    // folder.
    match document.parent() {
        Some(folder) => format!("{file} - {} - {APP_NAME}", file_name_shown(folder)),
        None => format!("{file} - {APP_NAME}"),
    }
}

impl Documents {
    /// Carries out what a claim recorded for the document at `path`, with `key`, as
    /// [`Documents::give`] describes, for a show and a step alike.
    pub(super) fn carry_out(
        &self,
        claimed: Claimed,
        path: &Path,
        key: PathBuf,
    ) -> Result<Given, ShowError> {
        match claimed {
            Claimed::NewWindow(window) => {
                self.open(&window, Some(path))
                    .map_err(ShowError::NotOpened)?;
                self.watch(&window, path, key);
                Ok(Given::NewWindow(window))
            }
            Claimed::Changed {
                window,
                claim,
                left,
            } => {
                self.change(&window, claim, *left)
                    .map_err(ShowError::NotTaken)?;
                self.watch(&window, path, key);
                Ok(Given::Changed(window))
            }
            Claimed::AlreadyHere(window) => {
                self.bring_to_front(&window)?;
                Ok(Given::AlreadyHere(window))
            }
            Claimed::AlreadyElsewhere { window, scroll_to } => {
                if let Some((showing, heading)) = scroll_to {
                    // The window that shows the document is brought to the front all the same,
                    // and the page that asked shows only what went wrong with its own request, so
                    // a scroll that can't be sent there is nobody's to tell.
                    let _ = self.send(&window, Message::ScrollToHeading { showing, heading });
                }
                self.bring_to_front(&window)?;
                Ok(Given::AlreadyElsewhere(window))
            }
            Claimed::WindowGone => Err(ShowError::WindowClosed),
        }
    }

    /// Opens the window `window` just recorded, titled after `document` (None for a window that
    /// shows none), or forgets it if it can't open, so that its document opens anew.
    pub(super) fn open(&self, window: &str, document: Option<&Path>) -> Result<(), WindowError> {
        let new_window = NewWindow {
            title: title_of(document),
        };
        self.inner
            .windows
            .open(window, &new_window)
            .inspect_err(|_| self.forget(window))
    }

    /// Titles `window` after the document it was just given by `claim`, and tells its page to ask
    /// for it. The title goes first, so that a page told can't show the new document under the
    /// old title. If either fails, the window is `left` again, what it was before, and titled
    /// after it, so the table, the title and the page agree again.
    fn change(&self, window: &str, claim: u64, left: WindowRecord) -> Result<(), WindowError> {
        let changed = self
            .retitle(window)
            .and_then(|()| self.send(window, Message::AskForDocument));
        if changed.is_err() {
            self.put_back(window, claim, left);
            // The error that failed the change is the one the reader is told; a title that can't
            // be put back either stays as wrong as the failure left it.
            let _ = self.retitle(window);
        }
        changed
    }

    /// Titles `window` after the document it shows now, or `Downlight` while it shows none; a
    /// closed window gets nothing. Every change of a window's document is followed by this, rather
    /// than by a title for the document it was given: two shows into one window, as two quick
    /// Back presses or a link followed during Ctrl+O, are carried out side by side, and a title
    /// worked out ahead could land after the other show's. Under the titles lock, each titling
    /// reads the window's document and sets its title before the next one reads, and the last
    /// titling comes after the last change, so the title the window is left with names the
    /// document it shows.
    fn retitle(&self, window: &str) -> Result<(), WindowError> {
        let _titling = self
            .inner
            .titles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let Some(document) = self
            .lock()
            .windows
            .get(window)
            .map(|record| record.shown.as_ref().map(|shown| shown.path.clone()))
        else {
            return Ok(());
        };
        self.inner
            .windows
            .set_title(window, &title_of(document.as_deref()))
    }

    /// Makes `window` `left` again, what it was before `claim` gave it a document: the document it
    /// showed and its history. Unless another claim has changed it since, or it closed. By the
    /// claim rather than the showing, since a step gives a window back a showing it had, so a
    /// later step could have given it the same one.
    fn put_back(&self, window: &str, claim: u64, left: WindowRecord) {
        // Dropped after the lock, since dropping a watch stops it.
        let _dropped = {
            let mut table = self.lock();
            match table.windows.get_mut(window) {
                Some(record)
                    if record
                        .shown
                        .as_ref()
                        .is_some_and(|shown| shown.claim == claim) =>
                {
                    std::mem::replace(record, left)
                }
                _ => left,
            }
        };
    }

    fn bring_to_front(&self, window: &str) -> Result<(), ShowError> {
        self.inner
            .windows
            .bring_to_front(window)
            .map_err(ShowError::NotBroughtToFront)
    }
}
