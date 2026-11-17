//! The document each window shows (ticket 22): which window shows which document, and giving a
//! window its document, the one way a window gets or changes one. A document is shown in at most
//! one window (ADR 0001). Reading and rendering the file is the render module's.
//!
//! This file holds the table and the module's interface. The rest is split by feature, as the
//! tests are:
//! - [`show`]: carrying out a show (ticket 28), opening, titling and bringing windows to the
//!   front through the [`Windows`] port, and undoing a show when part of it fails. What a
//!   window's page is told goes through [`Pages`], so all of this is tested without a webview.
//! - [`handover`]: rendering the document ahead, and answering the page's request for it with
//!   whether to show or reload it, with the heading and a notice held until the page asks (ticket
//!   23, [`handover::render_document`]).
//! - [`watching`]: watching the document's file, which reloads it while the window still shows
//!   it (ticket 10).
//! - [`history`]: the documents a window showed before and after the one it shows, which Back and
//!   Forward give it back (ticket 12).
//!
//! It also changes the reader's settings, since they shape every new window and reach every page
//! when they change ([`Documents::change_settings`], window design ticket 12), and keeps the window
//! the reader was last in, which a new window opens beside, at a size and place [`place`] works
//! out (ticket 16).
//!
//! A window's document, its render, its watch and its history are forgotten with the window.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::message::{Message, Pages};
use crate::notice::Notice;
use crate::platform;
use crate::render::{self, RenderTask};
use crate::settings::{PageSettings, Settings, SettingsError, SettingsStore};
use crate::watch::{Stamp, Watch};

pub mod handover;
pub mod history;
mod place;
#[cfg(test)]
pub mod recorder;
mod show;
mod watching;

#[cfg(test)]
pub use handover::{contract_samples, wait_note_samples};
use history::History;
pub use place::{Area, Point, WindowPlace, WindowSize};
pub use show::{NewWindow, ShowError, WindowError, Windows};

/// Which window shows which document, by window label. The app's managed state.
pub struct Documents {
    inner: Arc<Inner>,
}

struct Inner {
    table: Mutex<WindowTable>,
    /// Held while a window is titled ([`Documents::retitle`]), so that titles are set in the order
    /// their documents were read from the table. Apart from the table's lock, which no OS call is
    /// made under.
    titles: Mutex<()>,
    pages: Box<dyn Pages>,
    windows: Box<dyn Windows>,
    /// The reader's settings, which shape every new window and which every page is sent when they
    /// change ([`Documents::change_settings`], the one way they change). Shared with Tauri's
    /// managed state, where the app's other parts read them.
    settings: Arc<SettingsStore>,
}

/// Every open window by label, whether it shows a document or none.
#[derive(Default)]
struct WindowTable {
    /// Labels are `document-0`, `document-1` and so on, never reused while the process runs.
    labels_issued: usize,
    /// How many times a window has been given a document, for each to get its own [`Showing`].
    showings_issued: u64,
    /// How many times a window has been given a document or taken a step, for each to get its own
    /// claim number ([`ShownDocument::claim`]).
    claims_issued: u64,
    /// Every open window, with the document it shows now and its history.
    windows: HashMap<String, WindowRecord>,
    /// The window the reader was last in, which a new window opens beside (ticket 16), while it
    /// is open.
    active: Option<String>,
    /// The windows whose pages haven't asked for their document yet, by label, with whether the
    /// settings changed meanwhile. Such a page may not hear a `settings` message yet, so it is
    /// sent the settings when it first asks ([`Documents::page_hears_settings`]); it asks only
    /// once it hears them (`page/src/main.ts`). Kept beside the window's record rather than in
    /// it, since a show that fails puts the record back whole, and the page's hearing doesn't go
    /// back with it.
    not_hearing: HashMap<String, bool>,
}

/// An open window: the document it shows now, None while it shows none, and its history. Kept
/// apart from the shown document so that the history outlives every change of document, and goes
/// only with the window.
#[derive(Default)]
struct WindowRecord {
    shown: Option<ShownDocument>,
    history: History,
}

/// One time a window was given a document. A window given the same document again, after another,
/// has a new one, and Back or Forward gives the window back the one it had (ticket 12), so that
/// its page returns to where it was. The page sends the one its view shows when it asks for its
/// document, so that [`Documents::handover`] can tell whether to show the document or reload it,
/// whichever message made the page ask (ticket 23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Showing(u64);

impl Showing {
    /// The showing the contract samples name, the same in every sample that has one.
    #[cfg(test)]
    pub fn sample() -> Self {
        Self(7)
    }
}

struct ShownDocument {
    /// This time the window was given the document.
    showing: Showing,
    /// The claim that gave the window the document, unique while the process runs, unlike the
    /// showing, which a step gives back: a show that fails puts the window back only while it is
    /// still at this claim ([`Documents::put_back`]).
    claim: u64,
    /// As the launch, the dialog or the drop named it, for reading and the title.
    path: PathBuf,
    /// For finding the window when the document is opened again. Worked out once, when the window
    /// is given the document: worked out again later it could differ, had the file been renamed,
    /// deleted or named by a short name in between, and the window would then drop its watch.
    key: PathBuf,
    /// The document's read and render, started when the window was given it and taken by its
    /// page's first request. A new window's page asks only once its webview has started, about
    /// 600 ms into a cold start, and each grammar costs up to 300 ms on its first use in the
    /// process (ticket 07), so the render runs while the webview starts. A reload starts a new
    /// one.
    render_ahead: Option<RenderTask>,
    /// The file's stamp from just before the render ahead read it, which the watch starts from
    /// (ticket 30); taken when the watch starts.
    read_stamp: Option<Receiver<Option<Stamp>>>,
    /// The watch on the document's file, which reloads it; None until it has started.
    /// Replacing or forgetting the entry drops it, which stops it.
    watch: Option<Watch>,
    /// Whether the page has asked for this document yet, and so listens for messages. A new
    /// window's page listens for nothing until its webview has started, about 600 ms after the
    /// window was given its document, so a message sent before then would be lost (ticket 10).
    page_asked: bool,
    /// A notice about this document, held for the page's first request. Held on the document's
    /// entry, it goes when the window is given another document.
    held_notice: Option<Notice>,
    /// How the window came to show the document, which says where its page shows it. Handed over
    /// with every show rather than taken by the first, so that of two links followed in quick
    /// succession, the page's latest request still gets the heading; never with a reload, which
    /// keeps the reading position.
    arrival: Arrival,
}

/// How a window came to show its document.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Arrival {
    /// A launch, the dialog, a drop or a link gave it, at this heading from a link like
    /// `guide.md#install` (ticket 11), or at its top without one.
    Given { heading: Option<String> },
    /// Back or Forward gave it back (ticket 12), so the page returns to where it was in it.
    Stepped,
}

impl ShownDocument {
    fn given(document: DocumentAt<'_>, key: PathBuf, showing: Showing, claim: u64) -> Self {
        let arrival = Arrival::Given {
            heading: document.heading.map(str::to_owned),
        };
        Self::arrived(document.path.to_owned(), key, showing, claim, arrival)
    }

    /// The document at `path`, with `key`, shown as `showing` by `claim` after `arrival`. Its
    /// render starts now.
    fn arrived(
        path: PathBuf,
        key: PathBuf,
        showing: Showing,
        claim: u64,
        arrival: Arrival,
    ) -> Self {
        let (render_ahead, read_stamp) = render::start_with_stamp(&path);
        Self {
            showing,
            claim,
            render_ahead: Some(render_ahead),
            read_stamp: Some(read_stamp),
            path,
            key,
            watch: None,
            page_asked: false,
            held_notice: None,
            arrival,
        }
    }

    fn start_reload(&mut self) {
        self.render_ahead = Some(render::start(&self.path));
    }
}

/// A document to show, and where in it.
#[derive(Debug, Clone, Copy)]
pub struct DocumentAt<'a> {
    /// As the launch, the dialog, the drop or the link named it.
    pub path: &'a Path,
    /// The heading to show it at, from a link like `guide.md#install` (ticket 11), or None.
    pub heading: Option<&'a str>,
}

impl<'a> DocumentAt<'a> {
    /// The document at `path`, without a heading: a window given it shows its top, and a window
    /// that shows it already keeps its reading position.
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            heading: None,
        }
    }
}

/// The window a document is to be shown in, which also says the window the request came from:
/// [`Documents::give`] tells that one apart ([`Given::AlreadyHere`]). Part of the target rather
/// than a field of [`DocumentAt`] or an argument of its own (ticket 24): a request for a window
/// always comes from that window, so only a Ctrl+click has to name where it came from, and the two
/// can never disagree or the one be left out.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A new window, as for a launch from the file manager or the command line, which comes from
    /// no window.
    NewWindow,
    /// A new window, for a link followed with Ctrl+click in the window with this label.
    NewWindowFrom(&'a str),
    /// The window with this label, as for Ctrl+O, dropping a file and following a link.
    Window(&'a str),
}

/// What [`Documents::give`] did with a document, by window label.
#[derive(Debug, PartialEq, Eq)]
pub enum Given {
    /// A new window with this label opened, titled after the document, and shows it.
    NewWindow(String),
    /// The window shows the document now: it is titled after it, and its page has been told to
    /// ask for it.
    Changed(String),
    /// The document is open already in this window, the one the request came from ([`Target`]),
    /// as for a link to the document that window shows. It is brought to the front and its page
    /// is told nothing: it scrolls on the link's own result.
    AlreadyHere(String),
    /// The document is open already in this window, another than the one the request came from,
    /// or any window for a launch, and it is brought to the front. Its page has been told to
    /// scroll to the document's heading, if the document came with one and the page listens; a
    /// page that doesn't yet gets it with its first render.
    AlreadyElsewhere(String),
}

/// What [`Documents::claim`], or a step's claim, recorded under the table's lock, for
/// [`Documents::carry_out`] to carry out after it.
enum Claimed {
    /// A new window with this label is to open.
    NewWindow(String),
    /// `window` shows the document now, by `claim`, and is to be retitled and its page told;
    /// `left`, what the window was before, is put back if that fails.
    Changed {
        window: String,
        claim: u64,
        left: Box<WindowRecord>,
    },
    /// The document is open already in this window, the one the request came from.
    AlreadyHere(String),
    /// The document is open already in `window`, another than the one the request came from.
    /// Its page is to scroll to `scroll_to` now, if that is Some: the showing the window shows the
    /// document as, and the heading.
    AlreadyElsewhere {
        window: String,
        scroll_to: Option<(Showing, String)>,
    },
    /// The target window has closed.
    WindowGone,
}

impl Documents {
    /// No window shows a document yet; what pages are told goes to `pages`, what is done to
    /// windows to `windows`, and new windows open with `settings`.
    pub fn new(pages: impl Pages, windows: impl Windows, settings: Arc<SettingsStore>) -> Self {
        Self {
            inner: Arc::new(Inner {
                table: Mutex::default(),
                titles: Mutex::default(),
                pages: Box::new(pages),
                windows: Box::new(windows),
                settings,
            }),
        }
    }

    /// The reader's settings now.
    pub fn settings(&self) -> Settings {
        self.inner.settings.get()
    }

    /// Changes the settings with `change` and saves them, and sends every window's page the
    /// settings it shows if they changed (window design ticket 12), the one way a setting
    /// changes. The pages are told while the settings are still held, so that of two changes at
    /// once, they are told last what the settings hold last. A change that can't be saved holds
    /// until Downlight closes all the same, and the error says why it wasn't saved.
    pub fn change_settings(&self, change: impl FnOnce(&mut Settings)) -> Result<(), SettingsError> {
        self.inner
            .settings
            .update(change, |updated| {
                let shown = updated.settings.page();
                if updated.before.page() != shown {
                    self.send_settings_to_every_page(shown);
                }
            })
            .saved
    }

    /// Sends every page that hears the settings `settings`, in the order of their labels, and
    /// keeps that the others missed them, for when they first ask
    /// ([`Documents::page_hears_settings`]).
    fn send_settings_to_every_page(&self, settings: PageSettings) {
        let mut hearing = {
            let mut table = self.lock();
            let table = &mut *table;
            let mut hearing = Vec::new();
            for window in table.windows.keys() {
                match table.not_hearing.get_mut(window) {
                    Some(missed) => *missed = true,
                    None => hearing.push(window.clone()),
                }
            }
            hearing
        };
        hearing.sort();
        for window in hearing {
            // A page that can't be told is one whose webview failed to run an event, which the
            // window that made the change can't mend. What is sent to every window is a whole
            // snapshot, so that page catches up with the next one.
            let _ = self.send(&window, Message::Settings(settings));
        }
    }

    /// `window`'s page has asked for its document, which it does only once it hears the settings
    /// (`page/src/main.ts`), so it is sent every change from now on, and the settings now if they
    /// changed before (ticket 16's review): a window that opened while they changed has the
    /// settings from before the change in its page's URL, and missed the message. Sent while the
    /// settings are held, so that a change after it reaches the page after it.
    fn page_hears_settings(&self, window: &str) {
        let missed = self.lock().not_hearing.remove(window) == Some(true);
        if missed {
            self.inner.settings.read(|settings| {
                // As in `send_settings_to_every_page`, the next change catches the page up.
                let _ = self.send(window, Message::Settings(settings.page()));
            });
        }
    }

    /// The reader is in `window` now, so the next new window opens beside it (ticket 16), and its
    /// document's file is checked again, so a change its watch missed reloads it, or shows that
    /// it is gone, at the moment the reader looks (ticket 30).
    pub fn activated(&self, window: &str) {
        let mut table = self.lock();
        let Some(record) = table.windows.get(window) else {
            return;
        };
        if let Some(watch) = record.shown.as_ref().and_then(|shown| shown.watch.as_ref()) {
            watch.recheck();
        }
        table.active = Some(window.to_owned());
    }

    /// `window` is closing: its size is kept in the settings, for the next new window to open at
    /// (ticket 16). Called while the window is still there to measure. A minimized or maximized
    /// window keeps the size of the last one that closed before it, since it isn't at a size of
    /// its own.
    pub fn closing(&self, window: &str) {
        let Some(place) = self.inner.windows.place_of(window) else {
            return;
        };
        // The window is closing, so there is no page left to tell when the size can't be saved;
        // it holds for the windows that open until Downlight closes all the same.
        let _ = self.change_settings(|settings| settings.window_size = Some(place.size));
    }

    /// Opens a new window that shows no document, as for a launch that names none, and returns
    /// its label. A window that can't open is forgotten.
    pub fn open_empty(&self) -> Result<String, WindowError> {
        let window = self.lock().record_new(WindowRecord::default());
        self.open(&window, None)?;
        Ok(window)
    }

    /// Shows the document in `target`, unless the document is open already, the one way a window
    /// gets or changes its document. Looking up and recording happen under one lock, so two
    /// requests for the same document end up in one window.
    ///
    /// A new window opens titled after the document, and a window given another document gets
    /// that document's title and its page is told to ask for it. The document's render starts as
    /// soon as a window is given it, so that it runs while a new window's webview starts, and the
    /// window starts watching the document's file, to reload it when it changes.
    ///
    /// A document that is open already has its window brought to the front instead, even the
    /// window the request came from: dropping a file onto a window leaves the file manager in
    /// front. If the document came with a heading, that window scrolls there: told now if its
    /// page listens, or with its first render if it doesn't yet. The window the request came from
    /// ([`Target`]) is never told ([`Given::AlreadyHere`]): its page scrolls on the link's own
    /// result.
    ///
    /// Fails when the target window has closed, or a part of the show fails. A show that fails is
    /// undone: a new window that can't open is forgotten, so its document opens anew, and a window
    /// whose title or page can't be changed keeps the document it showed, and its history, so the
    /// table and the window agree (ticket 22's open item 6).
    ///
    /// A window given another document keeps the one it showed as its last step back, and has
    /// nothing to go forward to any more (ticket 12). Given a document it showed before, it shows
    /// it anew, as another showing, at its top or heading: only Back and Forward return to where
    /// it was.
    pub fn give(&self, document: DocumentAt<'_>, target: Target<'_>) -> Result<Given, ShowError> {
        let key = document_key(document.path);
        let claimed = self.claim(document, key.clone(), target);
        self.carry_out(claimed, document.path, key)
    }

    /// Records that `target` shows `document` with `key`, and what that changed, including a
    /// scroll for the window of an open document, which [`Documents::give`] sends outside the
    /// lock. Whether the request came from the window that shows the document is decided here,
    /// and only here.
    ///
    /// The window of an open document is to scroll to the document's heading, if it has one,
    /// unless the request came from that window. A page that hasn't asked for its document yet
    /// may not be listening, so it gets the heading with its first render instead, the way a
    /// notice is held (ticket 10). A page that has asked may still be placing the render, so the
    /// scroll names the showing, which the page waits to have in view.
    fn claim(&self, document: DocumentAt<'_>, key: PathBuf, target: Target<'_>) -> Claimed {
        let mut table = self.lock();
        if let Some(window) = table.window_showing(&key) {
            if matches!(target, Target::Window(from) | Target::NewWindowFrom(from) if from == window)
            {
                return Claimed::AlreadyHere(window);
            }
            let scroll_to = table.shown_mut(&window, &key).and_then(|shown| {
                let heading = document.heading?.to_owned();
                if shown.page_asked {
                    return Some((shown.showing, heading));
                }
                shown.arrival = Arrival::Given {
                    heading: Some(heading),
                };
                None
            });
            return Claimed::AlreadyElsewhere { window, scroll_to };
        }
        // Made only once a window takes the document, so that an open document or a closed
        // window starts no render.
        match target {
            Target::NewWindow | Target::NewWindowFrom(_) => {
                let (showing, claim) = (table.next_showing(), table.next_claim());
                let shown = ShownDocument::given(document, key, showing, claim);
                let window = table.record_new(WindowRecord {
                    shown: Some(shown),
                    history: History::default(),
                });
                Claimed::NewWindow(window)
            }
            Target::Window(window) => {
                let (showing, claim) = (table.next_showing(), table.next_claim());
                let Some(record) = table.windows.get_mut(window) else {
                    return Claimed::WindowGone;
                };
                let left = record.given(ShownDocument::given(document, key, showing, claim));
                Claimed::Changed {
                    window: window.to_owned(),
                    claim,
                    left: Box::new(left),
                }
            }
        }
    }

    /// The file of the document `window` shows, as the launch, the dialog, the drop or the link
    /// named it, or None while it shows none. The one way to learn a window's document: commands
    /// take it from here, never from the page.
    pub fn document_path(&self, window: &str) -> Option<PathBuf> {
        self.lock()
            .windows
            .get(window)?
            .shown
            .as_ref()
            .map(|shown| shown.path.clone())
    }

    /// The file of the document `window` shows, while it still shows it as `showing`; None once
    /// the window has been given another document, or closed. For a link, which comes from the
    /// showing its page's view showed when it was clicked (ticket 26): a link in a document the
    /// window has left would resolve against the new document's folder.
    pub fn document_path_shown_as(&self, window: &str, showing: Showing) -> Option<PathBuf> {
        self.lock()
            .windows
            .get(window)?
            .shown
            .as_ref()
            .filter(|shown| shown.showing == showing)
            .map(|shown| shown.path.clone())
    }

    /// Forgets a window that closed or never opened, so its document opens anew. A render started
    /// for it, the watch on its document and its history go with it.
    pub fn forget(&self, window: &str) {
        // Dropped after the lock, since dropping a watch stops it.
        let _forgotten = {
            let mut table = self.lock();
            if table.active.as_deref() == Some(window) {
                table.active = None;
            }
            table.not_hearing.remove(window);
            table.windows.remove(window)
        };
    }

    /// Sends `window`'s page `message`. Every message a page gets goes through here, also those
    /// from outside this module (a drop's notice, a menu choice, the window's state), so the
    /// tests see each one. Sent outside the table's lock, so that no page is told anything while
    /// the table is held.
    pub fn send(&self, window: &str, message: Message) -> Result<(), WindowError> {
        self.inner.pages.send(window, message)
    }

    fn lock(&self) -> MutexGuard<'_, WindowTable> {
        // The map stays valid even if a thread panicked while holding the lock.
        self.inner
            .table
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// What two paths to the same document have in common: the path with links, `..` and short names
/// resolved, in the case the file system compares names in. A missing file keeps its name as
/// given, in its resolved folder; with its folder missing too, the path stays as given, absolute
/// already, as launches, the open dialog and dropping all give it.
fn document_key(document: &Path) -> PathBuf {
    let resolved = std::fs::canonicalize(document)
        .or_else(|_| {
            let (Some(folder), Some(name)) = (document.parent(), document.file_name()) else {
                return Err(io::ErrorKind::NotFound.into());
            };
            std::fs::canonicalize(folder).map(|folder| folder.join(name))
        })
        .unwrap_or_else(|_: io::Error| document.to_owned());
    platform::fold_case(resolved)
}

impl WindowTable {
    /// What `window` shows, while it shows the document with `key`.
    fn shown_mut(&mut self, window: &str, key: &Path) -> Option<&mut ShownDocument> {
        self.windows
            .get_mut(window)
            .and_then(|record| record.shown.as_mut())
            .filter(|shown| shown.key == key)
    }

    fn window_showing(&self, key: &Path) -> Option<String> {
        self.windows.iter().find_map(|(label, record)| {
            record
                .shown
                .as_ref()
                .is_some_and(|shown| shown.key == key)
                .then(|| label.clone())
        })
    }

    /// Records a new window as `record`, whose page hears nothing yet, and gives its label.
    fn record_new(&mut self, record: WindowRecord) -> String {
        let window = format!("document-{}", self.labels_issued);
        self.labels_issued += 1;
        self.windows.insert(window.clone(), record);
        self.not_hearing.insert(window.clone(), false);
        window
    }

    fn next_showing(&mut self) -> Showing {
        let showing = Showing(self.showings_issued);
        self.showings_issued += 1;
        showing
    }

    fn next_claim(&mut self) -> u64 {
        let claim = self.claims_issued;
        self.claims_issued += 1;
        claim
    }
}

#[cfg(all(test, windows))]
mod tests;
