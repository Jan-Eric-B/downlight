// The shown document's list of headings (overview ruler ticket 01, Outline spec "One list of
// headings, shared"): the overview ruler's marks, the Outline's rows and the heading jump keys all
// come from it, so they always agree on what a heading is.
import { afterNextFrame } from "./reading-position";
import { whenDocumentShown } from "./view";

/** The heading levels the list holds: h5 and h6 would only crowd the ruler and the Outline. */
export type HeadingLevel = 1 | 2 | 3 | 4;

/** One heading of the shown document. */
export interface DocumentHeading {
  /** Its anchor id, which a link to it names and `visitHeading` goes to. */
  id: string;
  level: HeadingLevel;
  /** Its text as the reader reads it, each run of whitespace one space. */
  text: string;
  element: HTMLHeadingElement;
}

/** The shown document's headings as far as they are listed. */
export interface HeadingList {
  /** In document order. */
  /** In document order: a new array each time the list is rebuilt or grows. */
  headings: readonly DocumentHeading[];
  /**
   * Whether every heading is listed. A document's list is empty and incomplete until its first
   * slice is listed after the document's first frame, and grows a slice of its blocks per task
   * until it is complete, as Find builds its index (overview ruler ticket 05).
   */
  complete: boolean;
  /**
   * Whether the list is of the content in view. A reload's or another document's render is in the
   * view a landing before its list is rebuilt, and until then the list is of the content it
   * replaced; a list not yet built is of no content. A reader that draws from the list keeps what
   * it drew while this is false.
   */
  describesView(): boolean;
}

const LEVELS = new Map<string, HeadingLevel>([
  ["h1", 1],
  ["h2", 2],
  ["h3", 3],
  ["h4", 4],
]);

/**
 * Only a heading with an id: a raw HTML heading without one can't be named by a link, so a mark
 * or a row couldn't go to it.
 */
const LISTED_HEADINGS = [...LEVELS.keys()]
  .map((name) => `${name}[id]`)
  .join(", ");

const NONE: HeadingList = {
  headings: [],
  complete: true,
  describesView: () => false,
};

let list: HeadingList = NONE;

type HeadingsListener = (list: HeadingList) => void;

const listeners = new Set<HeadingsListener>();

function plainText(element: Element): string {
  return element.textContent.replace(/\s+/g, " ").trim();
}

/** `element` as a listed heading, or none when it isn't one. */
function headingOf(element: HTMLHeadingElement): DocumentHeading[] {
  const level = LEVELS.get(element.localName);
  return level === undefined || element.id === ""
    ? []
    : [{ id: element.id, level, text: plainText(element), element }];
}

/**
 * How many of a view's top-level blocks the list takes in per task. All of a large document's
 * headings in one task would hold the page up as Find's index would (ticket 14's Results); this
 * many blocks take a few milliseconds on the 1 MB test document (overview ruler ticket 05).
 */
const SLICE_BLOCKS = 2048;

/**
 * Lists a view's headings a slice of its top-level blocks at a time, also those in lists, quotes
 * and closed sections, in document order.
 */
class HeadingsBuilder {
  /** The top-level block the next slice starts at, or null once every block is listed. */
  private next: Element | null;
  /** The headings listed so far: a new array for each slice, as a reader tells lists apart by it. */
  headings: readonly DocumentHeading[] = [];

  constructor(view: HTMLElement) {
    this.next = view.firstElementChild;
  }

  /** Lists one slice of blocks, and returns whether every block is listed. */
  slice(): boolean {
    const found: DocumentHeading[] = [];
    for (let left = SLICE_BLOCKS; this.next && left > 0; left--) {
      const block = this.next;
      if (block instanceof HTMLHeadingElement) found.push(...headingOf(block));
      else if (block.firstElementChild)
        for (const nested of block.querySelectorAll<HTMLHeadingElement>(
          LISTED_HEADINGS,
        ))
          found.push(...headingOf(nested));
      this.next = block.nextElementSibling;
    }
    this.headings = [...this.headings, ...found];
    return this.next === null;
  }
}

/**
 * The list of the content `view` holds now, `headings` of it. The view's content is only ever
 * replaced as a whole (`whenDocumentShown`), so the list is of the content in view while the
 * view's first node is the one it had.
 */
function listOf(
  view: HTMLElement,
  headings: readonly DocumentHeading[],
  complete: boolean,
): HeadingList {
  const first = view.firstChild;
  return {
    headings,
    complete,
    describesView: () => view.firstChild === first,
  };
}

/** The shown document's headings, as far as they are listed, and whether that is all of them. */
export function documentHeadings(): HeadingList {
  return list;
}

/**
 * Runs `listener` with the list each time it changes: when it is rebuilt for a document shown,
 * returned to or reloaded, and when it grows or is complete. Returns a function that removes the
 * listener.
 */
export function whenHeadingsChange(listener: HeadingsListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** A document's list before it is built: of no content yet. */
const UNBUILT: HeadingList = {
  headings: [],
  complete: false,
  describesView: () => false,
};

function tellListeners(): void {
  for (const listener of listeners) listener(list);
}

/**
 * Keeps the list of the shown document's headings, rebuilt each time the view's content is
 * replaced: a document shown, returned to or reloaded. The list starts empty and incomplete and
 * is built after the frame that shows the document, a slice of its blocks per task, so the build
 * never holds up first paint or a frame after it (overview ruler spec, "Large documents"), as Find
 * builds its index. Started before the page parts that read it, so that their own
 * `whenDocumentShown` listeners, which run after this one, read the new list. The signal stops it,
 * for the tests.
 */
export function startHeadings(signal?: AbortSignal): void {
  /** Counts the rebuilds, so a slice is dropped if another rebuild came meanwhile. */
  let latestRebuild = 0;
  let stopped = false;
  const stopListening = whenDocumentShown(({ view, document }) => {
    const thisRebuild = ++latestRebuild;
    if (document === null) {
      list = listOf(view.view, [], true);
      tellListeners();
      return;
    }
    // Asked for before the listeners hear the empty list, so a frame they ask for then comes
    // after the first slice. Each slice after it comes in a task of its own, as Find's index's
    // do, and the listeners hear the list grow.
    const builder = new HeadingsBuilder(view.view);
    const slice = () => {
      if (stopped || thisRebuild !== latestRebuild) return;
      const complete = builder.slice();
      list = listOf(view.view, builder.headings, complete);
      tellListeners();
      if (!complete) setTimeout(slice, 0);
    };
    void afterNextFrame().then(slice);
    list = UNBUILT;
    tellListeners();
  });
  signal?.addEventListener("abort", () => {
    stopped = true;
    stopListening();
    list = NONE;
  });
}
