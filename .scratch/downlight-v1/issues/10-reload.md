# 10: Reload

**What to build:** When a document's file is saved, Downlight **reloads** it at the same reading position, also for editors that save by writing a new file and renaming it over the old one. F5 and Ctrl+R reload by hand. A deleted file leaves the last render visible with a bar saying so, and the document reloads when the file comes back.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 04 (Documents look like GitHub on Mica), 09 (Opening from inside a window)

**Status:** ready-for-agent

- [ ] The app's watching takes a document's path and reports Changed, Gone and Back. It watches the file's folder and reports a change 150 ms after the last event. On Changed and Back the app reads the file again and passes its bytes to the core's render.
- [ ] Saving in place and saving by renaming a new file over the old one both reload the document.
- [ ] After a reload, the block that was at the top of the view (found by its line number) is at the top again.
- [ ] F5 and Ctrl+R reload by hand.
- [ ] When the file is deleted or moved, a bar says "File deleted or moved" and the last render stays visible.
- [ ] When the file comes back, the document reloads and the bar goes away.
- [ ] Reload is in the right-click menu.
- [ ] App tests of the watching (`cargo test`, no window) with real temporary files cover a rewrite in place, a rename over the file, a delete and a recreate. A page test covers which block a reload anchors to.

## Comments

**2026-11-03, architecture review.** Reload is the second piece of work the timing switch measures (its read, render and anchoring, as `CODING_STANDARDS.md` asks for work that grows with a document's size). Today each measurement needs a public struct and a `Timing` method of its own: `app/src/document.rs` keeps its own `SystemTime` and `Instant` to fill `ReadAndRenderTimes` (`app/src/timing.rs`), and `open_window` in `app/src/window.rs` builds the `?timing` page URL itself. Before adding Reload's measurement, give `Timing` stages, for example `timing.stages(window, "reload")` with `.lap("read")` and `.lap("render")` and a report when it ends, so the clock work stays in `timing.rs` and the read-and-render measurement uses the same call. `Timing` can hand out the page URL too. The line format `timing <window> <event> key=value ...` is what `scripts/measure.js` reads, so keep it. Expected size: about 70 lines across `timing.rs`, `document.rs` and `window.rs`.
