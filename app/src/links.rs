//! Links (ticket 11): following a link in a window's document. The core's link resolution decides
//! what a link does; here the app does it, and the page carries out what is left to it, such as
//! scrolling to a heading.

use std::path::{Path, PathBuf};
use std::{fmt, io};

use downlight_core::{LinkAction, LinkError, PathKind, resolve_link};
use tauri::{AppHandle, Emitter as _, Manager as _, WebviewWindow};

use crate::platform;
use crate::window::{self, DocumentAt, OpenWindows, Shown, Target};

/// The event that tells a window's page to scroll to a heading: a link in another window named
/// a heading in this window's document.
const SCROLL_TO_HEADING: &str = "scroll-to-heading";

/// Follows the link `target`, as written in the calling window's document, and tells the page
/// what is left for it to do. With `new_window` (Ctrl+click), a Markdown document opens in a new
/// window instead of this one.
///
/// The page passes only the target: the document comes from the app's own state. Resolving looks
/// at the file system, and a network share connects only now, on a blocking thread, so the window
/// keeps answering.
#[tauri::command]
pub async fn follow_link(
    window: WebviewWindow,
    target: String,
    new_window: bool,
) -> Result<Followed, FollowError> {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || follow(&app, &label, &target, new_window))
        .await
        .map_err(|source| FollowError::Stopped { source })?
}

/// What the page does after a link was followed.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Followed {
    /// Scroll the shown document to the heading with this id, or to its top without one.
    ScrollTo { heading: Option<String> },
    /// Nothing is left for the page: the link showed a document, which its window's page renders
    /// when told (`document-shown`), opened something else, or does nothing.
    Nothing,
}

fn follow(
    app: &AppHandle,
    label: &str,
    target: &str,
    new_window: bool,
) -> Result<Followed, FollowError> {
    // A window that shows no document has no links.
    let Some(document) = app.state::<OpenWindows>().document_path(label) else {
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
/// document that is open already has its window brought to the front: this window scrolls to the
/// heading (or the top), and another window scrolls only if the link names a heading, so that
/// otherwise it keeps its reading position (`OpenWindows::claim`).
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
    let shown = window::show(app, Some(document), destination(label, new_window))
        .map_err(|source| FollowError::Show { path, source })?;
    if let Shown::AlreadyOpen {
        window,
        scroll_to: Some(scroll_to),
    } = &shown
        && window != label
    {
        // The only way to fail is that the window has closed, and then there is nothing to
        // scroll.
        let _ = app.emit_to(window.as_str(), SCROLL_TO_HEADING, scroll_to);
    }
    Ok(left_for_the_page(label, &shown, heading))
}

/// The window a link in window `label` shows a Markdown document in: this one, or a new one with
/// Ctrl+click.
fn destination(label: &str, new_window: bool) -> Target<'_> {
    if new_window {
        Target::NewWindow
    } else {
        Target::Window(label)
    }
}

/// What is left for the page in window `label` after its link to a document at `heading` was
/// `shown`: a link to the document this window shows already scrolls to the heading, or else to
/// the top.
fn left_for_the_page(label: &str, shown: &Shown, heading: Option<String>) -> Followed {
    match shown {
        Shown::AlreadyOpen { window, .. } if window == label => Followed::ScrollTo { heading },
        Shown::AlreadyOpen { .. } | Shown::Given => Followed::Nothing,
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

/// Why a link wasn't followed. The page shows the message to the reader.
#[derive(Debug)]
pub enum FollowError {
    /// The link names a file or folder that doesn't exist.
    NotFound {
        path: PathBuf,
    },
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
            LinkError::NotFound { path } => Self::NotFound { path },
            LinkError::Unreachable { path, source } => Self::Unreachable { path, source },
        }
    }
}

impl fmt::Display for FollowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "Can't find {}.", path.display()),
            Self::Unreachable { path, source } => {
                write!(f, "Can't reach {}: {source}", path.display())
            }
            Self::Reveal { path, source } => {
                write!(f, "Can't show {} in its folder: {source}", path.display())
            }
            Self::OpenFolder { path, source } => {
                write!(f, "Can't open the folder {}: {source}", path.display())
            }
            Self::OpenUrl { url, source } => write!(f, "Can't open {url}: {source}"),
            Self::Show { path, source } => write!(f, "Can't show {}: {source}", path.display()),
            Self::Stopped { source } => write!(f, "Following the link stopped: {source}"),
        }
    }
}

// Tauri sends a command's error to the page serialized; the page shows it as text.
impl serde::Serialize for FollowError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
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

    #[test]
    fn missing_file_message_names_the_file() {
        let error = FollowError::from(LinkError::NotFound {
            path: PathBuf::from(r"C:\docs\x.md"),
        });
        assert_eq!(error.to_string(), r"Can't find C:\docs\x.md.");
    }

    #[test]
    fn unreachable_file_message_names_the_file_and_why() {
        let error = FollowError::from(LinkError::Unreachable {
            path: PathBuf::from(r"\\server\share\a.md"),
            source: io::Error::other("The network path was not found."),
        });
        assert_eq!(
            error.to_string(),
            r"Can't reach \\server\share\a.md: The network path was not found."
        );
    }

    #[test]
    fn ctrl_click_shows_the_document_in_a_new_window() {
        assert!(matches!(destination("document-0", true), Target::NewWindow));
        assert!(matches!(
            destination("document-0", false),
            Target::Window("document-0")
        ));
    }

    #[test]
    fn link_to_the_document_this_window_shows_scrolls_to_its_heading_or_the_top() {
        let here = |scroll_to: Option<&str>| Shown::AlreadyOpen {
            window: "document-0".to_owned(),
            scroll_to: scroll_to.map(str::to_owned),
        };
        assert_eq!(
            left_for_the_page(
                "document-0",
                &here(Some("install")),
                Some("install".to_owned())
            ),
            Followed::ScrollTo {
                heading: Some("install".to_owned())
            }
        );
        assert_eq!(
            left_for_the_page("document-0", &here(None), None),
            Followed::ScrollTo { heading: None }
        );
    }

    #[test]
    fn link_to_a_document_shown_elsewhere_leaves_this_page_nothing_to_do() {
        // Another window scrolls on the app's word (`scroll-to-heading`), not this page's.
        let elsewhere = Shown::AlreadyOpen {
            window: "document-1".to_owned(),
            scroll_to: Some("install".to_owned()),
        };
        let heading = || Some("install".to_owned());
        assert_eq!(
            left_for_the_page("document-0", &elsewhere, heading()),
            Followed::Nothing
        );
        assert_eq!(
            left_for_the_page("document-0", &Shown::Given, heading()),
            Followed::Nothing
        );
    }

    #[test]
    fn followed_serializes_as_the_page_reads_it() {
        // `Followed` in page/src/links.ts.
        let scroll_to = Followed::ScrollTo {
            heading: Some("install".to_owned()),
        };
        assert_eq!(
            serde_json::to_value(scroll_to).unwrap(),
            serde_json::json!({ "kind": "scrollTo", "heading": "install" })
        );
        assert_eq!(
            serde_json::to_value(Followed::ScrollTo { heading: None }).unwrap(),
            serde_json::json!({ "kind": "scrollTo", "heading": null })
        );
        assert_eq!(
            serde_json::to_value(Followed::Nothing).unwrap(),
            serde_json::json!({ "kind": "nothing" })
        );
    }
}
