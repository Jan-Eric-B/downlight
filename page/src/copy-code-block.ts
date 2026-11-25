import { lineRangeOf, type LineRange } from "./sourcepos";
import { rangeWithin } from "./view";

/**
 * Copy as Markdown of part of a code block (copy-as-markdown ticket 02). A rendered code block's
 * line numbers (`data-sourcepos`) cover its fences as well as its code, so which of its lines are
 * code is found by counting the code's lines and comparing: an indented block's lines are all code,
 * an unclosed fence has one more line than its code, and a fenced block, front matter included,
 * has its opening fence before the code and its closing fence after it.
 */

/**
 * Selects the code of a rendered code block, the only code that comes from a Markdown block. A
 * block drawn in place of its code, such as a diagram, must sit outside its `pre > code`, or its
 * drawing's text is copied and counted as code lines, and must keep the `data-sourcepos` of its
 * block, or a selection in it copies the whole file.
 */
const CODE_SELECTOR = "pre[data-sourcepos] > code";

/** The code of the code block that `node` is in, or null outside every code block. */
export function codeHolding(node: Node): Element | null {
  const element = node instanceof Element ? node : node.parentElement;
  return element?.closest(CODE_SELECTOR) ?? null;
}

/** The text `range` selects in `code`, which is what Ctrl+C copies of it. */
export function selectedCode(range: Range, code: Element): string {
  return rangeWithin(range, code)?.toString() ?? "";
}

/**
 * The code lines of `block` that hold none of the characters `selected` runs over, which starts
 * at the first selected character and ends after the last, when `block` is a code block; its
 * fences are never left out. Undefined for any other block. A block whose lines are not ones this
 * understands leaves out nothing, which copies it whole.
 */
export function codeLinesLeftOut(
  block: Element,
  selected: Range,
): LineRange[] | undefined {
  const code = block.firstElementChild;
  if (!code?.matches(CODE_SELECTOR)) return undefined;
  const lines = lineRangeOf(block);
  const layout = lines && layoutOf(code, lines);
  const within = rangeWithin(selected, code);
  if (!layout || !within) return [];
  const before = document.createRange();
  before.setStart(code, 0);
  before.setEnd(within.startContainer, within.startOffset);
  // A line break is part of the line it ends.
  const first = layout.firstCode + lineBreaks(before.toString());
  const last = first + lineBreaks(within.toString().slice(0, -1));
  const leftOut: LineRange[] = [
    [layout.firstCode, first - 1],
    [last + 1, layout.lastCode],
  ];
  return leftOut.filter(([from, to]) => from <= to);
}

/** Where a code block's code lies in the file. */
interface Layout {
  firstCode: number;
  lastCode: number;
}

function layoutOf(code: Element, [start, end]: LineRange): Layout | undefined {
  const codeLines = countLines(code.textContent);
  if (codeLines === 0) return undefined;
  const blockLines = end - start + 1;
  if (blockLines < codeLines) return undefined;
  // Indented: no fences. Otherwise fenced: the code starts after the opening fence, closed or not.
  const firstCode = blockLines === codeLines ? start : start + 1;
  return { firstCode, lastCode: firstCode + codeLines - 1 };
}

function lineBreaks(text: string): number {
  return text.split("\n").length - 1;
}

/** The lines `text` holds, a last line break not starting another. */
function countLines(text: string): number {
  const breaks = lineBreaks(text);
  return text === "" || text.endsWith("\n") ? breaks : breaks + 1;
}
