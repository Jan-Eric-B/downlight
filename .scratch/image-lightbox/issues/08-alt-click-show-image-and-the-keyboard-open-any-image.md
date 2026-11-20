# 08: Alt+click, "Show image" and the keyboard open any image

**What to build:** Every image of the **Lightbox**'s list can be opened without a plain click: Alt+click opens any image, linked or not; the right-click menu on an image offers "Show image", which Shift+F10 and the menu key reach on a focused image or a link around one; and unlinked images are tab stops that Enter or Space opens. Inside the lightbox, the right-click menu offers only Close.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 02 (A click on an image shows it in the lightbox)

**Status:** ready-for-agent

- [ ] Alt+click on an image of the list opens the lightbox, linked or not; the link isn't followed (the link handling already ignores Alt+click).
- [ ] A new command "Show image", in the right-click menu only when the menu was opened on an image of the list, by the pointer or by Shift+F10 or the menu key on a focused image or a link around one; it opens the lightbox on that image. Its entry names Alt+click as the way to open a linked image's lightbox by mouse.
- [ ] Inside the lightbox, the right-click menu has only Close.
- [ ] Unlinked images (and self-linked ones, ticket 07, if built) are tab stops; Enter or Space opens the lightbox. A linked image keeps only its link's tab stop.
- [ ] Page tests (written first): Alt+click opens a linked and an unlinked image and follows no link; the menu has "Show image" over an image and not elsewhere, and choosing it opens that image; the menu key on a focused link around an image offers it; inside the lightbox the menu has only Close; Tab reaches an unlinked image and Enter and Space open it; a linked image has one tab stop.
- [ ] Contract: the menu entries keep their shape; if the samples change, `npm run fixtures` writes them.
- [ ] Hand check in the release build: the menu over an image and inside the lightbox; Shift+F10 and the menu key; Tab and Enter through a document's images with a screen reader.
- [ ] `npm run check` passes.
