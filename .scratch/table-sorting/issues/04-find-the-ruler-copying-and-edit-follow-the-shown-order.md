# 04: Find, the ruler, copying and Edit follow the shown order

**What to build:** After a **Sort**, everything that reads the shown document reads it in the order shown. **Find** counts its matches again, so "N of M", Enter and F3 step through the rows as the reader sees them, with the **Current match** on the same text and the view where it was; the **Overview ruler**'s Find marks follow. Ctrl+C copies rows in the order shown, **Copy as Markdown** in the file's order, Edit opens the editor at the line of the row at the top of the view, and the **Counts** don't change.

Spec: [Sort a document's tables](../spec.md)

**Blocked by:** 01 (Sort a table by a column, in natural order); overview-ruler 03 (Find's marks, and clicking them), which gives Find its match listener and the ruler its Find marks

**Status:** ready-for-agent

- [ ] The Sort module gains a listener for when the reader's press reordered a table's rows (a restore needs none: the shown listeners follow it). Find hears it.
- [ ] On a sort, Find counts again as after a reload, except that the current match stays on the same text, found by its start node and offset among the new matches (the nodes are the same), and Find doesn't scroll. With the find bar closed or empty, a sort costs Find no more than marking its index stale. How Find refreshes its index is left to this ticket, bounded by ticket 05's measurement.
- [ ] Find tells its listeners of the new count, so the ruler redraws its Find marks through Find's existing listener; the ruler gets no hook of its own (ruler ticket 03's decision: later Find features count through Find's count and step through its step).
- [ ] Tests only, as the code already does it: Ctrl+C of rows of a sorted table gives them in the order shown, in its plain text and its clean HTML, with no sort button; Copy as Markdown gives them in the file's order (its existing test reorders rows by hand; one now sorts with the button); Edit's line at a row of a sorted table is that row's own line; the document's and the selection's counts are the same after a sort.
- [ ] Page tests (written first): with the find bar open, a sort recounts, "N of M" stays the same, the current match is on the same text and the view doesn't move; Enter and F3 after it step in the order shown; the ruler's Find marks are redrawn after a sort; and the copy, Edit and counts cases above.
- [ ] `npm run check` passes.
