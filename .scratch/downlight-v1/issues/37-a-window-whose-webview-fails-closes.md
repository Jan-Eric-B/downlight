# 37: A window whose webview fails to start closes

**What to build:** When a window's webview can't be created, the reader is told why, and no empty window is left behind. Today the error box shows, and after it is dismissed an empty window stays open: nothing is in it, its title bar does nothing, and the process keeps running for it.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] A window whose webview fails to start is closed, or never shown, and the process exits when it was the only window, as it does when the last window closes (ADR 0001).
- [ ] The reader is still told why, as now ("Can't show **a.md**: its window didn't open: ...").
- [ ] Tested first where a test can make the webview fail, or else checked by hand with the failure below.

## Comments

**2026-11-17, seen while measuring ticket 31.** A measurement run launched Downlight while WebView2 processes from the run before were still shutting down on Downlight's user data folder. The new window's webview failed with `WebView2 error: WindowsError(Error { code: HRESULT(0x800700AA), message: "The requested resource is in use." })` (ERROR_BUSY, which WebView2 gives when the user data folder is held by a browser process it can't share). Downlight showed its error box: "Can't show C:\...\mixed-20mb.md: its window didn't open: runtime error: failed to create webview: ...". After the box was dismissed, an empty Downlight window stayed on the desktop with the process running, and a later launch was handed to that process (ADR 0001's single instance), so it opened nothing either. The reader had to close the window by hand. The measurement script now waits until no Downlight or WebView2 process is left on the folder before it launches (ticket 31), so it no longer causes this, but anything else holding the folder can, such as an installed Downlight started with other browser arguments.

Where to look:

- `open_window` in `app/src/window.rs`: `platform::with_browser_arguments(builder).build()?` returns the error, but the native window Tauri made before the webview may already exist, and nothing closes it. The 5 s fallback that shows a hidden window (`LONGEST_HIDDEN`) is spawned only after `build()` succeeds, so it is not what shows this one; check whether Tauri or tao shows the window itself, or leaves it registered with the app.
- The launch path that reports the failure: `window::open_launch` and `window::failure_report`, then `platform::show_error` in `main.rs`'s single-instance handler and at startup. After the report, the process should exit when no window is left.
- `Documents::forget` (`app/src/documents.rs`) already forgets a window that never opened; check that a failed build leaves no record whose window keeps the process alive.

**2026-11-19, triage, decided by the repo owner.** In v1, and a blocker of ticket 18. The empty window keeps the process alive, so every later launch is handed to it (ADR 0001's single instance) and opens nothing until the reader closes that window by hand, which a default app for Markdown can't ship with.
