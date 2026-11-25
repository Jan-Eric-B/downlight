//! Paste (paste-markdown tickets 01, 02 and 04, ADR 0006): the clipboard's text, or text dropped
//! onto a window, written to a file in the pasted folder, which the window is then given as Open
//! and a file drop give it, and deleted once no window can return to it. Each test hands the paste
//! module the clipboard's answer or the dropped text and gives the documents module a pasted
//! folder of its own, so no test reads the real clipboard or a real drag, or writes to or deletes
//! from the real pasted folder.

use super::*;
use crate::documents::history::Step;
use crate::paste::{Clipboard, LocalTime, PastedFolder, paste, show_dropped_text};

/// A pasted folder of the test's own, which is gone once the test ends. Not created: the paste
/// module creates it when the first paste needs it.
struct TestFolder(PathBuf);

/// The file the first paste at [`AT`] is written to.
const FIRST: &str = "Pasted 2026-11-20 14.32.05.md";

/// The file the second paste at [`AT`] is written to.
const SECOND: &str = "Pasted 2026-11-20 14.32.05 (2).md";

impl TestFolder {
    #[expect(
        clippy::unwrap_used,
        reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
    )]
    fn new(test: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("downlight-pasted-{}-{test}", std::process::id()));
        if folder.exists() {
            std::fs::remove_dir_all(&folder).unwrap();
        }
        Self(folder)
    }

    /// No window shows a document yet, and pastes go to this folder.
    fn documents(&self) -> (Documents, Recorder) {
        recorded_pasting(PastedFolder::at(self.0.clone()))
    }

    /// The file named `name` in the folder.
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Whether the folder has a file named `name`, once every pasted file let go of so far is
    /// deleted.
    fn has(&self, documents: &Documents, name: &str) -> bool {
        documents.finish_deleting_pastes();
        self.file(name).exists()
    }

    /// The names of the files in the folder, in order; none while the folder doesn't exist.
    fn names(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.0) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TestFolder {
    fn drop(&mut self) {
        // A folder a test never made, or one already gone, leaves nothing to clean up.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 2026-11-20 14:32:05 in local time.
const AT: LocalTime = LocalTime {
    year: 2026,
    month: 11,
    day: 20,
    hour: 14,
    minute: 32,
    second: 5,
};

/// Pastes `text` into `window`, at [`AT`], into the documents module's pasted folder.
fn paste_text(documents: &Documents, window: &str, text: &str) -> Result<(), Notice> {
    paste(documents, window, Clipboard::Text(text.to_owned()), AT).map_err(Notice::from)
}

/// Pastes into `window`, as [`paste_text`], where every paste is shown.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn paste_into(documents: &Documents, window: &str) {
    paste_text(documents, window, "# Pasted").unwrap();
}

#[test]
fn a_paste_into_the_empty_window_shows_a_new_file_named_by_the_time_holding_the_text() {
    let folder = TestFolder::new("empty-window");
    let (documents, recorder) = folder.documents();
    let window = open_empty(&documents);
    let text = "# Lamplighter\n\nGrüße, 灯\n";
    assert_eq!(paste_text(&documents, &window, text), Ok(()));
    let file = folder.0.join("Pasted 2026-11-20 14.32.05.md");
    assert_eq!(documents.document_path(&window), Some(file.clone()));
    // UTF-8 without a byte order mark.
    assert_eq!(std::fs::read(&file).ok(), Some(text.as_bytes().to_vec()));
    assert_eq!(recorder.sent(), [to(&window, Message::AskForDocument)]);
    assert!(rendered_html(&documents, &window).contains("Lamplighter"));
}

#[test]
fn two_pastes_in_the_same_second_get_two_files() {
    let folder = TestFolder::new("same-second");
    let (documents, _) = folder.documents();
    let window = open_empty(&documents);
    assert_eq!(paste_text(&documents, &window, "first"), Ok(()));
    arrive(&documents, &window, None);
    assert_eq!(paste_text(&documents, &window, "second"), Ok(()));
    assert_eq!(
        folder.names(),
        [
            "Pasted 2026-11-20 14.32.05 (2).md",
            "Pasted 2026-11-20 14.32.05.md"
        ]
    );
    let second = folder.0.join("Pasted 2026-11-20 14.32.05 (2).md");
    assert_eq!(documents.document_path(&window), Some(second.clone()));
    assert_eq!(
        std::fs::read_to_string(second).ok().as_deref(),
        Some("second")
    );
    assert_eq!(
        std::fs::read_to_string(folder.0.join("Pasted 2026-11-20 14.32.05.md"))
            .ok()
            .as_deref(),
        Some("first")
    );
}

#[test]
fn a_paste_joins_history_and_back_returns_to_the_document_before_it_where_it_was() {
    let folder = TestFolder::new("history");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    assert_eq!(paste_text(&documents, &window, "# Pasted"), Ok(()));
    let pasted = arrive(&documents, &window, Some(before));
    assert_eq!(
        documents.step(&window, Step::Back).ok().flatten(),
        Some(Given::Changed(window.clone()))
    );
    assert_eq!(documents.document_path(&window), Some(manifest()));
    // Return: the page goes back to the reading position it had there.
    let returned = handover(&documents, &window, Some(pasted));
    assert_eq!(returned.place, Place::Return);
    assert_eq!(returned.showing, before);
}

#[test]
fn a_paste_replaces_a_document_still_on_its_way() {
    let folder = TestFolder::new("on-its-way");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&window),
    );
    let on_its_way = handover(&documents, &window, Some(before));
    assert_eq!(paste_text(&documents, &window, "# Pasted"), Ok(()));
    assert!(on_its_way.render.is_left());
    let file = folder.0.join("Pasted 2026-11-20 14.32.05.md");
    assert_eq!(documents.document_path(&window), Some(file));
    arrive(&documents, &window, Some(before));
    // The document on its way never arrived, so Back goes to the one in view before it.
    documents.step(&window, Step::Back).ok();
    assert_eq!(documents.document_path(&window), Some(manifest()));
}

#[test]
fn text_with_nothing_to_show_says_the_clipboard_holds_no_text_and_changes_nothing() {
    for (name, text) in [("empty", ""), ("whitespace", " \r\n\t\n  ")] {
        let folder = TestFolder::new(&format!("nothing-{name}"));
        let (documents, recorder) = folder.documents();
        let window = open_new(&documents, &manifest());
        assert_eq!(
            paste_text(&documents, &window, text),
            Err(Notice::from(
                "The clipboard holds no text. To open a file, drop it here or use Open."
            )),
            "{name}"
        );
        assert_eq!(documents.document_path(&window), Some(manifest()), "{name}");
        assert_eq!(recorder.sent(), [], "{name}");
        assert!(!folder.0.exists(), "{name}");
    }
}

#[test]
fn text_that_would_be_a_binary_file_says_downlight_opens_text_files_and_changes_nothing() {
    let folder = TestFolder::new("binary");
    let (documents, recorder) = folder.documents();
    let window = open_new(&documents, &manifest());
    assert_eq!(
        paste_text(&documents, &window, "MZ\0\0\u{3}"),
        Err(Notice::from("Downlight opens text files."))
    );
    assert_eq!(documents.document_path(&window), Some(manifest()));
    assert_eq!(recorder.sent(), []);
    assert!(!folder.0.exists());
}

#[test]
fn a_clipboard_that_gives_no_text_says_why_and_changes_nothing() {
    for (clipboard, said) in [
        (
            Clipboard::NoText,
            "The clipboard holds no text. To open a file, drop it here or use Open.",
        ),
        (
            Clipboard::Busy,
            "The clipboard is in use by another program. Try again.",
        ),
        (
            Clipboard::NotAvailable,
            "Paste isn't available on this system yet.",
        ),
    ] {
        let folder = TestFolder::new("no-text");
        let (documents, recorder) = folder.documents();
        let window = open_empty(&documents);
        let pasted = paste(&documents, &window, clipboard, AT);
        assert_eq!(pasted.map_err(Notice::from), Err(Notice::from(said)));
        assert_eq!(documents.document_path(&window), None);
        assert_eq!(recorder.sent(), []);
        assert!(!folder.0.exists());
    }
}

#[test]
fn a_paste_that_cannot_be_written_names_the_file_and_changes_nothing() {
    // A file where the pasted folder should be, so the folder can't be made.
    let folder = TestFolder::new("unwritable");
    std::fs::write(&folder.0, "in the way").ok();
    let (documents, recorder) = folder.documents();
    let window = open_new(&documents, &manifest());
    let refused = paste_text(&documents, &window, "# Pasted");
    std::fs::remove_file(&folder.0).ok();
    let Err(Notice::NamingFile { before, file, .. }) = refused else {
        panic!("expected a notice naming the file, got {refused:?}");
    };
    assert_eq!(before, "Can't write ");
    assert_eq!(file, crate::notice::NamedFile::at(&folder.0));
    assert_eq!(documents.document_path(&window), Some(manifest()));
    assert_eq!(recorder.sent(), []);
}

#[test]
fn a_paste_is_deleted_when_its_window_closes() {
    let folder = TestFolder::new("window-closes");
    let (documents, _) = folder.documents();
    let window = open_empty(&documents);
    paste_into(&documents, &window);
    arrive(&documents, &window, None);
    assert!(folder.has(&documents, FIRST));
    documents.forget(&window);
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_left_by_going_somewhere_new_after_back_is_deleted() {
    let folder = TestFolder::new("somewhere-new");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    paste_into(&documents, &window);
    let pasted = arrive(&documents, &window, Some(before));
    documents.step(&window, Step::Back).ok();
    let back = arrive(&documents, &window, Some(pasted));
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&window),
    );
    // Until the new document arrives, Stop could return to where Forward goes to the paste.
    assert!(folder.has(&documents, FIRST));
    arrive(&documents, &window, Some(back));
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_left_by_visiting_a_heading_after_back_is_deleted() {
    let folder = TestFolder::new("heading-after-back");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    paste_into(&documents, &window);
    let pasted = arrive(&documents, &window, Some(before));
    documents.step(&window, Step::Back).ok();
    let back = arrive(&documents, &window, Some(pasted));
    // Forward still goes to it.
    assert!(folder.has(&documents, FIRST));
    documents.forget_forward(&window, back);
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_stopped_on_its_way_is_deleted() {
    let folder = TestFolder::new("stopped");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    paste_into(&documents, &window);
    assert!(folder.has(&documents, FIRST));
    assert!(documents.stop(&window, Some(before)));
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_replaced_on_its_way_by_another_paste_is_deleted() {
    let folder = TestFolder::new("replaced-by-paste");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    arrive(&documents, &window, None);
    paste_into(&documents, &window);
    assert!(folder.has(&documents, FIRST));
    paste_into(&documents, &window);
    assert!(!folder.has(&documents, FIRST));
    assert!(folder.has(&documents, SECOND));
}

#[test]
fn a_paste_replaced_on_its_way_by_another_document_is_deleted() {
    let folder = TestFolder::new("replaced-by-document");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    arrive(&documents, &window, None);
    paste_into(&documents, &window);
    assert!(folder.has(&documents, FIRST));
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&window),
    );
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_its_window_could_not_change_to_is_deleted() {
    let folder = TestFolder::new("not-taken");
    let (documents, recorder) = folder.documents();
    let window = open_new(&documents, &manifest());
    arrive(&documents, &window, None);
    recorder.fail_sends_to(&window);
    assert!(paste_text(&documents, &window, "# Pasted").is_err());
    assert_eq!(documents.document_path(&window), Some(manifest()));
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_into_a_window_that_has_closed_is_deleted() {
    let folder = TestFolder::new("window-closed");
    let (documents, _) = folder.documents();
    let window = open_empty(&documents);
    documents.forget(&window);
    assert!(paste_text(&documents, &window, "# Pasted").is_err());
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_stays_while_its_window_holds_it_in_history() {
    let folder = TestFolder::new("in-history");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    paste_into(&documents, &window);
    let pasted = arrive(&documents, &window, Some(before));
    documents.step(&window, Step::Back).ok();
    let back = arrive(&documents, &window, Some(pasted));
    assert!(folder.has(&documents, FIRST));
    documents.step(&window, Step::Forward).ok();
    arrive(&documents, &window, Some(back));
    assert_eq!(documents.document_path(&window), Some(folder.file(FIRST)));
    assert!(folder.has(&documents, FIRST));
}

#[test]
fn a_paste_stays_while_another_window_shows_it_or_holds_it_in_history_and_goes_with_the_last() {
    let folder = TestFolder::new("other-window");
    let (documents, _) = folder.documents();
    let first = open_new(&documents, &manifest());
    let before = arrive(&documents, &first, None);
    paste_into(&documents, &first);
    let pasted = arrive(&documents, &first, Some(before));
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&first),
    );
    arrive(&documents, &first, Some(pasted));
    // Opened again by another path to the same file, as Ctrl+O or a drop may name it.
    let again = folder
        .0
        .join("..")
        .join(folder.0.file_name().unwrap_or_default())
        .join(FIRST);
    let second = open_new(&documents, &again);
    let shown = arrive(&documents, &second, None);
    documents.forget(&first);
    assert!(folder.has(&documents, FIRST), "the second window shows it");
    give(
        &documents,
        DocumentAt::new(&manifest()),
        Target::Window(&second),
    );
    arrive(&documents, &second, Some(shown));
    assert!(
        folder.has(&documents, FIRST),
        "the second window holds it in history"
    );
    documents.forget(&second);
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_paste_given_to_a_window_as_its_last_window_closes_is_never_deleted_from_under_it() {
    // From the review of ticket 02: the deleting thread checked under the table's lock that no
    // window held the file, and deleted it after the lock, so a window given the file in between,
    // by Open, a link or a drop, lost it. The other window is given the file just before it is
    // deleted; that ends then if nothing holds it up.
    let folder = TestFolder::new("given-as-deleted");
    let (documents, _) = folder.documents();
    let first = open_empty(&documents);
    paste_into(&documents, &first);
    arrive(&documents, &first, None);
    let second = open_new(&documents, &manifest());
    arrive(&documents, &second, None);
    let (given, taken) = std::sync::mpsc::channel();
    let (other, window, file) = (shared(&documents), second.clone(), folder.file(FIRST));
    documents.before_next_delete(move || {
        meanwhile(other, move |documents| {
            let _ = documents.give(DocumentAt::new(&file), Target::Window(&window));
            // The test may have stopped waiting already.
            let _ = given.send(file.exists());
        });
    });
    documents.forget(&first);
    let taken_while_there = taken.recv_timeout(HELD_UP_END_WAIT) == Ok(true);
    assert_eq!(documents.document_path(&second), Some(folder.file(FIRST)));
    assert!(
        !taken_while_there || folder.has(&documents, FIRST),
        "the window was given the file while it was there, and it was deleted from under it"
    );
}

#[test]
fn a_paste_already_gone_is_no_error() {
    let folder = TestFolder::new("already-gone");
    let (documents, _) = folder.documents();
    let first = open_empty(&documents);
    paste_into(&documents, &first);
    arrive(&documents, &first, None);
    std::fs::remove_file(folder.file(FIRST)).ok();
    documents.forget(&first);
    assert!(!folder.has(&documents, FIRST));
    // Pastes are still deleted after it.
    let second = open_empty(&documents);
    paste_into(&documents, &second);
    arrive(&documents, &second, None);
    assert!(folder.has(&documents, FIRST));
    documents.forget(&second);
    assert!(!folder.has(&documents, FIRST));
}

#[test]
fn a_document_outside_the_pasted_folder_is_never_deleted() {
    let folder = TestFolder::new("outside");
    let (documents, _) = folder.documents();
    let outside = document("outside-the-pasted-folder", "# Kept");
    let window = open_new(&documents, &outside);
    arrive(&documents, &window, None);
    documents.forget(&window);
    documents.finish_deleting_pastes();
    assert!(outside.exists());
    std::fs::remove_file(outside).ok();
}

/// Writes `name` in `folder` as a paste of an earlier run would have left it: last modified well
/// before this process started.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn leave_from_an_earlier_run(folder: &TestFolder, name: &str) {
    std::fs::create_dir_all(&folder.0).unwrap();
    let file = folder.file(name);
    std::fs::write(&file, "left by a crash").unwrap();
    let a_year_ago = SystemTime::now() - Duration::from_secs(365 * 24 * 60 * 60);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|file| file.set_modified(a_year_ago))
        .unwrap();
}

#[test]
fn the_startup_sweep_empties_the_pasted_folder_but_for_a_file_a_window_shows() {
    let folder = TestFolder::new("sweep");
    for name in [FIRST, SECOND, "Notes.txt"] {
        leave_from_an_earlier_run(&folder, name);
    }
    let (documents, _) = folder.documents();
    // Opened from the file manager as Downlight starts.
    let window = open_new(&documents, &folder.file(SECOND));
    documents.sweep_pastes();
    assert!(!folder.has(&documents, FIRST));
    assert_eq!(folder.names(), [SECOND]);
    documents.forget(&window);
    assert!(!folder.has(&documents, SECOND));
}

#[test]
fn the_startup_sweep_keeps_a_file_written_since_the_process_started() {
    let folder = TestFolder::new("sweep-since-start");
    leave_from_an_earlier_run(&folder, FIRST);
    // As another Downlight process pasting while this one starts would write it.
    std::fs::write(folder.file(SECOND), "pasted just now").ok();
    let (documents, _) = folder.documents();
    documents.sweep_pastes();
    assert!(!folder.has(&documents, FIRST));
    assert_eq!(folder.names(), [SECOND]);
}

/// The file the first text dropped at [`AT`] is written to.
const DROPPED: &str = "Dropped 2026-11-20 14.32.05.md";

/// Drops `text` onto `window` at [`AT`], as the drop target hands it on.
fn drop_text(documents: &Documents, window: &str, text: &str) {
    show_dropped_text(documents, window, text, AT);
}

#[test]
fn text_dropped_onto_the_empty_window_shows_a_new_dropped_file_holding_the_text() {
    let folder = TestFolder::new("dropped-empty-window");
    let (documents, recorder) = folder.documents();
    let window = open_empty(&documents);
    let text = "# Dragged from a browser\n\nGrüße, 灯\n";
    drop_text(&documents, &window, text);
    let file = folder.file(DROPPED);
    assert_eq!(documents.document_path(&window), Some(file.clone()));
    // UTF-8 without a byte order mark.
    assert_eq!(std::fs::read(&file).ok(), Some(text.as_bytes().to_vec()));
    assert_eq!(recorder.sent(), [to(&window, Message::AskForDocument)]);
    assert!(rendered_html(&documents, &window).contains("Dragged from a browser"));
}

#[test]
fn two_drops_in_the_same_second_get_two_files() {
    let folder = TestFolder::new("dropped-same-second");
    let (documents, _) = folder.documents();
    let window = open_empty(&documents);
    drop_text(&documents, &window, "first");
    arrive(&documents, &window, None);
    drop_text(&documents, &window, "second");
    assert_eq!(
        folder.names(),
        ["Dropped 2026-11-20 14.32.05 (2).md", DROPPED]
    );
    assert_eq!(
        documents.document_path(&window),
        Some(folder.file("Dropped 2026-11-20 14.32.05 (2).md"))
    );
}

#[test]
fn dropped_text_replaces_the_shown_document_and_back_returns_to_it() {
    let folder = TestFolder::new("dropped-history");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    drop_text(&documents, &window, "# Dropped");
    assert_eq!(documents.document_path(&window), Some(folder.file(DROPPED)));
    let dropped = arrive(&documents, &window, Some(before));
    documents.step(&window, Step::Back).ok();
    assert_eq!(documents.document_path(&window), Some(manifest()));
    let returned = handover(&documents, &window, Some(dropped));
    assert_eq!(returned.place, Place::Return);
    assert_eq!(returned.showing, before);
}

#[test]
fn dropped_text_replaces_a_document_still_on_its_way() {
    let folder = TestFolder::new("dropped-on-its-way");
    let (documents, _) = folder.documents();
    let window = open_new(&documents, &manifest());
    let before = arrive(&documents, &window, None);
    give(
        &documents,
        DocumentAt::new(&app_file("build.rs")),
        Target::Window(&window),
    );
    let on_its_way = handover(&documents, &window, Some(before));
    drop_text(&documents, &window, "# Dropped");
    assert!(on_its_way.render.is_left());
    assert_eq!(documents.document_path(&window), Some(folder.file(DROPPED)));
    arrive(&documents, &window, Some(before));
    // The document on its way never arrived, so Back goes to the one in view before it.
    documents.step(&window, Step::Back).ok();
    assert_eq!(documents.document_path(&window), Some(manifest()));
}

#[test]
fn dropped_whitespace_leaves_the_window_and_the_folder_as_they_were_and_says_nothing() {
    for (name, text) in [("empty", ""), ("whitespace", " \r\n\t\n  ")] {
        let folder = TestFolder::new(&format!("dropped-nothing-{name}"));
        let (documents, recorder) = folder.documents();
        let window = open_new(&documents, &manifest());
        drop_text(&documents, &window, text);
        assert_eq!(documents.document_path(&window), Some(manifest()), "{name}");
        assert_eq!(recorder.sent(), [], "{name}");
        assert!(!folder.0.exists(), "{name}");
    }
}

#[test]
fn dropped_text_that_cannot_be_written_tells_the_window_naming_the_file() {
    // A file where the pasted folder should be, so the folder can't be made.
    let folder = TestFolder::new("dropped-unwritable");
    std::fs::write(&folder.0, "in the way").ok();
    let (documents, recorder) = folder.documents();
    let window = open_new(&documents, &manifest());
    drop_text(&documents, &window, "# Dropped");
    std::fs::remove_file(&folder.0).ok();
    let sent = recorder.sent();
    let [(to_window, Message::Notice(Notice::NamingFile { before, file, .. }))] = sent.as_slice()
    else {
        panic!("expected one notice naming the file, got {sent:?}");
    };
    assert_eq!(to_window, &window);
    assert_eq!(before, "Can't write ");
    assert_eq!(file, &crate::notice::NamedFile::at(&folder.0));
    assert_eq!(documents.document_path(&window), Some(manifest()));
}

#[test]
fn dropped_text_is_deleted_when_its_window_closes() {
    let folder = TestFolder::new("dropped-window-closes");
    let (documents, _) = folder.documents();
    let window = open_empty(&documents);
    drop_text(&documents, &window, "# Dropped");
    arrive(&documents, &window, None);
    assert!(folder.has(&documents, DROPPED));
    documents.forget(&window);
    assert!(!folder.has(&documents, DROPPED));
}
