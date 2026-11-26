# 05: Large documents, measured

**What to build:** The **Overview ruler** costs the reader nothing they can feel on any document Downlight shows: the document appears as soon as it does today, and typing in **Find** and scrolling stay as responsive. On a large document the heading marks arrive first, and at 100 MB the ruler still shows where matches cluster.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 03 (Find's marks, and clicking them)

**Status:** ready-for-agent

- [ ] Nothing of the ruler runs in the frame that puts a document in: it builds its list of headings and its block index after **First paint**, in slices as Find builds its index, heading marks first.
- [ ] Measured with the timing switch on the 1 MB test document: scrolling the whole document keeps every page task and frame under 100 ms, typing a one-letter search (tens of thousands of matches) makes the longest task or frame no longer with the ruler than without it, within the spread between runs (the spec's amended target; Find's own count is v1/40's), and the frame after the document goes in is no longer than ticket 02's baseline (v1).
- [ ] First, the open item from ticket 03's Results: typing in Find on 1 MB mixed had a longest frame of 120 to 193 ms with the ruler on against 0 to 126 with it off. Probe it (for example with the Long Animation Frames API) to tell whether the ruler's redraw after a count now lands in Find's frame, and fix it. Ticket 03 also leaves this ticket the per-render cache of each text node's offset in its block, so a count places its matches without walking the document.
- [ ] Measured on the 20 and 100 MB documents (`npm run measure -- large`), noting when the ruler first shows.
- [ ] If a redraw goes over 100 ms on the 1 MB document, the ruler draws on a canvas behind a painter interface (canvas in the app, a recorder in the tests), reading its colors from computed style and the system colors under forced colors; the page tests then read the recorder.
- [ ] The results go in this ticket's comments.
- [ ] `npm run check` passes.
