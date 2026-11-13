# 12: Back and Forward

**What to build:** Each window remembers the documents and headings visited in it, and Back and Forward move through them, each step returning to where the reader was. The history belongs to its window and disappears when the window closes.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 09 (Opening from inside a window), 10 (Reload), 11 (Links), 22 (One module owns a window's shown document), 23 (The app decides whether a page shows or reloads its document), 24 (The documents module sends a link's scroll where it decides it), 25 (One pipeline places every document in the page's view), 26 (One message makes the page ask, and every request names its showing), 28 (The documents module carries out the whole show), 29 (The page owns every key)

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

**2026-11-13, from the architecture review.** What to settle before building, and where the groundwork now sits:

- Tickets 25, 26, 28 and 29 come first. 25 gives the page's view one pipeline, so Back and Forward are one more way to land (at a history entry's line), and its snapshot before the swap is where an entry's line is saved. 26 makes every page request name its showing. 28 lets `Documents::give` retitle the window itself, which Back needs. 29 makes the page the only owner of keys, so Alt+Left and Alt+Right are entries in its command table and not also in a WebView2 handler.
- History needs a window record that outlives its shown document. Today the table's value is `Option<ShownDocument>` (`app/src/documents.rs`), and every `give` replaces it, so nothing per window survives a document change. Let the table hold a window record: its shown document and the documents and headings visited in it. The app has to hold the paths, since commands never take a path from the page.
- Key each history entry by its `Showing`. Every `give` takes a new one today, so the page has nothing that comes back to key its saved line (and ticket 13's ticks) to. A step back or forward gives the window an earlier entry with that entry's own `Showing`. The page then finds its saved state for it, and `handover` answers with the third `Place` ticket 23 left room for (return to a saved line).
- That changes one rule from ticket 23, tested by `window_given_back_a_document_it_showed_before_shows_it_anew`: for Ctrl+O and links it stays right, for Back it is wrong. Decide this first.
- Scrolling to a heading has three ways in today: the link's own result (`page/src/links.ts`), a show at a heading (`page/src/view.ts`) and the `scroll-to-heading` message. A heading link is a history entry too, so route the three through one scroll that records it.
