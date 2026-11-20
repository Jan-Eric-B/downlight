# 01: Image lightbox

**What to build:** Clicking an image in the document shows it large over the document, at up to its full size; arrow keys step to the previous or next image, and Esc or a click outside closes it. For screenshots and diagrams that the reading width shrinks.

Spec: none yet. Grill it first (`/grill-with-docs`).

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - An image inside a link: does the click follow the link (as GitHub does) or open the lightbox? Badges, which are usually linked, then never open it.
  - Which images: every image, or only ones shown smaller than their real size? Remote images and **Local images** alike (the lightbox reuses the loaded image, so it adds no request).
  - Zooming and panning inside the lightbox, or only fit to the window?
  - Esc is already taken by Find and **Stop**: the lightbox, while open, takes it first.
  - Its glossary term, and keyboard and screen reader access (focus moves into it and back to the image).
  - A **Reload** while it is open: it closes, or stays on the same image if it is still there.

## Comments

**2026-11-20, from the repo owner.** Picked from the mdhero comparison (`.scratch/mdhero-comparison/features.md`, "Image lightbox").
