# 25: One pipeline places every document in the page's view

**What to build:** A refactor of the page, with no change the reader sees. One view module asks the app for the window's document and places the answer for a reason (shown, reload, and later back and forward). "The latest request wins", preparing the content, keeping the reader's state across the swap and telling the listeners happen once, behind that module, instead of once per path.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] `page/src/view.ts` has one pipeline that every request goes through. It owns "the latest request wins", the swap of the view's content, and a single `announce` that passes the `whenDocumentShown` listeners a reason (`shown`, `reload`). Each kind of request says only how it lands: at the top or a heading, or at the reading position.
- [ ] Content is prepared in one place, for a show and a reload alike. Today copy buttons are added to the view after a show (`addCopyButtons(view)`) and to the parsed template on a reload (`addCopyButtons(parsed.content)`). Ticket 13 adds tickable checkboxes in the same place.
- [ ] The reader's state is read just before the swap and restored just after it, as one step that ticket 13 extends with ticks and ticket 12 with a history entry's line. The reading position is the only state today.
- [ ] The view exposes its request, so F5 can ask for the document directly (ticket 26).
- [ ] First paint's marks no longer travel as return values (`requestDocument(): Promise<number | undefined>`, `show` returning `received`); they go to `page/src/timing.ts` where they happen.
- [ ] A first request that a save overtakes still reports first paint once the window shows the document. Today the window is shown with nothing placed and first paint is never reported.
- [ ] The reading position is measured from the top of what the reader sees: while "File deleted or moved" shows, the block under the sticky bar isn't the reading position. The page's stylesheet now sets `scroll-padding-top` to the bar's height while it shows (`--file-gone-height` in `page/src/document.css`), so `readingBlock` in `page/src/reading-position.ts` can measure from the scroll padding instead of 0.
- [ ] Overtaking, the template parsed a frame apart (ADR 0003), the reading position, notices and reload timing behave as before. The existing tests in `page/src/view.test.ts` pass unchanged, apart from the listener's new reason argument.

## Comments

**2026-11-13, from the architecture review.** Ticket 12's comments (from ticket 14's review) already ask for this before Back and Forward; ticket 23 did the dispatch half (`requestDocument` hands the answer to `show` or `reload`). What is left:

- `request !== latestRequest` is checked after each `await` in three functions, and `announce()` is called on five exits by hand.
- The listeners can't tell why the view changed, which ticks need (keep on a reload with the same count, clear otherwise).
- Back would be a fourth path of the same shape.

Tests survive as they are: they drive `startDocumentView` through the fake app (`page/src/test-support.ts`) and read the view.
