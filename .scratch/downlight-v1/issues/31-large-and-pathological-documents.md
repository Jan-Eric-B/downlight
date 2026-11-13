# 31: Large and pathological documents fail gracefully

**What to build:** A document or image too large to show, or a code block too costly to highlight, gives the reader a clear message instead of a blank window, a frozen render or a crash that closes every window.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Measured first with the timing switch, results recorded here: read, render, IPC and insert for documents of 1, 5, 20 and 100 MB (`npm run documents` can be extended), and the memory used.
- [ ] A document over a ceiling chosen from those numbers is not rendered; the view says how large it is and why it isn't shown. The read stops at the ceiling rather than loading the whole file (`app/src/render.rs` reads it whole today).
- [ ] The image scheme serves no file over a ceiling of its own (`app/src/image.rs` reads every image whole). A failed allocation aborts the process, which closes every window (ADR 0001); a downloaded archive can carry a multi-GB zero-filled `x.png` that compresses to almost nothing.
- [ ] Highlighting has a budget: a code block over a size (and if needed a time) limit shows as plain code, as GitHub does for large files. Measure the slowest grammars on a large block first. Today a huge block, or input a grammar backtracks on, holds the render thread for as long as fancy-regex takes, and first paint waits for it.
- [ ] Tested first in the core (the highlight budget) and the app (a document and an image over the ceiling).

## Comments

**2026-11-13, from the architecture review.** The spec's performance targets stop at 1 MB; nothing says what happens beyond. A file opened from the command line is rendered whatever it is (`downlight some.bin`), so the ceiling matters for more than Markdown; ticket 34 asks what the command line does with files that aren't Markdown.
