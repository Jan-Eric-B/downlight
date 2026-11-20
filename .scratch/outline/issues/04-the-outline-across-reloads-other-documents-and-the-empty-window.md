# 04: The Outline across reloads, other documents and the empty window

**What to build:** The **Outline** always lists the document the reader is reading, or says why it lists nothing. A **Reload** keeps it open with what the reader typed and chose; going to another document closes it; a document without headings, the empty window and a document still on its way each get their own words.

Spec: [Outline](../spec.md)

**Blocked by:** 03 (Typing filters the Outline)

**Status:** ready-for-agent

- [ ] On a reload (`whenDocumentShown` with reason `reload`) an open Outline stays open, rebuilds its rows from the new list of headings, keeps the filter's text and keeps the selected heading by its id, or else selects the first kept row.
- [ ] A new document or a return (reasons `shown` and `return`: a link, Back, Forward, Open, a drop, Paste) closes it, and so does a **Wait note** for a new document appearing.
- [ ] The button, Ctrl+Shift+O and the menu entry always work: in a document whose list of headings is complete and empty the flyout says "No headings in this document"; in the empty window and while a Wait note for a new document shows it says "No document". In both the filter box is there but disabled, and the note is a live status.
- [ ] Page tests (written first): a reload keeps it open, its filter and its selected heading, and shows a heading the reload added; a reload that removed the selected heading selects the first kept row; giving another document, Back and a Wait note close it; "No headings in this document"; "No document" in the empty window and during a Wait note.
- [ ] `npm run check` passes.
