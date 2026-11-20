# 01: Sort a document's tables

**What to build:** The reader can sort a table in the shown document by one of its columns, to read a large table (a changelog, a comparison, a list of settings) in the order they need. Like a tick, the order belongs to the window and is never written to the file.

Spec: none yet. Grill it first (`/grill-with-docs`), then `/to-spec` writes `.scratch/table-sorting/spec.md`.

**Blocked by:** none

**Status:** needs-triage

- [x] Grill it before planning (2026-11-19). Decided:
  - **Sort only.** No filter: a row filter would be a second Find. Filtering rows (perhaps as a Find option, "show matching rows only") and a header that sticks to the top of a long table go on Later. One column at a time.
  - **Started from a sort button** at the end of each header cell, shown while the pointer is over the header row or the button has focus, and always on the sorted column, in the accent color with an up or down arrow from Fluent UI System Icons; the others show a neutral two-way arrow. The header cell carries `aria-sort`, and the tooltip names the next step ("Sort descending", "Show the file's order"). Each press steps ascending, descending, the file's order; sorting by another column starts it at ascending and drops the first. The sort is stable: rows that compare equal keep the file's order.
  - **How values compare:** the text the reader sees in the cell (links, code and formatting count only for their text), in natural order ignoring case (`1.9` before `1.10`, `v2` before `v10`, ISO dates in date order). A column whose non-empty cells are all numbers (with a minus sign, decimals, thousands separators, `%` or a currency sign) sorts by value. Empty cells come last in both directions. No other date formats are recognized.
  - **Which tables:** Markdown tables with at least 2 body rows. Tables written as raw HTML get no sort button.
  - **A sort belongs to the showing**, as ticks do: Back and Forward give it back, and a document shown anew starts in the file's order. A reload keeps it, re-sorting the new rows, while the document has as many tables as before and that table (by its place among them) as many columns; otherwise the table shows the file's order.
  - **The rest follows the shown order.** Find counts its matches again in the new order; the current match stays on the same text and the view doesn't move. Copying gives the rows in the order shown. Edit opens the editor at the line of the row at the top of the view, its own line in the file. The counts don't change. When they are built, the margin's line numbers (each row keeps its own, so they go out of sequence) and the overview ruler follow the shown order.
  - The glossary's **Sort** names it.
- [ ] Measured with the timing switch, when it is built: `npm run documents` also writes a test document of about 1 MB that is one table of about 10,000 rows (the 1 MB test document stays as it is, so ticket 02's baseline and ticket 31's results still compare). Every sort of it (ascending, descending, the file's order; a number column and a text column) keeps every task and frame under 100 ms. If it can't, the spec is revisited then.

## Comments

**2026-11-17, from the repo owner.** Asked for as a feature idea. It fits "reading only", since the file never changes. It is outside the v1 spec, so it gets its own feature folder rather than a v1 ticket; the v1 spec's Out of Scope may want a line for it after the grilling.
