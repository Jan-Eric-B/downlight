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
