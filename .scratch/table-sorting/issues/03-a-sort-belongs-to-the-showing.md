# 03: A Sort belongs to the showing

**What to build:** A **Sort** lives with the **Showing**, as a **Tick** does. Back and Forward give a document its sorts back, a document shown anew starts in the file's order, and a **Reload** keeps each sort, sorting the new rows the same way, while the table still fits it. A reload in a sorted table returns to the row the reader was reading.

Spec: [Sort a document's tables](../spec.md)

**Blocked by:** 01 (Sort a table by a column, in natural order)

**Status:** ready-for-agent

- [ ] Prefactor first, with no visible change: where the view hands the history the reader's state on leaving a showing (its line and its ticks), and where the placements take ticks ("keep" for a reload, the showing's for a return, none for a show, the empty state or an error), the ticks become one value of what the reader did to the showing, which the sorts then join. The ticks' tests pass unchanged.
- [ ] The Sort module reads the view's sorts as a value (how many Markdown tables the document has, and for each sorted table its place among them, its number of columns, its column and direction), and restores them, with a value for no sorts.
- [ ] The restore drops every sort when the number of Markdown tables differs, and one table's when its number of columns differs or it now has fewer than two body rows (no buttons then, the file's order).
- [ ] The sorts are restored where the ticks are: after the content goes in, before the landing and the shown listeners, so a reload lands on the reading position's row where the sort put it, and Find and the ruler count the sorted order.
- [ ] Another window showing the same file keeps its own order.
- [ ] Page tests (written first), through `fakeTheApp` and the History helpers: Back to another document and Forward again gives the sort back; the same file shown anew starts in the file's order; a reload with changed rows keeps the sort and sorts the new rows; a reload with a table more or fewer drops all sorts; a reload with a column fewer drops that table's sort and keeps another's; a reload that leaves a table one body row shows the file's order without buttons; a reload with the view inside a sorted table returns to the same row (`layOut`); zoom and Full width keep the sort.
- [ ] `npm run check` passes.
