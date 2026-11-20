//! Carrying out a show (ticket 28): opening the new window, titling a window given another
//! document, bringing a window to the front, and undoing the show when part of it fails. What is
//! done to a window itself goes through the [`Windows`] port, beside what its page is told
//! ([`Pages`]), so a whole show is tested without a webview. Kept apart from the table, which only
//! records who shows what, since the port grows with what a new window starts with (ticket 16).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use super::place::{self, Area, Point, WindowPlace, WindowSize};
use super::{Claimed, Documents, Given, Showing, WindowState};
use crate::APP_NAME;
use crate::message::Message;
#[cfg(doc)]
use crate::message::Pages;
use crate::notice::file_name_shown;
use crate::settings::{AccentScale, PageSettings, WindowLook};

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

    /// Where `window` is, for a new window to open beside it and at its size (ticket 16). None
    /// for a window that has closed or isn't built yet, and for one that is minimized, maximized
    /// or full screen, which isn't at a place and size of its own.
    fn place_of(&self, window: &str) -> Option<WindowPlace>;

    /// The work area of the main monitor, in its logical pixels: where the system places a new
    /// window when there is no active window to open beside (ticket 16's review). None when the
    /// system names no main monitor.
    fn primary_work_area(&self) -> Option<Area>;

    /// Gives `window` the reader's theme and backdrop (window design ticket 08). A window that
    /// has closed, or is still being built, gets nothing, and that is no failure: one being built
    /// gets the look when its page first asks for its document
    /// ([`Documents::page_hears_settings`]).
    ///
    /// It must never wait for the main thread: it is called while the settings are held, and
    /// the main thread takes them as a window closes ([`Documents::closing`]), so a getter here
    /// would deadlock against a closing window. Tauri's setters only post to the event loop.
    fn set_look(&self, window: &str, look: &WindowLook) -> Result<(), WindowError>;

    /// Windows' accent scale as the system tells it now (window design ticket 09), read afresh
    /// each time. None when the OS has no accent color, and when a read fails. Called while the
    /// settings are held, so, as with [`Windows::set_look`], it must never wait for the main
    /// thread.
    fn accent_scale(&self) -> Option<AccentScale>;
}

/// What a new window starts with that the documents module decides: its title, the settings its
/// page starts with, its theme and backdrop, and its size and place (ticket 16). What every window starts with alike
/// (the page and its flags, the navigation guard, the backdrop, the key setup and the 5 s
/// fallback for showing it) is decided in one place, `open_window` in `window.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewWindow {
    /// What Alt+Tab and the taskbar show.
    pub title: String,
    /// What its page shows of the settings from its start, in its URL.
    pub settings: PageSettings,
    /// The reader's theme and backdrop, which the window is built with (window design ticket
    /// 08).
    pub look: WindowLook,
    /// The size of the last window that closed, or the default size until one has.
    pub size: WindowSize,
    /// Where its top left corner goes, in the logical pixels of the active window's monitor:
    /// down and to the right of the active window, on that monitor's screen. None when there is
    /// no active window to open beside, and the system places it.
    pub at: Option<Point>,
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
                self.change(&window, claim, left)
                    .map_err(ShowError::NotTaken)?;
                self.watch(&window, path, key);
                Ok(Given::Changed(window))
            }
            Claimed::AlreadyHere { window, stop } => {
                self.stop_the_load(&window, stop);
                self.bring_to_front(&window)?;
                Ok(Given::AlreadyHere(window))
            }
            Claimed::OnItsWayHere(window) => {
                self.bring_to_front(&window)?;
                Ok(Given::OnItsWayHere(window))
            }
            Claimed::AlreadyElsewhere {
                window,
                stop,
                scroll_to,
            } => {
                self.stop_the_load(&window, stop);
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
    /// shows none), at the settings' zoom and the size of the last window that closed, beside the
    /// active window (ticket 16), or forgets it if it can't open, so that its document opens anew.
    pub(super) fn open(&self, window: &str, document: Option<&Path>) -> Result<(), WindowError> {
        let settings = self.settings();
        let active = self.lock().active.clone();
        let active = active.and_then(|active| self.inner.windows.place_of(&active));
        let (size, at) = place::size_and_place(settings.window_size, active.as_ref(), || {
            self.inner.windows.primary_work_area()
        });
        let new_window = NewWindow {
            title: title_of(document),
            settings: self.page_settings(&settings),
            look: settings.window_look(),
            size,
            at,
        };
        self.inner
            .windows
            .open(window, &new_window)
            .inspect_err(|_| self.forget(window))
    }

    /// Titles `window` after the document it was just given by `claim`, and tells its page to ask
    /// for it. The title goes first, so that a page told can't show the new document under the
    /// old title. If either fails, the window is what it was before again, and titled after it,
    /// so the table, the title and the page agree again.
    ///
    /// `left` is the document on its way that the claim left, if one was: it goes once the
    /// change is made, which leaves its render, and is put back on its way if it fails.
    fn change(
        &self,
        window: &str,
        claim: u64,
        left: Option<Box<WindowState>>,
    ) -> Result<(), WindowError> {
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
    pub(super) fn retitle(&self, window: &str) -> Result<(), WindowError> {
        let _titling = self
            .inner
            .titles
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let Some(document) = self
            .lock()
            .windows
            .get(window)
            .map(|record| record.now.shown.as_ref().map(|shown| shown.path.clone()))
        else {
            return Ok(());
        };
        self.inner
            .windows
            .set_title(window, &title_of(document.as_deref()))
    }

    /// Makes `window` what it was before `claim` gave it a document: the document its page's view
    /// shows, which it held until the document arrived, and its history, with `left` on its way
    /// again if the claim left one. Unless another claim has changed it since, or it closed. By
    /// the claim rather than the showing, since a step gives a window back a showing it had, so a
    /// later step could have given it the same one.
    fn put_back(&self, window: &str, claim: u64, left: Option<Box<WindowState>>) {
        // Dropped after the lock, since dropping a watch stops it.
        let _dropped = {
            let mut table = self.lock();
            match table.windows.get_mut(window) {
                Some(record)
                    if record
                        .now
                        .shown
                        .as_ref()
                        .is_some_and(|shown| shown.claim == claim) =>
                {
                    let given = record.leave_the_arriving();
                    if let Some(left) = left {
                        // On its way again, with the document in view held behind it.
                        record.before_arrival = Some(std::mem::replace(&mut record.now, *left));
                    }
                    given
                }
                _ => left.map(|left| *left),
            }
        };
    }

    /// Tells `window`'s page to stop its next document on its way, if `stop` names the showing
    /// its view shows (ticket 38's review): the reader asked for the document in view. The page
    /// stops it as Stop does, so the window and its page agree on which document arrived.
    fn stop_the_load(&self, window: &str, stop: Option<Showing>) {
        if let Some(showing) = stop {
            // The window is brought to the front all the same, and a page that can't be told
            // shows its next document when it arrives, as without the request.
            let _ = self.send(window, Message::StopDocument(showing));
        }
    }

    fn bring_to_front(&self, window: &str) -> Result<(), ShowError> {
        self.inner
            .windows
            .bring_to_front(window)
            .map_err(ShowError::NotBroughtToFront)
    }
}
