# 01: F11 and the menu switch full screen

**What to build:** F11, or "Full screen" in the right-click menu, shows the window **Full screen**: a borderless window covering its monitor and the taskbar, with other windows still able to go over it. F11 or the menu entry again leaves it, and the window returns to how it was, maximized or at its size and place. A Notice says how to leave each time the window goes full screen. Full screen belongs to one window, as **Keep on top** does. The title bar still shows as today; ticket 03 hides it.

Spec: [Full screen](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A new app command switches full screen on the window that asks and answers whether the window is full screen now, as Keep on top's does; a failure becomes a Notice. Its answer gets a contract sample beside Keep on top's, and the page's command types gain its entry.
- [ ] Leaving full screen brings a window that was maximized back maximized. If the system's restore doesn't, the app remembers, for that window only, that it was maximized and maximizes it again on leaving.
- [ ] The app's lookup of a window's place answers nothing for a full-screen window, as for a minimized or maximized one: a full-screen window that closes keeps no size, and a new window opened beside it is placed as beside a maximized one.
- [ ] A page module for full screen shaped like Keep on top's: the state is what the app last answered, a new window's page starts out of full screen (and again when the tests' signal aborts), an answer for an earlier start changes nothing, and a failed switch says why and leaves the state.
- [ ] A command "Full screen" with the shortcut F11 alone, checked while full screen, in the right-click menu after Keep on top; no title bar button. It is available with and without a document.
- [ ] Each answer that the window is now full screen shows the Notice "Press F11 or Esc to leave full screen"; leaving shows none.
- [ ] Page tests (written first): F11 and the menu entry ask the app to switch; the check mark shows what the app answered, not what the page expected; a failed switch says why and changes nothing; an answer after the page started again changes nothing; a new window starts out of full screen; a settings change leaves full screen as it is; the Notice on entering and none on leaving; F11 with a modifier, and AltGr combinations, switch nothing; it works in the empty window.
- [ ] `npm run fixtures` writes the new contract sample; review its diff.
- [ ] Hand check in the release build: F11 covers the taskbar on the monitor the window is on, a second monitor too; another window can be dragged over it and the taskbar shows while that one has the focus; Alt+Tab works; leaving brings a maximized window back maximized and a windowed one back to its size and place; Keep on top stays as it was, and with it on the full-screen window stays above others; closing a full-screen window and opening a new one gives the earlier windowed size.
- [ ] `npm run check` passes.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (v1.x), so the idea has a ticket. Not yet grilled.

**2026-11-20, from the repo owner.** Grilled and specced: `../spec.md` (ready-for-agent). Full screen only, no zen mode; borderless, so other windows still go over it; F11, a menu entry (no button), and Esc last after the settings flyout, Find and Stop; one window's state, never saved; the title bar hidden and sliding in at the top edge, its maximize button reading "Leave full screen". The stub became this ticket and tickets 02 to 05.
