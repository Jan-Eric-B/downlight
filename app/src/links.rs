//! Links (ticket 11): following a link in a window's document. The core's link resolution decides
//! what a link does; here the app does it, and the page carries out what is left to it, such as
//! scrolling to a heading.

use std::path::{Path, PathBuf};
use std::{fmt, io};

use downlight_core::{LinkAction, LinkError, PathKind, resolve_link};
use tauri::{AppHandle, Manager as _, WebviewWindow};

use crate::documents::{DocumentAt, Documents, Showing, Target};
use crate::notice::{NamedFile, Notice};
use crate::platform;
use crate::window::{self, Shown};

/// Follows the link `target`, as written in the calling window's document as `showing`, and tells
/// the page what is left for it to do. With `new_window` (Ctrl+click), a Markdown document opens
/// in a new window instead of this one. A link in a document the window has left since does
/// nothing (ticket 26): the old document stays in view while the next one renders, and its link
/// would otherwise resolve against the new document's folder.
///
/// The page passes only the showing and the target: the document comes from the app's own state.
/// Resolving looks at the file system, and a network share connects only now, on a blocking
/// thread, so the window keeps answering.
#[tauri::command]
pub async fn follow_link(
    window: WebviewWindow,
    showing: Showing,
    target: String,
    new_window: bool,
) -> Result<Followed, FollowError> {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || follow(&app, &label, showing, &target, new_window))
        .await
        .map_err(|source| FollowError::Stopped { source })?
}

/// What the page does after a link was followed.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Followed {
    /// Scroll the shown document to the heading with this id, or to its top without one.
    ScrollTo { heading: Option<String> },
    /// Nothing is left for the page: the link showed a document, which its window's page asks for
    /// when told (`ask-for-document`), opened something else, came from a document the window has
    /// left, or does nothing.
    Nothing,
}

/// A sample of each answer [`follow_link`] gives, by name, and of its failure for a link to a
/// missing file, whose folder is there or gone too (window design ticket 11), which the page's
/// tests read (`app/src/contract.rs`, ticket 27).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new answer stops the build here: give it a sample below, then name it here.
    let _sampled = |followed: &Followed| match followed {
        Followed::ScrollTo { .. } | Followed::Nothing => {}
    };
    let heading = Some("install".to_owned());
    Ok(serde_json::json!({
        "scrollToHeading": serde_json::to_value(Followed::ScrollTo { heading })?,
        "scrollToTop": serde_json::to_value(Followed::ScrollTo { heading: None })?,
        "nothing": serde_json::to_value(Followed::Nothing)?,
        "notFound": serde_json::to_value(FollowError::NotFound(NamedFile::sample(true)))?,
        "notFoundInGoneFolder":
            serde_json::to_value(FollowError::NotFound(NamedFile::sample(false)))?,
    }))
}

fn follow(
    app: &AppHandle,
    label: &str,
    showing: Showing,
    target: &str,
    new_window: bool,
) -> Result<Followed, FollowError> {
    // A window that shows no document has no links, and one that has left the link's has none
    // of that document's.
    let Some(document) = app
        .state::<Documents>()
        .document_path_shown_as(label, showing)
    else {
        return Ok(Followed::Nothing);
    };
    match resolve_link(&document, target, path_kind)? {
        LinkAction::ShowDocument { path, heading } => {
            show_document(app, label, path, heading, new_window)
        }
        LinkAction::ScrollToHeading { heading } => Ok(Followed::ScrollTo { heading }),
        LinkAction::RevealFile(path) => match platform::reveal_file(&path) {
            Ok(()) => Ok(Followed::Nothing),
            Err(source) => Err(FollowError::Reveal { path, source }),
        },
        LinkAction::OpenFolder(path) => match platform::open_folder(&path) {
            Ok(()) => Ok(Followed::Nothing),
            Err(source) => Err(FollowError::OpenFolder { path, source }),
        },
        LinkAction::OpenUrl(url) => match platform::open_url(&url) {
            Ok(()) => Ok(Followed::Nothing),
            Err(source) => Err(FollowError::OpenUrl { url, source }),
        },
        LinkAction::Nothing => Ok(Followed::Nothing),
    }
}

/// Shows the linked document `path` in window `label`, or in a new window with `new_window`. A
/// document that is open already has its window brought to the front: this window's page scrolls
/// to the heading (or the top) on the link's result, and another window's page is told to scroll
/// by `Documents::give`, only if the link names a heading, so that otherwise it keeps its reading
/// position.
fn show_document(
    app: &AppHandle,
    label: &str,
    path: PathBuf,
    heading: Option<String>,
    new_window: bool,
) -> Result<Followed, FollowError> {
    let document = DocumentAt {
        path: &path,
        heading: heading.as_deref(),
    };
    let shown = window::show(app, document, destination(label, new_window))
        .map_err(|source| FollowError::Show { path, source })?;
    Ok(left_for_the_page(&shown, heading))
}

/// The window a link in window `label` shows a Markdown document in: this one, or a new one with
/// Ctrl+click.
fn destination(label: &str, new_window: bool) -> Target<'_> {
    if new_window {
        Target::NewWindowFrom(label)
    } else {
        Target::Window(label)
    }
}

/// What is left for the page after its link to a document at `heading` was `shown`: a link to the
/// document this window shows already scrolls to the heading, or else to the top, also with
/// Ctrl+click. `Documents::give` sends this window no `scroll-to-heading`, so the page scrolls
/// only on this.
fn left_for_the_page(shown: &Shown, heading: Option<String>) -> Followed {
    match shown {
        Shown::AlreadyHere => Followed::ScrollTo { heading },
        Shown::AlreadyElsewhere | Shown::Opened => Followed::Nothing,
    }
}

/// Whether `path` is a file or a folder, or None when nothing is there, for the core's link
/// resolution. Any other error, such as an offline share or a folder the reader may not open,
/// stays an error: the file may well exist, so the reader mustn't be told it doesn't.
fn path_kind(path: &Path) -> io::Result<Option<PathKind>> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(Some(PathKind::Folder)),
        Ok(_) => Ok(Some(PathKind::File)),
        Err(error) if platform::is_missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Why a link wasn't followed. The page shows it to the reader as a notice.
#[derive(Debug)]
pub enum FollowError {
    /// The link names a file or folder that doesn't exist; the notice opens its folder (window
    /// design ticket 11).
    NotFound(NamedFile),
    /// Whether the link's file or folder exists can't be told.
    Unreachable {
        path: PathBuf,
        source: io::Error,
    },
    Reveal {
        path: PathBuf,
        source: io::Error,
    },
    OpenFolder {
        path: PathBuf,
        source: io::Error,
    },
    OpenUrl {
        url: String,
        source: io::Error,
    },
    Show {
        path: PathBuf,
        source: tauri::Error,
    },
    /// The background task that follows the link panicked or was cancelled.
    Stopped {
        source: tauri::Error,
    },
}

impl From<LinkError> for FollowError {
    fn from(error: LinkError) -> Self {
        match error {
            // Made on the blocking thread that follows the link, as it looks at the folder.
            LinkError::NotFound { path } => Self::NotFound(NamedFile::missing(&path)),
            LinkError::Unreachable { path, source } => Self::Unreachable { path, source },
        }
    }
}

impl FollowError {
    fn notice(&self) -> Notice {
        let why = |source: &dyn fmt::Display| format!(": {source}");
        match self {
            Self::NotFound(file) => Notice::naming("Can't find ", file.clone(), "."),
            Self::Unreachable { path, source } => {
                Notice::naming("Can't reach ", NamedFile::at(path), why(source))
            }
            Self::Reveal { path, source } => {
                Notice::naming("Can't show ", NamedFile::at(path), reveal_failed(source))
            }
            Self::OpenFolder { path, source } => {
                Notice::naming("Can't open the folder ", NamedFile::at(path), why(source))
            }
            Self::OpenUrl { url, source } => format!("Can't open {url}: {source}").into(),
            Self::Show { path, source } => {
                Notice::naming("Can't show ", NamedFile::at(path), why(source))
            }
            Self::Stopped { source } => format!("Following the link stopped: {source}").into(),
        }
    }
}

/// What follows the file in the notice of a reveal that failed, the same for a link's reveal and
/// the title bar's.
pub fn reveal_failed(source: &io::Error) -> String {
    format!(" in Explorer: {source}")
}

impl fmt::Display for FollowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.notice().fmt(f)
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as a notice.
impl serde::Serialize for FollowError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.notice().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_kind_tells_files_folders_and_missing_apart() {
        let folder = std::env::temp_dir().join(format!("downlight-links-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("sub")).unwrap();
        std::fs::write(folder.join("notes.txt"), "Notes").unwrap();
        let kinds = [
            path_kind(&folder.join("notes.txt")),
            path_kind(&folder.join("sub")),
            path_kind(&folder.join("missing.md")),
        ];
        // Cleaned up before asserting, so a failing run leaves no folder behind.
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            kinds.map(Result::ok),
            [
                Some(Some(PathKind::File)),
                Some(Some(PathKind::Folder)),
                Some(None)
            ]
        );
    }

    /// The link target `missing.md` in a folder of its own, which is there unless `folder_gone`.
    fn missing_link_target(folder_gone: bool) -> (PathBuf, FollowError) {
        let folder = std::env::temp_dir().join(format!(
            "downlight-links-{}-{folder_gone}",
            std::process::id()
        ));
        std::fs::create_dir_all(&folder).unwrap();
        if folder_gone {
            std::fs::remove_dir_all(&folder).unwrap();
        }
        let error = FollowError::from(LinkError::NotFound {
            path: folder.join("missing.md"),
        });
        let _ = std::fs::remove_dir_all(&folder);
        (folder, error)
    }

    #[test]
    fn link_to_a_missing_file_names_it_and_its_folder_which_opens() {
        let (folder, error) = missing_link_target(false);
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "before": "Can't find ",
                "file": {
                    "name": "missing.md",
                    "folder": folder.display().to_string(),
                    "folderOpens": true,
                },
                "after": ".",
            })
        );
        assert_eq!(
            error.to_string(),
            format!("Can't find missing.md in {}.", folder.display())
        );
    }

    #[test]
    fn link_to_a_missing_file_whose_folder_is_gone_too_names_a_folder_that_does_not_open() {
        let (folder, error) = missing_link_target(true);
        assert_eq!(
            serde_json::to_value(&error).unwrap()["file"],
            serde_json::json!({
                "name": "missing.md",
                "folder": folder.display().to_string(),
                "folderOpens": false,
            })
        );
    }

    #[test]
    fn unreachable_file_message_names_the_file_and_why() {
        let error = FollowError::from(LinkError::Unreachable {
            path: PathBuf::from(r"\\server\share\a.md"),
            source: io::Error::other("The network path was not found."),
        });
        assert_eq!(
            error.to_string(),
            r"Can't reach a.md in \\server\share\: The network path was not found."
        );
        assert!(
            !serde_json::to_value(&error).unwrap()["file"]["folderOpens"]
                .as_bool()
                .unwrap()
        );
    }

    #[test]
    fn ctrl_click_shows_the_document_in_a_new_window() {
        assert!(matches!(
            destination("document-0", true),
            Target::NewWindowFrom("document-0")
        ));
        assert!(matches!(
            destination("document-0", false),
            Target::Window("document-0")
        ));
    }

    #[test]
    fn link_to_the_document_this_window_shows_scrolls_to_its_heading_or_the_top() {
        assert_eq!(
            left_for_the_page(&Shown::AlreadyHere, Some("install".to_owned())),
            Followed::ScrollTo {
                heading: Some("install".to_owned())
            }
        );
        assert_eq!(
            left_for_the_page(&Shown::AlreadyHere, None),
            Followed::ScrollTo { heading: None }
        );
    }

    #[test]
    fn link_to_a_document_shown_elsewhere_leaves_this_page_nothing_to_do() {
        // Another window scrolls on the app's word (`scroll-to-heading`), not this page's.
        let heading = || Some("install".to_owned());
        assert_eq!(
            left_for_the_page(&Shown::AlreadyElsewhere, heading()),
            Followed::Nothing
        );
        assert_eq!(
            left_for_the_page(&Shown::Opened, heading()),
            Followed::Nothing
        );
    }
}
