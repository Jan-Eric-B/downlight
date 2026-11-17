//! Zoom (ticket 16): Zoom in, Zoom out and Reset zoom change one zoom level that every window
//! shares. It is kept in the settings, so every new window opens at it, and every open window's
//! page is sent it (`Documents::change_settings`). The page asks for a change with the settings'
//! `change_setting` command. The page zooms only the document view, with CSS `zoom`, so the title
//! bar, the find bar and notices keep their size (window design spec).

use serde::{Deserialize, Serialize};

/// The zoom level of a reader who never zoomed, and after Reset zoom.
const DEFAULT: u16 = 100;

/// The smallest zoom level, in percent.
const SMALLEST: u16 = 50;

/// The largest zoom level, in percent.
const LARGEST: u16 = 300;

/// The levels Zoom in and Zoom out step through, in percent: those of Edge and Chrome between
/// [`SMALLEST`] and [`LARGEST`], so a step feels as it does in the browser.
const LEVELS: [u16; 13] = [
    SMALLEST, 67, 75, 80, 90, DEFAULT, 110, 125, 150, 175, 200, 250, LARGEST,
];

/// A zoom level in percent, always between [`SMALLEST`] and [`LARGEST`]. It goes to the page and
/// into the settings file as the number. A level hand-edited into the file outside that range is
/// brought into it once, when the settings load, so nothing after has to check it; one between
/// two steps is kept, and steps to the next level either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u16", into = "u16")]
pub struct Zoom(u16);

impl Default for Zoom {
    fn default() -> Self {
        Self(DEFAULT)
    }
}

impl From<u16> for Zoom {
    fn from(percent: u16) -> Self {
        Self(percent.clamp(SMALLEST, LARGEST))
    }
}

impl From<Zoom> for u16 {
    fn from(zoom: Zoom) -> Self {
        zoom.0
    }
}

impl Zoom {
    /// The level `change` takes this one to; at either end it stays.
    pub fn step(self, change: ZoomChange) -> Self {
        let level = match change {
            ZoomChange::In => LEVELS.into_iter().find(|&level| level > self.0),
            ZoomChange::Out => LEVELS.into_iter().rev().find(|&level| level < self.0),
            ZoomChange::Reset => Some(DEFAULT),
        };
        level.map_or(self, Self)
    }
}

/// A change of the zoom level, as the page asks for it in a `SettingChange`
/// (`app/src/settings/change.rs`, `ZoomChange` in `page/src/app.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ZoomChange {
    /// To the next larger level.
    In,
    /// To the next smaller level.
    Out,
    /// Back to 100%.
    Reset,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The level in percent that `change` takes `percent` to.
    fn zoomed(percent: u16, change: ZoomChange) -> u16 {
        Zoom::from(percent).step(change).into()
    }

    #[test]
    fn zoom_in_and_out_step_through_the_levels() {
        assert_eq!(zoomed(100, ZoomChange::In), 110);
        assert_eq!(zoomed(110, ZoomChange::In), 125);
        assert_eq!(zoomed(100, ZoomChange::Out), 90);
        assert_eq!(zoomed(67, ZoomChange::Out), 50);
    }

    #[test]
    fn zoom_stays_at_either_end() {
        assert_eq!(zoomed(LARGEST, ZoomChange::In), LARGEST);
        assert_eq!(zoomed(SMALLEST, ZoomChange::Out), SMALLEST);
    }

    #[test]
    fn reset_zoom_goes_back_to_100_percent() {
        assert_eq!(zoomed(250, ZoomChange::Reset), 100);
        assert_eq!(zoomed(50, ZoomChange::Reset), 100);
    }

    #[test]
    fn hand_edited_level_steps_to_the_next_level_either_way() {
        assert_eq!(zoomed(130, ZoomChange::In), 150);
        assert_eq!(zoomed(130, ZoomChange::Out), 125);
        assert_eq!(zoomed(5000, ZoomChange::Out), 250);
        assert_eq!(zoomed(1, ZoomChange::In), 67);
    }

    #[test]
    fn hand_edited_level_out_of_range_loads_within_it() {
        let loaded = |json| u16::from(serde_json::from_str::<Zoom>(json).unwrap_or_default());
        assert_eq!(loaded("5000"), LARGEST);
        assert_eq!(loaded("1"), SMALLEST);
        assert_eq!(loaded("130"), 130);
    }
}
