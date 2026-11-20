//! Back and Forward (ticket 12): the documents a window showed before and after the one it shows,
//! which a step gives it back with the showing it had, so that its page returns to where it was.

use super::*;
use crate::documents::history::{Step, StepError, Steps};

/// A window that showed the manifest and was then given `build.rs`, as a link, Ctrl+O or a drop
/// gives it, with its page asking each time and placing what it got: its label, and the showings
/// of the two.
fn window_that_moved_on(documents: &Documents) -> (String, Showing, Showing) {
    let label = open_new(documents, &manifest());
    let first = arrive(documents, &label, None);
    give(
        documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    let second = arrive(documents, &label, Some(first));
    (label, first, second)
}

/// What a step did, where every window opens and every page can be told.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn step(documents: &Documents, window: &str, step: Step) -> Option<Given> {
    documents.step(window, step).unwrap()
}

#[test]
fn back_gives_the_window_the_document_it_showed_before_as_the_showing_it_had() {
    let (documents, recorder) = documents();
    let (label, first, second) = window_that_moved_on(&documents);
    assert_eq!(
        step(&documents, &label, Step::Back),
        Some(Given::Changed(label.clone()))
    );
    assert_eq!(documents.document_path(&label), Some(manifest()));
    let returned = handover(&documents, &label, Some(second));
    assert_eq!(returned.place, Place::Return);
    assert_eq!(returned.showing, first);
    assert!(html_of(returned.render).contains("downlight"));
    assert_eq!(
        recorder.done().last(),
        Some(&to(&label, titled("Cargo.toml - app - Downlight")))
    );
    assert_eq!(
        recorder.sent(),
        [
            to(&label, Message::AskForDocument),
            to(&label, Message::AskForDocument)
        ]
    );
}

#[test]
fn forward_after_back_gives_the_window_the_document_it_left_as_its_showing() {
    let (documents, _) = documents();
    let (label, first, second) = window_that_moved_on(&documents);
    step(&documents, &label, Step::Back);
    arrive(&documents, &label, Some(second));
    assert_eq!(
        step(&documents, &label, Step::Forward),
        Some(Given::Changed(label.clone()))
    );
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    let returned = handover(&documents, &label, Some(first));
    assert_eq!(returned.place, Place::Return);
    assert_eq!(returned.showing, second);
}

#[test]
fn a_step_with_nowhere_to_go_changes_nothing_and_tells_the_page_nothing() {
    let (documents, recorder) = documents();
    let label = open_new(&documents, &manifest());
    let _ = handover(&documents, &label, None);
    assert_eq!(step(&documents, &label, Step::Back), None);
    assert_eq!(step(&documents, &label, Step::Forward), None);
    assert_eq!(documents.document_path(&label), Some(manifest()));
    assert_eq!(recorder.sent(), []);
}

#[test]
fn an_empty_window_given_a_document_has_nothing_to_go_back_to() {
    // The empty state is no document, so it is no step in the window's history.
    let (documents, _) = documents();
    let label = open_empty(&documents);
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&label),
    );
    assert_eq!(step(&documents, &label, Step::Back), None);
}

#[test]
fn a_document_given_after_back_leaves_nothing_to_go_forward_to() {
    let (documents, _) = documents();
    let (label, first, second) = window_that_moved_on(&documents);
    step(&documents, &label, Step::Back);
    arrive(&documents, &label, Some(second));
    give(
        &documents,
        DocumentAt::new(&app_file("src").join("main.rs")),
        Target::Window(&label),
    );
    arrive(&documents, &label, Some(first));
    assert_eq!(step(&documents, &label, Step::Forward), None);
    assert_eq!(
        step(&documents, &label, Step::Back),
        Some(Given::Changed(label.clone()))
    );
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn every_document_given_is_one_more_step_back() {
    let (documents, _) = documents();
    let (label, _, second) = window_that_moved_on(&documents);
    give(
        &documents,
        DocumentAt::new(&app_file("src").join("main.rs")),
        Target::Window(&label),
    );
    let third = arrive(&documents, &label, Some(second));
    step(&documents, &label, Step::Back);
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    arrive(&documents, &label, Some(third));
    step(&documents, &label, Step::Back);
    assert_eq!(documents.document_path(&label), Some(manifest()));
    arrive(&documents, &label, Some(second));
    assert_eq!(step(&documents, &label, Step::Back), None);
}

#[test]
fn handover_says_whether_the_window_can_go_back_or_forward() {
    // For the page's menu, which offers Back and Forward only when there is somewhere to go.
    let (documents, _) = documents();
    let label = open_new(&documents, &manifest());
    let alone = handover(&documents, &label, None);
    documents.arrived(&label, alone.showing);
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&label),
    );
    let moved_on = handover(&documents, &label, Some(alone.showing));
    documents.arrived(&label, moved_on.showing);
    step(&documents, &label, Step::Back);
    let back = handover(&documents, &label, Some(moved_on.showing));
    let steps = |back, forward| Steps { back, forward };
    assert_eq!(alone.steps, steps(false, false));
    assert_eq!(moved_on.steps, steps(true, false));
    assert_eq!(back.steps, steps(false, true));
}

#[test]
fn back_to_a_document_open_in_another_window_brings_that_window_to_the_front() {
    // A document is shown in at most one window (ADR 0001), so the window stays where it is, as
    // it does when Ctrl+O chooses that document.
    let (documents, recorder) = documents();
    let (label, _, _) = window_that_moved_on(&documents);
    let other = open_new(&documents, &manifest());
    assert_eq!(
        step(&documents, &label, Step::Back),
        Some(Given::AlreadyElsewhere(other.clone()))
    );
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    assert_eq!(
        recorder.done().last(),
        Some(&to(&other, ToWindow::BroughtToFront))
    );
    // The step is still there to take once the other window has closed.
    documents.forget(&other);
    step(&documents, &label, Step::Back);
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn back_that_cannot_change_the_window_keeps_its_document_and_its_history() {
    let (documents, recorder) = documents();
    let (label, _, second) = window_that_moved_on(&documents);
    recorder.fail_sends_to(&label);
    let stepped = documents.step(&label, Step::Back);
    assert!(
        matches!(
            &stepped,
            Err(StepError::Show {
                source: ShowError::NotTaken(_),
                ..
            })
        ),
        "{stepped:?}"
    );
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
    assert_eq!(
        documents.document_path_shown_as(&label, second),
        Some(app_file("build.rs"))
    );
    assert_eq!(
        handover(&documents, &label, Some(second)).steps,
        Steps {
            back: true,
            forward: false
        }
    );
    assert_eq!(
        recorder.done().last(),
        Some(&to(&label, titled("build.rs - app - Downlight")))
    );
}

#[test]
fn back_that_fails_after_forward_and_back_again_keeps_where_they_took_the_window() {
    // From the review of ticket 12: a step gives the window back the showing it had, so a failed
    // Back that found its own showing in the window again, after a Forward and a Back on another
    // thread, took that for its own and put back the document Back had left, with its history.
    let (documents, recorder) = documents();
    // Each step's document arrives before the next, so that each goes from the one before.
    let (label, first, second) = window_that_moved_on(&documents);
    let (other, window) = (shared(&documents), label.clone());
    recorder.before_next_title(move || {
        meanwhile(other, move |documents| {
            documents.arrived(&window, first);
            step(documents, &window, Step::Forward);
            documents.arrived(&window, second);
            step(documents, &window, Step::Back);
        });
        Err(WindowError::new("the window can't be titled"))
    });
    assert!(documents.step(&label, Step::Back).is_err());
    recorder.wait_until(HELD_UP_END_WAIT, |sent| sent.len() == 4);
    assert_eq!(documents.document_path(&label), Some(manifest()));
    documents.arrived(&label, first);
    assert_eq!(
        step(&documents, &label, Step::Forward),
        Some(Given::Changed(label.clone()))
    );
    assert_eq!(documents.document_path(&label), Some(app_file("build.rs")));
}

#[test]
fn a_link_from_the_document_a_step_left_does_nothing() {
    // Ticket 26's rule holds for a step too: only the showing in view has links, the one stepped
    // from until the one stepped to has arrived (ticket 38).
    let (documents, _) = documents();
    let (label, first, second) = window_that_moved_on(&documents);
    step(&documents, &label, Step::Back);
    arrive(&documents, &label, Some(second));
    assert_eq!(documents.document_path_shown_as(&label, second), None);
    assert_eq!(
        documents.document_path_shown_as(&label, first),
        Some(manifest())
    );
}

#[test]
fn a_step_in_a_closed_window_says_so_in_the_reader_s_terms() {
    let (documents, _) = documents();
    let (label, _, _) = window_that_moved_on(&documents);
    documents.forget(&label);
    assert!(matches!(
        documents.step(&label, Step::Back),
        Err(StepError::WindowClosed)
    ));
}

#[test]
fn a_document_stepped_back_to_is_read_anew_and_reloads_on_a_save() {
    // The document's file may have changed while the window showed another, so it is read anew;
    // and a save to it reloads it as for any document the window shows.
    let path = document("step-watched", "# Before");
    let (documents, recorder) = documents();
    let label = open_new(&documents, &path);
    let first = arrive(&documents, &label, None);
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&label),
    );
    let second = arrive(&documents, &label, Some(first));
    std::fs::write(&path, "# After").unwrap();
    step(&documents, &label, Step::Back);
    let shown = html_of(handover(&documents, &label, Some(second)).render);
    let sent_before_save = recorder.sent().len();
    let mut on_change = watch_callback(&documents, &label, &path);
    on_change(Change::Changed);
    std::fs::remove_file(&path).unwrap();
    assert!(shown.contains("After"), "{shown}");
    assert_eq!(
        recorder.sent()[sent_before_save..],
        [to(&label, Message::AskForDocument)]
    );
}

#[test]
fn a_heading_visited_after_back_leaves_nothing_to_go_forward_to() {
    // As in a browser, going somewhere new drops what Forward would have gone to; a heading is
    // somewhere new too, though only the page keeps where it is.
    let (documents, _) = documents();
    let (label, first, second) = window_that_moved_on(&documents);
    step(&documents, &label, Step::Back);
    arrive(&documents, &label, Some(second));
    documents.forget_forward(&label, first);
    assert_eq!(step(&documents, &label, Step::Forward), None);
    assert_eq!(documents.document_path(&label), Some(manifest()));
}

#[test]
fn a_heading_visited_in_a_showing_the_window_has_left_keeps_its_history() {
    // The heading was visited in the document the window had just stepped back from, before the
    // page placed the one stepped to: by then the window's view has left it.
    let (documents, _) = documents();
    let (label, _, second) = window_that_moved_on(&documents);
    step(&documents, &label, Step::Back);
    arrive(&documents, &label, Some(second));
    documents.forget_forward(&label, second);
    assert_eq!(
        step(&documents, &label, Step::Forward),
        Some(Given::Changed(label.clone()))
    );
}
