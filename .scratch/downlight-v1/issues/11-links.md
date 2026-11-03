# 11: Links

**What to build:** Clicking a link does the one safe, expected thing. A Markdown document shows in the same window (or in a new one with Ctrl+click), headings scroll, web and mail links go to the system, other local files are shown in Explorer and never run, and other schemes do nothing. Every link shows its target on hover.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 04 (Documents look like GitHub on Mica), 08 (One process, a window per document)

**Status:** ready-for-agent

- [ ] The core's link resolution takes the document's path and a link target and returns one action: show a document in this window (optionally at a heading), scroll to a heading, reveal a file in the file manager, open a folder, hand a URL to the system, or nothing.
- [ ] A link to a Markdown file (one of the six extensions) shows that document in the same window, at its heading if the link has one.
- [ ] A link to a heading in this document scrolls to it.
- [ ] Ctrl+click on a link to a Markdown file opens it in a new window, or brings its window to the front if it is open.
- [ ] `http`, `https` and `mailto` links open in the system's browser or mail app.
- [ ] A link to a local file that isn't Markdown shows the file selected in Explorer; Downlight never opens or runs it.
- [ ] A link to a folder opens it in Explorer.
- [ ] A link with any other scheme does nothing.
- [ ] Paths on network shares are handled like local paths, and only when clicked.
- [ ] Hovering any link shows its target.
- [ ] A link to a missing file shows a message.
- [ ] Core tests are a table of document paths and link targets with the action each one gives.
