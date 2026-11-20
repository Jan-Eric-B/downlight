// Helpers shared by the page's tests. The name has no `.test`, so Vitest doesn't run it as a test
// file, and nothing the app runs imports it.
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { vi } from "vitest";
import editorChoicesSamples from "../../app/tests/fixtures/editor_choices.json?raw";
import followLinkSamples from "../../app/tests/fixtures/follow_link.json?raw";
import fullScreenSamples from "../../app/tests/fixtures/full_screen.json?raw";
import keepOnTopSamples from "../../app/tests/fixtures/keep_on_top.json?raw";
import menuEntrySamplesJson from "../../app/tests/fixtures/menu_entries.json?raw";
import messageSamples from "../../app/tests/fixtures/messages.json?raw";
import openNamedFolderSamples from "../../app/tests/fixtures/open_named_folder.json?raw";
import pageUrlSamples from "../../app/tests/fixtures/page_url.json?raw";
import renderDocumentSamples from "../../app/tests/fixtures/render_document.json?raw";
import stopDocumentSamples from "../../app/tests/fixtures/stop_document.json?raw";
import settingChangeSamplesJson from "../../app/tests/fixtures/setting_changes.json?raw";
import timingReportSamples from "../../app/tests/fixtures/timing_reports.json?raw";
import titleBarSamples from "../../app/tests/fixtures/title_bar.json?raw";
import waitNoteSamples from "../../app/tests/fixtures/wait_note.json?raw";
import type {
  AccentScale,
  AnswerOf,
  ArgsOf,
  ButtonRect,
  Command,
  DocumentFile,
  DocumentToShow,
  EditorChoices,
  FirstPaintMarks,
  Followed,
  LongTask,
  MenuEntry,
  Message,
  Messages,
  Notice,
  PageFlags,
  Place,
  ReloadMarks,
  SettingChange,
  WaitNote,
} from "./app";
import { startSettings } from "./settings";
import { SCROLLER_SELECTOR, whenDocumentShown } from "./view";
import { ZOOM_VARIABLE } from "./zoom";

/**
 * A scale of Windows' accent color, a green unlike amber and unlike the contract samples' blue, so
 * a test can tell which scale shows.
 */
export const WINDOWS_GREEN: AccentScale = {
  "light-3": "#b7f2b7",
  "light-2": "#86e386",
  "light-1": "#4fcf4f",
  base: "#2fa82f",
  "dark-1": "#238a23",
  "dark-2": "#176317",
  "dark-3": "#0c3f0c",
};

/** A document shown at a heading, as the app hands it over for a link like `a.md#install`. */
export interface AtHeading {
  document: string;
  heading: string;
}

/**
 * What reading the window's document gives the fake app: its HTML, its HTML at a heading, or
 * null while the window shows no document. A read that rejects is a file that can't be read.
 */
export type Read = () => Promise<string | AtHeading | null>;

/** The showing the fake app's window is at (`Showing` in `app/src/documents.rs`). */
let currentShowing = 0;

/** The showings the fake app has issued, each one new. */
let showingsIssued = 1;

/**
 * The fake app window's history (`History` in `app/src/documents/history.rs`): the showings to go
 * back and forward to, nearest last, and whether Back or Forward gave the window the showing it is
 * at, so that its page returns to where it was.
 */
let windowHistory: { back: number[]; forward: number[]; stepped: boolean } = {
  back: [],
  forward: [],
  stepped: false,
};

/**
 * How the fake app answers each command a test gives it, in the module's types, so a test can't
 * answer a command the app doesn't have, or answer it in another shape. A command a test leaves
 * out answers null, as a command that returns `()` does, except one whose answer is never null
 * (`follow_link`), which fails with a message that names it. An answer that throws or rejects is
 * a command that fails.
 */
export type FakeApp = {
  [C in Command]?: (args: ArgsOf<C>) => AnswerOf<C> | Promise<AnswerOf<C>>;
};

/** The commands whose answer is never null. */
type AnsweredWithMore = {
  [C in Command]: null extends AnswerOf<C> ? never : C;
}[Command];

/**
 * The commands a test has to tell the fake app how to answer. A null answer would reach the page
 * as a TypeError it shows as a notice, which hides the missing setup. The type makes a command the
 * app adds with such an answer join the list.
 */
const MUST_BE_ANSWERED: Record<AnsweredWithMore, true> = {
  follow_link: true,
  switch_keep_on_top: true,
  switch_full_screen: true,
  editor_choices: true,
  stop_document: true,
};

/** The commands the page asked the fake app for, in turn. */
let asked: Command[] = [];

/** Answers `command` as `app` says. The IPC hands the arguments over untyped. */
function answer<C extends Command>(
  app: FakeApp,
  command: C,
  args: unknown,
): AnswerOf<C> | Promise<AnswerOf<C>> | null {
  asked.push(command);
  const answering = app[command];
  if (answering) return answering(args as ArgsOf<C>);
  if (Object.hasOwn(MUST_BE_ANSWERED, command))
    return Promise.reject(
      new Error(`The fake app wasn't told how to answer ${command}.`),
    );
  return null;
}

/**
 * Stands in for the app behind Tauri's IPC: answers the page's commands as `app` says and passes
 * on the messages a test sends (`tell`). Its window is back at its first showing, with no history,
 * and it has been asked nothing yet, so that no test inherits another's.
 */
export function fakeTheApp(app: FakeApp = {}): void {
  currentShowing = 0;
  showingsIssued = 1;
  windowHistory = { back: [], forward: [], stepped: false };
  asked = [];
  mockIPC((command, args) => answer(app, command as Command, args), {
    shouldMockEvents: true,
  });
}

/** A command's failure as the app gives it: its notice as it is, never an `Error`. */
export function failingWith(notice: Notice): Promise<never> {
  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors -- Tauri rejects with the command's error as the app serialized it.
  return Promise.reject(notice);
}

/** The commands the page asked the fake app for since it started, in turn. */
export function askedOfTheApp(): Command[] {
  return [...asked];
}

/** Sends the page `message` with its `payload`, as the app does. */
export async function tell<M extends Message>(
  message: M,
  payload: Messages[M],
): Promise<void> {
  await emit(message, payload);
}

/** A message as the app's tests wrote it: the event it goes as, and its payload. */
export type MessageSample = {
  [M in Message]: { event: M; payload: Messages[M] };
}[Message];

/** Sends the page a message as the app's tests wrote it. */
export async function tellAsSampled(sample: MessageSample): Promise<void> {
  await emit(sample.event, sample.payload);
}

/**
 * The contract sample `name` in `json`, as the app's tests write them (`app/src/contract.rs`). A
 * sample the app no longer writes fails the test that asks for it.
 */
function sampleIn(json: string, name: string): unknown {
  const samples = JSON.parse(json) as Record<string, unknown>;
  if (!Object.hasOwn(samples, name)) {
    throw new Error(`The app writes no contract sample named ${name}.`);
  }
  return samples[name];
}

/** A sample of the app's answers to `render_document`, by name. */
export function renderDocumentSample(name: string): DocumentToShow {
  return sampleIn(renderDocumentSamples, name) as DocumentToShow;
}

/** A sample of the app's answers to `wait_note`, by name. */
export function waitNoteSample(name: string): WaitNote | null {
  return sampleIn(waitNoteSamples, name) as WaitNote | null;
}

/** A sample of the app's answers to `follow_link`, by name. */
export function followLinkSample(name: string): Followed {
  return sampleIn(followLinkSamples, name) as Followed;
}

/** A sample of the app's answers to `editor_choices`, with a chosen editor or without names. */
export function editorChoicesSample(
  name: "withEditor" | "withoutNames",
): EditorChoices {
  return sampleIn(editorChoicesSamples, name) as EditorChoices;
}

/** A sample of the app's answers to `switch_keep_on_top`, switched on or off. */
export function keepOnTopSample(
  name: "switchedOn" | "switchedOff",
): AnswerOf<"switch_keep_on_top"> {
  return sampleIn(keepOnTopSamples, name) as AnswerOf<"switch_keep_on_top">;
}

/** A sample of the app's answers to `switch_full_screen`, switched on or off. */
export function fullScreenSample(
  name: "switchedOn" | "switchedOff",
): AnswerOf<"switch_full_screen"> {
  return sampleIn(fullScreenSamples, name) as AnswerOf<"switch_full_screen">;
}

/** A sample of the app's answers to `stop_document`, stopped or not. */
export function stopDocumentSample(
  name: "stopped" | "notStopped",
): AnswerOf<"stop_document"> {
  return sampleIn(stopDocumentSamples, name) as AnswerOf<"stop_document">;
}

/**
 * A sample of `follow_link`'s failure for a link to a missing file, whose folder is there or gone
 * too.
 */
export function followLinkFailureSample(
  name: "notFound" | "notFoundInGoneFolder",
): Notice {
  return sampleIn(followLinkSamples, name) as Notice;
}

/** The sample of `reveal_document`'s failure for a missing file. */
export function revealFailureSample(): Notice {
  return sampleIn(titleBarSamples, "revealNotFound") as Notice;
}

/** The sample of `open_named_folder`'s failure for a folder gone since its notice offered it. */
export function openFolderFailureSample(): Notice {
  return sampleIn(openNamedFolderSamples, "notFound") as Notice;
}

/** A sample of the app's messages, by name. */
export function messageSample(name: string): MessageSample {
  return sampleIn(messageSamples, name) as MessageSample;
}

/**
 * Every sample of the menu entries the app's `show_document_menu` takes: each kind of entry, and
 * a command with and without its optional shortcut. All of them, so that a kind the app adds
 * reaches the page's tests without a name to ask for.
 */
export function menuEntrySamples(): MenuEntry[] {
  return Object.values(
    JSON.parse(menuEntrySamplesJson) as object,
  ) as MenuEntry[];
}

/**
 * Every change of a setting the app's `change_setting` takes. All of them, so that one the app
 * adds reaches the page's tests without a name to ask for.
 */
export function settingChangeSamples(): SettingChange[] {
  return Object.values(
    JSON.parse(settingChangeSamplesJson) as object,
  ) as SettingChange[];
}

/** The timing reports the app takes, by the name of their sample. */
type TimingReports = {
  firstPaint: FirstPaintMarks;
  reload: ReloadMarks;
  longTask: LongTask;
  longFrame: LongTask;
};

/** A sample of a timing report the app takes, by name. */
export function timingReportSample<N extends keyof TimingReports>(
  name: N,
): TimingReports[N] {
  return sampleIn(timingReportSamples, name) as TimingReports[N];
}

/**
 * The page's URL as the app opens it, with every flag on (`allOn`) or none (`allOff`), both with
 * the settings of the `settings` message's sample, or with no flag and the default settings
 * (`defaults`).
 */
export function pageUrlSample(name: "allOn" | "allOff" | "defaults"): string {
  return sampleIn(pageUrlSamples, name) as string;
}

/**
 * Opens the page at `url`, relative to the page's origin, as the app does, so the page reads its
 * flags from it. The tests open it at `/` again after each test.
 */
export function openPageAt(url: string): void {
  history.replaceState(null, "", `/${url}`);
}

/**
 * Opens the page with the flags named on, as `page_url` in `app/src/window.rs` writes them: the
 * flags' names are the page's own (`PageFlags`), which the contract sample keeps the app's.
 */
export function openPageWith(...flags: (keyof PageFlags)[]): void {
  openPageAt(flags.length === 0 ? "" : `?${flags.join("&")}`);
}

/** The title bar's arguments the app takes, by the name of their sample. */
type TitleBarArguments = { maximizeButton: ButtonRect };

/** A sample of an argument the app takes from the title bar, by name. */
export function titleBarSample<N extends keyof TitleBarArguments>(
  name: N,
): TitleBarArguments[N] {
  return sampleIn(titleBarSamples, name) as TitleBarArguments[N];
}

/**
 * Gives the fake app's window another document, as Ctrl+O, a drop or a link does, and tells the
 * page, which then asks for it. Returns the showing the window left.
 */
export async function giveAnotherDocument(): Promise<number> {
  const left = currentShowing;
  windowHistory.back.push(left);
  windowHistory.forward = [];
  windowHistory.stepped = false;
  currentShowing = showingsIssued++;
  await tell("ask-for-document", null);
  return left;
}

/**
 * Answers `step_history` as the app does (`Documents::step`): gives the fake app's window the
 * showing it went back or forward to and tells the page, which then asks for it, or does nothing
 * when there is nowhere to go. Resolves once the page has been told.
 */
export async function stepAsTheApp({
  step,
}: ArgsOf<"step_history">): Promise<null> {
  const [to, from] =
    step === "back"
      ? [windowHistory.back, windowHistory.forward]
      : [windowHistory.forward, windowHistory.back];
  const next = to.pop();
  if (next === undefined) return null;
  from.push(currentShowing);
  currentShowing = next;
  windowHistory.stepped = true;
  await tell("ask-for-document", null);
  return null;
}

/**
 * Answers `forget_forward` as the app does (`Documents::forget_forward`): the fake app's window,
 * while it is at `showing`, has nowhere to go forward to any more.
 */
export function forgetForwardAsTheApp({
  showing,
}: ArgsOf<"forget_forward">): null {
  if (showing === currentShowing) windowHistory.forward = [];
  return null;
}

/**
 * Tells the page that the document's file was saved, as the app's watch does: its new render is
 * ready to ask for.
 */
export async function saveTheFile(): Promise<void> {
  await tell("ask-for-document", null);
}

/**
 * Tells the page that the file of the document the window shows as `showing` is gone, as the
 * app's watch does; by default the showing the window is at now.
 */
export async function deleteTheFile(
  showing: number = currentShowing,
): Promise<void> {
  await tell("document-gone", showing);
}

/**
 * Tells the page to scroll the document the window shows as `showing` to `heading`, as the app
 * does for a link in another window; by default the showing the window is at now.
 */
export async function scrollAsTheApp(
  heading: string,
  showing: number = currentShowing,
): Promise<void> {
  await tell("scroll-to-heading", { showing, heading });
}

/**
 * The file of every document `answerAsTheApp` hands over, and of a test's document whose file
 * doesn't matter to it.
 */
export const SHOWN_FILE: DocumentFile = {
  name: "a.md",
  path: "C:\\docs\\a.md",
};

/**
 * Answers `render_document` as the app does (`Documents::handover`): a page whose view shows the
 * current showing reloads its document, and any other page returns to it after Back or Forward,
 * or else shows it, at its heading or the top. Decided when the page asks, before the read, as
 * the app does. `args` are the request's.
 */
export async function answerAsTheApp(
  args: ArgsOf<"render_document">,
  read: Read,
): Promise<DocumentToShow> {
  const showing = currentShowing;
  const reloads = args.showing === showing;
  const { stepped } = windowHistory;
  const placeAt = (heading: string | null): Place =>
    reloads
      ? { place: "reload" }
      : stepped
        ? { place: "return" }
        : { place: "show", heading };
  const steps = {
    back: windowHistory.back.length > 0,
    forward: windowHistory.forward.length > 0,
  };
  const handedOver = { showing, file: SHOWN_FILE, notice: null, steps };
  let content: Awaited<ReturnType<Read>>;
  try {
    content = await read();
  } catch (error) {
    const why = error instanceof Error ? error.message : String(error);
    return { ...handedOver, ...placeAt(null), error: why };
  }
  if (content === null) return null;
  return typeof content === "string"
    ? { ...handedOver, ...placeAt(null), document: content }
    : {
        ...handedOver,
        ...placeAt(content.heading),
        document: content.document,
      };
}

/**
 * Answers `wait_note` as the app does (`wait_note` in `app/src/documents/handover.rs`): `note`
 * while the page waits for a document its view doesn't show, and none for a reload, whose page
 * keeps its document in view until the new render replaces it. `args` are the request's.
 */
export function waitNoteAsTheApp(
  args: ArgsOf<"wait_note">,
  note: WaitNote,
): WaitNote | null {
  return args.showing === currentShowing ? null : note;
}

/**
 * Takes the next of `answers`, which a test lists in the order the page will ask for them. The
 * last one stays and answers every request after it, since a test can't always tell how often the
 * page asks, such as when a reload overtakes another. None left is undefined. Reads the array it
 * is given at each call, so a test can set its answers after the fake app is set up.
 */
export function nextInTurn<T>(answers: T[]): T | undefined {
  return answers.length > 1 ? answers.shift() : answers[0];
}

/**
 * A `render_document` call that fails before the app can answer it, so `invoke` itself rejects,
 * with why as Tauri gives it.
 */
export interface NoAnswer {
  noAnswer: string;
}

/** The fake app of the document view's tests (`fakeTheAppReading`). */
export interface FakeDocumentApp {
  /** The page's text when it asked the app to show its window, or undefined until then. */
  textWhenShown: () => string | undefined;
}

/**
 * Stands in for the app for the document view's tests (`fakeTheApp`): `render_document` answers
 * as the app does (`answerAsTheApp`) with the next of `documents()` in turn (`nextInTurn`), the
 * HTML, the HTML at a heading, or null for no document, or gives no answer at all; `show_window`
 * keeps the page's text; `app` answers the rest. `documents` is read at each request, so a test
 * sets its reads after the fake app is set up.
 */
export function fakeTheAppReading(
  documents: () => (Read | NoAnswer)[],
  app: FakeApp = {},
): FakeDocumentApp {
  let textWhenShown: string | undefined;
  fakeTheApp({
    show_window: () => {
      textWhenShown = document.body.textContent;
      return null;
    },
    render_document: (args) => {
      const next = nextInTurn(documents());
      if (typeof next === "object")
        // Tauri rejects a call it can't hand to the command (a denied or malformed one) with a
        // string, not an Error.
        // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors -- as Tauri does
        return Promise.reject(next.noAnswer);
      // No document given yet answers as a window that shows none.
      return next ? answerAsTheApp(args, next) : null;
    },
    ...app,
  });
  return { textWhenShown: () => textWhenShown };
}

/** The element the document scrolls in, below the title bar. */
export function scroller(): HTMLElement | null {
  return document.querySelector<HTMLElement>(SCROLLER_SELECTOR);
}

/** Lets the page's pending IPC calls, such as starting to listen, finish. */
export function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/** Resolves once the next shown document is in place, as `whenDocumentShown` says. */
export function nextShown(): Promise<void> {
  return new Promise((resolve) => {
    const stop = whenDocumentShown(() => {
      stop();
      resolve();
    });
  });
}

/** The text of the shown document's first-level heading, if it has one. */
export function heading(): string | null | undefined {
  return document.querySelector("article.markdown-body h1")?.textContent;
}

/**
 * The document view's zoom, as the factor its stylesheet zooms it by (`ZOOM_VARIABLE` in
 * `zoom.ts`), such as "1.25"; happy-dom doesn't apply the stylesheet.
 */
export function documentZoom(): string {
  return document.documentElement.style.getPropertyValue(ZOOM_VARIABLE);
}

/** What the document view shows while the window has no document. */
export function emptyWindow(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".empty-window");
}

/** The 'File deleted or moved' bar over the document. */
export function goneBar(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".file-gone");
}

/** The wait note, which says the window's document is on its way (ticket 31), while it shows. */
export function shownWaitNote(): HTMLElement | null {
  const note = document.querySelector<HTMLElement>(".wait-note");
  return note && !note.hidden ? note : null;
}

/**
 * How often the page has scrolled the document to its top since the spy started: its scroller,
 * which happy-dom doesn't lay out, to 0, 0.
 */
export function spyOnScrollsToTop(): () => number {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollTo")
    .mockImplementation(() => {});
  return () =>
    scrolls.mock.calls.filter(
      (args, index) =>
        args[0] === 0 &&
        args[1] === 0 &&
        (scrolls.mock.contexts[index] as Element).matches(SCROLLER_SELECTOR),
    ).length;
}

/** The ids of the elements the page scrolled to the top of the view, in turn. */
export function spyOnHeadingScrolls(): () => string[] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () => scrolls.mock.contexts.map((element) => (element as Element).id);
}

/** The first and last line of the file that `element` comes from, by its `data-sourcepos`. */
export function linesOf(element: Element): [number, number] | undefined {
  const own = element.getAttribute("data-sourcepos");
  if (own) {
    const [start, end] = own.split("-").map((point) => Number.parseInt(point));
    return [start ?? 0, end ?? 0];
  }
  // A wrapper, such as a code block's, sits where the block inside it does; inline text sits in
  // its block.
  const inside = element.querySelector("[data-sourcepos]");
  const around = element.parentElement?.closest("[data-sourcepos]");
  const block = inside ?? around;
  return block ? linesOf(block) : undefined;
}

/**
 * Stands in for happy-dom's `ResizeObserver`, which never reports, until the test's globals are
 * unstubbed. Returns a function that lays the title bar out, as the webview does when the window's
 * size, its DPI or the zoom changes: every observer reports.
 */
export function fakeTitleBarLayout(): () => void {
  const observers: ResizeObserverCallback[] = [];
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(report: ResizeObserverCallback) {
        observers.push(report);
      }
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    },
  );
  return () => {
    for (const report of observers) report([], {} as ResizeObserver);
  };
}

/** The fake app's window of the title bar's tests (`openWithTitleBar`). */
export interface TitleBarWindow {
  /** Each `place_maximize_button` request's arguments, in turn. */
  placed: ArgsOf<"place_maximize_button">[];
  /** Lays the title bar out, as the webview does when the window's size or DPI changes. */
  layOut: () => void;
}

/**
 * Opens the page as a window whose title bar the page draws (the app's `titleBar` flag), with its
 * settings started, and the fake app answering as `app` says: by default `editor_choices` for the
 * settings flyout, and `place_maximize_button` by keeping each place it is sent. For the title
 * bar's tests, whose `afterEach` calls `closeTitleBarWindow`.
 */
export async function openWithTitleBar(
  app: FakeApp = {},
): Promise<TitleBarWindow> {
  mockWindows("document-0");
  const placed: ArgsOf<"place_maximize_button">[] = [];
  const layOut = fakeTitleBarLayout();
  fakeTheApp({
    editor_choices: () => editorChoicesSample("withoutNames"),
    place_maximize_button: (args) => {
      placed.push(args);
      return null;
    },
    ...app,
  });
  openPageWith("titleBar");
  await startSettings();
  return { placed, layOut };
}

/** Closes what `openWithTitleBar` opened, so that no test inherits another's window. */
export function closeTitleBarWindow(): void {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  openPageWith();
  document.body.replaceChildren();
}

/**
 * The app's answer for a window that shows `file`, its `showing`th document, rendered as
 * `rendered`, or as a heading with a link to `guide.md` by default.
 */
export function shownFrom(
  file: DocumentFile,
  showing = 0,
  rendered = `<h1>${file.name}</h1>\n<p><a href="guide.md">The guide</a></p>\n`,
): DocumentToShow {
  return {
    showing,
    file,
    notice: null,
    steps: { back: false, forward: false },
    place: "show",
    heading: null,
    document: rendered,
  };
}

/** A command's line of the right-click menu. */
type CommandEntry = Extract<MenuEntry, { kind: "command" }>;

/**
 * The entry of the command `id` among the right-click menu's `entries`, as the page hands them to
 * the app's `show_document_menu`, or undefined if the menu has none.
 */
export function menuEntryOf(
  entries: MenuEntry[],
  id: string,
): CommandEntry | undefined {
  return entries.find(
    (entry): entry is CommandEntry =>
      entry.kind === "command" && entry.id === id,
  );
}

/** The title bar's logo. */
export function titleBarLogo(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".title-bar .logo");
  if (!found) throw new Error("The title bar has no logo.");
  return found;
}

/** The title bar's file name, which names the shown document's file. */
export function titleBarFileName(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".title-bar .file-name");
  if (!found) throw new Error("The title bar has no file name.");
  return found;
}

/** The app's name the title bar shows in the file name's place in the empty window (ticket 14). */
export function titleBarAppName(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".title-bar .app-name");
  if (!found) throw new Error("The title bar has no app name.");
  return found;
}

/** The title bar's zoom level beside the file name (window design ticket 04). */
export function titleBarZoomLevel(): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>(
    ".title-bar .zoom-level",
  );
  if (!found) throw new Error("The title bar has no zoom level.");
  return found;
}

/** The title bar's button that a screen reader names `label`. */
export function titleBarButton(label: string): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>(
    `.title-bar button[aria-label="${label}"]`,
  );
  if (!found) throw new Error(`The title bar has no ${label} button.`);
  return found;
}

/**
 * The title bar's buttons right of the file name, left to right, each by the name a screen reader
 * gives it, with "|" for a separator between groups (window design ticket 05).
 */
export function titleBarButtonRow(): string[] {
  return [
    ...document.querySelectorAll<HTMLElement>(
      ".title-bar :is(.command-button, .window-button, .separator)",
    ),
  ].map((item) =>
    item.classList.contains("separator")
      ? "|"
      : (item.getAttribute("aria-label") ?? ""),
  );
}

/**
 * Whether the title bar shows, rather than being hidden away at the top, as it is in full screen
 * (full screen ticket 03). happy-dom lays nothing out, so this is the state the page sets.
 */
export function titleBarShows(): boolean {
  const bar = document.querySelector<HTMLElement>(".title-bar");
  if (!bar) throw new Error("The page draws no title bar.");
  return !bar.classList.contains("away");
}

/**
 * The room the page keeps for the title bar at the top, which the view and the find bar start
 * below, when the page sets it itself, as in full screen (full screen ticket 03); undefined while
 * it leaves it to the stylesheet, which keeps the bar's height.
 */
export function titleBarRoom(): string | undefined {
  return (
    document.documentElement.style.getPropertyValue("--title-bar-room") ||
    undefined
  );
}

/**
 * The thin zone at the top edge of the screen whose pointer slides the hidden title bar in, in
 * full screen (full screen ticket 04).
 */
export function titleBarTopZone(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".title-bar .top-zone");
  if (!found) throw new Error("The title bar has no top zone.");
  return found;
}

/** `element` and its ancestors, innermost first. */
function withAncestors(element: Element | null): Element[] {
  const chain: Element[] = [];
  for (let at = element; at; at = at.parentElement) chain.push(at);
  return chain;
}

/**
 * Moves the pointer from over `from` to over `to` (null: outside the page), as the browser tells
 * it, which happy-dom doesn't: `pointerleave` on each element left, innermost first, then
 * `pointerenter` on each element entered, outermost first.
 */
export function movePointer(from: Element | null, to: Element | null): void {
  const left = withAncestors(from);
  const entered = withAncestors(to);
  for (const element of left.filter((at) => !entered.includes(at)))
    element.dispatchEvent(new PointerEvent("pointerleave"));
  for (const element of entered.filter((at) => !left.includes(at)).reverse())
    element.dispatchEvent(new PointerEvent("pointerenter"));
}

/** The height of the title bar the page draws (window design ticket 01), in px. */
export const TITLE_BAR_HEIGHT = 32;

/**
 * Lays the document out, which happy-dom doesn't: each line of the file is 20 px high, and
 * `topLine` is at the top of the view (1.5 is halfway down the first line). As in the window, the
 * document's scroller runs from below the title bar to the window's bottom, so the view's top is
 * not the window's. A range sits where the element holding its start does.
 *
 * The document's raw HTML has no line numbers, so it is laid out on the lines `rawHtml` gives
 * the first element that matches each selector, as the file has it.
 */
export function layOut(
  topLine: number,
  rawHtml: Record<string, [number, number]> = {},
): void {
  const scroller = new DOMRect(
    0,
    TITLE_BAR_HEIGHT,
    800,
    innerHeight - TITLE_BAR_HEIGHT,
  );
  const rawLines = (element: Element) =>
    Object.entries(rawHtml).find(
      ([selector]) => document.querySelector(selector) === element,
    )?.[1];
  const rectOf = (element: Element | null) => {
    if (element?.matches(SCROLLER_SELECTOR)) return scroller;
    const [start, end] = (element &&
      (rawLines(element) ?? linesOf(element))) ?? [topLine, topLine - 1];
    const top = TITLE_BAR_HEIGHT + (start - topLine) * 20;
    return new DOMRect(0, top, 800, (end - start + 1) * 20);
  };
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
    function (this: Element) {
      return rectOf(this);
    },
  );
  vi.spyOn(Range.prototype, "getBoundingClientRect").mockImplementation(
    function (this: Range) {
      return rectOf(this.startContainer.parentElement);
    },
  );
}
