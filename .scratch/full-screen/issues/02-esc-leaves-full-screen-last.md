# 02: Esc leaves full screen, last

**What to build:** Esc leaves **Full screen**, but only once nothing else open takes it: the settings flyout closes first, then Find, then a document on its way is stopped (**Stop**), and only then does Esc leave full screen. A reader with Find open presses Esc once to close Find and again to leave full screen.

Spec: [Full screen](../spec.md)

**Blocked by:** 01 (F11 and the menu switch full screen)

**Status:** built, hand checks left

- [x] Esc leaves full screen only while the window is full screen, the key isn't composing, no modifier is held, and the event isn't already taken, Find isn't open, the settings flyout isn't open and the **Wait note** can't Stop. It defers to the others as the existing Esc listeners defer to each other.
- [x] A comment by the listener names the order (the settings flyout, Find, Stop, full screen) and that the coming image lightbox goes before full screen too.
- [x] Page tests (written first): Esc leaves full screen; with Find open it closes only Find, and a second Esc leaves; with the settings flyout open it closes only the flyout; with a stoppable Wait note it only stops; an Esc that cancels an IME's composition, Esc with Ctrl, Alt, Shift or the Windows key, and Esc out of full screen ask the app nothing.
- [ ] Hand check in the release build: Esc after F11 leaves full screen, and with Find open takes two presses.
- [x] `npm run check` passes.

## Comments

**2026-11-20, built; the hand check is left.** What was built, and what it decided:

- **The listener lives in `startCommands` (`page/src/commands.ts`)**, beside the shortcuts' keydown listener, rather than in `full-screen.ts`: it runs the `fullScreen` command (`commands.fullScreen.run()`), so leaving takes the same path as F11 and the menu entry, `switchFullScreen().then(refreshCommands)`, and what shows full screen's state (ticket 05's "Leave full screen" button) refreshes with the app's answer. In `full-screen.ts` it would have needed `refreshCommands`, which is the commands table's, and an import cycle. Esc isn't the command's shortcut in the table: a shortcut takes its key from everything else.
- **It asks each earlier listener rather than relying on `defaultPrevented` alone**, since it runs before Find's and the view's Stop listener (it starts earlier in `main.ts`), as the view's Stop already asks `isFindOpen()`. It leaves full screen only when the key is Esc, not composing, with no Ctrl, Alt, Shift or Windows key, not `defaultPrevented`, the window is full screen, and `isSettingsFlyoutOpen()`, `isFindOpen()` and `canStop()` are all false. Two small exports for it: `isSettingsFlyoutOpen()` in `settings-flyout.ts` (the flyout's own listener also stops the key's propagation, but the check doesn't depend on where the focus is) and `canStop()` in `view.ts` (the started view's `waitNote.canStop()`, false before the view starts). The comment by the listener names the order: the settings flyout, Find, Stop, then full screen, and the coming image lightbox before full screen too.
- **A held Esc leaves once.** A repeat is taken from the webview but asks the app nothing: the app switches rather than sets, so a repeat that came before its answer, while the page still thinks the window is full screen, would switch it back in. Two separate quick presses before the answer still ask twice, as two quick F11s do.
- **Tests** (written first, failing): `page/src/full-screen.test.ts`: `Esc leaves full screen`; `Esc with Find open closes only Find, and a second Esc leaves full screen`; `Esc with the settings flyout open closes only the flyout`; `an Esc that cancels an IME's composition stays with the IME and asks the app nothing`; `Esc with Ctrl / Alt / Shift / the Windows key asks the app nothing`; `Esc out of full screen asks the app nothing`; `a held Esc leaves full screen once: its repeats ask the app nothing`. `page/src/wait-note.test.ts`: `Esc in full screen with the document on its way only stops it, and a second Esc leaves full screen`. Changed: `full-screen.test.ts`'s `press` now presses where the page has the focus, as `wait-note.test.ts`'s does, so Esc reaches the flyout's and Find's own listeners; `wait-note.test.ts`'s fake app answers `switch_full_screen`.
- **Not done:** the hand check in the release build.
