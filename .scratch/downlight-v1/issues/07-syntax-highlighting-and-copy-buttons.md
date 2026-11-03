# 07: Syntax highlighting and copy buttons

**What to build:** Code blocks with a language are highlighted in colors that fit light and dark, and each code block has a copy button. Highlighting happens in the core, so the page runs no highlighting code.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 06 (Raw HTML and images, safely)

**Status:** ready-for-agent

- [ ] Fenced code blocks with a known language are highlighted by syntect through comrak's plugin, using the pure-Rust regex feature, with CSS classes rather than inline colors.
- [ ] There are colors for light and dark, and a switch in Windows recolors the document without rendering it again.
- [ ] Sanitizing keeps the highlighting classes.
- [ ] A code block with an unknown language or none shows as plain code.
- [ ] The front matter block is highlighted as YAML.
- [ ] Each code block has a copy button that copies exactly the code's text.
- [ ] Measured again with the timing switch, with the results recorded in this ticket: the cold first paint target is still met, the 1 MB document still meets its targets, and the growth in binary size is recorded.
