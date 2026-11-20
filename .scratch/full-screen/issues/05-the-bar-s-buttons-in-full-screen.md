# 05: The bar's buttons in full screen

**What to build:** On the **Title bar** of a **Full screen** window, the maximize button reads "Leave full screen" and leaves full screen, without bringing up Windows' snap layouts, and the bar's empty strip does nothing: no drag, no double-click, no window menu. Minimize and close work as usual.

Spec: [Full screen](../spec.md)

**Blocked by:** 01 (F11 and the menu switch full screen) for the page tests; 04 (The title bar slides in at the top edge) for the hand check, which needs a bar the reader can see

**Status:** ready-for-agent

- [ ] In full screen the maximize button is named "Leave full screen", its tooltip is "Leave full screen (F11)", it shows the restore glyph and it runs the full screen command. Out of full screen it is maximize or restore as today.
- [ ] While the window is full screen, the app stops answering the pointer over that button as the system's maximize button, so no snap layouts flyout comes; out of full screen it answers it as today.
- [ ] While the window is full screen, the bar is no drag region: a drag, a double-click and a right-click on its strip do nothing. Out of full screen they are as today.
- [ ] Page tests (written first): in full screen the button's name, tooltip and glyph are Leave full screen's, and a click asks the app to switch full screen; leaving brings maximize or restore back; the strip is no drag region in full screen and is one again after.
- [ ] Hand check in the release build: hovering "Leave full screen" opens no snap layouts flyout and a click leaves full screen; a drag, a double-click and a right-click on the strip do nothing in full screen; minimize and close work; out of full screen, snap layouts, dragging and double-clicking work as before.
- [ ] `npm run check` passes.
