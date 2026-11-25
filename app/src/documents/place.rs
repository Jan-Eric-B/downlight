//! Where a new window opens and at what size (ticket 16): at the size of the last window that
//! closed, down and to the right of the active window, or of the window opened just before it,
//! and on its screen; before any window has closed, tall and narrow for reading (window design
//! ticket 17). The system says where windows are through the [`Windows`](super::Windows) port,
//! in logical pixels, so this is worked out and tested without a window.

use serde::{Deserialize, Serialize};

/// A window's size as the reader sees it, in logical pixels, so that it is the same on a monitor
/// with another scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

/// Where a window is on screen, in the logical pixels of its monitor, as
/// [`Windows::place_of`](super::Windows::place_of) gives it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlace {
    /// Its top left corner.
    pub at: Point,
    pub size: WindowSize,
    /// The part of its monitor that windows go in: the screen without the taskbar.
    pub work_area: Area,
}

/// A point on screen, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A rectangle on screen, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// How wide a window at the default size is, unless its screen is narrow: a text column of about
/// 100 characters at 100% zoom (window design ticket 17).
const READING_WIDTH: u32 = 900;

/// How high a window at the default size is at least, unless its screen is shorter.
const LEAST_DEFAULT_HEIGHT: u32 = 600;

/// The size of a new window before any window has closed where the system names no screen to
/// shape it for.
pub(super) const DEFAULT_SIZE: WindowSize = WindowSize {
    width: READING_WIDTH,
    height: LEAST_DEFAULT_HEIGHT,
};

/// How much of its screen's work area a window at the default size takes: as tall as the screen
/// allows, with a margin, so it doesn't look maximized and its edges stay easy to grab, and no
/// wider, on a narrow screen, so it doesn't touch both edges.
const SCREEN_SHARE: f64 = 0.9;

/// The smallest size a new window opens at, so a size hand-edited into the settings file can't
/// open a window too small to find.
const SMALLEST_SIZE: WindowSize = WindowSize {
    width: 320,
    height: 240,
};

/// How far down and to the right of the active window a new window opens, in logical pixels: the
/// title bar's height, so the active window's title bar stays in sight above the new one.
const OFFSET: f64 = 32.0;

/// Who places a new window that has no window to open beside.
enum PlacedAlone {
    /// The system, as for a window at the size the last window that closed had.
    BySystem,
    /// Downlight, centred on the main screen, as for a window at the default size.
    Centred,
}

/// The size of a new window, and its place on screen, None when the system is to place it. The
/// size is the last closed window's (`last_closed`), or before any has closed the default shaped
/// for the screen the window opens on ([`default_for`]), made to fit that screen. The place is
/// [`OFFSET`] down and to the right of `beside`, the window it opens beside, on that window's
/// screen, starting again at the screen's left or top edge where it wouldn't fit, as Windows
/// cascades windows. Without `beside`, the screen is the main one (`main_screen()`).
pub(super) fn size_and_place(
    last_closed: Option<WindowSize>,
    beside: Option<&WindowPlace>,
    main_screen: impl FnOnce() -> Option<Area>,
) -> (WindowSize, Option<WindowPlace>) {
    let screen = beside.map(|beside| beside.work_area).or_else(main_screen);
    let (size, alone) = match (last_closed, screen) {
        (Some(last_closed), _) => (last_closed, PlacedAlone::BySystem),
        (None, Some(screen)) => (default_for(screen), PlacedAlone::Centred),
        (None, None) => (DEFAULT_SIZE, PlacedAlone::BySystem),
    };
    let size = WindowSize {
        width: size.width.max(SMALLEST_SIZE.width),
        height: size.height.max(SMALLEST_SIZE.height),
    };
    let Some(screen) = screen else {
        return (size, None);
    };
    // Fitted even without a window to open beside, since a kept size may come from a larger
    // monitor.
    let size = fitted(size, screen);
    let placed = |at| {
        Some(WindowPlace {
            at,
            size,
            work_area: screen,
        })
    };
    let Some(beside) = beside else {
        return match alone {
            PlacedAlone::BySystem => (size, None),
            // The system's default place (CW_USEDEFAULT) moves each new window down about a title
            // bar, and fitting only shrinks a window, so one 90% of the screen high would reach
            // under the taskbar after a few launches (window design ticket 17).
            PlacedAlone::Centred => (size, placed(centred(size, screen))),
        };
    };
    let along = |start: f64, length: u32, screen_start: f64, screen_length: f64| {
        let start = start + OFFSET;
        if start + f64::from(length) > screen_start + screen_length {
            screen_start
        } else {
            start.max(screen_start)
        }
    };
    let at = Point {
        x: along(beside.at.x, size.width, screen.left, screen.width),
        y: along(beside.at.y, size.height, screen.top, screen.height),
    };
    (size, placed(at))
}

/// The default size for a window on `screen`: [`READING_WIDTH`], or less on a narrow screen, and
/// [`SCREEN_SHARE`] of its height, or [`LEAST_DEFAULT_HEIGHT`] on a short one, with no upper
/// limit, so a portrait screen's window is as tall as the screen allows. It may then be too tall
/// for a very short screen, and is fitted to it as every new window is.
fn default_for(screen: Area) -> WindowSize {
    // Rounded, as ticket 17's table works the default out; a share of a length never reaches
    // past it, so this can't take the window off the screen, as rounding in `fitted` could.
    let share = |length: f64| (length * SCREEN_SHARE).round() as u32;
    WindowSize {
        width: READING_WIDTH.min(share(screen.width)),
        height: LEAST_DEFAULT_HEIGHT.max(share(screen.height)),
    }
}

/// The top left corner of a window `size` large centred on `screen`.
fn centred(size: WindowSize, screen: Area) -> Point {
    Point {
        x: screen.left + (screen.width - f64::from(size.width)) / 2.0,
        y: screen.top + (screen.height - f64::from(size.height)) / 2.0,
    }
}

/// `size`, made no larger than `screen`, whose fractional logical lengths are truncated so the
/// window never runs a fraction of a pixel past its edge.
fn fitted(size: WindowSize, screen: Area) -> WindowSize {
    WindowSize {
        width: size.width.min(screen.width as u32),
        height: size.height.min(screen.height as u32),
    }
}
