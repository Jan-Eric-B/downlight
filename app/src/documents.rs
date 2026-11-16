//! The document each window shows (ticket 22): giving a window its document, watching the
//! document's file, rendering it ahead, holding a notice and the heading until the page asks,
//! reloading it while the window still shows it (ticket 10), answering the page's request for its
//! document with whether to show or reload it (ticket 23, [`render_document`]), and forgetting it
//! with the window. Which window shows which document is kept here, and a document is shown in at
//! most one window (ADR 0001). Reading and rendering the file is the render module's.
//!
//! A show is carried out here too (ticket 28): opening the new window, titling a window given
//! another document, bringing a window to the front, and undoing the show when part of it fails.
//! What a window's page is told goes through [`Pages`], and what is done to a window itself
//! through [`Windows`], so all of this is tested without a webview.
//!
//! Each window also keeps its history (ticket 12): the documents it showed before and after the
//! one it shows, which Back and Forward give it back ([`history`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::{fmt, io};

use downlight_core::RenderedDocument;
use tauri::{State, WebviewWindow};

use crate::message::{Message, Pages};
use crate::notice::{NamedFile, Notice};
use crate::render::{self, DocumentError, RenderTask};
use crate::timing::Timing;
use crate::watch::{Change, Watch};
use crate::{APP_NAME, platform};

pub mod history;
#[cfg(test)]
pub mod recorder;

use history::{History, Steps, WindowRecord};

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
}

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
    /// The watch on the document's file, which reloads it; None until it has started, or if it
    /// couldn't. Replacing or forgetting the entry drops it, which stops it.
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
        Self {
            showing,
            claim,
            render_ahead: Some(render::start(&path)),
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

/// What a window's page gets when it asks for its document ([`render_document`]).
struct Handover {
    /// The document's read and render.
    render: RenderTask,
    /// This time the window was given the document, for the page to send with its next request.
    showing: Showing,
    /// The document's file, for the title bar.
    file: DocumentFile,
    /// What the page does with the render.
    place: Place,
    /// A notice about the document held until the page first asked for it.
    notice: Option<Notice>,
    /// Whether the window has a document to go back to, and one to go forward to.
    steps: Steps,
}

/// The file of the document a window shows, as the page's title bar shows it (window design
/// ticket 04): its name, and its full path as the name's tooltip. Sent with every render rather
/// than worked out by the page, which would have to know each system's paths.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct DocumentFile {
    name: String,
    path: String,
}

impl DocumentFile {
    /// The file at `path`, as the launch, the dialog, the drop or the link named it.
    fn of(path: &Path) -> Self {
        Self {
            name: file_name_shown(path),
            path: path.display().to_string(),
        }
    }
}

/// The name the file or folder at `path` is shown by, in the title bar and in the window's title,
/// which share it so the two can't name a document differently. Launches, the dialog, the drop
/// and links give absolute paths, so only a drive root has no name of its own; it is named as its
/// path rather than left blank.
pub fn file_name_shown(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// `README.md - docs - Downlight`: the folder tells documents with the same name apart in Alt+Tab
/// and on the taskbar. A window that shows no document (None) is titled `Downlight`.
fn title_of(document: Option<&Path>) -> String {
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

/// What a page does with the render it asked for, decided by the showing its view shows rather
/// than by which message made it ask, since a `reload` sent for the document a window has just
/// left can reach the page after `document-shown` (ticket 23).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "place", rename_all = "camelCase")]
enum Place {
    /// The page's view shows another showing, or none: show the document at this heading, from a
    /// link like `guide.md#install` (ticket 11), or at its top without one.
    Show { heading: Option<String> },
    /// The page's view shows this showing already: show the new render at the reading position.
    Reload,
    /// The page's view shows another showing, and Back or Forward gave the window this one back
    /// (ticket 12): show the document where the page's view was when it left this showing, which
    /// only the page knows.
    Return,
}

impl Place {
    /// What the timing switch reports the render as: by the place the page gets rather than by
    /// why the render started, so a reload by hand, which reads the file on the page's request,
    /// is reported as a reload too (ticket 26). `npm run measure` reads both. A return reads and
    /// renders the document as a show does.
    fn reported_as(&self) -> &'static str {
        match self {
            Self::Show { .. } | Self::Return => "read-and-render",
            Self::Reload => "reload",
        }
    }
}

impl Documents {
    /// No window shows a document yet; what pages are told goes to `pages`, and what is done to
    /// windows to `windows`.
    pub fn new(pages: impl Pages, windows: impl Windows) -> Self {
        Self {
            inner: Arc::new(Inner {
                table: Mutex::default(),
                titles: Mutex::default(),
                pages: Box::new(pages),
                windows: Box::new(windows),
            }),
        }
    }

    /// Opens a new window that shows no document, as for a launch that names none, and returns
    /// its label. A window that can't open is forgotten.
    pub fn open_empty(&self) -> Result<String, WindowError> {
        let window = {
            let mut table = self.lock();
            let window = table.next_label();
            table
                .windows
                .insert(window.clone(), WindowRecord::default());
            window
        };
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

    /// Carries out what a claim recorded for the document at `path`, with `key`, as
    /// [`Documents::give`] describes, for a show and a step alike.
    fn carry_out(&self, claimed: Claimed, path: &Path, key: PathBuf) -> Result<Given, ShowError> {
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
                let window = table.next_label();
                let (showing, claim) = (table.next_showing(), table.next_claim());
                let shown = ShownDocument::given(document, key, showing, claim);
                let record = WindowRecord {
                    shown: Some(shown),
                    history: History::default(),
                };
                table.windows.insert(window.clone(), record);
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

    /// Opens the window `window` just recorded, titled after `document` (None for a window that
    /// shows none), or forgets it if it can't open, so that its document opens anew.
    fn open(&self, window: &str, document: Option<&Path>) -> Result<(), WindowError> {
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

    /// Hands `window`'s document over to its page, which has asked for it while its view shows
    /// `page_shows` (None while it shows no document), or gives None while the window shows none:
    /// the render started when the window was given the document or its file changed, if the page
    /// hasn't taken it yet, and a new read of its file otherwise, as for F5. The page shows the
    /// document when its view shows another showing, returns to where it was in it when Back or
    /// Forward gave it back, and reloads it when its view shows this one. A notice held until the
    /// page asked goes with the answer rather than as a message, since the page's listener for
    /// notices may not be registered yet (ticket 26).
    fn handover(&self, window: &str, page_shows: Option<Showing>) -> Option<Handover> {
        let mut table = self.lock();
        let record = table.windows.get_mut(window)?;
        let steps = record.history.steps();
        let shown = record.shown.as_mut()?;
        shown.page_asked = true;
        let place = if page_shows == Some(shown.showing) {
            Place::Reload
        } else {
            match &shown.arrival {
                Arrival::Given { heading } => Place::Show {
                    heading: heading.clone(),
                },
                Arrival::Stepped => Place::Return,
            }
        };
        Some(Handover {
            render: shown
                .render_ahead
                .take()
                .unwrap_or_else(|| render::start(&shown.path)),
            showing: shown.showing,
            file: DocumentFile::of(&shown.path),
            place,
            notice: shown.held_notice.take(),
            steps,
        })
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

    /// What `window`'s page asking for its document does, for the tests of other modules, which
    /// have no webview to call [`render_document`]: hands the document over, and gives the
    /// showing the page then sends with a link (None while the window shows no document).
    #[cfg(test)]
    pub fn page_asks(&self, window: &str) -> Option<Showing> {
        self.handover(window, None).map(|handover| handover.showing)
    }

    /// Forgets a window that closed or never opened, so its document opens anew. A render started
    /// for it, the watch on its document and its history go with it.
    pub fn forget(&self, window: &str) {
        // Dropped after the lock, since dropping a watch stops it.
        let _forgotten = self.lock().windows.remove(window);
    }

    /// Starts watching `document`'s file, which `window` was just given with `key`, on a blocking
    /// thread, since that reads the file system. A watch that can't start leaves the window
    /// without reloads on save, and the reader is told.
    fn watch(&self, window: &str, document: &Path, key: PathBuf) {
        // Weak, so that a watch never keeps the table that holds it alive.
        let documents = Arc::downgrade(&self.inner);
        let (window, document) = (window.to_owned(), document.to_owned());
        tauri::async_runtime::spawn_blocking(move || {
            let on_change = Self::on_change(documents.clone(), window.clone(), key.clone());
            let started = Watch::start(&document, on_change);
            let Some(documents) = Self::upgrade(&documents) else {
                return;
            };
            match started {
                Ok(watch) => documents.keep_watch(&window, &key, watch),
                Err(error) => documents.notice(
                    &window,
                    &key,
                    Notice::naming(
                        "Can't watch ",
                        NamedFile::at(&document),
                        format!(" for changes, so only F5 reloads it: {error}"),
                    ),
                ),
            }
        });
    }

    /// What the watch on the document `window` was given with `key` does on a change to its file.
    fn on_change(
        documents: Weak<Inner>,
        window: String,
        key: PathBuf,
    ) -> impl FnMut(Change) + Send + 'static {
        move |change| {
            if let Some(documents) = Self::upgrade(&documents) {
                documents.changed(&window, &key, change);
            }
        }
    }

    fn upgrade(documents: &Weak<Inner>) -> Option<Self> {
        documents.upgrade().map(|inner| Self { inner })
    }

    /// Acts on a change to the file of the document with `key`, unless `window` has left it since
    /// or closed: a watch can report a change after that, and a document the window has left is
    /// never reloaded. Checked and acted on under one lock (ticket 21): a document the window is
    /// given after a separate check would have its render ahead replaced by one for the window's
    /// new document. The page is told after the lock, never under it, so the message can still
    /// reach it after the one for a document given in between; the page then asks with the
    /// showing its view still shows, and is answered a show (ticket 23), or, for a `gone`, sees
    /// that it names another showing than its view's (ticket 26).
    fn changed(&self, window: &str, key: &Path, change: Change) {
        let message = {
            let mut table = self.lock();
            let Some(shown) = table.shown_mut(window, key) else {
                return;
            };
            match change {
                Change::Changed | Change::Back => {
                    shown.start_reload();
                    Message::AskForDocument
                }
                Change::Gone => Message::Gone(shown.showing),
            }
        };
        // Sent from the watch's thread, which nobody waits on, so nobody is left to tell.
        let _ = self.send(window, message);
    }

    /// Gives `window` the watch started for the document with `key`. A watch that started after
    /// the window was given another document, or closed, is dropped instead, so a save to a
    /// document the window has left never reloads it.
    fn keep_watch(&self, window: &str, key: &Path, watch: Watch) {
        if let Some(shown) = self.lock().shown_mut(window, key) {
            shown.watch = Some(watch);
        }
    }

    /// Sends `window`'s page the `notice` about the document with `key`, or holds it until the
    /// page first asks for that document, since until then the page may not be listening. A
    /// notice for a document the window has left, or for a closed window, is dropped.
    fn notice(&self, window: &str, key: &Path, notice: Notice) {
        {
            let mut table = self.lock();
            let Some(shown) = table.shown_mut(window, key) else {
                return;
            };
            if !shown.page_asked {
                shown.held_notice = Some(notice);
                return;
            }
        }
        // Sent from the watch's thread, which nobody waits on, so nobody is left to tell.
        let _ = self.send(window, Message::Notice(notice));
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

/// Reads and renders the calling window's document, and says whether the page shows it or reloads
/// it, or gives None while the window shows none.
///
/// The page passes only the showing its view shows, or None: the path comes from the app's own
/// state, so the page can't ask for other files. The work runs on a blocking thread, off the main
/// thread, so the window keeps answering while a large document renders. The first request after
/// the window was given its document gets the render started then ([`Documents::handover`]).
///
/// Never fails: a read that fails goes in the answer. The `Result` is Tauri's, which requires one
/// of an async command that borrows its arguments, as `State` does.
#[tauri::command]
pub async fn render_document(
    window: WebviewWindow,
    showing: Option<Showing>,
    documents: State<'_, Documents>,
    timing: State<'_, Timing>,
) -> Result<Option<DocumentToShow>, ()> {
    let Some(handover) = documents.handover(window.label(), showing) else {
        return Ok(None);
    };
    let content = match handover.render.await {
        Ok(Ok((document, stages))) => {
            timing.report(window.label(), handover.place.reported_as(), &stages);
            Content::Document(document)
        }
        Ok(Err(error)) => Content::Error(error.into()),
        Err(source) => Content::Error(DocumentError::ReadAndRenderStopped { source }.into()),
    };
    Ok(Some(DocumentToShow {
        showing: handover.showing,
        file: handover.file,
        place: handover.place,
        content,
        notice: handover.notice,
        steps: handover.steps,
    }))
}

/// What the page gets when it asks for its document: the showing the window is at, the
/// document's file, what to do with the render (show it at a heading or the top, return to where
/// the page was in it, or reload it at the reading position), the rendered document or why it
/// can't be read, a notice held until the page first asked, or null, and whether the window has
/// a document to go back or forward to. A read that fails is part of the answer, since only the
/// answer says whether the page keeps its last render, on a reload, or shows why, on a show
/// (ticket 23).
#[derive(serde::Serialize)]
pub struct DocumentToShow {
    showing: Showing,
    file: DocumentFile,
    #[serde(flatten)]
    place: Place,
    #[serde(flatten)]
    content: Content,
    notice: Option<Notice>,
    steps: Steps,
}

/// The rendered document, which goes as its HTML, or why it can't be read.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum Content {
    Document(RenderedDocument),
    Error(Notice),
}

/// A sample of each answer [`render_document`] gives, by name, which the page's tests read
/// (`app/src/contract.rs`, ticket 27): the rendered document goes as its HTML, the heading only
/// with a show, a read that fails as why, the document's file with each, a held notice with the
/// first (ticket 26), and whether there is somewhere to go back or forward to with each, which is
/// nowhere but after a return (ticket 12).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new place or content stops the build here: give it a sample below, then name it here.
    let _sampled = |answer: &Option<DocumentToShow>| match answer {
        None
        | Some(DocumentToShow {
            place: Place::Show { .. } | Place::Reload | Place::Return,
            content: Content::Document(_) | Content::Error(_),
            notice: None | Some(_),
            ..
        }) => {}
    };
    let answer_with = |place: Place, content: Content, notice: Option<Notice>| {
        let returned = place == Place::Return;
        serde_json::to_value(Some(DocumentToShow {
            showing: Showing::sample(),
            // Written out rather than `DocumentFile::of`, so the samples are the same on every
            // system the tests run on.
            file: DocumentFile {
                name: "a.md".to_owned(),
                path: r"C:\docs\a.md".to_owned(),
            },
            place,
            content,
            notice,
            steps: Steps {
                back: returned,
                forward: returned,
            },
        }))
    };
    let answer = |place: Place, content: Content| answer_with(place, content, None);
    let held_notice = Notice::naming(
        "Can't watch ",
        NamedFile::sample(false),
        " for changes, so only F5 reloads it: Access is denied.",
    );
    let rendered = || {
        Content::Document(downlight_core::render(
            b"# Install\n\nRun the installer.\n",
            Path::new("a.md"),
            "",
        ))
    };
    // A missing file whose folder is there, so the notice opens the folder (window design ticket
    // 11).
    let unread = || {
        Content::Error(
            DocumentError::Read {
                file: NamedFile::sample(true),
                source: io::ErrorKind::NotFound.into(),
            }
            .into(),
        )
    };
    let at_top = || Place::Show { heading: None };
    Ok(serde_json::json!({
        "showAtHeading": answer(Place::Show { heading: Some("install".to_owned()) }, rendered())?,
        "showAtTop": answer(at_top(), rendered())?,
        "showWithHeldNotice": answer_with(at_top(), rendered(), Some(held_notice))?,
        "reload": answer(Place::Reload, rendered())?,
        "return": answer(Place::Return, rendered())?,
        "unreadShow": answer(at_top(), unread())?,
        "unreadReturn": answer(Place::Return, unread())?,
        "unreadReload": answer(Place::Reload, unread())?,
        "noDocument": serde_json::to_value(None::<DocumentToShow>)?,
    }))
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

    fn next_label(&mut self) -> String {
        let label = format!("document-{}", self.labels_issued);
        self.labels_issued += 1;
        label
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
