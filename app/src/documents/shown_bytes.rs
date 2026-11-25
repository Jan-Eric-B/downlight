//! The bytes of the file each window's shown render was made from, which Copy as Markdown copies
//! (copy-as-markdown ticket 01): the file as the render the page shows was made from it, so that
//! a copy matches what the reader sees while a reload is on its way, and after the file was
//! changed or deleted. They cost about the document's size in memory per window, shared with the
//! render rather than copied.
//!
//! The page takes a render in two steps: its request's answer comes ([`Documents::rendered`]),
//! and it places the answer ([`Documents::arrived`]). Until then its view still shows the render
//! before, so an answer's bytes wait on the render handed over ([`Handed`]), on the document it is
//! for, and go with it when the window leaves that document, as Stop and going elsewhere do. The
//! window keeps what its page placed last as its [`View`].

use downlight_core::Leave;

use super::{Documents, Showing, WindowRecord, WindowTable};
use crate::render::{FileBytes, ReadAndRender, RenderTask};

/// One of the pages' requests for their documents, numbered for the whole table, so that only the
/// answer to the request a document's render was last handed over for keeps its bytes: the page
/// places only its latest request's answer (ticket 38). Leaving a render ends it only until the
/// core has started sanitizing it, which then runs to its end, so a render can finish after a
/// later request overtook it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Request(u64);

impl WindowTable {
    /// A request of its own for a page that asks for its document.
    pub(super) fn next_request(&mut self) -> Request {
        self.requests_issued += 1;
        Request(self.requests_issued)
    }
}

/// The render last handed to a document's page ([`Documents::handover`]), for the page's `request`,
/// and the bytes it was made from once it is done, until the page places its answer. Dropped, as
/// when a later request replaces it or the window leaves the document, it leaves the render: the
/// page waits for the latest one only.
pub(super) struct Handed {
    leave: Leave,
    request: Request,
    bytes: Option<FileBytes>,
}

impl Handed {
    pub(super) const fn new(leave: Leave, request: Request) -> Self {
        Self {
            leave,
            request,
            bytes: None,
        }
    }
}

impl Drop for Handed {
    fn drop(&mut self) {
        self.leave.leave();
    }
}

/// What a window's page's view shows: the showing of the answer it placed last, and the bytes of
/// the file its render was made from, None for an answer that says why the document can't be
/// shown.
pub(super) struct View {
    showing: Showing,
    bytes: Option<FileBytes>,
}

impl WindowRecord {
    /// The page placed the answer for `showing`: the render it was handed for the window's
    /// document, if the window still shows that one, and else an answer without bytes, such as one
    /// that says why the document can't be shown, unless it was for a reload, whose page keeps its
    /// last render.
    pub(super) fn placed(&mut self, showing: Showing) {
        let bytes = self
            .now
            .shown
            .as_mut()
            .filter(|shown| shown.showing == showing)
            .and_then(|shown| shown.rendering.as_mut()?.bytes.take());
        self.view = match (bytes, self.view.take()) {
            (None, Some(kept)) if kept.showing == showing => Some(kept),
            (bytes, _) => Some(View { showing, bytes }),
        };
    }

    /// The page shows the empty window next.
    pub(super) fn shows_none(&mut self) {
        self.view = None;
    }

    /// The showing of the answer the page placed last, which its view shows; None before the
    /// first, and once it shows the empty window.
    pub(super) fn placed_showing(&self) -> Option<Showing> {
        self.view.as_ref().map(|view| view.showing)
    }
}

impl Documents {
    /// What the render handed over to `window`'s page for its `request` gave, once done. Its
    /// bytes are kept on the render handed over, while the window's document is still the one
    /// shown as `showing` and its render was last handed over for this request, until the page
    /// places the answer ([`Documents::arrived`]).
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
            if let Some(handed) = table
                .windows
                .get_mut(window)
                .and_then(|record| record.now.shown.as_mut())
                .filter(|shown| shown.showing == showing)
                .and_then(|shown| shown.rendering.as_mut())
                .filter(|handed| handed.request == request)
            {
                handed.bytes = Some(rendered.bytes.clone());
            }
        }
        finished
    }

    /// The bytes of the file `window`'s page shows the render of, while its view shows it as
    /// `showing`; None once it shows another, or none, or the window closed.
    pub fn bytes_shown_as(&self, window: &str, showing: Showing) -> Option<FileBytes> {
        let table = self.lock();
        let view = table.windows.get(window)?.view.as_ref()?;
        (view.showing == showing).then(|| view.bytes.clone())?
    }
}
