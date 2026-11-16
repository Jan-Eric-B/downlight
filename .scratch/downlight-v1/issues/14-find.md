# 14: Find

**What to build:** Ctrl+F opens a find bar that highlights every match in the shown document as the reader types, shows "3 of 12", and steps through the matches. **Find** looks only in the shown document.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 09 (Opening from inside a window)

**Status:** closed

- [x] Ctrl+F opens the find bar with the focus in its box; pressing it again while open selects the text in the box.
- [x] Matches are highlighted as the reader types, ignoring case, without changing the document's DOM.
- [x] The bar shows "N of M", or "No results" when nothing matches.
- [x] Enter and F3 go to the next match, Shift+Enter and Shift+F3 to the previous one, wrapping at the ends. The current match scrolls into view and is highlighted differently from the rest.
- [x] Esc closes the bar and clears the highlights.
- [x] The bar has a previous and a next button (up and down arrows, as in Edge and VS Code) beside the count. A click steps like Shift+Enter and Enter, wrapping at the ends; each has a tooltip with its key ("Previous match (Shift+Enter)", "Next match (Enter)"), and both are unavailable when nothing matches.
- [x] The bar has a close button (an X, after the arrow buttons) with the tooltip "Close (Esc)". A click does what Esc does: it closes the bar and clears the highlights, and the box keeps its text for the next Ctrl+F.
- [x] After a reload while the bar is open, the matches are counted again.
- [x] Find is in the right-click menu.
- [x] Page tests cover the count, the current match, next and previous, wrapping, Esc and the count after a reload.
- [x] Typing in the box on the 1 MB document is measured with the timing switch, and the result is recorded in this ticket.

## Plan

**Approach:** A page module `page/src/find.ts` owns the find bar, which sits outside the `article`. Once per shown document, and only when Find needs it, it builds a text index with a TreeWalker: the view's text nodes joined into one case-folded string with node offsets. Each keystroke runs `indexOf` over that string, so a match can span inline formatting. Matches are kept in document order as plain `{startContainer, startOffset, endContainer, endOffset}` records, which the overview ruler can read later, and painted with two CSS Custom Highlights, `find-match` and `find-current` (priority 1), built from `StaticRange`, so the document's DOM never changes. Live Ranges aren't used for painting: about 100k of them would slow every later DOM change, including the next `innerHTML`. happy-dom 21.0.4 has no `CSS.highlights`, `Highlight` or `StaticRange` (WebView2 has had them since Chromium 105), so painting sits behind one `"highlights" in CSS` check and tests read the same records the highlights are built from, with no fake. The recount hooks into the one place the page puts a rendered document in place, `showDocument` inside `startDocumentView` in `page/src/view.ts`.

**Tests first:**

- `page/src/find.test.ts` setup: `startFind`, `startCommands` and `startDocumentView` with `mockIPC` answering the new `find.html` fixture, as `view.test.ts` does. Keys are dispatched `keydown` events; typing sets `box.value` and dispatches `input`. Observed: the bar's text, `document.activeElement`, the box's selection, `findMatches()` and `view.innerHTML`.
  - "Ctrl+F opens the bar with the focus in its box" - default prevented, so WebView2's own find bar stays away
  - "Ctrl+F while the bar is open selects the text in its box"
  - "typing counts the matches, ignoring case" - "lamp" over Lamp, LAMP and lamp gives "1 of N"
  - "a match across inline formatting counts once" - "lamp light" across `<strong>`
  - "nothing matching shows No results" - and an empty box shows no count and has no matches
  - "finding leaves the document's DOM unchanged" - `view.innerHTML` equal before typing, after stepping and after Esc
  - "the first match after the reading position is current" - with a block halfway down at the top of the view (stub `getBoundingClientRect`, as `view.test.ts` does for Reload), the current match is the first one at or after that block
  - "at the top of the document the first match is current"
  - "a current match in a closed section opens it" - every closed `<details>` around the current match gets `open`
  - "Enter and F3 go to the next match" / "Shift+Enter and Shift+F3 go to the previous match"
  - "next after the last match wraps to the first" / "previous before the first match wraps to the last"
  - "the current match's block is scrolled into view" - spy on `Element.prototype.scrollIntoView` (happy-dom has no layout; where it lands is a manual check)
  - "Esc closes the bar and clears the matches"
  - "a document shown while the bar is open is counted again" - `emit("document-shown")` with `notes.html`; once ticket 10 has landed, the same through its reload path
  - "a new document doesn't scroll to a match" - so Reload's anchoring and a link's heading win
- `commands.test.ts`: the menu test expects `{kind:"command", id:"find", label:"Find", shortcut:"Ctrl+F"}`; add "runs Find chosen in the menu".

**Changes:**

1. `core/tests/fixtures/find.md` - new fixture with known totals: a heading, paragraphs and a table with lamp, Lamp and LAMP, "lamp **light**", a code block with lamp. Run `npm run fixtures` to write `find.html`.
2. `page/src/view.ts` - `export function whenDocumentShown(listener: (view: HTMLElement) => void, signal?: AbortSignal)`. `showDocument` calls the listeners after every replacement of the view (document, empty and error), only for the request that wins, before it scrolls.
3. `page/src/find.ts` - `startFind(signal?)` builds the hidden bar (`div.find-bar.markdown-body`, an input labelled "Find", a status span with `role="status"`) and subscribes with `whenDocumentShown`: the index goes stale, and if the bar is open it recounts, keeping the current match at or after the old one's offset, without scrolling.
   - `openFind()` shows the bar and focuses the box, or selects its text if already open.
   - `findMatches(): {matches: readonly FindMatch[]; current: number | undefined}` for the tests and the overview ruler.
   - Enter and Shift+Enter in the box (ignored while `isComposing`); F3 and Shift+F3 on a window `keydown`, default always prevented, and F3 opens the bar if closed. Esc closes the bar and deletes both highlights; the box keeps its text.
   - Case folding keeps string lengths, so offsets map back to text nodes.
   - The first current match is the first one at or after `readingLine()` (from ticket 10's `page/src/view.ts`), so the reader keeps their place; at the top of the document it is the first match.
   - Going to a match opens every closed `<details>` around it, then calls `scrollIntoView({block: "center"})` on the element holding it, then after one frame checks the range's rect and scrolls once more if needed (`content-visibility: auto`, ADR 0003).
   - If the measurement (step 7) shows a keystroke over 100 ms, painting is limited to the matches in and near the view (about a screen above and below), repainted on scroll at most once per frame. The count and `findMatches()` still cover every match, so "N of M" stays exact and the overview ruler sees all of them. The choice of which matches to paint is a pure function with its own page test.
4. `page/src/commands.ts` - `find: { ctrlLetter: "f", run: openFind }` and a Find line in the menu, in the spec's order. `app/src/menu.rs` needs no change.
5. `page/src/main.ts` - `startFind()` before `startDocumentView`.
6. `page/src/document.css` - `.find-bar` fixed top right with `.notice`'s font and colors; `::highlight(find-match)` and `::highlight(find-current)` in light and dark, and `Mark`/`Highlight` under `forced-colors: active`.
7. `scripts/measure.js` - pull the CDP connect and `evaluate` code out of `scrollThrough` into one helper (shared with ticket 10's reload case). Add "Find while typing" on `prose-1mb.md` and `mixed-1mb.md`: Ctrl+F through `Input.dispatchKeyEvent`, then `Input.insertText` one character at a time for "e", "the", "lamplight" and a string that matches nothing, waiting a frame after each. Report each keystroke's time to the next frame (median and worst) and the longest `long-task` or `long-frame` line. No new timing line is needed: the switch already reports tasks and frames over 100 ms through `report_long_task`, and `timing.rs` is untouched.
8. This ticket - record the measurement in a dated Results section.

**Fits with other tickets:**

- `whenDocumentShown` is the one hook for "a rendered document is now in place". Ticket 10's Reload calls it after anchoring; ticket 11's in-window links go through `showDocument`, so both recount for free. Whichever ticket lands first adds the hook.
- Find needs no change to the `Command` shape; ticket 10 or 15 makes `ctrlLetter` optional and adds `key`.
- `readingLine()` in `page/src/view.ts` comes from ticket 10 (its Reload anchor). If Find is built first, Find adds it.
- Menu order follows the spec: Back, Forward, Reload, Find, Edit, Choose editor..., Open, Zoom, Copy.

**Verification:** `npm run check`. Then `npm run documents`, `npm run tauri build`, `npm run measure`; the target is no task or frame over 100 ms while typing on 1 MB. By hand in `npm run tauri dev`: Ctrl+F twice; Enter, F3 and Shift+F3 wrapping, with the current match centred far down the 1 MB document; the current match looks different in light, dark and high contrast; Esc clears the highlights; the menu's Find opens the bar; saving the file with the bar open recounts (after ticket 10); WebView2's own find bar never shows.

**Decisions (2026-11-12):**

- If typing on the 1 MB document goes over 100 ms per keystroke, Find counts every match but paints only those in and near the view, updated on scroll. Not chosen: one update per frame (may not be enough) and a minimum query length (one letter would find nothing).
- A current match inside a closed `<details>` opens it, as Chrome does. Setting `open` is a reader-visible change to the page, not part of the highlighting, so the "without changing the document's DOM" criterion still holds for the highlights.
- The first current match is the first one after the reading position, not the first in the document.
- The tickets are built one at a time, so each shared piece in "Fits with other tickets" is added by the first ticket that needs it.

## Results

Built 2026-11-12. `npm run check` passes. Measured with `npm run measure` (see Measured); not yet checked by hand in the real app.

**Beyond the plan:**

- Spaces: inside a block's text, a line break from the file, a no-break space and every other space match a space in the box, so a phrase matches across a hard-wrapped line. A text node of only line breaks and spaces, which sits between blocks, keeps its line breaks, so a match never runs from one block into the next; code keeps its lines. Each character stays one character, so offsets still map back to text nodes. Tests "a line break or no-break space in a paragraph matches a space", "a match never runs from one block into the next" and "code keeps its lines".
- Typing on keeps the current match: from the second character, the current match is the first one from where the last current match started, as in Chromium, and not the first after the reading position, which the scroll to the last match has moved. Opening the bar, and typing after "No results", start from the reading position. Tests "typing on keeps the current match" and "typing after No results starts again from the reading position".
- The reading position is compared by place in the document, not by line number: `view.ts` exports `readingBlock(view)`, the innermost element at the top of the view (Reload's `readingLine` is built on it), and a match is at or after it when it is inside it or follows it. Line numbers don't rise in document order: footnotes come last but keep their definitions' lines, and raw HTML has none, either of which broke the binary search, which then skipped matches or went back to the top. `find.md` has a footnote defined early for this. Tests "a match in raw HTML after the reading position is current" and "a footnote is after the reading position, though its definition is early in the file".
- A word-final sigma ("ς") and any other ("σ") match each other: case folding makes them one, which keeps the offsets. Test "a word-final sigma matches any other sigma".
- Typing, and opening the bar with text left in its box, go to the current match (open its sections, scroll), as in Chromium; only a recount after a shown document doesn't.
- A recount after a shown document (another document or a reload) takes the first match at or after the reading position, rather than the first after the old current match's offset: offsets in the last document's text say nothing about another one, and a reload has just put the view back at the reading position.
- `whenDocumentShown` also runs when the window shows no document or an error (plan step 2; ticket 10 left it out), so Find never counts matches in the last document's detached text. Tests "a window whose document goes away finds nothing" and "a document that can't be read runs the whenDocumentShown listeners". Its listener gets the view, as the plan said, so Find looks nothing up. A request that replaced the view and was then overtaken leaves the call to the request that overtook it, which makes it even when its read fails, so the listeners hear of every replaced view. Tests "a reload overtaken by a reload that can't read the file still runs the whenDocumentShown listeners" and "a reload overtaken by a reload that can't read the file is counted again".
- WebView2's keys: Microsoft's docs (`ICoreWebView2AcceleratorKeyPressedEventArgs2`, `CoreWebView2Settings.AreBrowserAcceleratorKeysEnabled`, "Differences between Microsoft Edge and WebView2") put WebView2's own handling of a browser key before the web content gets it, and list Ctrl+F, F3, Shift+F3, Ctrl+G and Ctrl+Shift+G as its find keys. So `preventDefault` alone can't be relied on, and the handler, renamed `leave_reload_and_find_keys_to_the_page`, also turns off WebView2's handling of Ctrl+F, Ctrl+G and F3 with any other modifiers; the page still gets them. The key check is a pure `is_reload_or_find_key`, with tests `reload_and_find_keys_are_left_to_the_page` and `other_keys_stay_with_webview2`. Compiled and unit tested, not pressed in the app.
- F3 and Shift+F3 are commands in `commands.ts` (`findNext`, `findPrevious`), not on a window `keydown` of Find's own as the plan said; `Command` has a `shift` flag for Shift+F3. Neither is in the right-click menu. F3 with the bar closed opens it ("F3 while the bar is closed opens it"); the key is always taken; with Ctrl, Alt or Meta it does nothing ("F3 with Ctrl held does nothing", "F3 with Alt held does nothing"). Unlike the old handler, F3 during an IME composition is a step, as F5 is a reload then. Esc closes the open bar wherever the focus is, not only in the box ("Esc with the focus outside the box closes the bar"), and Enter that ends an IME composition doesn't step ("Enter that ends an IME composition doesn't step").
- After the frame, the second scroll looks at the match's own rect and scrolls it to the middle (`scrollBy`) only if it is outside the view, so a match far down a paragraph taller than the view shows too. A later step, or closing the bar, cancels it. Tests "a current match still outside the view after a frame is scrolled to its middle", "a later step cancels the last match's scroll after its frame" and "closing the bar cancels the current match's scroll after its frame".
- The reading position and the scrolls that check they landed (ADR 0003) moved from `view.ts` to `page/src/reading-position.ts`, after the architecture review (see Comments). Find takes `atOrAfterReadingPosition` and `scrollToMiddle` from it. Open moved from `commands.ts` to `open.ts`, which ended the import cycle commands, find, view, commands. The page tests share `settle`, `nextShown`, `linesOf` and `layOut` from `page/src/test-support.ts`.
- `GLOSSARY.md` has "Current match".

**Measured (2026-11-12):** `npm run documents`, `npm run tauri build`, `npm run measure`, release build, median of 5 runs. The measurement types "e", "the", "lamplight" and a string that matches nothing into the find bar on the 1 MB mixed and prose documents. The target is no task or frame over 100 ms.

First, with the whole index built on the first keystroke:

| Measure (median of 5) | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: |
| keystrokeToFrame | 15.4 | 14.8 |
| firstKeystrokeToFrame | 139.3 | 117.5 |
| longestLaterKeystrokeToFrame | 40.7 (worst 45.2) | 32.8 (worst 39.7) |
| longestWhileTyping | 135.2 (worst 136.5) | 113.9 (worst 122.5) |

Over the target, and only on the first keystroke: every later one, "t" with tens of thousands of matches among them, stays under 46 ms, so painting every match is cheap enough and the plan's fallback (paint only near the view) isn't needed. The long part was building the index. So the index is now built a slice at a time (`IndexBuilder`, 64 KB of text per task, `SLICE_CHARACTERS` in `page/src/find.ts`), starting when the bar opens rather than at the first keystroke. A short document is still indexed at once, in the same task; for a long one, a count asked for before the index is built waits for it, with no count in the bar until then, and then goes to its match as typing does. A document shown during the build drops it. Tests "a long document is counted once its text is indexed", "typing while a long document is indexed counts the last text and goes to its match" and "a document shown while a long one is indexed is counted instead".

`scripts/measure.js` now times each keystroke until the end of the first frame that shows its count, so a keystroke waiting for the index isn't measured as fast, and reports the first keystroke apart. With the index built in slices:

| Measure (median of 5) | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: |
| keystrokeToCount | 14.8 | 15.0 |
| firstKeystrokeToCount | 158.4 | 129.8 |
| longestLaterKeystrokeToCount | 37.4 (worst 42.0) | 31.9 (worst 32.2) |
| longestWhileTyping | 0.0 (worst 0.0) | 0.0 (worst 0.0) |

No task or frame over 100 ms while typing: the target is met. The first count still comes 130 to 160 ms after the first keystroke, but the page stays responsive meanwhile. The measurement types one frame after Ctrl+F, when the index has barely started; a reader takes longer than that to start typing, so for them the index is usually ready by the first keystroke.

**Not yet checked by hand** (`npm run tauri dev` or the release build):

- Ctrl+F opens the bar with the focus in its box; Ctrl+F again selects its text.
- WebView2's own find bar never shows: Ctrl+F, F3, Shift+F3, Ctrl+G and Ctrl+Shift+G, with the bar open and closed.
- All matches are highlighted as you type, and the current match looks different, in light, dark and high contrast.
- Enter, F3, Shift+Enter and Shift+F3 step and wrap, with the current match centred far down the 1 MB document, in a paragraph taller than the window, and inside a closed `<details>`, which opens.
- Esc clears the highlights and keeps the box's text; Ctrl+F shows them again.
- The menu's Find opens the bar.
- Saving the file with the bar open recounts and leaves the view at the reading position; a link to another document with the bar open recounts and doesn't scroll to a match.
- An IME's Enter that ends a composition doesn't step.
- The bar over the "File deleted or moved" bar, and in a narrow window.
- F5 and Ctrl+R still reload, and Ctrl+plus, Ctrl+minus and Ctrl+0 still zoom (the key handler changed).

**2026-11-16, the bar's buttons, and the older boxes.** `npm run check` passes.

The three older open boxes (highlights as the reader types, Enter/F3 and Shift+Enter/Shift+F3 with the current match told apart, Esc) were already built on 2026-11-12 and covered by page tests; they were open only for want of a hand check, since happy-dom has no CSS Custom Highlights and the painting (`paint`, `paintCurrent`) runs only in the webview. Nothing was missing in code, so they are ticked now, and what still needs eyes is in the hand checks below. Since then ticket 29 turned WebView2's browser keys off (`leave_keys_to_the_page`), so `leave_reload_and_find_keys_to_the_page` above is gone, and F3 and Shift+F3 stay the `findNext` and `findPrevious` commands in `page/src/commands.ts`.

Built now, in `page/src/find.ts`: a previous and a next button (Fluent's `arrow_up_16_regular` and `arrow_down_16_regular`) after the count, and a close button (`dismiss_16_regular`, the window's X) after them. Their clicks call the same `step(-1)`, `step(1)` and `close()` that Shift+Enter, Enter and Esc call, so a click also wraps, scrolls the current match into view and opens its sections, and closing cancels a scroll still waiting for its frame. Tooltips "Previous match (Shift+Enter)", "Next match (Enter)" and "Close (Esc)"; screen readers hear "Previous match", "Next match" and "Close Find" (not "Close", which is the window's button), with `aria-keyshortcuts`. The arrows are `disabled` whenever there is no current match: an empty box, "No results", and a long document's count still waiting for its index; the close button is always available. A press on any of them keeps the focus where it was (`mousedown` default prevented, as the title bar does), so the reader can click and go on typing or press Enter; they stay in the Tab order for the keyboard.

The look, in `page/src/document.css` (`.find-button`): the title bar's, as the 2026-11-16 notes ask: a 16 px glyph in the bar's text color on a 28 px button with a 4 px radius, Windows 11's subtle fills under the pointer and while pressed, Windows 11's disabled text fill when unavailable, and the accent's focus ring (`--accent-emphasis`, Dark 1 in light and Light 1 in dark, per the window design spec's focus-ring row). The subtle fills moved from `.title-bar` to `:root` in `page/src/title-bar.css`, renamed `--subtle-fill-hover` and `--subtle-fill-pressed`, so both bars share them. High contrast: `ButtonText`, `Highlight` and `HighlightText` under the pointer, `GrayText` when unavailable, as on the title bar. The buttons sit 2 px apart, close together as in Edge's and VS Code's find bars. The window design spec says nothing else about the find bar beyond its colors (ticket 03, done) and that it keeps its size under zoom. `vite.config.ts`'s Fluent notice comment now names the find bar too.

**Tests:** seven new page tests in `page/src/find.test.ts`, written first and seen failing: "the arrow buttons sit beside the count and the close button after them", "each of the bar's buttons names its key in its tooltip", "the next and previous buttons step like Enter and Shift+Enter, wrapping at the ends", "a step with a button scrolls the current match into view", "the arrow buttons are unavailable when nothing matches", "pressing a bar's button leaves the focus in the box" and "the close button closes the bar and clears the matches, as Esc does". One existing test changed: `ROLE_USES` in `page/src/accent.test.ts`, which lists every use of an accent role in the stylesheets, gets the new `.find-button:focus-visible` outline as an emphasis use; it failed because the stylesheet gained a use, not because of a bug.

**Not measured:** matching, indexing and painting are unchanged; each count now also sets two buttons' `disabled`, which doesn't grow with the document. The 2026-11-12 numbers stand.

**Not yet checked by hand** (release build), besides the list above:

- The arrow and close buttons in light, dark and high contrast: hover and pressed fills, the amber focus ring when tabbing to them, the arrows dimmed with an empty box and with "No results".
- A click on next and previous steps and wraps with the current match centred, and the focus stays in the box (typing goes on, Enter steps); the close button clears the highlights, and Ctrl+F shows them again with the box's text kept.
- The tooltips' wording, and that the bar still fits in a narrow window with the three buttons.

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- `whenDocumentShown` exists in `page/src/view.ts`. It runs on the first show, on another document, and after a reload once the view is back at the reading position. Recount there.
- `readingLine(view)` exists but is module-private, since nothing outside `view.ts` used it yet. Export it to find the first match after the reading position.
- `Command` already has `ctrlLetter?` and `key?`, and Reload is in the menu. Find goes after Reload, in the spec's order.
- Ctrl+F is one of WebView2's own browser keys (it opens WebView2's find bar). Ticket 10 found that cancelling a key in the page doesn't always stop WebView2. If `preventDefault` doesn't hold for Ctrl+F, F3 or Shift+F3, extend the existing `AcceleratorKeyPressed` handler (`leave_reload_keys_to_the_page` in `app/src/platform/windows.rs`, renamed for what it covers) rather than adding a second handler.
- To keep each task under 100 ms on the 1 MB document, ticket 10 parses a reload's HTML into a `<template>` and swaps it in a frame later. Building the text index can follow the same rule: measure it with the timing switch, and split it across tasks if it doesn't fit in one.

**2026-11-12, architecture review.** Ticket 14 had copied Reload's "scroll, wait a frame, scroll again" into `goToCurrent` and imported `readingBlock` and `afterNextFrame` from `view.ts`. Both now live in `page/src/reading-position.ts`: Reload, heading links and Find call `scrollToTop`, `scrollToMiddle`, `readingPosition`, `returnToReadingPosition` and `atOrAfterReadingPosition`, and only that module knows how a reading block is found. The page tests didn't change apart from one import.

A lead for later, not worth doing now: happy-dom has no CSS Custom Highlights, so `canPaint` is false in the tests and `paint` and `paintCurrent` never run there; the tests read `findMatches()` instead. If Find had to paint only the matches near the view (the fallback in Measured) or the overview ruler gets built, give Find a painter: CSS Highlights in the app, a recorder in the tests. With two adapters the seam is real, and the tests could check what is painted, and the current match's own highlight. Today the painting is about 25 lines, and a seam for it would cost more than it saves.

**2026-11-13, after the review fixes.** A line break between two inline elements (`**bold**` and `*italic*` on consecutive lines) now matches a space; only whitespace beside a block keeps its line breaks (`isGapBetweenBlocks` in `page/src/find.ts`). Measured again (`npm run measure`, release build, median of 5), 1 MB mixed / prose: keystrokeToCount 15.2 / 14.8, firstKeystrokeToCount 152.8 / 134.9, longestLaterKeystrokeToCount 37.6 / 33.6, longestWhileTyping 0.0 / 0.0 (one run of the mixed document had a 101.2 ms task). Unchanged from 2026-11-12.

**2026-11-13, from the window design grilling.** Find's highlight colors move to the accent color: every match Light 3 in light and Dark 3 in dark, the current match Base. Window design ticket 03 makes that change; this ticket keeps today's colors until then. See [Window design](../../window-design/spec.md).

**2026-11-16, from the reader.** The find bar has no buttons to step back and forth through the matches; only the keys do it. Added as a box above: a previous and a next arrow button beside the count. They step through the page's existing next and previous, so keys and buttons can't disagree, and they take the focus ring and hover of the title bar's buttons.

**2026-11-16, from the reader.** The bar can only be closed with Esc, which a reader using the mouse doesn't know. Added as a box above: a close button after the arrow buttons, doing exactly what Esc does through the same code.

**2026-11-16, checked by hand and closed.** The reader went through every hand check above on the release build of commit abed67f, with the hand-check documents in `temp/handcheck/`, and found them all fine.
