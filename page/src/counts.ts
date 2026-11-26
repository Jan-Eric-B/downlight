// The counts (glossary: Counts, window design ticket 10): the document's lines and words, which
// the core counts while it renders and the title bar's file name tooltip shows, and the
// selection's words and characters, which only the page can count, shown in a bubble at the
// bottom right while text in the document is selected. No status bar: a reader seldom wants the
// document's counts, and wants the selection's only in the moment.
import type { DocumentCounts } from "./app";
import { createCornerBubble } from "./corner-bubble";
import { SLICE_CHARACTERS, TextWalk, type TextReader } from "./document-text";
import {
  DOCUMENT_VIEW_SELECTOR,
  SCROLLER_SELECTOR,
  selectionInView,
  whenDocumentShown,
} from "./view";

/** Digits grouped the English way, as the UI is English. */
const NUMBERS = new Intl.NumberFormat("en-US");

/** `count` with its noun, singular for one: "1 word", "1,234 words". */
function counted(count: number, one: string, many: string): string {
  return `${NUMBERS.format(count)} ${count === 1 ? one : many}`;
}

/** The document's counts as the title bar's tooltip shows them: "1,234 lines, 5,678 words". */
export function documentCountsText({ lines, words }: DocumentCounts): string {
  return `${counted(lines, "line", "lines")}, ${counted(words, "word", "words")}`;
}

/** The selection's counts as the bubble shows them: "12 words, 68 characters". */
function selectionCountsText({ words, characters }: SelectionCounts): string {
  return `${counted(words, "word", "words")}, ${counted(characters, "character", "characters")}`;
}

interface SelectionCounts {
  words: number;
  characters: number;
}

/**
 * Words as UAX #29 parts them, the word-like segments counted, as the core's `unicode_words`
 * counts the document's; the two can differ a little, as ICU parts Chinese and Japanese by a
 * dictionary, which the ticket accepts.
 */
const WORDS = new Intl.Segmenter("en", { granularity: "word" });

/** Grapheme clusters, so that "é" written as "e" and its accent, or an emoji, is one character. */
const GRAPHEMES = new Intl.Segmenter("en", { granularity: "grapheme" });

/**
 * A stretch of text that has characters beyond ASCII, with the ASCII character on each side of
 * it. Two ASCII characters next to each other are always two grapheme clusters, apart from CR LF,
 * which the selection's text never has, so only these stretches need the segmenter, which is slow
 * for a large selection, and each of the rest is one character.
 */
const BEYOND_ASCII =
  /[\0-\x7f]?[^\0-\x7f]+(?:[\0-\x7f][^\0-\x7f]+)*[\0-\x7f]?/g;

/** The characters of `text`, line breaks not counted, as Word's "characters (with spaces)". */
function charactersIn(text: string): number {
  const shown = text.replaceAll(LINE_BREAK, "");
  let characters = shown.length;
  for (const [stretch] of shown.matchAll(BEYOND_ASCII)) {
    characters -= stretch.length;
    characters += Array.from(GRAPHEMES.segment(stretch)).length;
  }
  return characters;
}

function wordsIn(text: string): number {
  let words = 0;
  for (const { isWordLike } of WORDS.segment(text)) if (isWordLike) words++;
  return words;
}

/** What the selection's text has between two blocks, or for a line of code or a `<br>`. */
const LINE_BREAK = "\n";

/** Space as the layout collapses it into one outside code: what a paragraph's line breaks are. */
const COLLAPSIBLE_SPACE = /[ \t\n\r\f]+/g;

/** A line ending in code, as the selection's text has it. */
const CODE_LINE_ENDING = /\r\n?/g;

/**
 * Counts the text of a range in the document view a slice per task, as the reader sees it: a
 * block's edges and a `<br>` are line breaks, the spaces outside code collapse as the layout
 * collapses them, code keeps its spaces and lines, and a closed section's content isn't there. A
 * word may run on into the next slice, so each slice's text is counted up to where no word can
 * run across, and the rest goes with the next slice.
 */
class SelectionCounter implements TextReader {
  private readonly counts: SelectionCounts = { words: 0, characters: 0 };
  /** The text read and not counted yet. */
  private pending = "";
  /** Whether the text is at the start of a line, where spaces collapse to nothing. */
  private lineStart = true;
  /** Whether a collapsed space waits for the text after it. */
  private space = false;
  private readonly walk: TextWalk;

  constructor(range: Range, view: Element) {
    this.walk = new TextWalk(view, this, { within: range, seenOnly: true });
  }

  /** Counts one slice of the text, and returns the counts once all of it is counted. */
  slice(): SelectionCounts | undefined {
    if (!this.walk.slice()) {
      this.countAllButItsEnd();
      return undefined;
    }
    if (this.space) this.pending += " ";
    this.count(this.pending.length);
    return this.counts;
  }

  /**
   * Counts the text read but the part a later slice may go on with: from its last space or line
   * break, which a word never runs across. Text with neither for over a slice, as Chinese or
   * Japanese may be, is counted but its last word, which the segmenter parts it into anyway; one
   * word longer than a slice waits for its end.
   */
  private countAllButItsEnd(): void {
    const text = this.pending;
    const lastBreak = Math.max(
      text.lastIndexOf(" "),
      text.lastIndexOf(LINE_BREAK),
    );
    if (lastBreak > 0) {
      this.count(lastBreak);
      return;
    }
    if (text.length < SLICE_CHARACTERS) return;
    let words = 0;
    let lastStart = 0;
    let lastIsWord = false;
    for (const { index, isWordLike = false } of WORDS.segment(text)) {
      if (isWordLike) words++;
      lastStart = index;
      lastIsWord = isWordLike;
    }
    if (lastStart === 0) return;
    this.counts.words += words - (lastIsWord ? 1 : 0);
    this.counts.characters += charactersIn(text.slice(0, lastStart));
    this.pending = text.slice(lastStart);
  }

  /** Counts the first `length` of the text read, which no word runs out of. */
  private count(length: number): void {
    const counted = this.pending.slice(0, length);
    this.counts.words += wordsIn(counted);
    this.counts.characters += charactersIn(counted);
    this.pending = this.pending.slice(length);
  }

  text(_node: Text, text: string, code: boolean): void {
    if (code) {
      const lines = text.replace(CODE_LINE_ENDING, LINE_BREAK);
      this.pending += lines;
      this.lineStart = lines.endsWith(LINE_BREAK);
      this.space = false;
      return;
    }
    const collapsed = text.replace(COLLAPSIBLE_SPACE, " ");
    const leads = collapsed.startsWith(" ");
    const trails = collapsed.endsWith(" ");
    const words = collapsed.slice(leads ? 1 : 0, trails ? -1 : undefined);
    if (words === "") {
      if (!this.lineStart) this.space = true;
      return;
    }
    if ((leads || this.space) && !this.lineStart) this.pending += " ";
    this.pending += words;
    this.lineStart = false;
    this.space = trails;
  }

  lineBreak(): void {
    if (!this.lineStart) this.pending += LINE_BREAK;
    this.lineStart = true;
    this.space = false;
  }
}

/**
 * Shows the selection's words and characters in a bubble at the bottom right while the reader
 * has text in the document selected, never for a selection in the page's own parts, such as the
 * find bar or the title bar, and only for the part of one that runs beyond the document. It
 * follows the selection as it changes, and hides when the selection collapses, has no character,
 * or the document changes: a reload, Back, Forward or another document. The selection is counted
 * in a task of its own, once for however many changes came before it, as a drag makes many, and a
 * large one a slice per task, so selecting all of a large document never holds up the page; the
 * bubble keeps the last counts until the new ones are done. The signal stops it, for the tests.
 */
export function startSelectionCounts(signal?: AbortSignal): void {
  // No live region either: Narrator reads the selection already, and one would speak on every
  // Shift+Arrow.
  const bubble = createCornerBubble("selection-counts");
  /** Counts the selection's changes, so that the count of an older selection stops. */
  let latest = 0;
  /** The count waiting for its task, if one is. */
  let waiting: ReturnType<typeof setTimeout> | undefined;
  /** Moves the bubble when the scroll bar comes or goes while it shows, once it has shown. */
  let scrollBarWatch: ResizeObserver | undefined;

  /**
   * Puts the bubble beside the scroll bar, which the scroller has only while the document is
   * longer than the view, rather than over its bottom arrow. Measured as the bubble shows and as
   * the scroller's size changes, which a scroll bar coming or going changes, never per selection
   * change.
   */
  const placeBesideScrollBar = () => {
    const scroller = document.querySelector<HTMLElement>(SCROLLER_SELECTOR);
    if (!scroller) return;
    if (!scrollBarWatch) {
      scrollBarWatch = new ResizeObserver(() => {
        if (!bubble.hidden) placeBesideScrollBar();
      });
      scrollBarWatch.observe(scroller);
    }
    const scrollBar = scroller.offsetWidth - scroller.clientWidth;
    bubble.style.right = `${String(scrollBar)}px`;
  };

  const hide = () => {
    latest++;
    clearTimeout(waiting);
    waiting = undefined;
    bubble.hidden = true;
  };

  const show = (counts: SelectionCounts) => {
    if (counts.characters === 0) {
      bubble.hidden = true;
      return;
    }
    bubble.textContent = selectionCountsText(counts);
    if (!bubble.hidden) return;
    placeBesideScrollBar();
    bubble.hidden = false;
  };

  const count = () => {
    waiting = undefined;
    const range = selectionInView();
    const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
    if (!range || !view) {
      hide();
      return;
    }
    const thisCount = latest;
    const counter = new SelectionCounter(range, view);
    const slice = () => {
      if (thisCount !== latest) return;
      const counts = counter.slice();
      if (counts) show(counts);
      else setTimeout(slice, 0);
    };
    slice();
  };

  const selectionChanged = () => {
    // The count of the selection before, if it is on its way, stops: this one overtakes it.
    latest++;
    // At once, as it is cheap, so the bubble never shows the counts of a selection that is gone.
    if (!selectionInView()) {
      hide();
      return;
    }
    waiting ??= setTimeout(count, 0);
  };

  document.addEventListener("selectionchange", selectionChanged, { signal });
  const stopHiding = whenDocumentShown(hide);
  signal?.addEventListener("abort", () => {
    stopHiding();
    hide();
    scrollBarWatch?.disconnect();
  });
}
