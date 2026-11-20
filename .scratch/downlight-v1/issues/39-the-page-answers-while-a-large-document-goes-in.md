# 39: The page answers while a large document goes in

**What to build:** While the page takes in a large document, on a show or a reload, it keeps answering: the title bar's buttons and drag region, scrolling, keys and the wait note's Stop work, and no task or frame holds the page for long, whatever the document's size.

Spec: [Downlight v1](../spec.md)

**Blocked by:** [38](38-a-large-document-s-load-can-be-left.md). After v1.

**Status:** needs-triage

- [ ] First, a performance trace of the frame after the insert of `mixed-100mb.md` (`npm run documents -- large`): 9.6 s in ticket 31's Results, though ADR 0003's `content-visibility: auto` skips layout of blocks away from the view. What it spends that time on (style, layout, garbage collection, the copy buttons and ticks `prepare` adds) decides the approach.
- [ ] Grill it after the trace. Open questions below.
- [ ] Measured with the timing switch on `mixed-20mb.md` and `mixed-100mb.md`, against ticket 31's Results.

## Comments

**2026-11-19, split from ticket 38 when it was grilled.** Ticket 38 lets the reader leave a large document while it is on its way and the page is free. The page itself still freezes while it takes the document in, about 15 s at 100 MB mixed and 3 s at 20 MB (ticket 31's Results: IPC 2.5 s, partly on the page's main thread; insert 5.5 s, which is `template.innerHTML` in `prepare` and `replaceChildren` in `page/src/view.ts`; the frame after it 9.6 s). While it does, the title bar (ADR 0005), keys and ticket 38's Stop are dead, and the wait note says "Laying out" instead.

**Open questions:**

1. **Insert in chunks over several frames.** ADR 0003's rejected option: keeps each frame short, but shows the document in stages, and Find, ticks, reload and the reading position wait for the last chunk, so it supersedes part of ADR 0003 and needs an ADR of its own. The core could hand over its top-level blocks apart, so the page needn't split HTML. A half-step: chunks only above a size (say 5 MB).
2. **A reload of a large document.** It shows no wait note and keeps the document in view until the new render replaces it, so each save of a 20 MB document freezes the window for about 3 s. Whether the new render can be built in chunks out of view and swapped in at once, and what the swap's frame then costs, depends on the trace.
3. **Stop while laying out.** Once the page answers during the insert, ticket 38's Stop could stay on the note through "Laying out" and drop the chunks still to come.
