//! A window's history (ticket 12): the documents it showed before the one it shows, and after it
//! once Back has stepped away from them, which Back and Forward give the window back. The history
//! belongs to the window's record in the table, so it is never saved and goes when the window
//! closes ([`Documents::forget`]).
//!
//! The app holds only which documents, by their paths, since commands never take a path from the
//! page. Each keeps the showing it had, so that the page, which keeps where its view was in each
//! showing and the headings it visited in it (`page/src/history.ts`), returns there.

use std::path::PathBuf;

use tauri::{State, WebviewWindow};

use super::pasted_files::{HeldDocument, HoldsDocuments};
use super::{
    Arrival, Claimed, Documents, Given, ShowError, Showing, ShownDocument, WindowRecord,
    WindowState,
};
use crate::notice::{NamedFile, Notice};

/// The documents a window showed before the one it shows, and after it, nearest last.
#[derive(Debug, Clone, Default)]
pub(super) struct History {
    back: Vec<Visited>,
    forward: Vec<Visited>,
}

/// A document a window showed, as the showing it had.
#[derive(Debug, Clone)]
struct Visited {
    showing: Showing,
    /// As the launch, the dialog, the drop or the link named it.
    path: PathBuf,
    /// The document's key, worked out when the window was first given it ([`ShownDocument`]).
    key: PathBuf,
}

impl Visited {
    fn of(shown: &ShownDocument) -> Self {
        Self {
            showing: shown.showing,
            path: shown.path.clone(),
            key: shown.key.clone(),
        }
    }
}

/// The document, as [`Documents::let_go`] takes it.
impl HoldsDocuments for Visited {
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        std::iter::once(HeldDocument {
            key: &self.key,
            path: &self.path,
        })
    }
}

/// Every document Back and Forward could give the window.
impl HoldsDocuments for History {
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        self.back
            .iter()
            .chain(&self.forward)
            .flat_map(HoldsDocuments::held)
    }
}

/// A step through a window's history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    Back,
    Forward,
}

/// Whether a window has a document to go back to, and one to go forward to, for the page, which
/// offers Back and Forward only when there is somewhere to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(super) struct Steps {
    pub(super) back: bool,
    pub(super) forward: bool,
}

impl History {
    /// The window has moved on from `left` to a document it was given: `left` is the last step
    /// back, and the documents after it are gone, as in a browser.
    fn moved_on_from(&mut self, left: &ShownDocument) {
        self.back.push(Visited::of(left));
        self.forward.clear();
    }

    pub(super) fn steps(&self) -> Steps {
        Steps {
            back: !self.back.is_empty(),
            forward: !self.forward.is_empty(),
        }
    }

    /// The document `step` goes to, if there is one.
    fn next(&self, step: Step) -> Option<&Visited> {
        match step {
            Step::Back => self.back.last(),
            Step::Forward => self.forward.last(),
        }
    }

    /// Takes `step` away from `left`, the document shown now, which becomes the step that goes
    /// the other way.
    fn take(&mut self, step: Step, left: Option<&ShownDocument>) {
        let (to, from) = match step {
            Step::Back => (&mut self.back, &mut self.forward),
            Step::Forward => (&mut self.forward, &mut self.back),
        };
        to.pop();
        from.extend(left.map(Visited::of));
    }
}

/// How a window's record in the table changes its history as it changes its document.
impl WindowRecord {
    /// Shows `shown` in place of the document in view, which becomes the last step back, and
    /// holds what the window was until `shown` arrives ([`WindowRecord::move_on`]). Gives the
    /// document on its way it left, if one was, to drop after the table's lock.
    pub(super) fn given(&mut self, shown: ShownDocument) -> Option<WindowState> {
        self.move_on(|in_view| {
            let mut history = in_view.history.clone();
            if let Some(left) = &in_view.shown {
                history.moved_on_from(left);
            }
            WindowState {
                shown: Some(shown),
                history,
            }
        })
    }

    /// Takes `step` from the document in view to `to`, the document it goes to, and holds what
    /// the window was until `to` arrives ([`WindowRecord::move_on`]). Gives the document on its
    /// way it left, if one was, to drop after the table's lock.
    fn stepped(&mut self, step: Step, to: ShownDocument) -> Option<WindowState> {
        self.move_on(|in_view| {
            let mut history = in_view.history.clone();
            history.take(step, in_view.shown.as_ref());
            WindowState {
                shown: Some(to),
                history,
            }
        })
    }
}

impl Documents {
    /// Takes `step` in `window`'s history: gives the window the document it showed before, or
    /// after, as the showing it had then, so that its page returns to where it was in it. It is
    /// carried out as [`Documents::give`] carries out a change of document: the window is titled
    /// after it, its page told to ask for it, its render started and its file watched, and if the
    /// title or the page can't be changed, the window keeps the document it shows and its history.
    ///
    /// A document open in another window by now has that window brought to the front instead, and
    /// this window stays where it is, as when Ctrl+O chooses it: a document is shown in at most one
    /// window (ADR 0001). Gives None, and does nothing, when there is nowhere to go.
    pub fn step(&self, window: &str, step: Step) -> Result<Option<Given>, StepError> {
        let Some((claimed, to)) = self.claim_step(window, step)? else {
            return Ok(None);
        };
        self.carry_out(claimed, &to.path, to.key.clone())
            .map(Some)
            .map_err(|source| StepError::Show {
                document: to.path,
                source,
            })
    }

    /// Records `step` in `window`'s history under the table's lock, as [`Documents::claim`]
    /// records a show, with the document stepped to. While the window's document is on its way,
    /// the step goes from the document its page's view still shows, and the one on its way never
    /// joins the history (ticket 38).
    fn claim_step(
        &self,
        window: &str,
        step: Step,
    ) -> Result<Option<(Claimed, Visited)>, StepError> {
        let mut table = self.lock();
        let record = table.windows.get(window).ok_or(StepError::WindowClosed)?;
        let Some(to) = record.in_view().history.next(step).cloned() else {
            return Ok(None);
        };
        if let Some((elsewhere, record)) = table
            .window_showing(&to.key)
            .filter(|(other, _)| *other != window)
        {
            // A step to the document another window's page's view shows during a load stops the
            // load there, as a request for it from anywhere does (ticket 38's review).
            let claimed = Claimed::AlreadyElsewhere {
                window: elsewhere.clone(),
                stop: record.held_showing(&to.key),
                scroll_to: None,
            };
            return Ok(Some((claimed, to)));
        }
        // Before the record is borrowed, since it is issued by the table.
        let claim = table.next_claim();
        // Looked up again, since finding the window that shows the document reads the table.
        let record = table
            .windows
            .get_mut(window)
            .ok_or(StepError::WindowClosed)?;
        let arrived = ShownDocument::arrived(
            to.path.clone(),
            to.key.clone(),
            to.showing,
            claim,
            Arrival::Stepped,
        );
        let left = record.stepped(step, arrived).map(Box::new);
        let claimed = Claimed::Changed {
            window: window.to_owned(),
            claim,
            left,
        };
        Ok(Some((claimed, to)))
    }

    /// The reader went somewhere new in the document `window` shows as `showing`: its page
    /// scrolled to a heading on a link. As in a browser, Forward has nowhere to go any more. A
    /// window that has left that showing since keeps its history, since the page's view showed
    /// a document the window had stepped away from. Each pasted file Forward went to that no
    /// window holds any more goes.
    pub fn forget_forward(&self, window: &str, showing: Showing) {
        let forgotten = {
            let mut table = self.lock();
            match table.windows.get_mut(window) {
                Some(record)
                    if record
                        .now
                        .shown
                        .as_ref()
                        .is_some_and(|shown| shown.showing == showing) =>
                {
                    std::mem::take(&mut record.now.history.forward)
                }
                _ => Vec::new(),
            }
        };
        self.let_go(forgotten);
    }
}

/// Why Back or Forward didn't show the reader the document it went to.
#[derive(Debug)]
pub enum StepError {
    /// The window closed first.
    WindowClosed,
    /// Showing the document failed, as a show fails.
    Show {
        document: PathBuf,
        source: ShowError,
    },
}

impl From<StepError> for Notice {
    fn from(error: StepError) -> Self {
        match error {
            // Nobody is likely left to read it, but a notice is the one way to say it.
            StepError::WindowClosed => "The window has closed.".into(),
            StepError::Show { document, source } => Self::naming(
                "Can't show ",
                NamedFile::at(&document),
                format!(": {source}"),
            ),
        }
    }
}

/// Goes back or forward in the calling window's history, as Alt+Left and Alt+Right, the mouse's
/// side buttons and the right-click menu do. The page passes only which way: the document comes
/// from the window's own history. Nowhere to go does nothing. Async, so that it runs off the main
/// thread, as every show does.
#[tauri::command]
pub async fn step_history(
    window: WebviewWindow,
    step: Step,
    documents: State<'_, Documents>,
) -> Result<(), Notice> {
    documents.step(window.label(), step)?;
    Ok(())
}

/// Forward has nowhere to go any more in the calling window, which shows its document as
/// `showing`: its page scrolled to a heading on a link. Passes only the showing, so a page whose
/// view shows a document the window has left changes nothing.
#[tauri::command]
pub fn forget_forward(window: WebviewWindow, showing: Showing, documents: State<'_, Documents>) {
    documents.forget_forward(window.label(), showing);
}
