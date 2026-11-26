// The overview ruler (overview ruler spec, ticket 01): a strip just left of the scroll bar with a
// mark for each of the shown document's headings, h1 to h4, and for each of Find's matches and its
// current match (ticket 03), level with the scroll bar's thumb while the mark's block is in view.
// Marks are page elements rather than a canvas, so the stylesheet colors them, and forced colors
// recolor them, with no redraw (spec, "Marks as page elements").
import {
  findMatches,
  goToMatch,
  whenMatchesChange,
  type FindMatch,
  type FoundMatches,
} from "./find";
import {
  documentHeadings,
  whenHeadingsChange,
  type DocumentHeading,
  type HeadingLevel,
} from "./headings";
import { afterNextFrame, type ScrolledView } from "./reading-position";
import { whenSettingChanges } from "./settings";
import { showsDocument, whenDocumentShown } from "./view";
import { whenWaitNoteChanges } from "./wait-note";

/** A heading's mark before placing: its level and its block's top in the scroller's content. */
interface HeadingMark {
  level: HeadingLevel;
  top: number;
}

/** A mark as the strip draws it: the pixel row it starts on, and the level its length is for. */
interface PlacedMark {
  level: HeadingLevel;
  row: number;
}

/**
 * Places `marks` in a strip `height` px high over a scroller `scrollHeight` px high, as the
 * scroll bar scales its thumb, one mark per pixel row: of the marks that fall in one row, the
 * longest, a heading of the highest level, stands for all of them. `marks` come in document
 * order, and so top to bottom.
 */
function placeHeadingMarks(
  marks: readonly HeadingMark[],
  scrollHeight: number,
  height: number,
): PlacedMark[] {
  const placed: PlacedMark[] = [];
  for (const { level, top } of marks) {
    const row = Math.floor((top * height) / scrollHeight);
    const last = placed.at(-1);
    if (last?.row !== row) placed.push({ level, row });
    else if (level < last.level) last.level = level;
  }
  return placed;
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

/** Each heading's mark at its top-level block's top. */
function measureHeadings(
  view: HTMLElement,
  headings: readonly DocumentHeading[],
  boxOf: (block: Element) => BlockBox,
): HeadingMark[] {
  const marks: HeadingMark[] = [];
  for (const { level, element } of headings) {
    const block = topLevelBlock(view, element);
    if (block) marks.push({ level, top: boxOf(block).top });
  }
  return marks;
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

/** The strip's width, in px, which the script sets `--overview-ruler-width` to for the stylesheet. */
const STRIP_WIDTH = 14;

/** How high a match's mark is, in px, from its row down. */
const MATCH_MARK_HEIGHT = 2;

/** How far from a mark, in px, a click still acts on it: a mark is only 2 or 3 px high. */
const MARK_REACH = 4;

/** Pixel rows of the strip that one mark covers: the row it starts on, and how many, in px. */
interface MarkRun {
  row: number;
  height: number;
}

/**
 * Find's matches by the pixel row of the strip they fall in, as a count lays them out: a click
 * looks only at the rows within reach of it, and the marks are drawn from the rows, one element for
 * each run of rows whose marks touch.
 */
class MatchRows {
  /** The matches of row `r` are `order[starts[r]]` to before `order[starts[r + 1]]`. */
  private readonly starts: Uint32Array;
  /** The matches' indices, row by row, each row's in document order. */
  private readonly order: Uint32Array;

  /** `at` gives each match's place, px down a strip `height` px high, NaN for none. */
  constructor(
    private readonly at: Float64Array,
    height: number,
  ) {
    const rows = Math.max(0, Math.floor(height));
    const rowOf = (place: number) =>
      place >= 0 && place < rows ? Math.floor(place) : undefined;
    // A counting sort: each row's count, then where each row starts, then the matches in place.
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

  /** The match nearest `y` px down the strip within reach of its mark, the first of equals. */
  nearest(y: number): number | undefined {
    const rows = this.starts.length - 1;
    const from = Math.max(0, Math.floor(y - MARK_REACH));
    const to = Math.min(rows - 1, Math.floor(y + MARK_REACH));
    let nearest: number | undefined;
    let distance = Infinity;
    for (
      let slot = this.starts[from] ?? 0;
      slot < (this.starts[to + 1] ?? 0);
      slot++
    ) {
      const index = this.order[slot] ?? 0;
      const away = Math.abs((this.at[index] ?? NaN) - y);
      if (away < distance || (away === distance && index < (nearest ?? 0))) {
        nearest = index;
        distance = away;
      }
    }
    return distance <= MARK_REACH ? nearest : undefined;
  }

  /** The runs of marked rows whose marks touch, and so look as one mark. */
  runs(): MarkRun[] {
    const runs: MarkRun[] = [];
    const rows = this.starts.length - 1;
    for (let row = 0; row < rows; row++) {
      if (this.starts[row] === this.starts[row + 1]) continue;
      const last = runs.at(-1);
      if (last && row <= last.row + last.height)
        last.height = row + MATCH_MARK_HEIGHT - last.row;
      else runs.push({ row, height: MATCH_MARK_HEIGHT });
    }
    return runs;
  }
}

function markElement(className: string, row: number): HTMLElement {
  const mark = document.createElement("div");
  mark.className = className;
  mark.style.top = `${String(row)}px`;
  return mark;
}

function headingMarkElement({ level, row }: PlacedMark): HTMLElement {
  const mark = markElement("heading-mark", row);
  mark.dataset.level = String(level);
  return mark;
}

/** The mark of a run of matches' rows, however many matches fall in them. */
function matchMarkElement({ row, height }: MarkRun): HTMLElement {
  const mark = markElement("match-mark", row);
  mark.style.height = `${String(height)}px`;
  return mark;
}

/** Find's marks as last drawn, for a step to move the current match's and a click to act on. */
interface DrawnMatches {
  /** Find's matches they were drawn for, the same array until Find counts again. */
  matches: readonly FindMatch[];
  /** Where each match is, px down the strip, NaN for one outside the view's blocks. */
  at: Float64Array;
  rows: MatchRows;
  /** The scroller's scroll bar's width, which the strip lies left of. */
  scrollBar: number;
}

/**
 * Starts the overview ruler, after Find and `startSettings`: its strip, hidden while the settings
 * have it off and until a document that scrolls is shown, and redrawn, at most once per frame,
 * whenever the document, its headings, Find's matches, or the size of the scroller or anything in
 * it changes; a step of Find only moves the current match's mark. A click on a match's mark makes
 * that match current through Find. The signal stops it, for the tests.
 */
export function startOverviewRuler(signal?: AbortSignal): void {
  const strip = document.createElement("div");
  // A markdown-body, for github-markdown-css's light or dark colors, which its marks take.
  strip.className = "overview-ruler markdown-body";
  // Everything on it is reachable otherwise: through the document's headings themselves.
  strip.setAttribute("aria-hidden", "true");
  strip.hidden = true;
  // For the stylesheet's marks, whose widths are the strip's or parts of it, and for a click,
  // which this script places by the strip's width: one value for both.
  strip.style.setProperty("--overview-ruler-width", `${String(STRIP_WIDTH)}px`);
  /** The current match's mark, kept from draw to draw, so that a step only moves it. */
  const currentMark = markElement("current-match-mark", 0);

  /** The view and its scroller, once the ruler has heard of them. */
  let shown: ScrolledView | undefined;
  /** Whether the reader has the ruler on in the settings (ticket 02), set as the ruler starts. */
  let switchedOn = false;
  /** Whether the wait note shows, for a document that isn't in view yet. */
  let waiting = false;
  let stopped = false;
  /** The frame a redraw waits for, once for any number of asks before it, or undefined. */
  let awaited: Promise<void> | undefined;
  let sizes: ResizeObserver | undefined;
  /** Where Find's matches of its last count are in their blocks, once a redraw has needed it. */
  let places:
    | { matches: readonly FindMatch[]; of: (MatchPlace | undefined)[] }
    | undefined;
  /** Find's marks as the strip shows them, once it has. */
  let drawn: DrawnMatches | undefined;
  /**
   * The match a press on its mark was on, for the click after it, which acts only then: the
   * webview gives a click whose press was elsewhere, as at the end of a selection dragged from
   * the text, to the element that holds both, so it reaches the scroller from wherever it began.
   */
  let armed: number | undefined;

  /**
   * Puts the current match's mark at its match's row, over the matches' marks, or takes it away
   * when there is none: a step changes nothing else on the strip.
   */
  const placeCurrentMark = (current: number | undefined) => {
    const at =
      current === undefined || !drawn ? NaN : (drawn.at[current] ?? NaN);
    if (Number.isNaN(at)) {
      currentMark.remove();
      return;
    }
    currentMark.style.top = `${String(Math.floor(at))}px`;
    if (currentMark.parentElement !== strip) strip.append(currentMark);
  };

  /**
   * Draws the strip as the page is now laid out, or hides it. Returns false when it keeps the
   * marks it has, as during a reload, for a redraw that waits to come once the list does.
   */
  const redraw = (): boolean => {
    if (!switchedOn || !shown || !showsDocument() || waiting) {
      strip.hidden = true;
      return true;
    }
    const { scroller } = shown;
    const height = scroller.clientHeight;
    const scrollHeight = scroller.scrollHeight;
    if (scrollHeight <= height) {
      strip.hidden = true;
      return true;
    }
    const list = documentHeadings();
    // A reload's new render is in, and its list comes once the view has landed: the old marks
    // stay until then, as the old document did until the new render replaced it. Find counts the
    // new render as the view is given it, before the list is rebuilt.
    if (!list.describesView()) return false;
    // Every read before any write, so the redraw lays the page out once.
    const boxOf = blockMeasurer(shown);
    const headingMarks = measureHeadings(shown.view, list.headings, boxOf);
    const found = findMatches();
    const at = matchesAt(shown.view, found, boxOf, height / scrollHeight);
    const scrollBar = scroller.offsetWidth - scroller.clientWidth;
    const rows = new MatchRows(at, height);
    drawn = { matches: found.matches, at, rows, scrollBar };
    strip.style.right = `${String(scrollBar)}px`;
    strip.style.height = `${String(height)}px`;
    strip.replaceChildren(
      ...placeHeadingMarks(headingMarks, scrollHeight, height).map(
        headingMarkElement,
      ),
      ...rows.runs().map(matchMarkElement),
    );
    placeCurrentMark(found.current);
    strip.hidden = false;
    return true;
  };

  /**
   * Where each of Find's matches is, px down the strip, as the view is laid out now, at `scale` px
   * of the strip to each px of the scroller's content. Where they are in their blocks changes only
   * with a count, so a redraw for a size or the headings reuses it.
   */
  const matchesAt = (
    view: HTMLElement,
    { matches }: FoundMatches,
    boxOf: (block: Element) => BlockBox,
    scale: number,
  ): Float64Array => {
    if (places?.matches !== matches)
      places = { matches, of: placeMatches(view, matches) };
    const at = new Float64Array(matches.length).fill(NaN);
    places.of.forEach((place, index) => {
      if (!place) return;
      const { top, height } = boxOf(place.block);
      at[index] = (top + place.share * height) * scale;
    });
    return at;
  };

  /**
   * The match whose mark a press or click at the pointer acts on: the nearest one within reach,
   * in the strip's right half, which is Find's lane (the left half is the headings'). Undefined
   * when there is none, also when the pointer is over the "File deleted or moved" bar or the wait
   * note, which lie over the strip, or Find has counted again since the strip was drawn. The strip
   * takes no pointer, so the wheel over it still scrolls the document: the press and the click
   * reach the scroller's content under it, and only the scroller's box is read.
   */
  const matchUnder = (event: MouseEvent): number | undefined => {
    if (strip.hidden || !shown || !drawn || event.button !== 0) return;
    // Something in the document has taken the press or the click for itself.
    if (event.defaultPrevented) return;
    const { view, scroller } = shown;
    const { target } = event;
    if (!(target instanceof Node)) return;
    if (target !== scroller && !view.contains(target)) return;
    if (findMatches().matches !== drawn.matches) return;
    const { top, right } = scroller.getBoundingClientRect();
    const stripRight = right - drawn.scrollBar;
    const x = event.clientX;
    if (x < stripRight - STRIP_WIDTH / 2 || x >= stripRight) return;
    return drawn.rows.nearest(event.clientY - top);
  };

  /** Redraws once the next frame has laid the page out, once for any number of asks before. */
  const redrawSoon = () => {
    if (awaited) return;
    const frame = afterNextFrame();
    awaited = frame;
    void frame.then(() => {
      if (awaited !== frame || stopped) return;
      awaited = undefined;
      redraw();
    });
  };

  /**
   * Redraws in the frame whose layout changed a size, where the layout is clean, so the marks
   * move in that frame's paint rather than two frames later. It stands for a redraw waiting for
   * the next frame, unless it kept the old marks: that one comes after the list it waits for.
   */
  const redrawNow = () => {
    if (!stopped && redraw()) awaited = undefined;
  };

  const stopHearingShown = whenDocumentShown(({ view }) => {
    if (!shown) {
      shown = view;
      sizes = new ResizeObserver(redrawNow);
      // The view's size changes as blocks are laid out the first time, with zoom and Full width,
      // the scroller's as the window's does, and the "File deleted or moved" bar's as it shows or
      // hides, which moves the view down or up without resizing it.
      sizes.observe(view.scroller);
      for (const part of view.scroller.children) sizes.observe(part);
      // In the scroller, after what it holds, so that the bar and the wait note, which lie over
      // the document, lie over the strip too, and the page's bubbles and bars over the scroller
      // lie over it as well. Put in after the parts are observed, so the strip, whose height a
      // redraw sets, never sets off a redraw itself.
      view.scroller.append(strip);
      // A press on a mark keeps the focus where it is, in the find box too, and starts no
      // selection; the click after it then acts.
      view.scroller.addEventListener(
        "mousedown",
        (event) => {
          armed = matchUnder(event);
          if (armed !== undefined) event.preventDefault();
        },
        { signal },
      );
      view.scroller.addEventListener(
        "click",
        (event) => {
          const pressed = armed;
          armed = undefined;
          if (pressed === undefined) return;
          const match = matchUnder(event);
          if (match === undefined) return;
          event.preventDefault();
          goToMatch(match);
        },
        { signal },
      );
    }
    redrawSoon();
  });
  // A step keeps the matches and their places and moves only the current match's mark, with one
  // write and no read; a count, and any redraw still to come, redraws the whole strip.
  const stopHearingMatches = whenMatchesChange(({ matches, current }) => {
    if (drawn?.matches === matches && !awaited && !strip.hidden)
      placeCurrentMark(current);
    else redrawSoon();
  });
  // Hears only snapshots that change the switch. The strip takes no layout, so switching it moves
  // nothing and lands nowhere (spec, "Placement"): a landing would snap a view part way through a
  // block to its top.
  const stopHearingSwitch = whenSettingChanges("overviewRuler", (on) => {
    switchedOn = on;
    redrawNow();
  });
  const stopHearingHeadings = whenHeadingsChange(redrawSoon);
  const stopHearingWaits = whenWaitNoteChanges((shows) => {
    waiting = shows;
    redrawSoon();
  });
  signal?.addEventListener("abort", () => {
    stopped = true;
    stopHearingShown();
    stopHearingMatches();
    stopHearingSwitch();
    stopHearingHeadings();
    stopHearingWaits();
    sizes?.disconnect();
    strip.remove();
  });
}
