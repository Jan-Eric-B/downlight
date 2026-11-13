# 10: Reload

**What to build:** When a document's file is saved, Downlight **reloads** it at the same reading position, also for editors that save by writing a new file and renaming it over the old one. F5 and Ctrl+R reload by hand. A deleted file leaves the last render visible with a bar saying so, and the document reloads when the file comes back.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 04 (Documents look like GitHub on Mica), 09 (Opening from inside a window)

**Status:** ready-for-agent

- [x] The app's watching takes a document's path and reports Changed, Gone and Back. It watches the file's folder and reports a change 150 ms after the last event. On Changed and Back the app reads the file again and passes its bytes to the core's render.
- [x] Saving in place and saving by renaming a new file over the old one both reload the document.
- [x] After a reload, the block that was at the top of the view (found by its line number) is at the top again.
- [x] F5 and Ctrl+R reload by hand.
- [x] When the file is deleted or moved, a bar says "File deleted or moved" and the last render stays visible.
- [x] When the file comes back, the document reloads and the bar goes away.
- [x] Reload is in the right-click menu.
- [x] App tests of the watching (`cargo test`, no window) with real temporary files cover a rewrite in place, a rename over the file, a delete and a recreate. A page test covers which block a reload anchors to.

## Plan

**Approach:** First the `Timing` refactor from the comment below: `app/src/timing.rs` gets a `Stages` clock (`Stages::start(event)`, `.lap("read")`, `.lap("render")`) and `Timing::report(window, &stages)`, which writes the same `timing <window> <event> at=.. read=.. render=..` line as today; `Timing::page_url(backdrop)` replaces `page_path`. Then a new `app/src/watch.rs` with `Watch::start(path, on_change)`: `notify` watches the file's folder (NonRecursive), keeps events for the file's name (case-insensitive), and a background thread waits until 150 ms pass with no event, then decides by whether the file exists (existed and exists: Changed; existed and missing: Gone; was missing and exists: Back), since notify's event kinds differ between platforms. Each window's `ShownDocument` in `app/src/window.rs` owns its `Watch`, so whenever `window::show` gives a window a document (Open, drop, a link from ticket 11), the old watch is dropped and a new one starts. On Changed, Back or F5 the app starts a new read and render ahead and emits `document-changed` `"reload"`; on Gone it emits `"gone"`. The page shows the reload anchored by line number, then runs the `whenDocumentShown` listeners in `page/src/view.ts` (see "Fits with other tickets"), which is how Find recounts.

**Tests first:**

- `watch.rs` (`cargo test`, real files in a temp folder per test):
  - `rewrite_in_place_reports_changed`
  - `rename_over_the_file_reports_changed` - `x.tmp` renamed over the file gives Changed, not Gone
  - `delete_reports_gone`
  - `recreate_after_delete_reports_back`
  - `writes_in_quick_succession_report_one_change` - the 150 ms debounce
  - `change_to_another_file_in_the_folder_reports_nothing`
- `window.rs`: `reload_ahead_reads_the_file_again` (after `reload_ahead`, `render_of` gives the new text); `watch_for_a_document_the_window_has_left_is_dropped` (`keep_watch` with the old key after the window claimed another document gives false; this is the seam ticket 11 relies on).
- `timing.rs`: `report_line_keeps_the_measure_format` (e.g. `timing document-0 reload at=1.0 read=2.0 render=3.0`); `page_url_tells_the_page_about_timing_and_backdrop` (moved from `window.rs`).
- `page/src/commands.test.ts`: "F5 reloads", "Ctrl+R reloads (QWERTY and the Cyrillic key in R's place)", each invoking `reload_document` with the default prevented; "Ctrl+F5, Shift+F5 and Ctrl+Shift+R are swallowed" (WebView2 must not reload the page itself); the menu test lists `{kind:"command", id:"reload", label:"Reload", shortcut:"F5"}`; "runs Reload chosen in the menu".
- `page/src/view.test.ts` (stub `getBoundingClientRect` on blocks, spy on `Element.prototype.scrollIntoView`):
  - "a reload puts the block that was at the top of the view back at the top"
  - "a reload anchors to the innermost block crossing the top of the view" (an `li` in a long list)
  - "a reload anchors to the last block starting before that line when none starts on it"
  - "a reload at the top of the document stays at the top"
  - "a reload that can't read the file keeps the last render and says why"
  - "a deleted file leaves the last render under a 'File deleted or moved' bar"
  - "a file that comes back reloads and the bar goes away"
  - "showing another document removes the bar"
  - "a reload runs the whenDocumentShown listeners once its render is in place and anchored"
  - "a reload overtaken by a newer document-shown shows the newer document"

**Changes:**

1. `app/src/timing.rs` - `Stages` (start `SystemTime` and `Instant`, laps of `(&'static str, Duration)`), `Timing::report(window, &stages)` with a pure `line()` for the test; remove `ReadAndRenderTimes` and `read_and_render`; add `page_url(backdrop)`; add a `report_reload` command for the page's marks, writing `timing <w> reload-shown at=<received> insert=<ms> anchor=<ms>`. The event name goes in `Stages::start`, not in `timing.stages(window, ..)` as the comment suggests, because a render ahead starts in `claim` before the window's `Timing` is at hand.
2. `app/src/render.rs` (the comment calls it `document.rs`) - `start(path, event)`; `ReadAndRender` carries `Stages` (`"read-and-render"` or `"reload"`) and `render_document` reports them. Add `#[tauri::command] reload_document(window)`: `OpenWindows::reload_ahead`, then emit `document-changed` `"reload"`, so F5 and the watch share one path.
3. `cargo add notify -p downlight` (8.x; CC0-1.0, notify-types MIT OR Apache-2.0, both allowed in `deny.toml`). No `notify-debouncer-mini`: it throttles per timeframe, while the ticket wants 150 ms after the last event (about 15 lines of `recv_timeout`). Run `cargo deny check`.
4. `app/src/watch.rs` - `pub enum Change { Changed, Gone, Back }`; `pub struct Watch` owns the `RecommendedWatcher`, and dropping it ends the debounce thread; `Watch::start(&Path, impl FnMut(Change) + Send + 'static) -> notify::Result<Watch>`.
5. `app/src/window.rs` - `ShownDocument` gets `watch: Option<Watch>`; replacing the entry in `claim` drops the old watch. After `Claim::New`/`Changed`, `show` runs `spawn_blocking`: `Watch::start`, then `OpenWindows::keep_watch(label, key, watch)`, kept only if the window still shows that key. The callback runs `reload_ahead` and emits `"reload"` on Changed and Back, `"gone"` on Gone. A watch that can't start gives the window a notice. Use `timing.page_url`.
6. `app/src/main.rs` - `mod watch`; register `render::reload_document` and `timing::report_reload`.
7. `app/capabilities/document-windows.json` - description only (reloads, a deleted file); app commands need no permission. Run `cargo clean -p tauri --release` before the release build anyway.
8. `page/src/commands.ts` - the shared `Command` shape (see below): add `reload: { ctrlLetter: "r", key: "F5", run: () => void invoke("reload_document") }`, swallow Ctrl+F5, Shift+F5 and Ctrl+Shift+R (Tauri 2.12 doesn't expose wry's `with_browser_accelerator_keys`), and put Reload in the menu in the spec's order.
9. `page/src/view.ts` and `page/src/document.css` - `listenToApp("document-changed")`:
   - `"gone"`: a sticky `.file-gone` bar, `role="status"`, "File deleted or moved".
   - `"reload"`: take the anchor line (start line in `data-sourcepos` of the innermost block crossing the top of the view), fetch `render_document` under the same `latestRequest` guard, insert the HTML and `addCopyButtons`, `scrollIntoView({block: "start"})` the block starting on that line or else the last one before it, and scroll again after the next frame because `content-visibility` only estimates heights (ADR 0003). Then hide the bar, run the `whenDocumentShown` listeners and call `reportReload`.
   - A failed reload keeps the render and calls `showNotice(error)`.
10. `page/src/timing.ts` - `reportReload({received, inserted, anchored})`.
11. `scripts/measure.js` - a "Reload, 1 MB" case: copy `mixed1mb` to its own file, open it, scroll to the middle over the debug port (the CDP `evaluate` helper, shared with ticket 14), append a line, wait for `reload-shown`; report read, render, insert, anchor and the longest task after. `[readAndRender]` stays the first report, so existing cases are unchanged.

**Fits with other tickets:**

- `whenDocumentShown(listener)` in `page/src/view.ts` is the one hook for "a rendered document is now in place" (first show, Reload, a link). Ticket 14 plans it; whichever ticket lands first adds it. Reload calls it after anchoring, instead of firing its own event.
- `Command` in `page/src/commands.ts` becomes `{ ctrlLetter?: string; key?: string; run(): void }`: `key` is a named key without modifiers (`"F5"`). The menu shows `key` if set, else `Ctrl+<letter>`, else no shortcut (ticket 15's Choose editor...). Whichever of 10, 14 and 15 lands first makes this change.
- Menu order follows the spec: Back, Forward, Reload, Find, Edit, Choose editor..., Open, Zoom, Copy.
- Ticket 11 changes `window::show` to take a heading and return `Shown`; the watch restarts on the same `Claim::New`/`Changed` path either way.
- The anchor line lookup (start line of the innermost block crossing the top of the view) is exported from `page/src/view.ts` as `readingLine()`; ticket 14's Find starts at the first match after it.

**Verification:** `npm run check`. `npm run tauri build`, then `npm run measure`, and record the reload numbers in this ticket; also `DOWNLIGHT_TIMING=1 target/release/downlight.exe doc.md`, save the file and read the `reload` and `reload-shown` lines. By hand: saves from VS Code, Notepad++ and the packaged Notepad, and `Move-Item -Force` over the file; delete, then restore from the Recycle Bin; the reading position mid-document on the 1 MB file; F5, Ctrl+R and the menu's Reload; Ctrl+F5 doesn't reload the webview; a window given another document by Ctrl+O no longer reloads on saves to the old file.

**Decisions (2026-11-12):**

- WebView2's own reload keys (Ctrl+F5, Shift+F5, Ctrl+Shift+R): block them in the page and check each by hand. If one still reloads the webview, turn WebView2's browser shortcut keys off through the native webview handle (to be confirmed when building: Tauri's `with_webview` and `ICoreWebView2Settings3::SetAreBrowserAcceleratorKeysEnabled(false)`), and have the page handle the shortcuts that this switches off.
- The reading position stays the line number of the block at the top of the view, as the spec says. A reload after lines were added above the view may land a few blocks off; that is accepted.
- The tickets are built one at a time, so each shared piece in "Fits with other tickets" is added by the first ticket that needs it.
- Revised when building: Ctrl+Shift+R still reloaded the webview with the page cancelling it (Ctrl+F5 and Shift+F5 held). Turning all browser keys off would also turn off Ctrl+plus, Ctrl+minus and Ctrl+0 zoom before ticket 16 replaces them, so instead an `AcceleratorKeyPressed` handler (`leave_reload_keys_to_the_page` in `app/src/platform/windows.rs`, with `webview2-com` and `windows-core` at wry's versions) turns off WebView2's handling of only F5, the Refresh key and Ctrl+R with any modifiers. The page still gets them.

## Results

Built 2026-11-12. `npm run check` passes.

**Beyond the plan:**

- The watch decides by the file's size and its write and creation times, not by whether it exists alone: Windows reports attribute and security changes as modifications, and a virus scanner touching the file after a save gave a second reload. Test `attribute_change_reports_nothing`.
- The page keeps the bar when the file goes away while a reload is on its way (seen in the real app with a 600 ms render). Test "a file deleted while its reload is on the way keeps the bar".
- A reload parses the new HTML into a `<template>` and adds the copy buttons there, waits a frame, then swaps it in and anchors. In one task, inserting and anchoring the 1 MB document took a frame of 124 ms (median, worst 308 ms).
- `whenDocumentShown` runs on first show and when another document is shown, too.

**Measured** (`npm run measure`, median of 5, release build): a save in the middle of the 1 MB mixed document, until the reload is shown.

| Measure | Reload, 1 MB mixed |
| --- | ---: |
| shownSinceSave (includes the 150 ms of quiet) | 818.2 |
| read | 80.3 |
| render | 470.5 |
| insert (HTML arrived until swapped in, parse and a frame included) | 49.2 |
| anchor | 47.1 |
| longestAfterReload | 0.0 (worst 0.0) |
| linesMoved (line at the top of the view, before and after) | 0.0 |

**Checked in the release build** over WebView2's debug port with real key presses: a save in place and a rename over the file each reload once, with line 9448 of the 1 MB document still at the top of the view; deleting the file shows the bar over the last render; restoring it reloads and removes the bar; F5 and Ctrl+R reload; Ctrl+F5, Shift+F5 and Ctrl+Shift+R leave the page loaded; Ctrl+plus and Ctrl+0 still reach the webview.

**Not yet checked by hand:** saves from VS Code, Notepad++ and the packaged Notepad; restoring from the Recycle Bin; the menu's Reload; a window given another document by Ctrl+O no longer reloading on saves to the old file (covered by `watch_for_a_document_the_window_has_left_is_dropped`).

**After the code quality review (2026-11-12):**

- Reload's app code is in `app/src/reload.rs` (the `reload_document` command, `watch_document`, the `document-changed` event), notices in `app/src/notice.rs`. `window.rs` keeps the window table; its tests moved to `app/src/window/tests.rs`.
- `Claim::New` and `Claim::Changed` carry the document's key, so the watch is kept under the key `claim` computed rather than one worked out again.
- A watch that can't start no longer loses its notice in a new window: `OpenWindows::hold_notice` keeps it until the page first asks for its document, and `render_document` shows it then.
- `keep_watch` returns nothing; its tests watch the callback's channel close when the watch is dropped. `OpenWindows::shows` (the guard against a change reported after the window left the document) has tests of its own.
- `platform::leave_reload_keys_to_the_page` returns nothing and never fails the window.
- In `page/src/view.ts`, one `place(content)` shows the render, an error or the empty state; `readingLine` is module-private until Find needs it.

## Comments

**2026-11-03, architecture review.** Reload is the second piece of work the timing switch measures (its read, render and anchoring, as `CODING_STANDARDS.md` asks for work that grows with a document's size). Today each measurement needs a public struct and a `Timing` method of its own: `app/src/document.rs` keeps its own `SystemTime` and `Instant` to fill `ReadAndRenderTimes` (`app/src/timing.rs`), and `open_window` in `app/src/window.rs` builds the `?timing` page URL itself. Before adding Reload's measurement, give `Timing` stages, for example `timing.stages(window, "reload")` with `.lap("read")` and `.lap("render")` and a report when it ends, so the clock work stays in `timing.rs` and the read-and-render measurement uses the same call. `Timing` can hand out the page URL too. The line format `timing <window> <event> key=value ...` is what `scripts/measure.js` reads, so keep it. Expected size: about 70 lines across `timing.rs`, `document.rs` and `window.rs`.

**2026-11-13, after the review fixes.** `blockAt` (`page/src/reading-position.ts`) no longer anchors to a footnote that starts before the reading line, which sent the view to the document's end after an edit above the reader. Measured again (`npm run measure`, release build, median of 5): anchor 58.1 ms (47.1 before; each block before the line now also checks whether it is in the footnotes), insert 59.3, render 484.1, read 229.0, longestAfterReload 0.0 (worst 0.0), linesMoved 0.0. No task or frame over 100 ms.
