# 10: Status bar

**What to build:** A small bar or bubble at the bottom of the window showing counts about the document: lines, words, and the selected text.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** needs-triage

- [ ] Grill it before planning. Open questions: a bar or a floating bubble; always shown or switched; what it counts (the file's lines or the rendered text, words, characters, the selection's words and characters); whether it shows anything else (the reading position's line number, the encoding); and how it stays fast on the 1 MB test document.

## Comments

**2026-11-13, from the grilling.** For a viewer, the counts should probably be about the rendered text the reader sees, not the Markdown source; the line count is the exception, since line numbers are the file's. Counting the whole document grows with its size, so it is measured with the timing switch when it is added.
