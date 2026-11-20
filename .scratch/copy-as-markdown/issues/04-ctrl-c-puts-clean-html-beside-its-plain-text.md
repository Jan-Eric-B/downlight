# 04: Ctrl+C puts clean HTML beside its plain text

**What to build:** Ctrl+C, and the menu's Copy, keep putting the same plain text on the clipboard, and put clean HTML beside it, so that a paste into Word or Outlook keeps the document's formatting in the target's own styles. That HTML has:

- headings, emphasis, lists, tables, quotes and code, with none of Downlight's colors, theme, copy buttons or heading anchors;
- code as plain monospace;
- alerts as quotes with their title, and footnotes as a numbered list;
- local images by their file path, web images by their URL, and images that don't load as their alt text;
- web and mail links as links, every other link as its text;
- ticks as ☑ and ☐;
- a sorted table in the order shown.

Ctrl+A then Ctrl+C copies the whole document this way. A copy in the Find box or another field is left to the webview.

Spec: [Copy as Markdown, and a clean rich copy](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Before building: a hand check of what today's Ctrl+C pastes into Word in both themes (expected: the dark theme's colors come along), written under Results below.
- [ ] A new page module listens for the copy event, which Ctrl+C and the menu's Copy both raise. When the selection lies in the document view, it cancels the webview's own copy and sets the plain text and the HTML itself. A selection in the Find box or another field is left alone.
- [ ] The plain text is the selection's text as the webview copied it before.
- [ ] The HTML is rebuilt from the selected part of the view with an allowlist:
  - Kept: the structural elements (headings, paragraphs, emphasis, lists, quotes, tables, `pre` and `code`, images, links, footnotes), with no classes, styles or ids.
  - Removed: copy buttons, heading anchors and Find's highlights (their text stays).
- [ ] Alerts become `blockquote`s whose first paragraph is their title. A task item's checkbox becomes ☑ or ☐ followed by a space, by its tick.
- [ ] Images: a local image's image-scheme URL becomes its `file:///` URL, a web image keeps its URL, and an image without a source becomes its alt text. Links: `http(s)` and `mailto` stay links, and any other becomes its text.
- [ ] The rows of a table come in the order the view shows them.
- [ ] Page tests (rendered fixtures), checking what the copy event puts on the clipboard:
  - colors, classes and highlighting gone;
  - copy buttons and anchors gone;
  - alerts as quotes with their title;
  - ticked and unticked items as ☑ and ☐;
  - local, web and unloaded images;
  - kept and dropped links;
  - reordered rows in the view's order;
  - the plain text set;
  - a copy in the Find box left alone.

  Prior art: `copy.test.ts`, `ticks.test.ts`.
- [ ] Ctrl+C of the whole 1 MB test document (Ctrl+A first) is measured with the timing switch, and the medians are written under Results below.
- [ ] Hand checks in the release build:
  - Ctrl+C and the menu's Copy pasted into Word and Outlook in both themes, with local and web images, links, ticks and alerts;
  - the plain text pasted into Notepad, compared with the build before on the same selections (headings, a table, a list, an image with alt text, a non-breaking space). If they differ, the plain text is made to match the build before;
  - copying in the Find box.
- [ ] `npm run check` passes.

## Results

(to be filled in when built)
