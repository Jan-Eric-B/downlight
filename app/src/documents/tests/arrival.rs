//! A document on its way (ticket 38): it joins the window's history only once its page says it
//! arrived, Stop puts the window back at the document in view, and going elsewhere during the
//! load goes from the document in view. A render nobody waits for any more is left.

use super::*;
use crate::documents::history::Step;
use crate::render::DocumentError;

/// A window that shows the manifest, which its page asked for and placed: its label and the
/// manifest's showing.
fn window_showing_the_manifest(documents: &Documents) -> (String, Showing) {
    let label = open_new(documents, &manifest());
    let showing = take(documents, &label, None);
    (label, showing)
}

/// `window`, whose page's view shows `page_shows`, is given `document`, and its page asks for it,
/// which is then on its way: the render it was handed.
fn start_loading(
    documents: &Documents,
    window: &str,
    page_shows: Option<Showing>,
    document: &Path,
) -> Handover {
    give(documents, DocumentAt::new(document), Target::Window(window));
    handover(documents, window, page_shows)
}

#[test]
fn stop_puts_the_window_back_at_the_document_in_view_with_its_history() {
    let (documents, recorder) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    documents.stop(&label, Some(first));
    assert_eq!(documents.document_path(&label), Some(manifest()));
    assert_eq!(
        recorder.done().last(),
        Some(&to(&label, titled("Cargo.toml - app - Downlight")))
    );
    let again = handover(&documents, &label, Some(first));
    assert_eq!(again.place, Place::Reload);
    assert_eq!(again.showing, first);
    assert_eq!(
        documents.step(&label, Step::Back).ok().flatten(),
        None,
        "the stopped document joined the history"
    );
}

#[test]
fn stop_in_a_new_window_leaves_it_empty_and_unwatched() {
    let (documents, recorder) = documents();
    let label = open_new(&documents, &manifest());
    let _loading = handover(&documents, &label, None);
    documents.stop(&label, None);
    assert_eq!(documents.document_path(&label), None);
    assert!(documents.handover(&label, None).is_none());
    assert_eq!(
        recorder.done().last(),
        Some(&to(&label, titled("Downlight")))
    );
}

#[test]
fn stop_from_a_page_at_another_showing_changes_nothing() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    documents.stop(&label, Some(loading.showing));
    documents.stop(&label, None);
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
}

#[test]
fn stop_once_the_document_arrived_changes_nothing() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    documents.arrived(&label, loading.showing);
    documents.stop(&label, Some(first));
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
}

#[test]
fn document_joins_history_once_the_page_says_it_arrived() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let second = start_loading(&documents, &label, Some(first), &app_file("build.rs")).showing;
    documents.arrived(&label, second);
    let third = start_loading(
        &documents,
        &label,
        Some(second),
        &app_file("tauri.conf.json"),
    );
    documents.arrived(&label, third.showing);
    documents.step(&label, Step::Back).unwrap();
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    take(&documents, &label, Some(third.showing));
    documents.step(&label, Step::Back).unwrap();
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn link_during_a_load_goes_from_the_document_in_view() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let third = start_loading(
        &documents,
        &label,
        Some(first),
        &app_file("tauri.conf.json"),
    );
    documents.arrived(&label, third.showing);
    documents.step(&label, Step::Back).unwrap();
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn back_during_a_load_goes_before_the_document_in_view() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let second = start_loading(&documents, &label, Some(first), &app_file("build.rs")).showing;
    documents.arrived(&label, second);
    let _loading = start_loading(
        &documents,
        &label,
        Some(second),
        &app_file("tauri.conf.json"),
    );
    documents.step(&label, Step::Back).unwrap();
    assert_eq!(documents.document_path(&label), Some(manifest()));
    let back = handover(&documents, &label, Some(second));
    assert_eq!(back.showing, first);
}

#[test]
fn link_in_the_document_in_view_resolves_during_a_load() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    assert_eq!(
        documents.document_path_shown_as(&label, first),
        Some(manifest())
    );
    assert_eq!(
        documents.document_path_shown_as(&label, loading.showing),
        Some(app_file("build.rs"))
    );
}

#[test]
fn document_in_view_during_a_load_is_still_open_in_its_window() {
    // ADR 0001: the document in view is still the window's, so opening it again brings the
    // window to the front rather than opening it in a second one.
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    assert_eq!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::AlreadyElsewhere(label)
    );
}

#[test]
fn save_to_the_document_in_view_during_a_load_reloads_it_after_stop() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let mut on_change = watch_callback(&documents, &label, &manifest());
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    on_change(Change::Changed);
    documents.stop(&label, Some(first));
    assert_eq!(
        pages.sent().last(),
        Some(&to(&label, Message::AskForDocument))
    );
    assert_eq!(
        handover(&documents, &label, Some(first)).place,
        Place::Reload
    );
}

#[test]
fn document_in_view_gone_during_a_load_tells_the_page_after_stop() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let mut on_change = watch_callback(&documents, &label, &manifest());
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    on_change(Change::Gone);
    let told = pages.sent().len();
    documents.stop(&label, Some(first));
    assert_eq!(pages.sent()[told..], [to(&label, Message::Gone(first))]);
}

#[test]
fn reload_overtaken_by_a_load_reloads_after_stop() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let mut on_change = watch_callback(&documents, &label, &manifest());
    on_change(Change::Changed);
    // The page asks for the reload, but a link overtakes it before it is placed.
    let _reload = handover(&documents, &label, Some(first));
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let told = pages.sent().len();
    documents.stop(&label, Some(first));
    assert_eq!(pages.sent()[told..], [to(&label, Message::AskForDocument)]);
}

#[test]
fn stop_without_a_change_to_the_document_in_view_tells_the_page_nothing() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let told = pages.sent().len();
    documents.stop(&label, Some(first));
    assert_eq!(pages.sent()[told..], []);
}

#[test]
fn second_request_during_a_load_leaves_the_first_render() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let first = handover(&documents, &label, None);
    let second = handover(&documents, &label, None);
    assert!(first.render.is_left());
    assert!(!second.render.is_left());
}

#[test]
fn stopping_or_leaving_a_document_leaves_its_render() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let stopped = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    documents.stop(&label, Some(first));
    assert!(stopped.render.is_left());

    let overtaken = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let _next = start_loading(
        &documents,
        &label,
        Some(first),
        &app_file("tauri.conf.json"),
    );
    assert!(overtaken.render.is_left());

    let closed = handover(&documents, &label, Some(first));
    documents.forget(&label);
    assert!(closed.render.is_left());
}

#[test]
fn render_the_page_placed_is_not_left() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let placed = handover(&documents, &label, None);
    documents.arrived(&label, placed.showing);
    assert!(!placed.render.is_left());
}

/// A binary file, which the core refuses, so a show of it fails.
fn binary_file() -> PathBuf {
    app_file("icons").join("icon.ico")
}

/// `window`'s page asked for `document`, which it was given while its view showed `page_shows`,
/// and placed why it can't be shown (ticket 38's review): the showing of that answer.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn place_a_failed_show(
    documents: &Documents,
    window: &str,
    page_shows: Option<Showing>,
    document: &Path,
) -> Showing {
    let failing = start_loading(documents, window, page_shows, document);
    let showing = failing.showing;
    match block_on(failing.render.finished()) {
        Ok(Err(DocumentError::NotText { .. })) => {}
        _ => panic!("expected {} to be refused", document.display()),
    }
    documents.arrived(window, showing);
    showing
}

#[test]
fn show_whose_render_fails_arrives_once_its_page_placed_why() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let mut on_change = watch_callback(&documents, &label, &manifest());
    place_a_failed_show(&documents, &label, Some(first), &binary_file());
    assert_eq!(documents.document_path_shown_as(&label, first), None);
    assert!(
        !documents.stop(&label, Some(first)),
        "the held record stayed"
    );
    let told = pages.sent().len();
    on_change(Change::Changed);
    assert_eq!(pages.sent()[told..], [], "the held document's watch stayed");
    documents.step(&label, Step::Back).unwrap();
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn stop_after_a_failed_show_and_a_further_load_puts_the_failed_show_back() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let failed = place_a_failed_show(&documents, &label, Some(first), &binary_file());
    // The page's view shows no render, only why the file can't be shown.
    let _loading = start_loading(&documents, &label, None, &app_file("build.rs"));
    assert!(documents.stop(&label, Some(failed)));
    assert_eq!(documents.document_path(&label), Some(binary_file()));
    documents.step(&label, Step::Back).unwrap();
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn failed_reload_its_page_told_of_is_not_reloaded_after_stop() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let mut on_change = watch_callback(&documents, &label, &manifest());
    on_change(Change::Changed);
    let _reload = handover(&documents, &label, Some(first));
    // The read failed, so the page kept the document in view, said why, and told the app.
    documents.arrived(&label, first);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let told = pages.sent().len();
    assert!(documents.stop(&label, Some(first)));
    assert_eq!(pages.sent()[told..], []);
}

#[test]
fn stop_tells_whether_it_stopped() {
    let (documents, _) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    assert!(
        !documents.stop(&label, Some(first)),
        "nothing was on its way"
    );
    let loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    assert!(!documents.stop(&label, Some(loading.showing)));
    assert!(documents.stop(&label, Some(first)));
}

#[test]
fn open_of_the_document_in_view_during_a_load_tells_its_page_to_stop() {
    // Ctrl+O or a drop of the document in view, as Stop: its page stops the load, which puts it
    // back at its reading position (ticket 38's review).
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let told = pages.sent().len();
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&manifest()),
            Target::Window(&label)
        ),
        Given::AlreadyHere(label.clone())
    );
    assert_eq!(
        pages.sent()[told..],
        [to(&label, Message::StopDocument(first))]
    );
    assert!(documents.stop(&label, Some(first)));
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn open_of_the_document_on_its_way_leaves_it_on_its_way() {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let told = pages.sent().len();
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&app_file("build.rs")),
            Target::Window(&label)
        ),
        Given::OnItsWayHere(label.clone())
    );
    assert_eq!(pages.sent()[told..], []);
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    assert!(documents.stop(&label, Some(first)), "the load was left");
}

#[test]
fn open_of_the_document_on_its_way_to_a_new_window_leaves_it_on_its_way() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let _loading = handover(&documents, &label, None);
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&manifest()),
            Target::Window(&label)
        ),
        Given::OnItsWayHere(label.clone())
    );
    assert_eq!(pages.sent(), []);
}

#[test]
fn link_from_another_window_to_a_heading_in_the_document_in_view_during_a_load_stops_it_and_scrolls_there()
 {
    let (documents, pages) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    let other = open_new(&documents, &app_file("tauri.conf.json"));
    take(&documents, &other, None);
    let told = pages.sent().len();
    let at_install = DocumentAt {
        path: &manifest(),
        heading: Some("install"),
    };
    assert_eq!(
        give(&documents, at_install, Target::Window(&other)),
        Given::AlreadyElsewhere(label.clone())
    );
    assert_eq!(
        pages.sent()[told..],
        [
            to(&label, Message::StopDocument(first)),
            to(
                &label,
                Message::ScrollToHeading {
                    showing: first,
                    heading: "install".to_owned()
                }
            ),
        ]
    );
    assert_eq!(
        documents.document_path(&other),
        Some(app_file("tauri.conf.json"))
    );
}

#[test]
fn back_in_another_window_to_the_document_in_view_during_a_load_stops_it() {
    let (documents, pages) = documents();
    let other = open_new(&documents, &manifest());
    let before = take(&documents, &other, None);
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&other),
    );
    take(&documents, &other, Some(before));
    let (label, first) = window_showing_the_manifest(&documents);
    let _loading = start_loading(
        &documents,
        &label,
        Some(first),
        &app_file("tauri.conf.json"),
    );
    let told = pages.sent().len();
    assert_eq!(
        documents.step(&other, Step::Back).unwrap(),
        Some(Given::AlreadyElsewhere(label.clone()))
    );
    assert_eq!(
        pages.sent()[told..],
        [to(&label, Message::StopDocument(first))]
    );
}

#[test]
fn change_that_fails_during_a_load_puts_the_document_on_its_way_back_with_the_one_in_view_held() {
    let (documents, recorder) = documents();
    let (label, first) = window_showing_the_manifest(&documents);
    let loading = start_loading(&documents, &label, Some(first), &app_file("build.rs"));
    recorder.fail_sends_to(&label);
    assert!(
        documents
            .give(
                DocumentAt::new(&app_file("tauri.conf.json")),
                Target::Window(&label)
            )
            .is_err()
    );
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    assert!(!loading.render.is_left(), "the load was left");
    assert!(documents.stop(&label, Some(first)));
    assert_eq!(documents.document_path(&label), Some(manifest()));
}
