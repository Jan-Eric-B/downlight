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
