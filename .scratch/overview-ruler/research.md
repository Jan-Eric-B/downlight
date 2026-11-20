# Research: a VS Code-style map beside the scroll bar, for a rendered document

**Question (repo owner):** VS Code has a scrollbar/minimap "map mode" that shows a summarized picture of the content beside the scroll bar. Could Downlight show something like it for a rendered document, what would be possible, and how?

**Short answer:** Yes, but not as VS Code draws it. The part that carries over well is the **Overview ruler** the glossary already names: a thin canvas beside the scroll bar with marks for headings, Find's matches and the current match, placed from the same layout the scroll bar uses. A shading of the document's blocks by kind (code, tables, images) could be a later layer of the same ruler. A scaled picture of the real rendered page, the literal minimap, is a poor fit: it doubles the page's DOM and layout, fights ADR 0003, can't scale to 100 MB, and shows little that prose doesn't already look like (grey bars).

Researched 2026-11-20 from the VS Code and Visual Studio docs and source, MDN and its browser-compat-data, Chromium's source, and this repo. Nothing was built or measured; every number about Downlight comes from its tickets.

## 1. What VS Code (and Visual Studio) actually offer

There are three different things behind "a map by the scroll bar", and "map mode" is the name of the third, which is Visual Studio's, not VS Code's.

### VS Code's minimap

A column of its own beside the editor's scroll bar, showing "a high-level overview of your source code" ([VS Code docs: User interface](https://code.visualstudio.com/docs/getstarted/userinterface)). It first shipped in 1.10 ([release notes](https://code.visualstudio.com/updates/v1_10)). Its settings today ([`editorOptions.ts`](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/config/editorOptions.ts), `EditorMinimap`):

| Setting | Values (default first) | What it does |
| --- | --- | --- |
| `editor.minimap.enabled` | `true` | shows the minimap |
| `editor.minimap.autohide` | `none`, `mouseover`, `scroll` | hides it until the pointer is over it or the editor scrolls |
| `editor.minimap.side` | `right`, `left` | |
| `editor.minimap.size` | `proportional`, `fill`, `fit` | proportional: a fixed height per line, so a long file's minimap scrolls itself; fill/fit: the whole file in the editor's height, sampling lines when there are too many ([1.43 notes](https://code.visualstudio.com/updates/v1_43)) |
| `editor.minimap.showSlider` | `mouseover`, `always` | the translucent box that marks the view |
| `editor.minimap.renderCharacters` | `true` | "Render the actual characters on a line as opposed to color blocks" |
| `editor.minimap.maxColumn` | `120` | width limit in columns |
| `editor.minimap.scale` | `1`, `2`, `3` | |
| `editor.minimap.showRegionSectionHeaders`, `showMarkSectionHeaders` | `true` | labels for folding regions (`//#region`) and `MARK:` comments, since 1.88 ([notes](https://code.visualstudio.com/updates/v1_88)) |
| `editor.minimap.markSectionHeaderRegex`, `sectionHeaderFontSize` (9), `sectionHeaderLetterSpacing` (1) | | the label's pattern (1.98) and look (1.90) |

How it is drawn ([`minimap.ts`](https://github.com/microsoft/vscode/blob/main/src/vs/editor/browser/viewParts/minimap/minimap.ts)):

- Two stacked canvases in device pixels: one for the text, one for selections, Find matches and other decorations, and a DOM slider on top.
- The text is rasterized per line into an `ImageData` double buffer: each character becomes a tiny glyph (`renderChar`) or a solid block (`blockRenderChar`) at `BASE_CHAR_WIDTH * fontScale`, colored by its syntax token; whitespace only advances. Only the band of changed lines is pushed with `putImageData`, and unchanged lines are copied from the previous frame.
- In proportional mode it renders only the lines that fit around the viewport; in fit/fill it samples lines (`MinimapSamplingState`, `Math.round(i * ratio + halfRatio)`, first and last lines kept).
- Section header labels are drawn with `fillText` over a translucent background, stacked when nearby headers would overlap.

The whole design rests on two facts of a code editor: every line has the same height, and the text is monospaced with meaningful indentation, so a 1:N raster of characters is both cheap and recognizable.

### VS Code's overview ruler

The strip under the scroll bar's slider that marks decorations: Find matches, selections, errors and warnings, Git changes, the cursor ([`decorationsOverviewRuler.ts`](https://github.com/microsoft/vscode/blob/main/src/vs/editor/browser/viewParts/overviewRuler/decorationsOverviewRuler.ts)):

- One `aria-hidden` canvas, in device pixels, in up to three lanes (left, center, right, or full width; `editor.overviewRulerLanes`, default 3).
- A mark's y is the line's **layout offset** scaled to the canvas: `getVerticalOffsetForLineNumber(line) * canvasHeight / scrollHeight`. So the ruler always agrees with the scroll bar.
- Marks are at least 6 px tall (`MIN_DECORATION_HEIGHT`, times the pixel ratio), grouped by color, and merged when they touch, so drawing is one `fillRect` per run, not per decoration.
- It redraws only when the decorations, cursors, scroll height or theme changed.

### Visual Studio's "map mode"

The full Visual Studio IDE, not VS Code, has a scroll bar with two modes ([Microsoft Learn: Scroll bar map mode and bar mode](https://learn.microsoft.com/en-us/visualstudio/ide/how-to-track-your-code-by-customizing-the-scrollbar)): **bar mode** shows annotations on the scroll bar (changes, marks such as Find matches and breakpoints, errors, the caret), and **map mode** "displays lines of code, in miniature, on the scroll bar", in a width chosen in "Source overview", with an optional preview tooltip of the code under the pointer, and a click that jumps there.

### Newer VS Code work

No newer scrollbar "map" mode turned up in the 2025-2026 release notes. Recent changes are small: scrollbar indicators for failing tests (1.116), coverage in the minimap (1.115), and the slider showing while scrolling with `showSlider: mouseover` ([PR #346862](https://github.com/microsoft/vscode/pull/346862)).

**What the owner most likely means:** VS Code's minimap, possibly remembered as Visual Studio's map mode, which puts the same idea into the scroll bar itself. Both are a scaled picture of the text plus marks. The marks part is the overview ruler.

## 2. What carries over to a rendered document

A rendered document has none of the facts the minimap rests on: blocks have very different heights (a heading, a 40-row table, a 600 px image, a paragraph that is one source line but ten rendered lines), the text is proportional, and prose has no indentation shape. So the candidates are:

| Idea | What the reader gets | Carries over? |
| --- | --- | --- |
| **Marks for headings** | where the sections are, and a click target to jump there | yes: Downlight's headings are real section headers, unlike `MARK:` comments |
| **Marks for Find's matches and the current match** | where the matches cluster, as VS Code, Visual Studio and Chromium's own find bar show them (Chromium paints find tickmarks in the native scroll bar's track, `ScrollbarTheme::PaintTickmarks`, [scrollbar_theme.h](https://chromium.googlesource.com/chromium/src/+/773f7fe147c10b958b0f87ca3b9143cc36f0a5a4/third_party/blink/renderer/platform/scroll/scrollbar_theme.h); Downlight replaced WebView2's find with its own, so those don't show) | yes, the most useful mark; `findMatches()` was built for it (ticket 14) |
| **Marks for task items** (open ones, or ticks) | where the unfinished tasks are | possible; the glossary's entry names only headings and Find matches |
| **Blocks shaded by kind** (code, tables, images, quotes and alerts, lists) | the document's shape: "the code is in the second half" | plausible, as a layer under the marks |
| **Heading labels in the strip** (VS Code's section headers) | names without hovering | only in a wide strip, which takes reading width; a hover label is the cheap version (Visual Studio's preview tooltip) |
| **The view's slider** | where the view is | not needed if the ruler aligns with the native scroll bar: its thumb already is the slider. VS Code needs one because its minimap scrolls on its own |
| **A scaled picture of the real layout** | a thumbnail of the page | poor fit, see section 3, option D |

**Prior art** for rendered Markdown is all in Obsidian plugins; no standalone Markdown viewer researched (Typora, Zettlr, Markdown Preview Enhanced) has a minimap of its rendered view:

- [Markdown-Minimap](https://github.com/Nymbo/Markdown-Minimap) (MIT): renders the whole note a second time, small (scale 0.05 to 0.3), with click and drag; in Reading view, whose sections Obsidian virtualizes, it falls back to "the global ratio" because it can't know the real positions. The same problem as ADR 0003's, met by giving up exact positions.
- [My Note Minimap](https://github.com/sythewardon/my-note-minimap): one canvas, "only the part you can see is painted", headings, lists, quotes, code and callouts each in the theme's color, rows placed by the real layout and sharing the scroll thumb's positions. This is option B below, built.
- [pagemap](https://github.com/lrsjng/pagemap) (MIT, a general web-page minimap): a canvas fed by CSS selectors mapped to fill colors; on every scroll and resize it runs `querySelectorAll` and `getBoundingClientRect` for every matching element and refills the canvas. Fine for a page; too much work per frame for a 1 MB document, and the rects of blocks `content-visibility` skips are estimates.

## 3. How each option could be built here, and what it costs

### The constraints that decide it

- **ADR 0003** ([docs/adr/0003](../../docs/adr/0003-lay-out-only-blocks-near-the-view.md)): each top-level block of `article.markdown-body` has `content-visibility: auto` with `contain-intrinsic-size: auto 3em` (`page/src/document.css`). A block never shown is 3em tall until a frame lays it out; then its real height is remembered ([MDN: contain-intrinsic-size](https://developer.mozilla.org/en-US/docs/Web/CSS/contain-intrinsic-size)). The 1 MB documents' total estimate was within 1 % (ticket 02).
- **Reading geometry inside a skipped block forces its layout** and loses what ADR 0003 bought ([web.dev: content-visibility](https://web.dev/articles/content-visibility) warns about APIs that force rendering of skipped subtrees). Reading a **top-level block's own** rect is safe: it is size-contained, so its rect is the estimate or the remembered size. `reading-position.ts` already relies on this: `firstNotAbove` binary-searches the top-level blocks by `getBoundingClientRect`.
- **Responsiveness** (CODING_STANDARDS.md): no page task or frame over 100 ms; work that grows with the document is measured on the 1 MB test document with the timing switch. Ticket 31: documents go up to 100 MB, and the page already holds for about 15 s taking 100 MB in (ticket 39 is to fix that), so the ruler must add nothing to the frame after the insert.
- **Tests** run in happy-dom 21.0.4, which has no layout (the tests fake it with `layOut` in `page/src/test-support.ts`) and whose `canvas.getContext` returns null unless a canvas adapter is configured (`HTMLCanvasElement.js`). Ticket 14 already named the seam this needs: "If ... the overview ruler gets built, give Find a painter: CSS Highlights in the app, a recorder in the tests."
- **Find** keeps its matches as `FindMatch` records in document order (`page/src/find.ts`), tens of thousands of them for "t" on 1 MB, and `findMatches()` only reads them; there is no event yet when they change.
- **Line numbers** are on every block as `data-sourcepos` (`core/src/render.rs`, the reading position reads them).

### Where a mark's y comes from

This is the decision under every option.

1. **From layout, top-level blocks only (recommended).** A mark's y is its top-level block's top (scroller's `scrollTop` plus the rect's top minus the scroller's top, as `reading-position.ts` measures), scaled by `canvasHeight / scrollHeight`, as VS Code does. A Find match or a nested heading takes its top-level ancestor's top, plus, if wanted, its share of the block's text times the block's height, so no rect inside a skipped block is ever read. Because the scroll bar is computed from the same estimated heights, **the ruler and the scroll bar agree at every moment**, and a click on a mark lands where the scroll bar says. The cost: marks move a little as blocks are laid out for the first time, exactly as the scroll bar's thumb does today. This refines ADR 0003's consequence ("can't place its marks from block offsets until every block has been laid out once"): it can place them, consistent with the scroll bar, just not final.
2. **From line numbers.** y = start line / the file's line count. No layout reads at all, stable from the first frame, trivially testable, and the same at any zoom or width. But it disagrees with the scroll bar wherever line count and height differ: a one-line paragraph that wraps to ten lines, an image on one line, a table, blank lines, front matter, footnotes (last in the view, early in the file). A mark and the thumb would sit in different places for the same block, which reads as a bug.
3. **Exact layout, by laying out every block once in the background** (taking batches of blocks out of `content-visibility` in idle tasks so their sizes are remembered). Makes ruler and scroll bar final, but is ADR 0003's rejected "whole document" cost spread over time (141 to 186 ms of layout at 1 MB, unknown and large at 100 MB), with unmeasured memory, and would need an ADR. Not recommended unless option 1's drift turns out to bother the reader.
4. **A summary from the core** (block kinds, line ranges, heading text from comrak's AST). It knows nothing of layout, so it only yields option 2's positions, and the page already has every block's kind and line in the DOM. It would add a contract shape and buy nothing. The core stays as it is.

### Option A: the overview ruler (marks only)

What: a canvas strip, as tall as the scroller, beside or over the native scroll bar; marks for headings (perhaps by level, longer for h1 and h2), Find's matches, the current match, and perhaps task items.

How, in this codebase:

- A page module `page/src/overview-ruler.ts`, started from `main.ts`, owning one `<canvas aria-hidden="true">` fixed next to `.view-scroller`'s scroll bar (its width is `offsetWidth - clientWidth` of the scroller).
- A pure placement function: given the top-level blocks' tops and heights, the marks (by block index and share), the scroll height and the ruler's height in device pixels, it returns runs per lane and color, bucketed per pixel row and merged as VS Code merges them. Drawing is then O(ruler height), not O(marks): with 100k matches it is still about 1000 `fillRect` calls at most. Vitest tests drive this function with faked geometry, and a painter interface (canvas in the app, a recorder in the tests) keeps the module testable without a canvas adapter.
- Hooks it needs: `whenDocumentShown` (show, Reload, Back and Forward replace the content); a new listener from Find when its matches or current match change (today only the `findMatches()` getter exists); a `ResizeObserver` on the article, which fires when its height changes: blocks laid out for the first time, zoom, Full width, window resize; `whenSettingsChange` (theme, accent color); `matchMedia` for `prefers-color-scheme` and `forced-colors`; the ticks' `change` events if task items get marks; Sort, once built (its ticket already says the ruler follows the shown order).
- Redraws coalesced to one per frame. The heavy part is re-reading the tops of the blocks that carry marks after a height change: one read per such block after layout is clean, no writes in between. Heading marks are few; Find's can span thousands of blocks. That is the number to measure on the 1 MB documents while scrolling and typing.
- Click: the mark nearest the pointer (within a few pixels) goes there through the existing `scrollToTop` or `scrollToMiddle`, which already correct for estimated heights. A hover label with the heading's text, in the `.link-target` style, gives the section header names without a wide strip.

Colors and modes:

- Canvas colors don't follow CSS, so the module reads them from computed style when it draws: the accent's Find roles (`--accent-find-match`, `--accent-find-current` in `page/src/accent.css`) for matches, a muted foreground for headings, and redraws when the theme, the accent color or the backdrop changes.
- High contrast: forced colors don't touch canvas pixels, so under `forced-colors: active` it reads the system colors (`Mark`, `Highlight`, `CanvasText`, `GrayText`) through computed style, as `document.css` uses them for Find's highlights. The alternative is a DOM ruler of absolutely placed runs (at most one per pixel row, so about 1000 elements), which forced colors would recolor by themselves; more DOM, less code for colors.
- Accessibility: decorative and `aria-hidden`, as VS Code's is; nothing on it is only reachable there (Find announces "N of M", F3 steps through matches, links and Outline reach headings). It takes no focus.

Large documents: start it only after first paint, build its block index in slices like Find's `IndexBuilder` (64 KB per task), and above some size show headings first or nothing until the index is ready. At 100 MB a 1000 px ruler gives each pixel about 100 KB of text, so marks merge heavily; it still shows where matches cluster.

Cost: one page module of a few hundred lines with its tests, a Find listener, a painter seam, and a measurement on 1 MB (typing in Find, scrolling) and on 20 and 100 MB. No app, core or contract change, no dependency.

### Option B: blocks shaded by kind (a layer of the ruler)

What: under the marks, the ruler's background tinted by the kind of block at each pixel row: code blocks, tables, images, quotes and alerts, lists, plain text (no tint). My Note Minimap does this.

How: for each pixel row, binary-search the top-level block at that y (the same search as `firstNotAbove`), and take its kind from its tag or class (`.highlight` for code, `table`, `img`, `blockquote`, GitHub's alert class). That is ruler height times log2(blocks) reads per redraw, about 20,000 for a 1000 px ruler over a million blocks, so it scales to 100 MB, unlike reading every block. It redraws when option A does.

Cost: small on top of A. Risks: visual noise next to the scroll bar, five or six tints that must work in light, dark, on Mica and with the accent color, and in high contrast only a few system colors to tell kinds apart (likely off there).

### Option C: heading labels in a wide strip

What: VS Code's section headers: the headings' text in the strip at their positions.

How: as A, plus `fillText` with collision stacking, as `_renderSectionLabel` does. Needs a strip of roughly 100 to 150 px. At GitHub's 980 px reading width in a wide window the margin has room; in a narrow window or with Full width the strip would cover text or narrow the document, which changes wrapping and so the reading position on every toggle. The hover label in A gives the names at no width. If the owner wants headings always visible, that is the table of contents sidebar the spec already lists ("after Outline"), not the ruler.

Cost: medium, mostly layout and design decisions, plus a new glossary concept if it becomes its own thing.

### Option D: a scaled picture of the rendered document (a true minimap)

What: the page itself, at 5 to 10 %, as Markdown-Minimap renders Obsidian's notes.

How it could be built, and why each way fails here:

- **A second, scaled DOM clone.** Doubles the document's DOM and memory (1 MB mixed already takes 359 MB private, 100 MB mixed 3.8 GB, ticket 31). Every block of a tiny clone is near its view, so `content-visibility` skips nothing: the clone needs the whole-document layout ADR 0003 avoided (141 to 186 ms at 1 MB in ticket 02, far more at 100 MB). Heading ids would be duplicated (the page finds heading targets with `getElementById`), links and checkboxes in it would have to be made inert, Find's CSS highlights wouldn't show in it (they are ranges over the real nodes), and ticks, Sort, Reload, zoom and Full width would all have to be mirrored.
- **CSS `element()`**, a live image of an element: not in Chromium or WebView2 at all, only Firefox's `-moz-element()` ([MDN browser-compat-data](https://github.com/mdn/browser-compat-data/blob/main/css/types/image.json): Chrome `version_added: false`; [MDN: element()](https://developer.mozilla.org/en-US/docs/Web/CSS/element)).
- **Rasterizing the page into a canvas** with an html2canvas-style library: a large dependency that re-implements CSS, slow on large documents, and contrary to "a dependency earns its place". WebView2's own capture shows only the visible part.

And the payoff is small: at 5 % a paragraph is a grey bar, so the picture shows block shapes, which option B shows at a fraction of the cost, and no text. VS Code's minimap is useful because code's indentation and syntax colors make a recognizable shape; prose doesn't have one.

Cost: high, and against ADR 0003, the memory numbers and the 100 MB documents. Not recommended.

### Placement against the native scroll bar

- **A strip just left of the native scroll bar** (about 8 to 14 px): simplest, takes clicks of its own, keeps every native scroll bar behavior. It narrows the view a little, or sits in the document's right padding (32 px) when the window is wider than the reading width.
- **Over the scroll bar's track** (`pointer-events: none`), as Chromium draws its find tickmarks and VS Code draws its ruler under the slider: no width taken, but the marks paint over the native thumb, and clicks go to the scroll bar, so marks can't be click targets.
- **Hiding the native scroll bar and drawing one's own** (what VS Code does): full control, but reimplements dragging, paging, arrow buttons and Windows' scroll bar settings. Not worth it. WebView2 can also be switched to Fluent overlay scroll bars (`ScrollBarStyle`, [WebView2 API overview](https://learn.microsoft.com/microsoft-edge/webview2/concepts/overview-features-apis#browser-features)), which auto-hide and would change where a strip "beside" them sits; Downlight uses the default style today.

## 4. Recommendation

1. **Build the overview ruler as planned (option A), positions from layout of the top-level blocks**, aligned with the native scroll bar so its thumb is the slider, with marks for headings, Find's matches and the current match. It is the part of VS Code's and Visual Studio's maps that means something for a rendered document, it reuses what exists (`findMatches()`, `reading-position.ts`'s geometry and scrolling, the accent roles), and it costs one page module with no app or core change.
2. **Order:** the spec puts it "after Outline". Find's marks don't depend on Outline at all, and heading marks only share the list of headings with it. If the owner's real wish is "see where my Find matches are", the ruler with Find's marks can come first; if it is "see the document's structure", Outline (and the sidebar after it) answers that better than any strip.
3. **Block shading (option B) only as a later, optional layer**, decided after A is in and measured, and probably off in high contrast.
4. **No scaled picture of the document (option D)**, and no line-number positions (they disagree with the scroll bar). Heading labels in a wide strip (option C) are the table of contents sidebar in disguise.
5. **Glossary:** options A and B stay within "Overview ruler"; its entry would grow to name task items (if marked) and the block shading. Only a scaled picture or a labeled strip would be a new concept, and the glossary already rules out its likely names ("Minimap, document map"); since neither is recommended, no new term is needed.
6. **ADR 0003:** if A is built, amend its consequence to say the ruler places marks from the blocks' current, possibly estimated offsets, so it agrees with the scroll bar and settles as blocks are laid out.

## Open questions to grill with the owner

1. **Which map did you mean, and for what?** VS Code's minimap (a picture of the text), Visual Studio's map mode (the picture in the scroll bar), or the marks (bar mode / the overview ruler)? Is the need to see where Find's matches are, to see the document's structure, or to jump around quickly?
2. **Positions:** marks that always agree with the scroll bar but move slightly as blocks are first laid out (recommended), or marks fixed by line number that don't line up with the thumb?
3. **Which marks:** all heading levels or only h1 to h3? Find's matches and the current match? Task items, open or ticked? The reading position? Block shading?
4. **Where and when:** a strip beside the native scroll bar or marks over its track; how wide; always shown, only while Find is open, or a setting in the flyout, and its default?
5. **Clicks:** does a click on a heading mark go there as a heading link does, joining History so Back returns? Does a click on a Find mark make that match the current match? A hover label with the heading's text?
6. **Large documents:** above what size may the ruler arrive late, show only headings, or stay off, and must it wait for ticket 39?
7. **Outline first or ruler first**, as the spec's "after Outline" says?
