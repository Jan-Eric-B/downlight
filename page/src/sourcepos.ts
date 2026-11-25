/**
 * The line numbers the core gives every rendered block: comrak's `data-sourcepos`,
 * `line:column-line:column`, where the block starts and ends in the file, 1-based.
 */

/** The first and last line of a range of the file, both included. */
export type LineRange = [first: number, last: number];

/** A place in the file: its line and column, 1-based. */
export interface SourcePoint {
  line: number;
  column: number;
}

/** Where a rendered block starts and ends in the file, both included. */
export interface SourcePosition {
  start: SourcePoint;
  end: SourcePoint;
}

const SOURCEPOS = /^(\d+):(\d+)-(\d+):(\d+)$/;

/**
 * Where `element` starts and ends in the file, by its `data-sourcepos`, as written. Undefined when
 * it has none, or one that isn't a position: no line 0, and no end before its start.
 */
export function sourcePosition(element: Element): SourcePosition | undefined {
  const [, ...parts] =
    SOURCEPOS.exec(element.getAttribute("data-sourcepos") ?? "") ?? [];
  const [startLine = 0, startColumn = 0, endLine = 0, endColumn = 0] =
    parts.map(Number);
  if (startLine === 0 || endLine < startLine) return undefined;
  return {
    start: { line: startLine, column: startColumn },
    end: { line: endLine, column: endColumn },
  };
}

/**
 * The lines of the file `element` comes from, by its `data-sourcepos`. comrak ends some blocks,
 * such as a list's last item, at column 0 of the line after them, which is none of theirs.
 */
export function lineRangeOf(element: Element): LineRange | undefined {
  const position = sourcePosition(element);
  if (!position) return undefined;
  const { start, end } = position;
  const last = end.column === 0 ? end.line - 1 : end.line;
  return [start.line, Math.max(start.line, last)];
}
