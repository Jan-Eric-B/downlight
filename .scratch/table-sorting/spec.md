# Sort a document's tables

Status: ready-for-agent

## Problem Statement

Many of the documents I read hold large tables: a changelog, a comparison of tools, a list of settings with their defaults, a price list. The table comes in the order its author wrote it, which is rarely the order I need: I want the newest versions first, the cheapest item at the top, or the settings in alphabetical order to find one. Downlight shows the table only as the file has it, so I either scan every row or open the file in a spreadsheet or an editor to reorder it, and then I'm no longer reading the document. GitHub, VS Code's preview and most Markdown viewers don't sort tables either, so I copy the table somewhere else just to read it.

## Solution

The reader can **Sort** any Markdown table with at least two body rows by one of its columns. Each header cell has a small sort button at its end, which shows while the pointer is over the header row or the button has the focus, and always on the column the table is sorted by. Each press steps through ascending, descending and the file's order; sorting by another column starts that column at ascending and drops the first. Values compare as the reader sees them, in natural order ignoring case (`1.9` before `1.10`, `v2` before `v10`, ISO dates in date order), a column of numbers sorts by value, empty cells come last in both directions, and rows that compare equal keep the file's order.

Like a **Tick**, a Sort belongs to the **Showing** and is never written to the file: Back and Forward give it back, a document shown anew starts in the file's order, and a **Reload** keeps it while the table still fits it. Everything that reads the shown document follows the order shown: **Find**, the **Overview ruler**, Ctrl+C, Edit's line, and the margin's line numbers once they are built. **Copy as Markdown** copies the file as written, so it ignores the Sort. The counts don't change.

## User Stories

1. As a reader, I want to sort a table by one of its columns, so that I can read a large table in the order I need.
2. As a reader, I want a sort button at the end of each header cell, so that I can sort by the column I'm looking at.
3. As a reader, I want the sort buttons to show while the pointer is over the header row, so that a table I'm only reading stays as clean as GitHub shows it.
4. As a reader, I want the sorted column's button to show always, in the accent color with an up or down arrow, so that I can see at a glance that the table is sorted, by which column and which way.
5. As a reader, I want the other columns' buttons to show a neutral two-way arrow, so that I can tell the sorted column from the sortable ones.
6. As a reader, I want the first press of a column's button to sort ascending, the second descending and the third to show the file's order again, so that one button does all three and I can always get back to the file's order.
7. As a reader, I want a press on another column's button to sort by that column ascending and drop the first sort, so that the table is sorted by one column at a time and I'm never unsure which one.
8. As a reader, I want the button's tooltip to name what the next press does ("Sort ascending", "Sort descending", "Show the file's order"), so that I know before I press it.
9. As a reader, I want rows that compare equal to keep the file's order, so that a sort never shuffles rows at random and the author's order still decides among equals.
10. As a reader, I want a cell compared by the text I see in it, so that links, code and emphasis sort by their text and not by their markup.
11. As a reader, I want text compared ignoring case, so that "apple" and "Apple" sort together.
12. As a reader, I want numbers inside text compared by value (`1.9` before `1.10`, `v2` before `v10`, `item 9` before `item 10`), so that version lists and numbered items sort as I expect.
13. As a reader, I want ISO dates (`2026-01-05`) to sort in date order, so that a changelog sorts by date.
14. As a reader, I want a column whose cells are all numbers to sort by value, with a minus sign, decimals, thousands separators, a percent sign or a currency sign, so that prices and percentages sort as amounts and not as text.
15. As a reader, I want empty cells last, whether I sort ascending or descending, so that the rows with a value always come first.
16. As a reader, I want a column with a single non-number cell to sort in natural order, so that one stray "n/a" makes the column sort as text rather than putting it somewhere arbitrary.
17. As a reader, I want only Markdown tables with at least two body rows to be sortable, so that a one-row table doesn't carry buttons that do nothing.
18. As a reader, I want a table written as raw HTML to show no sort buttons, so that a table the author laid out by hand, perhaps with merged cells, is never broken by a sort.
19. As a reader, I want a table inside a list, a quote or a collapsed section to be sortable too, so that every Markdown table behaves the same.
20. As a reader, I want the header row to stay where it is when I sort, so that the columns stay named.
21. As a reader, I want the view not to move when I sort, so that the table I'm sorting stays in front of me.
22. As a reader, I want the focus to stay on the button I pressed, so that I can press it again with the keyboard to step on.
23. As a keyboard reader, I want to reach each sort button with Tab and press it with Enter or Space, so that I can sort without a mouse.
24. As a keyboard reader, I want a sort button to show while it has the focus, so that I can see where the focus is.
25. As a screen reader user, I want the header cell to say how its column is sorted (ascending, descending or not), so that I know the table's order without seeing the arrows.
26. As a screen reader user, I want the button's name to say what the next press does, so that I know what pressing it will do.
27. As a reader, I want the sort buttons never to be found by Find or copied by Ctrl+C, so that Downlight's own controls never get mixed into the document's text.
28. As a reader, I want showing a button on hover never to reflow the table, so that the columns don't jump as the pointer moves over the header.
29. As a reader, I want the sort buttons in both themes, with either accent color, so that they fit whatever I chose.
30. As a reader using a high contrast theme, I want the sort buttons drawn in the system's colors, so that I can still see them and which column is sorted.
31. As a reader, I want a sort to stay when I go Back to another document and Forward again, so that returning to a document returns it as I left it, as ticks do.
32. As a reader, I want a document I open anew to show its tables in the file's order, so that a sort from an earlier visit never surprises me.
33. As a reader, I want a reload to keep the sort and sort the new rows the same way, so that an edit to the file doesn't undo my order.
34. As a reader, I want a reload to drop a table's sort when the document now has a different number of tables, or that table a different number of columns, so that a sort never lands on the wrong table or the wrong column.
35. As a reader, I want a table that a reload left with fewer than two body rows to show the file's order with no buttons, so that the sort never applies to a table it no longer fits.
36. As a reader, I want a reload to return to the row I was reading in the sorted table, so that a reload keeps my **Reading position** inside a sorted table too.
37. As a reader, I want sorts to stay when I zoom, switch Full width, resize the window or change the theme, so that none of those loses my order.
38. As a reader, I want a sort never written to the file, so that Downlight stays a viewer and the file is exactly as its author left it.
39. As a reader, I want sorting in one window never to change another window showing the same file, so that each window keeps its own order, as it keeps its own ticks.
40. As a reader, I want Find to count its matches again in the new order after a sort, so that "N of M" and Enter and F3 step through the rows as I see them.
41. As a reader, I want Find's current match to stay on the same text when I sort, without the view moving, so that a sort doesn't throw me to another match.
42. As a reader, I want the Overview ruler's Find marks to follow the new order, so that the ruler and the table agree.
43. As a reader, I want Ctrl+C of rows of a sorted table to give them in the order I see, as plain text and in the formatted copy, so that what I paste is what I selected.
44. As a reader, I want Copy as Markdown of a sorted table to give its rows in the file's order, so that it copies the file as written.
45. As a reader, I want Edit to open the editor at the line of the row at the top of the view, its own line in the file, so that the editor opens where I'm reading even in a sorted table.
46. As a reader, I want the document's counts (lines and words) and the selection's counts to stay the same after a sort, so that a sort is only ever an order.
47. As a reader, I want, once the margin's line numbers are built, each row to keep its own line number after a sort, so that the numbers still say where each row is in the file, even out of sequence.
48. As a reader of a large table (10,000 rows), I want every sort, and the step back to the file's order, to keep the window answering, with no task or frame over 100 ms, so that sorting a big table feels instant.
49. As a reader of a large document with many tables, I want the sort buttons to cost the document's first paint nothing I can feel, so that sorting never makes opening a file slower.
50. As a reader, I want a sort to make no network request and read nothing outside the shown document, so that it adds nothing to what Downlight touches.
51. As a reader, I want the printed or exported document, once Print and Export are built, to keep the sort as shown without the sort buttons, so that the paper shows the order I chose and none of Downlight's controls.

## Implementation Decisions

- **Page only.** Sort is a new page module named after the feature. The app and the core don't change, no app command or contract shape is added, and no dependency is added. The icons come from Fluent UI System Icons, which the page already uses: the sort up and sort down arrows (16, regular) for the sorted column, the two-way sort arrow for the others.
- **Which tables.** A Markdown table is one whose element carries the core's line numbers (`data-sourcepos`): sanitizing drops that attribute from a document's raw HTML, so a raw HTML table never has it. Only those with at least two body rows get buttons. A table's identity, for a reload and a return, is its place among the document's Markdown tables, in document order.
- **The module's interface** follows the ticks' (the closest prior art):
  - one call that makes the tables of a freshly parsed render sortable, beside the calls that add the copy buttons and make the task items tickable, before the content goes into the view;
  - a read of the sorts in the view, as a value: how many Markdown tables the document has, and for each sorted table its place, its number of columns, its column and its direction;
  - a restore that gives the view those sorts back, dropping all of them when the number of Markdown tables differs and one table's when its number of columns differs or it now has fewer than two body rows;
  - a value for no sorts, which a document shown anew starts with;
  - a listener for when the reader's press reordered a table's rows, which Find hears. A restore needs no event of its own, since the view's shown listeners follow it.
- **The showing keeps its sorts as it keeps its ticks.** Where the view gives the history the reader's state on leaving a showing (its line and its ticks), it gives the sorts too, and the three placements take the same rule as ticks: a reload keeps the view's own sorts from just before the swap, a return takes those its showing had when the view left it, and a show, the empty state or an error takes none. Ticks and sorts may be kept together as one value of what the reader did to the showing, so that a later addition (such as opened sections) joins it in one place. The restore happens where the ticks' does: after the content goes in and before the landing and the shown listeners, so a reload lands on the reading position's row where the sort put it, and Find and the ruler count the sorted order.
- **The order.** A sort reorders the table body's row elements in place: the same nodes are moved, never re-created, so Find's matches, a selection's nodes and the copy buttons' handlers stay valid. The header row never moves. The file's order is the rows' order by their line numbers. Keys are read once per press from each cell's text as the reader sees it, and compared with an `Intl.Collator` in the webview's locale with numeric comparison on and case ignored (sensitivity "accent"). The sort is stable. Empty cells (only whitespace) go last in both directions.
- **Numbers.** A column sorts by value when every non-empty cell in it is a number: optional minus sign (`-` or `−`), digits with optional `,` thousands separators in groups of three, an optional `.` and decimals, and optionally a percent sign or a currency sign (any Unicode currency symbol) before or after, with or without a space. A cell written in another convention, such as `1.234,5`, isn't a number, and its column sorts in natural order. The test is made per press over the whole column, so one non-number cell makes the column text.
- **The button.** A `button` of type `button` at the end of each header cell, with only an icon and no text, so Find never matches it and the counts never count it, and its accessible name is the tooltip's text, which names the next step: "Sort ascending", "Sort descending", "Show the file's order". The header cell carries `aria-sort` (`ascending`, `descending`, or none). Its space is always kept, and it is hidden by visibility, so showing it on hover or focus never reflows the table. It shows while the pointer is over the header row or it has the focus, and always on the sorted column, in the accent color; the accent test that lists every use of the accent roles in the stylesheets gains it. In forced colors it takes the system's button colors, and the sorted column's the highlight color. A press leaves the focus on the button and doesn't start a selection.
- **The view stays.** A sort changes the scroller's position not at all. The table is one top-level block (ADR 0003), so its height may change only as much as its rows' wrapping does, and nothing re-anchors the view to the reading position after a sort.
- **Find.** Find hears the sort's listener and counts again, as it does after a reload, with two differences: the current match stays on the same text, found by its start node and offset among the new matches (since the nodes are the same), and Find doesn't scroll. If that text no longer matches (it can't, since a sort changes no text) the current match is the first from the reading position. Find tells its listeners of the new count, so the Overview ruler redraws its Find marks through Find's existing listener; the ruler needs no hook of its own. With the find bar closed or empty, a sort costs Find nothing beyond marking its index stale. How Find refreshes its index (all of it, or the table's part) is left to the implementation, bounded by the measurement.
- **Copying.** Ctrl+C copies the view as shown, so its plain text and its clean HTML follow the sort already; the clean copy drops the sort button with every other `button`. Copy as Markdown takes rows by their line numbers and puts them in the file's order, as it does today. Neither changes; both get tests with a sorted table.
- **Edit.** The reading block already descends into a table to the row crossing the top of the view, and a row's line is its own, so Edit's line follows a sort with no change; it gets a test.
- **Unchanged.** The counts, the list of headings (a table holds no headings) and the ticks (a table holds no task items) are untouched by a sort.
- **Later features that follow the shown order**, recorded here so their tickets inherit it: the margin's line numbers (each row keeps its own, out of sequence), Print and Export (the sort as shown, without the buttons), and a table filter (tables ticket 06).
- **Glossary.** **Sort** already names it (added in the grilling). No new term.

## Testing Decisions

- **A good test** drives the page as the reader does and checks what the reader can observe: the rows' order as their text reads, the header cells' `aria-sort`, the buttons' names and whether they show, the find bar's "N of M" and which text is current, where Back and Forward and a reload leave the table, Edit's line, and what each copy puts on the clipboard. It never calls the comparison or reads the module's state. Names state the behavior, such as `a third press shows the file's order again` or `a reload with a column fewer drops that table's sort`.
- **One seam: the page** (Vitest with happy-dom), through the existing fakes: `fakeTheApp` to show the fixture, reload it with changed HTML and step through History; button presses and keyboard events on the sort buttons; typing in the find bar; `layOut` for the reading position and Edit's line; the copy tests' clipboard. Confirmed with the repo owner when the spec was written.
- **A new rendered fixture**, `sort.md` with its HTML beside the others, written by `npm run fixtures`: a table with a text column, a natural-order column (versions and numbered items), an ISO date column, a number column with minus signs, decimals, thousands separators, percent and currency signs, a column with a stray non-number cell, empty cells, rows that compare equal, cells with links, code and emphasis; a table with one body row; a raw HTML table; and a table inside a list. It isn't a rendering change, so it touches no other fixture.
- **Cases** include: each column kind sorting ascending and descending; equal rows keeping the file's order; empty cells last both ways; the third press showing the file's order; another column's press starting ascending and dropping the first; the tooltip and `aria-sort` at each step; no buttons on the one-row table or the raw HTML table; the table in a list sortable; Back and Forward giving the sort back; a document shown anew in the file's order; a reload keeping the sort over changed rows, and dropping it for a table count or a column count that changed; a reload landing on the reading position's row in a sorted table; Find's count, steps and current match after a sort; the Overview ruler's Find marks redrawn after a sort; Ctrl+C in the shown order with no button in it; Copy as Markdown in the file's order; Edit's line at a sorted row; the counts unchanged.
- **Prior art:** the ticks' page tests (a reader's mark kept through reload and Back, and dropped when the document no longer fits it), Find's page tests (a reload's recount and the current match), the History tests, the copy and Copy as Markdown tests (a sorted table is already named there), and the accent test.
- **Measurement, not a test** (ticket 05): `npm run documents` also writes a test document of about 1 MB that is one table of about 10,000 rows, with a number column and a text column; the 1 MB test documents stay as they are, so ticket 02's baseline and ticket 31's results still compare. `npm run measure` presses each sort of it with the timing switch (ascending, descending, the file's order; on the number column and on the text column), with the find bar open on a common word and closed, and every task and frame stays under 100 ms. First paint and the frame after the document goes in on the 1 MB mixed document (a table every 30 lines) stay within the spread of ticket 02's baseline. If a sort can't stay under 100 ms, the spec is revisited then. The results go in the ticket.

## Out of Scope

- **Filtering a table's rows**: tables ticket 06, perhaps as a Find option (find-options ticket 01).
- **A header that sticks to the top** of a long table: tables ticket 07.
- **Sorting by more than one column** at a time.
- **Raw HTML tables**, and any other date format than ISO.
- **Choosing how a column compares** (as text or as numbers) by hand, or a locale setting for it.
- **A shortcut, a right-click entry or a menu entry** for sorting.
- **Writing the order to the file**, or keeping it across windows or restarts.
- **Changes to the core**, to rendering or to the app.

## Further Notes

- Decided in the grilling of 2026-11-19 (the first Sort ticket's list, which tickets 01 to 05 replaced). The spec adds the decisions the grilling left open: the exact number format (`.` as the decimal point, `,` as the thousands separator, as GitHub's English shows them), the collator's settings, that the button's space is always kept so the table never reflows, that rows are moved rather than re-created, and that sorts are kept beside the ticks in the history.
- The number format is the one decision here most likely to need a second look: a document written with `,` as its decimal point sorts its number columns in natural order, which is right for whole numbers and wrong for decimals. Recognizing both would need a per-column guess.
- The Overview ruler is built (ruler tickets 01 to 03), and its spec already expects Sort's reorder; ruler ticket 03 decided that later Find features keep the ruler working as long as they count through Find's count and step through its step, which this spec follows.
- export-html ticket 01 plans one page module for what the page adds to a render (copy buttons, tickable checkboxes) that also gives back an inert copy of the view without them. The sort buttons are a third such addition; whichever of the two is built second joins the other.
