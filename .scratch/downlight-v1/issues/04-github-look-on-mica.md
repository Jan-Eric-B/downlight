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
- [ ] On Mica the page draws no background of its own. Code blocks, tables and quotes keep their fills, and where ticket 02 found Mica off (Windows 10, transparency effects off, high contrast) the background is solid and readable.
- [ ] The core's render tests cover each item above.
