# 12: Back and Forward

**What to build:** Each window remembers the documents and headings visited in it, and Back and Forward move through them, each step returning to where the reader was. The history belongs to its window and disappears when the window closes.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 09 (Opening from inside a window), 10 (Reload), 11 (Links)

**Status:** ready-for-agent

- [ ] Following a link to a document or heading, Ctrl+O and dropping a file each add a history entry in the window.
- [ ] Alt+Left and Alt+Right and the mouse's side buttons go back and forward.
- [ ] Going back returns to the reading position the document had when it was left.
- [ ] History belongs to its window, isn't saved, and is gone when the window closes.
- [ ] Back and Forward are in the right-click menu and are unavailable when there is nowhere to go.
- [ ] Page tests cover adding entries, going back and forward, and the restored reading position (by line number).

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- The reading position is already a line number. `readingLine(view)` in `page/src/view.ts` gives the line of the block at the top of the view. `blockAt(view, line)`, followed by two `scrollIntoView` calls a frame apart, puts that block back at the top; the second call is needed because `content-visibility` only estimates heights (ADR 0003). Both functions are module-private. Export them, or move them to a `reading-position.ts`, rather than writing a second anchoring. Reload's tests in `page/src/view.test.ts` show how to fake layout (`layOut`, `spyOnScrolls`).
- `whenDocumentShown` runs after every shown document and every reload. If history needs to know why a document was shown, pass the listeners a reason (shown, reload, back, forward) rather than adding a flag beside the hook.
- Alt+Left, Alt+Right, the BrowserBack and BrowserForward keys and the mouse's side buttons trigger WebView2's own navigation, the way F5 and Ctrl+R triggered its own reload. Ticket 10 found that cancelling a key in the page doesn't always stop WebView2 (Ctrl+Shift+R still reloaded). Its fix is the `AcceleratorKeyPressed` handler `leave_reload_keys_to_the_page` in `app/src/platform/windows.rs`. Extend that one handler to the navigation keys, and rename it for what it covers, rather than adding a second handler. Check each key and both side buttons by hand in the release build.
- `Command` in `page/src/commands.ts` has `ctrlLetter`, and `key`, which means a key pressed without modifiers. Alt+Left fits neither. Widen the shape for all commands, for example to a key with its modifiers, rather than adding a special case for Alt.
- In `startDocumentView`, the page's `latestRequest` guard makes the latest request win when a document is shown during a reload, or the other way round. Back and Forward must go through the same guard.
