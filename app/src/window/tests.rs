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
    match windows.claim(document.map(DocumentAt::new), Target::NewWindow) {
        Claim::New { window, .. } => window,
        other => panic!("expected a new window, got {other:?}"),
    }
}

#[test]
fn first_claim_of_a_document_names_a_new_window_that_shows_it() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    assert_eq!(windows.document_path(&label), Some(manifest()));
}

#[test]
fn new_window_comes_with_the_key_its_watch_is_kept_under() {
    let windows = OpenWindows::default();
    assert_eq!(
        windows.claim(
            Some(DocumentAt::new(&app_file("cargo.toml"))),
            Target::NewWindow
        ),
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
        windows.claim(Some(DocumentAt::new(&manifest())), Target::NewWindow),
        Claim::AlreadyOpen {
            window: first,
            scroll_to: None
        }
    );
}

#[test]
fn open_document_is_found_ignoring_case() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    assert_eq!(
        windows.claim(
            Some(DocumentAt::new(&app_file("CARGO.TOML"))),
            Target::NewWindow
        ),
        Claim::AlreadyOpen {
            window: first,
            scroll_to: None
        }
    );
}

#[test]
fn open_document_is_found_through_a_parent_folder_step() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let roundabout = app_file("src").join("..").join("Cargo.toml");
    assert_eq!(
        windows.claim(Some(DocumentAt::new(&roundabout)), Target::NewWindow),
        Claim::AlreadyOpen {
            window: first,
            scroll_to: None
        }
    );
}

#[test]
fn missing_document_claimed_again_in_other_case_gives_its_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&app_file("missing.md")));
    assert_eq!(
        windows.claim(
            Some(DocumentAt::new(&app_file("MISSING.md"))),
            Target::NewWindow
        ),
        Claim::AlreadyOpen {
            window: first,
            scroll_to: None
        }
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
        windows.claim(Some(DocumentAt::new(&manifest())), Target::NewWindow),
        Claim::New { .. }
    ));
    assert_eq!(windows.document_path(&first), None);
}

#[test]
fn empty_windows_are_never_the_same_window() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, None);
    let second = open_new(&windows, None);
    assert_ne!(first, second);
    assert_eq!(windows.document_path(&first), None);
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
        windows.claim(Some(DocumentAt::new(&manifest())), Target::Window(&label)),
        Claim::Changed {
            window: label.clone(),
            key: Some(document_key(&manifest()))
        }
    );
    assert_eq!(windows.document_path(&label), Some(manifest()));
}

#[test]
fn window_shows_another_document_in_place_of_its_own() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let other = app_file("build.rs");
    assert_eq!(
        windows.claim(Some(DocumentAt::new(&other)), Target::Window(&label)),
        Claim::Changed {
            window: label.clone(),
            key: Some(document_key(&other))
        }
    );
    assert_eq!(windows.document_path(&label), Some(other));
}

#[test]
fn document_a_window_gave_up_opens_anew() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    windows.claim(
        Some(DocumentAt::new(&app_file("build.rs"))),
        Target::Window(&label),
    );
    assert!(matches!(
        windows.claim(Some(DocumentAt::new(&manifest())), Target::NewWindow),
        Claim::New { .. }
    ));
}

#[test]
fn document_shown_in_a_window_is_found_there_again() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    windows.claim(Some(DocumentAt::new(&manifest())), Target::Window(&label));
    assert_eq!(
        windows.claim(
            Some(DocumentAt::new(&app_file("CARGO.TOML"))),
            Target::NewWindow
        ),
        Claim::AlreadyOpen {
            window: label,
            scroll_to: None
        }
    );
}

#[test]
fn document_open_in_another_window_stays_there() {
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let second = open_new(&windows, Some(&app_file("build.rs")));
    assert_eq!(
        windows.claim(Some(DocumentAt::new(&manifest())), Target::Window(&second)),
        Claim::AlreadyOpen {
            window: first,
            scroll_to: None
        }
    );
    assert_eq!(windows.document_path(&second), Some(app_file("build.rs")));
}

#[test]
fn window_document_path_is_the_one_it_shows() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&app_file("cargo.toml")));
    // As given, not as the key spells it, so the reader sees the name they know.
    assert_eq!(windows.document_path(&label), Some(app_file("cargo.toml")));
    windows.claim(
        Some(DocumentAt::new(&app_file("build.rs"))),
        Target::Window(&label),
    );
    assert_eq!(windows.document_path(&label), Some(app_file("build.rs")));
}

#[test]
fn window_without_a_document_has_no_document_path() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    assert_eq!(windows.document_path(&label), None);
    assert_eq!(windows.document_path("document-99"), None);
}

#[test]
fn document_claimed_for_its_own_window_is_already_there() {
    // A link to the shown document scrolls instead of rendering it again. The link was clicked in
    // the document, so the page has asked for it.
    let windows = OpenWindows::default();
    let label = open_new(&windows, Some(&manifest()));
    let _ = windows.render_of(&label);
    assert_eq!(
        windows.claim(
            Some(DocumentAt {
                path: &app_file("CARGO.TOML"),
                heading: Some("install")
            }),
            Target::Window(&label)
        ),
        Claim::AlreadyOpen {
            window: label,
            scroll_to: Some("install".to_owned())
        }
    );
}

#[test]
fn link_to_a_document_open_in_another_window_scrolls_it_only_with_a_heading() {
    // Without a heading, that window keeps its reading position (ticket 11's decisions).
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let _ = windows.render_of(&first);
    let second = open_new(&windows, Some(&app_file("build.rs")));
    let at_install = DocumentAt {
        path: &manifest(),
        heading: Some("install"),
    };
    assert_eq!(
        windows.claim(Some(at_install), Target::Window(&second)),
        Claim::AlreadyOpen {
            window: first.clone(),
            scroll_to: Some("install".to_owned())
        }
    );
    assert_eq!(
        windows.claim(Some(DocumentAt::new(&manifest())), Target::Window(&second)),
        Claim::AlreadyOpen {
            window: first,
            scroll_to: None
        }
    );
}

#[test]
fn heading_for_an_open_window_whose_page_has_not_asked_is_held_for_its_first_render() {
    // Ctrl+click on `b.md#x` opens a window whose page listens for nothing until its webview has
    // started, so a heading sent to it now would be lost.
    let windows = OpenWindows::default();
    let first = open_new(&windows, Some(&manifest()));
    let second = open_new(&windows, Some(&app_file("build.rs")));
    let at_install = DocumentAt {
        path: &manifest(),
        heading: Some("install"),
    };
    assert_eq!(
        windows.claim(Some(at_install), Target::Window(&second)),
        Claim::AlreadyOpen {
            window: first.clone(),
            scroll_to: None
        }
    );
    assert_eq!(
        heading_of(&windows, &first),
        Some(Some("install".to_owned()))
    );
}

#[test]
fn showing_in_a_closed_window_changes_nothing() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    windows.forget(&label);
    assert_eq!(
        windows.claim(Some(DocumentAt::new(&manifest())), Target::Window(&label)),
        Claim::WindowGone
    );
    assert!(matches!(
        windows.claim(Some(DocumentAt::new(&manifest())), Target::NewWindow),
        Claim::New { .. }
    ));
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
    windows.claim(Some(DocumentAt::new(&second)), Target::Window(&label));
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
    windows.claim(Some(DocumentAt::new(&manifest())), Target::NewWindow);
    // Its window shows it already, so a render ahead would be read and then never used.
    let shown = windows.lock().shown[&label]
        .as_ref()
        .map(|shown| shown.render_ahead.is_some());
    assert_eq!(shown, Some(false));
}

/// The heading `window`'s page gets with its document, or None while it shows none.
fn heading_of(windows: &OpenWindows, window: &str) -> Option<Option<String>> {
    windows.render_of(window).map(|handover| handover.heading)
}

#[test]
fn new_window_for_a_heading_link_keeps_the_heading() {
    let windows = OpenWindows::default();
    let claim = windows.claim(
        Some(DocumentAt {
            path: &manifest(),
            heading: Some("install"),
        }),
        Target::NewWindow,
    );
    assert!(matches!(claim, Claim::New { .. }), "{claim:?}");
    assert_eq!(
        heading_of(&windows, "document-0"),
        Some(Some("install".to_owned()))
    );
}

#[test]
fn window_shown_a_document_at_a_heading_hands_it_over_with_every_render() {
    // Two links followed in quick succession each tell the page to ask, and the page keeps only
    // its latest request, which must still get the heading.
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    windows.claim(
        Some(DocumentAt {
            path: &manifest(),
            heading: Some("install"),
        }),
        Target::Window(&label),
    );
    let first = heading_of(&windows, &label);
    let second = heading_of(&windows, &label);
    assert_eq!(first, Some(Some("install".to_owned())));
    assert_eq!(second, first);
}

#[test]
fn window_shown_a_document_without_a_heading_hands_over_none() {
    let windows = OpenWindows::default();
    let label = open_new(&windows, None);
    windows.claim(
        Some(DocumentAt {
            path: &manifest(),
            heading: Some("install"),
        }),
        Target::Window(&label),
    );
    windows.claim(
        Some(DocumentAt::new(&app_file("build.rs"))),
        Target::Window(&label),
    );
    assert_eq!(heading_of(&windows, &label), Some(None));
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
    windows.claim(Some(DocumentAt::new(&second)), Target::Window(&label));
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
    windows.claim(
        Some(DocumentAt::new(&app_file("build.rs"))),
        Target::Window(&label),
    );
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
    windows.claim(
        Some(DocumentAt::new(&app_file("build.rs"))),
        Target::Window(&label),
    );
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
    windows.claim(
        Some(DocumentAt::new(&app_file("build.rs"))),
        Target::Window(&label),
    );
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
