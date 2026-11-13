import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import find from "../../core/tests/fixtures/find.html?raw";
import notes from "../../core/tests/fixtures/notes.html?raw";
import { startCommands } from "./commands";
import { findMatches, startFind } from "./find";
import {
  answerAsTheApp,
  giveAnotherDocument,
  layOut,
  fakeTheApp,
  nextInTurn,
  nextShown,
  settle,
  tell,
} from "./test-support";
import { afterNextFrame } from "./reading-position";
import { startDocumentView } from "./view";

/** How often "lamp" is in `find.md`, whatever its case: in the heading, prose, table and code. */
const LAMPS = 9;

/**
 * What `render_document` gives, in turn: the HTML, or an error for a file that can't be read. The
 * last answer repeats, and none is no document.
 */
let documents: (string | Error)[] = [];
let stop: AbortController;

beforeEach(async () => {
  mockWindows("document-0");
  documents = [find];
  fakeTheApp({
    render_document: (args) => {
      const next = nextInTurn(documents);
      return answerAsTheApp(args, () =>
        next instanceof Error
          ? Promise.reject(next)
          : Promise.resolve(next ?? null),
      );
    },
  });
  stop = new AbortController();
  startFind(stop.signal);
  startCommands(stop.signal);
  await startDocumentView(0);
  await settle();
});

afterEach(() => {
  stop.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

function view(): HTMLElement {
  const article = document.querySelector<HTMLElement>("article.markdown-body");
  if (!article) throw new Error("no document view");
  return article;
}

function bar(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".find-bar");
  if (!found) throw new Error("no find bar");
  return found;
}

function box(): HTMLInputElement {
  const found = bar().querySelector("input");
  if (!found) throw new Error("no find box");
  return found;
}

function status(): string | null {
  return bar().querySelector('[role="status"]')?.textContent ?? null;
}

/** Presses a key where the focus is, as the reader would, and returns the event. */
function press(init: KeyboardEventInit): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    ...init,
    bubbles: true,
    cancelable: true,
  });
  (document.activeElement ?? document.body).dispatchEvent(event);
  return event;
}

function ctrlF(): KeyboardEvent {
  return press({ ctrlKey: true, key: "f", code: "KeyF" });
}

/** Opens the bar and types `text` into its box, replacing what was there. */
function type(text: string): void {
  if (bar().hidden) ctrlF();
  box().value = text;
  box().dispatchEvent(new Event("input", { bubbles: true }));
}

/** The element holding the start of the current match. */
function currentHolder(): HTMLElement | null | undefined {
  const { matches, current } = findMatches();
  return matches[current ?? -1]?.startContainer.parentElement;
}

/** The text of each match, in turn. */
function matchedTexts(): string[] {
  return findMatches().matches.map((match) => {
    const range = document.createRange();
    range.setStart(match.startContainer, match.startOffset);
    range.setEnd(match.endContainer, match.endOffset);
    return range.toString();
  });
}

/** The elements the page scrolled into view, in turn, and how. */
function spyOnScrolls(): () => [Element, unknown][] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () =>
    scrolls.mock.contexts.map((element, call) => [
      element as Element,
      scrolls.mock.calls[call]?.[0],
    ]);
}

/** Shows `html` in the window in place of `find.md`. */
async function show(html: string): Promise<void> {
  documents = [html];
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
}

const enter = { key: "Enter", code: "Enter" };
const f3 = { key: "F3", code: "F3" };
const escape = { key: "Escape", code: "Escape" };

test("Ctrl+F opens the bar with the focus in its box", () => {
  expect(bar().hidden).toBe(true);
  const event = ctrlF();
  // So WebView2's own find bar stays away.
  expect(event.defaultPrevented).toBe(true);
  expect(bar().hidden).toBe(false);
  expect(document.activeElement).toBe(box());
  expect(box().getAttribute("aria-label")).toBe("Find");
});

test("Ctrl+F while the bar is open selects the text in its box", () => {
  type("lamp");
  box().setSelectionRange(4, 4);
  ctrlF();
  expect(document.activeElement).toBe(box());
  expect([box().selectionStart, box().selectionEnd]).toEqual([0, 4]);
});

test("typing counts the matches, ignoring case", () => {
  type("lamp");
  expect(status()).toBe(`1 of ${String(LAMPS)}`);
  expect(findMatches().matches).toHaveLength(LAMPS);
  expect(new Set(matchedTexts())).toEqual(new Set(["lamp", "Lamp", "LAMP"]));
  type("LaMp");
  expect(status()).toBe(`1 of ${String(LAMPS)}`);
});

test("a match across inline formatting counts once", () => {
  type("lamp light");
  expect(status()).toBe("1 of 1");
  const [match] = findMatches().matches;
  expect(match?.startContainer).not.toBe(match?.endContainer);
  expect(matchedTexts()).toEqual(["lamp light"]);
});

test("nothing matching shows No results", () => {
  type("lantern");
  expect(status()).toBe("No results");
  expect(findMatches()).toEqual({ matches: [], current: undefined });

  type("");
  expect(status()).toBe("");
  expect(findMatches()).toEqual({ matches: [], current: undefined });
});

test("a word-final sigma matches any other sigma", async () => {
  // In lower case a sigma at the end of a word is "ς", and anywhere else "σ".
  await show(`<p data-sourcepos="1:1-1:10">ΟΔΟΣ, οδος</p>`);
  type("οδοσ");
  expect(status()).toBe("1 of 2");
  expect(matchedTexts()).toEqual(["ΟΔΟΣ", "οδος"]);
  type("ΟΔΟΣ");
  expect(status()).toBe("1 of 2");
});

test("at the top of the document the first match is current", () => {
  type("lamp");
  expect(findMatches().current).toBe(0);
  expect(status()).toBe(`1 of ${String(LAMPS)}`);
});

test("the first match after the reading position is current", () => {
  // Halfway down the table's header row, on line 11; the table's first lamp is on line 13.
  layOut(11.5);
  type("lamp");
  const { matches, current } = findMatches();
  expect(current).toBe(5);
  expect(matches[5]?.startContainer.parentElement?.localName).toBe("td");
  expect(status()).toBe(`6 of ${String(LAMPS)}`);
});

test("a match in raw HTML after the reading position is current", () => {
  // Halfway down the code block's first line. "or" is in "door" and "before" above it, and in
  // the raw HTML summary "Storing" below it, which has no line number.
  layOut(16.5);
  type("or");
  expect(status()).toBe("3 of 3");
  expect(currentHolder()?.localName).toBe("summary");
});

test("a footnote is after the reading position, though its definition is early in the file", () => {
  // Halfway down the table's first row, on line 13. "oil" is in the first paragraph, above it,
  // and in the footnote, which comes last in the document but whose definition is on line 5.
  layOut(13.5);
  type("oil");
  expect(status()).toBe("2 of 2");
  expect(currentHolder()?.closest(".footnotes")).not.toBeNull();
});

test("typing on keeps the current match", () => {
  type("la");
  for (let step = 0; step < 6; step++) press(enter);
  expect(currentHolder()?.textContent).toBe("Glass");
  // "Glass" doesn't match "lam": the first match after it, in the same row, is current.
  type("lam");
  expect(findMatches().current).toBe(6);
  expect(currentHolder()?.textContent).toBe("Clean it when the lamp is cold");
});

test("typing after No results starts again from the reading position", () => {
  layOut(11.5);
  type("lamp");
  press(enter);
  press(enter);
  expect(findMatches().current).toBe(7);
  type("lampx");
  expect(status()).toBe("No results");
  type("lamp");
  expect(findMatches().current).toBe(5);
});

test("Enter and F3 go to the next match", () => {
  type("lamp");
  press(enter);
  expect(findMatches().current).toBe(1);
  const event = press(f3);
  expect(event.defaultPrevented).toBe(true);
  expect(findMatches().current).toBe(2);
  expect(status()).toBe(`3 of ${String(LAMPS)}`);
  // F3 works with the focus in the document too.
  box().blur();
  press(f3);
  expect(findMatches().current).toBe(3);
});

test("Shift+Enter and Shift+F3 go to the previous match", () => {
  type("lamp");
  press(enter);
  press(enter);
  press(enter);
  press({ ...enter, shiftKey: true });
  expect(findMatches().current).toBe(2);
  const event = press({ ...f3, shiftKey: true });
  expect(event.defaultPrevented).toBe(true);
  expect(findMatches().current).toBe(1);
  expect(status()).toBe(`2 of ${String(LAMPS)}`);
});

test("next after the last match wraps to the first", () => {
  type("lamp");
  for (let step = 1; step < LAMPS; step++) press(enter);
  expect(status()).toBe(`${String(LAMPS)} of ${String(LAMPS)}`);
  press(enter);
  expect(findMatches().current).toBe(0);
  expect(status()).toBe(`1 of ${String(LAMPS)}`);
});

test("previous before the first match wraps to the last", () => {
  type("lamp");
  press({ ...f3, shiftKey: true });
  expect(findMatches().current).toBe(LAMPS - 1);
  expect(status()).toBe(`${String(LAMPS)} of ${String(LAMPS)}`);
});

test("F3 while the bar is closed opens it", () => {
  const event = press(f3);
  expect(event.defaultPrevented).toBe(true);
  expect(bar().hidden).toBe(false);
  expect(document.activeElement).toBe(box());
});

test("Enter that ends an IME composition doesn't step", () => {
  type("lamp");
  const event = press({ ...enter, isComposing: true });
  expect(event.defaultPrevented).toBe(false);
  expect(findMatches().current).toBe(0);
});

test.each([
  ["Ctrl", { ctrlKey: true }],
  ["Alt", { altKey: true }],
])("F3 with %s held does nothing", (_modifier, modifier) => {
  expect(press({ ...f3, ...modifier }).defaultPrevented).toBe(false);
  expect(bar().hidden).toBe(true);

  type("lamp");
  expect(press({ ...f3, ...modifier }).defaultPrevented).toBe(false);
  expect(press({ ...f3, shiftKey: true, ...modifier }).defaultPrevented).toBe(
    false,
  );
  expect(findMatches().current).toBe(0);
});

test("the current match's block is scrolled into view", () => {
  const scrolled = spyOnScrolls();
  type("lamp");
  press(enter);
  press(enter);
  press(enter);
  const holder = currentHolder();
  expect(holder?.textContent).toContain("LAMP");
  expect(scrolled().at(-1)).toEqual([holder, { block: "center" }]);
});

/** The page's `scrollBy` calls, which happy-dom doesn't lay out. */
function spyOnScrollBy() {
  return vi.spyOn(window, "scrollBy").mockImplementation(() => {});
}

/** Where `scrollBy` puts a block on `line`, with `topLine` at the top, in the middle of the view. */
function toMiddle(line: number, topLine: number): number {
  const top = (line - topLine) * 20;
  return (top + top + 20 - innerHeight) / 2;
}

test("a current match still outside the view after a frame is scrolled to its middle", async () => {
  // The blocks didn't move, as if the scroll had landed far from the match.
  layOut(60);
  spyOnScrolls();
  const scrollBy = spyOnScrollBy();
  type("lamp");
  expect(scrollBy).not.toHaveBeenCalled();
  await afterNextFrame();
  expect(scrollBy.mock.calls).toEqual([[0, toMiddle(1, 60)]]);
});

test("a later step cancels the last match's scroll after its frame", async () => {
  layOut(60);
  spyOnScrolls();
  const scrollBy = spyOnScrollBy();
  type("lamp");
  press(enter);
  await afterNextFrame();
  // Only to the second lamp, on line 3.
  expect(scrollBy.mock.calls).toEqual([[0, toMiddle(3, 60)]]);
});

test("closing the bar cancels the current match's scroll after its frame", async () => {
  layOut(60);
  spyOnScrolls();
  const scrollBy = spyOnScrollBy();
  type("lamp");
  press(escape);
  await afterNextFrame();
  expect(scrollBy).not.toHaveBeenCalled();
});

test("a current match in a closed section opens it", () => {
  const section = view().querySelector("details");
  expect(section?.open).toBe(false);
  type("spare lamp");
  expect(section?.open).toBe(true);
});

test("finding leaves the document's DOM unchanged", () => {
  const before = view().innerHTML;
  type("lamp");
  press(enter);
  press(enter);
  expect(view().innerHTML).toBe(before);
  press(escape);
  expect(view().innerHTML).toBe(before);
});

test("Esc closes the bar and clears the matches", () => {
  type("lamp");
  const event = press(escape);
  expect(event.defaultPrevented).toBe(true);
  expect(bar().hidden).toBe(true);
  expect(findMatches()).toEqual({ matches: [], current: undefined });
  // The box keeps its text for the next Find.
  expect(box().value).toBe("lamp");
  ctrlF();
  expect(findMatches().matches).toHaveLength(LAMPS);
});

test("Esc with the focus outside the box closes the bar", () => {
  type("lamp");
  box().blur();
  expect(document.activeElement).not.toBe(box());
  const event = press(escape);
  expect(event.defaultPrevented).toBe(true);
  expect(bar().hidden).toBe(true);
});

test("a document shown while the bar is open is counted again", async () => {
  documents = [notes];
  type("document");
  expect(status()).toBe("No results");

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  // "A second, short document, for the page tests that show one document after another."
  expect(status()).toBe("1 of 2");
  expect(findMatches().matches).toHaveLength(2);
});

test("a reload while the bar is open is counted again", async () => {
  documents = [notes];
  type("document");
  expect(status()).toBe("No results");

  const shown = nextShown();
  await tell("document-changed", "reload");
  await shown;
  expect(status()).toBe("1 of 2");
});

test("a reload overtaken by a reload that can't read the file is counted again", async () => {
  type("document");
  expect(status()).toBe("No results");
  documents = [notes, new Error("Can't read find.md: it is in use")];
  layOut(2.5);
  // The second reload starts while the first one anchors its render, which overtakes the first.
  let overtaken = false;
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(() => {
    if (overtaken) return;
    overtaken = true;
    void tell("document-changed", "reload");
  });

  await tell("document-changed", "reload");
  await vi.waitFor(() => {
    expect(status()).toBe("1 of 2");
  });
});

test("a new document doesn't scroll to a match", async () => {
  documents = [notes];
  type("document");
  const scrolled = spyOnScrolls();

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  await settle();
  expect(status()).toBe("1 of 2");
  expect(scrolled()).toEqual([]);
});

test("a window whose document goes away finds nothing", async () => {
  type("lamp");
  documents = [];
  // The app answers null: the window shows no document.
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(status()).toBe("No results");
  expect(findMatches().matches).toEqual([]);
});

/**
 * A document with far more text than Find indexes in one task, so its index is built over several:
 * `count` paragraphs, each with one "lamp".
 */
function longDocument(count: number): string {
  return Array.from({ length: count }, (_, at) => {
    const line = String(at + 1);
    const filler = "and some more words to make it long, ".repeat(2);
    return `<p data-sourcepos="${line}:1-${line}:99">The lamp on line ${line}, ${filler}</p>`;
  }).join("\n");
}

/** Lets every task the page queued with no delay run, a long document's indexing too. */
async function settleAll(): Promise<void> {
  for (let turn = 0; turn < 20; turn++) await settle();
}

test("a long document is counted once its text is indexed", async () => {
  await show(longDocument(2000));
  type("lamp");
  // Still indexing: neither a count nor No results yet.
  expect(status()).toBe("");
  await vi.waitFor(() => {
    expect(status()).toBe("1 of 2000");
  });
});

test("typing while a long document is indexed counts the last text and goes to its match", async () => {
  await show(longDocument(2000));
  const scrolled = spyOnScrolls();
  type("lam");
  type("lamp on line 1500,");
  await vi.waitFor(() => {
    expect(status()).toBe("1 of 1");
  });
  const holder = currentHolder();
  expect(holder?.getAttribute("data-sourcepos")).toBe("1500:1-1500:99");
  expect(scrolled().at(-1)).toEqual([holder, { block: "center" }]);
});

test("a document shown while a long one is indexed is counted instead", async () => {
  await show(longDocument(2000));
  type("document");
  await show(notes);
  expect(status()).toBe("1 of 2");
  await settleAll();
  expect(status()).toBe("1 of 2");
  expect(findMatches().matches).toHaveLength(2);
});

test("a line break or no-break space in a paragraph matches a space", async () => {
  // A line break in the file is a space on the page, and so is a no-break space.
  await show(`<p data-sourcepos="1:1-2:9">Light the
lamp&nbsp;at dusk.</p>`);
  type("the lamp at");
  expect(status()).toBe("1 of 1");
});

test("a match never runs from one block into the next", async () => {
  await show(`<p data-sourcepos="1:1-1:4">lamp</p>
<p data-sourcepos="3:1-3:5">light</p>`);
  type("lamp light");
  expect(status()).toBe("No results");
  type("lamplight");
  expect(status()).toBe("No results");
});

test("a line break between inline elements matches a space", async () => {
  // `**bold**` and `*italic*` on two lines of one paragraph, and two links on two lines of a list
  // item: the reader sees a space between them.
  await show(`<p data-sourcepos="1:1-2:12"><strong>lamp</strong>
<em>light</em></p>
<ul data-sourcepos="4:1-5:22">
<li data-sourcepos="4:1-5:22"><a href="a.md">wick</a>
<a href="b.md">glass</a></li>
</ul>`);
  type("lamp light");
  expect(status()).toBe("1 of 1");
  expect(matchedTexts()).toEqual(["lamp\nlight"]);
  type("wick glass");
  expect(status()).toBe("1 of 1");
});

test("code keeps its lines", async () => {
  await show(`<pre data-sourcepos="1:1-4:3"><code>lamp
light
</code></pre>`);
  type("lamp light");
  expect(status()).toBe("No results");
});
