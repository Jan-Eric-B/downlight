//! Most tests drive the module through its public interface and check what pages are told. A
//! page's request for its document goes through `handover`, which is all of `render_document`
//! but its webview and timing, so that it is tested without either. A few tests call the private
//! watch callbacks (`on_change`, `keep_watch`, `notice`) instead: they stand in for the watch's
//! thread, so that a test reports a change, a late watch or a notice at the moment of its
//! choosing and reproduces ticket 21's race and the held notice deterministically.
//!
//! The tests are split by feature, so that each is found by its name: giving a window its
//! document, the handover to its page, and watching its file. The helpers here are the ones
//! several of them use.

use std::path::PathBuf;
use std::time::Duration;

use tauri::async_runtime::block_on;

use crate::message::recorder::{Recorder, Sent};

use super::*;

mod give;
mod handover;
mod watch;

/// A file that exists: the app crate's own manifest.
fn manifest() -> PathBuf {
    app_file("Cargo.toml")
}

fn app_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// No window shows a document yet, and what pages are told is recorded.
fn documents() -> (Documents, Recorder) {
    let pages = Recorder::default();
    (Documents::new(pages.clone()), pages)
}

/// What [`Documents::give`] did, where every page can be told.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn give(documents: &Documents, document: DocumentAt<'_>, target: Target<'_>) -> Given {
    documents.give(document, target).unwrap()
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

/// `message` as sent to `window`'s page.
fn to(window: &str, message: Message) -> Sent {
    (window.to_owned(), message)
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
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn html_of(render: RenderTask) -> String {
    match block_on(render) {
        Ok(Ok((document, _))) => document.as_str().to_owned(),
        Ok(Err(error)) => panic!("expected a rendered document, got {error}"),
        Err(error) => panic!("the read and render stopped: {error}"),
    }
}

/// Waits until the render started ahead for `window` has read and rendered its document, so
/// that a test can change the file afterwards and tell the two reads apart.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn wait_for_render_ahead(documents: &Documents, window: &str) {
    for _ in 0..500 {
        let finished = documents.lock().shown[window]
            .as_ref()
            .and_then(|shown| shown.render_ahead.as_ref())
            .is_some_and(|task| task.inner().is_finished());
        if finished {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the render ahead for {window} didn't finish in 5 s");
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
            Place::Reload => panic!("a page that shows nothing yet was told to reload"),
        })
}

/// What the watch on `path`, given to `window`, does on a change to the file: the callback
/// [`Documents::watch`] hands the watch, so a test can report a change at a moment of its
/// choosing, as a watch's thread may.
fn watch_callback(documents: &Documents, window: &str, path: &Path) -> impl FnMut(Change) + use<> {
    Documents::on_change(
        Arc::downgrade(&documents.inner),
        window.to_owned(),
        document_key(path),
    )
}
