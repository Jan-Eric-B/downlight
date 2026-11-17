//! A change of a shared setting, as the page asks for it (window design ticket 06's review fixes):
//! one command, `change_setting`, for every setting the page changes, rather than a command, a
//! notice and a broadcast test of its own for each. A new setting is a [`SettingChange`] variant
//! and its sample; `Documents::change_settings` sends every page the result.

use serde::Deserialize;
use tauri::State;

use crate::documents::Documents;
use crate::notice::Notice;
use crate::settings::Settings;
use crate::zoom::ZoomChange;

/// A change of the settings every window shares (`SettingChange` in `page/src/app.ts`), tagged
/// with the setting it changes.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(tag = "setting", rename_all = "camelCase")]
pub enum SettingChange {
    /// Zoom in, Zoom out or Reset zoom (ticket 16).
    Zoom { change: ZoomChange },
    /// Full width (window design ticket 06), switched rather than set to what the page shows, so
    /// two windows that switch it at once each switch it, and every page then shows what the
    /// settings hold.
    SwitchFullWidth,
}

impl SettingChange {
    /// Changes `settings` as this change says.
    pub fn apply(self, settings: &mut Settings) {
        match self {
            Self::Zoom { change } => settings.zoom = settings.zoom.step(change),
            Self::SwitchFullWidth => settings.full_width = !settings.full_width,
        }
    }

    /// The setting as the notice of a change that couldn't be saved names it.
    fn name(self) -> &'static str {
        match self {
            Self::Zoom { .. } => "The zoom",
            Self::SwitchFullWidth => "Full width",
        }
    }
}

/// A sample of each change `change_setting` takes, by the command that asks for it, which the
/// page's tests check the changes the page asks for against (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new change stops the build here: give it a sample below, then name it here.
    let _sampled = |change: &SettingChange| match change {
        SettingChange::Zoom {
            change: ZoomChange::In | ZoomChange::Out | ZoomChange::Reset,
        }
        | SettingChange::SwitchFullWidth => {}
    };
    let zoom = |change| serde_json::to_value(SettingChange::Zoom { change });
    Ok(serde_json::json!({
        "zoomIn": zoom(ZoomChange::In)?,
        "zoomOut": zoom(ZoomChange::Out)?,
        "resetZoom": zoom(ZoomChange::Reset)?,
        "fullWidth": serde_json::to_value(SettingChange::SwitchFullWidth)?,
    }))
}

/// A change of a shared setting from any window, such as Zoom in or Full width: changes it for
/// every window. Rejects with why when the change couldn't be saved; it holds until Downlight
/// closes all the same. Async, so the settings file isn't written on the main thread.
#[tauri::command]
pub async fn change_setting(
    documents: State<'_, Documents>,
    change: SettingChange,
) -> Result<(), Notice> {
    change_in(&documents, change)
}

fn change_in(documents: &Documents, change: SettingChange) -> Result<(), Notice> {
    documents
        .change_settings(|settings| change.apply(settings))
        .map_err(|error| format!("{error}. {} holds until Downlight closes.", change.name()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::recorder::recorded;
    use crate::zoom::Zoom;

    /// `settings` after `change`.
    fn changed(mut settings: Settings, change: SettingChange) -> Settings {
        change.apply(&mut settings);
        settings
    }

    #[test]
    fn zoom_change_steps_the_zoom_and_leaves_the_rest() {
        let zoomed = |percent: u16, change| {
            let settings = Settings {
                zoom: Zoom::from(percent),
                ..Settings::default()
            };
            changed(settings, SettingChange::Zoom { change })
        };
        assert_eq!(zoomed(100, ZoomChange::In).zoom, Zoom::from(110));
        assert_eq!(zoomed(100, ZoomChange::Out).zoom, Zoom::from(90));
        assert_eq!(zoomed(250, ZoomChange::Reset).zoom, Zoom::from(100));
        assert!(!zoomed(100, ZoomChange::In).full_width);
    }

    #[test]
    fn switching_full_width_switches_it_on_and_off_again() {
        let on = changed(Settings::default(), SettingChange::SwitchFullWidth);
        assert!(on.full_width);
        assert_eq!(on.zoom, Zoom::default());
        let off = changed(on, SettingChange::SwitchFullWidth);
        assert!(!off.full_width);
    }

    #[test]
    fn unsaved_change_says_why_and_that_it_holds() {
        let (documents, _) = recorded();
        for (change, holds) in [
            (
                SettingChange::Zoom {
                    change: ZoomChange::In,
                },
                "The zoom holds until Downlight closes.",
            ),
            (
                SettingChange::SwitchFullWidth,
                "Full width holds until Downlight closes.",
            ),
        ] {
            // Without a config folder no change is saved.
            let Err(notice) = change_in(&documents, change) else {
                panic!("a store without a config folder saved {change:?}");
            };
            let notice = notice.to_string();
            assert!(notice.ends_with(holds), "{notice}");
            assert!(notice.starts_with("Downlight's settings"), "{notice}");
        }
    }
}
