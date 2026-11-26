// The overview ruler (overview ruler spec, ticket 01): a strip just left of the scroll bar with a
// mark for each of the shown document's headings, h1 to h4, and for each of Find's matches and its
// current match (ticket 03), level with the scroll bar's thumb while the mark's block is in view.
// Marks are page elements rather than a canvas, so the stylesheet colors them, and forced colors
// recolor them, with no redraw (spec, "Marks as page elements"). Where they go, and which one a
// click or the pointer acts on, is overview-ruler-placement.ts's; this module draws them, hears
// the pointer and keeps the strip in step with the page.
import { findMatches, goToMatch, whenMatchesChange } from "./find";
import {
  documentHeadings,
  whenHeadingsChange,
  type DocumentHeading,
} from "./headings";
import {
  placeStrip,
  STRIP_WIDTH,
  type HeadingMark,
  type MarkAt,
  type MatchRun,
  type PlacedStrip,
} from "./overview-ruler-placement";
import { afterNextFrame, type ScrolledView } from "./reading-position";
import { whenSettingChanges } from "./settings";
import { showsDocument, visitHeading, whenDocumentShown } from "./view";
import { whenWaitNoteChanges } from "./wait-note";

function markElement(className: string, row: number): HTMLElement {
  const mark = document.createElement("div");
  mark.className = className;
  mark.style.top = `${String(row)}px`;
  return mark;
}

function headingMarkElement({ level, row }: HeadingMark): HTMLElement {
  const mark = markElement("heading-mark", row);
  mark.dataset.level = String(level);
  return mark;
}

/** The mark of a run of matches' rows, however many matches fall in them. */
function matchMarkElement({ row, height }: MatchRun): HTMLElement {
  const mark = markElement("match-mark", row);
  mark.style.height = `${String(height)}px`;
  return mark;
}

/** How many of a mark's headings the hover label names before it says how many more there are. */
const LABELLED_HEADINGS = 5;

/**
 * The hover label's lines for a mark's headings (spec, "Hover label"), none when none of them has
 * text. A heading without text, as one of only an image, is left out rather than given an empty
 * line: its id is made for links, not for reading.
 */
function labelLines(headings: readonly DocumentHeading[]): HTMLElement[] {
  const named = headings.filter(({ text }) => text !== "");
  const lines = named.slice(0, LABELLED_HEADINGS).map(({ text }) => text);
  const more = named.length - LABELLED_HEADINGS;
  if (more > 0) lines.push(`and ${String(more)} more`);
  return lines.map((text) => {
    const line = document.createElement("div");
    line.textContent = text;
    return line;
  });
}

/** The strip as last drawn, for a step to move the current match's mark and the pointer to act on. */
interface DrawnStrip {
  placed: PlacedStrip;
  /**
   * Where the strip's top left corner is in the window, in px. Read as it is drawn: the scroller
   * moves only as its size changes, which redraws it, so a pointer move reads no layout.
   */
  top: number;
  left: number;
}

/**
 * Starts the overview ruler, after Find and `startSettings`: its strip, hidden while the settings
 * have it off and until a document that scrolls is shown, and redrawn, at most once per frame,
 * whenever the document, its headings, Find's matches, or the size of the scroller or anything in
 * it changes; a step of Find only moves the current match's mark. A click on a heading's mark goes
 * to the heading as a heading link does (ticket 04), and one on a match's mark makes that match
 * current through Find (ticket 03); hovering a heading's mark names its headings. The signal stops
 * it, for the tests.
 */
export function startOverviewRuler(signal?: AbortSignal): void {
  const strip = document.createElement("div");
  // A markdown-body, for github-markdown-css's light or dark colors, which its marks take.
  strip.className = "overview-ruler markdown-body";
  // Everything on it is reachable otherwise: through the document's headings themselves.
  strip.setAttribute("aria-hidden", "true");
  strip.hidden = true;
  // For the stylesheet's marks, whose widths are the strip's or parts of it, and for a click or the
  // pointer, which this script places by the strip's width: one value for both.
  strip.style.setProperty("--overview-ruler-width", `${String(STRIP_WIDTH)}px`);
  /** The current match's mark, kept from draw to draw, so that a step only moves it. */
  const currentMark = markElement("current-match-mark", 0);
  /**
   * The headings of the mark under the pointer, left of the strip: in the strip, so it takes no
   * pointer, is hidden from screen readers and goes with the strip.
   */
  const label = document.createElement("div");
  label.className = "overview-ruler-label";

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
  /** The strip's marks as it shows them, once it has. */
  let drawn: DrawnStrip | undefined;
  /**
   * Whether a press on a mark came before the click, which acts only then: the webview gives a
   * click whose press was elsewhere, as at the end of a selection dragged from the text, to the
   * element that holds both, so it reaches the scroller from wherever it began.
   */
  let armed = false;
  /** What the pointer is over, so a move that stays over it writes nothing. */
  let pointedAt: MarkAt | undefined;

  /**
   * Shows what the pointer is over: the hand over any mark, and the label over a heading's.
   * Writes only when that changes, so a move along the strip costs a lookup in its rows.
   */
  const pointAt = (under: MarkAt | undefined) => {
    // Most moves are over the text, nowhere near a mark, and were before.
    if (!under && !pointedAt) return;
    const before = pointedAt;
    pointedAt = under;
    if ((before === undefined) !== (under === undefined) && shown)
      // The strip takes no pointer, so the hand is the scroller's, which the content under the
      // strip, the view's padding or the margin beside it, takes it from.
      shown.scroller.style.cursor = under ? "pointer" : "";
    const mark = under?.kind === "heading" ? under.mark : undefined;
    if (
      mark &&
      before?.kind === "heading" &&
      before.mark === mark &&
      label.isConnected
    )
      return;
    const lines = mark ? labelLines(mark.headings) : [];
    if (!mark || !drawn || lines.length === 0) {
      label.remove();
      return;
    }
    label.replaceChildren(...lines);
    // From the mark down in the strip's upper half and from it up in the lower half, so the label
    // stays in the window without measuring it.
    const { height } = drawn.placed;
    const below = mark.row < height / 2;
    label.style.top = below ? `${String(mark.row)}px` : "";
    label.style.bottom = below ? "" : `${String(height - mark.row)}px`;
    strip.append(label);
  };

  /** Takes the hand and the label away: the pointer left, or the marks moved or went. */
  const forgetPointer = () => {
    pointAt(undefined);
  };

  const hide = () => {
    forgetPointer();
    strip.hidden = true;
  };

  /**
   * Puts the current match's mark at its match's row, over the matches' marks, or takes it away
   * when there is none: a step changes nothing else on the strip.
   */
  const placeCurrentMark = (current: number | undefined) => {
    const row =
      current === undefined ? undefined : drawn?.placed.matchRow(current);
    if (row === undefined) {
      currentMark.remove();
      return;
    }
    currentMark.style.top = `${String(row)}px`;
    if (currentMark.parentElement !== strip) strip.append(currentMark);
  };

  /**
   * Draws the strip as the page is now laid out, or hides it. Returns false when it keeps the
   * marks it has, as during a reload, for a redraw that waits to come once the list does.
   */
  const redraw = (): boolean => {
    if (!switchedOn || !shown || !showsDocument() || waiting) {
      hide();
      return true;
    }
    const { scroller } = shown;
    if (scroller.scrollHeight <= scroller.clientHeight) {
      hide();
      return true;
    }
    const list = documentHeadings();
    // A reload's new render is in, and its list comes once the view has landed: the old marks
    // stay until then, as the old document did until the new render replaced it. Find counts the
    // new render as the view is given it, before the list is rebuilt.
    if (!list.describesView()) return false;
    // Every read before any write, so the redraw lays the page out once.
    const found = findMatches();
    const placed = placeStrip(shown, list.headings, found.matches);
    const scrollBar = scroller.offsetWidth - scroller.clientWidth;
    const { top, right } = scroller.getBoundingClientRect();
    // The marks move, so what the pointer was over may be gone; the next move finds it again.
    forgetPointer();
    drawn = { placed, top, left: right - scrollBar - STRIP_WIDTH };
    strip.style.right = `${String(scrollBar)}px`;
    strip.style.height = `${String(placed.height)}px`;
    strip.replaceChildren(
      ...placed.headingMarks.map(headingMarkElement),
      ...placed.matchRuns.map(matchMarkElement),
    );
    placeCurrentMark(found.current);
    strip.hidden = false;
    return true;
  };

  /**
   * The mark a press, a click or the pointer at `event` acts on, as the strip was drawn (spec,
   * "Clicks"). Undefined where there is none, also when the pointer is over the "File deleted or
   * moved" bar or the wait note, which lie over the strip, and while the marks drawn may be wrong:
   * Find has counted again, or the view's content was replaced or its list of headings rebuilt,
   * since the strip was drawn. A reload's new render is in the view before its list is, and the
   * strip keeps the old marks until then, whose headings may have moved or gone. The strip takes
   * no pointer, so the wheel over it still scrolls the document: the events reach the scroller's
   * content under it, and the strip's place is the one it was drawn at.
   */
  const markUnder = (event: MouseEvent): MarkAt | undefined => {
    if (strip.hidden || !shown || !drawn) return;
    const { placed } = drawn;
    const under = placed.markAt(
      event.clientX - drawn.left,
      event.clientY - drawn.top,
    );
    // Something in the document has taken the press or the click for itself.
    if (!under || event.defaultPrevented) return;
    const { view, scroller } = shown;
    const { target } = event;
    if (!(target instanceof Node)) return;
    if (target !== scroller && !view.contains(target)) return;
    if (findMatches().matches !== placed.matches) return;
    const list = documentHeadings();
    if (list.headings !== placed.headings || !list.describesView()) return;
    return under;
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

  /** Acts on a click on a mark: goes to its first heading, or makes its match current. */
  const actOn = (under: MarkAt) => {
    if (under.kind === "match") goToMatch(under.index);
    // As a heading link goes there: it opens closed sections and joins History.
    else void visitHeading(under.mark.headings[0].id);
  };

  /** Hears the pointer on the scroller's content under the strip, which takes no pointer itself. */
  const listenOnScroller = (scroller: HTMLElement) => {
    // A press on a mark keeps the focus where it is, in the find box too, and starts no
    // selection; the click after it then acts.
    scroller.addEventListener(
      "mousedown",
      (event) => {
        armed = event.button === 0 && markUnder(event) !== undefined;
        if (armed) event.preventDefault();
      },
      { signal },
    );
    scroller.addEventListener(
      "click",
      (event) => {
        const pressed = armed;
        armed = false;
        const under = pressed ? markUnder(event) : undefined;
        if (!under) return;
        event.preventDefault();
        actOn(under);
      },
      { signal },
    );
    scroller.addEventListener(
      "mousemove",
      (event) => {
        // A drag, of a selection or of the scroll bar's thumb, shows nothing on its way.
        pointAt(event.buttons === 0 ? markUnder(event) : undefined);
      },
      { signal },
    );
    scroller.addEventListener("mouseleave", forgetPointer, { signal });
    // The wheel over the strip scrolls the document, and the label would then lie over the text.
    scroller.addEventListener(
      "scroll",
      () => {
        label.remove();
      },
      { passive: true, signal },
    );
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
      listenOnScroller(view.scroller);
    }
    // The label names headings of the document that was in view.
    forgetPointer();
    redrawSoon();
  });
  // A step keeps the matches and their places and moves only the current match's mark, with one
  // write and no read; a count, and any redraw still to come, redraws the whole strip.
  const stopHearingMatches = whenMatchesChange(({ matches, current }) => {
    if (drawn?.placed.matches === matches && !awaited && !strip.hidden)
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
    forgetPointer();
    strip.remove();
  });
}
