# 03: Step through the document's images

**What to build:** In the **Lightbox**, Right and Left step to the next and previous image of the document, PageDown and PageUp step too, and Home and End go to the first and last. Previous and Next buttons sit at the sides, each hidden at its end, and a touch swipe steps. Stepping stops at the ends. At the bottom, "3 of 12" and the image's alt text say where the reader is and what the author says about it.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 02 (A click on an image shows it in the lightbox)

**Status:** ready-for-agent

- [ ] Right and Left, PageDown and PageUp step through the list of ticket 02 in document order, linked images included; Home and End go to the first and last; stepping stops at the ends without wrapping.
- [ ] Previous and Next buttons at the sides (tooltips "Previous image (Left)", "Next image (Right)", Fluent icons), the first hidden on the first image and the second on the last.
- [ ] A horizontal touch swipe steps.
- [ ] The caption at the bottom: "3 of 12" and the alt text (none when the image has no alt text).
- [ ] Each image stepped to opens at its opening size, and the dialog's name follows the image shown.
- [ ] Closing after stepping returns the focus to the image the lightbox was opened from, and the document hasn't moved.
- [ ] Page tests (written first): each key steps, the ends stop, Home and End; the buttons step and hide at the ends; "3 of 12" and the alt text follow the step; the dialog's name follows; closing after stepping returns the focus to the opening image.
- [ ] Hand check in the release build: stepping through a document's screenshots by key, button and swipe on a touch screen; the caption in both themes.
- [ ] `npm run check` passes.
