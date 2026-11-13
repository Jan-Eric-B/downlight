# 07: Keep on top

**What to build:** **Keep on top** keeps one window above all others. A toggle button in the title bar, Ctrl+Alt+T and a right-click menu entry switch it. It belongs to the window and is gone when the window closes.

Spec: [Window design](../spec.md)

**Blocked by:** 03 (The accent color in the page), 05 (Open and Edit buttons, and the separators)

**Status:** ready-for-agent

- [ ] The button, Ctrl+Alt+T and the menu entry run one command, and the button shows in the accent color while it is on.
- [ ] The window stays above other windows while it is on, and only that window.
- [ ] It is not saved: a new window, or the same document opened again after closing, starts without it.
- [ ] Page tests: the command asks the app to switch it, and the button shows the state the app answers with.
- [ ] Checked by hand: above other apps, above another Downlight window, and while maximized.
