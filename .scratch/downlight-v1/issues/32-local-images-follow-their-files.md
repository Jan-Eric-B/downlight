# 32: Local images show their file's latest version after a reload

**What to build:** When a local image's file changes (a regenerated diagram, a new screenshot) and the document reloads, the window shows the new picture.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Checked first by hand in the release build: change a local image the document shows, save the document, and see whether the reload shows the new picture; then F5. Record the result here. If the new picture shows, close this ticket.
- [ ] If it doesn't: a reload loads each local image anew. A cache header alone likely won't do, since Chromium reuses an image its document has already loaded whatever the HTTP cache says. One way: the image scheme's URLs carry a per-render query, which `local_image` already ignores, written by the core from a value the app passes with each render, or added by the page to the template on a reload.
- [ ] An unchanged image doesn't flicker on a reload. Check it on a document with many images.
- [ ] Tested first where a test can observe it: the URLs a render writes (core), or the page's reload (page tests).

## Comments

**2026-11-13, from the architecture review.** The image scheme answers without cache headers (`app/src/image.rs`), and a reload writes the same URL for an image as before (`core/src/image.rs`, `local_source`).
