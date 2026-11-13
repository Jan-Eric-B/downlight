# 25: One pipeline places every document in the page's view

**What to build:** A refactor of the page, with no change the reader sees. One view module asks the app for the window's document and places the answer for a reason (shown, reload, and later back and forward). "The latest request wins", preparing the content, keeping the reader's state across the swap and telling the listeners happen once, behind that module, instead of once per path.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [x] `page/src/view.ts` has one pipeline that every request goes through. It owns "the latest request wins", the swap of the view's content, and a single `announce` that passes the `whenDocumentShown` listeners a reason (`shown`, `reload`). Each kind of request says only how it lands: at the top or a heading, or at the reading position.
- [x] Content is prepared in one place, for a show and a reload alike. Today copy buttons are added to the view after a show (`addCopyButtons(view)`) and to the parsed template on a reload (`addCopyButtons(parsed.content)`). Ticket 13 adds tickable checkboxes in the same place.
- [x] The reader's state is read just before the swap and restored just after it, as one step that ticket 13 extends with ticks and ticket 12 with a history entry's line. The reading position is the only state today.
- [x] The view exposes its request, so F5 can ask for the document directly (ticket 26).
- [x] First paint's marks no longer travel as return values (`requestDocument(): Promise<number | undefined>`, `show` returning `received`); they go to `page/src/timing.ts` where they happen.
- [x] A first request that a save overtakes still reports first paint once the window shows the document. Today the window is shown with nothing placed and first paint is never reported.
- [x] The reading position is measured from the top of what the reader sees: while "File deleted or moved" shows, the block under the sticky bar isn't the reading position. The page's stylesheet now sets `scroll-padding-top` to the bar's height while it shows (`--file-gone-height` in `page/src/document.css`), so `readingBlock` in `page/src/reading-position.ts` can measure from the scroll padding instead of 0.
- [x] Overtaking, the template parsed a frame apart (ADR 0003), the reading position, notices and reload timing behave as before. The existing tests in `page/src/view.test.ts` pass unchanged, apart from the listener's new reason argument.

## Comments

**2026-11-13, from the architecture review.** Ticket 12's comments (from ticket 14's review) already ask for this before Back and Forward; ticket 23 did the dispatch half (`requestDocument` hands the answer to `show` or `reload`). What is left:

- `request !== latestRequest` is checked after each `await` in three functions, and `announce()` is called on five exits by hand.
- The listeners can't tell why the view changed, which ticks need (keep on a reload with the same count, clear otherwise).
- Back would be a fourth path of the same shape.

Tests survive as they are: they drive `startDocumentView` through the fake app (`page/src/test-support.ts`) and read the view.

**2026-11-13, done.** In `page/src/view.ts`, `request` is the one pipeline: `ask` turns the app's answer into a `Placement` (a reason, the showing, the content and a `Landing`) or a notice, `place` prepares the content (`prepare` parses every render in a template and adds the copy buttons there), reads the `ReaderState` just before `replaceContent` and lands with it just after, and `settle` runs the listeners once with the reason. "The latest request wins" is two checks in `request` plus one after a reload's frame and one inside each landing. A show lands with `atHeading` (the top for null), a reload with `atReadingPosition`; the empty state and errors land at the top. A show and a reload overtaken before the listeners heard of it are announced as `shown`. `requestDocument()` is exported for ticket 26. `startDocumentView` waits for the first request that ends while still the latest, so a save that overtakes the first request no longer shows the window empty, and first paint's marks go to `markFirstPaint` in `page/src/timing.ts`, which `reportFirstPaint` sends once. `readingBlock` measures from the root's `scroll-padding-top`.

- A show now parses through the template too, but not a frame apart, which would hold up first paint. `npm run measure` on the release build, against `HEAD` built the same way: the show's insert (HTML received until inserted) is 46 ms for 1 MB mixed (`HEAD`: 47 to 51) and 14 ms for 1 MB prose (`HEAD`: 14); first paint is within the run-to-run spread. A reload: insert 62 ms, anchor 65 ms, nothing over 100 ms after it, no lines moved. One of five 1 MB mixed runs had a 110 ms frame after the document; `HEAD`'s runs had frames over 100 ms before the document in most runs, the gap ticket 02 recorded.
- New tests: the listeners' reason, a reason kept across an overtaking failed reload, `requestDocument` reloading at the reading position, the window showing its document when a save overtakes the first request, the reading position below the bar (`view.test.ts`), and first paint reported once, after an overtaken first request, and not for an empty window (`timing.test.ts`, which loads the modules afresh with `?timing`, since the switch is read when `timing.ts` loads).
