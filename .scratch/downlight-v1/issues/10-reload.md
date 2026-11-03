# 10: Reload

**What to build:** When a document's file is saved, Downlight **reloads** it at the same reading position, also for editors that save by writing a new file and renaming it over the old one. F5 and Ctrl+R reload by hand. A deleted file leaves the last render visible with a bar saying so, and the document reloads when the file comes back.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 04 (Documents look like GitHub on Mica), 09 (Opening from inside a window)

**Status:** ready-for-agent

- [ ] The core's watching takes a document's path and reports Changed, Gone and Back. It watches the file's folder and reports a change 150 ms after the last event.
- [ ] Saving in place and saving by renaming a new file over the old one both reload the document.
- [ ] After a reload, the block that was at the top of the view (found by its line number) is at the top again.
- [ ] F5 and Ctrl+R reload by hand.
- [ ] When the file is deleted or moved, a bar says "File deleted or moved" and the last render stays visible.
- [ ] When the file comes back, the document reloads and the bar goes away.
- [ ] Reload is in the right-click menu.
- [ ] Core tests with real temporary files cover a rewrite in place, a rename over the file, a delete and a recreate. A page test covers which block a reload anchors to.
