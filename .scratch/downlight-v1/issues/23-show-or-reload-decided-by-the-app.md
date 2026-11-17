# 23: The app decides whether a page shows or reloads its document

**What to build:** A window's page shows its document's render the way the app says, never by which event made it ask. Each time a window is given a document, that showing gets its own identifier; the page sends the identifier of what its view shows with each request, and the app answers either "show it, at this heading or the top" or "reload it at the reading position". A `reload` for a document the window has just left, arriving after `document-shown`, then shows the new document at its heading or top, instead of at the old document's reading position.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 22 (One module owns a window's shown document)

**Status:** closed

- [x] Each document a window is given gets an identifier for that showing, unique within the process. Opening a document that is open already gives no new one.
- [x] The page sends the identifier of the showing its view shows (or none while it shows none, the empty state or an error) with its `render_document` request.
- [x] `Documents::handover` answers a show (with the heading, or the top) when the page's identifier isn't the current one, and a reload when it is, and carries the current identifier back. The heading exists only on a show.
- [x] A failing read reaches the page inside the answer, so a failing reload still keeps the last render and shows a notice, and a failing show still shows why in the view.
- [x] `document-shown` and `document-changed: "reload"` both only make the page ask. Overtaking (latest request wins), parsing a reload a frame apart (ADR 0003), returning to the reading position, first paint and reload timing, and `whenDocumentShown` stay as they are.
- [x] Tested first: in `app/src/documents/tests/handover.rs` through the public interface with the recorder, a reload overtaken by another document is answered as a show of the new one at its heading; in `page/src/view.test.ts`, `document-shown` followed by a stale `reload` shows the new document at its heading, not at the old reading position.
- [x] A stale `gone` (for the document the window has left, arriving after `document-shown`) leaves no "File deleted or moved" bar over the new document, or the gap is recorded here.

## Decisions

**2026-11-12, from ticket 21's remaining gap and ticket 22's review.**

- **The race:** `Documents::changed` checks the key and starts the reload under the table's lock, but sends `Message::Reload` after releasing it, and `give` sends `Message::DocumentShown` after its own lock. So: a save to the old document is checked and its reload started, then Ctrl+O, a drop or a link gives the window a new document, the page gets `document-shown` and then the stale `reload`. The page's `reload()` overtakes its `showDocument()`, so the new document shows at the old document's reading position and the heading a link named is ignored.
- **Not sending under the lock:** the module sends every message outside the table's lock on purpose. Sending while holding it would let a page's request on the main thread wait on the table while the sender waits on the webview, a deadlock risk, and it would only narrow the race, since the page can still act on the events in either order.
- **Decided by data:** each `ShownDocument::given` takes a new identifier from a counter in `WindowTable` (`Showing`). The page sends the one its view shows; `Documents::handover(window, page_shows)` answers `Place::Show { heading }` when it differs and `Place::Reload` when it matches. The page's two events only make it ask. This also folds the "heading handed over with every render, dropped by the page on a reload" half-protocol from ticket 22's review into one place.
- **The answer's shape:** an enum, so that ticket 12 can add a third answer (return to a saved reading position) without touching the other two. A read that fails is part of the answer rather than the command's error, since only the answer says whether the page keeps its last render (reload) or shows why (show).
- **No glossary entry:** the showing's identifier is plumbing between the app and the page, not something the reader sees or names.

## Comments

**2026-11-12, done.** `Showing` (a counter in `WindowTable`, taken by `ShownDocument::given`) and `Place` (`Show { heading }` or `Reload`) live in `app/src/documents.rs`; `Documents::handover(window, page_shows)` decides between them under the table's lock. `render_document` takes `showing: Option<Showing>` and answers `{ showing, place: "show", heading, document | error }` or `{ showing, place: "reload", document | error }`; it no longer fails, since a failed read is part of the answer. In `page/src/view.ts`, `requestDocument` sends the view's showing for both `document-shown` and `reload`, and hands the answer to `show` or `reload`, which keep their behavior (overtaking, the template parsed a frame apart, the reading position, notices, timing, `whenDocumentShown`). The page's tests answer through `answerAsTheApp` in `page/src/test-support.ts`, which decides as `Documents::handover` does, and give the window another document with `giveAnotherDocument`.

Tests: `stale_reload_after_another_document_is_answered_with_a_show_at_its_heading`, `page_that_shows_the_current_document_is_answered_with_a_reload`, `reload_answer_keeps_the_heading_out`, `window_given_back_a_document_it_showed_before_shows_it_anew` and `opening_a_shown_document_again_keeps_its_showing` in `app/src/documents/tests.rs`; `document_to_show_serializes_as_the_page_reads_it` (updated) and `document_that_cannot_be_read_goes_with_why_as_the_page_reads_it` in `app/src/render.rs`; "a reload for the document the window left, arriving after another was shown, shows the new one at its heading" and "a 'gone' for the document the window left, arriving after another was shown, leaves no bar over the new one" in `page/src/view.test.ts`.

The stale `gone`: fixed by clearing the bar when a show is placed, since a newly shown document was just read. That trades one narrow gap for another: a `gone` for the newly shown document that reaches the page after its request went out and before the show is placed is now dropped too, where before it kept the bar. A wrong bar over a readable document seemed worse than a missing one for a document deleted in those few milliseconds. Closing both exactly would need `gone` to carry the showing it is about, which changes the `document-changed` payload from a plain string; left for now.

**2026-11-13, after the review of tickets 22-24.** `render_document` and `DocumentToShow` moved from `app/src/render.rs` into the documents module, so the two serialization tests named above are now in `app/src/documents/tests/handover.rs`, with the other `app/src/documents` tests named there. See ticket 22's comments.

**2026-11-16, closed by the repo owner.**
