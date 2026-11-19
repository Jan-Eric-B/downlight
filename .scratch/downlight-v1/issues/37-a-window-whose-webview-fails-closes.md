# 37: A window whose webview fails to start closes

**What to build:** When a window's webview can't be created, the reader is told why, and no empty window is left behind. Today the error box shows, and after it is dismissed an empty window stays open: nothing is in it, its title bar does nothing, and the process keeps running for it.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** closed

- [x] A window whose webview fails to start is closed, or never shown, and the process exits when it was the only window, as it does when the last window closes (ADR 0001).
- [x] The reader is still told why, as now ("Can't show **a.md**: its window didn't open: ...").
- [x] Tested first where a test can make the webview fail, or else checked by hand with the failure below.

## Comments

**2026-11-17, seen while measuring ticket 31.** A measurement run launched Downlight while WebView2 processes from the run before were still shutting down on Downlight's user data folder. The new window's webview failed with `WebView2 error: WindowsError(Error { code: HRESULT(0x800700AA), message: "The requested resource is in use." })` (ERROR_BUSY, which WebView2 gives when the user data folder is held by a browser process it can't share). Downlight showed its error box: "Can't show C:\...\mixed-20mb.md: its window didn't open: runtime error: failed to create webview: ...". After the box was dismissed, an empty Downlight window stayed on the desktop with the process running, and a later launch was handed to that process (ADR 0001's single instance), so it opened nothing either. The reader had to close the window by hand. The measurement script now waits until no Downlight or WebView2 process is left on the folder before it launches (ticket 31), so it no longer causes this, but anything else holding the folder can, such as an installed Downlight started with other browser arguments.

Where to look:

- `open_window` in `app/src/window.rs`: `platform::with_browser_arguments(builder).build()?` returns the error, but the native window Tauri made before the webview may already exist, and nothing closes it. The 5 s fallback that shows a hidden window (`LONGEST_HIDDEN`) is spawned only after `build()` succeeds, so it is not what shows this one; check whether Tauri or tao shows the window itself, or leaves it registered with the app.
- The launch path that reports the failure: `window::open_launch` and `window::failure_report`, then `platform::show_error` in `main.rs`'s single-instance handler and at startup. After the report, the process should exit when no window is left.
- `Documents::forget` (`app/src/documents.rs`) already forgets a window that never opened; check that a failed build leaves no record whose window keeps the process alive.

**2026-11-19, triage, decided by the repo owner.** In v1, and a blocker of ticket 18. The empty window keeps the process alive, so every later launch is handed to it (ADR 0001's single instance) and opens nothing until the reader closes that window by hand, which a default app for Markdown can't ship with.

**2026-11-19, built.** A window whose webview fails to start already left no window behind; the empty window in the measurement was most likely another one, whose webview had stopped. Such a window now closes once an error box has said why, and the process ends with its last window.

- **What the source says about a failed start** (tauri 2.12.5, tauri-runtime-wry 2.12.3, tao 0.37.2, wry 0.57.2). Downlight's own builder asks for the window hidden (`.visible(false)` in `open_window`, `app/src/window.rs`), and `create_window` in tauri-runtime-wry builds the native window so, then the webview with `?`. When the webview fails, the native window is dropped there, and tao's `Drop` posts its own destroy message, so the window is destroyed on the main thread and never shown. Tauri records a window only after `create_window` succeeds (`attach_window`), the 5 s fallback (`LONGEST_HIDDEN`) is spawned only after `build()` succeeds, and `Documents::open` forgets the label (`forget`). So nothing of the window is left to show or to keep the process alive. At startup, `setup` in `main.rs` already shows the box and exits when no window opened. For a second launch's failure, the process keeps running only for the windows it already had, as ADR 0001 wants. Nothing in this path changed, beyond a comment in `open_window` that records it.
- **What most likely happened instead.** Ticket 31's notes say the run before had left a Downlight process alive after `taskkill /T /F` failed partway, and `/T` ends the WebView2 processes in the tree. That process's window then had no webview: nothing in it, and the page-drawn title bar dead. The next run's launch was handed to it (ADR 0001), its new window's webview failed with ERROR_BUSY while WebView2 was still shutting down on the folder, and the box came from the single-instance handler. After the box, the window left was the old one, and every later launch was handed to the same process. This can't be confirmed from the code; the hand checks below cover both cases.
- **The fix: a window whose webview stops closes.** WebView2 raises `ProcessFailed` on the webview when one of its processes ends unexpectedly.
  - `platform::when_webview_stops` (`app/src/platform/windows.rs`) adds a handler for it when the window is built (`open_window` in `app/src/window.rs`). It calls back once, with why, when the browser process ends, since WebView2 then closes the webview for good. It also calls back when the main frame's render process ends, since WebView2 then shows its own error page, with no title bar to close the window by.
  - It ignores the failures WebView2 recovers from on its own (GPU, utility, sandbox helper and unknown processes, a frame-only render process), and an unresponsive render process, which a long task on a large document can cause (`why_the_window_stops`).
  - `close_stopped` (`app/src/window.rs`) then shows an error box off the main thread, "The window of C:\docs\a.md closed: WebView2's browser process ended" (or "A window that showed no document closed: ..."), and destroys the window once the box is dismissed. Destroying the last window ends the process, so a later launch starts a new one. The box comes first because ending the process would also end the box.
  - The window is destroyed rather than closed, so its size isn't kept for the next window. Other systems' fallback watches nothing (`app/src/platform/mod.rs`).
- **Spec.** The "One process" bullet now says a window whose webview fails to start never shows, and one whose webview stops closes after an error box.
- **Tests, test-first.** No test can make the webview fail: the app's tests have no webview, and none can be started for them. The tests cover what can be reached without one. `app/src/window/tests.rs`: `window_whose_webview_stopped_names_its_document_and_why`, `window_without_a_document_whose_webview_stopped_names_why`. `app/src/platform/windows.rs`: `window_stops_when_webview2_s_browser_or_render_process_ends`, `window_keeps_going_through_failures_webview2_recovers_from`. The rest is the hand checks.
- **Left as it is.**
  - When WebView2's browser process ends, every window gets a box of its own, each from its own thread (`close_stopped`), so they come up at the same time, stacked.
  - A window whose webview stops before its page first asked to be shown can still come up empty while its box is open, through the 5 s fallback. It closes with the box.
  - Should a measurement run's WebView2 processes end before `downlight.exe` again, that process now shows a box and waits for a click. `npm run measure` then stops after its 60 s wait instead of handing its next launch to a dead window.
  - A render process that ends could be recovered by loading the page again instead of closing the window. That was not chosen: the page would start over without its reading position, and a document that crashed the renderer would crash it again.

**Hand checks** for the owner, on the release build (`target\release\downlight.exe`):

1. **A webview that fails to start, at startup.** In a new Command Prompt, `set WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=C:\no-webview2-here`, then start `downlight.exe` on a document from that prompt. WebView2 reads that variable and finds no runtime there. One or two error boxes come up: Tauri's own "Could not find the WebView2 Runtime" may come first, then "Can't show ...: its window didn't open: ...". After the last box, no Downlight window is on the desktop, and Task Manager lists no `downlight.exe`. Close that prompt; opening the document normally works.
2. **A webview that stops, with two windows.** Open two documents. In Task Manager's Processes tab, expand Downlight and end the WebView2 process named "WebView2 Manager" (the browser process). A box "The window of ... closed: WebView2's browser process ended" comes up for each window. Each window closes after its box's OK, and after the last one no `downlight.exe` is left. Opening a document afterwards opens it normally.
3. **A render process that stops.** With one document open, end the WebView2 process under Downlight that draws its window. In Task Manager's Details tab, add the Command line column: it is the `msedgewebview2.exe` with `--type=renderer`. The box says "...: WebView2's render process ended". After OK, the window closes and the process ends.
4. **Nothing else closes a window.** Open a large document (the 20 MB test document from `npm run documents -- large`) and scroll through it while it renders. No box comes up, and the window stays.

**2026-11-19, review fixes.**

- **Two statements above corrected.** The window is hidden because Downlight's own builder asks for it so (`.visible(false)` in `open_window`), not because tauri-runtime-wry does; the conclusion holds, as tao's `Drop` destroys the native window and the 5 s fallback is spawned only after `build()` succeeds. And the boxes for several windows come up at the same time, stacked, since each `close_stopped` shows its own from its own thread, not one after the other; that is left as it is.
- **Failures to watch a window are said to be left at that.** `when_webview_stops` (`app/src/platform/windows.rs`) drops the errors of reaching the webview and of adding the handler, as `leave_keys_to_the_page` does. Its doc comment now says why: both fail only once the webview has closed or the app is ending, with no page left to watch, so the window is left unwatched, as before this ticket.

**2026-11-19, hand checks passed.** The repo owner ran hand checks 1 to 4 on the release build of commit 03b5e1d: a webview that fails at startup, a browser process that ends with two windows open and a render process that ends each leave no window and no process behind, and a large document scrolled while it renders brings up no box. The box is ticked and the ticket is closed.
