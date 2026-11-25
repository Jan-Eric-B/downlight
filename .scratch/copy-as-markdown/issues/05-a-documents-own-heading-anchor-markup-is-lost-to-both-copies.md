# 05: A document's own heading anchor markup is lost to both copies

**What to fix:** a document's raw HTML can write the markup that marks the heading anchors and footnote back references the core renders, and both copies then take the document's link for the page's own:

- `<a class="anchor" href="https://example.com">Text</a>` in a document is dropped from Ctrl+C's HTML together with its text (`linkMarkup` in `page/src/clean-copy.ts` leaves out every `a.anchor` and every link with `data-footnote-backref`).
- A Ctrl+Shift+C of a selection inside only that link finds no text of the document's (`documentRun` in `page/src/copy-as-markdown.ts` skips text in `a.anchor`) and copies the whole file instead of the link's line.

The sanitizer allows the class `anchor` (`CLASSES` in `core/src/sanitize.rs`) and the attribute `data-footnote-backref` on `a` (`TAG_ATTRIBUTES`) to every document, since rendering writes them and github-markdown-css styles them. Only rendering should be able to write them, as only rendering can write `data-sourcepos` (`LINE_NUMBER_ATTRIBUTE`, renamed in a document's raw HTML by `drop_from_raw_html` so that sanitizing drops it) and task list checkboxes (`RENDERING_ONLY_TAGS`).

Found in the architecture review of Copy as Markdown (2026-11-25). It takes a document that writes this markup on purpose, so it is rare.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The class `anchor` and the attributes `data-footnote-backref` and `data-footnote-backref-idx` are rendering-only: a document's raw HTML that writes them loses them in sanitizing, and keeps its link, its `href` and its text. Rendering's own heading anchors and back references keep them.
- [ ] Core tests (`core/src/sanitize.rs`): a raw `<a class="anchor" href="https://example.com">Text</a>` and a raw `<a data-footnote-backref href="#x">Back</a>` come out as plain links; a heading's anchor and a footnote's back reference still carry them.
- [ ] Page tests through Ctrl+C (`page/src/clean-copy.test.ts`) and Ctrl+Shift+C (`page/src/copy-as-markdown.test.ts`) on a fixture with such a link: the HTML keeps the link and its text, and a selection inside it copies its line.
- [ ] A rendering change: `npm run fixtures` writes the HTML of any fixture that changes; review its diff. Avoid running it beside right-to-left/01 or another rendering change.
