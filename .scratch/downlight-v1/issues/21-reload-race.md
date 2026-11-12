# 21: A save never reloads a document the window has just left

**What to build:** A change to a document's file reloads its window only while the window still shows that document. Today the check and the reload are two separate steps, so a document given to the window in between is reloaded as if it were the old one.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 10 (Reload)

**Status:** ready-for-agent

- [ ] A change reported for a document the window has left starts no render and tells the page nothing, even when the window is given another document between the check and the reload.
- [ ] F5, Ctrl+R and the right-click menu's Reload still reload whatever the window shows.
- [ ] Tested first, in `app/src/window/tests.rs`: a reload for a key the window has left starts no render, and leaves the new document's render ahead as it was.

## Comments

**2026-11-12, from ticket 14's architecture review.** Found while reading the app, not seen happen. `on_document_change` in `app/src/reload.rs` asks `OpenWindows::shows(window, key)`, and then `reload` calls `OpenWindows::reload_ahead(window)`, which takes no key. Each call takes the table's lock on its own. If Ctrl+O, a drop or a link gives the window another document between the two calls, `reload_ahead` replaces the new document's render ahead and the page is told `reload`. The page's `reload` then overtakes its `showDocument` (`page/src/view.ts`), so the new document shows at the old document's reading position, and the heading a link named is ignored. The window for this is short (two lock acquisitions on the watch's thread), but it is real.

Fix: a keyed `reload_ahead(window, key)` that checks and starts the render under one lock, used by `on_document_change`; the hand reload (`reload_document`) keeps reloading whatever is shown. Ticket 22 would fold this into a deeper module, but this fix doesn't need to wait for it.
