import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import code from "../../core/tests/fixtures/code.html?raw";
import codeInText from "../../core/tests/fixtures/code-in-text.html?raw";
import codeUnclosed from "../../core/tests/fixtures/code-unclosed.html?raw";
import github from "../../core/tests/fixtures/github.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import tables from "../../core/tests/fixtures/tables.html?raw";
import type { ArgsOf } from "./app";
import { startCommands } from "./commands";
import { openFind, startFind } from "./find";
import { startSettings } from "./settings";
import {
  askedOfTheApp,
  failingWith,
  fakeTheAppReading,
  settle,
  type FakeApp,
} from "./test-support";
import { startDocumentView } from "./view";

// Copy as Markdown (copy-as-markdown ticket 01): the page names the pieces of the file the
// selection covers by the line numbers its blocks carry, and the app copies their Markdown. The
// selections are made in the core's own HTML of the fixture documents, so the lines are the
// core's.

let stop: AbortController;

/** Each `copy_markdown` call's arguments, in turn. */
let copied: ArgsOf<"copy_markdown">[];

/** How the fake app answers besides the document; a test sets `copy_markdown` to make it fail. */
let app: FakeApp;

beforeEach(() => {
  mockWindows("document-0");
  copied = [];
  app = {
    copy_markdown: (args) => {
      copied.push(args);
      return null;
    },
  };
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  document.getSelection()?.removeAllRanges();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** Shows the rendered document `html` in the window, or the empty window for null. */
async function show(html: string | null): Promise<void> {
  fakeTheAppReading(() => [() => Promise.resolve(html)], {
    copy_markdown: (args) => app.copy_markdown?.(args) ?? null,
  });
  await startSettings();
  startCommands(stop.signal);
  await startDocumentView(0);
  await settle();
}

/** Presses Ctrl+Shift+C on `target`, as the reader would, and returns the event. */
function pressCopyAsMarkdown(
  target: EventTarget = document.body,
  repeat = false,
): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    ctrlKey: true,
    shiftKey: true,
    key: "C",
    code: "KeyC",
    repeat,
    bubbles: true,
    cancelable: true,
  });
  target.dispatchEvent(event);
  return event;
}

/** The element of the shown document that `selector` selects and whose text holds `text`. */
function elementWith(selector: string, text: string): Element {
  const found = [
    ...document.querySelectorAll(`article.markdown-body ${selector}`),
  ].find((element) => element.textContent.includes(text));
  if (!found) throw new Error(`The document has no ${selector} with ${text}.`);
  return found;
}

/** A point of the page, as a selection's ends name it: a node and an offset in it. */
type Point = [Node, number];

/** The text node in `element` that holds `text`, and where in it `text` starts. */
function pointAt(element: Element, text: string): [Text, number] {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const at = node.textContent?.indexOf(text) ?? -1;
    if (at >= 0) return [node as Text, at];
  }
  throw new Error(`No text ${text} in the element.`);
}

/** The point right after `text` in `element`. */
function pointAfter(element: Element, text: string): Point {
  const [node, at] = pointAt(element, text);
  return [node, at + text.length];
}

/** The point at the start of `node`, where Chromium's triple-click ends a selection: offset 0. */
function startOf(node: Node | null): Point {
  if (!node) throw new Error("No node to start at.");
  return [node, 0];
}

/** The point after the last child of `node`. */
function endOf(node: Node | null): Point {
  if (!node) throw new Error("No node to end at.");
  return [node, node.childNodes.length];
}

/** The first text node in `element`. */
function firstText(element: Element): Text {
  const text = document
    .createTreeWalker(element, NodeFilter.SHOW_TEXT)
    .nextNode();
  if (!(text instanceof Text)) throw new Error("The element has no text.");
  return text;
}

/**
 * Selects from `anchor` to `focus`, where the reader's drag or click started and where it ended,
 * so a `focus` before `anchor` makes a backward selection.
 */
function selectFrom(
  [anchor, anchorOffset]: Point,
  [focus, focusOffset]: Point,
): void {
  document
    .getSelection()
    ?.setBaseAndExtent(anchor, anchorOffset, focus, focusOffset);
}

/** Selects from the start of `from`'s text `fromText` to the end of `to`'s text `toText`. */
function select(
  [from, fromText]: [Element, string],
  [to, toText]: [Element, string],
): void {
  selectFrom(pointAt(from, fromText), pointAfter(to, toText));
}

/** The pieces the page sent with its only `copy_markdown` call. */
function piecesSent(): ArgsOf<"copy_markdown">["pieces"] {
  expect(copied).toHaveLength(1);
  return copied[0]?.pieces ?? [];
}

const lines = (first: number, last: number) => ({
  piece: "lines" as const,
  first,
  last,
});

describe("Ctrl+Shift+C copies the Markdown", () => {
  test("of the whole lines from the first block a selection touches to the last", async () => {
    await show(readme);
    // From the middle of the heading on line 1 to the middle of the paragraph on line 3.
    select(
      [elementWith("h1", "Lamplighter"), "lighter"],
      [elementWith("p", "small tool"), "printable"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(1, 3)]);
  });

  test("of the showing the view shows", async () => {
    await show(readme);
    pressCopyAsMarkdown();
    expect(copied.map(({ showing }) => showing)).toEqual([0]);
  });

  test("of a list item's lines, for a selection inside it", async () => {
    await show(readme);
    select(
      [elementWith("li", "Download"), "latest"],
      [elementWith("li", "Download"), "release"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(7, 7)]);
  });

  test("of a nested list item's lines, for a selection inside it", async () => {
    await show(github);
    select(
      [elementWith("li", "Write the notes"), "Write"],
      [elementWith("li", "Write the notes"), "notes"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(37, 37)]);
  });

  // comrak ends the last item of a list at column 0 of the blank line after it, which is no part
  // of the item.
  test("of a block that ends before the line its line numbers end on", async () => {
    await show(github);
    select(
      [elementWith("li", "Publish"), "Pub"],
      [elementWith("li", "Publish"), "lish"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(38, 38)]);
  });

  test("of a quote's lines, for a selection inside it", async () => {
    await show(github);
    select(
      [elementWith("blockquote p", "Quotes keep"), "keep"],
      [elementWith("blockquote p", "Quotes keep"), "border"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(65, 65)]);
  });

  test("of the lines from a paragraph into a list", async () => {
    await show(readme);
    select(
      [elementWith("h2", "Install"), "Install"],
      [elementWith("li", "Unpack"), "folder"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 8)]);
  });

  test("of the whole file, with nothing selected", async () => {
    await show(readme);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([]);
  });

  test("of the whole file, with a selection that isn't text", async () => {
    await show(readme);
    const [text, at] = pointAt(elementWith("p", "small tool"), "small");
    document.getSelection()?.collapse(text, at);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([]);
  });

  // Ctrl+A selects the whole page, the title bar too.
  test("of the whole file, with the whole page selected", async () => {
    await show(readme);
    const range = document.createRange();
    range.selectNodeContents(document.body);
    document.getSelection()?.addRange(range);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([]);
  });

  test("of the whole file, with a selection outside the document view", async () => {
    await show(readme);
    const outside = document.createElement("p");
    outside.textContent = "Not the document";
    document.body.prepend(outside);
    select([outside, "Not"], [outside, "document"]);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([]);
  });

  test("from where the document view starts, for a selection running into it", async () => {
    await show(readme);
    const before = document.createElement("p");
    before.textContent = "Before the document";
    document.body.prepend(before);
    select([before, "Before"], [elementWith("p", "small tool"), "small"]);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(1, 3)]);
  });

  test("of the whole file, with only the space between two blocks selected", async () => {
    await show(readme);
    selectFrom(
      pointAfter(elementWith("p", "small tool"), "first."),
      startOf(elementWith("h2", "Install")),
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([]);
  });

  // Chromium's triple-click ends the selection at offset 0 of what follows the block, which
  // selects none of its characters.
  test("of a triple-clicked paragraph's lines, not the next block's", async () => {
    await show(readme);
    const paragraph = elementWith("p", "small tool");
    const next = elementWith("h2", "Install");
    selectFrom(pointAt(paragraph, "A small"), startOf(next));
    pressCopyAsMarkdown();
    selectFrom(pointAt(paragraph, "A small"), startOf(firstText(next)));
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([
      [lines(3, 3)],
      [lines(3, 3)],
    ]);
  });

  test("of a triple-clicked list item's lines, not the next item's", async () => {
    await show(github);
    const next = elementWith("li", "Tag the version");
    selectFrom(
      pointAt(elementWith("li", "Update the changelog"), "Update"),
      startOf(firstText(next)),
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(35, 35)]);
  });

  test("of a list item's own lines, not those of the nested item the selection ends at", async () => {
    await show(github);
    const nested = elementWith("li li", "Write the notes");
    const item = elementWith("li", "Tag the version");
    selectFrom(pointAt(item, "Tag"), startOf(nested.parentElement));
    pressCopyAsMarkdown();
    selectFrom(pointAt(item, "Tag"), startOf(firstText(nested)));
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([
      [lines(36, 36)],
      [lines(36, 36)],
    ]);
  });

  test("of the next block's lines only, for a selection starting at the end of a block's text", async () => {
    await show(readme);
    selectFrom(
      pointAfter(elementWith("p", "small tool"), "first."),
      pointAfter(elementWith("h2", "Install"), "Install"),
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 5)]);
  });

  // Footnotes render at the end of the view but keep their definitions' lines.
  test("of every line from a paragraph to the end of the document, for a selection into a footnote", async () => {
    await show(github);
    select(
      [elementWith("p", "matte paper"), "matte"],
      [elementWith("p", "200 g"), "heavier"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(42, 69)]);
  });

  test("of the same lines, for a selection made backward from a footnote into a paragraph", async () => {
    await show(github);
    selectFrom(
      pointAfter(elementWith("p", "tray's limit"), "first."),
      pointAt(elementWith("p", "matte paper"), "matte"),
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(42, 69)]);
  });

  test("of the lines of every block a selection running past the view's end touches, its footnotes too", async () => {
    await show(github);
    const after = document.createElement("p");
    after.textContent = "After the document";
    document.body.append(after);
    select([elementWith("p", "punctuation"), "punctuation"], [after, "After"]);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(44, 69)]);
  });

  test("of only the blocks whose line numbers are line numbers", async () => {
    await show(
      '<p data-sourcepos="1:1">Broken</p>\n<p data-sourcepos="3:1-3:4">Fine</p>',
    );
    const broken = elementWith("p", "Broken");
    select([broken, "Broken"], [elementWith("p", "Fine"), "Fine"]);
    pressCopyAsMarkdown();
    select([broken, "Bro"], [broken, "ken"]);
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([[lines(3, 3)], []]);
  });

  test("once while held down", async () => {
    await show(readme);
    pressCopyAsMarkdown();
    const repeat = pressCopyAsMarkdown(document.body, true);
    expect(copied).toHaveLength(1);
    expect(repeat.defaultPrevented).toBe(true);
  });

  test("and shows the app's notice when it can't", async () => {
    app.copy_markdown = () =>
      failingWith("The clipboard is in use by another program. Try again.");
    await show(readme);
    pressCopyAsMarkdown();
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toBe(
        "The clipboard is in use by another program. Try again.",
      );
    });
  });

  test("but not without a document", async () => {
    await show(null);
    pressCopyAsMarkdown();
    expect(askedOfTheApp()).not.toContain("copy_markdown");
  });

  // The Find box keeps its keys, as for Ctrl+V.
  test("but not in the Find box, which gets the key", async () => {
    await show(readme);
    startFind(stop.signal);
    openFind();
    const box = document.querySelector<HTMLInputElement>(".find-bar input");
    if (!box) throw new Error("Find has no box.");
    const event = pressCopyAsMarkdown(box);
    expect(askedOfTheApp()).not.toContain("copy_markdown");
    expect(event.defaultPrevented).toBe(false);
  });
});

// Copy as Markdown of part of a code block (copy-as-markdown ticket 02): a block the selection
// covers only partly is copied as its opening fence, the code lines touched and its closing fence.
describe("Ctrl+Shift+C inside one code block", () => {
  test("writes exactly the selected text, without asking the app", async () => {
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue();
    await show(code);
    const rust = elementWith("pre", "Says hello");
    select([rust, "Says hello."], [rust, "name"]);
    pressCopyAsMarkdown();
    await settle();
    expect(writeText.mock.calls).toEqual([
      ["Says hello.\nfn main() {\n    let name"],
    ]);
    expect(copied).toEqual([]);
  });

  test("writes the text selected in the front matter, a code block too", async () => {
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue();
    await show(code);
    const frontMatter = elementWith("pre", "highlighting");
    select([frontMatter, "tags"], [frontMatter, "highlighting"]);
    pressCopyAsMarkdown();
    await settle();
    expect(writeText.mock.calls).toEqual([["tags: [highlighting"]]);
    expect(copied).toEqual([]);
  });

  test("writes the code selected, for a selection ending beside the block's copy button", async () => {
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue();
    await show(codeInText);
    const pre = elementWith("pre", "npm install");
    selectFrom(
      pointAt(pre, "npm test"),
      startOf(
        pre.parentElement?.querySelector(".zeroclipboard-container") ?? null,
      ),
    );
    pressCopyAsMarkdown();
    await settle();
    expect(writeText.mock.calls).toEqual([["npm test\n"]]);
    expect(copied).toEqual([]);
  });

  // The page's elements before the view hold none of the document's characters.
  test("writes the front matter's text selected, for a selection running into it from before the view", async () => {
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue();
    await show(code);
    const before = document.createElement("p");
    before.textContent = "Before the document";
    document.body.prepend(before);
    select(
      [before, "Before"],
      [elementWith("pre", "Code blocks"), "Code blocks"],
    );
    pressCopyAsMarkdown();
    await settle();
    expect(writeText.mock.calls).toEqual([["title: Code blocks"]]);
    expect(copied).toEqual([]);
  });

  test("writes the code selected in an unclosed fence, for a selection running out of the view", async () => {
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue();
    await show(codeInText);
    const after = document.createElement("p");
    after.textContent = "After the document";
    document.body.append(after);
    select([elementWith("pre", "still code"), "still"], [after, "After"]);
    pressCopyAsMarkdown();
    await settle();
    expect(writeText.mock.calls).toEqual([["still code\n"]]);
    expect(copied).toEqual([]);
  });

  test("shows a notice when the clipboard can't be written", async () => {
    vi.spyOn(navigator.clipboard, "writeText").mockRejectedValue(
      new Error("Clipboard is busy"),
    );
    await show(code);
    const rust = elementWith("pre", "Says hello");
    select([rust, "Says hello."], [rust, "name"]);
    pressCopyAsMarkdown();
    await settle();
    expect(document.querySelector(".notice")?.textContent).toBe(
      "Can't copy the code: Error: Clipboard is busy",
    );
    expect(copied).toEqual([]);
  });
});

describe("Ctrl+Shift+C into or out of a code block", () => {
  test("copies a paragraph through the code lines touched, then the closing fence", async () => {
    await show(code);
    select(
      [elementWith("p", "Code in the languages"), "languages"],
      [elementWith("pre", "println!"), "println!"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(8, 14), lines(16, 16)]);
  });

  test("copies the opening fence and the code lines touched through the end, then the text after", async () => {
    await show(code);
    select(
      [elementWith("pre", "Says hello"), "name"],
      [elementWith("p", "A language bat"), "grammars"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(10, 10), lines(13, 35)]);
  });

  test("copies the touched lines of both blocks, each inside its own fences", async () => {
    await show(code);
    select(
      [elementWith("pre", "Says hello"), "name"],
      [elementWith("pre", "return"), "return"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(10, 10), lines(13, 20), lines(22, 22)]);
  });

  test("copies a block the selection covers whole as its lines, with nothing left out", async () => {
    await show(codeInText);
    select(
      [elementWith("p", "Some code"), "Some code"],
      [elementWith("p", "Text after"), "Text after"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(14, 20)]);
  });

  test("copies the front matter's opening and closing lines around the lines touched, for a selection running out of it", async () => {
    await show(code);
    select(
      [elementWith("pre", "highlighting"), "tags"],
      [elementWith("h1", "Code blocks"), "Code blocks"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(1, 1), lines(3, 6)]);
  });

  test("copies a fenced block in a list item inside its fences, the list item's indentation left to the app", async () => {
    await show(codeInText);
    select(
      [elementWith("p", "Install it"), "Install"],
      [elementWith("pre", "npm install"), "npm install"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 8), lines(10, 10)]);
  });

  test("copies an indented code block's touched lines with no fence lines, into it", async () => {
    await show(codeInText);
    select(
      [elementWith("p", "Some code"), "indented"],
      [elementWith("pre", "first indented"), "first"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(14, 16)]);
  });

  test("copies an indented code block's touched lines with no fence lines, out of it", async () => {
    await show(codeInText);
    select(
      [elementWith("pre", "second indented"), "second"],
      [elementWith("p", "Text after"), "Text"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(18, 20)]);
  });

  test("copies none of a code block that a selection ends at the start of", async () => {
    await show(codeInText);
    const pre = elementWith("pre", "npm install");
    const paragraph = elementWith("p", "Install it");
    selectFrom(pointAt(paragraph, "Install"), startOf(firstText(pre)));
    pressCopyAsMarkdown();
    selectFrom(pointAt(paragraph, "Install"), startOf(pre.parentElement));
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([
      [lines(5, 5)],
      [lines(5, 5)],
    ]);
  });

  test("copies none of a code block that a selection starts at the end of", async () => {
    await show(codeInText);
    const pre = elementWith("pre", "npm install");
    const next = elementWith("p", "Run it");
    // After the code's last line break, and at the end of its last line, before the line break.
    selectFrom(pointAfter(pre, "npm test\n"), pointAfter(next, "Run it."));
    pressCopyAsMarkdown();
    selectFrom(pointAfter(pre, "npm test"), pointAfter(next, "Run it."));
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([
      [lines(12, 12)],
      [lines(12, 12)],
    ]);
  });

  test("copies the touched code lines only, for a selection whose end is in a block around the code block", async () => {
    await show(codeInText);
    const pre = elementWith("pre", "npm install");
    selectFrom(pointAt(pre, "npm test"), endOf(pre.closest("ol")));
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(7, 7), lines(9, 12)]);
  });

  test("copies the touched code lines only, for a selection made backward out of a code block", async () => {
    await show(codeInText);
    selectFrom(
      pointAfter(elementWith("p", "Run it"), "Run"),
      pointAt(elementWith("pre", "npm install"), "npm test"),
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(7, 7), lines(9, 12)]);
  });

  test("copies a code block's touched lines and a table's touched rows, for a selection over both", async () => {
    await show(readme);
    select(
      [elementWith("pre", "lamp --version"), "version"],
      [elementWith("td", "Card size"), "Card"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(11, 19)]);
  });

  test("copies a paragraph through the first line of an unclosed fence, with no closing line", async () => {
    await show(codeInText);
    select(
      [elementWith("p", "An unclosed fence"), "unclosed"],
      [elementWith("pre", "never closed"), "never closed"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(22, 25)]);
  });

  // In `code-unclosed.md`, a list item's text is on line 3, its unclosed fence on line 5, its
  // code on lines 6 and 7 (blank), and the paragraph after the list, which ends the fence, on line
  // 8; the paragraph before the last fence is on line 10, the fence on line 12, and its code on
  // lines 13 and 14 (blank), the last of the file.

  test("copies a list item's text through the code lines touched of the fence the list's end leaves unclosed", async () => {
    await show(codeUnclosed);
    select(
      [elementWith("p", "Install it"), "Install"],
      [elementWith("pre", "npm install"), "npm install"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(3, 6)]);
  });

  test("copies a paragraph through the code lines touched of an unclosed fence whose last line, the file's last, is blank", async () => {
    await show(codeUnclosed);
    select(
      [elementWith("p", "at the end of the document"), "unclosed"],
      [elementWith("pre", "never closed"), "never closed"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(10, 13)]);
  });
});

// Copy as Markdown of part of a table (copy-as-markdown ticket 03): a selection within one cell
// copies the cell's Markdown, and one across cells or rows copies the table's header and delimiter
// rows and the rows it touches. In `tables.md`, the paragraph before the table is on line 3, its
// header row on line 5, its delimiter row on line 6, its rows on lines 7 to 10 (Wick, Glass, Oil,
// Base), and the paragraph after it on line 12.

const span = (line: number, first: number, last: number) => ({
  piece: "span" as const,
  line,
  first,
  last,
});

/** Shows the table's rows in the order of their first cells' `parts`, as a sort shows them. */
function showRowsInOrder(...parts: string[]): void {
  const body = document.querySelector("article.markdown-body tbody");
  if (!body) throw new Error("The document has no table body.");
  body.replaceChildren(...parts.map((part) => elementWith("tr", part)));
}

describe("Ctrl+Shift+C in a table", () => {
  test("copies the span of the one cell a selection is within", async () => {
    await show(tables);
    const note = elementWith("td", "Trim it");
    select([note, "it"], [note, "lamp"]);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([span(7, 10, 50)]);
  });

  test("copies the span of a cell with a link in it, for a selection across the link", async () => {
    await show(tables);
    const note = elementWith("td", "Clean it");
    select([note, "Clean"], [note, "cold"]);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([span(8, 10, 52)]);
  });

  test("copies the span of a header cell", async () => {
    await show(tables);
    const header = elementWith("th", "Note");
    select([header, "No"], [header, "te"]);
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([span(5, 10, 50)]);
  });

  test("copies the span of a triple-clicked cell, not its row", async () => {
    await show(tables);
    const wick = elementWith("tr", "Wick");
    selectFrom(
      pointAt(elementWith("td", "Trim it"), "Trim"),
      startOf(firstText(elementWith("td", "2 €"))),
    );
    pressCopyAsMarkdown();
    // The last cell of a row, which ends at the next row's first cell.
    selectFrom(
      pointAt(elementWith("td", "2 €"), "2"),
      startOf(firstText(elementWith("td", "Glass"))),
    );
    pressCopyAsMarkdown();
    selectFrom(
      pointAt(elementWith("td", "2 €"), "2"),
      startOf(wick.nextElementSibling),
    );
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([
      [span(7, 10, 50)],
      [span(7, 52, 60)],
      [span(7, 52, 60)],
    ]);
  });

  test("copies the span of a cell a selection starts at the end of the cell before's text", async () => {
    await show(tables);
    selectFrom(
      pointAfter(elementWith("td", "Wick"), "Wick"),
      pointAfter(elementWith("td", "Trim it"), "lamp"),
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([span(7, 10, 50)]);
  });

  test("copies a triple-clicked paragraph's lines, not the table after it", async () => {
    await show(tables);
    const paragraph = elementWith("p", "before the table");
    const table = document.querySelector("article.markdown-body table");
    selectFrom(pointAt(paragraph, "A lamp"), startOf(table));
    pressCopyAsMarkdown();
    selectFrom(
      pointAt(paragraph, "A lamp"),
      startOf(firstText(elementWith("th", "Part"))),
    );
    pressCopyAsMarkdown();
    expect(copied.map(({ pieces }) => pieces)).toEqual([
      [lines(3, 3)],
      [lines(3, 3)],
    ]);
  });

  test("copies the header, delimiter and one row, for a selection across cells of that row", async () => {
    await show(tables);
    select(
      [elementWith("td", "Wick"), "Wick"],
      [elementWith("td", "2 €"), "2"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 7)]);
  });

  test("copies the header, delimiter and the rows touched, for a selection across rows", async () => {
    await show(tables);
    select(
      [elementWith("td", "Clean it"), "cold"],
      [elementWith("td", "Oil"), "Oil"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 6), lines(8, 9)]);
  });

  test("copies the header, delimiter and the row touched, for a selection from the header row", async () => {
    await show(tables);
    select(
      [elementWith("th", "Part"), "Part"],
      [elementWith("td", "Trim it"), "Trim"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 7)]);
  });

  test("copies a paragraph through the header, delimiter and rows touched, for a selection into a table", async () => {
    await show(tables);
    select(
      [elementWith("p", "before the table"), "parts"],
      [elementWith("td", "Clean it"), "Clean"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(3, 8)]);
  });

  test("copies the header, delimiter and rows touched, then the paragraph, for a selection out of a table", async () => {
    await show(tables);
    select(
      [elementWith("td", "Fill it"), "line"],
      [elementWith("p", "after the table"), "paragraph"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 6), lines(9, 12)]);
  });

  test("copies a table the selection covers whole as its lines, with no row left out", async () => {
    await show(tables);
    select(
      [elementWith("p", "before the table"), "parts"],
      [elementWith("p", "after the table"), "paragraph"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(3, 12)]);
  });

  // tables/01 isn't built: reordering the rows in the view is what a sort does to them.
  test("copies the rows touched in the file's order, whatever order the view shows them in", async () => {
    await show(tables);
    showRowsInOrder("Base", "Wick", "Oil", "Glass");
    select(
      [elementWith("td", "Trim it"), "Trim"],
      [elementWith("td", "Fill it"), "Fill"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(5, 7), lines(9, 9)]);
  });

  test("copies a paragraph and the rows touched in the file's order, for a selection into a sorted table", async () => {
    await show(tables);
    showRowsInOrder("Base", "Wick", "Oil", "Glass");
    select(
      [elementWith("p", "before the table"), "parts"],
      [elementWith("td", "Trim it"), "Trim"],
    );
    pressCopyAsMarkdown();
    expect(piecesSent()).toEqual([lines(3, 7), lines(10, 10)]);
  });
});
