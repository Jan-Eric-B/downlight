//! Watching a window's document: a change to its file reloads it or tells the page it is gone,
//! a watch or change for a document the window has left is dropped, and a notice waits until the
//! page first asks.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError};

use super::*;

#[test]
fn reload_for_the_shown_document_tells_the_page_and_reads_the_file_again() {
    // The file isn't written: the window's real watch would report that too, during the pause.
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let mut on_change = watch_callback(&documents, &label, &manifest());
    let page_shows = handover(&documents, &label, None).showing;
    on_change(Change::Changed);
    let asked = after_a_render_could_start();
    let reloaded = handover(&documents, &label, Some(page_shows));
    let (shown, started) = html_and_start_of(reloaded.render);
    // Started by the change, since the page took the render ahead with its first request.
    assert!(
        started < asked,
        "the read started {started:?}, after {asked:?}"
    );
    assert_eq!(reloaded.place, Place::Reload);
    assert!(shown.contains("downlight"), "{shown}");
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn file_of_the_shown_document_back_again_reloads_it() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let mut on_change = watch_callback(&documents, &label, &manifest());
    on_change(Change::Back);
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn file_of_the_shown_document_gone_tells_the_page_with_its_showing() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let showing = handover(&documents, &label, None).showing;
    let mut on_change = watch_callback(&documents, &label, &manifest());
    on_change(Change::Gone);
    assert_eq!(pages.sent(), [to(&label, Message::Gone(showing))]);
}

#[test]
fn gone_names_the_showing_of_the_document_the_window_was_given_since() {
    // Ticket 26: the page shows the bar only for the showing its view shows, so a `gone` for the
    // new document, which reaches the page while its view still shows the old one, is told apart
    // from a stale one by its showing.
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let page_shows = handover(&documents, &label, None).showing;
    let other = app_file("build.rs");
    give(&documents, DocumentAt::new(&other), Target::Window(&label));
    let mut on_other_change = watch_callback(&documents, &label, &other);
    on_other_change(Change::Gone);
    let given = handover(&documents, &label, Some(page_shows)).showing;
    assert_ne!(given, page_shows);
    assert_eq!(
        pages.sent(),
        [
            to(&label, Message::AskForDocument),
            to(&label, Message::Gone(given))
        ]
    );
}

#[test]
fn reload_overtaken_by_another_document_sends_nothing() {
    // Ticket 21: the watch on the first document reports a save just after the window was given
    // the second, before that watch has stopped.
    let first = document("reload-left-first", "# First document");
    let (documents, pages) = documents();
    let label = open_new(&documents, &first);
    let mut on_first_change = watch_callback(&documents, &label, &first);
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&label),
    );
    // A render started by the change would replace the one started ahead, and start after this.
    let changed = after_a_render_could_start();
    on_first_change(Change::Changed);
    on_first_change(Change::Gone);
    let (shown, started) = html_and_start_of(handover(&documents, &label, None).render);
    std::fs::remove_file(&first).unwrap();
    assert!(
        started < changed,
        "the read started {started:?}, after {changed:?}"
    );
    assert!(shown.contains("downlight"), "{shown}");
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

/// A watch on `path` whose callback holds the sender of the returned receiver. A dropped watch
/// ends its thread, which drops the callback, so the receiver tells a running watch (Timeout)
/// from a dropped one (Disconnected).
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn watch_with_its_life(path: &Path) -> (Watch, Receiver<Change>) {
    let (sender, receiver) = mpsc::channel();
    let watch = Watch::start(path, move |change| {
        let _ = sender.send(change);
    })
    .unwrap();
    (watch, receiver)
}

/// Long enough for a dropped watch's thread to end.
const WATCH_END_WAIT: Duration = Duration::from_secs(1);

#[test]
fn watch_for_a_document_the_window_has_left_is_dropped() {
    let first = document("left-first", "# First");
    let second = document("left-second", "# Second");
    let (documents, _) = documents();
    let label = open_new(&documents, &first);
    give(&documents, DocumentAt::new(&second), Target::Window(&label));
    let (watch, life) = watch_with_its_life(&first);
    documents.keep_watch(&label, &document_key(&first), watch);
    let running = life.recv_timeout(WATCH_END_WAIT);
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_file(&second).unwrap();
    assert_eq!(running, Err(RecvTimeoutError::Disconnected));
}

#[test]
fn change_to_a_document_the_window_has_left_is_ignored() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let mut on_change = watch_callback(&documents, &label, &manifest());
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    for change in [Change::Changed, Change::Gone, Change::Back] {
        on_change(change);
    }
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn change_for_a_closed_window_is_ignored() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let mut on_change = watch_callback(&documents, &label, &manifest());
    documents.forget(&label);
    for change in [Change::Changed, Change::Gone, Change::Back] {
        on_change(change);
    }
    assert_eq!(pages.sent(), []);
}

/// Longer than any watch takes to report a save, so a missing report fails the test rather than
/// hangs it.
const REPORT_WAIT: Duration = Duration::from_secs(3);

/// Saves `path` until its watch has reported it to `window`'s page, and returns every message sent
/// by then. The watch starts on a thread of its own after the window was given the document, and a
/// save before it started goes unreported, so the file is saved again until one is reported.
#[expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn save_until_reported(pages: &Recorder, window: &str, path: &Path) -> Vec<Sent> {
    let before = pages.sent().len();
    for save in 0..10 {
        std::fs::write(path, format!("# Saved {save}")).unwrap();
        let sent = pages.wait_until(Duration::from_millis(500), |sent| sent.len() > before);
        if sent.len() > before {
            return sent;
        }
    }
    panic!("no save to {} was reported to {window}", path.display());
}

#[test]
fn document_given_and_then_saved_reloads_with_the_saved_text() {
    let path = document("saved", "# First");
    let (documents, pages) = documents();
    let label = open_new(&documents, &path);
    rendered_html(&documents, &label);
    let sent = save_until_reported(&pages, &label, &path);
    let shown = rendered_html(&documents, &label);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(sent.first(), Some(&to(&label, Message::AskForDocument)));
    assert!(shown.contains("Saved"), "{shown}");
}

#[test]
fn document_given_and_then_deleted_tells_the_page_it_is_gone() {
    let path = document("deleted", "# First");
    let (documents, pages) = documents();
    let label = open_new(&documents, &path);
    let showing = handover(&documents, &label, None).showing;
    save_until_reported(&pages, &label, &path);
    std::fs::remove_file(&path).unwrap();
    let gone = to(&label, Message::Gone(showing));
    let sent = pages.wait_until(REPORT_WAIT, |sent| sent.contains(&gone));
    assert_eq!(sent.last(), Some(&gone));
}

#[test]
fn window_forgotten_before_its_page_asks_hands_over_nothing_and_drops_its_late_watch() {
    let path = document("closed", "# First");
    let (documents, _) = documents();
    let label = open_new(&documents, &path);
    // The window closes straight after it was given its document: its render ahead may still
    // run, and its watch starts only after the window was forgotten.
    let (watch, life) = watch_with_its_life(&path);
    documents.forget(&label);
    let handover = documents.handover(&label, None);
    documents.keep_watch(&label, &document_key(&path), watch);
    let running = life.recv_timeout(WATCH_END_WAIT);
    std::fs::remove_file(&path).unwrap();
    assert!(handover.is_none());
    assert_eq!(running, Err(RecvTimeoutError::Disconnected));
}

/// A document whose file can't be watched: its folder is missing.
fn unwatchable() -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "downlight-documents-{}-missing",
            std::process::id()
        ))
        .join("unwatched.md")
}

/// Long enough for a watch that can't start to have failed: its folder is missing, which it
/// finds at once on its blocking thread. Only a stalled machine takes longer.
const WATCH_START_WAIT: Duration = Duration::from_secs(1);

#[test]
fn watch_that_cannot_start_is_noticed_once_the_page_asks() {
    // The notice is held, not sent: a page that hasn't asked yet may not listen (ticket 10).
    let (documents, pages) = documents();
    let label = open_new(&documents, &unwatchable());
    let before_asking = pages.wait_until(WATCH_START_WAIT, |sent| !sent.is_empty());
    let answered = handover(&documents, &label, None).notice;
    assert_eq!(before_asking, []);
    assert!(
        answered
            .as_ref()
            .is_some_and(|notice| notice.to_string().starts_with("Can't watch")),
        "{answered:?}"
    );
    assert_eq!(pages.sent(), []);
}

#[test]
fn notice_held_before_the_page_asks_comes_with_its_first_answer() {
    // In the answer rather than as a message: the page's listener for notices may not be
    // registered yet when it first asks, and a message with no listener is dropped (ticket 26).
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    documents.notice(&label, &document_key(&manifest()), "Can't watch".into());
    let first = handover(&documents, &label, None);
    let next = handover(&documents, &label, Some(first.showing));
    assert_eq!(first.notice, Some("Can't watch".into()));
    assert_eq!(next.notice, None);
    assert_eq!(pages.sent(), []);
}

#[test]
fn notice_after_the_page_asked_is_sent_now() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let _ = documents.handover(&label, None);
    documents.notice(&label, &document_key(&manifest()), "Can't watch".into());
    assert_eq!(
        pages.sent(),
        [to(&label, Message::Notice("Can't watch".into()))]
    );
}

#[test]
fn notice_for_a_document_the_window_has_left_is_dropped() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    documents.notice(&label, &document_key(&manifest()), "Can't watch".into());
    let answered = handover(&documents, &label, None).notice;
    assert_eq!(answered, None);
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn notice_for_a_document_the_window_had_before_is_not_sent_with_the_new_one() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    documents.notice(&label, &document_key(&manifest()), "Can't watch".into());
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    let answered = handover(&documents, &label, None).notice;
    assert_eq!(answered, None);
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}
