# 33: The page's own parts stop borrowing the document's class

**What to build:** A refactor of the page's styles, with no change the reader sees. `.markdown-body` means the rendered document and nothing else. The page's own parts (the find bar, notices, the "File deleted or moved" bar, the link target bubble and the empty window) get github-markdown-css's font and light or dark colors another way.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** needs-triage

- [ ] Decide first whether this is worth it: github-markdown-css defines its color variables only inside `.markdown-body`, so the page's parts need them from somewhere else (a wrapper, the variables copied under a class of the page's own, or a build step). Record the choice here.
- [ ] The page's parts no longer carry `markdown-body`; the stylesheet's resets for them (`max-width: none`, `margin: 0`) go.
- [ ] `DOCUMENT_VIEW_SELECTOR` (`article.markdown-body`) can be `.markdown-body`, and `.markdown-body > *` (ADR 0003's rule) needs no `article`.
- [ ] Light, dark and high contrast look as before, checked by hand.

## Comments

**2026-11-13, from the architecture review.** Speculative. The visible effect, `content-visibility` on the find bar's input and the empty window's button, was fixed separately by narrowing ADR 0003's rule to `article.markdown-body > *`.
