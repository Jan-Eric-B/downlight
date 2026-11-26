//! The settings (ticket 15): the reader's choices that every window shares, kept in one file,
//! `settings.json` in the app's config folder. The file is created on the first change, so a
//! reader who changes nothing has none. Later tickets add their values to [`Settings`]; a file
//! without them loads with their defaults, and a file from a newer version, with values this one
//! doesn't know, still loads.

pub mod change;
mod look;

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Deserializer, Serialize};

#[cfg(doc)]
use crate::documents::Documents;
use crate::documents::WindowSize;
use crate::zoom::Zoom;

pub use look::{AccentColor, AccentScale, Backdrop, Rgb, Theme, WindowLook};

const FILE_NAME: &str = "settings.json";

/// Where a file that doesn't parse is kept, so a hand-edited file with a typo isn't lost.
const BAD_FILE_NAME: &str = "settings.json.bad";

/// The reader's choices, as the settings file holds them. A value the file leaves out takes its
/// default, and so does one it holds that doesn't parse, such as a theme a hand edit misspelled,
/// while the others keep theirs (window design ticket 08). Every field takes [`or_default`], or
/// [`or_on`] for a switch that is on by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// The program Edit opens a document in, as Choose editor... picked it; None for the text
    /// file program.
    #[serde(deserialize_with = "or_default")]
    pub editor: Option<PathBuf>,
    /// The document view's zoom level (ticket 16), as Zoom in, Zoom out and Reset zoom set it.
    #[serde(deserialize_with = "or_default")]
    pub zoom: Zoom,
    /// The size of the last window that closed, which a new window opens at (ticket 16); None
    /// until a window has closed. Kept as the window had it: a new window is fitted to its
    /// screen when it opens, since the screen may have changed since.
    #[serde(deserialize_with = "or_default")]
    pub window_size: Option<WindowSize>,
    /// Whether the document fills the window's width instead of GitHub's reading width (window
    /// design ticket 06); off for a reader who never switched it.
    #[serde(deserialize_with = "or_default")]
    pub full_width: bool,
    /// Whether every window shows the overview ruler beside its scroll bar (overview ruler ticket
    /// 02); on for a reader who never switched it off, so it is found without looking for it.
    #[serde(deserialize_with = "or_on")]
    pub overview_ruler: bool,
    /// Light or dark, or as Windows has it (window design ticket 08).
    #[serde(deserialize_with = "or_default")]
    pub theme: Theme,
    /// The material behind every window where the OS can show one (window design ticket 08).
    #[serde(deserialize_with = "or_default")]
    pub backdrop: Backdrop,
    /// The color scale of what Downlight adds to a document (window design tickets 08 and 09).
    #[serde(deserialize_with = "or_default")]
    pub accent_color: AccentColor,
}

/// The settings of a reader who changed none: each value's default, but for the overview ruler,
/// which is on. Also what `#[serde(default)]` takes a value the file leaves out from.
impl Default for Settings {
    fn default() -> Self {
        Self {
            editor: None,
            zoom: Zoom::default(),
            window_size: None,
            full_width: false,
            overview_ruler: true,
            theme: Theme::default(),
            backdrop: Backdrop::default(),
            accent_color: AccentColor::default(),
        }
    }
}

/// A field's value from the settings file, or the field's default when the value is there but
/// doesn't parse as one. A file that isn't JSON at all still fails as a whole, and is kept as
/// `settings.json.bad`: reading the value fails before its type is looked at.
fn or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

/// A switch's value from the settings file, as [`or_default`] reads one, but on when the value
/// doesn't parse: for a switch that is on by default, where `bool`'s own default is off.
fn or_on<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(bool::deserialize(value).unwrap_or(true))
}

impl Settings {
    /// What of the settings the pages show, as every page gets it: in its URL when its window
    /// opens, and in a `settings` message whenever it changes (window design ticket 12), with
    /// `windows_accent`, Windows' accent scale now, if the OS has one (window design ticket 09).
    /// The editor stays in the app.
    pub fn page(&self, windows_accent: Option<AccentScale>) -> PageSettings {
        PageSettings {
            zoom: self.zoom,
            full_width: self.full_width,
            overview_ruler: self.overview_ruler,
            theme: self.theme,
            backdrop: self.backdrop,
            accent_color: self.accent_color,
            windows_accent,
        }
    }

    /// What of the settings acts on the windows themselves (window design ticket 08): every new
    /// window is built with it, and a change of it reaches every window.
    pub fn window_look(&self) -> WindowLook {
        WindowLook {
            theme: self.theme,
            backdrop: self.backdrop,
        }
    }
}

/// The settings a page shows, one snapshot for all of them (window design ticket 12), so that a
/// setting a page shows is a field here and a listener in the page (`whenSettingsChange` in
/// `page/src/settings.ts`) rather than a message and a URL flag of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSettings {
    /// The document view's zoom level, in percent.
    pub zoom: Zoom,
    /// Whether the document fills the window's width (window design ticket 06).
    pub full_width: bool,
    /// Whether the page shows the overview ruler (overview ruler ticket 02).
    pub overview_ruler: bool,
    /// The settings flyout shows the reader's theme; the app sets it on the window, which gives
    /// the page its `prefers-color-scheme` (window design ticket 08).
    pub theme: Theme,
    /// The page draws its solid background with Backdrop None, and none on any other backdrop the
    /// OS can show (window design ticket 08).
    pub backdrop: Backdrop,
    pub accent_color: AccentColor,
    /// Windows' accent scale, which the page colors with while the reader's accent color is
    /// Windows', and changed whenever Windows' changes (window design ticket 09). Not the reader's
    /// choice but the OS's, sent with the settings so that it reaches every page as they do, held
    /// for a page that doesn't hear them yet. None where the OS has no accent color of its own,
    /// and the settings flyout then offers Amber alone.
    pub windows_accent: Option<AccentScale>,
}

/// The snapshot of a reader who changed nothing, from [`Settings::default`], so the defaults live
/// in one place, on an OS without an accent color of its own. The page's `DEFAULT_SETTINGS` is
/// checked against it through the page URL's "defaults" contract sample
/// (`window::contract_samples`).
impl Default for PageSettings {
    fn default() -> Self {
        Settings::default().page(None)
    }
}

impl PageSettings {
    /// The snapshot the contract samples name, the same in the page's URL and in the `settings`
    /// message. Off the defaults, so a page that ignored it would fail its test.
    #[cfg(test)]
    pub fn sample() -> Self {
        Self {
            zoom: Zoom::from(125),
            full_width: true,
            overview_ruler: false,
            theme: Theme::Dark,
            backdrop: Backdrop::None,
            accent_color: AccentColor::Windows,
            windows_accent: Some(AccentScale::sample()),
        }
    }
}

/// What [`SettingsStore::update`] did: the settings before and after the change, and whether they
/// were saved.
#[derive(Debug)]
pub struct Updated {
    /// The settings before the change.
    pub before: Settings,
    /// The settings now, which hold until Downlight closes even if they weren't saved.
    pub settings: Settings,
    /// Whether the change reached the file; a change that changed nothing writes nothing.
    pub saved: Result<(), SettingsError>,
}

/// The settings for this process: loaded once at the start, read by anyone who needs them
/// ([`SettingsStore::get`]), and changed only by [`Documents::change_settings`], so that a change
/// to what the pages show always reaches every page. One store is shared by the documents module
/// and Tauri's managed state (`Arc<SettingsStore>`), so a reader such as Edit reads it without
/// going through the documents module.
pub struct SettingsStore {
    /// None when the system gives Downlight no config folder; the settings then last until
    /// Downlight closes.
    file: Option<PathBuf>,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// The settings in `folder`'s settings file, or the defaults, with why when the file is there
    /// and can't be used. A missing file is no problem: it is created on the first change. A file
    /// that doesn't parse is kept as `settings.json.bad`, replacing an older one, so the next
    /// change doesn't write over it.
    pub fn load(folder: Option<&Path>) -> (Self, Option<SettingsError>) {
        let file = folder.map(|folder| folder.join(FILE_NAME));
        let (settings, error) = match &file {
            Some(file) => read(file),
            None => (Settings::default(), None),
        };
        let store = Self {
            file,
            current: Mutex::new(settings),
        };
        (store, error)
    }

    /// The settings for a process that has only these, which are never saved: there is no config
    /// folder.
    #[cfg(test)]
    pub fn holding(settings: Settings) -> Self {
        Self {
            file: None,
            current: Mutex::new(settings),
        }
    }

    /// The settings now, whole (window design ticket 12).
    pub fn get(&self) -> Settings {
        self.lock().clone()
    }

    /// Runs `read` with the settings now, and holds them until it returns, so that no change comes
    /// between what `read` reads and what it does with it: a page caught up with the settings is
    /// sent them before any change after.
    pub fn read<R>(&self, read: impl FnOnce(&Settings) -> R) -> R {
        read(&self.lock())
    }

    /// Changes the settings with `change`, saves the whole file, and runs `then` with what the
    /// change did before another change can start, so that of two changes at once, what `then`
    /// does for the later one comes last. A change that can't be saved still holds until
    /// Downlight closes, and [`Updated::saved`] says why it wasn't saved. Only
    /// [`Documents::change_settings`] calls this: `then` is how every page hears of a change.
    pub fn update(
        &self,
        change: impl FnOnce(&mut Settings),
        then: impl FnOnce(&Updated),
    ) -> Updated {
        // Saved under the lock, so two changes at once reach the file in the order they were made.
        let mut current = self.lock();
        let before = current.clone();
        change(&mut current);
        let saved = if *current == before {
            // Such as Zoom in at the largest zoom, or a window closed at the size the last one had.
            Ok(())
        } else {
            self.save(&current)
        };
        let updated = Updated {
            before,
            settings: current.clone(),
            saved,
        };
        then(&updated);
        updated
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        let file = self.file.as_deref().ok_or(SettingsError::NoConfigFolder)?;
        write(file, settings).map_err(|source| SettingsError::Write {
            file: file.to_owned(),
            source,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Settings> {
        // The settings are replaced whole, so a panic elsewhere never leaves them half-changed.
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The settings in `file`, or the defaults with why not.
fn read(file: &Path) -> (Settings, Option<SettingsError>) {
    let bytes = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return (Settings::default(), None);
        }
        Err(source) => {
            let error = SettingsError::Read {
                file: file.to_owned(),
                source,
            };
            return (Settings::default(), Some(error));
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(settings) => (settings, None),
        Err(source) => {
            let bad = file.with_file_name(BAD_FILE_NAME);
            // `rename` replaces an older bad file.
            let kept = std::fs::rename(file, &bad).map(|()| bad);
            let error = SettingsError::Parse {
                file: file.to_owned(),
                source,
                kept,
            };
            (Settings::default(), Some(error))
        }
    }
}

/// Writes `settings` to `file` whole, creating its folder. The file is written beside it first and
/// renamed over it, so a failure halfway leaves the old file as it was.
fn write(file: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut json = serde_json::to_string_pretty(settings)?;
    json.push('\n');
    let written = file.with_extension("json.tmp");
    std::fs::write(&written, json)?;
    std::fs::rename(&written, file).inspect_err(|_| {
        let _ = std::fs::remove_file(&written);
    })
}

/// Why the settings file can't be used or saved.
#[derive(Debug)]
pub enum SettingsError {
    /// The system gives Downlight no config folder to keep its settings in.
    NoConfigFolder,
    /// The file is there and can't be read; it is left in place.
    Read {
        file: PathBuf,
        source: io::Error,
    },
    /// The file isn't valid settings. `kept` is where it was kept, or why it couldn't be.
    Parse {
        file: PathBuf,
        source: serde_json::Error,
        kept: Result<PathBuf, io::Error>,
    },
    Write {
        file: PathBuf,
        source: io::Error,
    },
}

/// The error as the reader is told it, in a message box when Downlight starts or in a notice
/// when a change isn't saved.
impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConfigFolder => {
                f.write_str("Downlight's settings can't be saved: there is no folder for them")
            }
            Self::Read { file, source } => write!(
                f,
                "Downlight's settings in {} can't be read: {source}. Defaults are used, and the \
                 next change replaces the file.",
                file.display()
            ),
            Self::Parse { file, source, kept } => {
                write!(
                    f,
                    "Downlight's settings in {} can't be read: {source}. Defaults are used, and ",
                    file.display()
                )?;
                match kept {
                    Ok(bad) => write!(f, "the file was kept as {}.", bad.display()),
                    Err(error) => write!(
                        f,
                        "the next change replaces the file, which couldn't be kept as \
                         {BAD_FILE_NAME}: {error}."
                    ),
                }
            }
            Self::Write { file, source } => write!(
                f,
                "Downlight's settings can't be saved in {}: {source}",
                file.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own, which doesn't exist yet, and is removed when dropped.
    struct Folder(PathBuf);

    impl Folder {
        fn new(test: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("downlight-settings-{}-{test}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }

        fn with_file(test: &str, contents: &str) -> Self {
            let folder = Self::new(test);
            std::fs::create_dir_all(&folder.0).unwrap();
            std::fs::write(folder.file(), contents).unwrap();
            folder
        }

        fn file(&self) -> PathBuf {
            self.0.join(FILE_NAME)
        }

        fn load(&self) -> (SettingsStore, Option<SettingsError>) {
            SettingsStore::load(Some(&self.0))
        }
    }

    impl SettingsStore {
        /// An update that tells nobody, as these tests check only the store.
        fn change(&self, change: impl FnOnce(&mut Settings)) -> Updated {
            self.update(change, |_| {})
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_loads_defaults_and_creates_no_file() {
        let folder = Folder::new("missing");
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(store.get().editor, None);
        assert!(!folder.0.exists());
    }

    #[test]
    fn first_save_creates_the_file_and_its_folder() {
        let folder = Folder::new("first-save");
        let (store, _) = folder.load();
        store
            .change(|settings| settings.editor = Some(PathBuf::from(r"C:\Tools\edit.exe")))
            .saved
            .unwrap();
        assert!(folder.file().is_file());
    }

    #[test]
    fn saved_editor_loads_after_a_restart() {
        let folder = Folder::new("restart");
        let editor = PathBuf::from(r"C:\Program Files\Notepad++\notepad++.exe");
        folder
            .load()
            .0
            .change(|settings| settings.editor = Some(editor.clone()))
            .saved
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(restarted.get().editor, Some(editor));
    }

    #[test]
    fn corrupt_file_loads_defaults_and_reports_why() {
        let folder = Folder::with_file("corrupt", "{ \"editor\": ");
        let (store, error) = folder.load();
        assert_eq!(store.get().editor, None);
        let Some(error @ SettingsError::Parse { .. }) = error else {
            panic!("expected a parse error, got {error:?}");
        };
        let message = error.to_string();
        assert!(
            message.contains(&folder.file().display().to_string()),
            "{message}"
        );
        assert!(message.contains(BAD_FILE_NAME), "{message}");
    }

    #[test]
    fn corrupt_file_is_kept_as_bad() {
        let corrupt = "{ \"editor\": ";
        let folder = Folder::with_file("kept", corrupt);
        std::fs::write(folder.0.join(BAD_FILE_NAME), "an older bad file").unwrap();
        let (store, _) = folder.load();
        assert_eq!(
            std::fs::read_to_string(folder.0.join(BAD_FILE_NAME)).unwrap(),
            corrupt
        );
        store
            .change(|settings| settings.zoom = Zoom::from(150))
            .saved
            .unwrap();
        let (_, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
    }

    #[test]
    fn file_without_an_editor_loads_with_none() {
        let folder = Folder::with_file("without-editor", "{}");
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(store.get().editor, None);
    }

    #[test]
    fn file_with_unknown_fields_keeps_known_values() {
        let folder = Folder::with_file(
            "unknown-fields",
            r#"{ "editor": "C:\\Tools\\edit.exe", "fromANewerVersion": 3 }"#,
        );
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(
            store.get().editor,
            Some(PathBuf::from(r"C:\Tools\edit.exe"))
        );
    }

    #[test]
    fn failed_save_keeps_the_change_for_this_session() {
        let folder = Folder::new("failed-save");
        // A folder where the file should be, so writing it fails.
        std::fs::create_dir_all(folder.file()).unwrap();
        let (store, _) = folder.load();
        let editor = PathBuf::from(r"C:\Tools\edit.exe");
        let saved = store
            .change(|settings| settings.editor = Some(editor.clone()))
            .saved;
        assert!(
            matches!(saved, Err(SettingsError::Write { .. })),
            "{saved:?}"
        );
        assert_eq!(store.get().editor, Some(editor));
    }

    #[test]
    fn without_a_config_folder_a_change_holds_and_says_it_was_not_saved() {
        let (store, error) = SettingsStore::load(None);
        assert!(error.is_none(), "{error:?}");
        let editor = PathBuf::from(r"C:\Tools\edit.exe");
        let saved = store
            .change(|settings| settings.editor = Some(editor.clone()))
            .saved;
        assert!(
            matches!(saved, Err(SettingsError::NoConfigFolder)),
            "{saved:?}"
        );
        assert_eq!(store.get().editor, Some(editor));
    }

    #[test]
    fn zoom_defaults_to_100_percent_and_no_window_size() {
        let folder = Folder::with_file("zoom-default", r#"{ "editor": null }"#);
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(u16::from(store.get().zoom), 100);
        assert_eq!(store.get().window_size, None);
    }

    #[test]
    fn saved_zoom_and_window_size_load_after_a_restart() {
        let folder = Folder::new("zoom-restart");
        let size = WindowSize {
            width: 1024,
            height: 768,
        };
        folder
            .load()
            .0
            .change(|settings| {
                settings.zoom = Zoom::from(125);
                settings.window_size = Some(size);
            })
            .saved
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(u16::from(restarted.get().zoom), 125);
        assert_eq!(restarted.get().window_size, Some(size));
    }

    #[test]
    fn full_width_defaults_to_off() {
        let folder = Folder::with_file("full-width-default", r#"{ "zoom": 125 }"#);
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert!(!store.get().full_width);
        assert!(!store.get().page(None).full_width);
    }

    #[test]
    fn saved_full_width_loads_after_a_restart() {
        let folder = Folder::new("full-width-restart");
        folder
            .load()
            .0
            .change(|settings| settings.full_width = true)
            .saved
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert!(restarted.get().full_width);
        assert!(restarted.get().page(None).full_width);
    }

    #[test]
    fn overview_ruler_defaults_to_on() {
        let without_any = Folder::new("ruler-without-file");
        let (store, _) = without_any.load();
        assert!(store.get().page(None).overview_ruler);
        let without_it = Folder::with_file("ruler-default", r#"{ "zoom": 125 }"#);
        let (store, error) = without_it.load();
        assert!(error.is_none(), "{error:?}");
        assert!(store.get().overview_ruler);
        assert!(store.get().page(None).overview_ruler);
    }

    #[test]
    fn overview_ruler_switched_off_stays_off_after_a_restart() {
        let folder = Folder::new("ruler-restart");
        folder
            .load()
            .0
            .change(|settings| settings.overview_ruler = false)
            .saved
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert!(!restarted.get().overview_ruler);
        assert!(!restarted.get().page(None).overview_ruler);
    }

    #[test]
    fn update_gives_the_settings_after_the_change() {
        let (store, _) = SettingsStore::load(None);
        let updated = store.change(|settings| settings.zoom = Zoom::from(150));
        assert_eq!(u16::from(updated.settings.zoom), 150);
        assert_eq!(updated.settings, store.get());
    }

    #[test]
    fn change_that_changes_nothing_writes_no_file() {
        let folder = Folder::new("no-change");
        let (store, _) = folder.load();
        let updated = store.change(|settings| settings.zoom = Zoom::from(100));
        assert!(updated.saved.is_ok(), "{:?}", updated.saved);
        assert!(!folder.0.exists());
    }

    #[test]
    fn hand_edited_zoom_out_of_range_loads_within_it() {
        let too_large = Folder::with_file("zoom-too-large", r#"{ "zoom": 5000 }"#);
        let too_small = Folder::with_file("zoom-too-small", r#"{ "zoom": 1 }"#);
        let zoom_in = |folder: &Folder| {
            let (store, error) = folder.load();
            assert!(error.is_none(), "{error:?}");
            u16::from(store.get().page(None).zoom)
        };
        assert_eq!(zoom_in(&too_large), 300);
        assert_eq!(zoom_in(&too_small), 50);
    }

    #[test]
    fn theme_backdrop_and_accent_color_default_to_system_mica_and_amber() {
        let folder = Folder::with_file("look-default", r#"{ "zoom": 125 }"#);
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        let settings = store.get();
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.backdrop, Backdrop::Mica);
        assert_eq!(settings.accent_color, AccentColor::Amber);
    }

    #[test]
    fn saved_theme_and_backdrop_load_after_a_restart() {
        let folder = Folder::new("look-restart");
        folder
            .load()
            .0
            .change(|settings| {
                settings.theme = Theme::Dark;
                settings.backdrop = Backdrop::MicaAlt;
            })
            .saved
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(
            restarted.get().window_look(),
            WindowLook {
                theme: Theme::Dark,
                backdrop: Backdrop::MicaAlt,
            }
        );
    }

    #[test]
    fn broken_value_loads_its_default_and_keeps_the_others() {
        let folder = Folder::with_file(
            "broken-value",
            r#"{ "theme": "purple", "backdrop": 3, "zoom": 125, "fullWidth": "yes",
                 "overviewRuler": "off", "editor": "C:\\Tools\\edit.exe",
                 "windowSize": { "width": "wide" } }"#,
        );
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        let settings = store.get();
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.backdrop, Backdrop::Mica);
        assert!(!settings.full_width);
        assert!(settings.overview_ruler);
        assert_eq!(settings.window_size, None);
        assert_eq!(u16::from(settings.zoom), 125);
        assert_eq!(settings.editor, Some(PathBuf::from(r"C:\Tools\edit.exe")));
        // Nothing was wrong with the file as a whole, so it stays where it is.
        assert!(!folder.0.join(BAD_FILE_NAME).exists());
    }

    #[test]
    fn saved_windows_accent_color_loads_after_a_restart() {
        let folder = Folder::new("accent-restart");
        folder
            .load()
            .0
            .change(|settings| settings.accent_color = AccentColor::Windows)
            .saved
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(restarted.get().accent_color, AccentColor::Windows);
    }

    #[test]
    fn page_shows_the_theme_backdrop_and_accent_color() {
        let settings = Settings {
            theme: Theme::Light,
            backdrop: Backdrop::None,
            ..Settings::default()
        };
        let page = settings.page(None);
        assert_eq!(
            (page.theme, page.backdrop, page.accent_color),
            (Theme::Light, Backdrop::None, AccentColor::Amber)
        );
    }

    #[test]
    fn page_shows_windows_accent_scale_whatever_the_reader_chose() {
        // Sent with Amber chosen too, so a page whose reader picks Windows' has its scale at once.
        let page = Settings::default().page(Some(AccentScale::sample()));
        assert_eq!(page.windows_accent, Some(AccentScale::sample()));
    }

    #[test]
    fn accent_scale_goes_to_the_page_as_css_colors_by_the_stylesheet_s_shade_names() {
        let scale = serde_json::to_value(AccentScale::sample()).unwrap();
        assert_eq!(
            scale,
            serde_json::json!({
                "light-3": "#99ebff",
                "light-2": "#4cc2ff",
                "light-1": "#0091f8",
                "base": "#0078d4",
                "dark-1": "#0067c0",
                "dark-2": "#003e92",
                "dark-3": "#001a68",
            })
        );
    }

    #[test]
    fn update_tells_what_changed_before_another_change_starts() {
        let (store, _) = SettingsStore::load(None);
        let mut told = None;
        store.update(
            |settings| settings.zoom = Zoom::from(150),
            |updated| {
                // Held still: a read here would wait for ever if the update had let go.
                assert!(store.current.try_lock().is_err());
                told = Some((updated.before.zoom, updated.settings.zoom));
            },
        );
        assert_eq!(told, Some((Zoom::default(), Zoom::from(150))));
    }
}
