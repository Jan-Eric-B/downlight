# 01: Heading marks beside the scroll bar

**What to build:** The **Overview ruler**'s strip, just left of the scroll bar, with a mark for each of the shown document's h1 to h4 headings, level with the scroll bar's thumb when the heading's block is in view. It is always on for now (ticket 02 adds the switch). The reader sees the document's sections at a glance; clicking comes in ticket 04.

Spec: [Overview ruler](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] Prefactor first: the page tests' layout fake also fakes the scroller's scroll height and client height, which the ruler scales by, without changing what existing tests see.
- [x] A new page module, started at the page's startup after Find, owns the strip: about 14 px wide at any zoom, the scroller's full height, no background of its own, `aria-hidden` and never focused.
- [x] Marks are page elements (spec, "Marks as page elements"), placed from the top-level blocks' tops only, scaled by the strip's height over the scroll height, merged per pixel row; no rectangle inside a top-level block is read (ADR 0003).
- [x] Headings h1 to h4 with an anchor id get a mark, wherever they sit (placed by their top-level block); h1 the strip's full width, each level below shorter (h4 shortest), from its left edge, in a muted text color. h5, h6 and raw HTML headings without an id get none.
- [x] The strip is hidden in the empty window, while a **Wait note** shows for a new document, and when the document doesn't scroll. During a reload, the old ruler stays until the new render replaces the document.
- [x] It redraws, at most once per frame, when a document is shown (shown, reload, return), and when the article's or the scroller's size changes (blocks laid out for the first time, zoom, Full width, window resize).
- [x] The text never runs under the strip: in the margin in a wide window, otherwise over the document's right padding, which is wider than the strip at every zoom (see Decisions), so the view gives up nothing.
- [x] Page tests (written first): marks at the right heights for each level, none for h5 and h6; marks in one row merge; the strip hidden in the empty window, during a Wait note and for a document that doesn't scroll; the marks follow a reload and Back.
- [ ] Hand check in the release build: the marks are level with the thumb on a long document, in light and dark, on Mica and with Backdrop None, and in a Windows high contrast theme (headings in the text color). Also whether the marks near the top and bottom drift from the thumb because the scroll bar's arrow buttons shorten its track (the strip scales by the scroller's client height). For the owner.
- [x] `npm run check` passes.

## Decisions

- **The shared list of headings** is its own page module, `page/src/headings.ts`, started before the ruler: `documentHeadings()` gives the list (`headings`, each with `id`, `level`, `text` and `element`, in document order, `complete`, and `describesView()`), and `whenHeadingsChange(listener)` hears it each time it is rebuilt (shown, reload, return), grows or completes. The Outline and its jump keys read it as the ruler does. No glossary entry: the spec's grilling settled that the ruler adds no new term, and "heading" is the document's own word.
- **The list is built after first paint.** When a document is shown, its list starts empty and incomplete, and is built in the task after the next frame (`afterNextFrame`), as Find defers its index, so the build never runs in the frame that puts the document in (spec, "Large documents"). It is built at once for now, so it is complete when it is built; ticket 05 can build it in slices behind the same interface. The build is asked for before the listeners hear the empty list, so a frame they ask for then comes after the build, and the ruler draws the new list in the first frame it can. The empty window's list is empty and complete at once.
- **Whether the list is of the content in view** is the list's own answer, `describesView()`, so every reader shares it: a reload's or another document's render is in the view a landing before the list is rebuilt, and an empty list not yet built is of no content. It compares the view's first node with the one it had when the list was built, since the view's content is only ever replaced as a whole.
- **The rendered fixture** `core/tests/fixtures/headings.md` is added here, as the Outline's ticket 01 asks: h1 to h6, two headings with the same text, a heading in a list, a quote and a closed `details`, a raw HTML heading without an id, a heading with inline code and a link, and a long heading. Outline ticket 01 can use it as it is.
- **The layout fake** fakes the scroller's scroll top too, as the line at the top of the view gives it, so a block's top in the scroller's content (its rectangle's top less the scroller's, plus the scroll top) is the same wherever the view is scrolled. The scroll height runs to the end of the view's last block, at least the client height. No page code read any of the three before, so the existing tests see nothing new. `layOut` can also put px of content above the view, as the "File deleted or moved" bar does, and the fake `ResizeObserver` (`fakeTitleBarLayout`) can report for given elements only, so a test can tell which sizes the ruler watches; called without any, every observer reports, as before.
- **The wait note** tells its own listeners when it shows or hides (`whenWaitNoteChanges` in `wait-note.ts`), so the ruler hides while it shows and comes back after Stop, which replaces no content and so runs no `whenDocumentShown` listener.
- **Whether a document is shown** is the view's `showsDocument()`, read at each redraw, which changes when the content is replaced rather than when the `whenDocumentShown` listeners hear of it.
- **When a mark is placed:** a redraw asked for by a document shown, the list or the wait note waits for the frame after it was asked for (`afterNextFrame`), so it reads every block's top with layout clean, then writes the marks; asks before it share it. A size change redraws in the ResizeObserver's callback instead, within the frame whose layout changed the size, where layout is clean too, so the marks move in that frame's paint rather than two frames later (the callback's frame, then `afterNextFrame`'s frame and task); it stands for a redraw already waiting, so a frame still gets at most one. Its row is the block's top times the strip's height over the scroll height, rounded down. In one row, the highest heading's length stands for all.
- **What the ruler watches for sizes:** the scroller and each of its children (the "File deleted or moved" bar, the wait note, the view and the empty window). The bar is in flow above the view, so showing it moves the view down and grows the scroll height without resizing the view or the scroller.
- **During a reload**, a redraw whose list doesn't describe the view (`describesView()` is false: the new render is in, its list comes after it has landed) leaves the old marks as they are, and the redraw waiting for the next frame still comes.
- **The strip takes no pointer** until ticket 04 makes its marks clickable, so the wheel over it still scrolls the document. It lies in the scroller, after what it holds (`z-index: 0`, `position: fixed`, so it neither scrolls nor adds to the scroll height), so the bar and the wait note (`z-index: 1` in the scroller) lie over it, and so does everything the page lays over the scroller: the selection's counts, the link target, notices and the find bar. A click on a mark in ticket 04 still reaches the strip, which lies over the document's blocks.
- **Mark sizes:** 2 px high; h1 14 px long, h2 11 px, h3 8 px, h4 5 px, in `--fgColor-muted` (the strip is a `markdown-body` for github-markdown-css's colors), `CanvasText` under forced colors.
- **The text never runs under the strip** without any rule of the ruler's: the strip lies in the margin of a wide window, or else over the document's right padding, 32 px zoomed, which is 16 px at the smallest zoom (50 %, `SMALLEST` in `app/src/zoom.rs`) and so always wider than the 14 px strip. Neither github-markdown-css nor `document.css` narrows that padding in a narrow window (github-markdown-css has no media query for it), so the scroller padding the spec allows for, `max(0px, 14px - 32px * zoom)`, could never apply and was left out. A smaller zoom or a narrower padding would need it.
- **Strip height** is the scroller's client height (`clientHeight`), set by the script, so a horizontal scroll bar never puts marks beside it. It scales by that height, as the spec says, not by the thumb's track less the scroll bar's arrow buttons, which the page can't measure; the hand check shows whether the marks drift from the thumb near the ends of a long document.
- **Adjusted typing target, accepted by the owner (2026-11-26).** The spec's measurement ("typing in Find ... keep every page task and frame under 100 ms") can't be met by the ruler, since the frame over 100 ms is Find's first count of a one-letter search on 1 MB mixed, 80 to 125 ms with the ruler and without it (Results, "diagnosed"). Proposed for the ruler instead: while the reader types in Find, the ruler runs no redraw and reads no rectangle as long as nothing it watches changes size (until ticket 03 marks the matches, whose redraw is measured there), and the longest task or frame while typing on the 1 MB test documents is no longer with the ruler than without it, within the spread between runs. Find's own frame of a count with about 100,000 matches gets a ticket of its own (building its highlight in slices, or highlighting only the matches near the view), with the 100 ms target. The rest of the spec's measurement (scrolling, the frame after the document goes in) stands as it is.

## Results

**2026-11-26, measured** on the work machine with `npm run tauri build` and `npm run measure` (release build, median of 5 runs per case, each in a new process), against ticket 02's baseline. To tell the ruler's cost from the machine's, the same measurement ran four times: as built (the ruler started, redraws on a size change within the ResizeObserver's callback, the strip in the scroller), with that redraw deferred to the next frame, with the strip on the page's body instead of in the scroller, and without the ruler and the list of headings started. Points in time are ms since the process started.

**First paint** (ticket 02's baseline, then as built, then without the ruler):

| Stage | 100 KB | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: | ---: |
| First paint, baseline | 771 | 767 | 699 |
| First paint, as built | 947 | 1109 | 821 |
| First paint, without the ruler | 721 | 1163 | 899 |
| The HTML is inserted, baseline | 701 | 721 | 671 |
| The HTML is inserted, as built | 908 | 1018 | 781 |
| The HTML is inserted, without the ruler | 682 | 1086 | 847 |
| Insert to first paint, baseline | 70 | 46 | 28 |
| Insert to first paint, as built | 39 | 91 | 41 |
| Insert to first paint, without the ruler | 39 | 78 | 52 |
| Insert to first paint, redraw deferred | 39 | 79 | 42 |
| Insert to first paint, strip on the body | 40 | 82 | 46 |

- **The ruler doesn't hold up first paint.** From the insert to first paint the page takes about as long with the ruler as without it, in every variant; what is left of the ruler in that frame is a redraw that finds the list not yet built and keeps the strip as it is. The list is built, and the marks drawn, in the task after that frame.
- **First paint as a whole is slower than ticket 02's**, with or without the ruler, because read and render now take 465 to 503 ms for 100 KB and 835 to 926 ms for 1 MB mixed (2.2 and 17.6 ms in ticket 02, before highlighting and the rest of rendering). Read and render start as the window opens and run while the webview starts, so for 1 MB mixed they now end after the page is ready and set first paint. That is the core's, not the ruler's, and is outside this ticket. The runs also moved by up to about 200 ms between sessions with the machine's load (100 KB: 947 ms as built, 717 to 732 ms in the other three).
- **Scrolling** through both 1 MB documents: no task or frame over 100 ms in the median in any variant (worst single run 103 ms with the strip on the body).

**Typing in Find on 1 MB mixed goes over 100 ms with the ruler** (the measurement's "e", "the", "lamplight", "qxzjv", the longest task or frame while typing):

| Variant | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: |
| Without the ruler | 0 (worst 0) | 0 (worst 0) |
| As built | 103 (worst 146) | 0 (worst 0) |
| Redraw deferred to the next frame | 120 (worst 123) | 0 (worst 104) |
| Strip on the body | 114 (worst 140) | 0 (worst 0) |

The 1 MB mixed document has 578 headings, the prose one none. The first reading of this table was that Find's scroll laid out new blocks and the ruler's redraw, reading every heading's block, made the frame long. The diagnosis below shows otherwise.

**2026-11-26, diagnosed:** the long frame is Find's own, and the ruler does no work in it. Measured on the same release build, which for the diagnosis started the ruler and the list of headings or not by a temporary switch, so both ran in one session; the measurement typed in Find on 1 MB mixed only and recorded each long frame's scripts (the Long Animation Frames API), the ruler's redraws and how long Find took to build its highlights. The switch and the probes are removed again.

- **The ruler doesn't redraw while the reader types:** no redraw and no rectangle read in any of the 18 keystrokes of any run (one redraw follows a jump to the end, so the probe sees them). The scrolls to the matches resize nothing the ruler watches.
- **The long frame is the first count of a one-letter search:** "e" (105,831 matches), or the "t" of "the" (49,487) after it. It is one task, Find's last slice of the index and the count, of 49 to 150 ms, then 25 to 43 ms of style, layout and paint for the highlights. The task forces no layout (0 to 2 ms). Most of the task is building the matches' highlight, one `StaticRange` per match: 32 to 125 ms for "e".
- **It goes over 100 ms without the ruler too.** The longest frame while typing, run by run: with the ruler 114, 122, 119, 140, 120 and, later, 117, 119, 113, 106, 106; without it 86, 94, 80, 91, 113 and, later, 106, 121, 102, 94, 105. The "without the ruler" row above, 0 (worst 0), was a lucky session.
- **Garbage collection decides whether it crosses 100 ms.** Before the reader types, the page still holds the garbage the document's arrival left (about 3 MB of script heap, 2.6 MB of array buffers and 9 MB of the webview's own heap) unless the engine has collected it by then, with or without the ruler alike. While it is there, building the highlight for "e" takes 47 to 67 ms; once it was collected, 36 to 40 ms, and with a collection forced before typing the frame took 92 to 96 ms with the ruler on. Where the redraw ran and where the strip lay never mattered, as the table shows, because no redraw ran.

So typing in Find on 1 MB mixed has a frame of 80 to 125 ms (once 191) with or without the ruler, from Find's one-letter count, and the ruler adds nothing to it. Under 100 ms needs a change to Find, not to the ruler: building the highlight of a large count in slices over several tasks, or highlighting only the matches near the view. The owner accepted an adjusted target for the ruler (Decisions); Find's frame is v1 ticket 40.
