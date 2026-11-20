# 03: Find's marks, and clicking them

**What to build:** While the find bar has matches, the **Overview ruler** marks each match of **Find** and, apart from them, the **Current match**, so the reader sees where the matches cluster. A click on a Find mark makes the match nearest the pointer the current match and scrolls it into view, as F3 does.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 01 (Heading marks beside the scroll bar)

**Status:** ready-for-agent

- [ ] Find gains a listener for when its matches or its current match change (a recount as the reader types, a step with Enter, F3 or the arrow buttons, a reload's recount, the bar closing), and a way to make a given match current and scroll it into view as a step does. `findMatches()` stays the read side; Find keeps owning the current match.
- [ ] A match's mark sits at its top-level block's top plus its share of that block's text times the block's height; no rectangle inside the block is read.
- [ ] Match marks are short marks in the strip's right half in the accent's Find match role; the current match is the full width, a little thicker, in the current match role. A heading mark and a match mark in one row both show.
- [ ] The marks appear as the reader types, follow each step, and go when Find closes.
- [ ] A click in the strip's right half near a match mark makes the nearest match current and scrolls to it, without joining **History**.
- [ ] The accent test that lists every use of the accent roles in the stylesheets gains the ruler's two uses.
- [ ] Page tests (written first): marks appear for the typed text and follow Enter and F3; closing Find removes them; a click on a match mark makes the nearest match current and Back doesn't return from it.
- [ ] Hand check in the release build: the colors with Amber and with Windows' accent, in light and dark, and in a high contrast theme (the system's highlight colors).
- [ ] `npm run check` passes.
