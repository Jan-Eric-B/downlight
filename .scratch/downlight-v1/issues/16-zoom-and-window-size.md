# 16: Zoom and window size

**What to build:** The reader can zoom with the usual keys and Ctrl+wheel, and the zoom level carries over to every new window. A new window opens at the size of the last window that closed, offset from the active window so windows don't stack exactly.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 15 (Edit and Choose editor)

**Status:** ready-for-agent

- [ ] Ctrl+plus, Ctrl+minus and Ctrl+wheel zoom in and out, and Ctrl+0 resets to 100%.
- [ ] The zoom level is saved in the settings file and used by every new window.
- [ ] A new window takes the size of the last window that closed and opens offset down and to the right of the active window, staying on screen.
- [ ] Zoom in, Zoom out and Reset zoom are in the right-click menu.
