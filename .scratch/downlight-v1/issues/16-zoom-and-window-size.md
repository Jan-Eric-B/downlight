# 16: Zoom and window size

**What to build:** The reader can zoom with the usual keys and Ctrl+wheel, and the zoom level carries over to every new window. A new window opens at the size of the last window that closed, offset from the active window so windows don't stack exactly.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 15 (Edit and Choose editor), 28 (The documents module carries out the whole show), 29 (The page owns every key)

**Status:** ready-for-agent

- [ ] Ctrl+plus, Ctrl+minus and Ctrl+wheel zoom in and out, and Ctrl+0 resets to 100%.
- [ ] The zoom level is saved in the settings file and used by every new window.
- [ ] A new window takes the size of the last window that closed and opens offset down and to the right of the active window, staying on screen.
- [ ] Zoom in, Zoom out and Reset zoom are in the right-click menu.

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- Ticket 10 left WebView2's zoom keys (Ctrl+plus, Ctrl+minus, Ctrl+0) working on purpose: its `AcceleratorKeyPressed` handler (`leave_reload_keys_to_the_page` in `app/src/platform/windows.rs`) turns off only the reload keys. There are two ways to take zoom over:
  - Add the zoom keys to that handler.
  - Turn off all of WebView2's browser keys (`SetAreBrowserAcceleratorKeysEnabled(false)`) and remove the handler. The page must then handle the keys the handler covered, and still swallow Ctrl+F5, Shift+F5 and Ctrl+Shift+R.

  Either way, Ctrl+wheel is a separate setting (`IsZoomControlEnabled`). Check each key by hand in the release build.
- `Command` in `page/src/commands.ts` has `ctrlLetter` (a letter) and `key` (a key without modifiers). Ctrl+plus fits neither. Widen the shape for all commands rather than adding a special case for zoom; ticket 12 needs the same for Alt+Left.
- An OS hook that can't be added shouldn't fail the window (see `leave_reload_keys_to_the_page`).

**2026-11-13, from the architecture review.** The two ways to take zoom over above: ticket 29 takes the second, so WebView2's browser keys are off and the page's command table owns Ctrl+plus, Ctrl+minus and Ctrl+0. Ctrl+wheel is still `IsZoomControlEnabled`. Ticket 28 gathers what a new window starts with (page URL flags, backdrop, key setup) in one place behind a windows port the tests can record; the zoom level, the size and the offset go there, and are tested there. `leave_reload_keys_to_the_page`, named above, is now `leave_reload_and_find_keys_to_the_page`, and ticket 29 removes it.

**2026-11-13, from the window design grilling.** The page will draw the window's title bar (ADR 0005, [Window design](../../window-design/spec.md)), and the webview's zoom would scale that bar and its buttons too. So zoom applies to the document view only (CSS `zoom` on the view), not WebView2's zoom factor, and Ctrl+wheel is handled by the page with `IsZoomControlEnabled` off. The find bar and notices keep their size. If this ticket is built before window design ticket 01, build it this way already, so it isn't redone.

**2026-11-16, from ticket 29.** Built: WebView2's browser keys are off, zoom keys included, and `leave_reload_and_find_keys_to_the_page` is gone. A command in `page/src/commands.ts` takes a list of shortcuts, each a `KeyboardEvent.key` with exactly the modifiers it names, so Ctrl+plus is several entries, for example `{ ctrl: true, key: "+" }`, `{ ctrl: true, shift: true, key: "+" }` (Shift+= on a US layout) and `{ ctrl: true, key: "=" }`; the menu shows the first. Ctrl+wheel: Tauri leaves `zoom_hotkeys_enabled` false, so wry already sets `IsZoomControlEnabled` and `IsPinchZoomEnabled` false on every window; the page's own Ctrl+wheel needs no app change.

**2026-11-16, from ticket 28.** A new window opens through the documents module's `Windows` port: `Documents::open` builds a `NewWindow` (today only `title`) and calls `Windows::open(label, &new_window)`; Tauri's adapter (`impl Windows for AppHandle` in `app/src/window.rs`) hands it to `open_window`, the one place that sets up what every window starts with alike (page URL and flags, frame, navigation guard, backdrop, keys, maximize button, 5 s fallback). Put the size and the offset in `NewWindow`, decided in the documents module, and apply them in `open_window`; the tests then see them in `recorder.done()` as `ToWindow::Opened(NewWindow { .. })`. The offset needs the active window's position, which the port doesn't give yet: add it to `Windows` (and the recorder) rather than reading Tauri from the documents module. The size of the last window that closed sits near `forget_when_destroyed` in `window.rs`, but read it while the window still exists (for example on `CloseRequested`), not on `Destroyed`. A zoom level that goes to the page in its URL belongs to `page_url`'s flags and their contract sample (`app/tests/fixtures/page_url.json`).
