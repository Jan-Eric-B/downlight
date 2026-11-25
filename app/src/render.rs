//! Reading a document's file and rendering it on a blocking thread. The documents module starts
//! these and hands what they give to the window's page; this module knows nothing of windows.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use downlight_core::{Leave, RenderError, RenderedDocument};
use tauri::async_runtime::JoinHandle;

use crate::disk::Disk;
use crate::image;
use crate::notice::{NamedFile, Notice};
use crate::platform;
use crate::timing::Stages;
use crate::watch::ReadStamp;

/// The bytes of a document's file as one read gave them, shared by the render made from them and
/// the document table, which keeps those of the render a window's page took for Copy as Markdown
/// (copy-as-markdown ticket 01). A `Vec` behind the `Arc` rather than a slice, which would copy
/// a large document's bytes once more.
pub type FileBytes = Arc<Vec<u8>>;

/// A document's file rendered: the rendered document, the bytes it was made from, and how long
/// reading and rendering it took for the timing switch.
pub struct Rendered {
    pub document: RenderedDocument,
    pub bytes: FileBytes,
    pub stages: Stages,
}

impl fmt::Debug for Rendered {
    /// By sizes: a document's bytes and HTML can be 100 MB each, which a failed test or a log line
    /// must not print.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rendered")
            .field(
                "document",
                &format_args!("{} bytes of HTML", self.document.as_str().len()),
            )
            .field("bytes", &format_args!("{} bytes", self.bytes.len()))
            .field("stages", &self.stages)
            .finish()
    }
}

/// The rendered document, or why the window can't show it.
pub type ReadAndRender = Result<Rendered, DocumentError>;

/// A document being read and rendered on a blocking thread, by [`start`]. Dropped before it is
/// done, as when the window is given another document before its page took it, it is left
/// (ticket 38), so that it gives up soon rather than render for nobody.
pub struct RenderTask {
    task: JoinHandle<ReadAndRender>,
    leave: Leave,
}

impl RenderTask {
    /// What reading and rendering gave, once done. Fails only when the blocking thread panicked
    /// or was cancelled.
    pub async fn finished(mut self) -> Result<ReadAndRender, tauri::Error> {
        (&mut self.task).await
    }

    /// A handle that leaves the render, for whoever no longer waits for it once it is handed
    /// over.
    pub fn leaver(&self) -> Leave {
        self.leave.clone()
    }

    /// Whether the render has been left.
    #[cfg(test)]
    pub fn is_left(&self) -> bool {
        self.leave.is_left()
    }
}

impl Drop for RenderTask {
    fn drop(&mut self) {
        // Harmless once the render is done.
        self.leave.leave();
    }
}

/// Starts reading and rendering `path` on a blocking thread, off the main thread and the async
/// runtime's workers. The timing switch reports it by the place the page gets (`Place` in
/// `documents.rs`).
///
/// Gives the file's stamp from just before the read too, which a watch on the file starts from,
/// so that a save after the read reloads the document, even one made before the watch started
/// (ticket 30). A reload or a render the page asks for drops it: the watch runs already.
pub fn start(path: &Path) -> (RenderTask, ReadStamp) {
    let path = path.to_owned();
    let (noter, read_stamp) = ReadStamp::pending();
    let leave = Leave::new();
    let leaving = leave.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        noter.note(&path);
        read_and_render(&path, &leaving)
    });
    (RenderTask { task, leave }, read_stamp)
}

fn read_and_render(path: &Path, leave: &Leave) -> ReadAndRender {
    let mut stages = Stages::start();
    let bytes = std::fs::read(path).map_err(|source| unread(path, source))?;
    stages.lap("read");
    let rendered = downlight_core::render(&bytes, path, &image::origin(), Disk, leave);
    stages.lap("render");
    match rendered {
        Ok(document) => Ok(Rendered {
            document,
            bytes: Arc::new(bytes),
            stages,
        }),
        // With its stages, so the timing switch tells how soon a left render ended.
        Err(RenderError::Left) => Err(DocumentError::Left {
            path: path.to_owned(),
            stages,
        }),
        Err(RenderError::NotText) => Err(DocumentError::NotText {
            path: path.to_owned(),
        }),
    }
}

/// Why `path` couldn't be read. Windows reads a folder as access denied, which would send the
/// reader looking at permissions, so a folder is told apart; only after the read failed, so a
/// document still takes one read. A missing file's folder opens from the notice (window design
/// ticket 11), looked at here, on the blocking thread.
fn unread(path: &Path, source: io::Error) -> DocumentError {
    if path.is_dir() {
        return DocumentError::Folder {
            path: path.to_owned(),
        };
    }
    let file = if platform::is_missing(&source) {
        NamedFile::missing(path)
    } else {
        NamedFile::at(path)
    };
    DocumentError::Read { file, source }
}

/// Why the window can't show its document. The page shows it to the reader, in the view or as a
/// notice.
#[derive(Debug)]
pub enum DocumentError {
    Read {
        file: NamedFile,
        source: io::Error,
    },
    /// The path names a folder: a launch's argument, a dropped path or a link that isn't a file.
    Folder {
        path: PathBuf,
    },
    /// The core refused the file, a binary one: from the command line, as a link's `.md` target,
    /// or a document whose file became binary since it was shown. Open and a drop refuse a binary
    /// file before the window is given it (`open.rs`).
    NotText {
        path: PathBuf,
    },
    /// The background task that reads and renders the document panicked or was cancelled.
    ReadAndRenderStopped {
        source: tauri::Error,
    },
    /// Nobody waited for the render any more, so it gave up (ticket 38): the reader stopped the
    /// document or went elsewhere, or a later request for it overtook this one. The reader never
    /// sees its notice: the page has already dropped the request it answers.
    Left {
        path: PathBuf,
        stages: Stages,
    },
}

impl From<DocumentError> for Notice {
    fn from(error: DocumentError) -> Self {
        match error {
            DocumentError::Read { file, source } => {
                Self::naming("Can't read ", file, format!(": {source}"))
            }
            DocumentError::Folder { path } => {
                Self::naming("", NamedFile::at(&path), " is a folder, not a document")
            }
            DocumentError::NotText { path } => not_text(&path),
            DocumentError::ReadAndRenderStopped { source } => {
                format!("Reading and rendering the document stopped: {source}").into()
            }
            DocumentError::Left { path, .. } => {
                Self::naming("Stopped showing ", NamedFile::at(&path), "")
            }
        }
    }
}

/// What the reader is told about the binary file at `path`, however it arrived: the same words
/// for a launch, a link, Open and a drop (ticket 34).
pub fn not_text(path: &Path) -> Notice {
    Notice::naming(
        "",
        NamedFile::at(path),
        " isn't a text file. Downlight opens text files.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_is_told_apart_from_a_document() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Err(error) = read_and_render(folder, &Leave::new()) else {
            panic!("a folder rendered as a document");
        };
        assert_eq!(
            Notice::from(error).to_string(),
            format!("{} is a folder, not a document", NamedFile::at(folder))
        );
    }

    #[test]
    fn binary_file_is_refused_with_a_notice_that_downlight_opens_text_files() {
        let icon = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("icons")
            .join("icon.ico");
        let Err(error) = read_and_render(&icon, &Leave::new()) else {
            panic!("a binary file rendered as a document");
        };
        assert_eq!(
            Notice::from(error).to_string(),
            format!(
                "{} isn't a text file. Downlight opens text files.",
                NamedFile::at(&icon)
            )
        );
    }

    #[test]
    fn text_file_with_another_extension_renders_as_markdown() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let Ok(Rendered { document, .. }) = read_and_render(&manifest, &Leave::new()) else {
            panic!("a text file was refused");
        };
        assert!(document.as_str().contains("[package]"), "{document:?}");
    }

    #[test]
    fn debug_of_a_rendered_document_shows_its_sizes_and_not_its_text() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let Ok(rendered) = read_and_render(&manifest, &Leave::new()) else {
            panic!("a text file was refused");
        };
        let shown = format!("{rendered:?}");
        assert!(
            shown.contains(&format!("bytes: {} bytes", rendered.bytes.len())),
            "{shown}"
        );
        assert!(!shown.contains("[package]"), "{shown}");
    }

    /// Reads `missing.md` in a folder of its own, which is there unless `folder_gone`.
    fn read_missing(folder_gone: bool) -> (PathBuf, serde_json::Value) {
        let folder = std::env::temp_dir().join(format!(
            "downlight-render-{}-{folder_gone}",
            std::process::id()
        ));
        std::fs::create_dir_all(&folder).unwrap();
        if folder_gone {
            std::fs::remove_dir_all(&folder).unwrap();
        }
        let read = read_and_render(&folder.join("missing.md"), &Leave::new());
        let _ = std::fs::remove_dir_all(&folder);
        let Err(error) = read else {
            panic!("a missing file rendered as a document");
        };
        (folder, serde_json::to_value(Notice::from(error)).unwrap())
    }

    #[test]
    fn missing_file_cannot_be_read_and_the_notice_opens_its_folder() {
        let (folder, error) = read_missing(false);
        assert_eq!(error["before"], "Can't read ");
        assert_eq!(
            error["file"],
            serde_json::json!({
                "name": "missing.md",
                "folder": folder.display().to_string(),
                "folderOpens": true,
            })
        );
    }

    #[test]
    fn missing_file_whose_folder_is_gone_too_names_a_folder_that_does_not_open() {
        let (_, error) = read_missing(true);
        assert_eq!(error["file"]["folderOpens"], false);
    }
}
