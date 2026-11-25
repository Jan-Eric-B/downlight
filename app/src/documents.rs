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
//! - [`pasted_files`]: deleting each pasted file the table lets go of once no window holds it, and
//!   emptying the pasted folder at startup (paste-markdown ticket 02).
//! - [`settings`]: changing the reader's settings, since they shape every new window and reach
//!   every page when they change ([`Documents::change_settings`], window design ticket 12).
//! - [`shown_bytes`]: what a window's page's view shows, and the bytes of the file its render
//!   was made from, which Copy as Markdown copies (copy-as-markdown ticket 01).
//!
//! It also keeps the window the reader was last in, which a new window opens beside, at a size and
//! place [`place`] works out (ticket 16).
//!
//! A window's document, its render, its watch and its history are forgotten with the window.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::message::{Message, Pages};
use crate::notice::Notice;
use crate::paste::PastedFolder;
use crate::platform;
use crate::render::{self, RenderTask};
use crate::settings::{AccentScale, Settings, SettingsStore};
use crate::watch::{Change, ReadStamp, Watch};

pub mod arrival;
pub mod handover;
pub mod history;
mod pasted_files;
mod place;
#[cfg(test)]
pub mod recorder;
mod settings;
mod show;
mod shown_bytes;
mod watching;

#[cfg(test)]
pub use handover::{contract_samples, wait_note_samples};
use history::History;
use pasted_files::Pastes;
pub use place::{Area, Point, WindowPlace, WindowSize};
pub use show::{NewWindow, ShowError, WindowError, Windows};
use shown_bytes::{Handed, View};

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
    /// managed state, where the app's other parts read them. Its lock is taken before the
    /// table's, never under it: a change tells the pages, which reads the table, while the
    /// settings are held.
    settings: Arc<SettingsStore>,
    /// Windows' accent scale now, which every page is sent with the settings (window design
    /// ticket 09); None where the OS has none. Changed only while the settings are held, and read
    /// under their lock too, so it reaches the pages in the same order as a change of the settings.
    windows_accent: Mutex<Option<AccentScale>>,
    /// The pasted folder, whose files the table deletes once it lets go of them (paste-markdown
    /// ticket 02).
    pastes: Pastes,
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
    /// How many times a page has asked for its document, for each request to get its own number
    /// ([`shown_bytes::Request`]).
    requests_issued: u64,
    /// Every open window, with the document it shows now and its history.
    windows: HashMap<String, WindowRecord>,
    /// The window the reader was last in, which a new window opens beside (ticket 16), while it
    /// is open.
    active: Option<String>,
    /// The window opened last and the place it was given, until the reader is in a window again
    /// or it is forgotten: the next new window opens beside it rather than the active window, so
    /// windows opened together, as a launch opens one for each document before the reader is in
    /// any, cascade instead of opening on top of each other (window design ticket 17's review).
    opened_last: Option<(String, WindowPlace)>,
    /// The windows whose pages haven't asked for their document yet, by label, with whether the
    /// settings changed meanwhile. Such a page may not hear a `settings` message yet, so it is
    /// sent the settings when it first asks ([`Documents::page_hears_settings`]); it asks only
    /// once it hears them (`page/src/main.ts`). Kept beside the window's record rather than in
    /// it, since a show that fails puts the record back whole, and the page's hearing doesn't go
    /// back with it.
    not_hearing: HashMap<String, bool>,
}

/// An open window: what it shows now, and, while its document is on its way, what its page's
/// view still shows. Kept apart from the shown document so that the history outlives every
/// change of document, and goes only with the window.
#[derive(Default)]
struct WindowRecord {
    /// The document the window shows now, the one on its way while one is, and its history.
    now: WindowState,
    /// What the window was before it was given the document it shows, while that document is
    /// still on its way: its page's view still shows the document held here (ticket 38). Stop
    /// puts it back ([`Documents::stop`]), going elsewhere starts from it, and it goes once the
    /// page says the document arrived ([`Documents::arrived`]), so a document joins the history
    /// only then.
    before_arrival: Option<WindowState>,
    /// What its page's view shows: the answer it placed last ([`Documents::arrived`]) and the bytes
    /// Copy as Markdown copies; None before the first, and once it shows the empty window. Kept
    /// apart from the shown document, as it follows what the page placed rather than what the
    /// window was given. It can be a document the window moved on from before it arrived, as when
    /// a drop comes while the page lays out a large one, so Stop checks the page's showing against
    /// this one rather than the one held.
    view: Option<View>,
}

/// A window's document, None while it shows none, and its history: what the window is now, or
/// was before its document on its way ([`WindowRecord`]).
#[derive(Default)]
struct WindowState {
    shown: Option<ShownDocument>,
    history: History,
}

impl WindowState {
    /// The document, while it is the one with `key`.
    fn document(&self, key: &Path) -> Option<&ShownDocument> {
        self.shown.as_ref().filter(|shown| shown.key == key)
    }

    /// The document, while it is the one with `key`.
    fn document_mut(&mut self, key: &Path) -> Option<&mut ShownDocument> {
        self.shown.as_mut().filter(|shown| shown.key == key)
    }
}

impl WindowRecord {
    /// The window as its page's view shows it: what it was before the document on its way, if
    /// one is, or else what it is.
    fn in_view(&self) -> &WindowState {
        self.before_arrival.as_ref().unwrap_or(&self.now)
    }

    /// The window's documents: the one it shows, and the one its page's view still shows while
    /// that is on its way.
    fn documents(&self) -> impl Iterator<Item = &ShownDocument> {
        [Some(&self.now), self.before_arrival.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|state| state.shown.as_ref())
    }

    /// Whether the window shows the document with `key`: as its document, or in its page's view
    /// while its next document is on its way.
    fn shows(&self, key: &Path) -> bool {
        self.documents().any(|shown| shown.key == key)
    }

    /// Whether the document with `key` is on its way to the window: given it, but not placed by
    /// its page yet.
    fn is_on_its_way(&self, key: &Path) -> bool {
        self.before_arrival.is_some() && self.now.document(key).is_some()
    }

    /// The showing of the document with `key`, while the window holds it in its page's view
    /// behind its next document on its way.
    fn held_showing(&self, key: &Path) -> Option<Showing> {
        Some(self.before_arrival.as_ref()?.document(key)?.showing)
    }

    /// The document with `key`, whether the window shows it or holds it in view behind its next
    /// document on its way.
    fn document_mut(&mut self, key: &Path) -> Option<&mut ShownDocument> {
        let Self {
            now,
            before_arrival,
            ..
        } = self;
        now.document_mut(key)
            .or_else(|| before_arrival.as_mut()?.document_mut(key))
    }

    /// Puts back what the window was before the document on its way, if one is, and gives what
    /// it leaves, the document on its way, for the caller to drop after the table's lock, since
    /// dropping a watch stops it.
    fn leave_the_arriving(&mut self) -> Option<WindowState> {
        let before = self.before_arrival.take()?;
        Some(std::mem::replace(&mut self.now, before))
    }

    /// Moves the window on to what `next` makes of the window in view, and holds what that was
    /// until the new document arrives. A document still on its way is left first, so the window
    /// moves on from the one in view, and the one on its way never joins the history (ticket
    /// 38); it is given back for the caller to drop after the table's lock, since dropping a
    /// watch stops it.
    ///
    /// A reload of the held document that its page hasn't placed yet is lost to the page, which
    /// drops it for the new document, so the held document reloads if it is put back.
    fn move_on(&mut self, next: impl FnOnce(&WindowState) -> WindowState) -> Option<WindowState> {
        let left = self.leave_the_arriving();
        let next = next(&self.now);
        let mut before = std::mem::replace(&mut self.now, next);
        if let Some(held) = before.shown.as_mut()
            && held.reload_unplaced
        {
            held.missed = Some(Change::Changed);
        }
        self.before_arrival = Some(before);
        left
    }
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
    read_stamp: Option<ReadStamp>,
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
    /// The render last handed to the page, and its bytes once it is done until the page places
    /// it. A later request for the document, or the window leaving it, leaves it (ticket 38): the
    /// page waits for the latest one only. The render ahead leaves itself when it is dropped.
    rendering: Option<Handed>,
    /// Whether the watch started a reload that the page hasn't placed yet
    /// ([`Documents::arrived`]).
    reload_unplaced: bool,
    /// The last change to the document's file while it was held behind a document on its way
    /// ([`WindowRecord::before_arrival`]), which Stop then acts on.
    missed: Option<Change>,
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
        let (render_ahead, read_stamp) = render::start(&path);
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
            rendering: None,
            reload_unplaced: false,
            missed: None,
        }
    }

    fn start_reload(&mut self) {
        self.render_ahead = Some(render::start(&self.path).0);
        self.reload_unplaced = true;
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
    /// is told nothing: it scrolls on the link's own result. While the window's next document is
    /// on its way, its page is told to stop that, as Stop does (ticket 38's review).
    AlreadyHere(String),
    /// The document is on its way to this window, the one the request came from (ticket 38). It
    /// is brought to the front, and the document stays on its way.
    OnItsWayHere(String),
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
    /// what the window was before is held until the document arrives, and put back if that
    /// fails. `left` is the document still on its way that the claim left, if one was, to be
    /// dropped after the table's lock, since dropping a watch stops it.
    Changed {
        window: String,
        claim: u64,
        left: Option<Box<WindowState>>,
    },
    /// The document is open already in `window`, the one the request came from. Its page is to
    /// stop the window's next document on its way, if `stop` is Some: the showing the page's view
    /// shows the document as.
    AlreadyHere {
        window: String,
        stop: Option<Showing>,
    },
    /// The document is on its way to this window, the one the request came from.
    OnItsWayHere(String),
    /// The document is open already in `window`, another than the one the request came from.
    /// Its page is to stop the window's next document on its way, if `stop` is Some, as for
    /// [`Claimed::AlreadyHere`], and then to scroll to `scroll_to`, if that is Some: the showing
    /// the window shows the document as, and the heading.
    AlreadyElsewhere {
        window: String,
        stop: Option<Showing>,
        scroll_to: Option<(Showing, String)>,
    },
    /// The target window has closed.
    WindowGone,
}

impl Documents {
    /// No window shows a document yet; what pages are told goes to `pages`, what is done to
    /// windows to `windows`, new windows open with `settings`, and pastes are written to `pasted`.
    pub fn new(
        pages: impl Pages,
        windows: impl Windows,
        settings: Arc<SettingsStore>,
        pasted: PastedFolder,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                table: Mutex::default(),
                titles: Mutex::default(),
                pages: Box::new(pages),
                windows: Box::new(windows),
                settings,
                windows_accent: Mutex::default(),
                pastes: Pastes::new(pasted),
            }),
        }
    }

    /// The reader's settings now.
    pub fn settings(&self) -> Settings {
        self.inner.settings.get()
    }

    /// The reader is in `window` now, so the next new window opens beside it (ticket 16), and its
    /// document's file is checked again, so a change its watch missed reloads it, or shows that
    /// it is gone, at the moment the reader looks (ticket 30).
    pub fn activated(&self, window: &str) {
        let mut table = self.lock();
        let Some(record) = table.windows.get(window) else {
            return;
        };
        if let Some(watch) = record
            .now
            .shown
            .as_ref()
            .and_then(|shown| shown.watch.as_ref())
        {
            watch.recheck();
        }
        table.active = Some(window.to_owned());
        table.opened_last = None;
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
    ///
    /// While a window's document is on its way, both it and the document its page's view still
    /// shows are that window's (ticket 38). A request from the window for the one on its way
    /// leaves it on its way, and tells the page nothing ([`Claimed::OnItsWayHere`]). A request
    /// for the one in view, from anywhere, has the window's page stop the load, as Stop does
    /// (ticket 38's review). Another document for the window starts from the one in view, so the
    /// one on its way never joins the history.
    fn claim(&self, document: DocumentAt<'_>, key: PathBuf, target: Target<'_>) -> Claimed {
        let mut table = self.lock();
        if let Some((window, record)) = table.window_showing(&key) {
            let window = window.clone();
            let from_here = matches!(
                target,
                Target::Window(from) | Target::NewWindowFrom(from) if from == window
            );
            if from_here && record.is_on_its_way(&key) {
                return Claimed::OnItsWayHere(window);
            }
            let stop = record.held_showing(&key);
            if from_here {
                return Claimed::AlreadyHere { window, stop };
            }
            let scroll_to = record.document_mut(&key).and_then(|shown| {
                let heading = document.heading?.to_owned();
                if shown.page_asked {
                    return Some((shown.showing, heading));
                }
                shown.arrival = Arrival::Given {
                    heading: Some(heading),
                };
                None
            });
            return Claimed::AlreadyElsewhere {
                window,
                stop,
                scroll_to,
            };
        }
        // Made only once a window takes the document, so that an open document or a closed
        // window starts no render.
        match target {
            Target::NewWindow | Target::NewWindowFrom(_) => {
                let (showing, claim) = (table.next_showing(), table.next_claim());
                let mut record = WindowRecord::default();
                // Stop leaves the window empty, as it was before it was given the document.
                let _nothing_left =
                    record.given(ShownDocument::given(document, key, showing, claim));
                Claimed::NewWindow(table.record_new(record))
            }
            Target::Window(window) => {
                let (showing, claim) = (table.next_showing(), table.next_claim());
                let Some(record) = table.windows.get_mut(window) else {
                    return Claimed::WindowGone;
                };
                let left = record
                    .given(ShownDocument::given(document, key, showing, claim))
                    .map(Box::new);
                Claimed::Changed {
                    window: window.to_owned(),
                    claim,
                    left,
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
            .now
            .shown
            .as_ref()
            .map(|shown| shown.path.clone())
    }

    /// The file of the document `window` shows, while it still shows it as `showing`; None once
    /// the window has been given another document, or closed. For a link, which comes from the
    /// showing its page's view showed when it was clicked (ticket 26): a link in a document the
    /// window has left would resolve against the new document's folder. While the window's
    /// document is on its way, the one its page's view still shows is still the window's, so a
    /// link in it goes from there (ticket 38).
    pub fn document_path_shown_as(&self, window: &str, showing: Showing) -> Option<PathBuf> {
        let table = self.lock();
        table
            .windows
            .get(window)?
            .documents()
            .find(|shown| shown.showing == showing)
            .map(|shown| shown.path.clone())
    }

    /// Forgets a window that closed or never opened, so its document opens anew. A render started
    /// for it, the watch on its document and its history go with it, and so does each pasted
    /// file among them that no other window holds.
    pub fn forget(&self, window: &str) {
        // Dropped after the lock, since dropping a watch stops it.
        let forgotten = {
            let mut table = self.lock();
            if table.active.as_deref() == Some(window) {
                table.active = None;
            }
            if table
                .opened_last
                .as_ref()
                .is_some_and(|(opened, _)| opened == window)
            {
                table.opened_last = None;
            }
            table.not_hearing.remove(window);
            table.windows.remove(window)
        };
        self.let_go(forgotten);
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
/// already, as launches, the open dialog and dropping all give it. Deleting pasted files tells a
/// pasted file by it, so that one named another way still counts.
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
        self.windows.get_mut(window)?.now.document_mut(key)
    }

    /// The window that shows the document with `key`, by its label, with its record: as its
    /// document, or in its page's view while its document is on its way (ticket 38), since a
    /// document is shown in at most one window (ADR 0001).
    fn window_showing(&mut self, key: &Path) -> Option<(&String, &mut WindowRecord)> {
        self.windows
            .iter_mut()
            .find(|(_, record)| record.shows(key))
    }

    /// What `window` held behind its document on its way, while it is the document with `key`.
    fn held_mut(&mut self, window: &str, key: &Path) -> Option<&mut ShownDocument> {
        self.windows
            .get_mut(window)?
            .before_arrival
            .as_mut()?
            .document_mut(key)
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
