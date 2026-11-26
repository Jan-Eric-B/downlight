import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import headings from "../../core/tests/fixtures/headings.html?raw";
import { startHeadings } from "./headings";
import { startOverviewRuler } from "./overview-ruler";
import { afterNextFrame } from "./reading-position";
import type { WaitNote } from "./app";
import {
  deleteTheFile,
  fakeTheAppReading,
  fakeTitleBarLayout,
  forgetForwardAsTheApp,
  giveAnotherDocument,
  goneBar,
  layOut,
  nextShown,
  saveTheFile,
  shownWaitNote,
  stepAsTheApp,
  stopDocumentSample,
  waitNoteAsTheApp,
  type Read,
} from "./test-support";
import {
  DOCUMENT_VIEW_SELECTOR,
  startDocumentView,
  stepThroughHistory,
} from "./view";
import { WAIT_NOTE_MS } from "./wait-note";

// The overview ruler's strip beside the scroll bar, with a mark for each of the shown document's
// h1 to h4 headings (overview ruler ticket 01). The layout fake makes each line of the file 20 px
// high, and the scroller 736 px high (happy-dom's 768 px window, less the title bar).

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: Read[] = [];

let stop: AbortController;

/** What the fake app's wait note names. */
const BIG_DOCUMENT: WaitNote = {
  file: { name: "big.md", path: "C:\\docs\\big.md" },
  size: "100 MB",
};

/**
 * Lays the page out anew, as the webview does when sizes change: all of them, or those of the
 * elements given.
 */
let resize: (...changed: Element[]) => void;

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  resize = fakeTitleBarLayout();
  fakeTheAppReading(() => documents, {
    wait_note: (args) => waitNoteAsTheApp(args, BIG_DOCUMENT),
    stop_document: () => stopDocumentSample("stopped"),
    step_history: stepAsTheApp,
    forget_forward: forgetForwardAsTheApp,
  });
  stop = new AbortController();
  startHeadings(stop.signal);
  startOverviewRuler(stop.signal);
});

afterEach(() => {
  stop.abort();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** The documents the window is given, in turn, each read as its HTML. */
function showInTurn(...html: string[]): void {
  documents = html.map((each) => () => Promise.resolve(each));
}

/** The raw HTML of `headings.md` laid out on its file's lines, which the core gives no numbers. */
const HEADINGS_RAW_HTML: Record<string, [number, number]> = {
  details: [39, 46],
  "h2:not([id])": [48, 48],
};

/** A read that gives the HTML the test sends through the function it hands `send`. */
function readSentLater(send: (deliver: (html: string) => void) => void): Read {
  return () =>
    new Promise((resolve) => {
      send(resolve);
    });
}

/**
 * A document of 100 lines, 2,000 px, which the 736 px strip shows at 736/2000: its h1 on line 1
 * is in row 0 and its h2 on line 51 in row 368.
 */
const LONG = [
  '<h1 id="long" data-sourcepos="1:1-1:5">Long</h1>',
  '<h2 id="half-way" data-sourcepos="51:1-51:9">Half way</h2>',
  '<p data-sourcepos="100:1-100:4">End</p>',
].join("\n");

/** Starts the view and resolves once its first document is in place and the ruler drew it. */
async function startShowing(): Promise<void> {
  const shown = nextShown();
  await startDocumentView(0, stop.signal);
  await shown;
  await drawn();
}

/** Resolves once a redraw the ruler has asked for has been made: after the next frame. */
async function drawn(): Promise<void> {
  await afterNextFrame();
  await afterNextFrame();
}

/**
 * Lays the page out anew (`resize`) as soon as the view's content is next replaced, before the
 * `whenDocumentShown` listeners hear of it, as the webview does in the frame of a landing.
 */
function resizeOnceReplaced(): void {
  const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
  if (!view) throw new Error("The page has no document view.");
  const replaced = new MutationObserver(() => {
    replaced.disconnect();
    resize();
  });
  replaced.observe(view, { childList: true });
}

function strip(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".overview-ruler");
  if (!found) throw new Error("The page has no overview ruler.");
  return found;
}

/** What a mark stands for: the heading level it is drawn for. */
type MarkKind = "h1" | "h2" | "h3" | "h4";

/** The strip's marks, top to bottom, each as its kind and its top in the strip, in px. */
function marks(): { kind: MarkKind; top: number }[] {
  return [...strip().querySelectorAll<HTMLElement>(".heading-mark")].map(
    (mark) => ({
      kind: `h${mark.dataset.level ?? ""}` as MarkKind,
      top: Number.parseFloat(mark.style.top),
    }),
  );
}

test("each h1 to h4 heading with an id has a mark at its top-level block's top, scaled to the strip, and h5, h6 and raw headings none", async () => {
  layOut(1, HEADINGS_RAW_HTML);
  showInTurn(headings);
  await startShowing();

  // The document runs to line 56, 1120 px, so the 736 px strip shows each px of it at 736/1120.
  expect(strip().hidden).toBe(false);
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 52 },
    { kind: "h3", top: 105 },
    { kind: "h4", top: 157 },
    // Lines 17 and 21 hold the h5 and the h6.
    { kind: "h2", top: 315 },
    // The heading in the list (line 31) at the list's top, line 29.
    { kind: "h3", top: 368 },
    // The heading in the quote at the quote's top, line 35.
    { kind: "h2", top: 446 },
    // The heading in the closed section (line 42) at the section's top, line 39.
    { kind: "h3", top: 499 },
    // Line 48 holds the raw heading without an id.
    { kind: "h2", top: 644 },
    { kind: "h2", top: 696 },
  ]);
});

test("marks that fall in one pixel row merge into one, as long as its highest heading's", async () => {
  layOut(1);
  // A document of 1000 lines, 20,000 px, shows about 27 px of it in each px of the strip: the
  // h4 on line 12 and the h2 on line 13 both fall in row 8.
  showInTurn(
    [
      '<h4 id="small" data-sourcepos="12:1-12:5">Small</h4>',
      '<h2 id="large" data-sourcepos="13:1-13:5">Large</h2>',
      '<h3 id="alone" data-sourcepos="500:1-500:5">Alone</h3>',
      '<p data-sourcepos="1000:1-1000:4">End</p>',
    ].join("\n"),
  );
  await startShowing();

  expect(marks()).toEqual([
    { kind: "h2", top: 8 },
    { kind: "h3", top: 367 },
  ]);
});

test("the strip is hidden in the empty window", async () => {
  layOut(1);
  await startShowing();

  expect(strip().hidden).toBe(true);
});

test("the strip is hidden for a document that doesn't scroll", async () => {
  layOut(1);
  // 36 lines, 720 px, fit the 736 px view.
  showInTurn(
    '<h1 id="short" data-sourcepos="1:1-1:5">Short</h1>\n<p data-sourcepos="36:1-36:4">End</p>',
  );
  await startShowing();

  expect(strip().hidden).toBe(true);
});

test("the strip is hidden from screen readers and takes no focus", async () => {
  layOut(1, HEADINGS_RAW_HTML);
  showInTurn(headings);
  await startShowing();

  expect(strip().getAttribute("aria-hidden")).toBe("true");
  // happy-dom focuses any element; the webview focuses a div only with a tab index.
  expect(strip().hasAttribute("tabindex")).toBe(false);
  expect(strip().querySelector("[tabindex], button, a")).toBeNull();
});

test("the strip is hidden while a wait note shows for another document, and shows that one's marks once it is in view", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  layOut(1, HEADINGS_RAW_HTML);
  showInTurn(headings);
  await startShowing();
  expect(strip().hidden).toBe(false);

  let deliver: (html: string) => void = () => {};
  documents = [readSentLater((send) => (deliver = send))];
  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()).not.toBeNull();
  });
  await drawn();
  expect(strip().hidden).toBe(true);

  const shown = nextShown();
  deliver(LONG);
  await shown;
  await drawn();
  expect(strip().hidden).toBe(false);
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
});

test("stopping a document on its way brings back the ruler of the one in view", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  documents = [readSentLater(() => undefined)];
  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()).not.toBeNull();
  });
  shownWaitNote()?.querySelector("button")?.click();
  await vi.waitFor(() => {
    expect(shownWaitNote()).toBeNull();
  });
  await drawn();
  expect(strip().hidden).toBe(false);
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
});

test("during a reload the old marks stay until the new render replaces the document", async () => {
  // Scrolled to the h2, so the reload lands back at it, which takes a frame.
  layOut(51);
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(LONG),
    readSentLater((send) => (deliver = send)),
  ];
  await startShowing();

  await saveTheFile();
  await drawn();
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);

  // The webview lays the new render out while the view lands back at the reading position, before
  // its list of headings is built: the ruler redraws then.
  resizeOnceReplaced();
  const shown = nextShown();
  // The h2 moved to line 26: 500 px down the 2,000, row 184.
  deliver(LONG.replace("51:1-51:9", "26:1-26:9"));
  await shown;
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
  await drawn();
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 184 },
  ]);
});

test("Back gives the strip the marks of the document returned to", async () => {
  layOut(1, HEADINGS_RAW_HTML);
  showInTurn(LONG, headings);
  await startShowing();

  let shown = nextShown();
  await giveAnotherDocument();
  await shown;
  await drawn();
  expect(marks()).toHaveLength(10);

  showInTurn(LONG);
  shown = nextShown();
  stepThroughHistory("back");
  await shown;
  await drawn();
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
});

test("the marks follow the view's size as it changes, as when zoom wraps the text anew", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  // Wrapped wider, the document runs to line 80, 1,600 px, and the h2 is on line 21, 400 px down,
  // in row 184.
  layOut(1, { "#half-way": [21, 21], p: [80, 80] });
  await drawn();
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
  resize();
  await drawn();
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 184 },
  ]);
});

test("the marks move down when the 'File deleted or moved' bar shows above the document", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  await deleteTheFile();
  const bar = goneBar();
  expect(bar?.hidden).toBe(false);
  // The bar's 41 px come before the document, 2,041 px in all: the h1 is 41 px down, in row 14,
  // and the h2 1,041 px down, in row 375.
  layOut(1, {}, 41);
  if (bar) resize(bar);
  await drawn();
  expect(marks()).toEqual([
    { kind: "h1", top: 14 },
    { kind: "h2", top: 375 },
  ]);
});

test("the marks stay where they are while the view scrolls, as the scroll bar's track does", async () => {
  layOut(40);
  showInTurn(LONG);
  await startShowing();

  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
});
