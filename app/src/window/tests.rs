use std::path::PathBuf;

use super::*;

#[test]
fn window_without_a_document_is_titled_downlight() {
    assert_eq!(title_of(None), "Downlight");
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
