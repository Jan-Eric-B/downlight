# 10: Counts

**What to build:** The **counts** of a document: its lines and words in the title bar's file name tooltip, under the full path, and the selection's words and characters in a small bubble at the bottom right while text is selected. No status bar.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] The core counts the document's lines and words while it renders, and the rendered document carries them to the page.
- [ ] The file name's tooltip reads the path, then "1,234 lines, 5,678 words" on the next line; the file name button's accessible description has the counts too.
- [ ] The selection bubble shows "12 words, 68 characters" while the document's text is selected, and hides when the selection collapses or the document changes.
- [ ] Core tests, app contract samples and page tests (seams below), written first.
- [ ] Measured on the 1 MB test document with the timing switch: render time before and after, recorded here.
- [ ] The hand checks below, recorded here.

## Decisions

From the grilling of 2026-11-19. Ships in v1; ticket 18 (the release check) stays closed, and this ticket carries its own hand checks.

**Form.** No bar, always shown or switched: a bar takes a row from the document and the bottom left where the link target bubble sits, and a reader seldom wants the counts. The document's counts are one hover away in the tooltip; only the selection's, which a reader does want in the moment, show by themselves.

**What is counted.**

- **Lines:** the file's, as line numbers are. One per line break, plus one when the last line has none: `a\nb` and `a\nb\n` both have 2, an empty file 0.
- **Words:** of the text the reader sees: headings, paragraphs, lists, tables, code blocks, footnote labels and the text of HTML that survives sanitizing. Not image alt text or link targets, which aren't shown. Split by UAX #29 word boundaries, counting the word-like segments, so Chinese, Japanese and Korean count sensibly.
- **Selection:** words (`Intl.Segmenter`, word granularity, `isWordLike`) and characters (grapheme clusters, so "é" or an emoji is one), spaces counted and line breaks not, as Word's "characters (with spaces)". Only a selection in the document's text counts; the find bar and title bar are never part of it.
- **Not counted:** the document's characters, an estimated reading time, the reading position's line (the margin shows it), the encoding.

**Where the counts are made.** The core counts while it renders, on the render's background task, so a 100 MB document doesn't hold up the page and a reload brings new counts. Words come from `unicode-segmentation`'s `unicode_words` (already in the build through tao; the core gains it as a direct dependency with `cargo add`). `RenderedDocument` (`core/src/render.rs`) now serializes as a plain string; it gains the counts, so its shape on the way to the page changes, and the contract samples with it (`npm run fixtures`). The page counts only the selection. The two segmenters both follow UAX #29, so selecting the whole document gives nearly the document's word count, not always exactly; that is accepted.

**The tooltip** (`page/src/title-bar.ts`, the file name's `title`, which is the full path today):

- "C:\docs\README.md", a line break, "1,234 lines, 5,678 words". Singular for one ("1 line", "1 word"); digits grouped the English way, as the UI is English.
- While the document is on its way (the wait note) the tooltip is the path alone. The empty window keeps its plain "Downlight" and no tooltip.
- A reload updates the counts.

**The selection bubble:**

- At the bottom right, looking like the link target bubble (`.link-target` in `page/src/document.css`): solid background, small text, never takes the pointer, not zoomed with the document.
- "12 words, 68 characters", singular for one.
- Shows while the selection in the document has at least one character, follows it as it changes, and hides when it collapses or the document changes (reload, Back, Forward, Open).
- The link target bubble and notices are drawn above it where a narrow window makes them meet.
- `aria-hidden`, as the link target bubble is, not a live region: Narrator reads the selection already, and a live region would speak on every Shift+Arrow.

## Test seams

- **Core** (`cargo test -p downlight-core`), through `render`: lines with and without a final line break and for an empty file; words of headings, lists, tables and code; image alt text and link targets not counted; Chinese text counted by UAX #29; HTML removed by sanitizing not counted. The rendered fixtures get their counts.
- **Contract samples:** the rendered document's new shape in `app/tests/fixtures/`, read by `page/src/contract.test.ts`.
- **Page** (Vitest with happy-dom): the tooltip is the path and the counts, singular for one, grouped digits, the path alone during the wait note, updated on reload; the accessible description; the bubble shows on a selection in the document with its words and characters (graphemes, line breaks not counted), not for a selection outside it, and hides on collapse and on a document change.

## Hand checks

On the release build (`npm run tauri build`), to record here:

1. Light, dark and high contrast: the tooltip and the bubble are legible, the bubble on its solid background.
2. The 1 MB test document (`npm run documents`): the tooltip's counts are there once it shows; with `DOWNLIGHT_TIMING` set, no page task or frame over 100 ms from the counts, and select-all shows the bubble without one either.
3. A selection across a code block and a table counts what the selection holds.
4. A narrow window: the link target bubble and a notice draw above the selection bubble.

## Comments

**2026-11-13, from the grilling.** For a viewer, the counts should probably be about the rendered text the reader sees, not the Markdown source; the line count is the exception, since line numbers are the file's. Counting the whole document grows with its size, so it is measured with the timing switch when it is added.

**2026-11-19, grilled.** Decided as above: the tooltip and the selection bubble instead of a status bar, counted by the core, in v1. The glossary has **Counts**; the spec's story 30 and Out of Scope follow.
