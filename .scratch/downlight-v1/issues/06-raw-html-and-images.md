# 06: Raw HTML and images, safely

**What to build:** The raw HTML that GitHub allows renders, and nothing that could act on its own does. Relative and local images load through Downlight's own scheme, remote images load as on GitHub, and images on network shares never load, so opening a document can't make Windows send credentials to a server.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 04 (Documents look like GitHub on Mica)

**Status:** ready-for-agent

- [ ] Sanitizing keeps `<details>`, `<summary>`, `<picture>`, `<source>`, `<img>` with `width`, `height` and `align`, `align` on block elements, `<kbd>`, `<sub>` and `<sup>`, plus the task list checkboxes, heading ids, line numbers and classes that rendering produces.
- [ ] Sanitizing removes scripts, styles, frames, objects, event handler attributes and `javascript:` URLs.
- [ ] Sanitizing removes the tags GitHub's tagfilter escapes (`<title>`, `<textarea>`, `<style>`, `<xmp>`, `<iframe>`, `<noembed>`, `<noframes>`, `<script>`, `<plaintext>`). comrak 0.59 has no tagfilter (removed in 0.56, see ticket 04), so this is the only place they are stopped. GitHub shows them as escaped text; Downlight drops them.
- [ ] A `<picture>` with light and dark sources shows the image for the current mode.
- [ ] Relative image paths resolve against the document's folder and load through Downlight's own scheme, and so do absolute local paths.
- [ ] Remote `http(s)` images load.
- [ ] Images on network shares (UNC paths, `file://server/`) are removed before the page sees them; opening the document makes no connection to the share.
- [ ] The content security policy is final: scripts only from Downlight, images from Downlight's own scheme, `data:` URLs and remote `http(s)` addresses.
- [ ] Core tests cover what is kept, what is removed and each kind of image source.
