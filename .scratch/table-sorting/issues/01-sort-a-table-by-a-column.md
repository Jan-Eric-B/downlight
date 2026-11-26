# 01: Sort a table by a column, in natural order

**What to build:** The reader can **Sort** a Markdown table by one of its columns with a sort button at the end of each header cell. Each press steps ascending, descending, the file's order; a press on another column starts it at ascending and drops the first sort. Cells compare by the text the reader sees, in natural order ignoring case, rows that compare equal keep the file's order, and empty cells come last both ways. This is the first, complete path of the feature: in the window, on any document, for one showing (History and Reload are ticket 03's).

Spec: [Sort a document's tables](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A new rendered fixture, `sort.md` with its HTML, written by `npm run fixtures`: a table with a text column, a natural-order column (versions, numbered items), an ISO date column, a number column (minus signs, decimals, thousands separators, percent and currency signs; ticket 02 uses it), a column with one stray non-number cell, empty cells, rows that compare equal, cells with links, code and emphasis; a table with one body row; a raw HTML table; a table inside a list. No other fixture changes.
- [ ] A new page module for Sort makes the tables of a freshly parsed render sortable, beside the copy buttons and the tickable task items, before the content goes into the view. Only tables with the core's line numbers (Markdown tables) and at least two body rows get buttons; the raw HTML table and the one-row table get none; the table inside a list gets them.
- [ ] Each header cell gets a `button` of type `button` at its end, icon only (Fluent UI System Icons: sort up and sort down for the sorted column, the two-way sort arrow for the others), whose accessible name is its tooltip, naming the next step: "Sort ascending", "Sort descending", "Show the file's order". The header cell carries `aria-sort` (`ascending`, `descending`, or none).
- [ ] The buttons show while the pointer is over the header row or a button has the focus, and the sorted column's always, in the accent color. Their space is always kept (hidden by visibility), so showing one never reflows the table. In forced colors they take the system's button colors, the sorted column's the highlight color. The accent test that lists every use of the accent roles gains the new use.
- [ ] A press reorders the body's row elements in place (the same nodes, moved), never the header row; the file's order is the rows' order by their line numbers. Keys are each cell's visible text, read once per press, compared with an `Intl.Collator` in the webview's locale, numeric on, case ignored (sensitivity "accent"). The sort is stable; empty (whitespace-only) cells go last in both directions.
- [ ] A press leaves the focus on the button, starts no selection and doesn't move the view (the scroller's position is unchanged). Tab reaches each button, Enter and Space press it.
- [ ] Find never matches a button (it has no text) and Ctrl+C never copies one (the clean copy drops buttons; checked with a test).
- [ ] Page tests (written first), on the fixture through `fakeTheApp`: each press steps ascending, descending, the file's order, with tooltip and `aria-sort` at each step; another column's press starts at ascending and drops the first; natural order (`1.9` before `1.10`, `v2` before `v10`, ISO dates) ignoring case; links, code and emphasis compare by their text; equal rows keep the file's order; empty cells last both ways; no buttons on the raw HTML and one-row tables; the table in a list sorts; the focus stays on the button; the view doesn't move.
- [ ] Hand check in the release build: the buttons on hover and focus, the sorted column's arrow in both themes, with Amber and with Windows' accent, and in a high contrast theme; the header doesn't reflow when the pointer crosses it.
- [ ] `npm run check` passes.
