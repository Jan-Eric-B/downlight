//! Watching a window's document: a change to its file reloads it or tells the page it is gone,
//! a change for a document the window has left is ignored, and a notice waits until the page
//! first asks.

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
fn window_forgotten_before_its_page_asks_hands_over_nothing() {
    // The window closes straight after it was given its document, while its render ahead may
    // still run.
    let path = document("closed", "# First");
    let (documents, _) = documents();
    let label = open_new(&documents, &path);
    documents.forget(&label);
    let handover = documents.handover(&label, None);
    std::fs::remove_file(&path).unwrap();
    assert!(handover.is_none());
}

/// Longer than the watch's check takes to find a file its folder's events didn't report.
const RECHECK_WAIT: Duration = Duration::from_secs(5);

/// Long enough for a window's watch to have started, on its blocking thread, and made the
/// comparison it starts with. Only a stalled machine takes longer.
const WATCH_START_WAIT: Duration = Duration::from_secs(1);

/// Several times the 150 ms a watch waits for quiet before it reports, yet well under its
/// RECHECK.
const QUIET_WAIT: Duration = Duration::from_millis(500);

#[test]
fn document_whose_folder_is_missing_is_asked_for_again_once_its_file_is_written() {
    // Story 40: the window watches for the folder and its file, as after a build wiped the
    // folder, rather than telling the reader it can't watch it.
    let folder = std::env::temp_dir().join(format!(
        "downlight-documents-{}-missing",
        std::process::id()
    ));
    let path = folder.join("later.md");
    let (documents, pages) = documents();
    let label = open_new(&documents, &path);
    let answered = handover(&documents, &label, None).notice;
    let before_written = pages.wait_until(WATCH_START_WAIT, |sent| !sent.is_empty());
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(&path, "# Written at last").unwrap();
    let sent = pages.wait_until(RECHECK_WAIT, |sent| !sent.is_empty());
    std::fs::remove_dir_all(&folder).unwrap();
    assert_eq!(answered, None);
    assert_eq!(before_written, []);
    assert_eq!(sent, [to(&label, Message::AskForDocument)]);
}

#[test]
fn window_getting_the_focus_reloads_a_file_its_watch_missed() {
    // A deleted folder's watch has stopped, and the folder made again has none until a check
    // finds it, so only a check can see its file: as with any change the folder's watch misses.
    let folder =
        std::env::temp_dir().join(format!("downlight-documents-{}-focus", std::process::id()));
    let path = folder.join("focus.md");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(&path, "# First").unwrap();
    let (documents, pages) = documents();
    let label = open_new(&documents, &path);
    let showing = handover(&documents, &label, None).showing;
    save_until_reported(&pages, &label, &path);
    std::fs::remove_dir_all(&folder).unwrap();
    let gone = to(&label, Message::Gone(showing));
    let before = pages
        .wait_until(REPORT_WAIT, |sent| sent.contains(&gone))
        .len();
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(&path, "# Made again").unwrap();
    // Well inside the RECHECK after the `gone`, so only the focus can find the file.
    documents.activated(&label);
    let sent = pages.wait_until(QUIET_WAIT, |sent| sent.len() > before);
    std::fs::remove_dir_all(&folder).unwrap();
    assert_eq!(sent.get(before), Some(&to(&label, Message::AskForDocument)));
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
fn folder_whose_events_cannot_be_watched_tells_the_reader_saves_reload_late() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let mut on_report = report_callback(&documents, &label, &manifest());
    on_report(Report::EventsUnavailable {
        reason: "Access is denied.".to_owned(),
        checked_every: Duration::from_secs(2),
    });
    let answered = handover(&documents, &label, None).notice;
    assert_eq!(
        answered,
        Some(Notice::naming(
            "Can't watch ",
            crate::notice::NamedFile::at(&manifest()),
            " for changes as they are made, so a save reloads it up to 2 seconds later: \
             Access is denied.",
        ))
    );
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
