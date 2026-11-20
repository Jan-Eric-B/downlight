# 04: Zoom and pan the image

**What to build:** In the **Lightbox** the wheel zooms the image in and out around the pointer, from a tenth of its fitted size up to 16 times its full size, so a small image can always be made bigger. Ctrl+wheel, a touchpad's or a touch screen's pinch, Ctrl+Plus and Ctrl+Minus zoom it too, and Ctrl+0 fits it again, never changing Downlight's zoom setting. Double-click switches between fitted and full size. Dragging pans an image larger than the window, and so do the arrow keys while it is; once it fits again they step.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 03 (Step through the document's images)

**Status:** ready-for-agent

- [ ] The wheel zooms around the pointer, within a tenth of the fitted size and 16 times full size (16 times fitted for an SVG); the browser's smoothing is kept above full size.
- [ ] While the lightbox is open, the commands' Ctrl+wheel handling (which a touchpad's pinch sends) and Zoom in, Zoom out and Reset zoom act on the image; Reset zoom fits it; none of them asks the app to change the zoom setting. A touch screen's pinch zooms.
- [ ] Double-click switches between fitted and full size around the pointer.
- [ ] Dragging pans an image larger than the area, kept within the image's bounds; a pointer that moved beyond a small threshold between down and up is a drag and never closes the lightbox.
- [ ] The arrow keys pan while the image is larger than the area, and step (ticket 03) while it fits; PageUp, PageDown, Home and End always step.
- [ ] A zoom percentage, relative to full size, shows briefly after each change.
- [ ] Each image stepped to starts at its opening size again. On a window resize, a fitted image fits the new area and a zoomed one keeps its zoom and pan.
- [ ] Page tests (written first): the wheel zooms in and out within the limits; Ctrl+wheel, Ctrl+Plus and Ctrl+Minus zoom the image and Ctrl+0 fits it, and the page asks the app nothing; double-click switches between fitted and full size; a drag pans and doesn't close; the arrows pan a large image and step a fitted one; stepping resets the zoom; a resize refits a fitted image and keeps a zoomed one.
- [ ] Hand check in the release build: the wheel zooms around the pointer; pinch on a touchpad and on a touch screen; smoothing above full size; dragging and the arrows pan; the zoom percentage shows and goes; Downlight's zoom is unchanged after closing.
- [ ] `npm run check` passes.
