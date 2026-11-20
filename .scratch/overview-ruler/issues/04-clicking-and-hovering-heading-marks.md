# 04: Clicking and hovering heading marks

**What to build:** A click on a heading mark of the **Overview ruler** goes to that heading, as a heading link does, so Back returns to where the reader was. Hovering a heading mark names the heading before the reader clicks.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 01 (Heading marks beside the scroll bar)

**Status:** ready-for-agent

- [ ] A click in the strip's left half within a few pixels of a heading mark goes to the first heading of that mark's run through the view's `visitHeading`: it opens collapsed sections, scrolls as a heading link does and joins **History**.
- [ ] A click on the strip where no mark is near does nothing.
- [ ] Hovering a heading mark shows a label left of the strip, in the link target label's style, with the run's headings' text, up to five and then "and N more"; none over match marks or empty strip. The label goes when the pointer leaves.
- [ ] The pointer is a hand over a mark.
- [ ] Page tests (written first): a click on a heading mark goes there and Back returns; a click on a merged mark goes to its first heading; a click on empty strip leaves the view and History as they were; the hover label lists the run's headings.
- [ ] `npm run check` passes.
