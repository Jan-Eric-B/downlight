// What the overview ruler's tests share (overview-ruler.test.ts, overview-ruler-headings.test.ts):
// a window with the ruler started as the page starts it, its documents, and the reader's ways to
// look at the strip, use Find and click the strip. The layout fake makes each line of the file 20
// px high, and the scroller 736 px high (happy-dom's 768 px window, less the title bar).
import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { vi } from "vitest";
import { DEFAULT_SETTINGS, type PageSettings, type WaitNote } from "./app";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startHeadings } from "./headings";
import { startOverviewRuler } from "./overview-ruler";
import { afterNextFrame } from "./reading-position";
import { startSettings } from "./settings";
import {
  fakeTheAppReading,
  fakeTitleBarLayout,
  forgetForwardAsTheApp,
  nextShown,
  openPageAt,
  stepAsTheApp,
  stopDocumentSample,
  TITLE_BAR_HEIGHT,
  waitNoteAsTheApp,
  type Read,
} from "./test-support";
import { DOCUMENT_VIEW_SELECTOR, startDocumentView } from "./view";

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: Read[] = [];

let stop: AbortController;

/**
 * Lays the page out anew, as the webview does when sizes change: all of them, or those of the
 * elements given.
 */
let layOutSizes: (...changed: Element[]) => void;

/** What the fake app's wait note names. */
const BIG_DOCUMENT: WaitNote = {
  file: { name: "big.md", path: "C:\\docs\\big.md" },
  size: "100 MB",
};

/** The page's URL with `settings` in it, as the app opens a window. */
export function settingsUrl(settings: PageSettings): string {
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

/** Opens a window with the default settings and the fake app, for each test. */
export async function openRulerWindow(): Promise<void> {
  mockWindows("document-0");
  documents = [];
  layOutSizes = fakeTitleBarLayout();
  fakeTheAppReading(() => documents, {
    wait_note: (args) => waitNoteAsTheApp(args, BIG_DOCUMENT),
    stop_document: () => stopDocumentSample("stopped"),
    step_history: stepAsTheApp,
    forget_forward: forgetForwardAsTheApp,
  });
  await startRulerAt(settingsUrl(DEFAULT_SETTINGS));
}

/** Closes what `openRulerWindow` opened, so that no test inherits another's window. */
export function closeRulerWindow(): void {
  stop.abort();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  openPageAt("");
  document.body.replaceChildren();
}

/** Closes the window's page parts and starts them again as a window opened at `url` does. */
export async function reopenRulerAt(url: string): Promise<void> {
  stop.abort();
  await startRulerAt(url);
}

/** The documents the window is given, in turn, each read as given. */
export function readInTurn(...reads: Read[]): void {
  documents = reads;
}

/** A read that gives the HTML the test sends through the function it hands `send`. */
export function readSentLater(
  send: (deliver: (html: string) => void) => void,
): Read {
  return () =>
    new Promise((resolve) => {
      send(resolve);
    });
}

/** The documents the window is given, in turn, each read as its HTML. */
export function showInTurn(...html: string[]): void {
  readInTurn(...html.map((each) => () => Promise.resolve(each)));
}

/** Lays the page out anew, as the webview does when sizes change: all, or the elements'. */
export function resize(...changed: Element[]): void {
  layOutSizes(...changed);
}

/**
 * A document of 100 lines, 2,000 px, which the 736 px strip shows at 736/2000: its h1 on line 1
 * is in row 0 and its h2 on line 51 in row 368.
 */
export const LONG = [
  '<h1 id="long" data-sourcepos="1:1-1:5">Long</h1>',
  '<h2 id="half-way" data-sourcepos="51:1-51:9">Half way</h2>',
  '<p data-sourcepos="100:1-100:4">End</p>',
].join("\n");

/** Starts the view and resolves once its first document is in place and the ruler drew it. */
export async function startShowing(): Promise<void> {
  const shown = nextShown();
  await startDocumentView(0, stop.signal);
  await shown;
  await drawn();
}

/** Resolves once a redraw the ruler has asked for has been made: after the next frame. */
export async function drawn(): Promise<void> {
  await afterNextFrame();
  await afterNextFrame();
}

export function strip(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".overview-ruler");
  if (!found) throw new Error("The page has no overview ruler.");
  return found;
}

/** What a mark stands for: the heading level it is drawn for. */
type MarkKind = "h1" | "h2" | "h3" | "h4";

/** The strip's heading marks, top to bottom, each as its kind and its top in the strip, in px. */
export function marks(): { kind: MarkKind; top: number }[] {
  return [...strip().querySelectorAll<HTMLElement>(".heading-mark")].map(
    (mark) => ({
      kind: `h${mark.dataset.level ?? ""}` as MarkKind,
      top: Number.parseFloat(mark.style.top),
    }),
  );
}

export function documentView(): Element {
  const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
  if (!view) throw new Error("The page has no document view.");
  return view;
}

/** Where a press or click on the strip `row` px below its top lands, in its left or right half. */
export function onStrip(half: "left" | "right", row: number): MouseEventInit {
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
export function clickStrip(
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

/** Stands in for the webview's scrolls, and returns the spy that records them. */
export function spyOnScrolls() {
  return vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
}

export function findBar(): HTMLElement {
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
export function findStatus(): string | null | undefined {
  return findBar().querySelector('[role="status"]')?.textContent;
}

/** Presses a key where the focus is, as the reader would. */
export function press(init: KeyboardEventInit): void {
  (document.activeElement ?? document.body).dispatchEvent(
    new KeyboardEvent("keydown", { ...init, bubbles: true, cancelable: true }),
  );
}

/** Opens the find bar with Ctrl+F, unless it is open, and types `text` into its box. */
export function typeInFind(text: string): void {
  if (findBar().hidden) press({ ctrlKey: true, key: "f", code: "KeyF" });
  findBox().value = text;
  findBox().dispatchEvent(new Event("input", { bubbles: true }));
}

/** What a mark of Find stands for: a match, or the current match. */
type FindMarkKind = "match" | "current";

/** The strip's marks of Find, top to bottom, the current match's last, each as its top in px. */
export function findMarks(): { kind: FindMarkKind; top: number }[] {
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
export function currentMarkTop(): number | undefined {
  return findMarks().find(({ kind }) => kind === "current")?.top;
}
