use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};

use crate::watch::Change;

use tauri::async_runtime::block_on;

use super::*;

/// A file that exists: the app crate's own manifest.
fn manifest() -> PathBuf {
    app_file("Cargo.toml")
}

fn app_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// Opens a new window for `document` (None: an empty window) and returns its label.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn open_new(windows: &OpenWindows, document: Option<&Path>) -> String {
    match windows.claim(document, Target::NewWindow) {
        Claim::New { window, .. } => window,
        other => panic!("expected a new window, got {other:?}"),
    }
}

#[test]
fn first_claim_of_a_document_names_a_new_window_that_shows_it() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    assert_eq!(document_of(&windows, &label), Some(manifest()));
}

#[test]
fn new_window_comes_with_the_key_its_watch_is_kept_under() {
    let windows = OpenWindows::default();
    assert_eq!(
        windows.claim(Some(&app_file("cargo.toml")), Target::NewWindow),
        Claim::New {
            window: "document-0".to_owned(),
            key: Some(document_key(&manifest()))
        }
    );
}

#[test]
fn new_window_without_a_document_comes_without_a_key() {
    let windows = OpenWindows::default();
    assert_eq!(
        windows.claim(None, Target::NewWindow),
        Claim::New {
            window: "document-0".to_owned(),
            key: None
        }
    );
}

#[test]
fn open_document_claimed_again_gives_its_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    assert_eq!(
        windows.claim(Some(&manifest()), Target::NewWindow),
        Claim::AlreadyOpen(first)
    );
}

#[test]
fn open_document_is_found_ignoring_case() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    assert_eq!(
        windows.claim(Some(&app_file("CARGO.TOML")), Target::NewWindow),
        Claim::AlreadyOpen(first)
    );
}

#[test]
fn open_document_is_found_through_a_parent_folder_step() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let roundabout = app_file("src").join("..").join("Cargo.toml");
    assert_eq!(
        windows.claim(Some(&roundabout), Target::NewWindow),
        Claim::AlreadyOpen(first)
    );
}

#[test]
fn missing_document_claimed_again_in_other_case_gives_its_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&app_file("missing.md")));
    assert_eq!(
        windows.claim(Some(&app_file("MISSING.md")), Target::NewWindow),
        Claim::AlreadyOpen(first)
    );
}

#[test]
fn other_document_opens_another_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let other = open_new(&windows, Some(&app_file("build.rs")));
    assert_ne!(first, other);
}

#[test]
fn forgotten_window_gives_its_document_a_new_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    windows.forget(&first);
    assert!(matches!(
        windows.claim(Some(&manifest()), Target::NewWindow),
        Claim::New { .. }
    ));
    assert_eq!(document_of(&windows, &first), None);
}

#[test]
fn empty_windows_are_never_the_same_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, None);
    let second = open_new(&windows, None);
    assert_ne!(first, second);
    assert_eq!(document_of(&windows, &first), None);
}

#[test]
fn window_without_a_document_is_titled_downlight() {
    assert_eq!(title_of(None), "Downlight");
}

#[test]
fn empty_window_shows_the_document_it_is_given() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    assert_eq!(
        windows.claim(Some(&manifest()), Target::Window(&label)),
        Claim::Changed {
            window: label.clone(),
            key: Some(document_key(&manifest()))
        }
    );
    assert_eq!(document_of(&windows, &label), Some(manifest()));
}

#[test]
fn window_shows_another_document_in_place_of_its_own() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let other = app_file("build.rs");
    assert_eq!(
        windows.claim(Some(&other), Target::Window(&label)),
        Claim::Changed {
            window: label.clone(),
            key: Some(document_key(&other))
        }
    );
    assert_eq!(document_of(&windows, &label), Some(other));
}

#[test]
fn document_a_window_gave_up_opens_anew() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    windows.claim(Some(&app_file("build.rs")), Target::Window(&label));
    assert!(matches!(
        windows.claim(Some(&manifest()), Target::NewWindow),
        Claim::New { .. }
    ));
}

#[test]
fn document_shown_in_a_window_is_found_there_again() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    windows.claim(Some(&manifest()), Target::Window(&label));
    assert_eq!(
        windows.claim(Some(&app_file("CARGO.TOML")), Target::NewWindow),
        Claim::AlreadyOpen(label)
    );
}

#[test]
fn document_open_in_another_window_stays_there() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let second = open_new(&windows, Some(&app_file("build.rs")));
    assert_eq!(
        windows.claim(Some(&manifest()), Target::Window(&second)),
        Claim::AlreadyOpen(first)
    );
    assert_eq!(document_of(&windows, &second), Some(app_file("build.rs")));
}

#[test]
fn showing_in_a_closed_window_changes_nothing() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    windows.forget(&label);
    assert_eq!(
        windows.claim(Some(&manifest()), Target::Window(&label)),
        Claim::WindowGone
    );
    assert!(matches!(
        windows.claim(Some(&manifest()), Target::NewWindow),
        Claim::New { .. }
    ));
}

/// The document `window` shows, as the page's next render reads it.
fn document_of(windows: &OpenWindows, window: &str) -> Option<PathBuf> {
    windows
        .lock()
        .shown
        .get(window)?
        .as_ref()
        .map(|shown| shown.path.clone())
}

/// A document file of the test's own, so that tests running in parallel never share one.
#[expect(
    clippy::unwrap_used,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn document(test: &str, text: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("downlight-window-{}-{test}.md", std::process::id()));
    std::fs::write(&path, text).unwrap();
    path
}

/// The HTML `window`'s page gets when it asks for its document.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn rendered_html(windows: &OpenWindows, window: &str) -> String {
    let Some(handover) = windows.render_of(window) else {
        panic!("expected {window} to show a document");
    };
    match block_on(handover.render) {
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
fn wait_for_render_ahead(windows: &OpenWindows, window: &str) {
    for _ in 0..500 {
        let finished = windows.lock().shown[window]
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

#[test]
fn window_given_a_new_document_renders_it_ahead() {
    let path = document("new", "# Read ahead");
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&path));
    wait_for_render_ahead(&windows, &label);
    std::fs::write(&path, "# Changed since").unwrap();
    let shown = rendered_html(&windows, &label);
    std::fs::remove_file(&path).unwrap();
    assert!(shown.contains("Read ahead"), "{shown}");
}

#[test]
fn window_changed_to_another_document_renders_the_new_one_ahead() {
    let first = document("changed-first", "# First document");
    let second = document("changed-second", "# Second document");
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&first));
    windows.claim(Some(&second), Target::Window(&label));
    wait_for_render_ahead(&windows, &label);
    std::fs::write(&second, "# Changed since").unwrap();
    let shown = rendered_html(&windows, &label);
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_file(&second).unwrap();
    assert!(shown.contains("Second document"), "{shown}");
}

#[test]
fn next_render_after_the_first_reads_the_file_again() {
    let path = document("again", "# First");
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&path));
    rendered_html(&windows, &label);
    std::fs::write(&path, "# Second").unwrap();
    let shown = rendered_html(&windows, &label);
    std::fs::remove_file(&path).unwrap();
    assert!(shown.contains("Second"), "{shown}");
}

#[test]
fn opening_an_open_document_again_starts_no_render_for_its_window() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let _ = windows.render_of(&label);
    windows.claim(Some(&manifest()), Target::NewWindow);
    // Its window shows it already, so a render ahead would be read and then never used.
    let shown = windows.lock().shown[&label]
        .as_ref()
        .map(|shown| shown.render_ahead.is_some());
    assert_eq!(shown, Some(false));
}

#[test]
fn window_without_a_document_has_nothing_to_render() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    assert!(windows.render_of(&label).is_none());
}

#[test]
fn reload_ahead_reads_the_file_again() {
    let path = document("reload", "# First");
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&path));
    rendered_html(&windows, &label);
    std::fs::write(&path, "# Second").unwrap();
    assert!(windows.reload_ahead(&label));
    wait_for_render_ahead(&windows, &label);
    // Written after the reload's read, so only a read on the page's request would see it.
    std::fs::write(&path, "# Third").unwrap();
    let shown = rendered_html(&windows, &label);
    std::fs::remove_file(&path).unwrap();
    assert!(shown.contains("Second"), "{shown}");
}

#[test]
fn window_without_a_document_has_nothing_to_reload() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    assert!(!windows.reload_ahead(&label));
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
fn watch_for_the_shown_document_is_kept() {
    let path = document("kept", "# Watched");
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&path));
    let (watch, life) = watch_with_its_life(&path);
    windows.keep_watch(&label, &document_key(&path), watch);
    let running = life.recv_timeout(WATCH_END_WAIT);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(running, Err(RecvTimeoutError::Timeout));
}

#[test]
fn watch_for_a_document_the_window_has_left_is_dropped() {
    let first = document("left-first", "# First");
    let second = document("left-second", "# Second");
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&first));
    windows.claim(Some(&second), Target::Window(&label));
    let (watch, life) = watch_with_its_life(&first);
    windows.keep_watch(&label, &document_key(&first), watch);
    let running = life.recv_timeout(WATCH_END_WAIT);
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_file(&second).unwrap();
    assert_eq!(running, Err(RecvTimeoutError::Disconnected));
}

#[test]
fn change_to_the_shown_document_is_acted_on() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let key = document_key(&manifest());
    assert!(windows.shows(&label, &key));
}

#[test]
fn change_to_a_document_the_window_has_left_is_ignored() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    windows.claim(Some(&app_file("build.rs")), Target::Window(&label));
    let key = document_key(&manifest());
    assert!(!windows.shows(&label, &key));
}

#[test]
fn change_for_a_closed_window_is_ignored() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    windows.forget(&label);
    let key = document_key(&manifest());
    assert!(!windows.shows(&label, &key));
}

#[test]
fn notice_held_before_the_page_asks_comes_with_its_first_render() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let key = document_key(&manifest());
    assert_eq!(
        windows.hold_notice(&label, &key, "Can't watch".to_owned()),
        None
    );
    let first = windows.render_of(&label).map(|handover| handover.notice);
    let second = windows.render_of(&label).map(|handover| handover.notice);
    assert_eq!(first, Some(Some("Can't watch".to_owned())));
    assert_eq!(second, Some(None));
}

#[test]
fn notice_after_the_page_asked_is_to_be_shown_now() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let _ = windows.render_of(&label);
    let key = document_key(&manifest());
    assert_eq!(
        windows.hold_notice(&label, &key, "Can't watch".to_owned()),
        Some("Can't watch".to_owned())
    );
}

#[test]
fn notice_for_a_document_the_window_has_left_is_dropped() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    windows.claim(Some(&app_file("build.rs")), Target::Window(&label));
    let key = document_key(&manifest());
    assert_eq!(
        windows.hold_notice(&label, &key, "Can't watch".to_owned()),
        None
    );
    let notice = windows.render_of(&label).map(|handover| handover.notice);
    assert_eq!(notice, Some(None));
}

#[test]
fn notice_for_a_document_the_window_had_before_is_not_shown_with_the_new_one() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let key = document_key(&manifest());
    windows.hold_notice(&label, &key, "Can't watch".to_owned());
    windows.claim(Some(&app_file("build.rs")), Target::Window(&label));
    let notice = windows.render_of(&label).map(|handover| handover.notice);
    assert_eq!(notice, Some(None));
}

#[test]
fn failure_names_its_document_and_the_error() {
    let failure = LaunchFailure {
        document: Some(PathBuf::from(r"C:\work\docs\README.md")),
        error: tauri::Error::WindowNotFound,
    };
    assert_eq!(
        failure.to_string(),
        r"Can't open a window for C:\work\docs\README.md: window not found"
    );
}

#[test]
fn failure_of_the_window_without_a_document_names_the_error() {
    let failure = LaunchFailure {
        document: None,
        error: tauri::Error::WindowNotFound,
    };
    assert_eq!(failure.to_string(), "Can't open a window: window not found");
}

#[test]
fn report_names_each_failure_on_a_line_of_its_own() {
    let failures = [
        LaunchFailure {
            document: Some(PathBuf::from(r"C:\work\a.md")),
            error: tauri::Error::WindowNotFound,
        },
        LaunchFailure {
            document: Some(PathBuf::from(r"C:\work\b.md")),
            error: tauri::Error::WindowNotFound,
        },
    ];
    assert_eq!(
        failure_report(&failures).as_deref(),
        Some(
            "Can't open a window for C:\\work\\a.md: window not found\n\
             Can't open a window for C:\\work\\b.md: window not found"
        )
    );
}

#[test]
fn launch_without_failures_has_nothing_to_report() {
    assert_eq!(failure_report(&[]), None);
}

#[test]
fn title_names_the_file_then_its_folder_then_downlight() {
    assert_eq!(
        title_of(Some(Path::new(r"C:\work\docs\README.md"))),
        "README.md - docs - Downlight"
    );
}

#[test]
fn title_of_a_file_at_a_drive_root_names_the_drive() {
    assert_eq!(
        title_of(Some(Path::new(r"C:\README.md"))),
        r"README.md - C:\ - Downlight"
    );
}
