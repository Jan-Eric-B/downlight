//! The hand-over of a window's document to its page (ticket 23): the page asks for its document
//! with the showing its view shows ([`render_document`]), and gets the render started ahead or a
//! new read of the file, its file's name and path, a notice held until it asked, its steps, and
//! whether to show the document, return to it or reload it ([`Place`]). Kept apart from the
//! table, since this is the contract with the page: its answer's shape is a contract sample
//! ([`contract_samples`]) that the page's tests read.

use std::path::{Path, PathBuf};

use downlight_core::RenderedDocument;
use tauri::{State, WebviewWindow};

use super::history::Steps;
use super::shown_bytes::{Handed, Request};
use super::{Arrival, Documents, Showing};
use crate::notice::{Notice, file_name_shown};
use crate::render::{self, DocumentError, RenderTask};
use crate::timing::Timing;

/// What a window's page gets when it asks for its document ([`render_document`]).
pub(super) struct Handover {
    /// The document's read and render.
    pub(super) render: RenderTask,
    /// Which of the page's requests this answers, for keeping its render's bytes
    /// ([`Documents::rendered`]).
    pub(super) request: Request,
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
        let request = table.next_request();
        let record = table.windows.get_mut(window)?;
        let steps = record.now.history.steps();
        let Some(shown) = record.now.shown.as_mut() else {
            // The page shows the empty window next, which has nothing to copy.
            record.shows_none();
            return None;
        };
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
        let render = shown
            .render_ahead
            .take()
            .unwrap_or_else(|| render::start(&shown.path).0);
        // The page waits only for its latest request's render, as after a save or F5 during a
        // load, so the one before is left (ticket 38) as it is dropped, its bytes with it.
        shown.rendering = Some(Handed::new(render.leaver(), request));
        Some(Handover {
            render,
            request,
            showing: shown.showing,
            file: DocumentFile::of(&shown.path),
            place,
            notice: shown.held_notice.take(),
            steps,
        })
    }

    /// The file of the document `window`'s page waits for, when it asked for its document while
    /// its view showed `page_shows` (None while it shows no document): the window's document,
    /// unless the view shows it already, so the request is a reload, which keeps the document in
    /// view until the new render replaces it and so has no wait note (ticket 31). Decided by the
    /// showing, as [`Documents::handover`] decides the place. None while the window shows none.
    pub(super) fn awaited_file(
        &self,
        window: &str,
        page_shows: Option<Showing>,
    ) -> Option<PathBuf> {
        let table = self.lock();
        let shown = table.windows.get(window)?.now.shown.as_ref()?;
        (page_shows != Some(shown.showing)).then(|| shown.path.clone())
    }

    /// What `window`'s page asking for its document and placing it does, for the tests of other
    /// modules, which have no webview to call [`render_document`]: hands the document over, waits
    /// for its answer as `render_document` does, says it arrived (ticket 38), and gives the
    /// showing the page then sends with a link (None while the window shows no document).
    #[cfg(test)]
    pub fn page_asks(&self, window: &str) -> Option<Showing> {
        let Handover {
            render,
            request,
            showing,
            ..
        } = self.handover(window, None)?;
        // Rendered or not, the page gets an answer, which it places either way.
        let _answer =
            tauri::async_runtime::block_on(self.rendered(window, render, request, showing));
        self.arrived(window, showing);
        Some(showing)
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
    let Some(Handover {
        render,
        request,
        showing,
        file,
        place,
        notice,
        steps,
    }) = documents.handover(window.label(), showing)
    else {
        return Ok(None);
    };
    let content = match documents
        .rendered(window.label(), render, request, showing)
        .await
    {
        Ok(Ok(rendered)) => {
            timing.report(window.label(), place.reported_as(), &rendered.stages);
            Content::Document(rendered.document)
        }
        Ok(Err(error)) => {
            if let DocumentError::Left { stages, .. } = &error {
                timing.report(window.label(), "render-left", stages);
            }
            Content::Error(error.into())
        }
        Err(source) => Content::Error(DocumentError::ReadAndRenderStopped { source }.into()),
    };
    Ok(Some(DocumentToShow {
        showing,
        file,
        place,
        content,
        notice,
        steps,
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

/// What the page's wait note names while the window's document is on its way (ticket 31): the
/// document's file, and its size as the reader is told it, or None when it can't be told. Every
/// document is shown however large, and a large one takes seconds to read, render and lay out,
/// so the note says it is coming.
#[derive(Debug, serde::Serialize)]
pub struct WaitNote {
    file: DocumentFile,
    size: Option<String>,
}

impl WaitNote {
    /// The note for the document at `path`. Looks at the file system, so it is made on a blocking
    /// thread.
    fn of(path: &Path) -> Self {
        Self {
            file: DocumentFile::of(path),
            size: std::fs::metadata(path)
                .ok()
                .map(|metadata| size_shown(metadata.len())),
        }
    }
}

/// A file's size as the reader is told it, in Explorer's units: whole KB, then MB and GB, with
/// one decimal below 10.
fn size_shown(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    let (unit, name) = match bytes {
        ..MB => return format!("{} KB", bytes.div_ceil(KB)),
        MB..GB => (MB, "MB"),
        GB.. => (GB, "GB"),
    };
    // As f64 only to be shown, where a file of over 2^53 bytes losing its last bytes is no loss.
    #[allow(clippy::cast_precision_loss)]
    let size = bytes as f64 / unit as f64;
    // By the size as it is shown, so one that rounds up to 10 drops its decimal as 10 does.
    if (size * 10.0).round() < 100.0 {
        format!("{size:.1} {name}")
    } else {
        format!("{size:.0} {name}")
    }
}

/// What the page's wait note names for the calling window's document, or None while it shows
/// none or the page's request is a reload ([`Documents::awaited_file`]). The page asks when
/// `render_document` hasn't answered within half a second, with the showing that request sent.
/// The path comes from the app's own state, as for every command; the file's size is looked at on
/// a blocking thread, since a file on a share can take a while to answer.
#[tauri::command]
pub async fn wait_note(
    window: WebviewWindow,
    showing: Option<Showing>,
    documents: State<'_, Documents>,
) -> Result<Option<WaitNote>, ()> {
    let Some(path) = documents.awaited_file(window.label(), showing) else {
        return Ok(None);
    };
    // A blocking task that stopped leaves the note unsaid; the document still comes.
    Ok(
        tauri::async_runtime::spawn_blocking(move || WaitNote::of(&path))
            .await
            .ok(),
    )
}

/// A sample of each answer [`wait_note`] gives, by name, which the page's tests read
/// (`app/src/contract.rs`): with the document's size, without it, and for a window with no
/// document, which a reload gets too.
#[cfg(test)]
pub fn wait_note_samples() -> serde_json::Result<serde_json::Value> {
    let file = || DocumentFile {
        name: "a.md".to_owned(),
        path: r"C:\docs\a.md".to_owned(),
    };
    Ok(serde_json::json!({
        "withSize": serde_json::to_value(WaitNote { file: file(), size: Some("100 MB".to_owned()) })?,
        "withoutSize": serde_json::to_value(WaitNote { file: file(), size: None })?,
        "noDocument": serde_json::to_value(None::<WaitNote>)?,
    }))
}

/// A sample of each answer [`render_document`] gives, by name, which the page's tests read
/// (`app/src/contract.rs`, ticket 27): the rendered document goes as its HTML, the heading only
/// with a show, a read that fails as why, the document's file with each, a held notice with the
/// first (ticket 26), and whether there is somewhere to go back or forward to with each, which is
/// nowhere but after a return (ticket 12).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    use serde::ser::Error as _;

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
        " for changes as they are made, so a save reloads it up to 2 seconds later: \
         Access is denied.",
    );
    let rendered = || {
        downlight_core::render(
            b"# Install\n\nRun the installer.\n",
            Path::new("a.md"),
            "",
            Disk,
            &downlight_core::Leave::new(),
        )
        .map(Content::Document)
        .map_err(serde_json::Error::custom)
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
        "showAtHeading": answer(Place::Show { heading: Some("install".to_owned()) }, rendered()?)?,
        "showAtTop": answer(at_top(), rendered()?)?,
        "showWithHeldNotice": answer_with(at_top(), rendered()?, Some(held_notice))?,
        "reload": answer(Place::Reload, rendered()?)?,
        "return": answer(Place::Return, rendered()?)?,
        "unreadShow": answer(at_top(), unread())?,
        "unreadReturn": answer(Place::Return, unread())?,
        "unreadReload": answer(Place::Reload, unread())?,
        "noDocument": serde_json::to_value(None::<DocumentToShow>)?,
    }))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A folder of its own under the temp folder, empty.
    fn temp_folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("downlight-handover-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn wait_note_names_the_documents_file_and_its_size() {
        let folder = temp_folder("sized");
        let path = folder.join("big.md");
        std::fs::write(&path, vec![b'#'; 2 * 1024 * 1024 + 300 * 1024]).unwrap();
        let note = serde_json::to_value(WaitNote::of(&path)).unwrap();
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            note,
            serde_json::json!({
                "file": { "name": "big.md", "path": path.display().to_string() },
                "size": "2.3 MB",
            })
        );
    }

    #[test]
    fn wait_note_for_a_file_whose_size_cant_be_told_names_no_size() {
        let folder = temp_folder("gone");
        let note = WaitNote::of(&folder.join("gone.md"));
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            serde_json::to_value(note).unwrap()["size"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn sizes_are_told_in_explorers_units() {
        for (bytes, shown) in [
            (1, "1 KB"),
            (1024, "1 KB"),
            (1025, "2 KB"),
            (1024 * 1024, "1.0 MB"),
            (15 * 1024 * 1024 + 600 * 1024, "16 MB"),
            // Rounds to 10, so it is told without a decimal, as 10 MB itself is.
            (10 * 1024 * 1024 - 1, "10 MB"),
            (5 * 1024 * 1024 * 1024, "5.0 GB"),
            (10 * 1024 * 1024 * 1024 - 1, "10 GB"),
        ] {
            assert_eq!(size_shown(bytes), shown, "{bytes}");
        }
    }
}
