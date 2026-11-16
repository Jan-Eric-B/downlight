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
}

/// The settings for this process: loaded once at the start, changed through [`SettingsStore::update`],
/// which saves the whole file.
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

    /// The program Edit opens a document in, as the reader chose it.
    pub fn editor(&self) -> Option<PathBuf> {
        self.lock().editor.clone()
    }

    /// Changes the settings with `change` and saves the whole file. A change that can't be saved
    /// still holds until Downlight closes, and the error says why it wasn't saved.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<(), SettingsError> {
        // Saved under the lock, so two changes at once reach the file in the order they were made.
        let mut current = self.lock();
        change(&mut current);
        let file = self.file.as_deref().ok_or(SettingsError::NoConfigFolder)?;
        write(file, &current).map_err(|source| SettingsError::Write {
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
        assert_eq!(store.editor(), None);
        assert!(!folder.0.exists());
    }

    #[test]
    fn first_save_creates_the_file_and_its_folder() {
        let folder = Folder::new("first-save");
        let (store, _) = folder.load();
        store
            .update(|settings| settings.editor = Some(PathBuf::from(r"C:\Tools\edit.exe")))
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
            .update(|settings| settings.editor = Some(editor.clone()))
            .unwrap();
        let (restarted, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(restarted.editor(), Some(editor));
    }

    #[test]
    fn corrupt_file_loads_defaults_and_reports_why() {
        let folder = Folder::with_file("corrupt", "{ \"editor\": ");
        let (store, error) = folder.load();
        assert_eq!(store.editor(), None);
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
        store.update(|_| {}).unwrap();
        let (_, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
    }

    #[test]
    fn file_without_an_editor_loads_with_none() {
        let folder = Folder::with_file("without-editor", "{}");
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(store.editor(), None);
    }

    #[test]
    fn file_with_unknown_fields_keeps_known_values() {
        let folder = Folder::with_file(
            "unknown-fields",
            r#"{ "editor": "C:\\Tools\\edit.exe", "fromANewerVersion": 3 }"#,
        );
        let (store, error) = folder.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(store.editor(), Some(PathBuf::from(r"C:\Tools\edit.exe")));
    }

    #[test]
    fn failed_save_keeps_the_change_for_this_session() {
        let folder = Folder::new("failed-save");
        // A folder where the file should be, so writing it fails.
        std::fs::create_dir_all(folder.file()).unwrap();
        let (store, _) = folder.load();
        let editor = PathBuf::from(r"C:\Tools\edit.exe");
        let saved = store.update(|settings| settings.editor = Some(editor.clone()));
        assert!(
            matches!(saved, Err(SettingsError::Write { .. })),
            "{saved:?}"
        );
        assert_eq!(store.editor(), Some(editor));
    }

    #[test]
    fn without_a_config_folder_a_change_holds_and_says_it_was_not_saved() {
        let (store, error) = SettingsStore::load(None);
        assert!(error.is_none(), "{error:?}");
        let editor = PathBuf::from(r"C:\Tools\edit.exe");
        let saved = store.update(|settings| settings.editor = Some(editor.clone()));
        assert!(
            matches!(saved, Err(SettingsError::NoConfigFolder)),
            "{saved:?}"
        );
        assert_eq!(store.editor(), Some(editor));
    }
}
