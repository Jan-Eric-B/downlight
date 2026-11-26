import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import headings from "../../core/tests/fixtures/headings.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import {
  documentHeadings,
  startHeadings,
  whenHeadingsChange,
  type HeadingList,
} from "./headings";
import { afterNextFrame } from "./reading-position";
import {
  fakeTheAppReading,
  forgetForwardAsTheApp,
  giveAnotherDocument,
  nextShown,
  saveTheFile,
  settle,
  stepAsTheApp,
  type Read,
} from "./test-support";
import { startDocumentView, stepThroughHistory } from "./view";

// The shown document's list of headings, which the overview ruler, the Outline and the heading
// jump keys all read (overview ruler ticket 01, Outline spec "One list of headings, shared").

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: Read[] = [];

let stop: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  fakeTheAppReading(() => documents, {
    step_history: stepAsTheApp,
    forget_forward: forgetForwardAsTheApp,
  });
  stop = new AbortController();
  startHeadings(stop.signal);
});

afterEach(() => {
  stop.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** The documents the window is given, in turn, each read as its HTML. */
function showInTurn(...html: string[]): void {
  documents = html.map((each) => () => Promise.resolve(each));
}

/** The listed headings as id, level and text, in turn. */
function listed(): [string, number, string][] {
  return documentHeadings().headings.map(({ id, level, text }) => [
    id,
    level,
    text,
  ]);
}

/** Starts the view and resolves once its first document is in place and its list built. */
async function startShowing(): Promise<void> {
  const shown = nextShown();
  await startDocumentView(0, stop.signal);
  await shown;
  await afterNextFrame();
}

/** Resolves once the next document is in place and its list built, after its first frame. */
async function nextListed(): Promise<void> {
  await nextShown();
  await afterNextFrame();
}

test("the list starts empty and incomplete when a document is shown, and fills after its first frame", async () => {
  showInTurn(headings);
  const shown = nextShown();
  // Which resolves only once the window's first frame is drawn.
  const started = startDocumentView(0, stop.signal);
  await shown;
  expect(documentHeadings()).toMatchObject({ headings: [], complete: false });

  await started;
  expect(documentHeadings().complete).toBe(true);
  expect(listed()).toHaveLength(10);
});

test("the list holds the h1 to h4 headings with an id, in document order, wherever they sit", async () => {
  showInTurn(headings);
  await startShowing();

  expect(listed()).toEqual([
    ["keeping-lamps", 1, "Keeping lamps"],
    ["wicks", 2, "Wicks"],
    ["cotton-wicks", 3, "Cotton wicks"],
    ["braided-cotton", 4, "Braided cotton"],
    ["wicks-1", 2, "Wicks"],
    ["wicks-in-a-list", 3, "Wicks in a list"],
    ["oil-in-a-quote", 2, "Oil in a quote"],
    ["oil-in-a-closed-section", 3, "Oil in a closed section"],
    ["the---trim-option-and-the-guide", 2, "The --trim option and the guide"],
    [
      "a-long-heading-that-goes-on-and-on-about-the-many-ways-of-keeping-a-lamp-lit-through-a-long-and-windy-night",
      2,
      "A long heading that goes on and on about the many ways of keeping a lamp lit through a long and windy night",
    ],
  ]);
  expect(documentHeadings().complete).toBe(true);
});

test("each listed heading names its element in the document", async () => {
  showInTurn(headings);
  await startShowing();

  for (const { id, element } of documentHeadings().headings)
    expect(document.getElementById(id)).toBe(element);
});

test("a heading's text has its runs of whitespace as one space", async () => {
  showInTurn(
    '<h2 id="spread" data-sourcepos="1:1-2:9">  Spread\n   over <em>two</em>\tlines  </h2>',
  );
  await startShowing();

  expect(listed()).toEqual([["spread", 2, "Spread over two lines"]]);
});

test("the empty window lists no headings", async () => {
  await startShowing();

  expect(documentHeadings()).toMatchObject({ headings: [], complete: true });
});

/**
 * A document of more top-level blocks than the list takes in per task (overview ruler ticket 05),
 * each an h2 with an id, `heading-0` to `heading-9999`.
 */
const LONGER_THAN_A_SLICE = Array.from(
  { length: 10_000 },
  (_, line) =>
    `<h2 id="heading-${String(line)}" data-sourcepos="${String(line + 1)}:1-${String(line + 1)}:12">Heading ${String(line)}</h2>`,
).join("\n");

/** Resolves once the list is complete, a frame at a time, or fails after many frames. */
async function untilComplete(): Promise<void> {
  for (let frame = 0; frame < 1000; frame++) {
    if (documentHeadings().complete) return;
    await afterNextFrame();
  }
  throw new Error("The list was never complete.");
}

test("a long document's list grows a slice at a time, in document order, until it is complete", async () => {
  const heard: [boolean, number][] = [];
  const stopHearing = whenHeadingsChange(({ complete, headings }) => {
    heard.push([complete, headings.length]);
  });
  showInTurn(LONGER_THAN_A_SLICE);
  await startShowing();
  // The first slice is listed after the document's first frame, and the rest is on its way.
  expect(documentHeadings().complete).toBe(false);
  expect(listed().length).toBeGreaterThan(0);
  expect(listed().length).toBeLessThan(10_000);
  expect(documentHeadings().describesView()).toBe(true);

  await untilComplete();
  expect(listed().map(([id]) => id)).toEqual(
    Array.from({ length: 10_000 }, (_, line) => `heading-${String(line)}`),
  );
  const lengths = heard.map(([, length]) => length);
  expect(lengths).toEqual([...lengths].sort((a, b) => a - b));
  expect(heard.at(-1)).toEqual([true, 10_000]);
  expect(heard.filter(([complete]) => complete)).toHaveLength(1);
  stopHearing();
});

test("another document stops the list of a long document on its way", async () => {
  showInTurn(LONGER_THAN_A_SLICE);
  await startShowing();
  expect(documentHeadings().complete).toBe(false);

  showInTurn(headings);
  const heard: string[] = [];
  const stopHearing = whenHeadingsChange((list) => {
    heard.push(list.headings[0]?.id ?? "none");
  });
  const listedNext = nextListed();
  await giveAnotherDocument();
  await listedNext;
  await untilComplete();
  for (let frame = 0; frame < 5; frame++) await afterNextFrame();

  expect(listed()).toHaveLength(10);
  expect(listed()[0]).toEqual(["keeping-lamps", 1, "Keeping lamps"]);
  // Only the new document's lists: first empty, then built.
  expect(heard).toEqual(["none", "keeping-lamps"]);
  stopHearing();
});

test("the list is rebuilt for another document, a reload and Back, and its listeners hear each", async () => {
  const heard: HeadingList[] = [];
  const stopHearing = whenHeadingsChange((list) => {
    heard.push(list);
  });
  showInTurn(readme);
  await startShowing();
  expect(listed()[0]).toEqual(["lamplighter", 1, "Lamplighter"]);

  showInTurn(headings);
  let listedNext = nextListed();
  await giveAnotherDocument();
  await listedNext;
  expect(listed()[0]).toEqual(["keeping-lamps", 1, "Keeping lamps"]);

  showInTurn('<h1 id="saved" data-sourcepos="1:1-1:7">Saved</h1>');
  listedNext = nextListed();
  await saveTheFile();
  await listedNext;
  expect(listed()).toEqual([["saved", 1, "Saved"]]);

  showInTurn(readme);
  listedNext = nextListed();
  stepThroughHistory("back");
  await listedNext;
  await settle();
  expect(listed()[0]).toEqual(["lamplighter", 1, "Lamplighter"]);

  // Each one heard first empty and incomplete, then complete.
  expect(
    heard.map(({ headings, complete }) => [complete, headings[0]?.id]),
  ).toEqual([
    [false, undefined],
    [true, "lamplighter"],
    [false, undefined],
    [true, "keeping-lamps"],
    [false, undefined],
    [true, "saved"],
    [false, undefined],
    [true, "lamplighter"],
  ]);
  stopHearing();
});
