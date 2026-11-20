# 01: Copy as Markdown, and as rich text

**What to build:** The reader can copy Markdown source from Downlight, not only the rendered text, to paste it into another Markdown file, a chat or an issue; and copy the rendered document with its formatting, to paste into Word or Outlook.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, v1.x.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Copy what: the selection's source (the file's lines of the blocks it touches, from their line numbers, or exactly the selected part), the whole file, or a section under a heading?
  - How it starts: a right-click entry, Ctrl+Shift+C, a title bar button? What plain Ctrl+C copies stays as it is.
  - A selection that starts or ends mid-block: whole lines, or trimmed? A sorted table (**Sort**): the file's order or the shown one? **Ticks**: as in the file, never as ticked?
  - Large selections: the source comes from the app, which has the file; the page has only the rendered document.
  - **Copy as rich text** (the owner wants it): the rendered document, or the selection, with its formatting, to paste into Word or Outlook. First check what Ctrl+C already puts on the clipboard (the webview may copy HTML with it); then decide whether a "Copy document" command is needed, and what the copy leaves out (copy buttons, line numbers, Downlight's own colors, **Local images**, which another program can't load through Downlight's image scheme).

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (v1.x). mdhero copies as rich text or Markdown (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
