//! Copy as Markdown (copy-as-markdown ticket 01): the Markdown of the file as the render a
//! window's page shows was made from it. Each test replaces the clipboard with a recorder of what
//! is written to it, so no test touches the real clipboard.

use std::cell::RefCell;

use downlight_core::Piece;

use super::*;
use crate::copy::{ClipboardError, CopyError, copy};
use crate::documents::history::Step;

/// The clipboard as a test sees it: what was written to it, in turn.
#[derive(Default)]
struct TestClipboard {
    written: RefCell<Vec<String>>,
}

impl TestClipboard {
    /// Copies `pieces` of `window`'s document shown as `showing` to this clipboard.
    fn copy(
        &self,
        documents: &Documents,
        window: &str,
        showing: Showing,
        pieces: &[Piece],
    ) -> Result<(), CopyError> {
        copy(documents, window, showing, pieces, |markdown| {
            self.written.borrow_mut().push(markdown.to_owned());
            Ok(())
        })
    }

    fn written(&self) -> Vec<String> {
        self.written.borrow().clone()
    }
}

/// The lines from `first` to `last`.
const fn lines(first: usize, last: usize) -> Piece {
    Piece::Lines { first, last }
}

/// A window that shows `text` in a file of the test's own, which its page took: its label, the
/// showing its view shows, and the file.
fn shown(documents: &Documents, test: &str, text: &str) -> (String, Showing, PathBuf) {
    let path = document(test, text);
    let window = open_new(documents, &path);
    let showing = take(documents, &window, None);
    (window, showing, path)
}

#[test]
fn a_copy_writes_the_markdown_of_the_shown_render() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-shown", "# Title\n\nSome *text*.\n");
    let clipboard = TestClipboard::default();
    let whole = clipboard.copy(&documents, &window, showing, &[]);
    let part = clipboard.copy(&documents, &window, showing, &[lines(3, 3)]);
    std::fs::remove_file(&path).unwrap();
    assert!(whole.is_ok(), "{whole:?}");
    assert!(part.is_ok(), "{part:?}");
    assert_eq!(
        clipboard.written(),
        ["# Title\n\nSome *text*.", "Some *text*."]
    );
}

#[test]
fn a_copy_after_the_file_changed_writes_the_shown_render_while_its_reload_is_on_its_way() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-changed", "# Before\n");
    std::fs::write(&path, "# After\n").unwrap();
    // The page has the reload's answer, but still shows the render before it.
    let _reload = answered(&documents, &window, Some(showing));
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(clipboard.written(), ["# Before"]);
}

#[test]
fn a_copy_after_the_file_was_deleted_writes_the_shown_render() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-deleted", "# Kept\n");
    std::fs::remove_file(&path).unwrap();
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    assert_eq!(clipboard.written(), ["# Kept"]);
}

#[test]
fn a_copy_after_a_reload_that_cant_be_read_writes_the_render_the_page_kept() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-unread", "# Kept\n");
    std::fs::remove_file(&path).unwrap();
    // The reload says why it can't be read, and the page keeps its last render.
    take(&documents, &window, Some(showing));
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    assert_eq!(clipboard.written(), ["# Kept"]);
}

#[test]
fn a_copy_after_the_page_took_a_reload_writes_the_new_text() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-reloaded", "# Before\n");
    std::fs::write(&path, "# After\n").unwrap();
    let reloaded = take(&documents, &window, Some(showing));
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, reloaded, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(reloaded, showing);
    assert_eq!(clipboard.written(), ["# After"]);
}

#[test]
fn a_copy_while_another_document_is_on_its_way_writes_the_one_in_view() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-in-view", "# In view\n");
    let next = document("copy-on-its-way", "# On its way\n");
    give(&documents, DocumentAt::new(&next), Target::Window(&window));
    // Its answer has come, but the page hasn't placed it yet.
    let _on_its_way = answered(&documents, &window, Some(showing));
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&next).unwrap();
    assert_eq!(clipboard.written(), ["# In view"]);
}

#[test]
fn a_copy_after_a_stop_writes_the_document_put_back() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-stopped", "# Put back\n");
    let next = document("copy-stopped-next", "# Stopped\n");
    give(&documents, DocumentAt::new(&next), Target::Window(&window));
    let (_, stopped) = answered_with_bytes(&documents, &window, Some(showing));
    assert!(documents.stop(&window, Some(showing)));
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&next).unwrap();
    assert_eq!(clipboard.written(), ["# Put back"]);
    assert!(
        stopped.upgrade().is_none(),
        "the stopped document's bytes were kept"
    );
}

#[test]
fn a_render_that_finishes_after_a_stop_keeps_no_bytes() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-stopped-late", "# Put back\n");
    let next = document("copy-stopped-late-next", "# Stopped\n");
    give(&documents, DocumentAt::new(&next), Target::Window(&window));
    let Handover {
        render,
        request,
        showing: on_its_way,
        ..
    } = handover(&documents, &window, Some(showing));
    // The render is past its last check, as the core's sanitizing is once it has started, so
    // that leaving it with the stop doesn't end it.
    after_a_render_could_start();
    assert!(documents.stop(&window, Some(showing)));
    let late = block_on(documents.rendered(&window, render, request, on_its_way));
    let kept = weak_bytes(&late);
    drop(late);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&next).unwrap();
    assert!(
        kept.upgrade().is_none(),
        "a render that finished after the stop was kept"
    );
}

#[test]
fn a_copy_after_an_overtaken_render_writes_the_render_the_page_placed() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-overtaken", "# One\n");
    // The page asks twice, as after two saves in a row: the second render finishes first, and
    // then the first, which the page never places. Both read the same file, since a save would
    // start a reload of its own, so the bytes tell them apart by which read they came from.
    let first = handover(&documents, &window, Some(showing));
    // The first render is done, so that leaving it doesn't end it, as with a render that is past
    // its last check.
    after_a_render_could_start();
    let second = handover(&documents, &window, Some(showing));
    let second_answer =
        block_on(documents.rendered(&window, second.render, second.request, showing));
    let first_answer = block_on(documents.rendered(&window, first.render, first.request, showing));
    documents.arrived(&window, showing);
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    let (Ok(Ok(first_render)), Ok(Ok(second_render))) = (&first_answer, &second_answer) else {
        panic!("a render was left: {first_answer:?} {second_answer:?}");
    };
    let placed = documents.bytes_shown_as(&window, showing).unwrap();
    assert!(Arc::ptr_eq(&placed, &second_render.bytes));
    assert!(!Arc::ptr_eq(&placed, &first_render.bytes));
    assert_eq!(clipboard.written(), ["# One"]);
}

#[test]
fn a_copy_after_back_and_forward_writes_the_document_each_step_returned_to() {
    let (documents, _) = documents();
    let (window, first, path) = shown(&documents, "copy-steps-first", "# First\n");
    let next = document("copy-steps-second", "# Second\n");
    give(&documents, DocumentAt::new(&next), Target::Window(&window));
    let second = take(&documents, &window, Some(first));
    let clipboard = TestClipboard::default();
    documents.step(&window, Step::Back).unwrap();
    let back = take(&documents, &window, Some(second));
    clipboard.copy(&documents, &window, back, &[]).unwrap();
    // The showing it left is no longer the page's.
    clipboard.copy(&documents, &window, second, &[]).unwrap();
    documents.step(&window, Step::Forward).unwrap();
    let forward = take(&documents, &window, Some(back));
    clipboard.copy(&documents, &window, forward, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&next).unwrap();
    assert_eq!((back, forward), (first, second));
    assert_eq!(clipboard.written(), ["# First", "# Second"]);
}

#[test]
fn a_show_that_cant_be_read_lets_go_of_the_bytes_of_the_document_before() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-unreadable", "# Before\n");
    let kept = Arc::downgrade(&documents.bytes_shown_as(&window, showing).unwrap());
    let missing = std::env::temp_dir().join(format!(
        "downlight-documents-{}-copy-missing.md",
        std::process::id()
    ));
    give(
        &documents,
        DocumentAt::new(&missing),
        Target::Window(&window),
    );
    // The page places why the document can't be shown, in place of the render before.
    let (unread, on_its_way) = answered_with_bytes(&documents, &window, Some(showing));
    documents.arrived(&window, unread);
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    clipboard.copy(&documents, &window, unread, &[]).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(on_its_way.upgrade().is_none());
    assert!(
        kept.upgrade().is_none(),
        "the bytes of the document before were kept"
    );
    assert_eq!(clipboard.written(), Vec::<String>::new());
}

#[test]
fn a_copy_from_a_showing_the_window_has_left_writes_nothing_and_answers_without_a_notice() {
    let (documents, _) = documents();
    let (window, left, path) = shown(&documents, "copy-left", "# Left\n");
    let next = document("copy-left-next", "# Now\n");
    give(&documents, DocumentAt::new(&next), Target::Window(&window));
    let now = take(&documents, &window, Some(left));
    let clipboard = TestClipboard::default();
    let copied = clipboard.copy(&documents, &window, left, &[]);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&next).unwrap();
    assert_ne!(now, left);
    assert!(copied.is_ok(), "{copied:?}");
    assert_eq!(clipboard.written(), Vec::<String>::new());
}

#[test]
fn a_window_put_back_to_no_document_lets_go_of_the_bytes_its_page_placed_once_it_asks_again() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    // While the window is titled for its first document, its page asks for the document and
    // gets its answer, and then the title fails, which puts the window back to no document.
    let (other, name) = (shared(&documents), window.clone());
    let (sent, answer) = std::sync::mpsc::channel();
    recorder.before_next_title(move || {
        meanwhile(other, move |documents| {
            let _ = sent.send(answered_with_bytes(documents, &name, None));
        });
        Err(WindowError::new("the window can't be titled"))
    });
    let path = document("copy-put-back", "# Put back\n");
    let given = documents.give(DocumentAt::new(&path), Target::Window(&window));
    let (placed, bytes) = answer.recv_timeout(HELD_UP_END_WAIT).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(given, Err(ShowError::NotTaken(_))), "{given:?}");
    assert_eq!(documents.document_path(&window), None);
    // The page that was told nothing of this places its answer, and asks again when it is next
    // told to, getting the empty window.
    documents.arrived(&window, placed);
    assert!(documents.handover(&window, Some(placed)).is_none());
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, placed, &[]).unwrap();
    assert!(
        bytes.upgrade().is_none(),
        "the bytes outlived the window's document"
    );
    assert_eq!(clipboard.written(), Vec::<String>::new());
}

#[test]
fn a_busy_clipboard_answers_with_its_notice() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-busy", "# Title\n");
    let copied = copy(&documents, &window, showing, &[], |_| {
        Err(ClipboardError::Busy)
    });
    std::fs::remove_file(&path).unwrap();
    let Err(error) = copied else {
        panic!("a busy clipboard was written");
    };
    assert_eq!(
        Notice::from(error).to_string(),
        "The clipboard is in use by another program. Try again."
    );
}

#[test]
fn a_piece_the_file_doesnt_have_answers_with_its_notice_and_writes_nothing() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-outside", "# Title\n");
    let clipboard = TestClipboard::default();
    let copied = clipboard.copy(&documents, &window, showing, &[lines(2, 4)]);
    std::fs::remove_file(&path).unwrap();
    let Err(error) = copied else {
        panic!("a piece outside the file was copied");
    };
    assert_eq!(
        Notice::from(error).to_string(),
        "Can't copy the Markdown: the selection is no longer in the document."
    );
    assert_eq!(clipboard.written(), Vec::<String>::new());
}

// Copy as Markdown of part of a table (copy-as-markdown ticket 03).
const TABLE: &str = "| Part | Note |\n| --- | --- |\n| Wick | Trim *it* |\n";

#[test]
fn a_copy_of_a_cells_span_writes_the_cells_markdown() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-cell", TABLE);
    let clipboard = TestClipboard::default();
    // The note of the row on line 3, as its `td`'s line numbers (3:9-3:19) name it.
    let span = Piece::Span {
        line: 3,
        first: 9,
        last: 19,
    };
    let copied = clipboard.copy(&documents, &window, showing, &[span]);
    std::fs::remove_file(&path).unwrap();
    assert!(copied.is_ok(), "{copied:?}");
    assert_eq!(clipboard.written(), ["Trim *it*"]);
}

#[test]
fn a_span_that_splits_a_character_answers_with_its_notice_and_writes_nothing() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-split", "| Größe |\n| --- |\n");
    let clipboard = TestClipboard::default();
    // "ö" takes the fifth and sixth bytes of the line.
    let span = Piece::Span {
        line: 1,
        first: 6,
        last: 8,
    };
    let copied = clipboard.copy(&documents, &window, showing, &[span]);
    std::fs::remove_file(&path).unwrap();
    let Err(error) = copied else {
        panic!("a span inside a character was copied");
    };
    // The core's error stays the app's error's source, which the reader's notice doesn't say.
    assert_eq!(
        std::error::Error::source(&error).map(ToString::to_string),
        Some("columns 6 to 8 of line 1 split a character".to_owned())
    );
    assert_eq!(
        Notice::from(error).to_string(),
        "Can't copy the Markdown: the selection is no longer in the document."
    );
    assert_eq!(clipboard.written(), Vec::<String>::new());
}

#[test]
fn closing_the_window_lets_go_of_the_shown_renders_bytes() {
    let (documents, _) = documents();
    let (window, showing, path) = shown(&documents, "copy-closed", "# Title\n");
    let kept = Arc::downgrade(&documents.bytes_shown_as(&window, showing).unwrap());
    documents.forget(&window);
    std::fs::remove_file(&path).unwrap();
    assert!(kept.upgrade().is_none(), "the bytes outlived their window");
    let clipboard = TestClipboard::default();
    clipboard.copy(&documents, &window, showing, &[]).unwrap();
    assert_eq!(clipboard.written(), Vec::<String>::new());
}
