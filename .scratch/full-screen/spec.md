# Full screen

Status: ready-for-agent

## Problem Statement

When I read a long document I want nothing but the document on the screen. Today the window always keeps its title bar, and even maximized it leaves the taskbar in view, so part of the screen goes to things I'm not using while I read. Other programs I use (Edge, Explorer, VS Code) go full screen with F11, and in Downlight that key does nothing. At the same time I don't want what games call exclusive full screen: I still want to drag another window over the document, Alt+Tab away and back, and find the window as it was when I'm done.

## Solution

**Full screen**: F11, or "Full screen" in the right-click menu, makes the window fill its whole screen, over the taskbar, without its title bar. It is a borderless window covering its monitor, never an exclusive one, so other windows can still be put over it and the taskbar comes back while another window has the focus. The title bar slides in over the document while the pointer rests at the top edge of the screen, and its maximize button then reads "Leave full screen". F11 again, Esc (once nothing else open takes it), the menu entry or that button leave full screen, and the window returns to how it was, maximized or at its size and place. A Notice says how to leave each time the window goes full screen. Full screen belongs to one window, as Keep on top does: it isn't a setting, a new window never starts in it, and it is gone when the window closes.

## User Stories

1. As a reader, I want to press F11 to show the window full screen, so that the document has the whole screen.
2. As a reader, I want to press F11 again to leave full screen, so that the key that took me in takes me out.
3. As a reader, I want a "Full screen" entry in the right-click menu, so that I can find full screen without knowing its key.
4. As a reader, I want the menu entry to name its key, F11, so that I learn the faster way.
5. As a reader, I want the menu entry checked while the window is full screen, so that the menu shows the window's state, as it does for Full width and Keep on top.
6. As a reader, I want the entry beside Full width and Keep on top in the menu, so that the window's ways of being shown are together.
7. As a reader, I want no full screen button on the title bar, so that the bar keeps its few buttons for what I use often.
8. As a reader, I want full screen to cover the taskbar, so that nothing but the document is on the screen.
9. As a reader, I want full screen to fill the monitor the window is on, so that it stays where I was reading.
10. As a reader, I want to drag another window over a full-screen window, so that I can glance at something else without leaving full screen.
11. As a reader, I want Alt+Tab and the taskbar to work as usual while a window is full screen, so that full screen never traps me.
12. As a reader, I want the taskbar to show again while another window has the focus, so that I can still reach the rest of the system.
13. As a reader, I want a full-screen window that is also kept on top to stay above the others, so that Keep on top means what it says in full screen too.
14. As a reader, I want the title bar hidden while the window is full screen, so that the document starts at the top of the screen.
15. As a reader, I want the title bar to slide in over the document while the pointer rests at the top edge of the screen, so that its buttons and the file name are still within reach of the mouse.
16. As a reader, I want the sliding title bar to lie over the document instead of pushing it down, so that the reading position never moves when I reach for the bar.
17. As a reader, I want the title bar to slide away again once the pointer leaves it, so that it is gone as soon as I'm done with it.
18. As a reader, I want the title bar to stay while the pointer is over it, so that its tooltips and buttons don't vanish under the pointer.
19. As a keyboard reader, I want the title bar to slide in while the focus is in it, so that I never focus a button I can't see.
20. As a reader, I want the title bar to stay while the settings flyout I opened from its gear is open, so that moving from the gear into the flyout doesn't hide the bar the flyout hangs from.
21. As a reader, I want the title bar to slide away once neither the pointer, the focus nor the settings flyout keeps it, so that full screen goes back to showing only the document.
22. As a reader, I want the title bar's maximize button to read "Leave full screen" while the window is full screen, so that the mouse has a way out where I'd look for it.
23. As a reader, I want that button's tooltip to name F11, so that I learn the key.
24. As a reader, I want that button to show the restore glyph, so that it looks like the button that gives the window its size back.
25. As a reader, I want hovering "Leave full screen" not to open Windows' snap layouts, so that the button does only what it says.
26. As a reader, I want minimize and close on the sliding title bar to work as usual, so that I can put a full-screen window away without leaving full screen first.
27. As a reader, I want dragging the title bar's empty strip to do nothing while the window is full screen, so that reaching for the bar never pulls the window off the screen.
28. As a reader, I want double-clicking the strip to do nothing while the window is full screen, so that a stray double-click doesn't resize the window.
29. As a reader, I want a right-click on the strip to open no window menu while the window is full screen, so that the system's Move and Size don't act on a full-screen window.
30. As a reader, I want the file name on the sliding title bar to keep its tooltip with the path and the Counts, so that full screen hides nothing I can otherwise see.
31. As a reader, I want a click on the file name on the sliding bar to reveal the file, as it does outside full screen, so that the bar works the same wherever it shows.
32. As a reader, I want the Full width, Keep on top and settings buttons on the sliding bar to work as usual, so that I can change how I read without leaving full screen.
33. As a reader, I want Esc to leave full screen, so that the key I try first to get out works.
34. As a reader, I want Esc to close Find first while it is open, so that leaving Find doesn't also take me out of full screen.
35. As a reader, I want Esc to close the settings flyout first while it is open, so that Esc closes what is nearest first.
36. As a reader, I want Esc to Stop a document on its way first, so that Esc keeps doing what its Wait note says.
37. As a reader, I want an Esc that cancels an IME's composition to stay with the IME, so that typing in another script never leaves full screen.
38. As a reader, I want Esc with Ctrl, Alt, Shift or the Windows key held not to leave full screen, so that a shortcut with Esc in it does only what it does.
39. As a reader, I want Esc to do nothing while the window isn't full screen and nothing else takes it, so that Esc never changes the window's size.
40. As a reader, I want a Notice saying "Press F11 or Esc to leave full screen" each time the window goes full screen, so that I know the way back, even after pressing F11 by accident.
41. As a reader, I want no such Notice when the window leaves full screen, so that leaving is quiet.
42. As a reader, I want leaving full screen to bring a window that was maximized back maximized, so that full screen is a detour, not a change.
43. As a reader, I want leaving full screen to bring a window that wasn't maximized back to its size and place, so that I find the window where I left it.
44. As a reader, I want Keep on top to stay as it was through full screen, so that one doesn't switch the other.
45. As a reader, I want the reading position to stay when the window goes full screen and when it leaves, so that the change of size doesn't lose my place.
46. As a reader, I want the Find bar at the top of the screen while the title bar is hidden, so that it doesn't float below a bar that isn't there.
47. As a reader, I want the settings flyout at the top of the screen when I open it with Ctrl+, or the menu while the bar is hidden, so that it sits where the gear would be.
48. As a reader, I want the sliding title bar to cover the Find bar and the settings flyout while it shows, so that they don't jump down and back each time the bar slides in.
49. As a reader, I want the Notices and the Counts bubble where they always are, so that full screen moves nothing it doesn't have to.
50. As a reader, I want the Wait note and the file deleted or moved bar at the top of the document in full screen as they are outside it, so that they still say what they say.
51. As a reader, I want the mouse pointer to stay visible in full screen, so that it never disappears while I read with the wheel.
52. As a reader, I want full screen to stay when the window shows another document, by a link, Back, Forward or Open, so that following a link keeps me in full screen.
53. As a reader, I want full screen to stay through a Reload, so that an edit in my editor doesn't take me out of it.
54. As a reader, I want full screen in a window that shows no document too, so that the command works the same in every window.
55. As a reader, I want a new window never to start full screen, so that a file I open from Explorer doesn't take over the screen.
56. As a reader, I want full screen to belong to the one window I switched it in, so that my other windows stay as they are.
57. As a reader, I want full screen gone when its window closes, so that nothing about it is remembered.
58. As a reader, I want full screen kept out of the settings, so that a choice for one moment of reading doesn't become every window's.
59. As a reader, I want a new window opened after a full-screen window closed to get the size the last windowed window had, not the monitor's, so that one full-screen read doesn't make every later window huge.
60. As a reader, I want a new window opened while the active window is full screen to be placed as it is beside a maximized one, so that it opens on the screen as a normal window.
61. As a reader, I want a full screen that couldn't be switched to say why and leave the window as it was, so that every failure is shown.
62. As a reader, I want the menu entry's check mark and the "Leave full screen" button to show what the window is, not what the page expected, so that they never contradict the window.
63. As a screen reader user, I want the menu entry and the "Leave full screen" button named for what they do, so that I hear the state and the way out.
64. As a reader on a touch screen, I want the right-click menu, which a long press opens, to leave full screen, so that I have a way out without a keyboard.
65. As a reader who prefers reduced motion, I want the title bar to appear and disappear without sliding, so that full screen doesn't animate against my system's setting.

## Implementation Decisions

- **Glossary.** The term is **Full screen** (`GLOSSARY.md`, added during the grilling). Avoid zen mode, kiosk, presentation mode and borderless.
- **The window's full screen is the app's state, as Keep on top's is.** A new app command switches full screen on the window that asks and answers whether the window is full screen now, as `switch_keep_on_top` answers whether it is kept on top. It switches rather than sets, so the answer is the window's real state whatever the page thought, and it fails with a Notice. Tauri's `set_fullscreen` (tauri-runtime-wry 2.12) always makes a borderless window covering its current monitor, never an exclusive one, so no choice of kind is needed. The command takes no arguments. Its answer gets a contract sample beside Keep on top's, and the page's command types gain its entry, which only the page's app module calls.
- **Leaving full screen restores the window's earlier state.** Whether the system's restore after full screen brings back a maximized window maximized is a hand check; if it doesn't, the app remembers whether the window was maximized when it went full screen and maximizes it again on leaving. That memory belongs to the window and is never saved.
- **A full-screen window has no place to keep.** The app's lookup of a window's place answers nothing for a full-screen window, as it already does for a minimized or maximized one. So a full-screen window that closes leaves the kept size of the last windowed one as it was, and a new window opened while the active window is full screen is placed by the system on the main screen, as it is beside a maximized one.
- **No snap layouts in full screen.** Today the app answers the pointer over the title bar's maximize button as the system's maximize button, which brings Windows 11's snap layouts flyout, and the page tells the app where that button is and the app tells it how to show it. While the window is full screen the app stops answering that spot as the maximize button, so "Leave full screen" is a plain page button.
- **A page module for full screen, shaped like Keep on top's.** It holds whether the app last answered that this window is full screen, starts afresh as a new window's page (not full screen, and again when the tests' signal aborts), switches by asking the app and keeps only the answer that comes for the current start, and never rejects: a failed switch shows why and leaves the state. It also starts with the page's other per-window state.
- **The command.** A new entry in the page's commands table: label "Full screen", shortcut F11 alone (no Ctrl, Alt or Shift), `pressed` from the full screen module, running the switch and then refreshing the commands so the check mark and the title bar follow the answer. It goes in the right-click menu's group with Full width and Keep on top, after Keep on top. It has no title bar button. It is available in every window, with or without a document.
- **Esc.** A keydown listener for Esc that leaves full screen only when the window is full screen, the event isn't composing, no modifier is held, the event isn't already taken (`defaultPrevented`), Find isn't open, the settings flyout isn't open and the Wait note can't Stop. That is the order: the flyout, Find, Stop, then full screen, and the coming image lightbox goes before full screen too. Existing Esc listeners already defer to each other this way.
- **The Notice.** On every answer that the window is now full screen, the page shows the Notice "Press F11 or Esc to leave full screen". None on leaving.
- **The title bar in full screen.** While full screen, the room the page keeps for the bar at the top is zero: the view, the Find bar and the settings flyout start at the top of the screen, and the bar itself keeps its own height and lies over the document (today one variable is both the bar's height and the room kept for it, and they separate). The bar is hidden, and shown while any of these holds: the pointer rests in a thin zone at the top edge of the screen or is over the bar, the focus is within the bar, or the settings flyout is open. It hides once none holds. Its tooltips are the system's and show only while the pointer is over the bar, so the pointer rule covers them. It slides in and out, and appears and disappears at once under reduced motion. Its elements are the same in full screen: the logo, the file name with its tooltip and reveal, the zoom level, the command buttons and the window's buttons.
- **The maximize button in full screen** reads "Leave full screen" with the tooltip "Leave full screen (F11)", shows the restore glyph, and runs the full screen command. Outside full screen it is maximize or restore as today.
- **The strip drags nothing in full screen.** While full screen the bar is no drag region, so a drag, a double-click and the system's window menu on a right-click there do nothing.
- **No change to the page elsewhere.** The Notices, the Counts bubble, the Overview ruler, the Wait note and the file deleted or moved bar stay where they are, and the pointer is never hidden. The window's own title (Alt+Tab, the taskbar) doesn't change.
- **The reading position across the change of size.** Going full screen and leaving it resize the window and change the room for the bar; the view keeps the reading position across both, as it does for any change of layout that keeps it.

## Testing Decisions

- A good test drives the page as the reader does (a key, a menu choice, a click, the pointer, the focus) and checks what the reader can observe: what the page asks the app, the menu entry's check mark, the button's name and tooltip, whether the bar is shown, the Notice. It never checks the module's internals. happy-dom has no layout, so for the bar's position the tests check the state the page sets (the bar shown or hidden, the room kept for it), not pixels. Test names state the behavior, as `the button shows the state the app answers with, not the one the page expected` does.
- **The page (Vitest with happy-dom, the tests' fake app `fakeTheApp`).** The seam for nearly everything: F11 and the menu entry ask the app to switch full screen; the entry's check mark and the bar show what the app answered, not what the page expected; a failed switch says why and changes nothing; a switch answered after the page started again changes nothing; a new window's page starts out of full screen; full screen isn't a setting (a settings change leaves it); the Notice on entering and none on leaving; Esc leaves full screen, and doesn't while Find, the settings flyout or a stoppable Wait note is open, while composing, with a modifier, or out of full screen; the bar hides in full screen and shows while the pointer is in the top zone or over it, while the focus is in it, or while the flyout is open, and hides once none of these holds; the maximize button becomes "Leave full screen (F11)" and runs the command; the strip stops being a drag region; the room kept for the bar is zero in full screen; full screen works in the empty window; AltGr combinations and F11 with a modifier switch nothing. Prior art: `keep-on-top.test.ts` (a per-window state the app answers), `title-bar.test.ts` (the window buttons, maximize turning into restore, tooltips naming shortcuts), `find.test.ts` and `wait-note.test.ts` (Esc's order).
- **The contract samples.** The app's tests write a sample of the new command's answer, and the page's contract tests drive it through the page, as for Keep on top. After adding the shape, `npm run fixtures` writes it.
- **The app (`cargo test`).** No new unit test: switching full screen and the window's place talk to the real Tauri window. What the app does when a closing window has no place is already covered by the documents tests.
- **Hand checks in the release build.** The window covers the taskbar; another window can be dragged over it and the taskbar shows while it has the focus; Alt+Tab works; full screen fills the monitor the window is on, on a second monitor too; leaving brings a maximized window back maximized and a windowed one back to its size and place; hovering "Leave full screen" opens no snap layouts flyout; a drag, a double-click and a right-click on the strip do nothing in full screen; the bar slides in at the top edge and away again, and appears at once with reduced motion; the reading position stays when entering and leaving; closing a full-screen window and opening a new one gives the earlier windowed size; Keep on top and full screen together keep the window above others.
- Nothing full screen adds grows with a document's size, so it needs no measurement with the timing switch; keeping the reading position across the resize is existing code.

## Out of Scope

- Zen mode: hiding the title bar, the margin, the line numbers or the ruler while windowed. A separate idea, ticketed on its own if wanted.
- Exclusive full screen, which takes the display; Tauri doesn't make it and readers want other windows over theirs.
- A title bar button for full screen.
- Any shortcut but F11, such as Alt+Enter.
- Remembering full screen in the settings, or a new window starting full screen.
- Hiding the mouse pointer while it rests.
- A touch gesture to show the bar or leave full screen.
- Showing the Notice only the first time, which would need saved state.
- The Marp slideshow (`.scratch/slideshow/`), which will show its slides in full screen and decides its own keys, Reload and Find.
- Other systems: they keep the plain fallbacks until their ports (the title bar there is the native one).

## Further Notes

- From the v1 spec's Out of Scope (v1.x), ticket `issues/01-full-screen.md`, grilled with the repo owner on 2026-11-20.
- Esc's growing order (the settings flyout, Find, Stop, full screen, and later the image lightbox) is spread over the listeners of each feature, each deferring to the ones before it. If the lightbox makes that hard to follow, one place that owns Esc's order may be worth it then, not now.
- The slideshow (`.scratch/slideshow/issues/01-marp-slideshow.md`) will want to switch the window full screen; this spec's command and page module are what it reuses.
