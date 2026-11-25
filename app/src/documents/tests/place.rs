//! Where a new window opens and at what size (ticket 16): the size of the last window that
//! closed, fitted to the screen, and a place down and to the right of the active window. Before
//! any window has closed, a size shaped for reading, centred on the main screen without an active
//! window, and windows opened together cascade from each other (window design ticket 17).

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

/// Documents whose settings keep 800 by 600 as the last closed window's size, as the cascade
/// tests' places are worked out for.
fn with_a_kept_size() -> (Documents, Recorder) {
    recorded_with(Settings {
        window_size: Some(size(800, 600)),
        ..Settings::default()
    })
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

/// A 5120 by 2160 monitor's work area at 125%, its taskbar at the top.
const WIDE_SCREEN: Area = Area {
    left: 0.0,
    top: 48.0,
    width: 4096.0,
    height: 1680.0,
};

/// Documents with no size kept and `main_screen` as the main monitor's work area.
fn with_main_screen(main_screen: Area) -> (Documents, Recorder) {
    let (documents, recorder) = documents();
    recorder.set_primary_work_area(main_screen);
    (documents, recorder)
}

#[test]
fn new_window_opens_tall_and_narrow_until_a_window_has_closed() {
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    let label = open_new(&documents, &manifest());
    assert_eq!(opened_as(&recorder, &label).size, size(900, 1512));
}

#[test]
fn empty_window_opens_tall_and_narrow_until_a_window_has_closed() {
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(900, 1512));
}

#[test]
fn new_window_on_a_narrow_screen_keeps_a_margin_at_its_sides() {
    // A 1080 by 1920 portrait monitor at 150%.
    let (documents, recorder) = with_main_screen(Area {
        width: 720.0,
        height: 1232.0,
        ..SCREEN
    });
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(648, 1109));
}

#[test]
fn new_window_on_a_short_screen_is_at_least_600_high() {
    // 90% of 640 is 576.
    let (documents, recorder) = with_main_screen(Area {
        width: 1280.0,
        height: 640.0,
        ..SCREEN
    });
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(900, 600));
}

#[test]
fn new_window_on_a_screen_shorter_than_600_is_made_to_fit_it() {
    // A 1366 by 768 monitor at 125%.
    let (documents, recorder) = with_main_screen(Area {
        width: 1093.0,
        height: 566.0,
        ..SCREEN
    });
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).size, size(900, 566));
}

#[test]
fn new_window_without_an_active_window_is_centred_on_the_main_screen() {
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    let label = open_new(&documents, &manifest());
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point {
            x: 1598.0,
            y: 132.0
        })
    );
}

#[test]
fn windows_opened_together_without_an_active_window_cascade() {
    // As `downlight a.md b.md` opens them, before the reader is in either.
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    let first = open_new(&documents, &manifest());
    let second = open_new(&documents, &app_file("build.rs"));
    assert_eq!(
        opened_as(&recorder, &first).at,
        Some(Point {
            x: 1598.0,
            y: 132.0
        })
    );
    assert_eq!(
        opened_as(&recorder, &second).at,
        Some(Point {
            x: 1630.0,
            y: 164.0
        })
    );
}

#[test]
fn windows_opened_together_beside_an_active_window_cascade_from_each_other() {
    // As a second launch hands over two documents while the reader is in a window.
    let (documents, recorder) = with_a_kept_size();
    active_window_at(&documents, &recorder, place_at(100.0, 200.0));
    let first = open_new(&documents, &manifest());
    let second = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &first).at,
        Some(Point { x: 132.0, y: 232.0 })
    );
    assert_eq!(
        opened_as(&recorder, &second).at,
        Some(Point { x: 164.0, y: 264.0 })
    );
}

#[test]
fn reader_in_a_window_again_ends_the_cascade_from_the_window_opened_last() {
    // On SCREEN the default is 900 by 936, centred at 510, 52.
    let (documents, recorder) = with_main_screen(SCREEN);
    let first = open_empty(&documents);
    open_empty(&documents);
    recorder.place(
        &first,
        WindowPlace {
            size: size(900, 936),
            ..place_at(100.0, 50.0)
        },
    );
    documents.activated(&first);
    let label = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point { x: 132.0, y: 82.0 })
    );
}

#[test]
fn new_window_after_the_window_opened_last_closed_is_centred_again() {
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    let closed = open_new(&documents, &manifest());
    documents.forget(&closed);
    let label = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point {
            x: 1598.0,
            y: 132.0
        })
    );
}

#[test]
fn window_that_fails_to_open_leaves_the_cascade_from_the_one_before() {
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    open_new(&documents, &manifest());
    recorder.fail_window("document-1");
    assert!(documents.open_empty().is_err());
    let label = open_empty(&documents);
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point {
            x: 1630.0,
            y: 164.0
        })
    );
}

#[test]
fn new_window_beside_an_active_window_on_another_monitor_is_shaped_for_that_monitor() {
    let (documents, recorder) = with_main_screen(WIDE_SCREEN);
    // A 1440 by 2560 portrait monitor at 100%, right of the main one.
    let portrait = Area {
        left: 4096.0,
        top: 0.0,
        width: 1440.0,
        height: 2512.0,
    };
    active_window_at(
        &documents,
        &recorder,
        WindowPlace {
            work_area: portrait,
            ..place_at(4200.0, 50.0)
        },
    );
    let label = open_empty(&documents);
    let new_window = opened_as(&recorder, &label);
    assert_eq!(new_window.size, size(900, 2261));
    assert_eq!(new_window.at, Some(Point { x: 4232.0, y: 82.0 }));
}

#[test]
fn kept_size_wins_over_the_default_and_leaves_the_place_to_the_system() {
    let (documents, recorder) = recorded_with(Settings {
        window_size: Some(size(1000, 700)),
        ..Settings::default()
    });
    recorder.set_primary_work_area(WIDE_SCREEN);
    let label = open_new(&documents, &manifest());
    let new_window = opened_as(&recorder, &label);
    assert_eq!(new_window.size, size(1000, 700));
    assert_eq!(new_window.at, None);
}

#[test]
fn new_window_opens_down_and_right_of_the_active_window() {
    let (documents, recorder) = with_a_kept_size();
    active_window_at(&documents, &recorder, place_at(100.0, 200.0));
    let label = open_new(&documents, &manifest());
    assert_eq!(
        opened_as(&recorder, &label).at,
        Some(Point { x: 132.0, y: 232.0 })
    );
}

#[test]
fn new_window_follows_the_window_the_reader_was_in_last() {
    let (documents, recorder) = with_a_kept_size();
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
    let (documents, recorder) = with_a_kept_size();
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
    let (documents, recorder) = with_a_kept_size();
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

/// Documents with a size kept, as [`with_a_kept_size`], and [`SCREEN`] as the main monitor's work
/// area, so that a window left to the system is left to it by choice, not for want of a screen.
fn with_a_kept_size_and_a_main_screen() -> (Documents, Recorder) {
    let (documents, recorder) = with_a_kept_size();
    recorder.set_primary_work_area(SCREEN);
    (documents, recorder)
}

/// Where a window at the default size, 900 by 936 on [`SCREEN`], is centred there.
const CENTRED_ON_SCREEN: Point = Point { x: 510.0, y: 52.0 };

#[test]
fn window_at_a_kept_size_beside_one_the_system_placed_is_placed_by_the_system_too() {
    // The first has no place Downlight gave it to open beside, and the system cascades its own.
    let (documents, recorder) = with_a_kept_size_and_a_main_screen();
    let first = open_empty(&documents);
    recorder.place(&first, place_at(100.0, 100.0));
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).at, None);
}

#[test]
fn closed_active_window_leaves_a_window_at_a_kept_size_to_the_system() {
    let (documents, recorder) = with_a_kept_size_and_a_main_screen();
    let active = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    documents.forget(&active);
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).at, None);
}

#[test]
fn minimized_or_maximized_active_window_leaves_a_window_at_a_kept_size_to_the_system() {
    let (documents, recorder) = with_a_kept_size_and_a_main_screen();
    let active = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    recorder.unplace(&active);
    let label = open_empty(&documents);
    assert_eq!(opened_as(&recorder, &label).at, None);
}

#[test]
fn closed_active_window_leaves_a_window_at_the_default_size_centred_on_the_main_screen() {
    let (documents, recorder) = with_main_screen(SCREEN);
    let active = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    documents.forget(&active);
    let label = open_empty(&documents);
    let new_window = opened_as(&recorder, &label);
    assert_eq!(new_window.size, size(900, 936));
    assert_eq!(new_window.at, Some(CENTRED_ON_SCREEN));
}

#[test]
fn minimized_or_maximized_active_window_leaves_a_window_at_the_default_size_centred_on_the_main_screen()
 {
    let (documents, recorder) = with_main_screen(SCREEN);
    let active = active_window_at(&documents, &recorder, place_at(100.0, 100.0));
    recorder.unplace(&active);
    let label = open_empty(&documents);
    let new_window = opened_as(&recorder, &label);
    assert_eq!(new_window.size, size(900, 936));
    assert_eq!(new_window.at, Some(CENTRED_ON_SCREEN));
}
