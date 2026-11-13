# 29: The page owns every key

**What to build:** WebView2's browser keys are turned off as a whole, and the page's command table becomes the one place that says which key does what. Today the keys the page handles are listed twice, in the page's command table and in the app's WebView2 key handler, and tickets 12 and 16 would each extend both.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] The app sets `AreBrowserAcceleratorKeysEnabled` to false on each webview (`ICoreWebView2Settings3`). Microsoft documents that this turns off Find (Ctrl+F, Ctrl+G, F3), every reload (F5, Ctrl+R, Ctrl+F5, Shift+F5, Ctrl+Shift+R, the Refresh key), zoom (Ctrl+plus, Ctrl+minus, Ctrl+0), Back and Forward (Alt+Left, Alt+Right, the browser keys), Print (Ctrl+P), Downloads (Ctrl+J), caret browsing (F7), Stop (Esc) and DevTools, and keeps Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+A, Ctrl+Z and the movement keys ([Differences between Microsoft Edge and WebView2](https://learn.microsoft.com/microsoft-edge/webview2/concepts/browser-features#additional-keyboard-shortcuts-information)).
- [ ] `leave_reload_and_find_keys_to_the_page`, `is_reload_or_find_key` and their tests in `app/src/platform/windows.rs` go. A setting that can't be applied doesn't fail the window, as today.
- [ ] Dev builds keep the DevTools keys (F12, Ctrl+Shift+I), for example by enabling those keys alone through `IsBrowserAcceleratorKeyEnabled`.
- [ ] `Command` in `page/src/commands.ts` takes a key with its modifiers, replacing `ctrlLetter`, `key` and `shift`, so Alt+Left (ticket 12) and Ctrl+plus (ticket 16) fit without a special case. The menu shows each command's shortcut from it. `isWebviewReload` goes if nothing is left for it to swallow.
- [ ] Checked by hand in the release build: Ctrl+P, Ctrl+J, F7 and Esc do nothing of WebView2's own; Ctrl+F, F3, F5, Ctrl+R and Ctrl+Shift+R do only the page's; Ctrl+C and Ctrl+A still work in the document and in the find box; zoom keys do nothing until ticket 16 gives them commands. Ctrl+wheel is a separate setting (`IsZoomControlEnabled`) and the mouse's side buttons aren't keys; note what they do.

## Comments

**2026-11-13, from the architecture review.** Ticket 10 kept WebView2's zoom keys on purpose, which was the reason to turn off only some keys. Ticket 16 takes zoom over and lists this as its option 2, so that reason goes. Ctrl+P, Ctrl+J, F7 and Esc still reach WebView2 today: Ctrl+P opens its print dialog, which the spec only plans for v1.x, and then as a command of the page's own.
