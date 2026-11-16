//! Giving a window its document: which window shows it, what its page is told, and the scroll a
//! link to a heading sends the window that already shows the document.

use super::*;

#[test]
fn first_given_document_opens_a_new_window_that_shows_it() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn open_document_given_again_gives_its_window() {
    let (documents, _) = documents();
    let first = open_new(&documents, &manifest());
    assert_eq!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::AlreadyElsewhere(first)
    );
}

#[test]
fn open_document_is_found_ignoring_case() {
    let (documents, _) = documents();
    let first = open_new(&documents, &manifest());
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&app_file("CARGO.TOML")),
            Target::NewWindow
        ),
        Given::AlreadyElsewhere(first)
    );
}

#[test]
fn open_document_is_found_through_a_parent_folder_step() {
    let (documents, _) = documents();
    let first = open_new(&documents, &manifest());
    let roundabout = app_file("src").join("..").join("Cargo.toml");
    assert_eq!(
        give(&documents, DocumentAt::new(&roundabout), Target::NewWindow),
        Given::AlreadyElsewhere(first)
    );
}

#[test]
fn missing_document_given_again_in_other_case_gives_its_window() {
    let (documents, _) = documents();
    let first = open_new(&documents, &app_file("missing.md"));
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&app_file("MISSING.md")),
            Target::NewWindow
        ),
        Given::AlreadyElsewhere(first)
    );
}

#[test]
fn other_document_opens_another_window() {
    let (documents, _) = documents();
    let first = open_new(&documents, &manifest());
    let other = open_new(&documents, &app_file("build.rs"));
    assert_ne!(first, other);
}

#[test]
fn forgotten_window_gives_its_document_a_new_window() {
    let (documents, _) = documents();
    let first = open_new(&documents, &manifest());
    documents.forget(&first);
    assert!(matches!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::NewWindow(_)
    ));
    assert_eq!(documents.document_path(&first), None);
}

#[test]
fn empty_windows_are_never_the_same_window() {
    let (documents, _) = documents();
    let first = open_empty(&documents);
    let second = open_empty(&documents);
    assert_ne!(first, second);
    assert_eq!(documents.document_path(&first), None);
}

#[test]
fn new_window_is_told_nothing_since_its_page_asks_once_it_has_started() {
    let (documents, pages) = documents();
    open_new(&documents, &manifest());
    open_empty(&documents);
    assert_eq!(pages.sent(), []);
}

#[test]
fn empty_window_shows_the_document_it_is_given_and_its_page_is_told() {
    let (documents, pages) = documents();
    let label = open_empty(&documents);
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&manifest()),
            Target::Window(&label)
        ),
        Given::Changed(label.clone())
    );
    assert_eq!(documents.document_path(&label), Some(manifest()));
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn document_given_to_a_window_whose_page_cannot_be_told_is_not_taken() {
    // Ticket 22's open item 6: the page keeps its old document, so the reader learns the new one
    // isn't shown, and the window keeps the old one too, so its links and title stay with it.
    let (documents, recorder) = documents();
    let label = open_new(&documents, &manifest());
    let page_shows = handover(&documents, &label, None).showing;
    recorder.fail_sends_to(&label);
    let other = app_file("build.rs");
    let given = documents.give(DocumentAt::new(&other), Target::Window(&label));
    assert!(matches!(&given, Err(ShowError::NotTaken(_))), "{given:?}");
    assert_eq!(documents.document_path(&label), Some(manifest()));
    assert_eq!(
        documents.document_path_shown_as(&label, page_shows),
        Some(manifest())
    );
    assert_eq!(
        recorder.done(),
        [
            to(&label, opened("Cargo.toml - app - Downlight")),
            to(&label, titled("build.rs - app - Downlight")),
            to(&label, titled("Cargo.toml - app - Downlight")),
        ]
    );
    // The other document is open nowhere, and the window's own is found there still.
    assert!(matches!(
        give(&documents, DocumentAt::new(&other), Target::NewWindow),
        Given::NewWindow(_)
    ));
    assert_eq!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::AlreadyElsewhere(label)
    );
}

#[test]
fn document_given_to_a_window_that_cannot_be_titled_is_not_taken_and_its_page_is_not_told() {
    let (documents, recorder) = documents();
    let label = open_empty(&documents);
    recorder.fail_window(&label);
    let given = documents.give(DocumentAt::new(&manifest()), Target::Window(&label));
    assert!(matches!(&given, Err(ShowError::NotTaken(_))), "{given:?}");
    assert_eq!(documents.document_path(&label), None);
    assert_eq!(recorder.sent(), []);
}

#[test]
fn window_given_a_document_is_titled_after_it_and_its_page_told() {
    let (documents, recorder) = documents();
    let label = open_empty(&documents);
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&label),
    );
    assert_eq!(
        recorder.done(),
        [
            to(&label, opened("Downlight")),
            to(&label, titled("Cargo.toml - app - Downlight")),
        ]
    );
    assert_eq!(recorder.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn new_window_opens_titled_after_its_document_and_its_folder() {
    let (documents, recorder) = documents();
    let label = open_new(&documents, &manifest());
    assert_eq!(
        recorder.done(),
        [to(&label, opened("Cargo.toml - app - Downlight"))]
    );
}

#[test]
fn title_of_a_file_at_a_drive_root_names_the_drive() {
    // No launch, dialog or drop gives a path without a folder, so this is the title's own.
    assert_eq!(
        title_of(Some(Path::new(r"C:\README.md"))),
        r"README.md - C:\ - Downlight"
    );
}

#[test]
fn new_window_that_cannot_open_is_forgotten_so_its_document_opens_anew() {
    let (documents, recorder) = documents();
    recorder.fail_window("document-0");
    let given = documents.give(DocumentAt::new(&manifest()), Target::NewWindow);
    assert!(matches!(&given, Err(ShowError::NotOpened(_))), "{given:?}");
    assert_eq!(documents.document_path("document-0"), None);
    assert_eq!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::NewWindow("document-1".to_owned())
    );
}

#[test]
fn empty_window_that_cannot_open_is_forgotten() {
    let (documents, recorder) = documents();
    recorder.fail_window("document-0");
    assert!(documents.open_empty().is_err());
    assert_eq!(
        documents.give(DocumentAt::new(&manifest()), Target::Window("document-0")),
        Err(ShowError::WindowClosed)
    );
}

#[test]
fn open_document_given_again_brings_its_window_to_the_front() {
    // Also the window the request came from: a drop leaves the file manager in front.
    let (documents, recorder) = documents();
    let first = open_new(&documents, &manifest());
    let _ = documents.handover(&first, None);
    give(&documents, DocumentAt::new(&manifest()), Target::NewWindow);
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&first),
    );
    assert_eq!(
        recorder.done()[1..],
        [
            to(&first, ToWindow::BroughtToFront),
            to(&first, ToWindow::BroughtToFront)
        ]
    );
}

#[test]
fn open_document_whose_window_cannot_come_to_the_front_says_so() {
    let (documents, recorder) = documents();
    let first = open_new(&documents, &manifest());
    recorder.fail_window(&first);
    let given = documents.give(DocumentAt::new(&manifest()), Target::NewWindow);
    assert!(
        matches!(&given, Err(ShowError::NotBroughtToFront(_))),
        "{given:?}"
    );
}

#[test]
fn window_shows_another_document_in_place_of_its_own() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let other = app_file("build.rs");
    assert_eq!(
        give(&documents, DocumentAt::new(&other), Target::Window(&label)),
        Given::Changed(label.clone())
    );
    assert_eq!(documents.document_path(&label), Some(other));
    assert_eq!(pages.sent(), [to(&label, Message::AskForDocument)]);
}

#[test]
fn document_a_window_gave_up_opens_anew() {
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    assert!(matches!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::NewWindow(_)
    ));
}

#[test]
fn document_shown_in_a_window_is_found_there_again() {
    let (documents, _) = documents();
    let label = open_empty(&documents);
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&label),
    );
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&app_file("CARGO.TOML")),
            Target::NewWindow
        ),
        Given::AlreadyElsewhere(label)
    );
}

#[test]
fn document_open_in_another_window_stays_there() {
    let (documents, pages) = documents();
    let first = open_new(&documents, &manifest());
    let second = open_new(&documents, &app_file("build.rs"));
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&manifest()),
            Target::Window(&second)
        ),
        Given::AlreadyElsewhere(first)
    );
    assert_eq!(documents.document_path(&second), Some(app_file("build.rs")));
    assert_eq!(pages.sent(), []);
}

#[test]
fn window_document_path_is_the_one_it_shows() {
    let (documents, _) = documents();
    let label = open_new(&documents, &app_file("cargo.toml"));
    // As given, not as the key spells it, so the reader sees the name they know.
    assert_eq!(
        documents.document_path(&label),
        Some(app_file("cargo.toml"))
    );
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
}

#[test]
fn window_without_a_document_has_no_document_path() {
    let (documents, _) = documents();
    let label = open_empty(&documents);
    assert_eq!(documents.document_path(&label), None);
    assert_eq!(documents.document_path("document-99"), None);
}

#[test]
fn link_from_a_showing_the_window_has_left_has_no_document() {
    // Ticket 26: a link clicked in the old document while the next one renders would otherwise
    // resolve against the new document's folder.
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let left = handover(&documents, &label, None).showing;
    let other = app_file("build.rs");
    give(&documents, DocumentAt::new(&other), Target::Window(&label));
    let given = handover(&documents, &label, Some(left)).showing;
    assert_eq!(documents.document_path_shown_as(&label, left), None);
    assert_eq!(documents.document_path_shown_as(&label, given), Some(other));
    documents.forget(&label);
    assert_eq!(documents.document_path_shown_as(&label, given), None);
}

#[test]
fn document_given_to_its_own_window_is_already_there() {
    // A link to the shown document scrolls instead of rendering it again. The link was clicked in
    // the document, so the page has asked for it.
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let _ = documents.handover(&label, None);
    assert_eq!(
        give(
            &documents,
            DocumentAt {
                path: &app_file("CARGO.TOML"),
                heading: Some("install")
            },
            Target::Window(&label)
        ),
        Given::AlreadyHere(label)
    );
}

/// The document at `heading`.
fn at<'a>(path: &'a Path, heading: &'a str) -> DocumentAt<'a> {
    DocumentAt {
        path,
        heading: Some(heading),
    }
}

#[test]
fn request_from_the_window_that_shows_the_document_is_given_as_already_here() {
    // A link in the document, and a Ctrl+click on it, so the page has asked for its document.
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let _ = documents.handover(&label, None);
    assert_eq!(
        give(
            &documents,
            DocumentAt::new(&manifest()),
            Target::Window(&label)
        ),
        Given::AlreadyHere(label.clone())
    );
    assert_eq!(
        give(
            &documents,
            at(&manifest(), "install"),
            Target::NewWindowFrom(&label)
        ),
        Given::AlreadyHere(label.clone())
    );
}

#[test]
fn link_to_a_heading_in_the_document_its_own_window_shows_sends_that_window_nothing() {
    // That window's page scrolls on the link's own result (`Followed::ScrollTo`).
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let _ = documents.handover(&label, None);
    give(
        &documents,
        at(&manifest(), "install"),
        Target::Window(&label),
    );
    assert_eq!(pages.sent(), []);
}

#[test]
fn ctrl_click_on_a_heading_in_the_document_its_own_window_shows_sends_that_window_nothing() {
    let (documents, pages) = documents();
    let label = open_new(&documents, &manifest());
    let _ = documents.handover(&label, None);
    assert_eq!(
        give(
            &documents,
            at(&manifest(), "install"),
            Target::NewWindowFrom(&label)
        ),
        Given::AlreadyHere(label.clone())
    );
    assert_eq!(pages.sent(), []);
}

#[test]
fn ctrl_click_on_a_heading_in_a_document_open_in_another_window_scrolls_that_window() {
    let (documents, pages) = documents();
    let first = open_new(&documents, &manifest());
    let _ = documents.handover(&first, None);
    let second = open_new(&documents, &app_file("build.rs"));
    give(
        &documents,
        at(&manifest(), "install"),
        Target::NewWindowFrom(&second),
    );
    assert_eq!(
        pages.sent(),
        [to(&first, Message::ScrollToHeading("install".to_owned()))]
    );
}

#[test]
fn ctrl_click_on_a_document_open_nowhere_opens_a_new_window() {
    let (documents, _) = documents();
    let label = open_new(&documents, &app_file("build.rs"));
    let given = give(
        &documents,
        at(&manifest(), "install"),
        Target::NewWindowFrom(&label),
    );
    assert!(
        matches!(&given, Given::NewWindow(window) if *window != label),
        "{given:?}"
    );
}

#[test]
fn scroll_that_cannot_be_sent_still_gives_the_window_that_shows_the_document() {
    // Its window is brought to the front all the same, and the link's page has nothing to show.
    let (documents, pages) = documents();
    let first = open_new(&documents, &manifest());
    let _ = documents.handover(&first, None);
    let second = open_new(&documents, &app_file("build.rs"));
    pages.fail_sends_to(&first);
    let given = documents.give(at(&manifest(), "install"), Target::Window(&second));
    assert!(
        matches!(&given, Ok(Given::AlreadyElsewhere(window)) if *window == first),
        "{given:?}"
    );
}

#[test]
fn link_to_a_document_open_in_another_window_scrolls_it_only_with_a_heading() {
    // Without a heading, that window keeps its reading position (ticket 11's decisions).
    let (documents, pages) = documents();
    let first = open_new(&documents, &manifest());
    let _ = documents.handover(&first, None);
    let second = open_new(&documents, &app_file("build.rs"));
    let at_install = DocumentAt {
        path: &manifest(),
        heading: Some("install"),
    };
    give(&documents, at_install, Target::Window(&second));
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&second),
    );
    assert_eq!(
        pages.sent(),
        [to(&first, Message::ScrollToHeading("install".to_owned()))]
    );
}

#[test]
fn heading_for_an_open_window_whose_page_has_not_asked_is_held_for_its_first_render() {
    // Ctrl+click on `b.md#x` opens a window whose page listens for nothing until its webview has
    // started, so a heading sent to it now would be lost.
    let (documents, pages) = documents();
    let first = open_new(&documents, &manifest());
    let second = open_new(&documents, &app_file("build.rs"));
    assert_eq!(
        give(
            &documents,
            at(&manifest(), "install"),
            Target::Window(&second)
        ),
        Given::AlreadyElsewhere(first.clone())
    );
    assert_eq!(pages.sent(), []);
    assert_eq!(
        heading_of(&documents, &first),
        Some(Some("install".to_owned()))
    );
}

#[test]
fn showing_in_a_closed_window_changes_nothing_and_says_so_in_the_reader_s_terms() {
    // Ticket 28: it reached the reader as "window not found".
    let (documents, pages) = documents();
    let label = open_empty(&documents);
    documents.forget(&label);
    let given = documents.give(DocumentAt::new(&manifest()), Target::Window(&label));
    assert_eq!(given, Err(ShowError::WindowClosed));
    assert_eq!(ShowError::WindowClosed.to_string(), "its window has closed");
    assert!(matches!(
        give(&documents, DocumentAt::new(&manifest()), Target::NewWindow),
        Given::NewWindow(_)
    ));
    assert_eq!(pages.sent(), []);
}
