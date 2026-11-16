import {
  atOrAfterReadingPosition,
  openSectionsAround,
  scrollToMiddle,
  type ScrolledView,
} from "./reading-position";
import { whenDocumentShown } from "./view";

/**
 * One match of Find in the shown document, as plain offsets in its text nodes. It is the shape a
 * `StaticRange` is built from, and the overview ruler can read the matches later.
 */
export interface FindMatch {
  startContainer: Text;
  startOffset: number;
  endContainer: Text;
  endOffset: number;
}

/** A match and where it starts in the index, to keep the current match while the reader types. */
interface IndexedMatch extends FindMatch {
  at: number;
}

/**
 * Where a recount takes the current match from: the first match from where the last current one
 * started, or the first from the reading position.
 */
type CurrentFrom = "lastCurrent" | "readingPosition";

/** A text node of the document and where its text starts in the index. */
interface IndexedNode {
  node: Text;
  start: number;
}

/**
 * The shown document's text as one case-folded string, so a match can span inline formatting,
 * with where each text node starts in it. Each character of the string is the same character of
 * the document, folded, so an offset in it maps back to a text node.
 */
interface TextIndex {
  text: string;
  nodes: IndexedNode[];
}

/** The names the stylesheet paints the matches with (`::highlight(...)` in `document.css`). */
const MATCH_HIGHLIGHT = "find-match";
const CURRENT_HIGHLIGHT = "find-current";

/**
 * Whether the page can paint the matches. WebView2 has the CSS Custom Highlight API; happy-dom has
 * none of it, so the tests read the matches the highlights are built from instead.
 */
const canPaint = "highlights" in CSS;

/**
 * `text` case-folded, with each character kept where it was, so offsets in the folded text are
 * offsets in `text`.
 */
function foldCase(text: string): string {
  // A sigma at the end of a word lowers to "ς" and any other to "σ", so they wouldn't match each
  // other. Both are one character, so making them one keeps the offsets.
  return lowerInPlace(text).replaceAll("ς", "σ");
}

/**
 * `text` in lower case, of the same length: the few characters whose lower case is longer, such
 * as İ, stay as they are.
 */
function lowerInPlace(text: string): string {
  const lower = text.toLowerCase();
  // Lower case never shortens a character, so an equal length means no character grew.
  if (lower.length === text.length) return lower;
  let folded = "";
  for (const character of text) {
    const lowerCharacter = character.toLowerCase();
    folded +=
      lowerCharacter.length === character.length ? lowerCharacter : character;
  }
  return folded;
}

/**
 * Text as Find matches it, folded and of the same length. In prose, a line break from the file is
 * a space on the page, so the reader's words match across it. Text that `keepsLines` keeps its
 * line breaks, which the box never holds, so no match runs across them: code, and the gaps between
 * blocks. Every other kind of space, a no-break space too, is a space to the reader, and each is
 * one character.
 */
function searchable(text: string, keepsLines: boolean): string {
  const folded = foldCase(text);
  return keepsLines ? folded : folded.replace(/\s/g, " ");
}

/**
 * The block elements of those a document can hold (`TAGS` in `core/src/sanitize.rs`), the `div`
 * the page wraps code blocks in among them. Every other element is inline.
 */
const BLOCK_ELEMENTS = new Set([
  "BLOCKQUOTE",
  "CAPTION",
  "DD",
  "DETAILS",
  "DIV",
  "DL",
  "DT",
  "FIGCAPTION",
  "FIGURE",
  "H1",
  "H2",
  "H3",
  "H4",
  "H5",
  "H6",
  "HR",
  "LI",
  "OL",
  "P",
  "PRE",
  "SECTION",
  "SUMMARY",
  "TABLE",
  "TBODY",
  "TD",
  "TFOOT",
  "TH",
  "THEAD",
  "TR",
  "UL",
]);

/** Whether `node` is inline content: text, or an element that isn't a block. */
function isInline(node: Node | null): boolean {
  return node !== null && !BLOCK_ELEMENTS.has(node.nodeName);
}

/**
 * Whether `text` is a gap between blocks: only line breaks and spaces, next to a block or at the
 * edge of its parent. The same text between two inline elements, such as `**bold**` and `*italic*`
 * on two lines of one paragraph, is a space the reader sees. Its neighbours tell which, rather
 * than its parent: a list item holds both blocks and inline text. Element names rather than
 * computed styles, so building the index never forces a layout.
 */
function isGapBetweenBlocks(text: Text): boolean {
  return (
    !/\S/.test(text.data) &&
    text.data.includes("\n") &&
    !(isInline(text.previousSibling) && isInline(text.nextSibling))
  );
}

/**
 * How much of the document's text the index takes in per task. Building the whole index of a 1 MB
 * test document in one task held the page up for over 100 ms; a slice of this size takes a few
 * milliseconds there (ticket 14's Results).
 */
const SLICE_CHARACTERS = 64 * 1024;

/** Builds the index of a view's text a slice at a time, walking its nodes once, in order. */
class IndexBuilder {
  private readonly parts: string[] = [];
  private readonly nodes: IndexedNode[] = [];
  private length = 0;
  /** The node the walk visits next, or null once it is done. */
  private next: Node | null;
  /** How many `<pre>` elements `next` is in: text in one is code, which keeps its lines. */
  private preDepth = 0;

  constructor(private readonly view: HTMLElement) {
    this.next = view.firstChild;
  }

  /** Takes in about one slice of text, and returns the index once the walk is done. */
  slice(): TextIndex | undefined {
    const end = this.length + SLICE_CHARACTERS;
    while (this.next && this.length < end) this.visit(this.next);
    if (this.next) return undefined;
    return { text: this.parts.join(""), nodes: this.nodes };
  }

  private visit(node: Node): void {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node as Text;
      if (text.data.length > 0) {
        this.nodes.push({ node: text, start: this.length });
        this.parts.push(
          searchable(text.data, this.preDepth > 0 || isGapBetweenBlocks(text)),
        );
        this.length += text.data.length;
      }
    } else if (node.nodeType === Node.ELEMENT_NODE && node.firstChild) {
      if (node.nodeName === "PRE") this.preDepth++;
      this.next = node.firstChild;
      return;
    }
    this.next = this.after(node);
  }

  /** The node after `node` and everything in it, or null at the end of the view. */
  private after(node: Node): Node | null {
    for (let at: Node | null = node; at && at !== this.view;) {
      if (at.nextSibling) return at.nextSibling;
      at = at.parentNode;
      if (at?.nodeName === "PRE") this.preDepth--;
    }
    return null;
  }
}

/** Every match of `query` in the index, in document order. */
function matchesIn(index: TextIndex, query: string): IndexedMatch[] {
  const matches: IndexedMatch[] = [];
  if (query === "") return matches;
  const { text, nodes } = index;
  // The matches come in order, so the node each starts in only moves forward.
  let first = 0;
  for (
    let at = text.indexOf(query);
    at !== -1;
    at = text.indexOf(query, at + query.length)
  ) {
    while ((nodes[first + 1]?.start ?? Infinity) <= at) first++;
    const end = at + query.length;
    let last = first;
    while ((nodes[last + 1]?.start ?? Infinity) < end) last++;
    const start = nodes[first];
    const finish = nodes[last];
    if (!start || !finish) break;
    matches.push({
      startContainer: start.node,
      startOffset: at - start.start,
      endContainer: finish.node,
      endOffset: end - finish.start,
      at,
    });
  }
  return matches;
}

/**
 * The first match at or after the reading position, so the reader keeps their place; at the top
 * of the document, or with no match after it, the first one. The matches are in document order,
 * so a binary search finds it with few lookups.
 */
function firstAfterReadingPosition(
  shown: ScrolledView,
  matches: readonly FindMatch[],
): number {
  const isAtOrAfter = atOrAfterReadingPosition(shown);
  let low = 0;
  let high = matches.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    const match = matches[middle];
    if (match && isAtOrAfter(match.startContainer)) high = middle;
    else low = middle + 1;
  }
  return low < matches.length ? low : 0;
}

/** The find bar, the query and its matches. */
class Find {
  readonly bar: HTMLElement;
  readonly box: HTMLInputElement;
  private readonly status: HTMLElement;
  /**
   * The view and its scroller, once Find has heard of them: before that there is no document to
   * find in.
   */
  private shown: ScrolledView | undefined;
  /** The index of the shown document, built when Find first needs it. */
  private index: TextIndex | undefined;
  /** The index being built, while it is. */
  private builder: IndexBuilder | undefined;
  /**
   * The count waiting for the index: where it takes the current match from, and whether it goes
   * to it.
   */
  private waiting: { from: CurrentFrom; go: boolean } | undefined;
  matches: IndexedMatch[] = [];
  current: number | undefined;
  /** Counts the moves to a match, so a later move overtakes the scroll of an earlier one. */
  private moves = 0;

  constructor() {
    this.bar = document.createElement("div");
    // A markdown-body, for github-markdown-css's font and its light or dark colors.
    this.bar.className = "find-bar markdown-body";
    this.bar.hidden = true;
    this.box = document.createElement("input");
    // Not type=search: Chromium empties a search box on Esc, and the box keeps its text.
    this.box.type = "text";
    this.box.setAttribute("aria-label", "Find");
    this.box.spellcheck = false;
    this.status = document.createElement("span");
    this.status.setAttribute("role", "status");
    this.bar.append(this.box, this.status);
    document.body.append(this.bar);
  }

  get isOpen(): boolean {
    return !this.bar.hidden;
  }

  open(): void {
    const wasOpen = this.isOpen;
    this.bar.hidden = false;
    this.box.focus();
    this.box.select();
    if (!wasOpen) {
      // Now rather than at the first keystroke, so the reader's typing finds it ready.
      this.startIndexing();
      this.recount("readingPosition", true);
    }
  }

  close(): void {
    this.bar.hidden = true;
    this.box.blur();
    // A move still waiting for its frame doesn't scroll once the bar is closed.
    this.moves++;
    // The index goes on being built, for the next Find, but nothing counts when it's done.
    this.waiting = undefined;
    this.show([], undefined);
  }

  /**
   * The view's content was replaced: the index is stale, and an open bar counts again, from the
   * reading position. It doesn't move: a new document starts at its top or heading, and a reload
   * has just put the view back at the reading position.
   */
  documentShown(shown: ScrolledView): void {
    this.shown = shown;
    this.index = undefined;
    this.builder = undefined;
    this.waiting = undefined;
    if (this.isOpen) {
      this.startIndexing();
      this.recount("readingPosition", false);
    }
  }

  /**
   * Builds the shown document's index, unless it is built or being built: the first slice at
   * once, which is all of a short document, and each further slice in a task of its own. A count
   * that waited for the index runs once it is built.
   */
  private startIndexing(): void {
    if (!this.shown || this.index || this.builder) return;
    const builder = new IndexBuilder(this.shown.view);
    this.builder = builder;
    const slice = () => {
      // Another document's index has replaced this one.
      if (this.builder !== builder) return;
      const index = builder.slice();
      if (!index) {
        setTimeout(slice, 0);
        return;
      }
      this.builder = undefined;
      this.index = index;
      const waiting = this.waiting;
      this.waiting = undefined;
      if (waiting) this.recount(waiting.from, waiting.go);
    };
    slice();
  }

  /**
   * Finds the box's text in the shown document, and with `go` goes to the current match. The
   * current match is the first one from where `from` says, or from the reading position when
   * there was no current match, so typing on keeps the reader at the same match. Until the index
   * is built, the count waits for it, and the bar shows none.
   */
  recount(from: CurrentFrom, go: boolean): void {
    const query = searchable(this.box.value, false);
    if (!this.shown || query === "") {
      this.waiting = undefined;
      this.show([], undefined);
      return;
    }
    this.startIndexing();
    if (!this.index) {
      // A count that waits with `go` still goes once it runs, whatever later typing asked for.
      this.waiting = { from, go: go || (this.waiting?.go ?? false) };
      this.show([], undefined);
      return;
    }
    const matches = matchesIn(this.index, query);
    const lastAt =
      from === "lastCurrent" && this.current !== undefined
        ? this.matches[this.current]?.at
        : undefined;
    let current: number | undefined;
    if (matches.length > 0) {
      if (lastAt !== undefined) {
        const next = matches.findIndex((match) => match.at >= lastAt);
        current = next === -1 ? 0 : next;
      } else {
        current = firstAfterReadingPosition(this.shown, matches);
      }
    }
    this.show(matches, current);
    if (go) void this.goToCurrent();
  }

  /** Makes the next match current, or with `-1` the previous one, wrapping at the ends. */
  step(by: 1 | -1): void {
    if (this.current === undefined) return;
    const count = this.matches.length;
    this.current = (this.current + by + count) % count;
    this.paintCurrent();
    this.showCount();
    void this.goToCurrent();
  }

  /** F3 and Shift+F3: a step while the bar is open, and otherwise Find. */
  stepOrOpen(by: 1 | -1): void {
    if (this.isOpen) this.step(by);
    else this.open();
  }

  private show(matches: IndexedMatch[], current: number | undefined): void {
    this.matches = matches;
    this.current = current;
    this.paint();
    this.showCount();
  }

  private showCount(): void {
    if (this.current !== undefined) {
      this.status.textContent = `${String(this.current + 1)} of ${String(this.matches.length)}`;
    } else {
      const counted = this.box.value !== "" && !this.waiting;
      this.status.textContent = counted ? "No results" : "";
    }
  }

  /**
   * Paints the matches with CSS Custom Highlights, from static ranges, so the document's DOM
   * never changes. Live ranges would slow every later change to the DOM, the next document too.
   */
  private paint(): void {
    if (!canPaint) return;
    if (this.matches.length === 0) {
      CSS.highlights.delete(MATCH_HIGHLIGHT);
    } else {
      const all = new Highlight();
      for (const match of this.matches) all.add(new StaticRange(match));
      CSS.highlights.set(MATCH_HIGHLIGHT, all);
    }
    this.paintCurrent();
  }

  private paintCurrent(): void {
    if (!canPaint) return;
    const match =
      this.current === undefined ? undefined : this.matches[this.current];
    if (!match) {
      CSS.highlights.delete(CURRENT_HIGHLIGHT);
      return;
    }
    const current = new Highlight(new StaticRange(match));
    // Over the other matches' highlight, which it is one of.
    current.priority = 1;
    CSS.highlights.set(CURRENT_HIGHLIGHT, current);
  }

  /**
   * Scrolls the current match to the middle of the view, opening the sections it is in. A later
   * move, or closing the bar, cancels the scroll's second step.
   */
  private async goToCurrent(): Promise<void> {
    const match =
      this.current === undefined ? undefined : this.matches[this.current];
    // A match comes from a shown view's index, so there is a scroller whenever there is a match.
    if (!match || !this.shown) return;
    const move = ++this.moves;
    openSectionsAround(match.startContainer);
    await scrollToMiddle(this.shown.scroller, match, () => move === this.moves);
  }
}

/** Find, while the page has it: the tests start it anew for each test. */
let find: Find | undefined;

/** Find: shows the bar with the focus in its box, its text selected so typing replaces it. */
export function openFind(): void {
  find?.open();
}

/**
 * F3: the next match, wrapping at the end, or Find while the bar is closed. The command always
 * takes the key, so WebView2's own find never shows.
 */
export function findNext(): void {
  find?.stepOrOpen(1);
}

/** Shift+F3: the previous match, wrapping at the start, or Find while the bar is closed. */
export function findPrevious(): void {
  find?.stepOrOpen(-1);
}

/** The matches of the box's text in the shown document, in order, and which one is current. */
export function findMatches(): {
  matches: readonly FindMatch[];
  current: number | undefined;
} {
  return { matches: find?.matches ?? [], current: find?.current };
}

/**
 * Starts Find (ticket 14): the find bar, hidden until Find opens it, which looks for the box's
 * text in the shown document only. Enter goes to the next match and Shift+Enter to the previous
 * one (F3 and Shift+F3 are commands, in `commands.ts`), and Esc closes the bar. The signal removes
 * its listeners again, for the tests.
 */
export function startFind(signal?: AbortSignal): void {
  const started = new Find();
  find = started;
  const { box } = started;

  box.addEventListener(
    "input",
    () => {
      started.recount("lastCurrent", true);
    },
    { signal },
  );
  box.addEventListener(
    "keydown",
    (event) => {
      // Enter also ends an IME's composition, which isn't a step.
      if (event.key !== "Enter" || event.isComposing) return;
      event.preventDefault();
      started.step(event.shiftKey ? -1 : 1);
    },
    { signal },
  );
  // Esc closes the open bar wherever the focus is. It isn't a command: it does something only
  // while the bar is open, and otherwise stays with the page. An Esc that cancels an IME's
  // composition stays with the IME.
  addEventListener(
    "keydown",
    (event) => {
      if (event.key !== "Escape" || !started.isOpen || event.isComposing)
        return;
      if (event.ctrlKey || event.altKey || event.metaKey) return;
      event.preventDefault();
      started.close();
    },
    { signal },
  );

  const stopListening = whenDocumentShown(({ view }) => {
    started.documentShown(view);
  });
  signal?.addEventListener("abort", () => {
    stopListening();
    started.close();
    if (find === started) find = undefined;
  });
}
