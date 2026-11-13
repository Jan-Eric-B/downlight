// The page's side of the contract samples (ticket 27), the counterpart to `app/src/contract.rs`:
// each answer and message as the app's tests wrote them (`app/tests/fixtures/`) goes through the
// page here, and what the page sends the app is checked against the app's samples of what it
// takes, so that a change to a shape on either side fails a test.
import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { ArgsOf, MenuEntry } from "./app";
import { startCommands } from "./commands";
import { startLinks } from "./links";
import {
  askedOfTheApp,
  emptyWindow,
  fakeTheApp,
  followLinkSample,
  goneBar,
  heading,
  menuEntrySamples,
  messageSample,
  nextInTurn,
  renderDocumentSample,
  settle,
  spyOnHeadingScrolls,
  tellAsSampled,
  timingReportSample,
} from "./test-support";
import {
  requestDocument,
  startDocumentView,
  whenDocumentShown,
  type ShownReason,
} from "./view";

/** Each `render_document` request's arguments, in turn. */
let requested: ArgsOf<"render_document">[] = [];

/** Each `follow_link` request's arguments, in turn. */
let follows: ArgsOf<"follow_link">[] = [];

let stopPage: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  requested = [];
  follows = [];
  stopPage = new AbortController();
});

afterEach(() => {
  stopPage.abort();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  clearMocks();
  document.body.replaceChildren();
  history.replaceState(null, "", "/");
});

/** Answers `render_document` with the samples named, in turn; the last one repeats. */
function answerWithSamples(...names: string[]): void {
  const answers = names.map(renderDocumentSample);
  fakeTheApp({
    render_document: (args) => {
      requested.push(args);
      return nextInTurn(answers) ?? null;
    },
  });
}

/** The showing in the `render_document` sample `name`. */
function showingIn(name: string): number {
  const sample = renderDocumentSample(name);
  if (sample === null) throw new Error(`The sample ${name} has no showing.`);
  return sample.showing;
}

/** How often the page has asked the app for its document. */
function requests(): number {
  return askedOfTheApp().filter((command) => command === "render_document")
    .length;
}

// `render_document.json`

test("the app's answer to show a document shows it", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);

  expect(heading()).toBe("Install");
  expect(document.querySelector("article")?.textContent).toContain(
    "Run the installer.",
  );
});

test("the app's answer to show a document at a heading scrolls to it", async () => {
  answerWithSamples("showAtHeading");
  const scrolledTo = spyOnHeadingScrolls();

  await startDocumentView(0);
  expect(new Set(scrolledTo())).toEqual(new Set(["install"]));
});

test("the app's answer to reload the document reloads it", async () => {
  answerWithSamples("showAtTop", "reload");
  await startDocumentView(0);
  await settle();
  const reasons: ShownReason[] = [];
  const stop = whenDocumentShown((_view, reason) => {
    reasons.push(reason);
  });

  await requestDocument();
  stop();
  expect(reasons).toEqual(["reload"]);
  expect(heading()).toBe("Install");
  // The showing the page sends back is how the app tells this reload from a show.
  expect(requested).toEqual([
    { showing: null },
    { showing: showingIn("showAtTop") },
  ]);
});

test.each(["showAtTop", "showAtHeading", "reload"])(
  "the page sends the showing of the app's %s answer with its next request",
  async (name) => {
    answerWithSamples(name);
    await startDocumentView(0);
    await settle();

    await requestDocument();
    expect(requested.at(-1)).toEqual({ showing: showingIn(name) });
  },
);

test("the app's answer that a document can't be read shows why", async () => {
  answerWithSamples("unreadShow");
  await startDocumentView(0);

  expect(document.querySelector("article")?.textContent).toContain(
    "Can't read C:\\docs\\a.md",
  );
});

test("the app's answer that a reload can't be read keeps the document and says why", async () => {
  answerWithSamples("showAtTop", "unreadReload");
  await startDocumentView(0);
  await settle();

  await requestDocument();
  expect(heading()).toBe("Install");
  expect(document.querySelector(".notice")?.textContent).toContain(
    "Can't read C:\\docs\\a.md",
  );
});

test("the app's answer that the window shows no document offers to open one", async () => {
  answerWithSamples("noDocument");
  await startDocumentView(0);

  expect(emptyWindow()?.hidden).toBe(false);
});

// `messages.json`

test.each(["documentShown", "reload"])(
  "the app's %s message makes the page ask for its document",
  async (name) => {
    answerWithSamples("showAtTop");
    await startDocumentView(0);
    await settle();
    const before = requests();

    await tellAsSampled(messageSample(name));
    expect(requests()).toBe(before + 1);
  },
);

test("the app's message that the file is gone shows the bar", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);
  await settle();

  await tellAsSampled(messageSample("gone"));
  expect(goneBar()?.hidden).toBe(false);
});

test("the app's message to scroll to a heading scrolls to it", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();

  await tellAsSampled(messageSample("scrollToHeading"));
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("install");
  });
});

test("the app's notice shows", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);
  await settle();
  const notice = messageSample("notice");

  await tellAsSampled(notice);
  expect(document.querySelector(".notice")?.textContent).toBe(notice.payload);
});

test("the app's message that the reader chose a menu command runs it", async () => {
  fakeTheApp();
  startCommands(stopPage.signal);
  await settle();

  // The sample names Open.
  await tellAsSampled(messageSample("menuCommand"));
  expect(askedOfTheApp()).toEqual(["open_document"]);
});

// `follow_link.json`

/**
 * Shows the sampled document, whose heading links to itself, and answers following a link with
 * the `follow_link` sample `name`. Returns the heading's link.
 */
async function showLinkFollowedAs(name: string): Promise<HTMLAnchorElement> {
  fakeTheApp({
    render_document: () => renderDocumentSample("showAtTop"),
    follow_link: (args) => {
      follows.push(args);
      return followLinkSample(name);
    },
  });
  startLinks(stopPage.signal);
  await startDocumentView(0);
  await settle();
  const link = document.querySelector<HTMLAnchorElement>(
    'article.markdown-body a[href="#install"]',
  );
  if (!link) throw new Error("The sampled document has no link to #install.");
  return link;
}

test("the app's scroll to a heading scrolls there", async () => {
  const link = await showLinkFollowedAs("scrollToHeading");
  const scrolledTo = spyOnHeadingScrolls();

  link.click();
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("install");
  });
});

test("the app's scroll to the top scrolls to the top", async () => {
  const link = await showLinkFollowedAs("scrollToTop");
  const scrolls = vi.spyOn(window, "scrollTo").mockImplementation(() => {});

  link.click();
  await vi.waitFor(() => {
    expect(scrolls).toHaveBeenCalledWith(0, 0);
  });
});

test("the app's nothing left to do leaves the view where it is", async () => {
  const link = await showLinkFollowedAs("nothing");
  const scrolledTo = spyOnHeadingScrolls();
  const scrolls = vi.spyOn(window, "scrollTo").mockImplementation(() => {});

  link.click();
  await settle();
  await settle();
  expect(follows).toHaveLength(1);
  expect(scrolledTo()).toEqual([]);
  expect(scrolls).not.toHaveBeenCalled();
});

/**
 * The keys of `value` that reach the app. The IPC sends JSON, which leaves out a key whose value
 * is undefined, such as the shortcut of a command that has none.
 */
function keysSent(value: object): string[] {
  return Object.entries(value)
    .filter(([, field]) => field !== undefined)
    .map(([key]) => key)
    .sort();
}

/** The kind of `value` and the keys it reaches the app with, such as `separator: kind`. */
function shapeOf(value: { kind: string }): string {
  return `${value.kind}: ${keysSent(value).join(", ")}`;
}

// `menu_entries.json`

test("the page's menu entries are the kinds the app takes, each with the keys it takes", async () => {
  const sent: MenuEntry[] = [];
  fakeTheApp({
    show_document_menu: ({ entries }) => {
      sent.push(...entries);
      return null;
    },
  });
  startCommands(stopPage.signal);

  document.body.dispatchEvent(
    new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
  );
  await settle();
  const samples = menuEntrySamples();
  expect(new Set(sent.map((entry) => entry.kind))).toEqual(
    new Set(samples.map((sample) => sample.kind)),
  );
  // A command's shortcut is optional, so an entry matches any sample of its kind.
  const sampled = samples.map(shapeOf);
  for (const entry of sent) expect(sampled).toContain(shapeOf(entry));
});

// `timing_reports.json`. The timing switch is read when `timing.ts` loads, so these tests load the
// page's modules afresh with it on, as the app does by opening the page with `?timing`.

/** The timing reports the page sent the app, by command, in turn. */
interface TimingReports {
  firstPaint: ArgsOf<"report_first_paint">[];
  reload: ArgsOf<"report_reload">[];
  longTask: ArgsOf<"report_long_task">[];
}

/**
 * Loads the document view and the timing reports with the timing switch on, and a fake app that
 * answers `render_document` with the samples named, in turn, and keeps the reports it is sent.
 */
async function loadTimedPage(...answerNames: string[]) {
  history.replaceState(null, "", "/?timing");
  vi.resetModules();
  const reports: TimingReports = { firstPaint: [], reload: [], longTask: [] };
  const answers = answerNames.map(renderDocumentSample);
  fakeTheApp({
    render_document: () => nextInTurn(answers) ?? null,
    report_first_paint: (args) => {
      reports.firstPaint.push(args);
      return null;
    },
    report_reload: (args) => {
      reports.reload.push(args);
      return null;
    },
    report_long_task: (args) => {
      reports.longTask.push(args);
      return null;
    },
  });
  return {
    reports,
    view: await import("./view"),
    timing: await import("./timing"),
  };
}

test("the page's first paint report has the marks the app takes", async () => {
  const { reports, view } = await loadTimedPage("showAtTop");
  await view.startDocumentView(0);

  expect(reports.firstPaint.map(({ marks }) => keysSent(marks))).toEqual([
    keysSent(timingReportSample("firstPaint")),
  ]);
});

test("the page's reload report has the marks the app takes", async () => {
  const { reports, view } = await loadTimedPage("showAtTop", "reload");
  await view.startDocumentView(0);
  await settle();

  await tellAsSampled(messageSample("reload"));
  await vi.waitFor(() => {
    expect(reports.reload.map(({ marks }) => keysSent(marks))).toEqual([
      keysSent(timingReportSample("reload")),
    ]);
  });
});

/** What a `PerformanceObserver` hands its callback, as far as the page reads it. */
type ObservedEntries = (list: {
  getEntries(): { startTime: number; duration: number }[];
}) => void;

/**
 * Stands in for the webview's `PerformanceObserver`, whose long task and long frame entries
 * happy-dom doesn't have: each observer is handed one entry that took over 100 ms.
 */
class SlowPageObserver {
  static readonly supportedEntryTypes = ["longtask", "long-animation-frame"];
  readonly #observed: ObservedEntries;

  constructor(observed: ObservedEntries) {
    this.#observed = observed;
  }

  observe(): void {
    queueMicrotask(() => {
      this.#observed({ getEntries: () => [{ startTime: 250, duration: 150 }] });
    });
  }
}

test("the page's long task and long frame reports are the kinds the app takes, with its keys", async () => {
  vi.stubGlobal("PerformanceObserver", SlowPageObserver);
  const { reports, timing } = await loadTimedPage();

  timing.watchLongTasks();
  const sampled = [
    timingReportSample("longTask"),
    timingReportSample("longFrame"),
  ].map(shapeOf);
  await vi.waitFor(() => {
    expect(reports.longTask.map(({ task }) => shapeOf(task)).sort()).toEqual(
      sampled.sort(),
    );
  });
});
