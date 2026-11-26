# 05: Large documents, measured

**What to build:** The **Overview ruler** costs the reader nothing they can feel on any document Downlight shows: the document appears as soon as it does today, and typing in **Find** and scrolling stay as responsive. On a large document the heading marks arrive first, and at 100 MB the ruler still shows where matches cluster.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 03 (Find's marks, and clicking them)

**Status:** closed by the owner, 2026-11-26, who tested the large documents and accepted the open items below as they stand (see "Open").

- [x] Nothing of the ruler runs in the frame that puts a document in: it builds its list of headings and its block index after **First paint**, in slices as Find builds its index, heading marks first. (The list of headings is built a slice of blocks per task; the "block index" is the per-render cache of text offsets, filled by counts rather than after first paint: see Decisions.)
- [ ] Measured with the timing switch on the 1 MB test document: scrolling the whole document keeps every page task and frame under 100 ms, typing a one-letter search (tens of thousands of matches) makes the longest task or frame no longer with the ruler than without it, within the spread between runs (the spec's amended target; Find's own count is v1/40's), and the frame after the document goes in is no longer than ticket 02's baseline (v1). Scrolling and the frame after the document are met; **typing is not** (open, see Results).
- [x] First, the open item from ticket 03's Results: typing in Find on 1 MB mixed had a longest frame of 120 to 193 ms with the ruler on against 0 to 126 with it off. Probe it (for example with the Long Animation Frames API) to tell whether the ruler's redraw after a count now lands in Find's frame, and fix it. Ticket 03 also leaves this ticket the per-render cache of each text node's offset in its block, so a count places its matches without walking the document. (Probed: the redraw never lands in Find's frame, so there is no redraw to move; what makes the frame longer is a garbage collection, see Results. The cache is built.)
- [x] From ticket 04: the hand over a mark is the scroller's `cursor`, an inherited property, so each time the pointer comes onto or leaves a mark the scroller's content may be restyled. Measure moving the pointer along the strip of the 1 MB mixed document (578 headings); if a frame goes over 100 ms, bound it (for example `cursor: auto` on the view's top-level blocks, checked against the trace) or move the hand to an element of its own. (No frame over 100 ms, nor over 50 ms: nothing to bound.)
- [x] Measured on the 20 and 100 MB documents (`npm run measure -- large`), noting when the ruler first shows. (Measured; it shows 30 to 136 ms after first paint, but its redraws as the list grows make long frames on 20 and 100 MB: open, see Results.)
- [x] If a redraw goes over 100 ms on the 1 MB document, the ruler draws on a canvas behind a painter interface (canvas in the app, a recorder in the tests), reading its colors from computed style and the system colors under forced colors; the page tests then read the recorder. (Not needed: the ruler's longest redraw on 1 MB is 22 to 33 ms of script, in a frame of 55 to 78 ms.)
- [x] The results go in this ticket's comments.
- [ ] `npm run check` passes. (Not run: the owner stopped the work before it. `npx vitest run` passes.)

## Decisions

- **The list of headings in slices** (`headings.ts`): a slice is 2,048 of the view's top-level blocks (`SLICE_BLOCKS`), each a heading itself or searched for nested headings (lists, quotes, closed sections), so no single `querySelectorAll` walks a large document in one task. The first slice is listed in the task after the document's first frame, as before, and each further slice in a task of its own (`setTimeout`), as Find's index slices are. Each slice gives a new list, with a new `headings` array, which its listeners hear, `complete` only on the last; a rebuild (another document, a reload, Back) drops the slices of the one before. A count of blocks rather than a time budget, so that a test's document and a measured run slice alike. On 1 MB mixed (about 4,000 top-level blocks) that is 2 slices of 2 to 3 ms each.
- **Heading marks first.** The ruler draws from each slice as it comes, so its heading marks show from the first slice on, before the list is complete and before any of Find's marks. No change to the ruler was needed for that: it already redraws when it hears the list change.
- **The per-render cache of text offsets** (`overview-ruler-placement.ts`): how much of its top-level block's text comes before each text node, and each block's text length, so a count looks its matches up rather than walking every block with a match through a `TreeWalker` again. It is filled by the first count that has a match in a block, not built after first paint: built ahead it would hold an entry per text node for every document, whether or not the reader ever uses Find, and the measurements below show that the ruler's resident memory is exactly what costs Find's frame. Plain `Map`s rather than `WeakMap`s, dropped once a redraw finds another render in the view: a weak map's entries are ephemerons, which every full collection goes through again, and the traces below found ephemeron marking in the full collections that land in Find's frame. A text node the block didn't have when it was walked has the block walked again. A **Sort** reorders a table's rows without new text nodes, so the table's offsets would then be out of date: table-sorting ticket 04, which makes the ruler follow the shown order, has to drop that block's entry.
- **Measuring tool** (`scripts/measure.js`): a lasting case moves the pointer down the strip of 1 MB mixed 2 px at a time (`movePointerAlongStrip`), each move once the last one's frame is done, and reports each move's time to the end of its frame and any long task or frame. `npm run measure -- large` now also watches over the debug port for the strip to show (`rulerShows`, giving up at once when the window's settings have the ruler off) and reports `rulerAfterFirstPaint` and `longestAfterFirstPaint`.
- **No canvas.** The spec's switch to a canvas is for a redraw over 100 ms on the 1 MB document, and none is.

## Results

**2026-11-26, measured** on the work machine with the release build and `npm run measure` (median of 5 runs, each in a new process), the ruler on (the settings file without `overviewRuler`) and off (`"overviewRuler": false` added for the session), the settings file restored byte for byte after each session. Earlier in the day, probe builds added temporary marks around Find's count and highlight, the ruler's redraw, the placing of matches and the list's slices, and the measuring script recorded every animation frame of 50 ms or more with its scripts (the Long Animation Frames API) and, in some sessions, the renderer's garbage collections (DevTools tracing). The probes are removed again.

### Typing in Find on 1 MB mixed: the ruler's redraw is not in Find's frame, a garbage collection is

- **In every run, the longest frame while typing is Find's own task**, its last index slice and its count for "e" (105,831 matches): 84 to 124 ms with the ruler on, of which 74 to 102 ms is building the highlight (one `StaticRange` per match), then 28 to 38 ms of rendering, 136 to 176 ms in all. The ruler's redraw for those matches always comes in the next frame: 22 to 33 ms of script (placing the matches, 10 to 15 ms of it), a frame of 55 to 78 ms. Nothing of the ruler runs in Find's frame.
- **With the ruler off, Find builds the same highlight in 34 to 50 ms** (once 80), so its frame stays near 100 ms: 0 in 2 of 5 runs, 102 to 132 in the others.
- **A forced garbage collection before typing removes the difference.** With a full collection through DevTools just before the first keystroke, the highlight takes 32 to 40 ms with the ruler on and 36 to 42 ms off, and the ruler-on runs had no frame over 100 ms at all (0 in all 5). Waiting 3 s before typing instead changes nothing (ruler on: highlight 63 to 111 ms, longest frame 110 to 188), so it isn't work still running after first paint.
- **The traces name the collection.** With the ruler on, a full mark-compact collection of 12 to 25 ms is finalized inside Find's count task in 17 of 18 traced runs (three sessions, warm-up runs included), and its incremental marking adds 3 to 5 ms more; with the ruler off, the incremental marking starts in the same task but finishes after it, in all 6 traced runs. The one ruler-on run without the collection in the task had no frame over 100 ms.
- So the ruler adds no work to Find's frame, but its resident memory (the list of headings, the strip's mark elements, the placed marks: about 0.2 MB of script heap kept, 0.4 MB before a collection) leaves less room under the engine's limit before Find's burst of about 105,000 highlight ranges, which then reaches it inside the task. Nothing the ruler runs can move that collection out; making Find's highlight smaller per task (v1 ticket 40: building it in slices, or only near the view) would.

**The final session** (clean release build: the list in slices and the cache, no probes), ruler on and off in one sitting:

| Measure, 1 MB | ruler on | ruler off |
| --- | ---: | ---: |
| First paint, 100 KB / mixed / prose | 973 / 1076 / 668 | 923 / 1076 / 738 |
| Insert to first paint, 100 KB / mixed / prose | 56 / 87 / 47 | 48 / 85 / 42 |
| longestAfterDocument, all three | 0 (worst 0) | 0 (worst 0) |
| Scrolling, mixed and prose, longest | 0 (worst 0) | 0 (worst 121) |
| Typing, keystrokeToCount, mixed | 15.2 | 15.1 |
| Typing, longestWhileTyping, mixed | 104.8 (worst 128.3) | 0.0 (worst 101.5) |
| Typing, longestWhileTyping, prose | 0 (worst 0) | 0 (worst 0) |
| Held F3, f3ToTwoFrames / longest step | 32.5 / 51.8 (worst 69.2) | 67.8 / 97.8 (worst 101.0) |
| Held F3, longestWhileStepping | 0 (worst 0) | 0 (worst 0) |
| Pointer along the strip, per move / longest move | 8.4 / 31.7 (worst 34.2) | 0.8 / 9.5 (worst 19.7) |
| Pointer along the strip, longestWhilePointing | 0 (worst 0) | 0 (worst 0) |

- **Typing on 1 MB mixed, run by run:** with the ruler on 104, 128, 103, 122, 105; off 102, 0, 0, 0, 0. The target ("no longer with the ruler than without it, within the spread between runs") is **not met**, for the reason above.
- **The frame after the document goes in** has no task or frame over 100 ms with the ruler on or off, and the insert to first paint is the same either way (87 against 85 ms on 1 MB mixed). Both are above ticket 02's baseline (46 ms on 1 MB mixed) with or without the ruler, as ticket 01's Results found: that is the rest of the page and the core since ticket 02, not the ruler.
- **Scrolling** through both 1 MB documents: no task or frame over 100 ms with the ruler on.
- **Pointer along the strip** (the hand and the hover label coming and going at 578 headings): no frame over 50 ms in any run, so the inherited `cursor` needs no bound. Each move's frame is done in 8 to 10 ms with the ruler on against under 1 ms off: the hand's and the label's restyle, far from 100 ms.
- **Placing a later count** with the cache: the "t" after "e" (49,487 matches) places in 5.6 to 7.4 ms in 4 of 5 probed runs (16.5 in one), against 10 to 15 ms with the walk; the first count, which fills the cache, about as before (9.6 to 12.2 ms against 12 to 15).
- **Held F3** was faster with the ruler on than off again (32.5 against 67.8 ms a step), as ticket 03 found.

### Large documents (`npm run measure -- large`, median of 3 runs above 1 MB)

| Measure | 1 MB mixed | 5 MB mixed | 20 MB mixed | 100 MB mixed | 1 MB prose | 5 MB prose | 20 MB prose | 100 MB prose |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| First paint, ruler on | 1246 | 2931 | 9175 | 43414 | 713 | 976 | 2330 | 16254 |
| First paint, ruler off | 1187 | 2978 | 7361 | failed | 1200 | 1367 | 2576 | 27237 |
| Ruler shows, ms after first paint | 39 | 62 | 32 | 116 | 33 | 29 | 113 | 136 |
| Longest task or frame from first paint on, ruler on | 0 | 0 | 299 (worst 305) | 1350 (worst 1360) | 0 | 0 | 110 | 735 (worst 740) |
| Longest task or frame from first paint on, ruler off | 0 | 0 | 0 (worst 156) | failed | 0 | 0 | 132 | 405 (worst 424) |
| Memory, private MB, ruler on | 330 | 476 | 1098 | 4400 | 204 | 275 | 605 | 2280 |
| Memory, private MB, ruler off | 325 | 486 | 1093 | failed | 204 | 272 | 587 | 2043 |

- **The ruler shows within 30 to 136 ms of first paint** at every size, with the heading marks of the list's first slice.
- **100 MB mixed failed with the ruler off** (no first paint within 180 s in the session's runs); with it on it showed at 43 s. The machine was slow that session (100 MB prose's first paint 16 to 27 s against 13 s in ticket 31), so the two sessions' first paints differ by more than the ruler could cause.
- **The ruler makes long frames after first paint on the large documents with many headings.** On 20 MB mixed (11,522 headings) the longest frame from first paint on is 299 ms with the ruler against 0 (worst 156) without. A probe build on 20 MB mixed found why: the ruler redraws for each slice of the growing list (about 40 slices there), and each of those frames takes 140 to 350 ms, of which the ruler's script is only 6 to 26 ms and forces no layout (Long Animation Frames: `forcedStyleAndLayoutDuration` 0); the rest is the rendering after the redraw has replaced the strip's marks. With the ruler off no frame follows. Which rendering phase grows with the document (style, layout, pre-paint or paint after a change to the strip, which lies in the scroller) was the next probe, a trace of those frames, which was stopped before it ran.

### Open, accepted by the owner (2026-11-26)

The owner closed the ticket with these left as they are: the typing frame waits for v1 ticket 40, and the long frames as the list grows were accepted after the owner tried the large documents.

- **Typing on 1 MB mixed** stays longer with the ruler than without it (above). The ruler can't move the collection out of Find's task; v1 ticket 40's change to Find's highlight would remove the burst that meets it. Whether to accept the ruler's share until then is the owner's call.
- **Long frames as the list grows on 20 and 100 MB.** Next steps: trace one of those frames to see which rendering phase grows with the document; then redraw the heading marks less often while the list grows (for example once for the first slice and once when complete, or at most every few hundred ms), and, depending on the trace, change the strip's marks in place rather than replacing them, or take the strip out of the scroller's layout. Each redraw frame would still cost the rendering once.
- **`npm run check`** passes (run when the ticket was closed). The release build in `target/release` is the last probe build (20 MB probes in the page), not one from the clean tree: rebuild before using it.
