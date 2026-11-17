//! Where a new window opens and at what size (ticket 16): the size of the last window that
//! closed, fitted to the screen, and a place down and to the right of the active window.

use super::*;
use crate::settings::Settings;
use crate::zoom::Zoom;

/// The new window `window` opened as, as the recorder kept it.
#[expect(
    clippy::panic,
    reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
)]
fn opened_as(recorder: &Recorder, window: &str) -> NewWindow {
    recorder
        .opened_as(window)
        .unwrap_or_else(|| panic!("{window} didn't open"))
}

fn size(width: u32, height: u32) -> WindowSize {
    WindowSize { width, height }
}

/// A screen of 1920 by 1040 logical pixels, a 1080p monitor above its taskbar.
const SCREEN: Area = Area {
    left: 0.0,
    top: 0.0,
    width: 1920.0,
    height: 1040.0,
};

/// A window at `x`, `y` on [`SCREEN`], 800 by 600.
fn place_at(x: f64, y: f64) -> WindowPlace {
    WindowPlace {
        at: Point { x, y },
        size: size(800, 600),
        work_area: SCREEN,
    }
}

/// Opens a window, puts it at `place` and makes it the one the reader is in.
fn active_window_at(documents: &Documents, recorder: &Recorder, place: WindowPlace) -> String {
    let label = open_empty(documents);
    recorder.place(&label, place);
    documents.activated(&label);
    label
}

/// Opens a window at `place`, or minimized or maximized for None, and closes it.
fn close_window_at(documents: &Documents, recorder: &Recorder, place: Option<WindowPlace>) {
    let label = open_empty(documents);
    if let Some(place) = place {
        recorder.place(&label, place);
    }
    // Without a config folder the size isn't saved, but holds until Downlight closes.
    documents.closing(&label);
    documents.forget(&label);
}

#[test]
fn new_window_opens_at_the_saved_zoom() {
    let (documents, recorder) = recorded_with(Settings {
        zoom: Zoom::from(150),
        ..Settings::default()
    });
    let label = open_new(&documents, &manifest());
    assert_eq!(opened_as(&recorder, &label).settings.zoom, Zoom::from(150));
}

#[test]
fn new_window_takes_the_size_of_the_last_window_that_closed() {
    let (documents, recorder) = documents();
    let closed = WindowPlace {
        size: size(1000, 700),
        ..place_at(100.0, 100.0)
    };
    close_window_at(&documents, &recorder, Some(closed));
    let label = open_new(&documents, &manifest());
    assert_eq!(opened_as(&recorder, &label).size, size(1000, 700));
    assert_eq!(documents.settings().window_size, Some(size(1000, 700)));
}

#[test]
fn minimized_or_maximized_window_that_closes_leaves_the_size_the_last_one_had() {
    let (documents, recorder) = documents();
    let closed = WindowPlace {
        size: size(1000, 700),
        ..place_at(100.0, 100.0)
    };
    close_window_at(&documents, &recorder, Some(closed));
    close_window_at(&documents, &recorder, None);
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(1000, 700));
}

#[test]
fn new_window_opens_at_the_default_size_until_a_window_has_closed() {
    let (documents, recorder) = documents();
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(800, 600));
}

#[test]
fn new_window_opens_down_and_right_of_the_active_window() {
    let (documents, recorder) = documents();
    active_window_at(&documents, &recorder, place_at(100.0, 200.0));
    let label = open_new(&documents, &manifest());
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point { x: 132.0, y: 232.0 })
    );
}

#[test]
fn new_window_follows_the_window_the_reader_was_in_last() {
    let (documents, recorder) = documents();
    let first = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    active_window_at(&documents, &recorder, place_at(500.0, 300.0));
    documents.activated(&first);
    let label = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point { x: 132.0, y: 132.0 })
    );
}

#[test]
fn new_window_that_would_leave_the_screen_starts_again_at_its_edge() {
    let (documents, recorder) = documents();
    // 1150 + 32 + 800 runs past 1920, and 400 + 32 + 600 still fits in 1040.
    active_window_at(&documents, &recorder, place_at(1150.0, 400.0));
    let right = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &right).at,
        Some(Point { x: 0.0, y: 432.0 })
    );
    active_window_at(&documents, &recorder, place_at(100.0, 420.0));
    let bottom = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &bottom).at,
        Some(Point { x: 132.0, y: 0.0 })
    );
}

#[test]
fn new_window_stays_on_the_screen_of_an_active_window_on_another_monitor() {
    let (documents, recorder) = documents();
    let left_monitor = Area {
        left: -1920.0,
        ..SCREEN
    };
    let on_left_monitor = |x| WindowPlace {
        work_area: left_monitor,
        ..place_at(x, 100.0)
    };
    active_window_at(&documents, &recorder, on_left_monitor(-1900.0));
    let beside = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &beside).at,
        Some(Point {
            x: -1868.0,
            y: 132.0
        })
    );
    // Partly off the monitor's left edge, as a reader may drag a window.
    active_window_at(&documents, &recorder, on_left_monitor(-2000.0));
    let kept_on = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &kept_on).at,
        Some(Point {
            x: -1920.0,
            y: 132.0
        })
    );
}

#[test]
fn new_window_larger_than_the_screen_is_made_to_fit_it() {
    let (documents, recorder) = documents();
    let huge = WindowPlace {
        size: size(3000, 2000),
        ..place_at(0.0, 0.0)
    };
    close_window_at(&documents, &recorder, Some(huge));
    active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    let label = open_empty(&documents);
    let new_window = opened_as(&recorder, &label);
    assert_eq!(new_window.size, size(1920, 1040));
    assert_eq!(new_window.at, Some(Point { x: 0.0, y: 0.0 }));
}

#[test]
fn new_window_without_an_active_window_is_made_to_fit_the_main_screen() {
    // As a size saved on a large monitor, at a cold start on a smaller one, from Explorer.
    let (documents, recorder) = recorded_with(Settings {
        window_size: Some(size(3000, 2000)),
        ..Settings::default()
    });
    recorder.set_primary_work_area(SCREEN);
    let label = open_new(&documents, &manifest());
    let new_window = opened_as(&recorder, &label);
    assert_eq!(new_window.size, size(1920, 1040));
    assert_eq!(new_window.at, None);
}

#[test]
fn new_window_never_opens_too_small_to_find() {
    // As a hand-edited settings file may say.
    let (documents, recorder) = recorded_with(Settings {
        window_size: Some(size(20, 10)),
        ..Settings::default()
    });
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(320, 240));
}

#[test]
fn without_an_active_window_the_system_places_the_new_window() {
    let (documents, recorder) = documents();
    let first = open_empty(&documents);
    recorder.place(&first, place_at(100.0, 100.0));
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).at, None);
}

#[test]
fn closed_active_window_leaves_the_new_window_to_the_system() {
    let (documents, recorder) = documents();
    let active = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    documents.forget(&active);
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).at, None);
}

#[test]
fn minimized_or_maximized_active_window_leaves_the_new_window_to_the_system() {
    let (documents, recorder) = documents();
    let active = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    recorder.unplace(&active);
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).at, None);
}
