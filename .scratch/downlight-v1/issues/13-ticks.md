# 13: Ticks

**What to build:** A reader can tick task list items while working through a checklist. **Ticks** survive a reload as long as the number of task items stays the same, come back with Back and Forward, and are never written to the file.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 12 (Back and Forward)

**Status:** ready-for-agent

- [ ] Task items can be ticked and unticked with the mouse and with the keyboard.
- [ ] A reload with the same number of task items keeps the ticks; a reload with a different number clears them.
- [ ] Back and Forward restore the ticks of that history entry.
- [ ] Ticks are never written to the file and are gone when the window closes.
- [ ] Page tests cover each rule above.

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- A reload replaces the whole rendered document in one step: `place(parsed.content)` in `reload`, in `page/src/view.ts`. After it, the old task items are gone. Read the ticks just before that swap and apply them just after, in the same place. A listener that runs later can't read them, because the old items are already gone.
- `whenDocumentShown` doesn't say why a document was shown, but ticks must tell a reload (keep them if the count matches) from another document (clear them). Pass the listeners a reason rather than adding a separate flag; ticket 12 needs the same.
- Reload's page tests (the tests named "a reload ..." in `page/src/view.test.ts`) show how to drive a reload in a test: queue the next `render_document` answer, then `emit("document-changed", "reload")`.

**2026-11-13, from the architecture review.** Ticket 25 gives the view one pipeline, with one place that prepares the content (make the task checkboxes tickable there; the core writes them `disabled`) and one snapshot read before the swap and restored after it. Ticks go into that snapshot. Ticket 12 keys history entries by their `Showing`, so key the saved ticks the same way. The `whenDocumentShown` listeners get a reason after ticket 25, which says whether a swap was a reload.

**2026-11-16, from ticket 26.** The page's tests no longer send `document-changed`: a save is `saveTheFile()` in `page/src/test-support.ts` (the app's `ask-for-document`), and a reload by hand is `requestDocument()` from `page/src/view.ts`, which F5 now calls directly. Either way the app's answer says `reload`, and the listeners hear the reason `reload`.

**2026-11-16, from ticket 12.** Where ticks plug into Back and Forward:

- The page keeps the history's per-showing state in `WindowHistory` (`page/src/history.ts`), keyed by `Showing` as the architecture review asked. Its `Kept` record per showing holds the visits (line numbers) and has room for the ticks beside them: ticks belong to the showing, not to a visit, since a heading link within a document changes no ticks.
- Save them where the line is saved: `replaceContent` in `page/src/view.ts` calls `history.leave(left, before.line)` with the `ReaderState` read just before the swap. Add the ticks to `ReaderState`, read them there, and have `leave` keep them on the showing's `Kept`.
- Restore them on a return: the answer's place is `return` (`Place::Return`, reason `return` for the `whenDocumentShown` listeners), and `atReturn` lands through `history.returnTo(showing)`. Give the ticks back there, or in `replaceContent` once the new content is in, before the landing.
- `history.showAnew` starts a showing afresh on a show (Ctrl+O, a drop, a link, and a document given again, which is a new showing): no ticks, as the spec says for another document. A reload keeps its showing and never calls `leave`, so the reload rule (same number of task items) stays the reload's own.
