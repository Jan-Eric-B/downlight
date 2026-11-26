// The overview ruler (overview ruler spec, ticket 01): a strip just left of the scroll bar with a
// mark for each of the shown document's headings, h1 to h4, level with the scroll bar's thumb
// while the heading's block is in view. Marks are page elements rather than a canvas, so the
// stylesheet colors them, and forced colors recolor them, with no redraw (spec, "Marks as page
// elements").
import {
  documentHeadings,
  whenHeadingsChange,
  type DocumentHeading,
  type HeadingLevel,
} from "./headings";
import { afterNextFrame, type ScrolledView } from "./reading-position";
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

/** Each heading's mark at its top-level block's top in `scroller`'s content. */
function measureHeadings(
  { view, scroller }: ScrolledView,
  headings: readonly DocumentHeading[],
): HeadingMark[] {
  const contentTop = scroller.getBoundingClientRect().top - scroller.scrollTop;
  /** Many headings can share one block, as in a long list; each block is measured once. */
  const blockTops = new Map<Element, number>();
  const marks: HeadingMark[] = [];
  for (const { level, element } of headings) {
    const block = topLevelBlock(view, element);
    if (!block) continue;
    let top = blockTops.get(block);
    if (top === undefined) {
      top = block.getBoundingClientRect().top - contentTop;
      blockTops.set(block, top);
    }
    marks.push({ level, top });
  }
  return marks;
}

function markElement({ level, row }: PlacedMark): HTMLElement {
  const mark = document.createElement("div");
  mark.className = "heading-mark";
  mark.dataset.level = String(level);
  mark.style.top = `${String(row)}px`;
  return mark;
}

/**
 * Starts the overview ruler, after Find: its strip, hidden until a document that scrolls is shown,
 * and redrawn, at most once per frame, whenever the document, its headings or the size of the
 * scroller or anything in it changes. The signal stops it, for the tests.
 */
export function startOverviewRuler(signal?: AbortSignal): void {
  const strip = document.createElement("div");
  // A markdown-body, for github-markdown-css's light or dark colors, which its marks take.
  strip.className = "overview-ruler markdown-body";
  // Everything on it is reachable otherwise: through the document's headings themselves.
  strip.setAttribute("aria-hidden", "true");
  strip.hidden = true;

  /** The view and its scroller, once the ruler has heard of them. */
  let shown: ScrolledView | undefined;
  /** Whether the wait note shows, for a document that isn't in view yet. */
  let waiting = false;
  let stopped = false;
  /** The frame a redraw waits for, once for any number of asks before it, or undefined. */
  let awaited: Promise<void> | undefined;
  let sizes: ResizeObserver | undefined;

  /**
   * Draws the strip as the page is now laid out, or hides it. Returns false when it keeps the
   * marks it has, as during a reload, for a redraw that waits to come once the list does.
   */
  const redraw = (): boolean => {
    if (!shown || !showsDocument() || waiting) {
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
    // stay until then, as the old document did until the new render replaced it.
    if (!list.describesView()) return false;
    // Every read before any write, so the redraw lays the page out once.
    const marks = measureHeadings(shown, list.headings);
    const scrollBar = scroller.offsetWidth - scroller.clientWidth;
    strip.style.right = `${String(scrollBar)}px`;
    strip.style.height = `${String(height)}px`;
    strip.replaceChildren(
      ...placeHeadingMarks(marks, scrollHeight, height).map(markElement),
    );
    strip.hidden = false;
    return true;
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
    }
    redrawSoon();
  });
  const stopHearingHeadings = whenHeadingsChange(redrawSoon);
  const stopHearingWaits = whenWaitNoteChanges((shows) => {
    waiting = shows;
    redrawSoon();
  });
  signal?.addEventListener("abort", () => {
    stopped = true;
    stopHearingShown();
    stopHearingHeadings();
    stopHearingWaits();
    sizes?.disconnect();
    strip.remove();
  });
}
