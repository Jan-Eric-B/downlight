//! Where a new window opens and at what size (ticket 16): at the size of the last window that
//! closed, down and to the right of the active window, and on its screen. The system says where
//! windows are through the [`Windows`](super::Windows) port, in logical pixels, so this is worked
//! out and tested without a window.

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

/// The size of a new window before any window has closed: Tauri's default.
pub(super) const DEFAULT_SIZE: WindowSize = WindowSize {
    width: 800,
    height: 600,
};

/// The smallest size a new window opens at, so a size hand-edited into the settings file can't
/// open a window too small to find.
const SMALLEST_SIZE: WindowSize = WindowSize {
    width: 320,
    height: 240,
};

/// How far down and to the right of the active window a new window opens, in logical pixels: the
/// title bar's height, so the active window's title bar stays in sight above the new one.
const OFFSET: f64 = 32.0;

/// The size and place of a new window: the size of the last window that closed (`last_closed`),
/// [`OFFSET`] down and to the right of the `active` window if there is one. A new window that
/// wouldn't fit on the active window's screen there starts again at the screen's left or top
/// edge, as Windows cascades windows, and one larger than the screen is made to fit. Without an
/// active window the system places the new window on the main screen, `main_screen()`'s work
/// area, which it is made to fit too, since the size may come from a larger monitor.
pub(super) fn size_and_place(
    last_closed: Option<WindowSize>,
    active: Option<&WindowPlace>,
    main_screen: impl FnOnce() -> Option<Area>,
) -> (WindowSize, Option<Point>) {
    let size = last_closed.unwrap_or(DEFAULT_SIZE);
    let size = WindowSize {
        width: size.width.max(SMALLEST_SIZE.width),
        height: size.height.max(SMALLEST_SIZE.height),
    };
    let Some(active) = active else {
        let size = main_screen().map_or(size, |screen| fitted(size, screen));
        return (size, None);
    };
    let screen = active.work_area;
    let size = fitted(size, screen);
    let along = |start: f64, length: u32, screen_start: f64, screen_length: f64| {
        let start = start + OFFSET;
        if start + f64::from(length) > screen_start + screen_length {
            screen_start
        } else {
            start.max(screen_start)
        }
    };
    let at = Point {
        x: along(active.at.x, size.width, screen.left, screen.width),
        y: along(active.at.y, size.height, screen.top, screen.height),
    };
    (size, Some(at))
}

/// `size`, made no larger than `screen`.
fn fitted(size: WindowSize, screen: Area) -> WindowSize {
    WindowSize {
        width: size.width.min(screen.width as u32),
        height: size.height.min(screen.height as u32),
    }
}
