# 07: A link to its own image opens the lightbox

**What to build:** A link whose only content is an image and whose target is that same image, such as `[![shot](shot.png)](shot.png)` or the same with a remote URL, opens the **Lightbox** on a click instead of revealing the file in Explorer or opening the browser. Every other linked image still follows its link.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 02 (A click on an image shows it in the lightbox)

**Status:** ready-for-agent

- [ ] At render time, after sanitizing, the core marks a link whose only content is an image and whose target names the same file or URL as the image's source, read the way link targets and image sources are already read (no file system lookup). The mark is an attribute only the core writes; the sanitizer strips the same attribute when a document brings it.
- [ ] The page treats a marked link as an unlinked image: a click, Enter or Space opens the lightbox, no link target bubble shows, and its link is never followed.
- [ ] Core tests (written first): a local image linked to itself is marked, by a relative and an absolute path naming the same file; a remote one is marked; a link to another file, one with text beside the image and one to the image with a fragment or query that names something else are not; a document's own mark is stripped.
- [ ] A rendered fixture gains a self-linked image and a linked badge; `npm run fixtures` writes the HTML again and its diff shows only the mark.
- [ ] Page tests (written first): a click on a self-linked image opens the lightbox and asks the app nothing; a badge's link is still followed.
- [ ] `npm run check` passes.
