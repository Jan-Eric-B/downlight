# Overview ruler

Status: ready-for-agent

## Problem Statement

In a long document I lose track of where I am and where things are. The scroll bar's thumb tells me how far down I am, but not where the sections start, and when Find says "6 of 24" I can't see where the other 23 matches are: clustered in one section, or spread over the whole file. VS Code's minimap and Visual Studio's map mode answer that for code, with marks beside the scroll bar that show the file's structure and its search results at a glance and take me there with a click. Downlight has nothing like it, and a literal picture of the page would cost too much on a large document and show little, since rendered prose shrinks to grey bars.

## Solution

An **Overview ruler**: a slim strip just left of the scroll bar that marks where the document's main headings (h1 to h4), Find's matches and Find's **Current match** are. Each mark is level with the scroll bar's thumb when its block is in view. A click on a heading mark goes to that heading, as a heading link does, so Back returns. A click on a Find mark makes that match the current match. Hovering a heading mark names the heading. The ruler is on by default and can be switched off in the settings flyout. It draws nothing on the document itself, so reading, Find and every other feature work as before. This is option A of the research (`.scratch/overview-ruler/research.md`, mockups `mockups-light.png` and `mockups-dark.png`).

## User Stories

1. As a reader, I want marks beside the scroll bar where the document's main headings are, so that I can see the document's sections at a glance.
2. As a reader, I want each heading level's mark shorter than the one above it, from h1 to h4, so that I can tell the large sections from the small ones.
3. As a reader, I want no marks for h5 and h6, so that a deeply nested document doesn't fill the strip with noise.
4. As a reader, I want a mark for every match of Find, so that I can see where the matches cluster.
5. As a reader, I want the current match marked apart from the others, full width and in its own color, so that I can see where in the document the match I'm on is.
6. As a reader, I want a mark to be level with the scroll bar's thumb whenever its block is in view, so that the marks and the thumb never disagree.
7. As a reader, I want the marks to settle as the document is laid out, as the thumb does, so that I'm not shown positions that contradict the scroll bar.
8. As a reader, I want to click a heading mark to go to that heading, so that I can jump between sections without scrolling.
9. As a reader, I want Back to return me to where I was after I clicked a heading mark, so that a jump through the ruler is as safe as following a heading link.
10. As a reader, I want a click on a mark that stands for several headings to take me to the first of them, so that I land at the start of the cluster rather than in its middle.
11. As a reader, I want to click a Find mark to make the match nearest the pointer the current match, so that I can go straight to a cluster of matches.
12. As a reader, I want a click on a Find mark to show the match as F3 does, without adding a step to History, so that Find's steps stay out of Back and Forward.
13. As a reader, I want a click where there's no mark to do nothing, so that a stray click near the scroll bar doesn't move the view.
14. As a reader, I want to hover a heading mark and see the heading's text, so that I know where a mark leads before I click it.
15. As a reader, I want the hover label to list the headings that share one mark, up to about five, so that a crowded mark is still readable.
16. As a reader, I want no hover label on Find marks, so that the find bar's "N of M" stays the one place that counts matches.
17. As a reader, I want the pointer to show that a mark can be clicked, so that I find out the marks do something.
18. As a reader, I want Find's marks to appear as I type and to go when I close Find, so that the ruler shows exactly what Find shows.
19. As a reader, I want Find's marks to follow the current match as I step with Enter, F3 or the arrow buttons, so that the ruler and the document agree.
20. As a reader, I want the heading marks to stay while the ruler is on, whether or not Find is open, so that the structure is always there.
21. As a reader, I want the ruler to show the new document after a reload, Back, Forward or a new document, so that its marks never describe a document I'm not reading.
22. As a reader, I want the ruler to keep up when I zoom, switch Full width or resize the window, so that its marks stay level with the thumb after the text wraps differently.
23. As a reader, I want the ruler to follow a table's **Sort** once Sort is built, so that its marks follow the order shown.
24. As a reader, I want the ruler hidden while a **Wait note** shows for a new document, so that it never shows marks for nothing.
25. As a reader, I want the ruler of the document in view to stay during a reload until the new render replaces it, as the document does, so that a reload doesn't flicker.
26. As a reader, I want no ruler when the document doesn't scroll, so that a short document stays as clean as it is today.
27. As a reader, I want the strip to have no background of its own, so that the window's **Backdrop** shows through it as it does behind the page.
28. As a reader, I want heading marks in a muted text color, so that the document's structure doesn't compete with the **Accent color**, which marks what Downlight adds.
29. As a reader, I want Find's marks in the accent color's Find colors, so that they look like the highlights they stand for.
30. As a reader, I want a heading mark and a match mark in the same row both to show, so that neither hides the other.
31. As a reader, I want the ruler in both themes and with either accent color, so that it fits whatever I chose.
32. As a reader using a high contrast theme, I want the ruler drawn in the system's colors, headings in the text color and matches in the highlight colors, so that I can still see the marks.
33. As a reader, I want the strip the same width at any zoom, as the title bar is, so that zoom changes the document and nothing else.
34. As a reader, I want the text never to run under the strip, so that the marks never cover what I'm reading.
35. As a reader, I want the strip to sit in the margin when the window is wider than the reading width, so that it takes nothing from the text.
36. As a reader, I want switching the ruler on or off to keep my reading position, so that a settings change never loses my place.
37. As a reader, I want to switch the ruler off with an "Overview ruler" checkbox in a "View" group of the settings flyout, so that I can have the plain window back.
38. As a reader, I want the ruler on by default, so that I find it without looking for it.
39. As a reader, I want my choice kept in the **Settings** for every window, now and after a restart, so that I choose it once.
40. As a reader, I want a change of the switch in one window to reach every open window, so that all windows look alike.
41. As a reader, I want no title bar button, shortcut or right-click entry for the ruler, so that a set-once choice doesn't crowd the commands I use while reading.
42. As a keyboard reader, I want the strip never to take the focus, so that Tab moves through the document and the title bar as before.
43. As a screen reader user, I want the strip hidden from my screen reader, so that it doesn't read out a list of marks I reach better through Find's "N of M" and the headings themselves.
44. As a reader of a large document, I want the document to appear as soon as it does today, with the ruler arriving after it, so that the ruler never delays **First paint**.
45. As a reader of a large document, I want heading marks before Find's, so that the structure is there first.
46. As a reader of a large document, I want typing in Find and scrolling to stay as responsive as today, so that the ruler costs nothing I can feel.
47. As a reader of a 100 MB document, I want the ruler still to show where matches cluster, even though many marks merge into one row, so that it stays useful at any size.
48. As a reader, I want the ruler to make no network request and read nothing outside the shown document, so that it adds nothing to what Downlight touches.

## Implementation Decisions

- **Page only, plus one setting.** The ruler is a new page module, started at the page's startup after Find, that owns one strip element. The app gains one setting; the core doesn't change and no dependency is added.
- **Where a mark sits (ADR 0003, amended).** A mark's height in the strip is its block's top in the scroller, scaled by the strip's height over the scroller's scroll height, as the scroll bar scales. Only top-level blocks of the document view are measured: they are size-contained, so their rectangles are the browser's estimate or remembered size and never force a skipped block's layout. A heading or a Find match inside a block takes its top-level block's top, and a match adds its share of that block's text times the block's height. No rectangle inside a top-level block is ever read. Marks therefore agree with the scroll bar at every moment and settle as the thumb does. Positions from line numbers and laying out every block in the background were rejected (research, section 3).
- **Marks as page elements, not a canvas.** The strip holds one element per merged run: per pixel row and lane at most, so about a thousand at the most for a tall window, however many matches there are. A pure placement step turns the blocks' tops and heights, the marks, the scroll height and the strip's height into those runs, merging marks that fall in the same row. Elements take their colors from CSS (the accent's Find match and current match roles, a muted text color for headings), so theme and accent changes need no redraw, and Windows' high contrast mode recolors them through forced colors with the system colors Find's highlights already use there. If the measurement shows a redraw over 100 ms on the 1 MB test document, the ruler switches to a canvas behind a painter interface (canvas in the app, a recorder in the tests), and reads its colors from computed style.
- **Look (mockup A).** About 14 px wide, at any zoom. No background of its own. Heading marks start at the strip's left edge in four lengths: h1 the full width (about 14 px), h2 about 11 px, h3 about 8 px, h4 about 5 px. Find's matches are short marks in the right half. The current match is the full width and a little thicker. A heading mark and a match mark in the same row both show. The pointer is a hand over a mark.
- **Placement.** The strip sits just left of the scroller's native scroll bar, at the scroller's full height (its width is the scroller's offset width minus its client width). The text never runs under it: in a window wider than the reading width it lies in the margin, and otherwise over the document's right padding (32 px at 100% zoom); only where that padding is narrower than the strip (at low zoom) does the view give up the difference. The ruler switches inside `keepReadingPosition`, so the reading position holds either way.
- **When it shows.** While the setting is on, a document is shown and the scroller overflows. Hidden in the empty window, while a Wait note shows for a new document, and when the document doesn't scroll. During a reload, the ruler of the document in view stays until the new render replaces the document.
- **Which marks.** Headings h1 to h4 that have an anchor id, wherever they sit (in lists, quotes and collapsed sections too, placed by their top-level block). A raw HTML heading without an id gets no mark, since a click couldn't name it. Find's matches and the current match while the find bar has matches.
- **Find's interface grows.** Find gains a listener for when its matches or its current match change (a recount as the reader types, a step, a reload's recount, the bar closing), and a way to make a given match the current match and scroll it into view as a step does, without joining History. `findMatches()` stays the read side. Find keeps owning the current match.
- **Clicks.** A click within a few pixels of a mark acts on the nearest mark; heading marks win over match marks in the same row only when the click is in the heading lane (the left half), match marks in the right half. A heading mark goes through the view's `visitHeading` with the first heading of its run, so it opens collapsed sections, scrolls as a heading link does and joins **History**. A match mark makes the match nearest the pointer's row current through Find. A click elsewhere on the strip does nothing.
- **Hover label.** Over a heading mark, a label left of the strip in the link target label's style, with the run's headings' text, up to five and then "and N more". None over match marks or empty strip.
- **Redraws**, coalesced to at most one per frame: when a document is shown (shown, reload, return), when Find's listener fires, when the article's or the scroller's size changes (blocks laid out for the first time, zoom, Full width, window resize; a resize observer), when the setting changes, and when Sort reorders rows once Sort exists. Reading the tops of the blocks that carry marks happens in one pass after layout is clean, with no writes in between.
- **Large documents.** Nothing runs in the frame that puts a document in: the ruler builds its list of headings and its block index after first paint, in slices, as Find builds its index, heading marks first. No size turns it off unless the measurement forces one. It doesn't wait for v1 ticket 39.
- **The setting.** The settings store gains the ruler's switch, on by default, kept across a restart and sent to every page in the settings snapshot (`PageSettings` gains a boolean). The setting change is set, not switched, like the flyout's other choices (`SettingChange` gains a variant carrying the new value). Contract samples are written for both.
- **The flyout.** A new "View" group after Accent color and before Edit with, holding one checkbox, "Overview ruler". It is the flyout's first checkbox; its focus order follows the browser's, as the radio groups' does.
- **Accessibility.** The strip is `aria-hidden` and takes no focus. Everything on it is reachable otherwise: Find's "N of M", F3 and Enter, and the document's headings and links.
- **Glossary.** **Overview ruler** and **Settings** are updated (done in the grilling). No new term.

## Testing Decisions

- **A good test** drives the page as the reader does and checks what the reader can observe: which marks the strip shows, of what kind and at what height relative to the strip; where a click scrolls; which match becomes current; what the hover label says; whether Back returns. It never calls the placement step or reads the ruler's internal state. Names state the behavior, such as `a click on a heading mark goes there and Back returns`.
- **Seam 1, the page** (Vitest with happy-dom), through the existing fakes: `fakeTheApp` and settings messages for the switch, typing in the find bar for Find's marks, `layOut` for geometry, click and pointer events on the strip, and the History helpers for Back. `layOut` is extended to fake the scroller's scroll height and client height, which the ruler scales by. Cases include: heading marks at the right heights for each level and none for h5 and h6; marks merging in one row; Find's marks appearing, following the current match and going when Find closes; a heading mark click joining History; a match mark click making the nearest match current without joining History; a click on empty strip doing nothing; the hover label listing several headings; the strip hidden in the empty window, while a Wait note shows, and when the document doesn't scroll; the ruler following a reload and Back; switching the setting on and off; the flyout's checkbox showing and sending the setting. The accent test that lists every use of the accent roles in the stylesheets gains the ruler's uses.
- **Seam 2, the app** (`cargo test`): the settings store keeps the ruler's switch across a restart and defaults to on when it is missing; the contract samples for the new setting change and the settings field, which the page's contract test drives through the page.
- **Prior art:** Find's page tests (typing, steps, the current match against `layOut`), Full width's and the settings flyout's page tests (a setting switched and shown), the History tests (Back after a heading link), and the app's settings tests.
- **Measurement, not a test:** with the timing switch on the 1 MB test document, typing in Find (a one-letter search with tens of thousands of matches) and scrolling the whole document keep every page task and frame under 100 ms, and the frame after the document goes in is no longer than today (ticket 02's baseline). Then the same on the 20 and 100 MB documents (`npm run measure -- large`), noting when the ruler first shows. The results go in the ticket.

## Out of Scope

- **Block shading by kind** (option B): a later, optional layer of the ruler, decided after this is built and measured, probably off in high contrast.
- **Marks for task items**, open or ticked: Later.
- **Marks for h5 and h6** and for the reading position (the thumb shows it).
- **Heading labels always visible** (option C) and **a scaled picture of the page** (option D): the first is the Outline (`.scratch/outline/spec.md`), the second was rejected in the research.
- **Replacing the native scroll bar**, or marks drawn over its track.
- **A title bar button, shortcut or right-click entry** for the ruler.
- **Keyboard access to the strip itself.**
- **Changes to the core** or to how documents are rendered.

## Further Notes

- Decided in the grilling of 2026-11-20, from the research and mockups in this folder. The v1 spec's Later list and ADR 0003's consequence were updated in the same session.
- The list of headings is shared with the Outline (`.scratch/outline/spec.md`). The ruler is built first, so it builds that list in a form the Outline can reuse.
- Choosing page elements over a canvas was decided when the test seams were checked: it needs no painter seam, and forced colors and the accent's CSS roles color it for free. The measurement can reverse it.
- The research's point 6 (amending ADR 0003) is done; the drift of marks as blocks are first laid out is accepted, as the thumb's is.
- h4 joined the marks in the Outline's grilling (2026-11-20), so the ruler, the Outline and the heading jump keys share one list of headings, h1 to h4.
