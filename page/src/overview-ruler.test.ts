import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import headings from "../../core/tests/fixtures/headings.html?raw";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startHeadings } from "./headings";
import { startOverviewRuler } from "./overview-ruler";
import { afterNextFrame } from "./reading-position";
import { DEFAULT_SETTINGS, type PageSettings, type WaitNote } from "./app";
import { startSettings } from "./settings";
import {
  deleteTheFile,
  fakeTheAppReading,
  fakeTitleBarLayout,
  forgetForwardAsTheApp,
  giveAnotherDocument,
  goneBar,
  layOut,
  linesOf,
  messageSample,
  nextShown,
  openPageAt,
  pageUrlSample,
  saveTheFile,
  shownWaitNote,
  stepAsTheApp,
  stopDocumentSample,
  tell,
  TITLE_BAR_HEIGHT,
  tellAsSampled,
  waitNoteAsTheApp,
  type Read,
} from "./test-support";
import {
  canStepThroughHistory,
  DOCUMENT_VIEW_SELECTOR,
  SCROLLER_SELECTOR,
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

beforeEach(async () => {
  mockWindows("document-0");
  documents = [];
  resize = fakeTitleBarLayout();
  fakeTheAppReading(() => documents, {
    wait_note: (args) => waitNoteAsTheApp(args, BIG_DOCUMENT),
    stop_document: () => stopDocumentSample("stopped"),
    step_history: stepAsTheApp,
    forget_forward: forgetForwardAsTheApp,
  });
  await startRulerAt(settingsUrl(DEFAULT_SETTINGS));
});

afterEach(() => {
  stop.abort();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  openPageAt("");
  document.body.replaceChildren();
});

/** The page's URL with `settings` in it, as the app opens a window. */
function settingsUrl(settings: PageSettings): string {
  return `?settings=${encodeURIComponent(JSON.stringify(settings))}`;
}

/**
 * Starts the settings, the commands, Find, the list of headings and the ruler as a new window
 * opened at `url` does, in the page's order.
 */
async function startRulerAt(url: string): Promise<void> {
  openPageAt(url);
  await startSettings();
  stop = new AbortController();
  startCommands(stop.signal);
  startFind(stop.signal);
  startHeadings(stop.signal);
  startOverviewRuler(stop.signal);
}

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

// The ruler's switch in the settings (overview ruler ticket 02).

const RULER_OFF: PageSettings = { ...DEFAULT_SETTINGS, overviewRuler: false };

test("switching the ruler off in the settings hides the strip, and switching it on shows its marks again", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  expect(strip().hidden).toBe(false);

  await tell("settings", RULER_OFF);
  await drawn();
  expect(strip().hidden).toBe(true);

  await tell("settings", DEFAULT_SETTINGS);
  await drawn();
  expect(strip().hidden).toBe(false);
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
});

test("a window opened with the ruler switched off shows no strip until it is switched on", async () => {
  stop.abort();
  await startRulerAt(settingsUrl(RULER_OFF));
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  expect(strip().hidden).toBe(true);

  await tell("settings", DEFAULT_SETTINGS);
  await drawn();
  expect(strip().hidden).toBe(false);
  expect(marks()).toEqual([
    { kind: "h1", top: 0 },
    { kind: "h2", top: 368 },
  ]);
});

test("switching the ruler leaves the view where it was", async () => {
  // Halfway down the h2's line, 51, part way into its block.
  layOut(51.5);
  showInTurn(LONG);
  await startShowing();
  const scroller = document.querySelector<HTMLElement>(SCROLLER_SELECTOR);
  const before = scroller?.scrollTop;
  // A scroll the webview makes puts the element's first line at the top of the view.
  const scrolled = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(function (this: Element) {
      layOut(linesOf(this)?.[0] ?? 1);
    });

  await tell("settings", RULER_OFF);
  await drawn();
  expect(strip().hidden).toBe(true);
  await tell("settings", DEFAULT_SETTINGS);
  await drawn();
  expect(strip().hidden).toBe(false);

  expect(scroller?.scrollTop).toBe(before);
  expect(scrolled).not.toHaveBeenCalled();
});

test("a settings change that leaves the ruler as it is doesn't move the view", async () => {
  layOut(51);
  showInTurn(LONG);
  await startShowing();
  const scrolled = vi.spyOn(Element.prototype, "scrollIntoView");

  await tell("settings", { ...DEFAULT_SETTINGS, theme: "dark" });
  await drawn();
  expect(scrolled).not.toHaveBeenCalled();
  expect(strip().hidden).toBe(false);
});

// The app's samples of the snapshot (`messages.json` and `page_url.json`), whose ruler is off.

test("the app's settings message switches the ruler off", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  expect(strip().hidden).toBe(false);

  const sample = messageSample("settings");
  expect(sample.event === "settings" && sample.payload.overviewRuler).toBe(
    false,
  );
  await tellAsSampled(sample);
  await drawn();
  expect(strip().hidden).toBe(true);
});

test("a window opened at the app's URL starts with the ruler of its settings", async () => {
  stop.abort();
  await startRulerAt(pageUrlSample("allOff"));
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  expect(strip().hidden).toBe(true);
});

// Find's marks, and clicking them (overview ruler ticket 03).

/**
 * A document of 100 lines, 2,000 px, shown at 736/2000 as `LONG` is, with "lamp" four times: in
 * the h1 on line 1, row 0; at the start and halfway through the text of the paragraph on lines 11
 * to 20, 200 px and 300 px down, rows 73 and 110; and in the paragraph on line 51, row 368.
 */
const LAMPS = [
  '<h1 id="lamps" data-sourcepos="1:1-1:5">Lamps</h1>',
  '<p data-sourcepos="11:1-20:20">lamp wick lamp glass</p>',
  '<p data-sourcepos="51:1-51:4">lamp</p>',
  '<p data-sourcepos="100:1-100:4">End</p>',
].join("\n");

function findBar(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".find-bar");
  if (!found) throw new Error("The page has no find bar.");
  return found;
}

function findBox(): HTMLInputElement {
  const found = findBar().querySelector("input");
  if (!found) throw new Error("The find bar has no box.");
  return found;
}

/** The find bar's count, such as "3 of 4". */
function findStatus(): string | null | undefined {
  return findBar().querySelector('[role="status"]')?.textContent;
}

/** Presses a key where the focus is, as the reader would. */
function press(init: KeyboardEventInit): void {
  (document.activeElement ?? document.body).dispatchEvent(
    new KeyboardEvent("keydown", { ...init, bubbles: true, cancelable: true }),
  );
}

/** Opens the find bar with Ctrl+F, unless it is open, and types `text` into its box. */
function typeInFind(text: string): void {
  if (findBar().hidden) press({ ctrlKey: true, key: "f", code: "KeyF" });
  findBox().value = text;
  findBox().dispatchEvent(new Event("input", { bubbles: true }));
}

const ENTER = { key: "Enter", code: "Enter" };

/** What a mark of Find stands for: a match, or the current match. */
type FindMarkKind = "match" | "current";

/** The strip's marks of Find, top to bottom, the current match's last, each as its top in px. */
function findMarks(): { kind: FindMarkKind; top: number }[] {
  return [
    ...strip().querySelectorAll<HTMLElement>(
      ".match-mark, .current-match-mark",
    ),
  ].map((mark) => ({
    kind: mark.classList.contains("current-match-mark") ? "current" : "match",
    top: Number.parseFloat(mark.style.top),
  }));
}

/** The top of the current match's mark, in px, or undefined while it has none. */
function currentMarkTop(): number | undefined {
  return findMarks().find(({ kind }) => kind === "current")?.top;
}

function documentView(): Element {
  const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
  if (!view) throw new Error("The page has no document view.");
  return view;
}

/** Where a press or click on the strip `row` px below its top lands, in its left or right half. */
function onStrip(half: "left" | "right", row: number): MouseEventInit {
  return {
    bubbles: true,
    cancelable: true,
    button: 0,
    clientX: half === "right" ? 797 : 789,
    clientY: TITLE_BAR_HEIGHT + row,
  };
}

/**
 * Clicks the strip `row` px below its top, in its left half (the heading marks' lane) or its right
 * half (Find's), as the reader does. The layout fake's scroller runs from below the title bar to
 * x = 800, with no scroll bar, so the strip covers x = 786 to 800. The strip takes no pointer, so
 * that the wheel over it scrolls the document, and what is under it, the document view unless the
 * test names another element, gets the press and the click. Returns the press.
 */
function clickStrip(
  half: "left" | "right",
  row: number,
  under: Element = documentView(),
): MouseEvent {
  const at = onStrip(half, row);
  const pressed = new MouseEvent("mousedown", at);
  under.dispatchEvent(pressed);
  under.dispatchEvent(new MouseEvent("mouseup", at));
  under.dispatchEvent(new MouseEvent("click", at));
  return pressed;
}

test("Find's matches get marks as the reader types, and the current match one of its own", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  expect(findMarks()).toEqual([]);

  typeInFind("lamp");
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 0 },
    { kind: "match", top: 73 },
    { kind: "match", top: 110 },
    { kind: "match", top: 368 },
    { kind: "current", top: 0 },
  ]);
  // The h1's mark shares row 0 with its match's, and both show.
  expect(marks()).toEqual([{ kind: "h1", top: 0 }]);

  typeInFind("lamp g");
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 110 },
    { kind: "current", top: 110 },
  ]);
});

test("the current match's mark follows Enter, F3 and Shift+Enter", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");

  press(ENTER);
  await drawn();
  expect(currentMarkTop()).toBe(73);
  press({ key: "F3", code: "F3" });
  await drawn();
  expect(currentMarkTop()).toBe(110);
  press({ ...ENTER, shiftKey: true });
  await drawn();
  expect(currentMarkTop()).toBe(73);
  expect(findMarks().filter(({ kind }) => kind === "match")).toHaveLength(4);
});

test("closing Find takes its marks away, and the heading marks stay", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();

  press({ key: "Escape", code: "Escape" });
  await drawn();
  expect(findBar().hidden).toBe(true);
  expect(findMarks()).toEqual([]);
  expect(marks()).toEqual([{ kind: "h1", top: 0 }]);
});

test("a text with no match leaves Find no marks", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();

  typeInFind("lantern");
  await drawn();
  expect(findStatus()).toBe("No results");
  expect(findMarks()).toEqual([]);
});

test("a reload's recount moves Find's marks to where its matches are now", async () => {
  layOut(1);
  documents = [
    () => Promise.resolve(LAMPS),
    // The paragraph on line 51 moved to line 26: 500 px down the 2,000, row 184.
    () => Promise.resolve(LAMPS.replace("51:1-51:4", "26:1-26:4")),
  ];
  await startShowing();
  typeInFind("lamp");
  await drawn();

  const shown = nextShown();
  await saveTheFile();
  await shown;
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 0 },
    { kind: "match", top: 73 },
    { kind: "match", top: 110 },
    { kind: "match", top: 184 },
    { kind: "current", top: 0 },
  ]);
});

test("a click near a match mark makes the nearest match current and scrolls to it, as F3 does, and Back doesn't return from it", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolled = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});

  // Row 112 is 1.6 px below the match 110.4 px down the strip, and 38.4 px below the one above it.
  const pressed = clickStrip("right", 112);
  expect(findStatus()).toBe("3 of 4");
  expect(scrolled.mock.contexts.at(-1)).toBe(
    document.querySelector('[data-sourcepos="11:1-20:20"]'),
  );
  expect(scrolled.mock.calls.at(-1)).toEqual([{ block: "center" }]);
  // The press doesn't move the focus out of the find box, nor start a selection.
  expect(pressed.defaultPrevented).toBe(true);
  await drawn();
  expect(currentMarkTop()).toBe(110);

  // Find's steps stay out of History: there is nowhere to go back to.
  expect(canStepThroughHistory("back")).toBe(false);
});

test("a click on the strip near no mark does nothing", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolled = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});

  const pressed = clickStrip("right", 250);
  expect(pressed.defaultPrevented).toBe(false);
  expect(findStatus()).toBe("1 of 4");
  expect(scrolled).not.toHaveBeenCalled();
  await drawn();
  expect(currentMarkTop()).toBe(0);
});

test("a click on the strip while Find is closed does nothing", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  const scrolled = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});

  expect(clickStrip("right", 0).defaultPrevented).toBe(false);
  expect(scrolled).not.toHaveBeenCalled();
});

/** Stands in for the webview's scroll to a match, and returns the spy that records it. */
function spyOnScrolls() {
  return vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
}

test("a selection dragged from the text and let go beside a match mark leaves the current match and the view as they were", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolled = spyOnScrolls();

  // Pressed in the paragraph's text and let go in the right padding, 1.6 px below the match 110.4
  // px down the strip: the webview gives the click to the view, which holds both.
  const paragraph = document.querySelector('[data-sourcepos="11:1-20:20"]');
  paragraph?.dispatchEvent(
    new MouseEvent("mousedown", { ...onStrip("right", 100), clientX: 400 }),
  );
  documentView().dispatchEvent(
    new MouseEvent("mouseup", onStrip("right", 112)),
  );
  documentView().dispatchEvent(new MouseEvent("click", onStrip("right", 112)));
  expect(findStatus()).toBe("1 of 4");
  expect(scrolled).not.toHaveBeenCalled();
  await drawn();
  expect(currentMarkTop()).toBe(0);
});

test("a click in the strip's left half beside a match mark does nothing", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolled = spyOnScrolls();

  expect(clickStrip("left", 112).defaultPrevented).toBe(false);
  expect(findStatus()).toBe("1 of 4");
  expect(scrolled).not.toHaveBeenCalled();
});

test("a right-click on a match mark does nothing", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolled = spyOnScrolls();

  const at = { ...onStrip("right", 112), button: 2 };
  const pressed = new MouseEvent("mousedown", at);
  documentView().dispatchEvent(pressed);
  documentView().dispatchEvent(new MouseEvent("mouseup", at));
  documentView().dispatchEvent(new MouseEvent("contextmenu", at));
  expect(pressed.defaultPrevented).toBe(false);
  expect(findStatus()).toBe("1 of 4");
  expect(scrolled).not.toHaveBeenCalled();
});

test("a click on the 'File deleted or moved' bar over a match mark does nothing", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await deleteTheFile();
  const bar = goneBar();
  if (!bar) throw new Error("The page has no 'File deleted or moved' bar.");
  // The bar's 41 px come before the document, 2,041 px in all: the h1's match is 41 px down, 14.8
  // px down the strip, under the bar.
  layOut(1, {}, 41);
  resize(bar);
  await drawn();
  expect(currentMarkTop()).toBe(14);
  const scrolled = spyOnScrolls();

  expect(clickStrip("right", 15, bar).defaultPrevented).toBe(false);
  expect(scrolled).not.toHaveBeenCalled();
});

test("a click in the frame between a count and the strip's redraw of it does nothing", async () => {
  layOut(1);
  showInTurn(LAMPS);
  await startShowing();
  typeInFind("lamp");
  await drawn();

  // Row 0 still shows the mark of the h1's match, which "lamp g" doesn't have.
  typeInFind("lamp g");
  const scrolled = spyOnScrolls();
  expect(clickStrip("right", 0).defaultPrevented).toBe(false);
  expect(findStatus()).toBe("1 of 1");
  expect(scrolled).not.toHaveBeenCalled();
});

/**
 * "ab" three times in a paragraph on lines 26 to 50, 500 px down and 500 px high, rows 184 to 368
 * of the strip: its 368 characters are half a pixel each, so the matches at 1, 7 and 11 are 184.5,
 * 187.5 and 189.5 px down the strip.
 */
const NEAR = [
  '<h1 id="near" data-sourcepos="1:1-1:4">Near</h1>',
  `<p data-sourcepos="26:1-50:10">xabxxxxabxxab${"x".repeat(355)}</p>`,
  '<p data-sourcepos="100:1-100:3">End</p>',
].join("\n");

/** The strip's pixel rows that marks of Find's matches, not the current one, cover. */
function matchMarkRows(): number[] {
  return [...strip().querySelectorAll<HTMLElement>(".match-mark")].flatMap(
    (mark) => {
      const top = Number.parseFloat(mark.style.top);
      const height = Number.parseFloat(mark.style.height);
      return Array.from({ length: height }, (_, row) => top + row);
    },
  );
}

test("each match's mark is 2 px high from its row, and marks in rows close together run into one another", async () => {
  layOut(1);
  showInTurn(NEAR);
  await startShowing();
  typeInFind("ab");
  await drawn();

  expect(matchMarkRows()).toEqual([184, 185, 187, 188, 189, 190]);
});

test("a click between two match marks makes the nearer match current", async () => {
  layOut(1);
  showInTurn(NEAR);
  await startShowing();
  typeInFind("ab");
  await drawn();
  spyOnScrolls();

  // 2.5 px below the first match and 0.5 px above the second.
  clickStrip("right", 187);
  expect(findStatus()).toBe("2 of 3");
  await drawn();
  expect(currentMarkTop()).toBe(187);
});

test("Back with Find open counts the document returned to and moves Find's marks to its matches", async () => {
  layOut(1);
  // The paragraph on line 51 is on line 26 in the other document: 500 px down the 2,000, row 184.
  showInTurn(LAMPS, LAMPS.replace("51:1-51:4", "26:1-26:4"));
  await startShowing();
  typeInFind("lamp");
  let shown = nextShown();
  await giveAnotherDocument();
  await shown;
  await drawn();
  expect(findMarks().at(3)).toEqual({ kind: "match", top: 184 });

  showInTurn(LAMPS);
  shown = nextShown();
  stepThroughHistory("back");
  await shown;
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 0 },
    { kind: "match", top: 73 },
    { kind: "match", top: 110 },
    { kind: "match", top: 368 },
    { kind: "current", top: 0 },
  ]);
});
