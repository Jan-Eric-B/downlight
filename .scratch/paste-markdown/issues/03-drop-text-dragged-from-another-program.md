# 03: Drop text dragged from another program

**What to build:** The reader drags text from another program (a browser, an editor, a chat) onto a Downlight window, and the window shows it rendered, as **Paste** shows the clipboard's text.

Spec: none yet. Grill it first (`/grill-with-docs`). Split from [Paste Markdown](../spec.md) on 2026-11-20, which leaves it out of scope.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Feasibility first: today file drops come through Tauri's native drag and drop, which gives only file paths, and Tauri turns HTML5 drag and drop in the webview off on Windows while that is on (to confirm against Tauri's current docs). Taking dragged text likely means turning native drop off and rebuilding file drops in the page, where a dropped file comes without its path (WebView2 has a way to hand the host the file's path; check what Tauri exposes). Is that rework worth it, or is there a way to keep native file drops and take text too?
  - Does a text drop reuse Paste's path (the paste module writes the file, ADR 0006), so that it differs from Paste only in where the text comes from?
  - Does a drop of text over a shown document replace it, as a dropped file does?
  - A drag carrying both files and text (some programs offer both): which wins?

## Comments

**2026-11-20.** Split off while grilling Paste Markdown: the owner wants it, but not at the cost of holding Paste back.
