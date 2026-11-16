//! Notices (window design ticket 11): what the reader is told when something fails, as one shape
//! for every error and notice the page shows. A notice that names a file names it by its name and
//! its folder, so the page shows the name in bold and the folder apart, never a whole path as
//! plain text. When the file is missing and its folder is there, the folder opens from the notice
//! ([`open_named_folder`]), and no other folder does: the page can't have a path of its own
//! choosing opened.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::documents::file_name_shown;
use crate::platform;

/// What the reader is told: plain text, or text before and after the file it names. Goes to the
/// page as a string or as `{ before, file, after }` (`Notice` in `page/src/app.ts`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum Notice {
    Text(String),
    NamingFile {
        before: String,
        file: NamedFile,
        after: String,
    },
}

impl Notice {
    /// `before`, the file, and `after`, as in "Can't find **a.md** in C:\docs."
    pub fn naming(before: &str, file: NamedFile, after: impl Into<String>) -> Self {
        Self::NamingFile {
            before: before.to_owned(),
            file,
            after: after.into(),
        }
    }
}

impl From<String> for Notice {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for Notice {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

/// The notice as plain text, as the page shows it without the bold and the link.
impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::NamingFile {
                before,
                file,
                after,
            } => write!(f, "{before}{file}{after}"),
        }
    }
}

/// A file or folder a notice names: its name, and the folder it is in, which the reader can open
/// from the notice when `folder_opens`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedFile {
    name: String,
    /// None for a path with no folder, such as a drive's root.
    folder: Option<String>,
    /// The file is missing and its folder is there, and the folder is one [`open_named_folder`]
    /// opens.
    folder_opens: bool,
}

impl NamedFile {
    /// The file or folder at `path`, which is there or may be, as an offline share's: its folder
    /// is plain text, since Explorer can show the file itself.
    pub fn at(path: &Path) -> Self {
        Self {
            name: file_name_shown(path),
            folder: folder_of(path).map(|folder| folder.display().to_string()),
            folder_opens: false,
        }
    }

    /// The file or folder at `path`, which is missing. Its folder opens from the notice when it is
    /// there, and is remembered as one the reader may open. Looks at the file system, so it is
    /// made where the error is, on a background thread, never while the error is sent.
    pub fn missing(path: &Path) -> Self {
        let mut file = Self::at(path);
        if let (Some(folder), Some(shown)) = (folder_of(path), &file.folder)
            && is_folder(folder)
        {
            named_folders().insert(shown.clone(), folder.to_owned());
            file.folder_opens = true;
        }
        file
    }

    /// The file the contract samples name, `a.md` in `C:\docs`, the same on every system the
    /// tests run on.
    #[cfg(test)]
    pub fn sample(folder_opens: bool) -> Self {
        Self {
            name: "a.md".to_owned(),
            folder: Some(r"C:\docs".to_owned()),
            folder_opens,
        }
    }
}

/// `a.md in C:\docs`, as the page shows it.
impl fmt::Display for NamedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.folder {
            Some(folder) => write!(f, "{} in {folder}", self.name),
            None => f.write_str(&self.name),
        }
    }
}

/// The folder `path` is in; a relative path's empty one counts as none.
fn folder_of(path: &Path) -> Option<&Path> {
    path.parent()
        .filter(|folder| !folder.as_os_str().is_empty())
}

fn is_folder(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Every folder a notice offered to open, by the text the page was given and sends back: the only
/// folders [`open_named_folder`] opens. Kept for the process's life, as they are few, one for each
/// missing file the reader was told of. Process-wide rather than the app's managed state, since
/// the errors that name them are made on background threads that hold no app handle (rendering
/// one is the render module's, which knows nothing of windows).
static NAMED_FOLDERS: Mutex<BTreeMap<String, PathBuf>> = Mutex::new(BTreeMap::new());

fn named_folders() -> std::sync::MutexGuard<'static, BTreeMap<String, PathBuf>> {
    // The map stays valid even if a thread panicked while holding the lock.
    NAMED_FOLDERS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A click on a notice's folder: opens the folder in the file manager, as a link's "open a
/// folder" action does.
///
/// The page passes the folder as the notice named it, and the app opens it only if a notice named
/// it and it is a folder now, so the page can't have any other path opened. Looking at it reads
/// the file system, and a network share connects only now, on a blocking thread, so the window
/// keeps answering.
#[tauri::command]
pub async fn open_named_folder(folder: String) -> Result<(), OpenFolderError> {
    tauri::async_runtime::spawn_blocking(move || open_named_with(&folder, platform::open_folder))
        .await
        .map_err(|source| OpenFolderError::Stopped { source })?
}

/// [`open_named_folder`] with the OS's open passed in, so a test can see what it is given without
/// an Explorer window opening.
fn open_named_with(
    folder: &str,
    open_folder: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), OpenFolderError> {
    let Some(path) = named_folders().get(folder).cloned() else {
        return Err(OpenFolderError::NotNamed {
            folder: folder.to_owned(),
        });
    };
    match std::fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => {}
        // A file put where the folder was isn't the folder.
        Ok(_) => return Err(OpenFolderError::NotFound(NamedFile::missing(&path))),
        Err(error) if platform::is_missing(&error) => {
            return Err(OpenFolderError::NotFound(NamedFile::missing(&path)));
        }
        Err(source) => return Err(OpenFolderError::Unreachable { path, source }),
    }
    open_folder(&path).map_err(|source| OpenFolderError::Open { path, source })
}

/// Why a notice's folder wasn't opened. The page shows it as a notice.
#[derive(Debug)]
pub enum OpenFolderError {
    /// No notice named the folder, so the page asked for one of its own choosing.
    NotNamed {
        folder: String,
    },
    /// The folder was deleted or moved since the notice named it.
    NotFound(NamedFile),
    /// Whether the folder is there can't be told, as on an offline share.
    Unreachable {
        path: PathBuf,
        source: io::Error,
    },
    Open {
        path: PathBuf,
        source: io::Error,
    },
    /// The background task that opens the folder panicked or was cancelled.
    Stopped {
        source: tauri::Error,
    },
}

impl OpenFolderError {
    fn notice(&self) -> Notice {
        match self {
            Self::NotNamed { folder } => {
                format!("Can't open {folder}: no notice offered to open it.").into()
            }
            Self::NotFound(folder) => Notice::naming("Can't find ", folder.clone(), "."),
            Self::Unreachable { path, source } => {
                Notice::naming("Can't reach ", NamedFile::at(path), format!(": {source}"))
            }
            Self::Open { path, source } => Notice::naming(
                "Can't open the folder ",
                NamedFile::at(path),
                format!(": {source}"),
            ),
            Self::Stopped { source } => format!("Opening the folder stopped: {source}").into(),
        }
    }
}

impl fmt::Display for OpenFolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.notice().fmt(f)
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as a notice.
impl serde::Serialize for OpenFolderError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.notice().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the temp folder, empty, for one test.
    fn temp_folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("downlight-notice-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn json(value: impl serde::Serialize) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn missing_file_in_a_folder_that_is_there_names_the_folder_to_open() {
        let folder = temp_folder("there");
        let file = NamedFile::missing(&folder.join("delete-me.md"));
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            json(file),
            serde_json::json!({
                "name": "delete-me.md",
                "folder": folder.display().to_string(),
                "folderOpens": true,
            })
        );
    }

    #[test]
    fn missing_file_whose_folder_is_gone_too_names_a_folder_that_does_not_open() {
        let folder = temp_folder("gone");
        std::fs::remove_dir_all(&folder).unwrap();
        let file = NamedFile::missing(&folder.join("delete-me.md"));
        assert_eq!(
            json(file),
            serde_json::json!({
                "name": "delete-me.md",
                "folder": folder.display().to_string(),
                "folderOpens": false,
            })
        );
    }

    #[test]
    fn file_that_is_there_names_a_folder_that_does_not_open() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let file = NamedFile::at(&manifest);
        assert!(!json(file)["folderOpens"].as_bool().unwrap());
    }

    #[test]
    fn drive_root_is_named_without_a_folder() {
        let root = Path::new(r"Q:\");
        let file = NamedFile::missing(root);
        assert_eq!(file.to_string(), root.display().to_string());
        assert_eq!(json(file)["folder"], serde_json::Value::Null);
    }

    #[test]
    fn notice_naming_a_file_goes_as_its_parts_and_reads_as_name_in_folder() {
        let notice = Notice::naming("Can't find ", NamedFile::sample(true), ".");
        assert_eq!(notice.to_string(), r"Can't find a.md in C:\docs.");
        assert_eq!(
            json(&notice),
            serde_json::json!({
                "before": "Can't find ",
                "file": { "name": "a.md", "folder": r"C:\docs", "folderOpens": true },
                "after": ".",
            })
        );
        assert_eq!(json(Notice::from("Can't watch")), "Can't watch");
    }

    /// Opens `folder` as the page asks, with the OS's open recorded rather than run.
    fn open(folder: &str) -> (Result<(), OpenFolderError>, Vec<PathBuf>) {
        let mut opened = Vec::new();
        let result = open_named_with(folder, |path| {
            opened.push(path.to_owned());
            Ok(())
        });
        (result, opened)
    }

    #[test]
    fn folder_a_notice_named_opens() {
        let folder = temp_folder("opens");
        NamedFile::missing(&folder.join("delete-me.md"));
        let (result, opened) = open(&folder.display().to_string());
        std::fs::remove_dir_all(&folder).unwrap();
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(opened, [folder]);
    }

    #[test]
    fn folder_no_notice_named_is_refused_though_it_is_there() {
        // A folder of its own: the named folders are the process's, so another test may well
        // have named a folder such as the crate's own.
        let there = temp_folder("never-named");
        let folder = there.display().to_string();
        let (result, opened) = open(&folder);
        std::fs::remove_dir_all(&there).unwrap();
        assert!(
            matches!(&result, Err(OpenFolderError::NotNamed { folder: refused }) if *refused == folder),
            "{result:?}"
        );
        assert_eq!(opened, Vec::<PathBuf>::new());
    }

    #[test]
    fn named_folder_deleted_since_is_refused() {
        let folder = temp_folder("deleted");
        NamedFile::missing(&folder.join("delete-me.md"));
        std::fs::remove_dir_all(&folder).unwrap();
        let (result, opened) = open(&folder.display().to_string());
        assert!(
            matches!(&result, Err(OpenFolderError::NotFound(_))),
            "{result:?}"
        );
        assert_eq!(opened, Vec::<PathBuf>::new());
    }

    #[test]
    fn file_put_where_a_named_folder_was_is_refused() {
        let folder = temp_folder("replaced");
        NamedFile::missing(&folder.join("delete-me.md"));
        std::fs::remove_dir_all(&folder).unwrap();
        std::fs::write(&folder, "not a folder").unwrap();
        let (result, opened) = open(&folder.display().to_string());
        std::fs::remove_file(&folder).unwrap();
        assert!(
            matches!(&result, Err(OpenFolderError::NotFound(_))),
            "{result:?}"
        );
        assert_eq!(opened, Vec::<PathBuf>::new());
    }
}
