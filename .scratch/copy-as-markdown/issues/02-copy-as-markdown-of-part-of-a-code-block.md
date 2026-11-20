# 02: Copy as Markdown of part of a code block

**What to build:** **Copy as Markdown** of a selection within one code block copies exactly the selected text, as Ctrl+C does. A selection that runs into or out of a code block copies the code lines it touches, inside the block's own fences with its language, so the paste is valid Markdown holding only the code the reader chose. Front matter counts as a fenced block: its `---` lines are its fences. The copy button keeps copying the whole block's code.

Spec: [Copy as Markdown, and a clean rich copy](../spec.md)

**Blocked by:** 01 (Ctrl+Shift+C and the menu copy the Markdown of whole lines)

**Status:** ready-for-agent

- [ ] A selection that starts and ends inside the same code block is not sent to the app: the page writes the selected text to the clipboard itself, as the copy buttons do, and shows a Notice if that fails.
- [ ] A code block the selection covers only partly becomes its own pieces: its opening fence line, the code lines touched, and its closing fence line. The code line comes from counting the code's lines before the selection's point. A fenced block is told from an indented one, which has no fences, by comparing the block's lines with its code's lines. An unclosed fence at the end of the document has only its opening line.
- [ ] A selection from a paragraph into a code block copies the paragraph's lines through the touched code lines, then the closing fence. A selection from a code block into the text after it copies the opening fence, the touched code lines through the block's end, then the text's lines.
- [ ] Core tests: code lines between their fences, from pieces the page would send for a fenced block, a fenced block inside a list item (its shared indentation dropped), front matter, and an indented code block.
- [ ] Page tests (rendered fixtures): a selection within one code block writes its text without asking the app; the pieces sent for a selection into a code block, out of one, and into front matter; an indented code block sends no fence lines.
- [ ] Hand checks in the release build: Copy as Markdown pasted into VS Code and a GitHub comment from within a code block, into one, out of one, and into front matter.
- [ ] `npm run check` passes.

## Results

(to be filled in when built)
