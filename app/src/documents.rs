//! The document each window shows (ticket 22): giving a window its document, watching the
//! document's file, rendering it ahead, holding a notice and the heading until the page asks,
//! reloading it while the window still shows it (ticket 10), answering the page's request for its
//! document with whether to show or reload it (ticket 23, [`render_document`]), and forgetting it
//! with the window. Which window shows which document is kept here, and a document is shown in at
//! most one window (ADR 0001). Reading and rendering the file is the render module's. What a
//! window's page is told goes through [`Pages`], so all of this is tested without a webview.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use downlight_core::RenderedDocument;
use tauri::{State, WebviewWindow};

use crate::message::{Message, Pages};
use crate::platform;
use crate::render::{self, DocumentError, RenderTask};
use crate::timing::Timing;
use crate::watch::{Change, Watch};

/// Which window shows which document, by window label. The app's managed state.
pub struct Documents {
    inner: Arc<Inner>,
}

struct Inner {
    table: Mutex<WindowTable>,
    pages: Box<dyn Pages>,
}

/// Every open window by label, whether it shows a document or none.
#[derive(Default)]
struct WindowTable {
    /// Labels are `document-0`, `document-1` and so on, never reused while the process runs.
    labels_issued: usize,
    /// How many times a window has been given a document, for each to get its own [`Showing`].
    showings_issued: u64,
    /// Every open window, with the document it shows now; None while it shows none.
    shown: HashMap<String, Option<ShownDocument>>,
}

/// One time a window was given a document, unique while the process runs: a window given the
/// same document again, after another, has a new one. The page sends the one its view shows when
/// it asks for its document, so that [`Documents::handover`] can tell whether to show the
/// document or reload it, whichever message made the page ask (ticket 23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Showing(u64);

struct ShownDocument {
    /// This time the window was given the document.
    showing: Showing,
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
    held_notice: Option<String>,
    /// The heading the window shows the document at, from a link like `guide.md#install` (ticket
    /// 11). Handed over with every show rather than taken by the first, so that of two links
    /// followed in quick succession, the page's latest request still gets it; never with a
    /// reload, which keeps the reading position.
    heading: Option<String>,
}

impl ShownDocument {
    fn given(document: DocumentAt<'_>, key: PathBuf, showing: Showing) -> Self {
        Self {
            showing,
            path: document.path.to_owned(),
            key,
            render_ahead: Some(render::start(document.path, "read-and-render")),
            watch: None,
            page_asked: false,
            held_notice: None,
            heading: document.heading.map(str::to_owned),
        }
    }

    fn start_reload(&mut self) {
        self.render_ahead = Some(render::start(&self.path, "reload"));
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

/// What [`Documents::give`] did with a document, by window label. The caller acts on it: it opens
/// the new window, titles a changed one, or brings the one that shows the document to the front.
#[derive(Debug, PartialEq, Eq)]
pub enum Given {
    /// A new window with this label shows the document, and the caller opens it. A window that
    /// can't open is forgotten ([`Documents::forget`]).
    NewWindow(String),
    /// The window shows the document now, and its page has been told to ask for it.
    Changed(String),
    /// The document is open already in this window, the one the request came from ([`Target`]),
    /// as for a link to the document that window shows. It is told nothing: its page scrolls on
    /// the link's own result.
    AlreadyHere(String),
    /// The document is open already in this window, another than the one the request came from,
    /// or any window for a launch. Its page has been told to scroll to the document's heading, if
    /// the document came with one and the page listens; a page that doesn't yet gets it with its
    /// first render.
    AlreadyElsewhere(String),
    /// The target window has closed.
    WindowGone,
}

/// What [`Documents::claim`] recorded under the table's lock, for [`Documents::give`] to act on
/// after it.
enum Claimed {
    /// No window is to scroll now, and `give` returns this as it is.
    Given(Given),
    /// The document is open already in `window`, another than the one the request came from,
    /// whose page listens: it is to scroll to `heading` now. `give` returns it as
    /// [`Given::AlreadyElsewhere`].
    ScrollElsewhere { window: String, heading: String },
}

/// What a window's page gets when it asks for its document ([`render_document`]).
struct Handover {
    /// The document's read and render.
    render: RenderTask,
    /// This time the window was given the document, for the page to send with its next request.
    showing: Showing,
    /// What the page does with the render.
    place: Place,
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
}

impl Documents {
    /// No window shows a document yet; what pages are told goes to `pages`.
    pub fn new(pages: impl Pages) -> Self {
        Self {
            inner: Arc::new(Inner {
                table: Mutex::default(),
                pages: Box::new(pages),
            }),
        }
    }

    /// Records a new window that shows no document, as for a launch that names none, and returns
    /// its label for the caller to open it. A window that can't open is forgotten
    /// ([`Documents::forget`]).
    pub fn open_empty(&self) -> String {
        let mut table = self.lock();
        let window = table.next_label();
        table.shown.insert(window.clone(), None);
        window
    }

    /// Gives `target` the document, unless the document is open already, the one way a window
    /// gets or changes its document. Looking up and recording happen under one lock, so two
    /// requests for the same document end up in one window.
    ///
    /// The document's render starts as soon as a window is given it, so that it runs while a new
    /// window's webview starts, and the window starts watching the document's file, to reload it
    /// when it changes.
    ///
    /// A document that is open already, and came with a heading, has the window that shows it
    /// scroll there: told now if its page listens, or with its first render if it doesn't yet.
    /// The window the request came from ([`Target`]) is never told ([`Given::AlreadyHere`]): its
    /// page scrolls on the link's own result.
    ///
    /// Fails when a window given another document can't tell its page so: the page keeps showing
    /// its old document, and the caller tells the reader the new one isn't shown. The window
    /// records the new document all the same, as before ticket 22.
    pub fn give(&self, document: DocumentAt<'_>, target: Target<'_>) -> tauri::Result<Given> {
        let key = document_key(document.path);
        let given = match self.claim(document, key.clone(), target) {
            Claimed::Given(given) => given,
            Claimed::ScrollElsewhere { window, heading } => {
                // The window that shows the document is brought to the front all the same, and
                // the page that asked shows only what went wrong with its own request, so a
                // scroll that can't be sent there is nobody's to tell.
                let _ = self.send(&window, Message::ScrollToHeading(heading));
                Given::AlreadyElsewhere(window)
            }
        };
        match &given {
            Given::NewWindow(window) => self.watch(window, document.path, key),
            Given::Changed(window) => {
                self.watch(window, document.path, key);
                self.send(window, Message::DocumentShown)?;
            }
            Given::AlreadyHere(_) | Given::AlreadyElsewhere(_) | Given::WindowGone => {}
        }
        Ok(given)
    }

    /// Records that `target` shows `document` with `key`, and what that changed, including a
    /// scroll for the window of an open document, which the caller sends outside the lock. Whether
    /// the request came from the window that shows the document is decided here, and only here.
    ///
    /// The window of an open document is to scroll to the document's heading, if it has one,
    /// unless the request came from that window. A page that hasn't asked for its document yet
    /// may not be listening, so it gets the heading with its first render instead, the way a
    /// notice is held (ticket 10).
    fn claim(&self, document: DocumentAt<'_>, key: PathBuf, target: Target<'_>) -> Claimed {
        let mut table = self.lock();
        if let Some(window) = table.window_showing(&key) {
            if matches!(target, Target::Window(from) | Target::NewWindowFrom(from) if from == window)
            {
                return Claimed::Given(Given::AlreadyHere(window));
            }
            let Some(heading) = document.heading.map(str::to_owned) else {
                return Claimed::Given(Given::AlreadyElsewhere(window));
            };
            if let Some(shown) = table.shown_mut(&window, &key)
                && !shown.page_asked
            {
                shown.heading = Some(heading);
                return Claimed::Given(Given::AlreadyElsewhere(window));
            }
            return Claimed::ScrollElsewhere { window, heading };
        }
        // Made only once a window takes the document, so that an open document or a closed
        // window starts no render.
        let given = match target {
            Target::NewWindow | Target::NewWindowFrom(_) => {
                let window = table.next_label();
                let shown = ShownDocument::given(document, key, table.next_showing());
                table.shown.insert(window.clone(), Some(shown));
                Given::NewWindow(window)
            }
            Target::Window(window) => {
                if !table.shown.contains_key(window) {
                    return Claimed::Given(Given::WindowGone);
                }
                let shown = ShownDocument::given(document, key, table.next_showing());
                table.shown.insert(window.to_owned(), Some(shown));
                Given::Changed(window.to_owned())
            }
        };
        Claimed::Given(given)
    }

    /// Hands `window`'s document over to its page, which has asked for it while its view shows
    /// `page_shows` (None while it shows no document), or gives None while the window shows none:
    /// the render started when the window was given the document, the first time, and a new read
    /// of its file after that. The page shows the document when its view shows another showing,
    /// and reloads it when its view shows this one. A notice held until the page asked is sent
    /// now, since the page listens for notices before it first asks.
    fn handover(&self, window: &str, page_shows: Option<Showing>) -> Option<Handover> {
        let (path, ahead, notice, showing, place) = {
            let mut table = self.lock();
            let shown = table.shown.get_mut(window)?.as_mut()?;
            shown.page_asked = true;
            let place = if page_shows == Some(shown.showing) {
                Place::Reload
            } else {
                Place::Show {
                    heading: shown.heading.clone(),
                }
            };
            (
                shown.path.clone(),
                shown.render_ahead.take(),
                shown.held_notice.take(),
                shown.showing,
                place,
            )
        };
        if let Some(notice) = notice {
            // The page asked for its document, not for the notice, and gets its document either
            // way; a notice that can't reach it has nobody left to tell.
            let _ = self.send(window, Message::Notice(notice));
        }
        Some(Handover {
            render: ahead.unwrap_or_else(|| render::start(&path, "read-and-render")),
            showing,
            place,
        })
    }

    /// Reloads `window`'s document by hand (F5, Ctrl+R and the right-click menu), the way a save
    /// to its file does: its new render starts, and the page is told, and then asks for it. A
    /// window that shows no document has nothing to reload. Fails when the page can't be told.
    ///
    /// The page is told after the table's lock is released, so the window may have been given
    /// another document by the time the page asks; [`Documents::handover`] then answers a show.
    pub fn reload(&self, window: &str) -> tauri::Result<()> {
        let reloading = self
            .lock()
            .shown
            .get_mut(window)
            .and_then(Option::as_mut)
            .map(ShownDocument::start_reload)
            .is_some();
        if reloading {
            self.send(window, Message::Reload)?;
        }
        Ok(())
    }

    /// The file of the document `window` shows, as the launch, the dialog, the drop or the link
    /// named it, or None while it shows none. The one way to learn a window's document: commands
    /// take it from here, never from the page.
    pub fn document_path(&self, window: &str) -> Option<PathBuf> {
        self.lock()
            .shown
            .get(window)?
            .as_ref()
            .map(|shown| shown.path.clone())
    }

    /// Forgets a window that closed or never opened, so its document opens anew. A render started
    /// for it, and the watch on its document, go with it.
    pub fn forget(&self, window: &str) {
        self.lock().shown.remove(window);
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
                    format!(
                        "Can't watch {} for changes, so only F5 reloads it: {error}",
                        document.display()
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
    /// new document. The page is told after the lock, never under it, so a `reload` can still
    /// reach it after the `document-shown` of a document given in between; the page then asks
    /// with the showing its view still shows, and is answered a show (ticket 23).
    fn changed(&self, window: &str, key: &Path, change: Change) {
        let message = {
            let mut table = self.lock();
            let Some(shown) = table.shown_mut(window, key) else {
                return;
            };
            match change {
                Change::Changed | Change::Back => {
                    shown.start_reload();
                    Message::Reload
                }
                Change::Gone => Message::Gone,
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

    /// Sends `window`'s page the notice `text`, about the document with `key`, or holds it until
    /// the page first asks for that document, since until then the page may not be listening. A
    /// notice for a document the window has left, or for a closed window, is dropped.
    fn notice(&self, window: &str, key: &Path, text: String) {
        {
            let mut table = self.lock();
            let Some(shown) = table.shown_mut(window, key) else {
                return;
            };
            if !shown.page_asked {
                shown.held_notice = Some(text);
                return;
            }
        }
        // Sent from the watch's thread, which nobody waits on, so nobody is left to tell.
        let _ = self.send(window, Message::Notice(text));
    }

    /// Sent outside the table's lock, so that no page is told anything while the table is held.
    fn send(&self, window: &str, message: Message) -> tauri::Result<()> {
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
            timing.report(window.label(), &stages);
            Content::Document(document)
        }
        Ok(Err(error)) => Content::Error(error),
        Err(source) => Content::Error(DocumentError::ReadAndRenderStopped { source }),
    };
    Ok(Some(DocumentToShow {
        showing: handover.showing,
        place: handover.place,
        content,
    }))
}

/// What the page gets when it asks for its document: the showing the window is at, what to do
/// with the render (show it at a heading or the top, or reload it at the reading position), and
/// the rendered document or why it can't be read. A read that fails is part of the answer, since
/// only the answer says whether the page keeps its last render, on a reload, or shows why, on a
/// show (ticket 23).
#[derive(serde::Serialize)]
pub struct DocumentToShow {
    showing: Showing,
    #[serde(flatten)]
    place: Place,
    #[serde(flatten)]
    content: Content,
}

/// The rendered document, which goes as its HTML, or why it can't be read.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum Content {
    Document(RenderedDocument),
    Error(DocumentError),
}

/// A sample of each answer [`render_document`] gives, by name, which the page's tests read
/// (`app/src/contract.rs`, ticket 27): the rendered document goes as its HTML, the heading only
/// with a show, and a read that fails as why.
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new place or content stops the build here: give it a sample below, then name it here.
    let _sampled = |answer: &Option<DocumentToShow>| match answer {
        None
        | Some(DocumentToShow {
            place: Place::Show { .. } | Place::Reload,
            content: Content::Document(_) | Content::Error(_),
            ..
        }) => {}
    };
    let answer = |place: Place, content: Content| {
        serde_json::to_value(Some(DocumentToShow {
            showing: Showing(7),
            place,
            content,
        }))
    };
    let rendered = || {
        Content::Document(downlight_core::render(
            b"# Install\n\nRun the installer.\n",
            Path::new("a.md"),
            "",
        ))
    };
    let unread = || {
        Content::Error(DocumentError::Read {
            path: PathBuf::from(r"C:\docs\a.md"),
            source: io::ErrorKind::NotFound.into(),
        })
    };
    let at_top = || Place::Show { heading: None };
    Ok(serde_json::json!({
        "showAtHeading": answer(Place::Show { heading: Some("install".to_owned()) }, rendered())?,
        "showAtTop": answer(at_top(), rendered())?,
        "reload": answer(Place::Reload, rendered())?,
        "unreadShow": answer(at_top(), unread())?,
        "unreadReload": answer(Place::Reload, unread())?,
        "noDocument": serde_json::to_value(None::<DocumentToShow>)?,
    }))
}

/// Reloads the calling window's document by hand (F5, Ctrl+R and the right-click menu).
#[tauri::command]
pub fn reload_document(
    window: WebviewWindow,
    documents: State<'_, Documents>,
) -> tauri::Result<()> {
    documents.reload(window.label())
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
        self.shown
            .get_mut(window)
            .and_then(Option::as_mut)
            .filter(|shown| shown.key == key)
    }

    fn window_showing(&self, key: &Path) -> Option<String> {
        self.shown.iter().find_map(|(label, shown)| {
            shown
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
}

#[cfg(all(test, windows))]
mod tests;
