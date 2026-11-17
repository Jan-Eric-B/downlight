//! The hand-over of a window's document to its page (ticket 23): the page asks for its document
//! with the showing its view shows ([`render_document`]), and gets the render started ahead or a
//! new read of the file, its file's name and path, a notice held until it asked, its steps, and
//! whether to show the document, return to it or reload it ([`Place`]). Kept apart from the
//! table, since this is the contract with the page: its answer's shape is a contract sample
//! ([`contract_samples`]) that the page's tests read.

use std::path::Path;

use downlight_core::RenderedDocument;
use tauri::{State, WebviewWindow};

use super::history::Steps;
use super::{Arrival, Documents, Showing};
use crate::notice::{Notice, file_name_shown};
use crate::render::{self, DocumentError, RenderTask};
use crate::timing::Timing;

/// What a window's page gets when it asks for its document ([`render_document`]).
pub(super) struct Handover {
    /// The document's read and render.
    pub(super) render: RenderTask,
    /// This time the window was given the document, for the page to send with its next request.
    pub(super) showing: Showing,
    /// The document's file, for the title bar.
    pub(super) file: DocumentFile,
    /// What the page does with the render.
    pub(super) place: Place,
    /// A notice about the document held until the page first asked for it.
    pub(super) notice: Option<Notice>,
    /// Whether the window has a document to go back to, and one to go forward to.
    pub(super) steps: Steps,
}

/// The file of the document a window shows, as the page's title bar shows it (window design
/// ticket 04): its name, and its full path as the name's tooltip. Sent with every render rather
/// than worked out by the page, which would have to know each system's paths.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct DocumentFile {
    pub(super) name: String,
    pub(super) path: String,
}

impl DocumentFile {
    /// The file at `path`, as the launch, the dialog, the drop or the link named it.
    fn of(path: &Path) -> Self {
        Self {
            name: file_name_shown(path),
            path: path.display().to_string(),
        }
    }
}

/// What a page does with the render it asked for, decided by the showing its view shows rather
/// than by which message made it ask, since a `reload` sent for the document a window has just
/// left can reach the page after `document-shown` (ticket 23).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "place", rename_all = "camelCase")]
pub(super) enum Place {
    /// The page's view shows another showing, or none: show the document at this heading, from a
    /// link like `guide.md#install` (ticket 11), or at its top without one.
    Show { heading: Option<String> },
    /// The page's view shows this showing already: show the new render at the reading position.
    Reload,
    /// The page's view shows another showing, and Back or Forward gave the window this one back
    /// (ticket 12): show the document where the page's view was when it left this showing, which
    /// only the page knows.
    Return,
}

impl Place {
    /// What the timing switch reports the render as: by the place the page gets rather than by
    /// why the render started, so a reload by hand, which reads the file on the page's request,
    /// is reported as a reload too (ticket 26). `npm run measure` reads both. A return reads and
    /// renders the document as a show does.
    pub(super) fn reported_as(&self) -> &'static str {
        match self {
            Self::Show { .. } | Self::Return => "read-and-render",
            Self::Reload => "reload",
        }
    }
}

impl Documents {
    /// Hands `window`'s document over to its page, which has asked for it while its view shows
    /// `page_shows` (None while it shows no document), or gives None while the window shows none:
    /// the render started when the window was given the document or its file changed, if the page
    /// hasn't taken it yet, and a new read of its file otherwise, as for F5. The page shows the
    /// document when its view shows another showing, returns to where it was in it when Back or
    /// Forward gave it back, and reloads it when its view shows this one. A notice held until the
    /// page asked goes with the answer rather than as a message, since the page's listener for
    /// notices may not be registered yet (ticket 26).
    pub(super) fn handover(&self, window: &str, page_shows: Option<Showing>) -> Option<Handover> {
        self.page_hears_settings(window);
        let mut table = self.lock();
        let record = table.windows.get_mut(window)?;
        let steps = record.history.steps();
        let shown = record.shown.as_mut()?;
        shown.page_asked = true;
        let place = if page_shows == Some(shown.showing) {
            Place::Reload
        } else {
            match &shown.arrival {
                Arrival::Given { heading } => Place::Show {
                    heading: heading.clone(),
                },
                Arrival::Stepped => Place::Return,
            }
        };
        Some(Handover {
            render: shown
                .render_ahead
                .take()
                .unwrap_or_else(|| render::start(&shown.path)),
            showing: shown.showing,
            file: DocumentFile::of(&shown.path),
            place,
            notice: shown.held_notice.take(),
            steps,
        })
    }

    /// What `window`'s page asking for its document does, for the tests of other modules, which
    /// have no webview to call [`render_document`]: hands the document over, and gives the
    /// showing the page then sends with a link (None while the window shows no document).
    #[cfg(test)]
    pub fn page_asks(&self, window: &str) -> Option<Showing> {
        self.handover(window, None).map(|handover| handover.showing)
    }
}

/// Reads and renders the calling window's document, and says whether the page shows it or reloads
/// it, or gives None while the window shows none.
///
/// The page passes only the showing its view shows, or None: the path comes from the app's own
/// state, so the page can't ask for other files. The work runs on a blocking thread, off the main
/// thread, so the window keeps answering while a large document renders. The first request after
/// the window was given its document gets the render started then ([`Documents::handover`]).
///
/// Never fails: a read that fails goes in the answer. The `Result` is Tauri's, which requires one
/// of an async command that borrows its arguments, as `State` does.
#[tauri::command]
pub async fn render_document(
    window: WebviewWindow,
    showing: Option<Showing>,
    documents: State<'_, Documents>,
    timing: State<'_, Timing>,
) -> Result<Option<DocumentToShow>, ()> {
    let Some(handover) = documents.handover(window.label(), showing) else {
        return Ok(None);
    };
    let content = match handover.render.await {
        Ok(Ok((document, stages))) => {
            timing.report(window.label(), handover.place.reported_as(), &stages);
            Content::Document(document)
        }
        Ok(Err(error)) => Content::Error(error.into()),
        Err(source) => Content::Error(DocumentError::ReadAndRenderStopped { source }.into()),
    };
    Ok(Some(DocumentToShow {
        showing: handover.showing,
        file: handover.file,
        place: handover.place,
        content,
        notice: handover.notice,
        steps: handover.steps,
    }))
}

/// What the page gets when it asks for its document: the showing the window is at, the
/// document's file, what to do with the render (show it at a heading or the top, return to where
/// the page was in it, or reload it at the reading position), the rendered document or why it
/// can't be read, a notice held until the page first asked, or null, and whether the window has
/// a document to go back or forward to. A read that fails is part of the answer, since only the
/// answer says whether the page keeps its last render, on a reload, or shows why, on a show
/// (ticket 23).
#[derive(serde::Serialize)]
pub struct DocumentToShow {
    showing: Showing,
    file: DocumentFile,
    #[serde(flatten)]
    place: Place,
    #[serde(flatten)]
    content: Content,
    notice: Option<Notice>,
    steps: Steps,
}

/// The rendered document, which goes as its HTML, or why it can't be read.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum Content {
    Document(RenderedDocument),
    Error(Notice),
}

/// A sample of each answer [`render_document`] gives, by name, which the page's tests read
/// (`app/src/contract.rs`, ticket 27): the rendered document goes as its HTML, the heading only
/// with a show, a read that fails as why, the document's file with each, a held notice with the
/// first (ticket 26), and whether there is somewhere to go back or forward to with each, which is
/// nowhere but after a return (ticket 12).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    use crate::disk::Disk;
    use crate::notice::NamedFile;
    use std::io;

    // A new place or content stops the build here: give it a sample below, then name it here.
    let _sampled = |answer: &Option<DocumentToShow>| match answer {
        None
        | Some(DocumentToShow {
            place: Place::Show { .. } | Place::Reload | Place::Return,
            content: Content::Document(_) | Content::Error(_),
            notice: None | Some(_),
            ..
        }) => {}
    };
    let answer_with = |place: Place, content: Content, notice: Option<Notice>| {
        let returned = place == Place::Return;
        serde_json::to_value(Some(DocumentToShow {
            showing: Showing::sample(),
            // Written out rather than `DocumentFile::of`, so the samples are the same on every
            // system the tests run on.
            file: DocumentFile {
                name: "a.md".to_owned(),
                path: r"C:\docs\a.md".to_owned(),
            },
            place,
            content,
            notice,
            steps: Steps {
                back: returned,
                forward: returned,
            },
        }))
    };
    let answer = |place: Place, content: Content| answer_with(place, content, None);
    let held_notice = Notice::naming(
        "Can't watch ",
        NamedFile::sample(false),
        " for changes, so only F5 reloads it: Access is denied.",
    );
    let rendered = || {
        Content::Document(downlight_core::render(
            b"# Install\n\nRun the installer.\n",
            Path::new("a.md"),
            "",
            Disk,
        ))
    };
    // A missing file whose folder is there, so the notice opens the folder (window design ticket
    // 11).
    let unread = || {
        Content::Error(
            DocumentError::Read {
                file: NamedFile::sample(true),
                source: io::ErrorKind::NotFound.into(),
            }
            .into(),
        )
    };
    let at_top = || Place::Show { heading: None };
    Ok(serde_json::json!({
        "showAtHeading": answer(Place::Show { heading: Some("install".to_owned()) }, rendered())?,
        "showAtTop": answer(at_top(), rendered())?,
        "showWithHeldNotice": answer_with(at_top(), rendered(), Some(held_notice))?,
        "reload": answer(Place::Reload, rendered())?,
        "return": answer(Place::Return, rendered())?,
        "unreadShow": answer(at_top(), unread())?,
        "unreadReturn": answer(Place::Return, unread())?,
        "unreadReload": answer(Place::Reload, unread())?,
        "noDocument": serde_json::to_value(None::<DocumentToShow>)?,
    }))
}
