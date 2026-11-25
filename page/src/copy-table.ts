import type { Piece } from "./app";
import { lineRangeOf, sourcePosition, type LineRange } from "./sourcepos";

/**
 * Copy as Markdown of part of a table (copy-as-markdown ticket 03). A rendered table, each of its
 * rows and each of its cells carry their line numbers (`data-sourcepos`), and a GFM table's row is
 * one line of the file: its header row's, then its delimiter row's on the line after it, which
 * renders as nothing, then one line for each row. A cell's line numbers name its columns too, so a
 * selection within one cell is that cell's span, and one across cells or rows leaves out the rows
 * it doesn't touch, taken by their line numbers, so a sorted table copies in the file's order.
 */

const TABLE_SELECTOR = "table[data-sourcepos]";
const CELL_SELECTOR = "th[data-sourcepos], td[data-sourcepos]";

function holding(node: Node, selector: string): Element | null {
  const element = node instanceof Element ? node : node.parentElement;
  return element?.closest(selector) ?? null;
}

/** The table that `node` is in, or null outside every table. */
export function tableHolding(node: Node): Element | null {
  return holding(node, TABLE_SELECTOR);
}

/** The table cell that `node` is in, or null outside every cell. */
export function cellHolding(node: Node): Element | null {
  return holding(node, CELL_SELECTOR);
}

/**
 * The span of `cell`: its line and its first and last column, spaces around its text included,
 * which the app trims. Undefined when its line numbers aren't a cell's, which are on one line.
 */
export function cellSpan(cell: Element): Piece | undefined {
  const position = sourcePosition(cell);
  if (!position) return undefined;
  const { start, end } = position;
  if (
    end.line !== start.line ||
    start.column === 0 ||
    end.column < start.column
  )
    return undefined;
  return {
    piece: "span",
    line: start.line,
    first: start.column,
    last: end.column,
  };
}

/**
 * The lines of `block`'s body rows that `selected`, from the first selected character to the
 * last, doesn't reach, when `block` is a table: one range for each row. The header row, and the
 * delimiter row after it, which no selection can touch, are never left out. Undefined for any
 * other block.
 */
export function rowsLeftOut(
  block: Element,
  selected: Range,
): LineRange[] | undefined {
  if (!block.matches(TABLE_SELECTOR)) return undefined;
  return [...block.querySelectorAll(":scope > tbody > tr[data-sourcepos]")]
    .filter((row) => !selected.intersectsNode(row))
    .map(lineRangeOf)
    .filter((lines) => lines !== undefined);
}
