# 03: Typing filters the Outline

**What to build:** Typing in the **Outline**'s filter box keeps only the headings whose text contains what the reader typed, ignoring case as **Find** does, so the reader finds a section by its name and Enter goes to it.

Spec: [Outline](../spec.md)

**Blocked by:** 02 (The Outline lists the headings and goes to one)

**Status:** ready-for-agent

- [ ] The filter box is enabled and takes typing; a row is kept when its heading's text, folded as Find folds text (case, final sigma), contains the box's text folded the same way.
- [ ] Kept rows keep their indentation; their parent headings are hidden.
- [ ] After each change the first kept row is selected; Up, Down, Page Up, Page Down and Enter work on the kept rows as on the whole list.
- [ ] None kept: the list shows "No headings match", as a live status, and Enter does nothing.
- [ ] Clearing the box brings back the whole list with the reading position's heading selected again.
- [ ] The keys that edit text (typing, Ctrl+A, Ctrl+C, Ctrl+V, Ctrl+Z) act in the box, as they do in Find's.
- [ ] Each keystroke costs one pass over the headings' text plus the rows shown (only the rows near the list's view, as in ticket 02).
- [ ] Page tests (written first): typing keeps the matching rows, ignoring case, parents hidden, indentation kept; the first match selected and Enter going to it; "No headings match"; clearing restores the list and the reading position's heading; Ctrl+A selects the box's text, not the document's.
- [ ] `npm run check` passes.
