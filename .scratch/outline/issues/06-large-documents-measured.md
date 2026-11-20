# 06: Large documents, measured

**What to build:** The **Outline** and the heading jump keys cost the reader nothing they can feel on any document Downlight shows: the document appears as soon as it does today, and the Outline opens and filters at once, even with tens of thousands of headings. While a large document's headings are still being listed, the Outline says so rather than looking complete.

Spec: [Outline](../spec.md)

**Blocked by:** 01 (Alt+Down and Alt+Up step through the headings), 03 (Typing filters the Outline), 04 (The Outline across reloads, other documents and the empty window)

**Status:** ready-for-agent

- [ ] Nothing of the Outline or the jump keys runs in the frame that puts a document in; they read the shared list of headings, which is built after **First paint**, in slices.
- [ ] Opened before the list is complete, the flyout shows the headings listed so far and a last row "Listing headings...", as a live status, and grows in place, keeping the selection and the filter; the row goes once the list is complete. The jump keys use what is listed so far.
- [ ] Measured with the timing switch on the 1 MB test document: the frame after the document goes in is no longer than ticket 02's baseline (v1); opening the Outline, typing a letter in its filter and holding Alt+Down keep every page task and frame under 100 ms.
- [ ] The same on the 20 and 100 MB documents (`npm run measure -- large`), noting when the list of headings is complete; anything over 100 ms is fixed or recorded with its cause.
- [ ] The results go in this ticket's Results section.
- [ ] Page tests (written first): opened while the list is still growing, the Outline shows "Listing headings..." and the headings so far, grows as the list does keeping the selection and the filter, and drops the row once the list is complete.
- [ ] `npm run check` passes.
