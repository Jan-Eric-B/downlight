//! A change of a shared setting, as the page asks for it (window design ticket 06's review fixes):
//! one command, `change_setting`, for every setting the page changes, rather than a command, a
//! notice and a broadcast test of its own for each. A new setting is a [`SettingChange`] variant
//! and its sample; `Documents::change_settings` sends every page the result.

use serde::Deserialize;
use tauri::State;

use crate::documents::Documents;
use crate::notice::Notice;
use crate::settings::{AccentColor, Backdrop, Settings, Theme};
use crate::zoom::ZoomChange;

/// A change of the settings every window shares (`SettingChange` in `page/src/app.ts`), tagged
/// with the setting it changes.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(
    tag = "setting",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SettingChange {
    /// Zoom in, Zoom out or Reset zoom (ticket 16).
    Zoom { change: ZoomChange },
    /// Full width (window design ticket 06), switched rather than set to what the page shows, so
    /// two windows that switch it at once each switch it, and every page then shows what the
    /// settings hold.
    SwitchFullWidth,
    /// The theme the reader picked in the settings flyout (window design ticket 08). Like every
    /// choice there, a set rather than a switch: the reader picked a value, not a change.
    Theme { theme: Theme },
    /// The backdrop the reader picked in the settings flyout (window design ticket 08).
    Backdrop { backdrop: Backdrop },
    /// The accent color the reader picked in the settings flyout (window design tickets 08 and
    /// 09).
    AccentColor { accent_color: AccentColor },
    /// "Default" under Edit with in the settings flyout (window design ticket 08): Edit forgets
    /// the chosen editor and opens the text file program again, or Notepad. Choosing an editor stays a command
    /// of its own (`choose_editor`), since it opens a dialog.
    ClearEditor,
}

impl SettingChange {
    /// Changes `settings` as this change says.
    pub fn apply(self, settings: &mut Settings) {
        match self {
            Self::Zoom { change } => settings.zoom = settings.zoom.step(change),
            Self::SwitchFullWidth => settings.full_width = !settings.full_width,
            Self::Theme { theme } => settings.theme = theme,
            Self::Backdrop { backdrop } => settings.backdrop = backdrop,
            Self::AccentColor { accent_color } => settings.accent_color = accent_color,
            Self::ClearEditor => settings.editor = None,
        }
    }

    /// The setting as the notice of a change that couldn't be saved names it.
    fn name(self) -> &'static str {
        match self {
            Self::Zoom { .. } => "The zoom",
            Self::SwitchFullWidth => "Full width",
            Self::Theme { .. } => "The theme",
            Self::Backdrop { .. } => "The backdrop",
            Self::AccentColor { .. } => "The accent color",
            Self::ClearEditor => "Edit with Default",
        }
    }
}

/// A sample of each change `change_setting` takes, by the command that asks for it, which the
/// page's tests check the changes the page asks for against (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new change stops the build here: give it a sample below, then name it here.
    // Every value of a choice too, since the settings flyout offers each one.
    let _sampled = |change: &SettingChange| match change {
        SettingChange::Zoom {
            change: ZoomChange::In | ZoomChange::Out | ZoomChange::Reset,
        }
        | SettingChange::SwitchFullWidth
        | SettingChange::Theme {
            theme: Theme::System | Theme::Light | Theme::Dark,
        }
        | SettingChange::Backdrop {
            backdrop: Backdrop::Mica | Backdrop::MicaAlt | Backdrop::Acrylic | Backdrop::None,
        }
        | SettingChange::AccentColor {
            accent_color: AccentColor::Amber,
        }
        | SettingChange::ClearEditor => {}
    };
    let zoom = |change| serde_json::to_value(SettingChange::Zoom { change });
    let theme = |theme| serde_json::to_value(SettingChange::Theme { theme });
    let backdrop = |backdrop| serde_json::to_value(SettingChange::Backdrop { backdrop });
    Ok(serde_json::json!({
        "zoomIn": zoom(ZoomChange::In)?,
        "zoomOut": zoom(ZoomChange::Out)?,
        "resetZoom": zoom(ZoomChange::Reset)?,
        "fullWidth": serde_json::to_value(SettingChange::SwitchFullWidth)?,
        "themeSystem": theme(Theme::System)?,
        "themeLight": theme(Theme::Light)?,
        "themeDark": theme(Theme::Dark)?,
        "backdropMica": backdrop(Backdrop::Mica)?,
        "backdropMicaAlt": backdrop(Backdrop::MicaAlt)?,
        "backdropAcrylic": backdrop(Backdrop::Acrylic)?,
        "backdropNone": backdrop(Backdrop::None)?,
        "accentColorAmber": serde_json::to_value(SettingChange::AccentColor {
            accent_color: AccentColor::Amber,
        })?,
        "clearEditor": serde_json::to_value(SettingChange::ClearEditor)?,
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
    fn choices_set_what_the_reader_picked_and_leave_the_rest() {
        let picked = changed(
            Settings::default(),
            SettingChange::Theme { theme: Theme::Dark },
        );
        let picked = changed(
            picked,
            SettingChange::Backdrop {
                backdrop: Backdrop::Acrylic,
            },
        );
        let picked = changed(
            picked,
            SettingChange::AccentColor {
                accent_color: AccentColor::Amber,
            },
        );
        assert_eq!(
            picked,
            Settings {
                theme: Theme::Dark,
                backdrop: Backdrop::Acrylic,
                ..Settings::default()
            }
        );
        // A set, not a switch: picking it again keeps it.
        let again = changed(picked.clone(), SettingChange::Theme { theme: Theme::Dark });
        assert_eq!(again, picked);
    }

    #[test]
    fn clearing_the_editor_forgets_the_chosen_one() {
        let chosen = Settings {
            editor: Some(r"C:\Tools\edit.exe".into()),
            zoom: Zoom::from(150),
            ..Settings::default()
        };
        let cleared = changed(chosen, SettingChange::ClearEditor);
        assert_eq!(cleared.editor, None);
        assert_eq!(cleared.zoom, Zoom::from(150));
    }

    #[test]
    fn theme_change_is_the_one_the_page_sends() {
        let change: SettingChange =
            serde_json::from_str(r#"{ "setting": "theme", "theme": "light" }"#).unwrap();
        assert_eq!(changed(Settings::default(), change).theme, Theme::Light);
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
