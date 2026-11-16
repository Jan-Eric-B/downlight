//! Notices (window design ticket 11): what the reader is told when something fails, as one shape
//! for every error and notice the page shows. A notice that names a file names it by its name and
//! its folder, so the page shows the name in bold and the folder apart, never a whole path as
//! plain text. When the file is missing and its folder is there, the folder opens from the notice
//! ([`open_named_folder`]), and no other folder does: the page can't have a path of its own
//! choosing opened.
//!
//! Each feature's error type turns into a `Notice` (`From<...> for Notice`), and a command that
//! fails rejects with the notice, so how an error reads is written once, there.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::documents::file_name_shown;
use crate::platform;

/// What the reader is told: plain text, or text before and after the file it names. Goes to the
/// page as a string or as `{ before, file, after }` (`Notice` in `page/src/app.ts`), as a
/// message, in a command's rejection, or in `render_document`'s answer.
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

    /// A reveal of the file at `path` that failed, read the same for a link's reveal and the
    /// title bar's.
    pub fn reveal_failed(path: &Path, source: &io::Error) -> Self {
        Self::naming(
            "Can't show ",
            NamedFile::at(path),
            format!(" in Explorer: {source}"),
        )
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
    /// The file is missing and its folder is there, and the folder was offered to open
    /// ([`NamedFolders::offer`]), which only sets this.
    folder_opens: bool,
}

impl NamedFile {
    /// The file or folder at `path`, which is there or may be, as an offline share's: its folder
    /// is plain text, since Explorer can show the file itself.
    pub fn at(path: &Path) -> Self {
        Self {
            name: file_name_shown(path),
            folder: folder_of(path).map(folder_shown),
            folder_opens: false,
        }
    }

    /// The file or folder at `path`, which is missing. Its folder is offered to the page to open
    /// from the notice if it is there ([`NamedFolders::offer`]), and only then opens. Looks at the
    /// file system, so it is made where the error is, on a background thread, never while the
    /// error is sent.
    pub fn missing(path: &Path) -> Self {
        Self::missing_offered_by(path, &OFFERED_FOLDERS)
    }

    /// [`NamedFile::missing`] with the folders it is offered in, so a test has its own.
    fn missing_offered_by(path: &Path, folders: &NamedFolders) -> Self {
        let mut file = Self::at(path);
        file.folder_opens = folder_of(path).is_some_and(|folder| folders.offer(folder));
        file
    }

    /// The file the contract samples name, `a.md` in `C:\docs`, the same on every system the
    /// tests run on. Its folder is offered nowhere, whatever `folder_opens` says: the samples are
    /// only shapes, and the page's tests answer `open_named_folder` with their fake app.
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

/// A folder as a notice names it: the text the page shows, and sends back to have it opened.
fn folder_shown(folder: &Path) -> String {
    folder.display().to_string()
}

fn is_folder(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Every folder a notice offered to open, by the text the page was given and sends back: the only
/// folders [`NamedFolders::open`] opens. Kept for the process's life, as they are few, one for
/// each missing file the reader was told of, so never pruned.
struct NamedFolders(Mutex<BTreeMap<String, PathBuf>>);

/// The folders offered to every window's page. Process-wide rather than the app's managed state,
/// since the errors that name them are made on background threads that hold no app handle
/// (rendering one is the render module's, which knows nothing of windows).
static OFFERED_FOLDERS: NamedFolders = NamedFolders::new();

impl NamedFolders {
    const fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    /// Offers `folder` to the page to open from a notice, if it is a folder now, and says whether
    /// it did: the one way a folder becomes one the page may have opened.
    fn offer(&self, folder: &Path) -> bool {
        let offered = is_folder(folder);
        if offered {
            self.lock().insert(folder_shown(folder), folder.to_owned());
        }
        offered
    }

    /// Opens `folder`, as a notice named it, in the file manager with the call behind a link's
    /// "open a folder" action, only if it was offered and is a folder now.
    fn open(&self, folder: &str) -> Result<(), OpenFolderError> {
        self.open_with(folder, platform::open_folder)
    }

    /// [`NamedFolders::open`] with the OS's open passed in, so a test can see what it is given
    /// without an Explorer window opening.
    fn open_with(
        &self,
        folder: &str,
        open_folder: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), OpenFolderError> {
        let Some(path) = self.lock().get(folder).cloned() else {
            return Err(OpenFolderError::NotNamed {
                folder: folder.to_owned(),
            });
        };
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {}
            // A file put where the folder was isn't the folder.
            Ok(_) => return Err(self.gone(&path)),
            Err(error) if platform::is_missing(&error) => return Err(self.gone(&path)),
            Err(source) => return Err(OpenFolderError::Unreachable { path, source }),
        }
        open_folder(&path).map_err(|source| OpenFolderError::Open { path, source })
    }

    /// The folder at `path`, offered once, is gone now; the notice that says so offers the folder
    /// it was in.
    fn gone(&self, path: &Path) -> OpenFolderError {
        OpenFolderError::NotFound(NamedFile::missing_offered_by(path, self))
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, PathBuf>> {
        // The map stays valid even if a thread panicked while holding the lock.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A click on a notice's folder: opens the folder in the file manager, as a link's "open a
/// folder" action does.
///
/// The page passes the folder as the notice named it, and the app opens it only if a notice
/// offered it and it is a folder now, so the page can't have any other path opened. Looking at it
/// reads the file system, and a network share connects only now, on a blocking thread, so the
/// window keeps answering.
#[tauri::command]
pub async fn open_named_folder(folder: String) -> Result<(), Notice> {
    tauri::async_runtime::spawn_blocking(move || OFFERED_FOLDERS.open(&folder))
        .await
        .map_err(|source| OpenFolderError::Stopped { source })??;
    Ok(())
}

/// A sample of `open_named_folder`'s failure for a folder gone since its notice offered it, which
/// the page's tests show (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "notFound": serde_json::to_value(Notice::from(OpenFolderError::NotFound(
            NamedFile::sample(true)
        )))?,
    }))
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

impl From<OpenFolderError> for Notice {
    fn from(error: OpenFolderError) -> Self {
        match error {
            OpenFolderError::NotNamed { folder } => {
                format!("Can't open {folder}: no notice offered to open it.").into()
            }
            OpenFolderError::NotFound(folder) => Self::naming("Can't find ", folder, "."),
            OpenFolderError::Unreachable { path, source } => {
                Self::naming("Can't reach ", NamedFile::at(&path), format!(": {source}"))
            }
            OpenFolderError::Open { path, source } => Self::naming(
                "Can't open the folder ",
                NamedFile::at(&path),
                format!(": {source}"),
            ),
            OpenFolderError::Stopped { source } => {
                format!("Opening the folder stopped: {source}").into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crate's own folder, which is there and which no test changes.
    fn crate_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// A folder of its own under the temp folder, empty, for a test that deletes it.
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

    /// Opens `folder` as the page asks, with the OS's open recorded rather than run.
    fn open(folders: &NamedFolders, folder: &str) -> (Result<(), OpenFolderError>, Vec<PathBuf>) {
        let mut opened = Vec::new();
        let result = folders.open_with(folder, |path| {
            opened.push(path.to_owned());
            Ok(())
        });
        (result, opened)
    }

    #[test]
    fn missing_file_in_a_folder_that_is_there_names_the_folder_to_open() {
        let folder = crate_folder();
        let file = NamedFile::missing_offered_by(&folder.join("missing.md"), &NamedFolders::new());
        assert_eq!(
            json(file),
            serde_json::json!({
                "name": "missing.md",
                "folder": folder.display().to_string(),
                "folderOpens": true,
            })
        );
    }

    #[test]
    fn missing_file_whose_folder_is_gone_too_names_a_folder_that_does_not_open() {
        let folders = NamedFolders::new();
        let folder = std::env::temp_dir().join(format!(
            "downlight-notice-{}-never-made",
            std::process::id()
        ));
        let file = NamedFile::missing_offered_by(&folder.join("missing.md"), &folders);
        assert_eq!(
            json(file),
            serde_json::json!({
                "name": "missing.md",
                "folder": folder.display().to_string(),
                "folderOpens": false,
            })
        );
        // Not offered either, so the folder can't be opened should it be made later.
        let (result, _) = open(&folders, &folder.display().to_string());
        assert!(
            matches!(&result, Err(OpenFolderError::NotNamed { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn file_that_is_there_names_a_folder_that_does_not_open() {
        let file = NamedFile::at(&crate_folder().join("Cargo.toml"));
        assert!(!json(file)["folderOpens"].as_bool().unwrap());
    }

    #[test]
    fn drive_root_is_named_without_a_folder() {
        let root = Path::new(r"Q:\");
        let file = NamedFile::missing_offered_by(root, &NamedFolders::new());
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

    #[test]
    fn folder_a_notice_offered_opens() {
        let folders = NamedFolders::new();
        let folder = crate_folder();
        NamedFile::missing_offered_by(&folder.join("missing.md"), &folders);
        let (result, opened) = open(&folders, &folder.display().to_string());
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(opened, [folder]);
    }

    #[test]
    fn folder_no_notice_offered_is_refused_though_it_is_there() {
        let folder = crate_folder().display().to_string();
        let (result, opened) = open(&NamedFolders::new(), &folder);
        assert!(
            matches!(&result, Err(OpenFolderError::NotNamed { folder: refused }) if *refused == folder),
            "{result:?}"
        );
        assert_eq!(opened, Vec::<PathBuf>::new());
    }

    #[test]
    fn offered_folder_deleted_since_is_refused() {
        let folders = NamedFolders::new();
        let folder = temp_folder("deleted");
        NamedFile::missing_offered_by(&folder.join("delete-me.md"), &folders);
        std::fs::remove_dir_all(&folder).unwrap();
        let (result, opened) = open(&folders, &folder.display().to_string());
        assert!(
            matches!(&result, Err(OpenFolderError::NotFound(_))),
            "{result:?}"
        );
        assert_eq!(opened, Vec::<PathBuf>::new());
    }

    #[test]
    fn file_put_where_an_offered_folder_was_is_refused() {
        let folders = NamedFolders::new();
        let folder = temp_folder("replaced");
        NamedFile::missing_offered_by(&folder.join("delete-me.md"), &folders);
        std::fs::remove_dir_all(&folder).unwrap();
        std::fs::write(&folder, "not a folder").unwrap();
        let (result, opened) = open(&folders, &folder.display().to_string());
        std::fs::remove_file(&folder).unwrap();
        assert!(
            matches!(&result, Err(OpenFolderError::NotFound(_))),
            "{result:?}"
        );
        assert_eq!(opened, Vec::<PathBuf>::new());
    }
}
