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

**2026-11-12, from ticket 14's architecture review.** What changed since the comment above, and one thing to do first:

- The reading position now has its own module, `page/src/reading-position.ts`, which replaces the first bullet above. `readingPosition(view)` gives the line number of the block at the top of the view, and `returnToReadingPosition(view, line, stillWanted)` puts the view back there, with the second scroll after a frame that ADR 0003 needs. Reload uses both, so going back uses them too. Don't write a second anchoring.
- Before adding Back and Forward, give `startDocumentView` one request pipeline. Today `showDocument` and `reload` each bump `latestRequest`, check after every `await` whether a newer request overtook them, and must remember to call `announce` on every way out (ticket 14 added it to three exits by hand). Back and Forward would be two more requests written the same way. Make one pipeline that every request goes through (show, reload, back, forward). It owns "the latest request wins", the swap of the view's content, and a single `announce` that passes the listeners a reason (shown, reload, back, forward). Each kind of request then says only how it gets its render and where it lands: at the top or a heading, at the reading position, or at a history entry's line. The swap is also where ticket 13 reads the ticks before the old items go and applies them after.
- `Command` in `page/src/commands.ts` now has a `shift` flag too, for Shift+F3 (ticket 14). Widening it to a key with its modifiers, as the bullet above says, replaces `shift` as well.
- The WebView2 key handler is now `leave_reload_and_find_keys_to_the_page` (ticket 14). When it takes the navigation keys, name it for what it is, the keys the page handles itself, rather than adding "and navigation" to the name, so ticket 16's zoom keys don't rename it again.
