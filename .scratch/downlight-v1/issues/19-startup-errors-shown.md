# 19: Startup errors are shown

**What to build:** When Downlight can't open its window, for example because WebView2 is missing or can't create its webview, the reader sees a native message box that says what went wrong, instead of the process ending without a word. `CODING_STANDARDS.md` asks that every error end up handled or shown to the reader, and today this one is neither.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 08 (One process, a window per document)

**Status:** ready-for-agent

- [ ] A failure while creating a window shows a native message box with the error, and the process exits with a non-zero code.
- [ ] An error that `tauri::Builder::run` returns to `main` is shown the same way.
- [ ] The message box lives in the app's per-OS module behind its one interface, and its `unsafe` block carries a `SAFETY:` comment. It uses the `windows` crate that Tauri already depends on (0.62), so no second version of it enters the tree.
- [ ] Checked by hand on the release build: with a Downlight window open under a different `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` (for example `--remote-debugging-port=9333`), a second launch shows the message box.

## Comments

**2026-11-03, review.** Tauri 2.12.5 doesn't return an error from the setup hook through `run()`: it panics with "Failed to setup app" (`tauri-2.12.5/src/app.rs:1444`). So the `?` after `document::open_window` in `app/src/main.rs` and the `tauri::Result` that `main` returns look like handling, but a window that fails to build ends in that panic. Release builds have no console (`windows_subsystem = "windows"`), so the panic message goes nowhere and the process just exits. Seen during the review: a WebView2 instance left over from a test run held Downlight's profile folder with other browser arguments, and the next launch failed with HRESULT 0x8007139F and exited silently. Handling the error inside the setup hook, rather than returning it, avoids depending on Tauri's panic.

**2026-11-03, architecture review.** `window::open_launch` now returns the launch's failures (`Vec<LaunchFailure>`, `app/src/window.rs`); each one's message names the document and the error, and an empty list means every window opened. Both callers in `app/src/main.rs` print them on stderr for now, so this ticket replaces those two loops with the message box. The first launch's `setup` still returns an `Err` built from the messages when no window opened, which is the Tauri panic above; handle it in the hook instead. A second launch's failures arrive in the first process after the second launch has returned, so its message box comes from the running process.
