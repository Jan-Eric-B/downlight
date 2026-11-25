import { askApp, type Piece } from "./app";
import { codeHolding, codeLinesLeftOut, selectedCode } from "./copy-code-block";
import { copyCode } from "./copy";
import { cellHolding, cellSpan, rowsLeftOut, tableHolding } from "./copy-table";
import { showingWhy } from "./notice";
import { lineRangeOf, type LineRange } from "./sourcepos";
import { DOCUMENT_VIEW_SELECTOR, selectionInView, showingInView } from "./view";

/**
 * Copy as Markdown (copy-as-markdown ticket 01): the document's Markdown on the clipboard instead
 * of the text the reader sees. The page names the pieces of the file the selection covers, by the
 * line numbers its rendered blocks carry (`data-sourcepos`), and the app copies their Markdown
 * from the file as the shown render was made from it, so a large copy never passes through the
 * page. Only blocks carry line numbers, so a selection copies the whole lines of the blocks it
 * touches, except in a code block, which copies only the code lines it touches
 * (`copy-code-block.ts`, ticket 02), and in a table, whose cells and rows carry their own
 * (`copy-table.ts`, ticket 03).
 *
 * A selection touches a block, a code line or a row by holding one of its characters, wherever
 * its ends are: Chromium's triple-click ends a selection at the start of the next block, and a
 * drag can start at the end of the block before, and neither holds any of that block's text.
 */

/** Selects every block of the rendered document, each of which carries its lines. */
const BLOCK_SELECTOR = "[data-sourcepos]";

/**
 * Selects what the page adds to the document view, whose text is none of the document's. Neither
 * holds text yet, but a part the page adds with text of its own must be listed here, or a
 * selection over it copies lines it doesn't come from.
 */
const OWN_PARTS_SELECTOR = ".zeroclipboard-container, a.anchor";

/** No pieces: the whole file. */
const WHOLE_FILE: Piece[] = [];

/** Whether Copy as Markdown can run now: while the view shows a document's render. */
export function canCopyAsMarkdown(): boolean {
  return showingInView() !== null;
}

/**
 * Asks the app to copy the Markdown of what the selection holds in the document view, or of the
 * whole file, and shows why if it can't. Does nothing while the view shows no render.
 */
export function copyAsMarkdown(): void {
  const showing = showingInView();
  if (showing === null) return;
  const range = selectionInView();
  // Ctrl+A selects the whole view, which copies the whole file without a walk over its text.
  const runs = range && !holdsWholeView(range) ? selectedRuns(range) : [];
  // Inside one code block, the Markdown is the selected text itself, which the page writes as the
  // copy buttons do.
  const code = sharedHolder(runs, codeHolding);
  if (range && code) {
    void copyCode(selectedCode(range, code));
    return;
  }
  showingWhy(askApp("copy_markdown", { showing, pieces: piecesOf(runs) }));
}

function holdsWholeView(range: Range): boolean {
  const { startContainer: view } = range;
  return (
    view instanceof Element &&
    view.matches(DOCUMENT_VIEW_SELECTOR) &&
    range.endContainer === view &&
    range.startOffset === 0 &&
    range.endOffset === view.childNodes.length
  );
}

/** The characters of one text node of the document that a selection holds, `start` to `end`. */
interface Run {
  text: Text;
  start: number;
  end: number;
}

/**
 * The runs of characters `range` holds, in document order. The page's own parts hold none of the
 * document's, and space between blocks, outside code, is no block's. The walk starts where the
 * range does and visits each of its text nodes once, reading no layout, so it grows with the
 * selection, not with the document.
 */
function selectedRuns(range: Range): Run[] {
  const walker = document.createTreeWalker(
    range.commonAncestorContainer,
    NodeFilter.SHOW_TEXT,
  );
  const runs: Run[] = [];
  for (let node = walkFrom(range, walker); node; node = walker.nextNode()) {
    if (!(node instanceof Text)) continue;
    // `comparePoint` is above 0 for a point after the range.
    if (range.comparePoint(node, 0) > 0) break;
    const start = node === range.startContainer ? range.startOffset : 0;
    const end = node === range.endContainer ? range.endOffset : node.length;
    const run = documentRun(node, start, end);
    if (run) runs.push(run);
  }
  return runs;
}

/** Puts `walker` where `range` starts, and returns the first text node there, if it has one. */
function walkFrom(range: Range, walker: TreeWalker): Node | null {
  const { startContainer: container, startOffset: offset } = range;
  // The node the range starts at: the text node it starts in, or the child its offset names.
  const first =
    container instanceof Text ? container : container.childNodes[offset];
  let at = first ?? container;
  // A start after an element's last child: the walk goes on after everything in it.
  if (!first) while (at.lastChild) at = at.lastChild;
  walker.currentNode = at;
  return first instanceof Text ? first : walker.nextNode();
}

/** The characters `start` to `end` of `text` as a run of the document's, if they are one. */
function documentRun(text: Text, start: number, end: number): Run | undefined {
  const parent = text.parentElement;
  if (!parent || parent.closest(OWN_PARTS_SELECTOR)) return undefined;
  let from = start;
  if (parent.closest("pre")) {
    // A line break is part of the line it ends, and a selection that starts at the end of a code
    // line, before its line break, touches none of that line.
    while (from < end && text.data[from] === "\n") from++;
  } else if (text.data.slice(start, end).trim() === "") return undefined;
  return from < end ? { text, start: from, end } : undefined;
}

/** What `holder` gives for every run alike, or null when the runs are in different ones or none. */
function sharedHolder(
  runs: readonly Run[],
  holder: (node: Node) => Element | null,
): Element | null {
  const [first] = runs;
  const shared = first && holder(first.text);
  return shared && runs.every(({ text }) => holder(text) === shared)
    ? shared
    : null;
}

/**
 * The pieces of the file `runs` cover: the span of the one table cell they are all in, or else
 * the lines from the first line of the blocks they touch to the last, leaving out the code lines,
 * table rows and nested blocks of those blocks that they don't. No runs, and runs in no block, as
 * in raw HTML, are the whole file.
 */
function piecesOf(runs: readonly Run[]): Piece[] {
  const cell = sharedHolder(runs, cellHolding);
  const span = cell && cellSpan(cell);
  if (span) return [span];
  const first = runs[0];
  const last = runs.at(-1);
  if (!first || !last) return WHOLE_FILE;
  const selected = document.createRange();
  selected.setStart(first.text, first.start);
  selected.setEnd(last.text, last.end);
  const blocks = new Map<Element, LineRange>();
  for (const { text } of runs) {
    const block = blockHolding(text);
    const lines = block && !blocks.has(block) && lineRangeOf(block);
    if (block && lines) blocks.set(block, lines);
  }
  if (blocks.size === 0) return WHOLE_FILE;
  let from = Infinity;
  let to = 0;
  // Footnotes render at the end of the view, but keep their definitions' lines.
  for (const [start, end] of blocks.values()) {
    from = Math.min(from, start);
    to = Math.max(to, end);
  }
  const leftOut = [...blocks.keys()].flatMap(
    (block) =>
      codeLinesLeftOut(block, selected) ??
      rowsLeftOut(block, selected) ??
      blocksLeftOut(block, selected),
  );
  return linesWithout(from, to, leftOut);
}

/**
 * The block whose lines a character in `text` copies: the innermost one, or the table of a cell,
 * which copies as its header, delimiter and touched rows (`copy-table.ts`).
 */
function blockHolding(text: Text): Element | null {
  return (
    tableHolding(text) ?? text.parentElement?.closest(BLOCK_SELECTOR) ?? null
  );
}

/**
 * The lines of the blocks inside `block` that `selected` doesn't reach, such as a list item's
 * nested list after the text selected in it: a block is touched by its own characters, and the
 * blocks inside it only by theirs.
 */
function blocksLeftOut(block: Element, selected: Range): LineRange[] {
  return blocksIn(block)
    .filter((inner) => !selected.intersectsNode(inner))
    .map(lineRangeOf)
    .filter((lines) => lines !== undefined);
}

/** The outermost blocks inside `element`, looking through the elements that aren't blocks. */
function blocksIn(element: Element): Element[] {
  return [...element.children].flatMap((child) =>
    child.matches(BLOCK_SELECTOR) ? [child] : blocksIn(child),
  );
}

/** The lines from `first` to `last` except the `leftOut` ranges, which are inside them, as pieces. */
function linesWithout(
  first: number,
  last: number,
  leftOut: LineRange[],
): Piece[] {
  const pieces: Piece[] = [];
  let next = first;
  for (const [outFirst, outLast] of leftOut.sort(([a], [b]) => a - b)) {
    if (outFirst > next)
      pieces.push({ piece: "lines", first: next, last: outFirst - 1 });
    next = Math.max(next, outLast + 1);
  }
  if (next <= last) pieces.push({ piece: "lines", first: next, last });
  return pieces;
}
