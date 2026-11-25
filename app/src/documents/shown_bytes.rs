//! The bytes of the file each window's shown render was made from, which Copy as Markdown copies
//! (copy-as-markdown ticket 01): the file as the render the page shows was made from it, so that
//! a copy matches what the reader sees while a reload is on its way, and after the file was
//! changed or deleted. They cost about the document's size in memory per window, shared with the
//! render rather than copied.
//!
//! The page takes a render in two steps: its request's answer comes ([`Documents::rendered`]),
//! and it places the answer ([`Documents::arrived`]). The bytes of the latest request's render
//! wait for the page to place it, since until then its view still shows the render before, and
//! the reader may stop the document on its way.

use super::{Documents, Showing};
use crate::render::{FileBytes, ReadAndRender, RenderTask};

/// One of a window's page's requests for its document, numbered for that window, so that only
/// the latest one's bytes are kept: the page places only its latest request's answer (ticket 38).
///
/// A number beside the render's own leaver (`ShownDocument::rendering`), which already leaves
/// the render of a request that a later one overtakes, or that Stop drops: leaving ends a render
/// only until the core has started sanitizing it, which then runs to its end, and Stop leaves it
/// after the table's lock is released. So a render can finish after the table has moved on, and
/// whether its bytes still belong to the page's latest request is settled here, under the lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Request(u64);

/// The bytes a window's renders were made from, each with the showing its render is for.
#[derive(Default)]
pub(super) struct ShownBytes {
    /// Those of the render the page placed last, which its view shows, if it shows one.
    placed: Option<RenderedFrom>,
    /// Those of the latest request's render, which the page hasn't placed yet.
    answered: Option<RenderedFrom>,
    /// How many requests the page made, so that each gets a number of its own.
    requests: u64,
    /// The latest request, until the document it asked for is stopped or none is shown: no
    /// render's bytes are kept for any other.
    latest: Option<Request>,
}

/// The bytes a render for `showing` was made from.
struct RenderedFrom {
    showing: Showing,
    bytes: FileBytes,
}

impl ShownBytes {
    /// The page asks for its document again: the request before is overtaken, so its render's
    /// bytes are never placed.
    pub(super) fn requested(&mut self) -> Request {
        self.answered = None;
        self.requests += 1;
        let request = Request(self.requests);
        self.latest = Some(request);
        request
    }

    /// The page placed the answer for `showing`: the bytes of its render if it had one, and else,
    /// for an answer that says why the document can't be shown, none, unless the answer was for
    /// a reload, whose page keeps its last render.
    pub(super) fn placed(&mut self, showing: Showing) {
        if let Some(answered) = self
            .answered
            .take_if(|answered| answered.showing == showing)
        {
            self.placed = Some(answered);
        } else if self
            .placed
            .as_ref()
            .is_some_and(|placed| placed.showing != showing)
        {
            self.placed = None;
        }
    }

    /// The reader stopped the document on its way, whose answer the page then never places: its
    /// render may still finish, and finds its request retired.
    pub(super) fn stopped(&mut self) {
        self.answered = None;
        self.latest = None;
    }

    /// The window shows no document, so its page shows the empty window next.
    pub(super) fn none_shown(&mut self) {
        self.placed = None;
        self.answered = None;
        self.latest = None;
    }
}

impl Documents {
    /// What the render handed over to `window`'s page for its `request` gave, once done. Its
    /// bytes are kept for the render's `showing` while the request is the page's latest, until
    /// the page says whether it placed it ([`Documents::arrived`]).
    pub(super) async fn rendered(
        &self,
        window: &str,
        render: RenderTask,
        request: Request,
        showing: Showing,
    ) -> Result<ReadAndRender, tauri::Error> {
        let finished = render.finished().await;
        if let Ok(Ok(rendered)) = &finished {
            let mut table = self.lock();
            if let Some(record) = table.windows.get_mut(window)
                && record.shown_bytes.latest == Some(request)
            {
                record.shown_bytes.answered = Some(RenderedFrom {
                    showing,
                    bytes: rendered.bytes.clone(),
                });
            }
        }
        finished
    }

    /// The bytes of the file `window`'s page shows the render of, while its view shows it as
    /// `showing`; None once it shows another, or none, or the window closed.
    pub fn bytes_shown_as(&self, window: &str, showing: Showing) -> Option<FileBytes> {
        let table = self.lock();
        let placed = table.windows.get(window)?.shown_bytes.placed.as_ref()?;
        (placed.showing == showing).then(|| placed.bytes.clone())
    }
}
