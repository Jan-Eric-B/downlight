// The selection's counts (window design ticket 10): its words and characters in a bubble at the
// bottom right while text in the document is selected. The document's own counts are the title
// bar's (title-bar.test.ts).
import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import code from "../../core/tests/fixtures/code.html?raw";
import codeInText from "../../core/tests/fixtures/code-in-text.html?raw";
import codeUnclosed from "../../core/tests/fixtures/code-unclosed.html?raw";
import counts from "../../core/tests/fixtures/counts.json";
import find from "../../core/tests/fixtures/find.html?raw";
import github from "../../core/tests/fixtures/github.html?raw";
import imagesAndLinks from "../../core/tests/fixtures/images-and-links.html?raw";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import tables from "../../core/tests/fixtures/tables.html?raw";
import { startSelectionCounts } from "./counts";
import {
  answerAsTheApp,
  fakeTheApp,
  fakeTitleBarLayout,
  giveAnotherDocument,
  nextInTurn,
  nextShown,
  saveTheFile,
  stepAsTheApp,
} from "./test-support";
import {
  DOCUMENT_VIEW_SELECTOR,
  SCROLLER_SELECTOR,
  startDocumentView,
} from "./view";

/** The HTML the window's document reads as, in turn; the last one repeats. */
let documents: string[] = [];

let stop: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  fakeTheApp({
    render_document: (args) => {
      const next = nextInTurn(documents);
      return answerAsTheApp(args, () => Promise.resolve(next ?? null));
    },
    step_history: stepAsTheApp,
  });
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.getSelection()?.removeAllRanges();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** Starts the selection's counts and shows `html` as the window's document. Returns the view. */
async function showDocument(html: string): Promise<HTMLElement> {
  documents = [html];
  startSelectionCounts(stop.signal);
  await startDocumentView(0);
  const view = document.querySelector<HTMLElement>(DOCUMENT_VIEW_SELECTOR);
  if (!view) throw new Error("The document view didn't start.");
  return view;
}

/** The bubble with the selection's counts. */
function bubble(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".selection-counts");
  if (!found) throw new Error("The selection's counts haven't started.");
  return found;
}

/** What the bubble says once the selection is counted. */
async function bubbleSays(text: string): Promise<void> {
  await vi.waitFor(() => {
    expect(bubble().hidden).toBe(false);
    expect(bubble().textContent).toBe(text);
  });
}

/** The first text node in `element` whose text has `text` in it. */
function textNodeWith(element: Element, text: string): Text {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node.textContent?.includes(text)) return node as Text;
  }
  throw new Error(`No text node has "${text}" in it.`);
}

/** Selects from `start` to `end`, each a node and an offset in it, as a drag would. */
function select(
  [startNode, startOffset]: [Node, number],
  [endNode, endOffset]: [Node, number],
): void {
  const range = document.createRange();
  range.setStart(startNode, startOffset);
  range.setEnd(endNode, endOffset);
  const selection = document.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}

/** Selects `text` in the first text node of `element` that has it. */
function selectText(element: Element, text: string): void {
  const node = textNodeWith(element, text);
  const at = node.data.indexOf(text);
  select([node, at], [node, at + text.length]);
}

/** Selects everything in `element`, as a drag from its start to its end would. */
function selectAllOf(element: Element): void {
  select([element, 0], [element, element.childNodes.length]);
}

test("a selection in the document shows its words and characters in a bubble", async () => {
  const view = await showDocument("<p>Hello wide world</p>\n");

  selectText(view, "wide world");
  await bubbleSays("2 words, 10 characters");
});

test("one word of one character is counted in the singular", async () => {
  const view = await showDocument("<p>A b</p>\n");

  selectText(view, "A");
  await bubbleSays("1 word, 1 character");
});

test("digits are grouped the English way", async () => {
  const view = await showDocument(`<p>${"word ".repeat(1499)}word</p>\n`);

  selectAllOf(view);
  await bubbleSays("1,500 words, 7,499 characters");
});

test("an accented letter or an emoji is one character, as the reader sees it", async () => {
  const view = await showDocument("<p>café 👍🏽 ok</p>\n");

  selectAllOf(view);
  await bubbleSays("2 words, 9 characters");
});

test("spaces count as characters, the line breaks between blocks and in code don't", async () => {
  const view = await showDocument(
    "<p>one two</p>\n<pre><code>a b\nc\n</code></pre>\n",
  );

  selectAllOf(view);
  await bubbleSays("5 words, 11 characters");
});

test("a line break in a paragraph's text is the space the reader sees", async () => {
  const view = await showDocument("<p>first\nsecond</p>\n");

  selectAllOf(view);
  await bubbleSays("2 words, 12 characters");
});

test("text that runs across inline markup is one word", async () => {
  const view = await showDocument(
    "<p>H<sub>2</sub>O is <strong>bold</strong>ly said</p>\n",
  );

  selectAllOf(view);
  await bubbleSays("4 words, 18 characters");
});

test("table cells count apart from each other", async () => {
  const view = await showDocument(
    "<table><tbody><tr><td>one</td><td>two</td></tr></tbody></table>\n",
  );

  selectAllOf(view);
  await bubbleSays("2 words, 6 characters");
});

test("the hidden content of a closed section is not counted", async () => {
  const view = await showDocument(
    "<details><summary>More</summary><p>Hidden words here</p></details>\n<p>Shown</p>\n",
  );

  selectAllOf(view);
  await bubbleSays("2 words, 9 characters");
});

test("a selection outside the document, as in the title bar, shows no bubble", async () => {
  await showDocument("<p>Hello</p>\n");
  const outside = document.createElement("header");
  outside.textContent = "README.md";
  document.body.prepend(outside);

  selectText(outside, "README");
  await Promise.resolve();
  expect(bubble().hidden).toBe(true);
});

test("a selection that runs beyond the document counts only its part in the document", async () => {
  const view = await showDocument("<p>Hello wide world</p>\n");
  const outside = document.createElement("header");
  outside.textContent = "README.md and more";
  document.body.prepend(outside);

  select(
    [textNodeWith(outside, "README"), 0],
    [textNodeWith(view, "wide"), 10],
  );
  await bubbleSays("2 words, 10 characters");
});

test("a selection of only the gap between blocks shows no bubble", async () => {
  const view = await showDocument("<p>one</p>\n<p>two</p>\n");

  select([textNodeWith(view, "one"), 3], [textNodeWith(view, "two"), 0]);
  await Promise.resolve();
  expect(bubble().hidden).toBe(true);
});

test("the bubble follows the selection as it changes", async () => {
  const view = await showDocument("<p>Hello wide world</p>\n");
  const text = textNodeWith(view, "Hello");

  select([text, 0], [text, 5]);
  await bubbleSays("1 word, 5 characters");
  select([text, 0], [text, 10]);
  await bubbleSays("2 words, 10 characters");
});

test("the bubble hides when the selection collapses", async () => {
  const view = await showDocument("<p>Hello wide world</p>\n");
  selectText(view, "wide");
  await bubbleSays("1 word, 4 characters");

  document.getSelection()?.collapseToEnd();
  expect(bubble().hidden).toBe(true);
});

test("the bubble hides when the document reloads", async () => {
  const view = await showDocument("<p>Hello wide world</p>\n");
  selectText(view, "wide");
  await bubbleSays("1 word, 4 characters");

  documents = ["<p>Hello changed world</p>\n"];
  const shown = nextShown();
  await saveTheFile();
  await shown;
  expect(bubble().hidden).toBe(true);
});

test("the bubble hides when the window shows another document, and when it goes back", async () => {
  const view = await showDocument("<p>Hello wide world</p>\n");
  selectText(view, "wide");
  await bubbleSays("1 word, 4 characters");

  documents = ["<p>Another document</p>\n"];
  let shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(bubble().hidden).toBe(true);

  selectText(view, "Another");
  await bubbleSays("1 word, 7 characters");
  documents = ["<p>Hello wide world</p>\n"];
  shown = nextShown();
  await stepAsTheApp({ step: "back" });
  await shown;
  expect(bubble().hidden).toBe(true);
});

test("the bubble is hidden from screen readers and is no live region", async () => {
  const view = await showDocument("<p>Hello</p>\n");
  selectText(view, "Hello");
  await bubbleSays("1 word, 5 characters");

  expect(bubble().getAttribute("aria-hidden")).toBe("true");
  expect(bubble().hasAttribute("aria-live")).toBe(false);
  expect(bubble().hasAttribute("role")).toBe(false);
});

test("a selection larger than one task takes in is counted in full", async () => {
  // Each paragraph's middle word runs across an inline element, wherever a task's slice ends.
  const view = await showDocument(
    "<p>alpha be<em>ta</em> gamma</p>\n".repeat(5000),
  );

  selectAllOf(view);
  await bubbleSays("15,000 words, 80,000 characters");
});

/** What the bubble says of the words once the selection is counted, as a number. */
async function wordsSaid(): Promise<number> {
  let words = Number.NaN;
  await vi.waitFor(() => {
    const said = /^([\d,]+) words?, /.exec(bubble().textContent);
    expect(said).not.toBeNull();
    words = Number(said?.[1]?.replaceAll(",", ""));
  });
  return words;
}

test("selecting all of a document with a closed section counts the words but those the section hides", async () => {
  const view = await showDocument(find);

  selectAllOf(view);
  // "Keep a spare lamp in the cellar." is in the closed section, which the core counts.
  expect(await wordsSaid()).toBe(counts["find.md"].words - 7);
});

test("selecting all of a document with a superscript digit counts a word fewer than the core", async () => {
  const view = await showDocument(github);

  selectAllOf(view);
  // The "²" of "200 g/m²" is a word to `unicode_words`, which counts any segment with a number in
  // it, and not to `Intl.Segmenter`, whose words are letters and digits: the two counts may differ
  // a little, as the ticket accepts.
  expect(await wordsSaid()).toBe(counts["github.md"].words - 1);
});

test.each([
  ["code.md", code],
  ["code-in-text.md", codeInText],
  ["code-unclosed.md", codeUnclosed],
  ["images-and-links.md", imagesAndLinks],
  ["notes.md", notes],
  ["readme.md", readme],
  ["tables.md", tables],
])(
  "selecting all of %s counts the words the core counts for it",
  async (name, html) => {
    const view = await showDocument(html);

    selectAllOf(view);
    expect(await wordsSaid()).toBe(counts[name as keyof typeof counts].words);
  },
);

/** A document whose text is longer than one task takes in, with "gamma" in each paragraph. */
const LONGER_THAN_A_SLICE = "<p>alpha be<em>ta</em> gamma</p>\n".repeat(5000);

test("a selection that starts in code keeps the code's spaces", async () => {
  const view = await showDocument("<pre><code>a  b\n</code></pre>\n");

  selectText(view, "a  b");
  await bubbleSays("2 words, 4 characters");
});

test("a selection that ends at the start of the next block, as a triple click makes, counts only its block", async () => {
  const view = await showDocument("<p>one two</p>\n<p>three</p>\n");
  const next = view.querySelectorAll("p")[1];
  if (!next) throw new Error("The document has no second paragraph.");

  select([textNodeWith(view, "one"), 0], [next, 0]);
  await bubbleSays("2 words, 7 characters");
});

test("one text longer than a slice is counted a slice per task, in full", async () => {
  const view = await showDocument(
    `<pre><code>${"word ".repeat(29999)}word\n</code></pre>\n`,
  );
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

  selectAllOf(view);
  vi.advanceTimersByTime(0);
  expect(bubble().hidden).toBe(true);
  vi.runAllTimers();
  expect(bubble().textContent).toBe("30,000 words, 149,999 characters");
});

test("text with no space for longer than a slice is counted in full", async () => {
  const view = await showDocument(`<p>${"word,".repeat(20000)}</p>\n`);

  selectAllOf(view);
  await bubbleSays("20,000 words, 100,000 characters");
});

test("a newer selection overtakes the count of a large one on its way", async () => {
  const view = await showDocument(LONGER_THAN_A_SLICE);
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

  selectAllOf(view);
  vi.advanceTimersByTime(0);
  // The large selection's first slice is counted, and the rest is still on its way.
  expect(bubble().hidden).toBe(true);
  selectText(view, "gamma");
  vi.runAllTimers();
  expect(bubble().hidden).toBe(false);
  expect(bubble().textContent).toBe("1 word, 5 characters");
});

test("another document stops the count of a large selection on its way", async () => {
  const view = await showDocument(LONGER_THAN_A_SLICE);
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  selectAllOf(view);
  vi.advanceTimersByTime(0);
  expect(bubble().hidden).toBe(true);

  documents = ["<p>Another document</p>\n"];
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  vi.runAllTimers();
  expect(bubble().hidden).toBe(true);
});

test("the bubble moves beside the scroll bar when one appears while it shows", async () => {
  const layOut = fakeTitleBarLayout();
  const view = await showDocument("<p>Hello wide world</p>\n");
  const scroller = document.querySelector<HTMLElement>(SCROLLER_SELECTOR);
  if (!scroller) throw new Error("The document view has no scroller.");
  const scrollBar = (width: number) => {
    Object.defineProperty(scroller, "offsetWidth", {
      value: 800,
      configurable: true,
    });
    Object.defineProperty(scroller, "clientWidth", {
      value: 800 - width,
      configurable: true,
    });
  };
  scrollBar(0);

  selectText(view, "wide");
  await bubbleSays("1 word, 4 characters");
  expect(bubble().style.right).toBe("0px");
  scrollBar(17);
  layOut();
  expect(bubble().style.right).toBe("17px");
});
