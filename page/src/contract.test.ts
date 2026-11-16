// The page's side of the contract samples (ticket 27), the counterpart to `app/src/contract.rs`:
// each answer and message as the app's tests wrote them (`app/tests/fixtures/`) goes through the
// page here, what the page sends the app is checked against the app's samples of what it takes,
// and the page reads its flags from the app's sampled URLs, so that a change to a shape on either
// side fails a test.
import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { pageFlags, type ArgsOf, type MenuEntry, type NamedFile } from "./app";
import { startCommands } from "./commands";
import { startLinks } from "./links";
import { showNotice } from "./notice";
import { startTitleBar } from "./title-bar";
import {
  askedOfTheApp,
  emptyWindow,
  failingWith,
  fakeTheApp,
  fakeTitleBarLayout,
  followLinkFailureSample,
  followLinkSample,
  goneBar,
  heading,
  menuEntrySamples,
  messageSample,
  nextInTurn,
  openFolderFailureSample,
  openPageAt,
  openPageWith,
  pageUrlSample,
  renderDocumentSample,
  revealFailureSample,
  settle,
  spyOnHeadingScrolls,
  spyOnScrollsToTop,
  tellAsSampled,
  timingReportSample,
  titleBarButton,
  titleBarFileName,
  titleBarSample,
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

/** Each opening of a notice's folder the page asked the app for, by the folder it sent. */
let foldersOpened: string[] = [];

let stopPage: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  requested = [];
  follows = [];
  foldersOpened = [];
  stopPage = new AbortController();
});

afterEach(() => {
  stopPage.abort();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  clearMocks();
  document.body.replaceChildren();
  openPageWith();
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
  const stop = whenDocumentShown(({ reason }) => {
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

/** The file the `render_document` sample `name` says can't be read. */
function unreadFileIn(name: string): NamedFile {
  const sample = renderDocumentSample(name);
  if (
    sample === null ||
    !("error" in sample) ||
    typeof sample.error === "string"
  )
    throw new Error(`The sample ${name} names no file it can't read.`);
  return sample.error.file;
}

/** Answers `open_named_folder` by keeping the folder the page sent. */
function openFolder({ folder }: ArgsOf<"open_named_folder">): null {
  foldersOpened.push(folder);
  return null;
}

/** The link that opens a folder in `container`, if it has one. */
function folderLinkIn(container: Element | null): HTMLElement | null {
  return container?.querySelector<HTMLElement>('[role="link"]') ?? null;
}

test("the app's answer that a document can't be read shows why, naming its file and opening its folder", async () => {
  const unread = unreadFileIn("unreadShow");
  fakeTheApp({
    render_document: () => renderDocumentSample("unreadShow"),
    open_named_folder: openFolder,
  });
  await startDocumentView(0);

  const view = document.querySelector("article");
  expect(view?.textContent).toContain(
    `Can't read ${unread.name} in ${unread.folder ?? ""}`,
  );
  expect(view?.querySelector("strong")?.textContent).toBe(unread.name);
  folderLinkIn(view)?.click();
  await vi.waitFor(() => {
    expect(foldersOpened).toEqual([unread.folder]);
  });
});

test("the app's answer that a reload can't be read keeps the document and says why", async () => {
  const unread = unreadFileIn("unreadReload");
  answerWithSamples("showAtTop", "unreadReload");
  await startDocumentView(0);
  await settle();

  await requestDocument();
  expect(heading()).toBe("Install");
  expect(document.querySelector(".notice")?.textContent).toContain(
    `Can't read ${unread.name} in ${unread.folder ?? ""}`,
  );
});

test("the app's answer to return to a document shows it, heard as a return", async () => {
  answerWithSamples("showAtTop", "return");
  await startDocumentView(0);
  await settle();
  const reasons: ShownReason[] = [];
  const stop = whenDocumentShown(({ reason }) => {
    reasons.push(reason);
  });

  await requestDocument();
  stop();
  expect(reasons).toEqual(["return"]);
  expect(heading()).toBe("Install");
});

test("the app's answer that a document returned to can't be read shows why", async () => {
  const unread = unreadFileIn("unreadReturn");
  answerWithSamples("showAtTop", "unreadReturn");
  await startDocumentView(0);
  await settle();

  await requestDocument();
  expect(document.querySelector("article")?.textContent).toContain(
    `Can't read ${unread.name} in ${unread.folder ?? ""}`,
  );
});

test.each([
  ["showAtTop", false],
  ["return", true],
])(
  "the app's %s answer offers Back and Forward in the menu as its steps say",
  async (name, offered) => {
    let entries: MenuEntry[] = [];
    fakeTheApp({
      render_document: () => renderDocumentSample(name),
      show_document_menu: (args) => {
        entries = args.entries;
        return null;
      },
    });
    startCommands(stopPage.signal);
    await startDocumentView(0);

    document.body.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
    );
    await settle();
    const enabled = (id: string) =>
      entries.some(
        (entry) => entry.kind === "command" && entry.id === id && entry.enabled,
      );
    expect([enabled("back"), enabled("forward")]).toEqual([offered, offered]);
  },
);

test("the app's answer that the window shows no document offers to open one", async () => {
  answerWithSamples("noDocument");
  await startDocumentView(0);

  expect(emptyWindow()?.hidden).toBe(false);
});

test.each(["showAtTop", "unreadShow"])(
  "the app's %s answer names the document's file in the title bar",
  async (name) => {
    fakeTitleBarLayout();
    openPageWith("titleBar");
    answerWithSamples(name);
    startTitleBar(stopPage.signal);
    await startDocumentView(0);

    const sample = renderDocumentSample(name);
    expect(titleBarFileName().textContent).toBe(sample?.file.name);
    expect(titleBarFileName().title).toBe(sample?.file.path);
  },
);

// `messages.json`

test("the app's askForDocument message makes the page ask for its document", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);
  await settle();
  const before = requests();

  await tellAsSampled(messageSample("askForDocument"));
  expect(requests()).toBe(before + 1);
});

test("the app's message that the file of the showing in view is gone shows the bar", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);
  await settle();
  const gone = messageSample("gone");

  await tellAsSampled(gone);
  expect(gone.payload).toBe(showingIn("showAtTop"));
  expect(goneBar()?.hidden).toBe(false);
});

test("the app's held notice in its answer shows with the document", async () => {
  // Ticket 26: in the answer, since a notice sent before the page's listener is registered is
  // dropped.
  answerWithSamples("showWithHeldNotice");
  const sample = renderDocumentSample("showWithHeldNotice");
  if (sample?.notice == null || typeof sample.notice === "string")
    throw new Error("The sample holds no notice naming a file.");
  const { before, file, after } = sample.notice;

  await startDocumentView(0);
  expect(heading()).toBe("Install");
  expect(document.querySelector(".notice")?.textContent).toBe(
    `${before}${file.name} in ${file.folder ?? ""}${after}`,
  );
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

test("the app's notice naming a file shows its name in bold and its folder after it", async () => {
  answerWithSamples("showAtTop");
  await startDocumentView(0);
  await settle();
  const sample = messageSample("noticeNamingFile");
  if (sample.event !== "notice" || typeof sample.payload === "string")
    throw new Error("The sample is no notice naming a file.");
  const { before, file, after } = sample.payload;

  await tellAsSampled(sample);
  const notice = document.querySelector(".notice");
  expect(notice?.textContent).toBe(
    `${before}${file.name} in ${file.folder ?? ""}${after}`,
  );
  expect(notice?.querySelector("strong")?.textContent).toBe(file.name);
});

test("the app's message that the reader chose a menu command runs it", async () => {
  fakeTheApp();
  startCommands(stopPage.signal);
  await settle();

  // The sample names Open.
  await tellAsSampled(messageSample("menuCommand"));
  expect(askedOfTheApp()).toEqual(["open_document"]);
});

/** Starts the title bar, as the page does in a window without a native frame. */
async function startTheTitleBar(): Promise<HTMLButtonElement> {
  fakeTitleBarLayout();
  openPageWith("titleBar");
  fakeTheApp();
  startTitleBar(stopPage.signal);
  await settle();
  return titleBarButton("Maximize");
}

test("the app's window states turn maximize into restore and back", async () => {
  const maximize = await startTheTitleBar();

  await tellAsSampled(messageSample("maximized"));
  expect(maximize.getAttribute("aria-label")).toBe("Restore");
  await tellAsSampled(messageSample("restored"));
  expect(maximize.getAttribute("aria-label")).toBe("Maximize");
});

test.each([
  ["maximizeButtonHovered", "hovered"],
  ["maximizeButtonPressed", "pressed"],
  ["maximizeButtonNormal", "normal"],
])("the app's %s message shows the maximize button so", async (name, look) => {
  const maximize = await startTheTitleBar();

  await tellAsSampled(messageSample(name));
  expect(maximize.dataset.state).toBe(look);
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
  const scrollsToTop = spyOnScrollsToTop();

  link.click();
  await vi.waitFor(() => {
    expect(scrollsToTop()).toBe(1);
  });
});

test("the app's nothing left to do leaves the view where it is", async () => {
  const link = await showLinkFollowedAs("nothing");
  const scrolledTo = spyOnHeadingScrolls();
  const scrollsToTop = spyOnScrollsToTop();

  link.click();
  await settle();
  await settle();
  expect(follows).toHaveLength(1);
  expect(scrolledTo()).toEqual([]);
  expect(scrollsToTop()).toBe(0);
});

test("a link names the showing of the app's answer its document came in", async () => {
  const link = await showLinkFollowedAs("nothing");

  link.click();
  await settle();
  expect(follows).toEqual([
    { showing: showingIn("showAtTop"), target: "#install", newWindow: false },
  ]);
});

/** Follows the sampled document's link, which fails as the `follow_link` failure sample `name`. */
async function followLinkFailingAs(
  name: "notFound" | "notFoundInGoneFolder",
): Promise<NamedFile> {
  const failure = followLinkFailureSample(name);
  if (typeof failure === "string")
    throw new Error(`The sample ${name} names no file.`);
  fakeTheApp({
    render_document: () => renderDocumentSample("showAtTop"),
    follow_link: () => failingWith(failure),
    open_named_folder: openFolder,
  });
  startLinks(stopPage.signal);
  await startDocumentView(0);
  await settle();
  document
    .querySelector<HTMLAnchorElement>(
      'article.markdown-body a[href="#install"]',
    )
    ?.click();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice strong")?.textContent).toBe(
      failure.file.name,
    );
  });
  return failure.file;
}

test("the app's failure for a link to a missing file opens its folder from the notice", async () => {
  const file = await followLinkFailingAs("notFound");

  folderLinkIn(document.querySelector(".notice"))?.click();
  await vi.waitFor(() => {
    expect(foldersOpened).toEqual([file.folder]);
  });
});

test("the app's failure for a link to a missing file whose folder is gone too shows the folder as text", async () => {
  const file = await followLinkFailingAs("notFoundInGoneFolder");

  const notice = document.querySelector(".notice");
  expect(notice?.textContent).toContain(`in ${file.folder ?? ""}`);
  expect(folderLinkIn(notice)).toBeNull();
});

test("the app's failure to reveal a missing file opens its folder from the notice", async () => {
  const failure = revealFailureSample();
  if (typeof failure === "string")
    throw new Error("The sample revealNotFound names no file.");
  fakeTitleBarLayout();
  openPageWith("titleBar");
  fakeTheApp({
    render_document: () => renderDocumentSample("showAtTop"),
    reveal_document: () => failingWith(failure),
    open_named_folder: openFolder,
  });
  startTitleBar(stopPage.signal);
  await startDocumentView(0);

  titleBarFileName().click();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice strong")?.textContent).toBe(
      failure.file.name,
    );
  });
  folderLinkIn(document.querySelector(".notice"))?.click();
  await vi.waitFor(() => {
    expect(foldersOpened).toEqual([failure.file.folder]);
  });
});

// `open_named_folder.json`

test("the app's failure to open a folder gone since shows why, naming it in bold", async () => {
  const offering = followLinkFailureSample("notFound");
  const failure = openFolderFailureSample();
  if (typeof failure === "string")
    throw new Error("The sample notFound names no file.");
  fakeTheApp({ open_named_folder: () => failingWith(failure) });
  showNotice(offering);

  folderLinkIn(document.querySelector(".notice"))?.click();
  const { before, file, after } = failure;
  await vi.waitFor(() => {
    expect(
      document.querySelector(".notice")?.lastElementChild?.textContent,
    ).toBe(`${before}${file.name} in ${file.folder ?? ""}${after}`);
  });
  expect(
    document.querySelector(".notice")?.lastElementChild?.querySelector("strong")
      ?.textContent,
  ).toBe(file.name);
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

// `title_bar.json`

test("the page tells the app where its maximize button is in the keys the app takes", async () => {
  const placed: ArgsOf<"place_maximize_button">[] = [];
  const layOutTitleBar = fakeTitleBarLayout();
  openPageWith("titleBar");
  fakeTheApp({
    place_maximize_button: (args) => {
      placed.push(args);
      return null;
    },
  });
  startTitleBar(stopPage.signal);

  layOutTitleBar();
  await settle();
  const [sent] = placed;
  if (!sent) throw new Error("The page didn't place its maximize button.");
  expect(keysSent(sent.button)).toEqual(
    keysSent(titleBarSample("maximizeButton")),
  );
});

// `page_url.json`

/** The flags the page reads as on. */
function flagsOn(): string[] {
  return Object.entries(pageFlags())
    .filter(([, on]) => on)
    .map(([flag]) => flag)
    .sort();
}

test("the page reads every flag in the app's URL with all flags on, and no other", () => {
  const url = pageUrlSample("allOn");
  openPageAt(url);

  const named = [...new URL(url, location.origin).searchParams.keys()];
  expect(flagsOn()).toEqual(named.sort());
  // A flag the page reads but the app never sends would read as off here.
  expect(flagsOn()).toEqual(Object.keys(pageFlags()).sort());
});

test("the page reads no flag in the app's URL with all flags off", () => {
  openPageAt(pageUrlSample("allOff"));

  expect(flagsOn()).toEqual([]);
});

// `timing_reports.json`. The timing switch is read when `timing.ts` loads, so these tests load the
// page's modules afresh with it on, as the app does by opening the page with the `timing` flag.

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
  openPageWith("timing");
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

  await tellAsSampled(messageSample("askForDocument"));
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
