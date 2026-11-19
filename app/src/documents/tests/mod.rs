//! Most tests drive the module through its public interface and check what pages are told and
//! what is done to windows (the recorder). A page's request for its document goes through
//! `handover`, which is all of `render_document` but its webview and timing, so that it is tested
//! without either. A few tests call the private watch callbacks (`on_report`, `notice`) instead:
//! they stand in for the watch's thread, so that a test reports a change or a notice at the
//! moment of its choosing and reproduces ticket 21's race and the held notice deterministically.
//!
//! The tests are split by feature, so that each is found by its name: giving a window its
//! document, the handover to its page, and watching its file. The helpers here are the ones
//! several of them use.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tauri::async_runtime::block_on;

use super::handover::{Handover, Place};
use super::recorder::{Recorder, Sent, ToWindow, recorded, recorded_with};
use super::*;
use crate::watch::{Change, Report};

mod give;
mod handover;
mod history;
mod place;
mod settings;
mod watch;

/// A file that exists: the app crate's own manifest.
fn manifest() -> PathBuf {
    app_file("Cargo.toml")
}

fn app_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// No window shows a document yet, and what pages are told and what is done to windows is
/// recorded.
fn documents() -> (Documents, Recorder) {
    recorded()
}

/// What [`Documents::give`] did, where every window opens and every page can be told.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn give(documents: &Documents, document: DocumentAt<'_>, target: Target<'_>) -> Given {
    documents.give(document, target).unwrap()
}

/// Opens a new window that shows no document and returns its label.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn open_empty(documents: &Documents) -> String {
    documents.open_empty().unwrap()
}

/// Gives a new window `document` and returns its label.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn open_new(documents: &Documents, document: &Path) -> String {
    match give(documents, DocumentAt::new(document), Target::NewWindow) {
        Given::NewWindow(window) => window,
        other => panic!("expected a new window, got {other:?}"),
    }
}

/// `what` as sent to `window`'s page, or done to `window`.
fn to<T>(window: &str, what: T) -> (String, T) {
    (window.to_owned(), what)
}

/// What a window opened with `title` is recorded as, with the default settings, at the default
/// size, and placed by the system, as when no window has closed and none is active.
fn opened(title: &str) -> ToWindow {
    ToWindow::Opened(NewWindow {
        title: title.to_owned(),
        settings: Settings::default().page(),
        look: Settings::default().window_look(),
        size: super::place::DEFAULT_SIZE,
        at: None,
    })
}

/// What a window titled `title` is recorded as.
fn titled(title: &str) -> ToWindow {
    ToWindow::Titled(title.to_owned())
}

/// A document file of the test's own, so that tests running in parallel never share one.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn document(test: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "downlight-documents-{}-{test}.md",
        std::process::id()
    ));
    std::fs::write(&path, text).unwrap();
    path
}

/// What `window`'s page gets when it asks for its document while its view shows `page_shows`.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn handover(documents: &Documents, window: &str, page_shows: Option<Showing>) -> Handover {
    let Some(handover) = documents.handover(window, page_shows) else {
        panic!("expected {window} to show a document");
    };
    handover
}

/// The HTML `window`'s page gets when it asks for its document.
fn rendered_html(documents: &Documents, window: &str) -> String {
    html_of(handover(documents, window, None).render)
}

/// The HTML a render handed over gives.
fn html_of(render: RenderTask) -> String {
    html_and_start_of(render).0
}

/// The HTML a render handed over gives, and when its read started, as the timing switch reports
/// it (`at`): a render started ahead, when the window was given its document or its file
/// changed, started before the page asked, and one started on the page's request after it.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn html_and_start_of(render: RenderTask) -> (String, SystemTime) {
    match block_on(render) {
        Ok(Ok((document, stages))) => (document.as_str().to_owned(), stages.started()),
        Ok(Err(error)) => panic!("expected a rendered document, got {}", Notice::from(error)),
        Err(error) => panic!("the read and render stopped: {error}"),
    }
}

/// Long enough for a render started ahead to have started reading on its blocking thread, so
/// that a test can tell it from a render started later by when its read started. Only a stalled
/// machine takes longer; a render that was never started ahead fails the test on any.
const RENDER_START_WAIT: Duration = Duration::from_millis(200);

/// Waits [`RENDER_START_WAIT`] and gives the time after it.
fn after_a_render_could_start() -> SystemTime {
    std::thread::sleep(RENDER_START_WAIT);
    SystemTime::now()
}

/// Another handle on the same documents, as each of Tauri's commands gets the app's state, for a
/// request on another thread.
fn shared(documents: &Documents) -> Documents {
    Documents {
        inner: Arc::clone(&documents.inner),
    }
}

/// Long enough for a request on another thread to end if nothing holds it up, so that a test can
/// tell a request that something holds up, as a lock does, by its ending only later. Only a stalled
/// machine takes longer.
const HELD_UP_WAIT: Duration = Duration::from_millis(200);

/// Long enough for a request held up on another thread to end once what held it up has.
const HELD_UP_END_WAIT: Duration = Duration::from_secs(3);

/// Runs `request` on another thread, as a second request for the same window may run while a
/// first is carried out, and waits until it ends or [`HELD_UP_WAIT`] has passed.
fn meanwhile(documents: Documents, request: impl FnOnce(&Documents) + Send + 'static) {
    let (ended, end) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        request(&documents);
        // The test may have stopped waiting already.
        let _ = ended.send(());
    });
    let _ = end.recv_timeout(HELD_UP_WAIT);
}

/// The heading `window`'s page gets with its document when its view shows none yet, or None
/// while the window shows none.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn heading_of(documents: &Documents, window: &str) -> Option<Option<String>> {
    documents
        .handover(window, None)
        .map(|handover| match handover.place {
            Place::Show { heading } => heading,
            Place::Reload | Place::Return => {
                panic!("a page that shows nothing yet was told to reload or return")
            }
        })
}

/// What the watch on `path`, given to `window`, does with a report: the callback
/// [`Documents::watch`] hands the watch, so a test can report at a moment of its choosing, as a
/// watch's thread may.
fn report_callback(documents: &Documents, window: &str, path: &Path) -> impl FnMut(Report) + use<> {
    Documents::on_report(
        Arc::downgrade(&documents.inner),
        window.to_owned(),
        path.to_owned(),
        document_key(path),
    )
}

/// What the watch on `path`, given to `window`, does on a change to the file, as
/// [`report_callback`] for the reports of changes.
fn watch_callback(documents: &Documents, window: &str, path: &Path) -> impl FnMut(Change) + use<> {
    let mut on_report = report_callback(documents, window, path);
    move |change| on_report(Report::Change(change))
}
