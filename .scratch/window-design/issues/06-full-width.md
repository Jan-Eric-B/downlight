# 06: Full width

**What to build:** **Full width** lets the document fill the window instead of GitHub's reading width. A toggle button in the title bar, Ctrl+Alt+W and a right-click menu entry switch it, and the choice is kept in the settings for every window.

Spec: [Window design](../spec.md)

**Blocked by:** 03 (The accent color in the page), 05 (Open and Edit buttons, and the separators)

**Status:** ready-for-agent

- [ ] The button, Ctrl+Alt+W and the menu entry run one command, and the button shows in the accent color while Full width is on.
- [ ] Switching it keeps the reading position.
- [ ] The choice is saved in the settings file and applies to every open window and every new one.
- [ ] Page tests: the command switches the view's width and the button's state; a new window starts with the saved choice.
- [ ] App tests: the settings keep Full width across a restart and default to off.
