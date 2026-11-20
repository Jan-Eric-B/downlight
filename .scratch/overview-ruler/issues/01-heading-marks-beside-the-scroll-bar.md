# 01: Heading marks beside the scroll bar

**What to build:** The **Overview ruler**'s strip, just left of the scroll bar, with a mark for each of the shown document's h1 to h3 headings, level with the scroll bar's thumb when the heading's block is in view. It is always on for now (ticket 02 adds the switch). The reader sees the document's sections at a glance; clicking comes in ticket 04.

Spec: [Overview ruler](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Prefactor first: the page tests' layout fake also fakes the scroller's scroll height and client height, which the ruler scales by, without changing what existing tests see.
- [ ] A new page module, started at the page's startup after Find, owns the strip: about 14 px wide at any zoom, the scroller's full height, no background of its own, `aria-hidden` and never focused.
- [ ] Marks are page elements (spec, "Marks as page elements"), placed from the top-level blocks' tops only, scaled by the strip's height over the scroll height, merged per pixel row; no rectangle inside a top-level block is read (ADR 0003).
- [ ] Headings h1 to h3 with an anchor id get a mark, wherever they sit (placed by their top-level block); h1 the strip's full width, h2 shorter, h3 shortest, from its left edge, in a muted text color. h4 to h6 and raw HTML headings without an id get none.
- [ ] The strip is hidden in the empty window, while a **Wait note** shows for a new document, and when the document doesn't scroll. During a reload, the old ruler stays until the new render replaces the document.
- [ ] It redraws, at most once per frame, when a document is shown (shown, reload, return), and when the article's or the scroller's size changes (blocks laid out for the first time, zoom, Full width, window resize).
- [ ] The text never runs under the strip: in the margin in a wide window, otherwise over the document's right padding, with the view giving up only the difference where that padding is narrower than the strip.
- [ ] Page tests (written first): marks at the right heights for each level, none for h4 to h6; marks in one row merge; the strip hidden in the empty window, during a Wait note and for a document that doesn't scroll; the marks follow a reload and Back.
- [ ] Hand check in the release build: the marks are level with the thumb on a long document, in light and dark, on Mica and with Backdrop None, and in a Windows high contrast theme (headings in the text color).
- [ ] `npm run check` passes.
