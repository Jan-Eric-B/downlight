# 14: Find

**What to build:** Ctrl+F opens a find bar that highlights every match in the shown document as the reader types, shows "3 of 12", and steps through the matches. **Find** looks only in the shown document.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 09 (Opening from inside a window)

**Status:** ready-for-agent

- [ ] Ctrl+F opens the find bar with the focus in its box; pressing it again while open selects the text in the box.
- [ ] Matches are highlighted as the reader types, ignoring case, without changing the document's DOM.
- [ ] The bar shows "N of M", or "No results" when nothing matches.
- [ ] Enter and F3 go to the next match, Shift+Enter and Shift+F3 to the previous one, wrapping at the ends. The current match scrolls into view and is highlighted differently from the rest.
- [ ] Esc closes the bar and clears the highlights.
- [ ] After a reload while the bar is open, the matches are counted again.
- [ ] Find is in the right-click menu.
- [ ] Page tests cover the count, the current match, next and previous, wrapping, Esc and the count after a reload.
- [ ] Typing in the box on the 1 MB document is measured with the timing switch, and the result is recorded in this ticket.

## Plan

**Approach:** A page module `page/src/find.ts` owns the find bar, which sits outside the `article`. Once per shown document, and only when Find needs it, it builds a text index with a TreeWalker: the view's text nodes joined into one case-folded string with node offsets. Each keystroke runs `indexOf` over that string, so a match can span inline formatting. Matches are kept in document order as plain `{startContainer, startOffset, endContainer, endOffset}` records, which the overview ruler can read later, and painted with two CSS Custom Highlights, `find-match` and `find-current` (priority 1), built from `StaticRange`, so the document's DOM never changes. Live Ranges aren't used for painting: about 100k of them would slow every later DOM change, including the next `innerHTML`. happy-dom 21.0.4 has no `CSS.highlights`, `Highlight` or `StaticRange` (WebView2 has had them since Chromium 105), so painting sits behind one `"highlights" in CSS` check and tests read the same records the highlights are built from, with no fake. The recount hooks into the one place the page puts a rendered document in place, `showDocument` inside `startDocumentView` in `page/src/view.ts`.

**Tests first:**

- `page/src/find.test.ts` setup: `startFind`, `startCommands` and `startDocumentView` with `mockIPC` answering the new `find.html` fixture, as `view.test.ts` does. Keys are dispatched `keydown` events; typing sets `box.value` and dispatches `input`. Observed: the bar's text, `document.activeElement`, the box's selection, `findMatches()` and `view.innerHTML`.
  - `ctrl_f_opens_the_bar_with_the_focus_in_its_box` - default prevented, so WebView2's own find bar stays away
  - `ctrl_f_while_open_selects_the_text_in_the_box`
  - `typing_counts_matches_ignoring_case` - "lamp" over Lamp, LAMP and lamp gives "1 of N"
  - `a_match_across_inline_formatting_counts_once` - "lamp light" across `<strong>`
  - `nothing_matching_shows_no_results` - and an empty box shows no count and has no matches
  - `finding_leaves_the_documents_dom_unchanged` - `view.innerHTML` equal before typing, after stepping and after Esc
  - `the_first_match_after_the_reading_position_is_current` - with a block halfway down at the top of the view (stub `getBoundingClientRect`, as `view.test.ts` does for Reload), the current match is the first one at or after that block
  - `at_the_top_of_the_document_the_first_match_is_current`
  - `a_current_match_in_a_closed_section_opens_it` - every closed `<details>` around the current match gets `open`
  - `enter_and_f3_go_to_the_next_match` / `shift_enter_and_shift_f3_go_to_the_previous_match`
  - `next_after_the_last_match_wraps_to_the_first` / `previous_before_the_first_wraps_to_the_last`
  - `the_current_matchs_block_is_scrolled_into_view` - spy on `Element.prototype.scrollIntoView` (happy-dom has no layout; where it lands is a manual check)
  - `esc_closes_the_bar_and_clears_the_matches`
  - `a_document_shown_while_the_bar_is_open_is_counted_again` - `emit("document-shown")` with `notes.html`; once ticket 10 has landed, the same through its reload path
  - `a_new_document_does_not_scroll_to_a_match` - so Reload's anchoring and a link's heading win
- `commands.test.ts`: the menu test expects `{kind:"command", id:"find", label:"Find", shortcut:"Ctrl+F"}`; add `runs_find_chosen_in_the_menu`.

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

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- `whenDocumentShown` exists in `page/src/view.ts`. It runs on the first show, on another document, and after a reload once the view is back at the reading position. Recount there.
- `readingLine(view)` exists but is module-private, since nothing outside `view.ts` used it yet. Export it to find the first match after the reading position.
- `Command` already has `ctrlLetter?` and `key?`, and Reload is in the menu. Find goes after Reload, in the spec's order.
- Ctrl+F is one of WebView2's own browser keys (it opens WebView2's find bar). Ticket 10 found that cancelling a key in the page doesn't always stop WebView2. If `preventDefault` doesn't hold for Ctrl+F, F3 or Shift+F3, extend the existing `AcceleratorKeyPressed` handler (`leave_reload_keys_to_the_page` in `app/src/platform/windows.rs`, renamed for what it covers) rather than adding a second handler.
- To keep each task under 100 ms on the 1 MB document, ticket 10 parses a reload's HTML into a `<template>` and swaps it in a frame later. Building the text index can follow the same rule: measure it with the timing switch, and split it across tasks if it doesn't fit in one.
