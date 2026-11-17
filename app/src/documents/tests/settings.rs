//! The settings reaching every page (window design ticket 12): a change is sent to every page
//! that hears it, and held for a page that doesn't yet (ticket 16's review).

use super::*;
use crate::settings::PageSettings;
use crate::zoom::Zoom;

/// The `settings` message for the zoom `percent`.
fn zoomed_to(percent: u16) -> Message {
    Message::Settings(PageSettings {
        zoom: Zoom::from(percent),
        ..PageSettings::default()
    })
}

#[test]
fn settings_change_reaches_a_page_that_has_asked_at_once() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    let _ = documents.change_settings(|settings| settings.zoom = Zoom::from(150));
    assert_eq!(recorder.sent(), [to(&window, zoomed_to(150))]);
}

// The one broadcast test for every setting: each changes through `change_settings`
// (`SettingChange` in `app/src/settings/change.rs`).
#[test]
fn settings_change_reaches_every_page_and_new_windows_open_with_it() {
    let (documents, recorder) = documents();
    let first = open_empty(&documents);
    let second = open_empty(&documents);
    documents.page_asks(&first);
    documents.page_asks(&second);
    // Without a config folder the change isn't saved, and holds all the same.
    let changed = documents.change_settings(|settings| settings.zoom = Zoom::from(110));
    assert!(changed.is_err(), "{changed:?}");
    assert_eq!(
        recorder.sent(),
        [to(&first, zoomed_to(110)), to(&second, zoomed_to(110))]
    );
    let new_window = open_empty(&documents);
    assert_eq!(
        recorder
            .opened_as(&new_window)
            .map(|opened| opened.settings.zoom),
        Some(Zoom::from(110))
    );
}

#[test]
fn settings_change_before_a_page_asks_reaches_it_when_it_asks() {
    let (documents, recorder) = documents();
    // Its page's URL has 100%, and it doesn't hear a message yet.
    let window = open_new(&documents, &manifest());
    let _ = documents.change_settings(|settings| settings.zoom = Zoom::from(125));
    let _ = documents.change_settings(|settings| settings.zoom = Zoom::from(150));
    assert_eq!(recorder.sent(), []);
    documents.page_asks(&window);
    assert_eq!(recorder.sent(), [to(&window, zoomed_to(150))]);
    // Once only.
    documents.page_asks(&window);
    assert_eq!(recorder.sent(), [to(&window, zoomed_to(150))]);
}

#[test]
fn page_that_asks_after_no_change_is_sent_no_settings() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    assert_eq!(recorder.sent(), []);
}

#[test]
fn changing_a_setting_no_page_shows_tells_no_page() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    let _ = documents.change_settings(|settings| {
        settings.editor = Some(PathBuf::from(r"C:\Tools\edit.exe"));
    });
    assert_eq!(recorder.sent(), []);
    // Nor later, when it asks again.
    documents.page_asks(&window);
    assert_eq!(recorder.sent(), []);
}
