# 05: Large documents, measured

**What to build:** The **Overview ruler** costs the reader nothing they can feel on any document Downlight shows: the document appears as soon as it does today, and typing in **Find** and scrolling stay as responsive. On a large document the heading marks arrive first, and at 100 MB the ruler still shows where matches cluster.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 03 (Find's marks, and clicking them)

**Status:** ready-for-agent

- [ ] Nothing of the ruler runs in the frame that puts a document in: it builds its list of headings and its block index after **First paint**, in slices as Find builds its index, heading marks first.
- [ ] Measured with the timing switch on the 1 MB test document: typing a one-letter search (tens of thousands of matches) and scrolling the whole document keep every page task and frame under 100 ms, and the frame after the document goes in is no longer than ticket 02's baseline (v1).
- [ ] Measured on the 20 and 100 MB documents (`npm run measure -- large`), noting when the ruler first shows.
- [ ] If a redraw goes over 100 ms on the 1 MB document, the ruler draws on a canvas behind a painter interface (canvas in the app, a recorder in the tests), reading its colors from computed style and the system colors under forced colors; the page tests then read the recorder.
- [ ] The results go in this ticket's comments.
- [ ] `npm run check` passes.
