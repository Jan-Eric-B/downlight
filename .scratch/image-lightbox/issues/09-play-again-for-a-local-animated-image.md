# 09: "Play again" for a local animated image

**What to build:** An animated image starts from its first frame whenever the **Lightbox** shows it, while the document's copy plays on. Once a **Local image** (GIF, WebP or APNG) has played as many times as its file says, a "Play again" button appears beside "3 of 12"; it, or Space while it shows, plays the image once more. An image that loops forever, and a remote one, never show it.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 03 (Step through the document's images)

**Status:** ready-for-agent

- [ ] A new core function reads, from an image's bytes, whether it is animated, and if so whether it loops forever or how many times it plays and how long one play lasts, for GIF, WebP and APNG. A truncated or malformed file reads as not animated, never a panic. Hand-written or with maintained crates (`gif`, `png`, `image-webp`), decided by how much code a crate saves; a crate is added with `cargo add` and passes cargo-deny and the notices.
- [ ] A new app command takes only a local image's source, validates it as the image scheme does (network share rules included), reads the file on a background task and answers with the core's reading. Only the page's app module calls it.
- [ ] The lightbox shows each image through its own image element, so the document's copy is untouched. For a local image that plays a finite number of times, it starts the image from its first frame when shown and shows "Play again" once that time has passed; the button (Fluent icon, tooltip "Play again (Space)") and Space replay it.
- [ ] No button for an image that isn't animated, loops forever, or is remote.
- [ ] Core tests (written first): small GIF, WebP and APNG test files that aren't animated, loop forever, and play once and three times give the right reading and the length of one play; truncated and malformed files read as not animated.
- [ ] Page tests (written first): "Play again" shows after the answer's time (with fake timers) and not before; it and Space restart the timer; none for loops forever, not animated, or remote; stepping away and back starts over.
- [ ] Contract: the app's tests write a sample of the answer, and the page's contract tests drive it; `npm run fixtures` writes it.
- [ ] Hand check in the release build: a GIF that plays once restarts with "Play again"; the document's copy is untouched. If a second element for the same URL doesn't restart the animation in WebView2 (Chromium may play every copy of a URL in step), the lightbox's copy of a local image gets its own URL, a query the image scheme ignores; record which in a comment.
- [ ] `npm run check` passes.
