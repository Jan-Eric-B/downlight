# 19: Startup errors are shown

**What to build:** When Downlight can't open its window, for example because WebView2 is missing or can't create its webview, the reader sees a native message box that says what went wrong, instead of the process ending without a word. `CODING_STANDARDS.md` asks that every error end up handled or shown to the reader, and today this one is neither.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 08 (One process, a window per document)

**Status:** ready-for-agent

- [x] A failure while creating a window shows a native message box with the error, and the process exits with a non-zero code.
- [x] An error that `tauri::Builder::run` returns to `main` is shown the same way.
- [x] The message box lives in the app's per-OS module behind its one interface, and its `unsafe` block carries a `SAFETY:` comment. It uses the `windows` crate that Tauri already depends on (0.62), so no second version of it enters the tree.
- [ ] Checked by hand on the release build: with a Downlight window open under a different `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` (for example `--remote-debugging-port=9333`), a second launch shows the message box.

## Comments

**2026-11-03, review.** Tauri 2.12.5 doesn't return an error from the setup hook through `run()`: it panics with "Failed to setup app" (`tauri-2.12.5/src/app.rs:1444`). So the `?` after `document::open_window` in `app/src/main.rs` and the `tauri::Result` that `main` returns look like handling, but a window that fails to build ends in that panic. Release builds have no console (`windows_subsystem = "windows"`), so the panic message goes nowhere and the process just exits. Seen during the review: a WebView2 instance left over from a test run held Downlight's profile folder with other browser arguments, and the next launch failed with HRESULT 0x8007139F and exited silently. Handling the error inside the setup hook, rather than returning it, avoids depending on Tauri's panic.

**2026-11-03, architecture review.** `window::open_launch` now returns the launch's failures (`Vec<LaunchFailure>`, `app/src/window.rs`); each one's message names the document and the error, and an empty list means every window opened. Both callers in `app/src/main.rs` print them on stderr for now, so this ticket replaces those two loops with the message box. The first launch's `setup` still returns an `Err` built from the messages when no window opened, which is the Tauri panic above; handle it in the hook instead. A second launch's failures arrive in the first process after the second launch has returned, so its message box comes from the running process.

**2026-11-09, implementation.**

- **Message box:** `platform::show_error` (`app/src/platform/windows.rs`) calls `MessageBoxW` with no owner window, an error icon and `MB_SETFOREGROUND`, captioned `Downlight`; other systems print on stderr until their ports. It uses `windows-sys` 0.61.2, which the app already depends on for `process_start`, with the `Win32_UI_WindowsAndMessaging` feature added, rather than the `windows` 0.62 crate the criterion names: same aim (no second version of a crate enters the tree), and no new direct dependency. `Cargo.lock` is unchanged.
- **Report:** `window::failure_report` turns a launch's failures into one message, a line per failure, or None when every window opened, so a launch shows at most one box.
- **First launch:** the setup hook no longer returns an `Err`. With no window open, it shows the box on the main thread and then calls `std::process::exit(1)`. With some windows open, it shows the box on a thread of its own, so the open windows keep working while the box is up.
- **Second launch:** the running process shows the box from the blocking task that opened the windows, off the main thread. The second launch itself exits with 0, since it handed its documents over.
- **`run()` error:** `main` returns `ExitCode` now, and shows `Downlight can't start: <error>` before exiting with `FAILURE`. Not triggered by hand: nothing found makes `run()` return an error on demand.
- `APP_NAME` moved from `window.rs` to the crate root, shared by the titles and the box's caption.

**Hand check, 2026-11-09, release build.** The scenario in the last criterion does not fail: the second launch hands its document to the running process (single instance, ADR 0001), and that process opens the window fine with its own WebView2 arguments. Holding Downlight's profile folder with Edge (`--user-data-dir`, other arguments) instead made WebView2 hang while building the webview, not return an error: the window stays hidden, and the 5 s fallback can't show it because the main thread is blocked in the setup hook. That hang is not covered by this ticket. Killing a Downlight started with `--remote-debugging-port=9333` and relaunching at once didn't recreate the review's leftover instance either: its WebView2 processes ended with it.

So the error paths were checked with a release build temporarily patched so that `open_window` fails for documents named `fail-on-purpose*` (patch reverted, release rebuilt without it). The boxes were read from their dialog windows:

- Launch with only a failing document: a `Downlight` box with `Can't open a window for C:\work\fail-on-purpose.md: window not found`; the process waits for it and exits with code 1 once it is dismissed.
- Launch with `github.md` and a failing document: the document's window opens and is visible while the box is up.
- A second launch with a failing document while that box was still up: the second launch exits with 0, and the running process shows a second box. Its handover being handled shows the main thread stays free while a box is up.

Still open: the last criterion's real WebView2 failure (HRESULT 0x8007139F), for want of a way to bring it about, and the WebView2 hang above, which needs its own ticket if it should be handled.
