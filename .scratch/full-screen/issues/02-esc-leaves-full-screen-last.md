# 02: Esc leaves full screen, last

**What to build:** Esc leaves **Full screen**, but only once nothing else open takes it: the settings flyout closes first, then Find, then a document on its way is stopped (**Stop**), and only then does Esc leave full screen. A reader with Find open presses Esc once to close Find and again to leave full screen.

Spec: [Full screen](../spec.md)

**Blocked by:** 01 (F11 and the menu switch full screen)

**Status:** ready-for-agent

- [ ] Esc leaves full screen only while the window is full screen, the key isn't composing, no modifier is held, and the event isn't already taken, Find isn't open, the settings flyout isn't open and the **Wait note** can't Stop. It defers to the others as the existing Esc listeners defer to each other.
- [ ] A comment by the listener names the order (the settings flyout, Find, Stop, full screen) and that the coming image lightbox goes before full screen too.
- [ ] Page tests (written first): Esc leaves full screen; with Find open it closes only Find, and a second Esc leaves; with the settings flyout open it closes only the flyout; with a stoppable Wait note it only stops; an Esc that cancels an IME's composition, Esc with Ctrl, Alt, Shift or the Windows key, and Esc out of full screen ask the app nothing.
- [ ] Hand check in the release build: Esc after F11 leaves full screen, and with Find open takes two presses.
- [ ] `npm run check` passes.
