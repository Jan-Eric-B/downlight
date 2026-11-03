# 04: Documents look like GitHub on Mica

**What to build:** A README renders the way GitHub shows it: GitHub Flavored Markdown, footnotes, alerts, GitHub's heading anchors, emoji shortcodes, and front matter as a YAML code block. Every block carries its **line number**. The document sits on Mica in light and dark, following Windows, with a solid background where Mica is off.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [ ] Tables, task lists, strikethrough, autolinks and tagfilter render as on GitHub.
- [ ] Footnotes render with links from the reference to the note and back.
- [ ] Alerts (NOTE, TIP, IMPORTANT, WARNING, CAUTION) render as colored callouts in light and dark.
- [ ] Heading anchors match GitHub's, including repeated headings, punctuation and non-ASCII text.
- [ ] Emoji shortcodes such as `:tada:` render as emoji.
- [ ] YAML front matter between `---` lines shows as a YAML code block at the top.
- [ ] Every block element carries the line of the file where it starts.
- [ ] Light and dark follow Windows, also when Windows switches while the window is open.
- [ ] On Mica the page draws no background of its own. Code blocks, tables and quotes keep their fills, and where Mica is off (Windows 10, transparency effects off, high contrast) the background is solid and readable.
- [ ] Checked by hand and recorded in this ticket, moved here from ticket 02: with "Transparency effects" off the background is solid, and in high contrast it is solid and readable.
- [ ] The window opens hidden and shows once the document is painted, so it never flashes white or shows an empty frame while WebView2 starts. Ticket 02 found about 500 ms of solid white in dark mode; `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` doesn't help, and a transparent background makes the window see-through instead. First paint with `npm run measure` stays within the spec's targets.
- [ ] The page's first frame, before the document arrives, measured again with `npm run measure` after the stylesheet work. Ticket 02 found it at 124 to 189 ms in about half the runs, over the 100 ms target. If it is still over, a ticket is opened for it.
- [ ] The core's render tests cover each item above that the core renders.
