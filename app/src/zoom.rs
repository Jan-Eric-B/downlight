//! Zoom (ticket 16): Zoom in, Zoom out and Reset zoom change one zoom level that every window
//! shares. It is kept in the settings, so every new window opens at it, and every open window's
//! page is sent it (`Documents::change_settings`). The page zooms only the document view, with
//! CSS `zoom`, so the title bar, the find bar and notices keep their size (window design spec).

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::documents::Documents;
use crate::notice::Notice;

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

/// A change of the zoom level, as the page asks for it (`ZoomChange` in `page/src/app.ts`).
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

/// A sample of each change of the zoom `change_zoom` takes, by name, which the page's tests check
/// the changes the page asks for against (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new change stops the build here: give it a sample below, then name it here.
    let _sampled = |change: &ZoomChange| match change {
        ZoomChange::In | ZoomChange::Out | ZoomChange::Reset => {}
    };
    Ok(serde_json::json!({
        "in": serde_json::to_value(ZoomChange::In)?,
        "out": serde_json::to_value(ZoomChange::Out)?,
        "reset": serde_json::to_value(ZoomChange::Reset)?,
    }))
}

/// Zoom in, Zoom out or Reset zoom, from any window: changes the zoom level of every window.
/// Rejects with why when the level couldn't be saved; it holds until Downlight closes all the
/// same. Async, so the settings file isn't written on the main thread.
#[tauri::command]
pub async fn change_zoom(
    documents: State<'_, Documents>,
    change: ZoomChange,
) -> Result<(), Notice> {
    zoom(&documents, change)
}

fn zoom(documents: &Documents, change: ZoomChange) -> Result<(), Notice> {
    documents
        .change_settings(|settings| settings.zoom = settings.zoom.step(change))
        .map_err(|error| format!("{error}. The zoom holds until Downlight closes.").into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::recorder::recorded;
    use crate::message::Message;
    use crate::settings::PageSettings;

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

    #[test]
    fn zoom_in_reaches_every_window_and_new_ones_open_at_it() {
        let (documents, recorder) = recorded();
        let first = documents.open_empty().unwrap();
        let second = documents.open_empty().unwrap();
        documents.page_asks(&first);
        documents.page_asks(&second);
        // Without a config folder the zoom isn't saved, and the reader is told so.
        let zoomed = zoom(&documents, ZoomChange::In);
        assert!(zoomed.is_err(), "{zoomed:?}");
        let settings = Message::Settings(PageSettings {
            zoom: Zoom::from(110),
        });
        assert_eq!(
            recorder.sent(),
            [(first, settings.clone()), (second, settings.clone())]
        );
        let new_window = documents.open_empty().unwrap();
        assert_eq!(
            recorder
                .opened_as(&new_window)
                .map(|opened| opened.settings),
            Some(PageSettings {
                zoom: Zoom::from(110)
            })
        );
    }

    #[test]
    fn zoom_in_at_the_largest_level_tells_no_window() {
        let (documents, recorder) = recorded();
        let window = documents.open_empty().unwrap();
        documents.page_asks(&window);
        let _ = documents.change_settings(|settings| settings.zoom = Zoom::from(LARGEST));
        let before = recorder.sent().len();
        assert_eq!(zoom(&documents, ZoomChange::In), Ok(()));
        assert_eq!(recorder.sent().len(), before);
    }
}
