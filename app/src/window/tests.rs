use crate::documents::recorder::{Recorder, ToWindow, recorded};
use crate::zoom::Zoom;

use super::*;

/// A document of a launch, which needn't exist: a missing one opens a window that says so.
const LAUNCHED: &str = r"C:\work\docs\README.md";

#[test]
fn launch_that_names_no_document_opens_a_window_titled_downlight() {
    let (documents, recorder) = recorded();
    let failures = open_launch(&documents, ["downlight"], Path::new(r"C:\work"));
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(opened_titles(&recorder), ["Downlight"]);
}

/// The title of each window that opened, in turn.
fn opened_titles(recorder: &Recorder) -> Vec<String> {
    recorder
        .done()
        .into_iter()
        .filter_map(|(_, done)| match done {
            ToWindow::Opened(NewWindow { title, .. }) => Some(title),
            _ => None,
        })
        .collect()
}

#[test]
fn launch_opens_a_window_for_each_document_titled_after_it() {
    let (documents, recorder) = recorded();
    let failures = open_launch(
        &documents,
        ["downlight", LAUNCHED, r"C:\work\b.md"],
        Path::new(r"C:\work"),
    );
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        opened_titles(&recorder),
        ["README.md - docs - Downlight", "b.md - work - Downlight"]
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
    let plain = PageFlags {
        timing: false,
        backdrop_supported: false,
        title_bar: false,
    };
    let timing = PageFlags {
        timing: true,
        ..plain
    };
    let backdrop_supported = PageFlags {
        backdrop_supported: true,
        ..plain
    };
    let title_bar = PageFlags {
        title_bar: true,
        ..plain
    };
    let all = PageFlags {
        timing: true,
        backdrop_supported: true,
        title_bar: true,
    };
    let url = |flags| page_url(flags, PageSettings::default()).unwrap();
    // The settings come last, after the flags, as the URL without flags has them;
    // `page_url_gives_the_page_the_settings_it_starts_with` checks what they say.
    let plain_url = url(plain);
    let settings = plain_url
        .strip_prefix("index.html?")
        .filter(|query| query.starts_with("settings=") && !query.contains('&'))
        .unwrap_or_else(|| panic!("a URL without flags gives only the settings: {plain_url}"));
    assert_eq!(url(timing), format!("index.html?timing&{settings}"));
    assert_eq!(
        url(backdrop_supported),
        format!("index.html?backdropSupported&{settings}")
    );
    assert_eq!(url(title_bar), format!("index.html?titleBar&{settings}"));
    assert_eq!(
        url(all),
        format!("index.html?timing&backdropSupported&titleBar&{settings}")
    );
}

#[test]
fn page_url_gives_the_page_the_settings_it_starts_with() {
    let plain = PageFlags {
        timing: false,
        backdrop_supported: false,
        title_bar: false,
    };
    assert_eq!(
        page_url(
            plain,
            PageSettings {
                zoom: Zoom::from(150),
                full_width: true,
                ..PageSettings::default()
            }
        )
        .unwrap(),
        "index.html?settings=%7B%22zoom%22%3A150%2C%22fullWidth%22%3Atrue%2C%22theme%22%3A%22system%22\
         %2C%22backdrop%22%3A%22mica%22%2C%22accentColor%22%3A%22amber%22%7D"
    );
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
            "http://tauri.localhost/index.html?timing&backdropSupported",
        ),
        ("http://localhost:5173", "http://localhost:5173/"),
        (
            "http://localhost:5173",
            "http://localhost:5173/index.html?backdropSupported",
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
