// Where the overview ruler's marks go (overview ruler spec, "Where a mark sits") and which one a
// click or the pointer acts on ("Clicks"): measuring the top-level blocks that hold the headings
// and Find's matches, and placing their marks in the strip's pixel rows. Only top-level blocks are
// measured, so no rectangle inside a block is ever read (ADR 0003).
import type { FindMatch } from "./find";
import type { DocumentHeading, HeadingLevel } from "./headings";
import type { ScrolledView } from "./reading-position";

/** The strip's width, in px, which the script sets `--overview-ruler-width` to for the stylesheet. */
export const STRIP_WIDTH = 14;

/**
 * How far from a mark, in px, a click or the pointer still acts on it: a mark is only 2 or 3 px
 * high.
 */
const MARK_REACH = 4;

/** How high a heading's mark is, in px, from its row down, as the stylesheet draws it. */
const HEADING_MARK_HEIGHT = 2;

/** How high a match's mark is, in px, from its row down. */
const MATCH_MARK_HEIGHT = 2;

/**
 * A heading mark as the strip draws it: the pixel row it starts on, the level its length is for,
 * and the headings it stands for, in document order, which a click goes to the first of and the
 * hover label names.
 */
export interface HeadingMark {
  row: number;
  level: HeadingLevel;
  headings: readonly [DocumentHeading, ...DocumentHeading[]];
}

/** The pixel rows of the strip that one match mark covers: the row it starts on, and how many. */
export interface MatchRun {
  row: number;
  height: number;
}

/** What a click or the pointer at a place on the strip acts on: a heading mark, or a match. */
export type MarkAt =
  { kind: "heading"; mark: HeadingMark } | { kind: "match"; index: number };

/** The strip's marks, placed for one list of headings and one count of Find. */
export interface PlacedStrip {
  /** The list of headings it was placed for, the same array until the list is rebuilt. */
  readonly headings: readonly DocumentHeading[];
  /** Find's matches it was placed for, the same array until Find counts again. */
  readonly matches: readonly FindMatch[];
  /** The strip's height, in px: the scroller's view's. */
  readonly height: number;
  /** Top to bottom, one per pixel row at most. */
  readonly headingMarks: readonly HeadingMark[];
  /** Top to bottom, one for each run of rows whose match marks touch, and so look as one. */
  readonly matchRuns: readonly MatchRun[];
  /** The pixel row match `index` is in, or undefined for one outside the view's blocks. */
  matchRow(index: number): number | undefined;
  /**
   * The mark a click or the pointer `x` px right of the strip's left edge and `y` px below its top
   * acts on, or undefined off the strip and where no mark is within reach.
   */
  markAt(x: number, y: number): MarkAt | undefined;
}

/**
 * Places the strip's marks for `headings` and `matches` as `shown` is laid out now, as the scroll
 * bar scales its thumb: a block's top in the scroller's content, scaled by the strip's height over
 * the scroller's scroll height. It only reads the layout, so a redraw that writes after it lays the
 * page out once.
 */
export function placeStrip(
  shown: ScrolledView,
  headings: readonly DocumentHeading[],
  matches: readonly FindMatch[],
): PlacedStrip {
  const { view, scroller } = shown;
  const height = scroller.clientHeight;
  const scrollHeight = scroller.scrollHeight;
  const toStrip = (px: number) => (px * height) / scrollHeight;
  const boxOf = blockMeasurer(shown);

  const headingMarks = markHeadings(view, headings, (block) =>
    Math.floor(toStrip(boxOf(block).top)),
  );
  const headingAt = Float64Array.from(
    headingMarks,
    ({ row }) => row + HEADING_MARK_HEIGHT / 2,
  );
  const matchAt = new Float64Array(matches.length).fill(NaN);
  placesOf(view, matches).forEach((place, index) => {
    if (!place) return;
    const { top, height: blockHeight } = boxOf(place.block);
    matchAt[index] = toStrip(top + place.share * blockHeight);
  });

  const headingRows = new RowIndex(headingAt, height);
  const matchRows = new RowIndex(matchAt, height);
  return {
    headings,
    matches,
    height,
    headingMarks,
    matchRuns: matchRows.runs(MATCH_MARK_HEIGHT),
    matchRow: (index) => {
      const at = matchAt[index] ?? NaN;
      return Number.isNaN(at) ? undefined : Math.floor(at);
    },
    markAt: (x, y) => {
      // Most moves are over the text, far from the strip: they cost this one comparison.
      if (!(x >= 0 && x < STRIP_WIDTH)) return undefined;
      const heading = headingRows.nearest(y);
      const match = matchRows.nearest(y);
      const headingMark = heading && headingMarks[heading.index];
      if (!match) return headingMark && { kind: "heading", mark: headingMark };
      if (!heading || !headingMark)
        return { kind: "match", index: match.index };
      // The nearer mark wins. Only in one row, where a heading's mark and a match's lie side by
      // side, does the lane decide: the left half is the headings', the right half Find's.
      const tied =
        headingMark.row === Math.floor(match.at) || heading.away === match.away;
      const headingWins = tied
        ? x < STRIP_WIDTH / 2
        : heading.away < match.away;
      return headingWins
        ? { kind: "heading", mark: headingMark }
        : { kind: "match", index: match.index };
    },
  };
}

/**
 * Places, px down a strip, by the pixel row each falls in, so that the one nearest a pointer is
 * found among the rows within reach of it, however many places there are, and the marks are drawn
 * from the rows.
 */
class RowIndex {
  /** The places of row `r` are `order[starts[r]]` to before `order[starts[r + 1]]`. */
  private readonly starts: Uint32Array;
  /** The places' indices, row by row, each row's in the order they came. */
  private readonly order: Uint32Array;

  /**
   * `at` gives each place, px down a strip `height` px high, NaN for none. Its rows run one past
   * the strip's last, where the middle of a heading mark in the last row is.
   */
  constructor(
    private readonly at: Float64Array,
    height: number,
  ) {
    const rows = Math.max(0, Math.floor(height) + 1);
    const rowOf = (place: number) =>
      place >= 0 && place < rows ? Math.floor(place) : undefined;
    // A counting sort: each row's count, then where each row starts, then the places in it.
    const starts = new Uint32Array(rows + 1);
    for (const place of at) {
      const row = rowOf(place);
      if (row !== undefined) starts[row + 1] = (starts[row + 1] ?? 0) + 1;
    }
    for (let row = 0; row < rows; row++)
      starts[row + 1] = (starts[row + 1] ?? 0) + (starts[row] ?? 0);
    const order = new Uint32Array(starts[rows] ?? 0);
    const filled = starts.slice(0, rows);
    at.forEach((place, index) => {
      const row = rowOf(place);
      if (row === undefined) return;
      const slot = filled[row] ?? 0;
      order[slot] = index;
      filled[row] = slot + 1;
    });
    this.starts = starts;
    this.order = order;
  }

  /**
   * The place nearest `y` px down the strip, within reach of it, the first of equals (the upper
   * one, as places come top to bottom): its index, where it is and how far from `y`, in px.
   */
  nearest(y: number): { index: number; at: number; away: number } | undefined {
    const rows = this.starts.length - 1;
    const from = Math.max(0, Math.floor(y - MARK_REACH));
    const to = Math.min(rows - 1, Math.floor(y + MARK_REACH));
    let nearest: { index: number; at: number; away: number } | undefined;
    for (
      let slot = this.starts[from] ?? 0;
      slot < (this.starts[to + 1] ?? 0);
      slot++
    ) {
      const index = this.order[slot] ?? 0;
      const at = this.at[index] ?? NaN;
      const away = Math.abs(at - y);
      if (
        away <= MARK_REACH &&
        (!nearest ||
          away < nearest.away ||
          (away === nearest.away && index < nearest.index))
      )
        nearest = { index, at, away };
    }
    return nearest;
  }

  /** The runs of rows with places whose marks, `markHeight` px from their row, touch. */
  runs(markHeight: number): MatchRun[] {
    const runs: MatchRun[] = [];
    const rows = this.starts.length - 1;
    for (let row = 0; row < rows; row++) {
      if (this.starts[row] === this.starts[row + 1]) continue;
      const last = runs.at(-1);
      if (last && row <= last.row + last.height)
        last.height = row + markHeight - last.row;
      else runs.push({ row, height: markHeight });
    }
    return runs;
  }
}

/**
 * The heading marks of `headings`, each at the row `rowOf` gives its top-level block. Only the
 * headings that fall in one row share a mark, not those in rows that touch, so a document with a
 * heading in every few rows still shows its structure, and the label names only the headings of
 * the mark under the pointer. Of the headings of one row, the longest mark, a heading of the
 * highest level, stands for all of them. `headings` come in document order, and so top to bottom.
 */
function markHeadings(
  view: HTMLElement,
  headings: readonly DocumentHeading[],
  rowOf: (block: Element) => number,
): HeadingMark[] {
  const marks: {
    row: number;
    level: HeadingLevel;
    headings: [DocumentHeading, ...DocumentHeading[]];
  }[] = [];
  for (const heading of headings) {
    const block = topLevelBlock(view, heading.element);
    if (!block) continue;
    const row = rowOf(block);
    const last = marks.at(-1);
    if (last?.row !== row)
      marks.push({ row, level: heading.level, headings: [heading] });
    else {
      last.headings.push(heading);
      if (heading.level < last.level) last.level = heading.level;
    }
  }
  return marks;
}

/**
 * The top-level block of `view` that holds `element`, the only kind of block the ruler measures:
 * they are size-contained (ADR 0003), so reading their rectangles never lays out a block the
 * browser skipped.
 */
function topLevelBlock(
  view: HTMLElement,
  element: Element,
): Element | undefined {
  let at: Element | null = element;
  while (at && at.parentElement !== view) at = at.parentElement;
  return at ?? undefined;
}

/** A top-level block's top in the scroller's content, and its height, in px. */
interface BlockBox {
  top: number;
  height: number;
}

/**
 * Measures the top-level blocks of `shown` in its scroller's content, each block once however
 * many marks it carries: many headings can share one block, as in a long list, and many matches.
 */
function blockMeasurer({
  scroller,
}: ScrolledView): (block: Element) => BlockBox {
  const contentTop = scroller.getBoundingClientRect().top - scroller.scrollTop;
  const boxes = new Map<Element, BlockBox>();
  return (block) => {
    let box = boxes.get(block);
    if (!box) {
      const { top, height } = block.getBoundingClientRect();
      box = { top: top - contentTop, height };
      boxes.set(block, box);
    }
    return box;
  };
}

/**
 * Where a match starts in its top-level block: the block, and how far through the block's text,
 * from 0 to 1. Its mark lies that far down the block, so that no rectangle inside a block is ever
 * read (ADR 0003).
 */
interface MatchPlace {
  block: Element;
  share: number;
}

/**
 * Where each count's matches are in their blocks, kept for as long as the count is: that changes
 * only with a count, so a redraw for a size or the headings reuses it.
 */
const placesByCount = new WeakMap<
  readonly FindMatch[],
  (MatchPlace | undefined)[]
>();

/** Where each of `matches` starts in its top-level block of `view`, once for each count. */
function placesOf(
  view: HTMLElement,
  matches: readonly FindMatch[],
): (MatchPlace | undefined)[] {
  let places = placesByCount.get(matches);
  if (!places) {
    places = placeMatches(view, matches);
    placesByCount.set(matches, places);
  }
  return places;
}

/**
 * Where each of `matches` starts in its top-level block of `view`, or undefined for one outside
 * the view's blocks. The matches come in document order, so the matches of one block come
 * together, and one walk through the block's text places them all.
 */
function placeMatches(
  view: HTMLElement,
  matches: readonly FindMatch[],
): (MatchPlace | undefined)[] {
  const blocks = matches.map(({ startContainer }) => {
    const holder = startContainer.parentElement;
    return holder ? topLevelBlock(view, holder) : undefined;
  });
  const places = new Array<MatchPlace | undefined>(matches.length).fill(
    undefined,
  );
  let first = 0;
  while (first < matches.length) {
    const block = blocks[first];
    let end = first + 1;
    while (end < matches.length && blocks[end] === block) end++;
    if (block) placeInBlock(block, matches, first, end, places);
    first = end;
  }
  return places;
}

/**
 * Places the matches from `first` to before `end`, which all start in `block`, in `places`, from
 * one walk through the block's text that counts each text node's length. No string is built, as
 * the block's `textContent` would build one for every block with a match on every count.
 */
function placeInBlock(
  block: Element,
  matches: readonly FindMatch[],
  first: number,
  end: number,
  places: (MatchPlace | undefined)[],
): void {
  const text = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
  const placed: MatchPlace[] = [];
  let next = first;
  /** How much of the block's text comes before the node the walk is at, and then in all. */
  let before = 0;
  for (let node = text.nextNode(); node; node = text.nextNode()) {
    let match = matches[next];
    while (next < end && match?.startContainer === node) {
      const place = { block, share: before + match.startOffset };
      places[next] = place;
      placed.push(place);
      match = matches[++next];
    }
    before += (node as Text).length;
  }
  const length = Math.max(1, before);
  for (const place of placed) place.share /= length;
}
