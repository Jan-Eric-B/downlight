//! Links (ticket 11): following a link in a window's document. The core's link resolution decides
//! what a link does; here the app does it, and the page carries out what is left to it, such as
//! scrolling to a heading.

use std::path::{Path, PathBuf};
use std::{fmt, io};

use downlight_core::{LinkAction, LinkError, PathKind, resolve_link};
use tauri::{Manager as _, WebviewWindow};

use crate::documents::{DocumentAt, Documents, Given, ShowError, Showing, Target};
use crate::notice::{NamedFile, Notice};
use crate::platform;

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
) -> Result<Followed, Notice> {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    let followed = tauri::async_runtime::spawn_blocking(move || {
        follow(
            &app.state::<Documents>(),
            &label,
            showing,
            &target,
            new_window,
        )
    })
    .await
    .map_err(|source| FollowError::Stopped { source })??;
    Ok(followed)
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
        "notFound": serde_json::to_value(Notice::from(FollowError::NotFound(NamedFile::sample(true))))?,
        "notFoundInGoneFolder":
            serde_json::to_value(Notice::from(FollowError::NotFound(NamedFile::sample(false))))?,
    }))
}

fn follow(
    documents: &Documents,
    label: &str,
    showing: Showing,
    target: &str,
    new_window: bool,
) -> Result<Followed, FollowError> {
    // A window that shows no document has no links, and one that has left the link's has none
    // of that document's.
    let Some(document) = documents.document_path_shown_as(label, showing) else {
        return Ok(Followed::Nothing);
    };
    match resolve_link(&document, target, path_kind)? {
        LinkAction::ShowDocument { path, heading } => {
            let document = DocumentAt {
                path: &path,
                heading: heading.as_deref(),
            };
            let target = if new_window {
                Target::NewWindowFrom(label)
            } else {
                Target::Window(label)
            };
            // A link to the document this window shows already scrolls to the heading, or else
            // to the top, also with Ctrl+click: `give` sends this window nothing, so the page
            // scrolls only on this. Any other window that shows the document has been told to
            // scroll by `give`, only if the link names a heading, so that otherwise it keeps its
            // reading position.
            match documents.give(document, target) {
                Ok(Given::AlreadyHere(_)) => Ok(Followed::ScrollTo { heading }),
                Ok(Given::NewWindow(_) | Given::Changed(_) | Given::AlreadyElsewhere(_)) => {
                    Ok(Followed::Nothing)
                }
                Err(source) => Err(FollowError::Show { path, source }),
            }
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
    /// The link names a Markdown document that isn't shown.
    Show {
        path: PathBuf,
        source: ShowError,
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

impl From<FollowError> for Notice {
    fn from(error: FollowError) -> Self {
        let why = |source: &dyn fmt::Display| format!(": {source}");
        match error {
            FollowError::NotFound(file) => Self::naming("Can't find ", file, "."),
            FollowError::Unreachable { path, source } => {
                Self::naming("Can't reach ", NamedFile::at(&path), why(&source))
            }
            FollowError::Reveal { path, source } => Self::reveal_failed(&path, &source),
            FollowError::OpenFolder { path, source } => {
                Self::naming("Can't open the folder ", NamedFile::at(&path), why(&source))
            }
            FollowError::OpenUrl { url, source } => format!("Can't open {url}: {source}").into(),
            FollowError::Show { path, source } => {
                Self::naming("Can't show ", NamedFile::at(&path), why(&source))
            }
            FollowError::Stopped { source } => {
                format!("Following the link stopped: {source}").into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::recorder::{ToWindow, recorded};
    use crate::message::Message;

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

    /// The notice for the link target `missing.md` in a folder of its own, which is there unless
    /// `folder_gone`.
    fn missing_link_target(folder_gone: bool) -> (PathBuf, Notice) {
        let folder = std::env::temp_dir().join(format!(
            "downlight-links-{}-{folder_gone}",
            std::process::id()
        ));
        std::fs::create_dir_all(&folder).unwrap();
        if folder_gone {
            std::fs::remove_dir_all(&folder).unwrap();
        }
        let error = Notice::from(FollowError::from(LinkError::NotFound {
            path: folder.join("missing.md"),
        }));
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
        let error = Notice::from(FollowError::from(LinkError::Unreachable {
            path: PathBuf::from(r"\\server\share\a.md"),
            source: io::Error::other("The network path was not found."),
        }));
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
    fn link_to_another_document_shows_it_in_this_window() {
        let (documents, recorder) = recorded();
        let (window, showing) = window_showing_notes(&documents);
        let followed = follow(&documents, &window, showing, "readme.md", false);
        assert_eq!(followed.ok(), Some(Followed::Nothing));
        assert_eq!(
            documents.document_path(&window),
            Some(fixtures().join("readme.md"))
        );
        assert_eq!(recorder.sent(), [(window, Message::AskForDocument)]);
    }

    #[test]
    fn ctrl_click_shows_the_document_in_a_new_window() {
        let (documents, recorder) = recorded();
        let (window, showing) = window_showing_notes(&documents);
        let followed = follow(&documents, &window, showing, "readme.md#install", true);
        assert_eq!(followed.ok(), Some(Followed::Nothing));
        assert_eq!(
            documents.document_path(&window),
            Some(fixtures().join("notes.md"))
        );
        assert!(
            matches!(recorder.done().as_slice(), [_, (new, ToWindow::Opened(_))] if *new != window),
            "{:?}",
            recorder.done()
        );
    }

    #[test]
    fn link_to_the_document_this_window_shows_scrolls_to_its_heading_or_the_top() {
        // Also with Ctrl+click: the document is shown in at most one window (ADR 0001).
        let (documents, recorder) = recorded();
        let (window, showing) = window_showing_notes(&documents);
        let follow_here =
            |target, new_window| follow(&documents, &window, showing, target, new_window).ok();
        let followed = [
            follow_here("notes.md#install", false),
            follow_here("notes.md", false),
            follow_here("notes.md#install", true),
        ];
        let scroll_to = |heading: Option<&str>| {
            Some(Followed::ScrollTo {
                heading: heading.map(str::to_owned),
            })
        };
        assert_eq!(
            followed,
            [
                scroll_to(Some("install")),
                scroll_to(None),
                scroll_to(Some("install"))
            ]
        );
        assert_eq!(recorder.sent(), []);
    }

    #[test]
    fn link_to_a_document_shown_elsewhere_brings_that_window_to_the_front_and_scrolls_it() {
        // That window scrolls on the app's word (`scroll-to-heading`), not this page's.
        let (documents, recorder) = recorded();
        let (window, showing) = window_showing_notes(&documents);
        let other = documents
            .give(
                DocumentAt::new(&fixtures().join("readme.md")),
                Target::NewWindow,
            )
            .unwrap();
        let Given::NewWindow(other) = other else {
            panic!("expected a new window, got {other:?}");
        };
        documents.page_asks(&other);
        let followed = follow(&documents, &window, showing, "readme.md#install", false);
        assert_eq!(followed.ok(), Some(Followed::Nothing));
        assert_eq!(
            recorder.sent(),
            [(
                other.clone(),
                Message::ScrollToHeading("install".to_owned())
            )]
        );
        assert_eq!(
            recorder.done().last(),
            Some(&(other, ToWindow::BroughtToFront))
        );
    }

    #[test]
    fn link_to_a_document_whose_window_cannot_open_names_the_document_and_why() {
        let (documents, recorder) = recorded();
        let (window, showing) = window_showing_notes(&documents);
        recorder.fail_window("document-1");
        let followed = follow(&documents, &window, showing, "readme.md", true);
        assert_eq!(
            followed.err().map(|error| Notice::from(error).to_string()),
            Some(format!(
                "Can't show readme.md in {}: its window didn't open: the window can't be changed",
                fixtures().display()
            ))
        );
        // Forgotten, so it opens anew.
        assert_eq!(documents.document_path("document-1"), None);
    }

    #[test]
    fn link_in_a_document_the_window_has_left_does_nothing() {
        let (documents, recorder) = recorded();
        let (window, left) = window_showing_notes(&documents);
        let readme = fixtures().join("readme.md");
        documents
            .give(DocumentAt::new(&readme), Target::Window(&window))
            .unwrap();
        let followed = follow(&documents, &window, left, "notes.md", false);
        assert_eq!(followed.ok(), Some(Followed::Nothing));
        assert_eq!(documents.document_path(&window), Some(readme));
        assert_eq!(recorder.sent(), [(window, Message::AskForDocument)]);
    }

    /// The core's committed fixture documents, among them `notes.md` and `readme.md`. Nothing
    /// writes or removes them while the tests run, so the windows' watches start on them.
    fn fixtures() -> PathBuf {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures");
        // Resolves the `..`, as a launch does, so the folder is named as a link names it.
        std::path::absolute(&folder).unwrap()
    }

    /// A new window that shows `notes.md`, and whose page has asked for it: its label, and the
    /// showing its links name.
    fn window_showing_notes(documents: &Documents) -> (String, Showing) {
        let given = documents
            .give(
                DocumentAt::new(&fixtures().join("notes.md")),
                Target::NewWindow,
            )
            .unwrap();
        let Given::NewWindow(window) = given else {
            panic!("expected a new window, got {given:?}");
        };
        let showing = documents.page_asks(&window).unwrap();
        (window, showing)
    }
}
