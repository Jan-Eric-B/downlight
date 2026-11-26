# 02: Number columns sort by value

**What to build:** A column whose non-empty cells are all numbers sorts by their value, so prices, percentages and amounts sort as amounts and not as text. One cell that isn't a number makes the column sort in natural order, as ticket 01 sorts text.

Spec: [Sort a document's tables](../spec.md)

**Blocked by:** 01 (Sort a table by a column, in natural order)

**Status:** ready-for-agent

- [ ] A cell is a number when it is: an optional minus sign (`-` or `−`), digits with optional `,` thousands separators in groups of three, an optional `.` and decimals, and optionally a percent sign or a currency sign (any Unicode currency symbol) before or after, with or without a space. Surrounding whitespace doesn't count.
- [ ] The test is made per press over the whole column's non-empty cells; a column of numbers compares by value, empty cells still last both ways, equal values keep the file's order.
- [ ] A cell written in another convention (`1.234,5`) or with other text (`n/a`, `12 kg`) isn't a number, and its column sorts in natural order.
- [ ] Page tests (written first), on the `sort.md` fixture's number column: ascending and descending by value with negatives, decimals, thousands separators, `%` and currency signs before and after; the column with one stray cell sorts in natural order; empty cells last.
- [ ] `npm run check` passes.
