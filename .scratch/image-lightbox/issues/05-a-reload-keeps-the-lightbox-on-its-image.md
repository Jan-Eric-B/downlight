# 05: A Reload keeps the lightbox on its image

**What to build:** When the document's file changes while the **Lightbox** is open, the **Reload** keeps it open on the same image if the reloaded document still has it, with its zoom, and "3 of 12" counts the new document's images. When the image is gone, the lightbox closes.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 03 (Step through the document's images)

**Status:** ready-for-agent

- [ ] After a Reload, the lightbox looks in the new render for the image with the same source at the same occurrence of that source (the second `shot.png` stays the second `shot.png`), and stays on it with the list rebuilt and its zoom and pan kept.
- [ ] When no such image is there, it closes, and the focus goes to the view.
- [ ] When it stays, closing it later returns the focus to the opening image's counterpart in the new render, or to the view when that one is gone.
- [ ] A Reload started by its key still closes it first (ticket 02); only a Reload because the file changed keeps it.
- [ ] Page tests (written first): a Reload with the image still there keeps it shown and updates the count; one that removes it closes the lightbox; a repeated source keeps its occurrence; the focus after closing goes to the counterpart, or to the view.
- [ ] Hand check in the release build: editing the document's text elsewhere in an editor while a screenshot is open keeps it; deleting the image's line closes it.
- [ ] `npm run check` passes.
