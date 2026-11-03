# 13: Ticks

**What to build:** A reader can tick task list items while working through a checklist. **Ticks** survive a reload as long as the number of task items stays the same, come back with Back and Forward, and are never written to the file.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 12 (Back and Forward)

**Status:** ready-for-agent

- [ ] Task items can be ticked and unticked with the mouse and with the keyboard.
- [ ] A reload with the same number of task items keeps the ticks; a reload with a different number clears them.
- [ ] Back and Forward restore the ticks of that history entry.
- [ ] Ticks are never written to the file and are gone when the window closes.
- [ ] Page tests cover each rule above.
