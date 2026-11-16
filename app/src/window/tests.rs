use crate::documents::recorder::{ToWindow, recorded};

use super::*;

/// A document of a launch, which needn't exist: a missing one opens a window that says so.
const LAUNCHED: &str = r"C:\work\docs\README.md";

#[test]
fn launch_that_names_no_document_opens_a_window_titled_downlight() {
    let (documents, recorder) = recorded();
    let failures = open_launch(&documents, ["downlight"], Path::new(r"C:\work"));
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        recorder.done(),
        [(
            "document-0".to_owned(),
            ToWindow::Opened(NewWindow {
                title: "Downlight".to_owned()
            })
        )]
    );
}

#[test]
fn launch_opens_a_window_for_each_document_titled_after_it() {
    let (documents, recorder) = recorded();
    let failures = open_launch(
        &documents,
        ["downlight", LAUNCHED, r"C:\work\b.md"],
        Path::new(r"C:\work"),
    );
    let titles = recorder
        .done()
        .into_iter()
        .map(|(_, done)| done)
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        titles,
        [
            ToWindow::Opened(NewWindow {
                title: "README.md - docs - Downlight".to_owned()
            }),
            ToWindow::Opened(NewWindow {
                title: "b.md - work - Downlight".to_owned()
            })
        ]
    );
}

#[test]
fn failure_names_its_document_and_why_in_the_reader_s_terms() {
    let (documents, recorder) = recorded();
    recorder.fail_window("document-0");
    let failures = open_launch(&documents, ["downlight", LAUNCHED], Path::new(r"C:\work"));
    assert_eq!(
        failure_report(&failures).as_deref(),
        Some(
            r"Can't show C:\work\docs\README.md: its window didn't open: the window can't be changed"
        )
    );
}

#[test]
fn failure_of_the_window_without_a_document_names_the_error() {
    let (documents, recorder) = recorded();
    recorder.fail_window("document-0");
    let failures = open_launch(&documents, ["downlight"], Path::new(r"C:\work"));
    assert_eq!(
        failure_report(&failures).as_deref(),
        Some("Can't open a window: the window can't be changed")
    );
}

#[test]
fn report_names_each_failure_on_a_line_of_its_own() {
    let (documents, recorder) = recorded();
    recorder.fail_window("document-0");
    recorder.fail_window("document-1");
    let failures = open_launch(
        &documents,
        ["downlight", "a.md", "b.md"],
        Path::new(r"C:\work"),
    );
    assert_eq!(
        failure_report(&failures).as_deref(),
        Some(
            "Can't show C:\\work\\a.md: its window didn't open: the window can't be changed\n\
             Can't show C:\\work\\b.md: its window didn't open: the window can't be changed"
        )
    );
}

#[test]
fn launch_without_failures_has_nothing_to_report() {
    assert_eq!(failure_report(&[]), None);
}

#[test]
fn page_url_tells_the_page_about_timing_backdrop_and_title_bar() {
    let plain = WindowLook {
        backdrop: false,
        title_bar: false,
    };
    let backdrop = WindowLook {
        backdrop: true,
        ..plain
    };
    let title_bar = WindowLook {
        title_bar: true,
        ..plain
    };
    let both = WindowLook {
        backdrop: true,
        title_bar: true,
    };
    assert_eq!(page_url(false, plain), "index.html");
    assert_eq!(page_url(true, plain), "index.html?timing");
    assert_eq!(page_url(false, backdrop), "index.html?backdrop");
    assert_eq!(page_url(false, title_bar), "index.html?titleBar");
    assert_eq!(page_url(true, both), "index.html?timing&backdrop&titleBar");
}

#[test]
fn only_the_page_itself_may_load_in_a_window() {
    for (page, url) in [
        ("http://tauri.localhost", "http://tauri.localhost/"),
        (
            "http://tauri.localhost",
            "http://tauri.localhost/index.html",
        ),
        (
            "http://tauri.localhost",
            "http://tauri.localhost/index.html?timing&backdrop",
        ),
        ("http://localhost:5173", "http://localhost:5173/"),
        (
            "http://localhost:5173",
            "http://localhost:5173/index.html?backdrop",
        ),
    ] {
        let url = tauri::Url::parse(url).unwrap();
        assert!(is_the_page(&url, page), "{url}");
    }
}

#[test]
fn other_urls_on_the_pages_origin_never_load_in_a_window() {
    for url in [
        "http://tauri.localhost/docs/guide.md",
        "http://tauri.localhost/C:/work/README.md",
        "http://tauri.localhost/assets/index.js",
        "http://localhost:5174/",
        "http://image.localhost/",
        "https://example.com/",
        "file:///C:/work/README.md",
    ] {
        let url = tauri::Url::parse(url).unwrap();
        assert!(!is_the_page(&url, "http://tauri.localhost"), "{url}");
    }
}
