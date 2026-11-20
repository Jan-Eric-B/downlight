# 02: A click on an image shows it in the lightbox

**What to build:** The tracer bullet. A click on an image in the document that isn't inside a link shows it in the **Lightbox**: on its own over the document, fitted to the window but never larger than its full size (one image pixel per screen pixel, whatever Windows' scaling and Downlight's zoom), an SVG filling the window. The backdrop is the theme's background, nearly opaque, and covers the area below the **Title bar** (the whole screen in **Full screen**), which keeps working. Esc, a Close button at the top right or a click on the backdrop closes it, and the reader is back where they were reading: the document never moves. An image inside a link still follows its link.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 01 (One place owns Esc's order)

**Status:** ready-for-agent

- [ ] A page module for the lightbox starts with the page's other per-window modules and gets the shown document as Find does; its signal removes its listeners for the tests.
- [ ] A plain click (button 0, no modifier) on an image outside a link opens it; the pointer over such an image is the zoom-in cursor. A click on an image inside a link goes to the link as today.
- [ ] The list of images, built on opening: every image of the shown document in its order that has loaded and that the theme doesn't hide (`#gh-dark-mode-only`, `#gh-light-mode-only`); a `<picture>` shows its current source. An image without a source (on a **Network share**) or that failed to load opens nothing. Building it reads no layout (ADR 0003).
- [ ] Opening size: the smaller of fitted and full size, where full size is the natural size divided by the screen pixels per CSS pixel; the lightbox sits outside the document view, so Downlight's zoom never applies to it. An SVG opens fitted.
- [ ] The lightbox covers the area below the title bar, the Find bar, the **Overview ruler** and the **Counts** bubble included, and the whole screen in full screen; the title bar and its buttons keep working. The backdrop is the theme's background, nearly opaque. Nothing animates under reduced motion.
- [ ] Closing: Esc (in its place in the order of ticket 01: after the settings flyout, before Find, Stop and full screen), the Close button (tooltip "Close (Esc)", drawn as the title bar's buttons with a Fluent icon) and a click on the backdrop. A click on the image does nothing. The document view never scrolls while it is open or when it closes, and the wheel doesn't scroll it.
- [ ] A modal dialog (`aria-modal`), named by the image's alt text, or its file name, or "Image"; the focus moves into it on opening, Tab cycles inside it, and closing returns the focus to the image it was opened from.
- [ ] Commands while it is open: Find and its next and previous, Open, Back, Forward, Reload by its key, Paste, Edit and Close close the lightbox first and then run; Keep on top, Full screen, Full width, Minimize, Maximize and the settings flyout leave it open.
- [ ] A new showing (Back, Forward, a drop, the command line, Paste) closes it at once.
- [ ] It adds no network request: the lightbox shows the image the document already loaded.
- [ ] Page tests (written first): a click opens the image and Esc, Close and a backdrop click close it; a linked image follows its link; broken, sourceless and theme-hidden images open nothing; the opening size from the natural size and the screen pixels per CSS pixel, never above full size, and an SVG fitted; Esc goes to the settings flyout first and to the lightbox before Find, Stop and full screen, and not while composing or with a modifier; the focus goes in and back; the dialog's name; which commands close it and which leave it; a new showing closes it.
- [ ] Measured: opening the lightbox on the 1 MB test document with the timing switch shows no task over 100 ms; the Results go in a comment here.
- [ ] Hand check in the release build: a screenshot opens at one image pixel per screen pixel at 100% and 150% Windows scaling, with Downlight's zoom at 100% and 150%; an SVG fills the window; the backdrop in both themes; the lightbox in full screen; the title bar's buttons work while it is open; a screen reader announces the dialog.
- [ ] `npm run check` passes.
