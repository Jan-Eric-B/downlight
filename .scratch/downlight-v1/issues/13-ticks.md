# 13: Ticks

**What to build:** A reader can tick task list items while working through a checklist. **Ticks** survive a reload as long as the number of task items stays the same, come back with Back and Forward, and are never written to the file.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 12 (Back and Forward)

**Status:** ready-for-human

- [x] Task items can be ticked and unticked with the mouse and with the keyboard.
- [x] A reload with the same number of task items keeps the ticks; a reload with a different number clears them.
- [x] Back and Forward restore the ticks of that history entry.
- [x] Ticks are never written to the file and are gone when the window closes.
- [x] Page tests cover each rule above.

## Results

Built 2026-11-16. `npm run check` passes. Ticks are the page's alone: a new module, `page/src/ticks.ts`, and no change to the core, the app or the contract. `prepare` in `page/src/view.ts` makes the task items tickable (`makeTickable`), the reader state read just before every swap holds the ticks (`readTicks`), and `replaceContent` gives the new content the ticks it takes (`restoreTicks`) right after the swap, before the landing and the `whenDocumentShown` listeners. Each placement says which ticks it takes, as it says where it lands: a reload those from before the swap, a return those `WindowHistory` kept for its showing (`ticksOf`), a show, the empty state and an error none. `history.leave` keeps the ticks on the showing's `Kept`, and `showAnew` starts a showing without them.

**Beyond the plan:**

- **The page enables the checkboxes, not the core.** The core still writes them `disabled`, as GitHub does, so the rendered fixtures and the sanitizing rules are unchanged; the page, which handles the reader's input, takes `disabled` off in `prepare`, as the architecture review asked. That stays safe because sanitizing keeps `input` only where rendering writes it (`RENDERING_ONLY_TAGS`, tested by `checkboxes_come_only_from_task_lists`), so every checkbox the page enables is a task item's, never a form field of the document's.
- **A tick is kept as where the reader's mark differs from the file** (`checked` against `defaultChecked`, which is the render's `checked` attribute), not as every item's state. So a reload that keeps the ticks still shows what the file now says of the items the reader left alone: marking `[x]` in the editor and saving shows it ticked. A click also unticks an item the file has ticked, and that untick is kept like a tick.
- **A return checks the number of task items too.** Back and Forward read the file again, so if its number of task items changed since the view left it, the ticks are dropped rather than land on the wrong items, as for a reload (story 65).
- **The keyboard:** a tickable checkbox takes the focus with Tab or a click, and Space ticks it through the webview's own click; no command or Find handler takes Space. happy-dom gives no click for Space, so the page test checks that the item takes the focus and that the page leaves the key alone.

**Tests:** eleven new page tests in `page/src/ticks.test.ts`, written first: ticking and unticking with a click (also an item the file ticked), the keyboard's focus and Space, no command to the app when ticking, a reload with the same and with a different number of task items, a reload showing the file's new state of untouched items, Back, Forward and Back again, Back to a document whose number of task items changed, and a document shown anew. No existing test changed.

**Measured:** inconclusive, the machine was under heavy load. The change adds two `querySelectorAll` walks per swap (the old view and the new content) and restores nothing when there are no ticks; the generated documents have no task items. `npm run measure` on the release build, three sessions: the reload of the 1 MB mixed document was bimodal, and in its quiet runs (read about 77 ms) insert was 54 ms, anchor 51 to 55 ms and nothing over 100 ms, under ticket 28's 63 and 66 ms. In the loaded runs the core's read (430 to 470 ms) and render (up to 1426 ms) were several times slower too, which the page can't cause, and insert, anchor and the longest frame were 100 to 240 ms. The first session's cold first paint was 1035 ms for 100 KB and 1263 ms for 1 MB mixed, against ticket 28's 787 and 1137 ms, on the same load. The scrolling and Find cases timed out with no window shown in 20 s in the first and third sessions, after the same documents had shown normally earlier in the session. Run `npm run measure` again on a quiet machine.

**Not yet checked by hand** (release build): a ticked checkbox is amber in light and dark (window design ticket 03 expected gray while they were disabled) and its focus ring shows; a click and Tab then Space tick and untick, Space on a focused item doesn't scroll; ticks survive a save with the same number of task items and clear when an item is added; Back and Forward after a link and Ctrl+O bring them back; the file is unchanged on disk after ticking; closing the window and opening the document again shows the file's own ticks.

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

**2026-11-16, decided by the repo owner.** Both choices in Results stay: a tick is kept only where the reader's mark differs from the file, and Back and Forward drop the ticks when the number of task items changed, as a reload does.
