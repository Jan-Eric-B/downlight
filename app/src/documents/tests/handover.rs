//! The handover to a page that asks for its document: the render started ahead, and whether the
//! page shows the document or reloads it (`Place`, `Showing`). The answer as the page reads it is
//! a contract sample (`contract_samples` in `documents.rs`).

use super::*;

#[test]
fn window_given_a_new_document_renders_it_ahead() {
    let path = document("new", "# Read ahead");
    let (documents, _) = documents();
    let label = open_new(&documents, &path);
    wait_for_render_ahead(&documents, &label);
    std::fs::write(&path, "# Changed since").unwrap();
    let shown = rendered_html(&documents, &label);
    std::fs::remove_file(&path).unwrap();
    assert!(shown.contains("Read ahead"), "{shown}");
}

#[test]
fn window_changed_to_another_document_renders_the_new_one_ahead() {
    let first = document("changed-first", "# First document");
    let second = document("changed-second", "# Second document");
    let (documents, _) = documents();
    let label = open_new(&documents, &first);
    give(&documents, DocumentAt::new(&second), Target::Window(&label));
    wait_for_render_ahead(&documents, &label);
    std::fs::write(&second, "# Changed since").unwrap();
    let shown = rendered_html(&documents, &label);
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_file(&second).unwrap();
    assert!(shown.contains("Second document"), "{shown}");
}

#[test]
fn next_render_after_the_first_reads_the_file_again() {
    let path = document("again", "# First");
    let (documents, _) = documents();
    let label = open_new(&documents, &path);
    rendered_html(&documents, &label);
    std::fs::write(&path, "# Second").unwrap();
    let shown = rendered_html(&documents, &label);
    std::fs::remove_file(&path).unwrap();
    assert!(shown.contains("Second"), "{shown}");
}

#[test]
fn opening_an_open_document_again_starts_no_render_for_its_window() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let _ = documents.handover(&label, None);
    give(&documents, DocumentAt::new(&manifest()), Target::NewWindow);
    // Its window shows it already, so a render ahead would be read and then never used.
    let shown = documents.lock().shown[&label]
        .as_ref()
        .map(|shown| shown.render_ahead.is_some());
    assert_eq!(shown, Some(false));
}

#[test]
fn new_window_for_a_heading_link_keeps_the_heading() {
    let (documents, _) = documents();
    let given = give(
        &documents,
        DocumentAt {
            path: &manifest(),
            heading: Some("install"),
        },
        Target::NewWindow,
    );
    assert!(matches!(given, Given::NewWindow(_)), "{given:?}");
    assert_eq!(
        heading_of(&documents, "document-0"),
        Some(Some("install".to_owned()))
    );
}

#[test]
fn window_shown_a_document_at_a_heading_hands_it_over_with_every_render() {
    // Two links followed in quick succession each tell the page to ask, and the page keeps only
    // its latest request, which must still get the heading.
    let (documents, _) = documents();
    let label = documents.open_empty();
    give(
        &documents,
        DocumentAt {
            path: &manifest(),
            heading: Some("install"),
        },
        Target::Window(&label),
    );
    let first = heading_of(&documents, &label);
    let second = heading_of(&documents, &label);
    assert_eq!(first, Some(Some("install".to_owned())));
    assert_eq!(second, first);
}

#[test]
fn window_shown_a_document_without_a_heading_hands_over_none() {
    let (documents, _) = documents();
    let label = documents.open_empty();
    give(
        &documents,
        DocumentAt {
            path: &manifest(),
            heading: Some("install"),
        },
        Target::Window(&label),
    );
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    assert_eq!(heading_of(&documents, &label), Some(None));
}

#[test]
fn window_without_a_document_has_nothing_to_render() {
    let (documents, _) = documents();
    let label = documents.open_empty();
    assert!(documents.handover(&label, None).is_none());
}

#[test]
fn stale_reload_after_another_document_is_answered_with_a_show_at_its_heading() {
    // Ticket 23: the watch on the first document reported a save, and a link gave the window the
    // second before the page got `reload`. The page then gets `document-shown` and the stale
    // `reload`, in either order, and asks twice while its view still shows the first document.
    let first = document("stale-reload-first", "# First document");
    let second = document("stale-reload-second", "# Second document\n\n## Install");
    let (documents, pages) = documents();
    let label = open_new(&documents, &first);
    let mut on_first_change = watch_callback(&documents, &label, &first);
    let page_shows = handover(&documents, &label, None).showing;
    on_first_change(Change::Changed);
    let at_install = DocumentAt {
        path: &second,
        heading: Some("install"),
    };
    give(&documents, at_install, Target::Window(&label));
    let for_shown = handover(&documents, &label, Some(page_shows));
    let for_stale_reload = handover(&documents, &label, Some(page_shows));
    let shown = html_of(for_stale_reload.render);
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_file(&second).unwrap();
    let at_its_heading = Place::Show {
        heading: Some("install".to_owned()),
    };
    assert_eq!(for_shown.place, at_its_heading);
    assert_eq!(for_stale_reload.place, at_its_heading);
    assert_ne!(for_stale_reload.showing, page_shows);
    assert!(shown.contains("Second document"), "{shown}");
    assert_eq!(
        pages.sent(),
        [
            to(&label, Message::AskForDocument),
            to(&label, Message::AskForDocument)
        ]
    );
}

#[test]
fn request_by_hand_reads_the_file_again_as_a_reload_and_tells_the_page_nothing() {
    // F5, Ctrl+R and the menu's Reload ask with the showing the view shows (ticket 26), rather
    // than going through the app and back.
    let path = document("by-hand", "# First");
    let (documents, pages) = documents();
    let label = open_new(&documents, &path);
    let page_shows = handover(&documents, &label, None).showing;
    std::fs::write(&path, "# Second").unwrap();
    let reloaded = handover(&documents, &label, Some(page_shows));
    let shown = html_of(reloaded.render);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(reloaded.place, Place::Reload);
    assert!(shown.contains("Second"), "{shown}");
    assert_eq!(pages.sent(), []);
}

#[test]
fn render_is_timed_as_the_place_the_page_gets() {
    // A reload by hand reads the file anew on the page's request, which isn't a render started
    // for a reload, and is still timed as one (ticket 26).
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let shown = handover(&documents, &label, None);
    let reloaded = handover(&documents, &label, Some(shown.showing));
    assert_eq!(shown.place.reported_as(), "read-and-render");
    assert_eq!(reloaded.place.reported_as(), "reload");
}

#[test]
fn page_that_shows_the_current_document_is_answered_with_a_reload() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let first = handover(&documents, &label, None);
    let next = handover(&documents, &label, Some(first.showing));
    assert_eq!(first.place, Place::Show { heading: None });
    assert_eq!(next.place, Place::Reload);
    assert_eq!(next.showing, first.showing);
}

#[test]
fn reload_answer_keeps_the_heading_out() {
    // A reload keeps the reading position, so the heading a link named goes only with a show.
    let (documents, _) = documents();
    let label = documents.open_empty();
    let at_install = DocumentAt {
        path: &manifest(),
        heading: Some("install"),
    };
    give(&documents, at_install, Target::Window(&label));
    let shown = handover(&documents, &label, None);
    let reloaded = handover(&documents, &label, Some(shown.showing));
    assert_eq!(
        shown.place,
        Place::Show {
            heading: Some("install".to_owned())
        }
    );
    assert_eq!(reloaded.place, Place::Reload);
}

#[test]
fn window_given_back_a_document_it_showed_before_shows_it_anew() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let page_shows = handover(&documents, &label, None).showing;
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&label),
    );
    let again = handover(&documents, &label, Some(page_shows));
    assert_eq!(again.place, Place::Show { heading: None });
    assert_ne!(again.showing, page_shows);
}

#[test]
fn opening_a_shown_document_again_keeps_its_showing() {
    // Its window scrolls instead, and a reload after that still keeps the reading position.
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let page_shows = handover(&documents, &label, None).showing;
    give(&documents, DocumentAt::new(&manifest()), Target::NewWindow);
    let next = handover(&documents, &label, Some(page_shows));
    assert_eq!(next.place, Place::Reload);
}

#[test]
fn handover_names_the_document_s_file_and_its_full_path() {
    // For the title bar, which shows the name and the path as its tooltip (window design ticket
    // 04).
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    assert_eq!(
        handover(&documents, &label, None).file,
        DocumentFile {
            name: "Cargo.toml".to_owned(),
            path: manifest().display().to_string(),
        }
    );
}

#[test]
fn window_given_another_document_hands_over_the_new_file() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let _ = handover(&documents, &label, None);
    let other = app_file("build.rs");
    give(&documents, DocumentAt::new(&other), Target::Window(&label));
    assert_eq!(handover(&documents, &label, None).file.name, "build.rs");
}

#[test]
fn name_shown_for_a_path_without_one_is_never_blank() {
    // No document is a drive root, so this only guards the window's title and the title bar
    // against a blank name, should a path without a name of its own ever reach them.
    assert_eq!(file_name_shown(Path::new(r"C:\")), r"C:\");
}
