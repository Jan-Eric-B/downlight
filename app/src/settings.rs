//! The settings (ticket 15): the reader's choices that every window shares, kept in one file,
//! `settings.json` in the app's config folder. The file is created on the first change, so a
//! reader who changes nothing has none. Later tickets add their values to [`Settings`]; a file
//! without them loads with their defaults, and a file from a newer version, with values this one
//! doesn't know, still loads.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

#[cfg(doc)]
use crate::documents::Documents;
use crate::documents::WindowSize;
use crate::zoom::Zoom;

const FILE_NAME: &str = "settings.json";

/// Where a file that doesn't parse is kept, so a hand-edited file with a typo isn't lost.
const BAD_FILE_NAME: &str = "settings.json.bad";

/// The reader's choices, as the settings file holds them. A value the file leaves out takes its
/// default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// The program Edit opens a document in, as Choose editor... picked it; None for the text
    /// file program.
    pub editor: Option<PathBuf>,
    /// The document view's zoom level (ticket 16), as Zoom in, Zoom out and Reset zoom set it.
    pub zoom: Zoom,
    /// The size of the last window that closed, which a new window opens at (ticket 16); None
    /// until a window has closed. Kept as the window had it: a new window is fitted to its
    /// screen when it opens, since the screen may have changed since.
    pub window_size: Option<WindowSize>,
}

impl Settings {
    /// What of the settings the pages show, as every page gets it: in its URL when its window
    /// opens, and in a `settings` message whenever it changes (window design ticket 12). The
    /// editor stays in the app.
    pub fn page(&self) -> PageSettings {
        PageSettings { zoom: self.zoom }
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
}

impl PageSettings {
    /// The snapshot the contract samples name, the same in the page's URL and in the `settings`
    /// message. Off the defaults, so a page that ignored it would fail its test.
    #[cfg(test)]
    pub fn sample() -> Self {
        Self {
            zoom: Zoom::from(125),
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
            u16::from(store.get().page().zoom)
        };
        assert_eq!(zoom_in(&too_large), 300);
        assert_eq!(zoom_in(&too_small), 50);
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
