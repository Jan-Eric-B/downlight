//! The settings reaching every page (window design ticket 12): a change is sent to every page
//! that hears it, and held for a page that doesn't yet (ticket 16's review).

use super::*;
use crate::settings::change::SettingChange;
use crate::settings::{AccentColor, AccentScale, Backdrop, PageSettings, Theme, WindowLook};
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

const DARK_ACRYLIC: WindowLook = WindowLook {
    theme: Theme::Dark,
    backdrop: Backdrop::Acrylic,
};

/// Changes the settings to `look`.
fn change_look(documents: &Documents, look: WindowLook) {
    let _ = documents.change_settings(|settings| {
        settings.theme = look.theme;
        settings.backdrop = look.backdrop;
    });
}

/// Every look set on a window so far, in turn.
fn looks_set(recorder: &Recorder) -> Vec<(String, WindowLook)> {
    recorder
        .done()
        .into_iter()
        .filter_map(|(window, done)| match done {
            ToWindow::Looked(look) => Some((window, look)),
            _ => None,
        })
        .collect()
}

#[test]
fn new_window_opens_with_the_reader_s_theme_and_backdrop() {
    let (documents, recorder) = recorded_with(Settings {
        theme: Theme::Light,
        backdrop: Backdrop::None,
        ..Settings::default()
    });
    let window = open_empty(&documents);
    assert_eq!(
        recorder.opened_as(&window).map(|opened| opened.look),
        Some(WindowLook {
            theme: Theme::Light,
            backdrop: Backdrop::None,
        })
    );
}

#[test]
fn change_of_look_reaches_every_window() {
    let (documents, recorder) = documents();
    let first = open_empty(&documents);
    let second = open_empty(&documents);
    documents.page_asks(&first);
    documents.page_asks(&second);
    change_look(&documents, DARK_ACRYLIC);
    assert_eq!(
        looks_set(&recorder),
        [to(&first, DARK_ACRYLIC), to(&second, DARK_ACRYLIC)]
    );
    // And the pages, whose settings flyouts show the new look.
    let shown = recorder.sent();
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().all(|(_, message)| matches!(
        message,
        Message::Settings(settings) if settings.theme == Theme::Dark
            && settings.backdrop == Backdrop::Acrylic
    )));
}

#[test]
fn change_that_leaves_the_look_as_it_was_sets_no_window_s_look() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    let _ = documents.change_settings(|settings| settings.full_width = true);
    let _ = documents.change_settings(|settings| settings.backdrop = Backdrop::Mica);
    assert_eq!(looks_set(&recorder), []);
}

// A window still being built when the look changes isn't there for the adapter to change yet, so
// it gets the look the settings hold when its page first asks.
#[test]
fn window_whose_page_missed_a_change_of_look_gets_it_when_its_page_asks() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    change_look(&documents, DARK_ACRYLIC);
    let before_asking = looks_set(&recorder).len();
    documents.page_asks(&window);
    assert_eq!(
        looks_set(&recorder)[before_asking..],
        [to(&window, DARK_ACRYLIC)]
    );
    // Once only.
    documents.page_asks(&window);
    assert_eq!(looks_set(&recorder).len(), before_asking + 1);
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

/// Windows' accent colors change, and the scale reads as `scale` now (None for a read that fails).
fn windows_accent_reads(documents: &Documents, recorder: &Recorder, scale: Option<AccentScale>) {
    recorder.set_accent_scale(scale);
    documents.change_windows_accent();
}

#[test]
fn windows_accent_change_reaches_every_page_and_new_windows_open_with_it() {
    let (documents, recorder) = documents();
    let first = open_empty(&documents);
    let second = open_empty(&documents);
    documents.page_asks(&first);
    documents.page_asks(&second);
    let scale = Some(AccentScale::sample());
    windows_accent_reads(&documents, &recorder, scale);
    let told = Message::Settings(PageSettings {
        windows_accent: scale,
        ..PageSettings::default()
    });
    assert_eq!(
        recorder.sent(),
        [to(&first, told.clone()), to(&second, told)]
    );
    let new_window = open_empty(&documents);
    assert_eq!(
        recorder
            .opened_as(&new_window)
            .and_then(|opened| opened.settings.windows_accent),
        scale
    );
}

// Windows tells of a change of its theme too, and of one change more than once.
#[test]
fn windows_accent_told_unchanged_tells_no_page() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    windows_accent_reads(&documents, &recorder, Some(AccentScale::sample()));
    windows_accent_reads(&documents, &recorder, Some(AccentScale::sample()));
    assert_eq!(recorder.sent().len(), 1);
}

// A read that fails says nothing of the accent color: None from a read never means the OS has
// none, which the pages' settings flyouts decide from once (window design ticket 09's review).
#[test]
fn failed_windows_accent_read_keeps_the_scale_the_pages_have() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    windows_accent_reads(&documents, &recorder, Some(AccentScale::sample()));
    let before_failing = recorder.sent().len();
    windows_accent_reads(&documents, &recorder, None);
    assert_eq!(recorder.sent()[before_failing..], []);
    let new_window = open_empty(&documents);
    assert_eq!(
        recorder
            .opened_as(&new_window)
            .and_then(|opened| opened.settings.windows_accent),
        Some(AccentScale::sample())
    );
}

#[test]
fn choosing_windows_accent_color_sends_the_scale_windows_has_now() {
    let (documents, recorder) = documents();
    let window = open_empty(&documents);
    documents.page_asks(&window);
    windows_accent_reads(&documents, &recorder, Some(AccentScale::sample()));
    let before_choosing = recorder.sent().len();
    let _ = documents.change_settings(|settings| {
        SettingChange::AccentColor {
            accent_color: AccentColor::Windows,
        }
        .apply(settings);
    });
    assert_eq!(
        recorder.sent()[before_choosing..],
        [to(
            &window,
            Message::Settings(PageSettings {
                accent_color: AccentColor::Windows,
                windows_accent: Some(AccentScale::sample()),
                ..PageSettings::default()
            })
        )]
    );
}

#[test]
fn windows_accent_change_before_a_page_asks_reaches_it_when_it_asks() {
    let (documents, recorder) = documents();
    let window = open_new(&documents, &manifest());
    windows_accent_reads(&documents, &recorder, Some(AccentScale::sample()));
    assert_eq!(recorder.sent(), []);
    documents.page_asks(&window);
    assert!(matches!(
        recorder.sent().as_slice(),
        [(_, Message::Settings(settings))] if settings.windows_accent == Some(AccentScale::sample())
    ));
}
