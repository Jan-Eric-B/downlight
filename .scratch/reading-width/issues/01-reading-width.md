# 01: The reader sets the reading width

**What to build:** When **Full width** is off, the reader can make the document's column wider or narrower than GitHub's reading width (980 px), and the choice is kept in the **Settings** for every window, as Full width is. Changing it keeps the reading position (`keepReadingPosition` in `page/src/view.ts`).

Spec: none yet. Grill it first (`/grill-with-docs`). It may belong under the v1 spec's Later item "reader settings".

**Blocked by:** none

**Status:** wontfix

- [ ] Grill it before planning. Open questions:
  - **Its unit.** The owner's idea is a percentage. A percentage of the window makes the line length change with every resize, which is what a reading width exists to prevent, so the recommendation is a width in px at 100% zoom (or in characters), from about 600 px up to Full width.
  - **Where it is set.** The owner's idea is a slider at the bottom right of the window. The recommendation is a slider in the settings flyout, since the width is set once and then left alone: a control always on the document takes room and covers the selection's **Counts** bubble at the bottom right. Possible additions: dragging the column's edge, or shortcuts that step it.
  - How it meets Full width (the slider's last step, or two separate settings) and a docked Outline (`.scratch/outline/issues/01-outline.md`), which needs the margin a wider column takes.

## Comments

**2026-11-20, from the repo owner.** Asked for as a feature idea while discussing the overview ruler and the outline.

**2026-11-20, from the repo owner.** Dropped (wontfix). GitHub's reading width and **Full width** stay the two widths; reader settings no longer asks about a width of its own.
