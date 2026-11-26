// The shown document's text as the reader reads it, walked a slice per task: Find's index
// (ticket 14) and the selection's counts (window design ticket 10) both read it so. Blocks are
// told by their element names rather than their computed styles, so the walk never forces a
// layout, as the core's word count tells them (`breaks_text` in `core/src/counts.rs`).

/**
 * The block elements a document can hold (`BLOCK_TAGS` in `core/src/sanitize.rs`), the `div` the
 * page wraps code blocks in among them. Every other element is inline.
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

/** Whether `node` is a block element of the document. */
function isBlock(node: Node): boolean {
  return BLOCK_ELEMENTS.has(node.nodeName);
}

/** Whether `node` is inline content: text, or an element that isn't a block. */
export function isInline(node: Node | null): boolean {
  return node !== null && !isBlock(node);
}

/**
 * How much of the document's text a walk takes in per task. All of a 1 MB test document's text in
 * one task held the page up for over 100 ms; a slice of this size takes a few milliseconds there
 * (ticket 14's Results).
 */
export const SLICE_CHARACTERS = 64 * 1024;

/** What a walk of the document's text tells its reader, in document order. */
export interface TextReader {
  /**
   * A text node's text, or the part of it the walk's range holds, never empty. A text longer than
   * what is left of a slice comes in parts, one after the other, never between the two halves of
   * a surrogate pair. `code` when it is in a `<pre>`, which shows its spaces and lines as written.
   */
  text(node: Text, text: string, code: boolean): void;
  /** Where the reader sees a line end: at a block's start and end, and at a `<br>`. */
  lineBreak?(): void;
}

export interface TextWalkOptions {
  /** The part of the view the walk reads, rather than all of it. */
  within?: Range;
  /** Whether the walk skips the text the reader doesn't see: a closed section's but its summary. */
  seenOnly?: boolean;
}

/** Whether the reader doesn't see `node`: the content of a closed section but its summary. */
function isUnseen(node: Node): boolean {
  const parent = node.parentNode;
  return (
    parent instanceof HTMLDetailsElement &&
    !parent.open &&
    node.nodeName !== "SUMMARY"
  );
}

function isHighSurrogate(code: number): boolean {
  return code >= 0xd800 && code <= 0xdbff;
}

/** Walks a view's text a slice at a time, visiting its nodes once, in order. */
export class TextWalk {
  private readonly within: Range | undefined;
  private readonly seenOnly: boolean;
  /** The node the walk visits next, or null once it is done. */
  private next: Node | null;
  /** Where in `next`, a text node, the walk goes on: past a range's start or a slice's end. */
  private offset = 0;
  /** How many `<pre>` elements `next` is in: text in one is code. */
  private preDepth = 0;
  /** The node the range ends just before, when it ends between an element's children. */
  private readonly endsBefore: Node | null = null;

  constructor(
    private readonly view: Node,
    private readonly reader: TextReader,
    { within, seenOnly = false }: TextWalkOptions = {},
  ) {
    this.within = within;
    this.seenOnly = seenOnly;
    if (!within) {
      this.next = view.firstChild;
      return;
    }
    const { startContainer, startOffset, endContainer, endOffset } = within;
    if (!(endContainer instanceof CharacterData))
      this.endsBefore = endContainer.childNodes[endOffset] ?? null;
    if (startContainer instanceof Text) {
      this.next = startContainer;
      this.offset = startOffset;
    } else if (startContainer instanceof CharacterData) {
      this.next = this.after(startContainer);
    } else {
      this.next =
        startContainer.childNodes[startOffset] ?? this.after(startContainer);
    }
    // Counted from where the walk starts: `after` may have climbed out of a `<pre>` to get there.
    this.preDepth = 0;
    for (let at = this.next?.parentNode; at && at !== view; at = at.parentNode)
      if (at.nodeName === "PRE") this.preDepth++;
  }

  /** Reads on through one slice of text, and returns whether the walk is done. */
  slice(): boolean {
    let left = SLICE_CHARACTERS;
    while (this.next && left > 0) left -= this.visit(this.next, left);
    return this.next === null;
  }

  /** Visits `node`, reading at most `left` of its text, and returns how much it read. */
  private visit(node: Node, left: number): number {
    if (node === this.endsBefore) {
      this.next = null;
      return 0;
    }
    if (this.seenOnly && isUnseen(node)) {
      const end = this.within?.endContainer;
      this.next = end && node.contains(end) ? null : this.after(node);
      return 0;
    }
    if (node instanceof Text) return this.read(node, left);
    if (node.nodeName === "BR" || isBlock(node)) this.reader.lineBreak?.();
    if (node.firstChild) {
      if (node.nodeName === "PRE") this.preDepth++;
      this.next = node.firstChild;
      return 0;
    }
    this.next = this.after(node);
    return 0;
  }

  /** Reads at most `left` of `text` from where the walk is in it, and returns how much it read. */
  private read(text: Text, left: number): number {
    const ends = text === this.within?.endContainer;
    const end = ends ? this.within.endOffset : text.length;
    const from = this.offset;
    let cut = Math.min(end, from + left);
    // A surrogate pair stays whole, for case folding and for counting characters; when only its
    // first half fits, the slice takes all of it.
    if (cut < end && isHighSurrogate(text.data.charCodeAt(cut - 1)))
      cut += cut - 1 > from ? -1 : 1;
    if (cut > from)
      this.reader.text(text, text.data.slice(from, cut), this.preDepth > 0);
    if (cut < end) {
      this.offset = cut;
    } else {
      this.offset = 0;
      this.next = ends ? null : this.after(text);
    }
    return cut - from;
  }

  /**
   * The node after `node` and everything in it, leaving each element the walk climbs out of, or
   * null once the walk leaves the range's end or the view.
   */
  private after(node: Node): Node | null {
    const end = this.within?.endContainer;
    let at = node;
    while (at !== end && at !== this.view) {
      if (at.nextSibling) return at.nextSibling;
      const parent = at.parentNode;
      if (!parent || parent === this.view) return null;
      at = parent;
      if (at.nodeName === "PRE") this.preDepth--;
      if (isBlock(at)) this.reader.lineBreak?.();
    }
    return null;
  }
}
