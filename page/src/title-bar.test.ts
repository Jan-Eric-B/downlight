// The title bar out of full screen; title-bar-full-screen.test.ts has it in full screen.
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import {
  DEFAULT_SETTINGS,
  type AnswerOf,
  type ArgsOf,
  type DocumentCounts,
  type DocumentFile,
  type DocumentToShow,
  type WaitNote,
} from "./app";
import { startCommands } from "./commands";
import { startLinks } from "./links";
import {
  askedOfTheApp,
  closeTitleBarWindow,
  fakeTheApp,
  giveAnotherDocument,
  nextInTurn,
  nextShown,
  openPageAt,
  openPageWith,
  openWithTitleBar,
  settle,
  shownFrom,
  shownWaitNote,
  tell,
  titleBarButton,
  titleBarAppName,
  titleBarButtonRow,
  titleBarFileName,
  titleBarLogo,
  titleBarZoomLevel,
} from "./test-support";
import { startSettings } from "./settings";
import { startTitleBar } from "./title-bar";
import { startDocumentView } from "./view";
import { WAIT_NOTE_MS } from "./wait-note";

/** Each `place_maximize_button` request's arguments, in turn. */
let placed: ArgsOf<"place_maximize_button">[];

/**
 * The app's answers to `render_document`, in turn, or one that comes only once it resolves; the
 * last one repeats.
 */
let answers: (DocumentToShow | Promise<DocumentToShow>)[];

/** What the app's `wait_note` names. */
let waitedFor: WaitNote | null;

/** How the app answers `reveal_document`. */
let reveal: () => Promise<AnswerOf<"reveal_document">>;

/** Lays the title bar out, as the webview does when the window's size or DPI changes. */
let layOutTitleBar: () => void;

let stop: AbortController;

beforeEach(async () => {
  answers = [];
  waitedFor = null;
  reveal = () => Promise.resolve(null);
  ({ placed, layOut: layOutTitleBar } = await openWithTitleBar({
    render_document: () => nextInTurn(answers) ?? null,
    wait_note: () => waitedFor,
    reveal_document: () => reveal(),
    // A link shows another document in this window: the app tells the page, which asks for it.
    follow_link: async () => {
      await giveAnotherDocument();
      return { kind: "nothing" };
    },
  }));
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  vi.useRealTimers();
  closeTitleBarWindow();
});

function bar(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".title-bar");
}

test("the title bar has the window's minimize, maximize and close buttons", () => {
  startTitleBar(stop.signal);
  expect(titleBarButton("Minimize")).toBeDefined();
  expect(titleBarButton("Maximize")).toBeDefined();
  expect(titleBarButton("Close")).toBeDefined();
});

test("without the app's flag the native title bar stays and the page draws none", () => {
  openPageWith();
  startTitleBar(stop.signal);
  expect(bar()).toBeNull();
});

test("each window button runs its command, which asks the app to minimize, maximize and close", async () => {
  startTitleBar(stop.signal);
  titleBarButton("Minimize").click();
  titleBarButton("Maximize").click();
  titleBarButton("Close").click();
  await settle();
  expect(askedOfTheApp()).toEqual([
    "minimize_window",
    "toggle_maximize_window",
    "close_window",
  ]);
});

test("the close button and Ctrl+W ask the app the same thing", async () => {
  startTitleBar(stop.signal);
  startCommands(stop.signal);

  titleBarButton("Close").click();
  const byButton = askedOfTheApp();
  document.body.dispatchEvent(
    new KeyboardEvent("keydown", {
      ctrlKey: true,
      key: "w",
      code: "KeyW",
      bubbles: true,
      cancelable: true,
    }),
  );
  await settle();
  expect(byButton).toEqual(["close_window"]);
  expect(askedOfTheApp()).toEqual([...byButton, ...byButton]);
});

test("a window button's tooltip names its command and the command's shortcut", () => {
  startTitleBar(stop.signal);
  expect(titleBarButton("Close").title).toBe("Close (Ctrl+W)");
});

test("a window button whose command has no shortcut is named alone", () => {
  startTitleBar(stop.signal);
  expect(titleBarButton("Minimize").title).toBe("Minimize");
  expect(titleBarButton("Maximize").title).toBe("Maximize");
});

test("maximize turns into restore while the window is maximized", async () => {
  startTitleBar(stop.signal);
  await settle();

  await tell("window-state", "maximized");
  expect(titleBarButton("Restore").title).toBe("Restore");

  await tell("window-state", "restored");
  expect(titleBarButton("Maximize").title).toBe("Maximize");
});

test("a window state the button already shows leaves its glyph as it is", async () => {
  startTitleBar(stop.signal);
  await settle();
  await tell("window-state", "maximized");
  const glyph = titleBarButton("Restore").firstElementChild;

  // The app says so on every resize.
  await tell("window-state", "maximized");
  expect(titleBarButton("Restore").firstElementChild).toBe(glyph);
});

test("the maximize button looks hovered and pressed as the app says", async () => {
  startTitleBar(stop.signal);
  await settle();
  const maximize = titleBarButton("Maximize");

  await tell("maximize-button", "hovered");
  expect(maximize.dataset.state).toBe("hovered");
  await tell("maximize-button", "pressed");
  expect(maximize.dataset.state).toBe("pressed");
  await tell("maximize-button", "normal");
  expect(maximize.dataset.state).toBe("normal");
});

test("the title bar dims while the window doesn't have the focus", () => {
  startTitleBar(stop.signal);

  dispatchEvent(new FocusEvent("blur"));
  expect(bar()?.classList.contains("inactive")).toBe(true);

  dispatchEvent(new FocusEvent("focus"));
  expect(bar()?.classList.contains("inactive")).toBe(false);
});

test("laying the bar out tells the app where its maximize button is, in device pixels", async () => {
  vi.stubGlobal("devicePixelRatio", 1.5);
  startTitleBar(stop.signal);
  vi.spyOn(titleBarButton("Maximize"), "getBoundingClientRect").mockReturnValue(
    new DOMRect(908, 0, 46, 32),
  );

  layOutTitleBar();
  await settle();
  expect(placed).toEqual([
    { button: { x: 1362, y: 0, width: 69, height: 48 } },
  ]);
});

test("a press on the title bar leaves the focus where it was", () => {
  startTitleBar(stop.signal);
  for (const label of ["Minimize", "Maximize", "Close"]) {
    const press = new MouseEvent("mousedown", {
      bubbles: true,
      cancelable: true,
    });
    titleBarButton(label).dispatchEvent(press);
    expect(press.defaultPrevented, label).toBe(true);
  }
});

test("the window buttons never take the focus from the keyboard", () => {
  startTitleBar(stop.signal);
  for (const label of ["Minimize", "Maximize", "Close"]) {
    expect(titleBarButton(label).tabIndex, label).toBe(-1);
  }
});

/** A document's file as the app hands it over, the name worked out by the app. */
const README: DocumentFile = {
  name: "README.md",
  path: "C:\\work\\docs\\README.md",
};
const GUIDE: DocumentFile = {
  name: "guide.md",
  path: "C:\\work\\docs\\guide.md",
};

/** Starts the title bar and the document view, and waits until the view shows its first answer. */
async function startWithTheDocumentView(): Promise<void> {
  startTitleBar(stop.signal);
  await startDocumentView(0);
}

test("the logo is Downlight's icon, named Downlight on hover", () => {
  startTitleBar(stop.signal);
  expect(titleBarLogo().title).toBe("Downlight");
  expect(titleBarLogo().querySelector("svg")).not.toBeNull();
});

test("a click on the logo asks the app for the window menu", async () => {
  startTitleBar(stop.signal);
  titleBarLogo().click();
  await settle();
  expect(askedOfTheApp()).toEqual(["show_window_menu"]);
});

/** The app's answer that the window shows `file`'s new render, counted as `counts`, at once. */
function reloadedFrom(
  file: DocumentFile,
  counts: DocumentCounts,
): DocumentToShow {
  return {
    showing: 0,
    file,
    notice: null,
    steps: { back: false, forward: false },
    place: "reload",
    document: { html: `<h1>${file.name}</h1>\n<p>Changed.</p>\n`, counts },
  };
}

/** The app's answer that `file` can't be read. */
function unreadFrom(file: DocumentFile): DocumentToShow {
  return {
    showing: 0,
    file,
    notice: null,
    steps: { back: false, forward: false },
    place: "show",
    heading: null,
    error: `Can't read ${file.path}: entity not found`,
  };
}

test("the title bar names the shown document's file, with its full path and its counts on hover", async () => {
  answers = [shownFrom(README, { counts: { lines: 1234, words: 5678 } })];
  await startWithTheDocumentView();

  expect(titleBarFileName().hidden).toBe(false);
  expect(titleBarFileName().textContent).toBe("README.md");
  expect(titleBarFileName().title).toBe(
    "C:\\work\\docs\\README.md\n1,234 lines, 5,678 words",
  );
});

test("a document of one line and one word is counted in the singular", async () => {
  answers = [
    shownFrom(README, {
      rendered: "<p>Hi</p>\n",
      counts: { lines: 1, words: 1 },
    }),
  ];
  await startWithTheDocumentView();

  expect(titleBarFileName().title).toBe(
    "C:\\work\\docs\\README.md\n1 line, 1 word",
  );
});

test("an empty document is counted as none of either", async () => {
  answers = [
    shownFrom(README, { rendered: "", counts: { lines: 0, words: 0 } }),
  ];
  await startWithTheDocumentView();

  expect(titleBarFileName().title).toBe(
    "C:\\work\\docs\\README.md\n0 lines, 0 words",
  );
});

test("the file name's accessible description has its path and counts", async () => {
  answers = [shownFrom(README, { counts: { lines: 30, words: 1000 } })];
  await startWithTheDocumentView();

  expect(titleBarFileName().getAttribute("aria-description")).toBe(
    "C:\\work\\docs\\README.md\n30 lines, 1,000 words",
  );
});

test("a reload updates the counts", async () => {
  answers = [
    shownFrom(README, { counts: { lines: 3, words: 4 } }),
    reloadedFrom(README, { lines: 5, words: 9 }),
  ];
  await startWithTheDocumentView();

  const shown = nextShown();
  await tell("ask-for-document", null);
  await shown;
  expect(titleBarFileName().title).toBe(
    "C:\\work\\docs\\README.md\n5 lines, 9 words",
  );
  expect(titleBarFileName().getAttribute("aria-description")).toBe(
    "C:\\work\\docs\\README.md\n5 lines, 9 words",
  );
});

test("a document that can't be read has its path alone on hover", async () => {
  answers = [unreadFrom(README)];
  await startWithTheDocumentView();

  expect(titleBarFileName().hidden).toBe(false);
  expect(titleBarFileName().title).toBe("C:\\work\\docs\\README.md");
  expect(titleBarFileName().getAttribute("aria-description")).toBe(
    "C:\\work\\docs\\README.md",
  );
});

test("while the next document is on its way, the tooltip keeps the shown document's path and counts", async () => {
  answers = [
    shownFrom(README, { counts: { lines: 3, words: 4 } }),
    new Promise<never>(() => undefined),
  ];
  waitedFor = { file: GUIDE, size: "100 MB" };
  await startWithTheDocumentView();
  vi.useFakeTimers({ shouldAdvanceTime: true });

  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()?.textContent).toBe("Showing guide.md (100 MB) ...");
  });
  expect(titleBarFileName().textContent).toBe("README.md");
  expect(titleBarFileName().title).toBe(
    "C:\\work\\docs\\README.md\n3 lines, 4 words",
  );
});

test("the empty window has no tooltip of counts", async () => {
  answers = [shownFrom(README), null];
  await startWithTheDocumentView();

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(titleBarAppName().hidden).toBe(false);
  expect(titleBarAppName().title).toBe("");
  expect(titleBarFileName().hidden).toBe(true);
  expect(titleBarFileName().title).toBe("");
  expect(titleBarFileName().hasAttribute("aria-description")).toBe(false);
});

test("the name follows the document a link shows", async () => {
  answers = [
    shownFrom(README),
    shownFrom(GUIDE, { showing: 1, counts: { lines: 2, words: 7 } }),
  ];
  startLinks(stop.signal);
  await startWithTheDocumentView();
  await settle();

  const shown = nextShown();
  document.querySelector<HTMLAnchorElement>('a[href="guide.md"]')?.click();
  await shown;
  expect(titleBarFileName().textContent).toBe("guide.md");
  expect(titleBarFileName().title).toBe(
    "C:\\work\\docs\\guide.md\n2 lines, 7 words",
  );
});

// The app's name in the empty window (window design ticket 14).

test("the empty window shows Downlight beside the logo, in the file name's place", async () => {
  answers = [null];
  await startWithTheDocumentView();

  expect(titleBarLogo().hidden).toBe(false);
  expect(titleBarAppName().hidden).toBe(false);
  expect(titleBarAppName().textContent).toBe("Downlight");
  expect(titleBarFileName().hidden).toBe(true);
  expect(titleBarAppName().previousElementSibling).toBe(titleBarLogo());
});

test("showing a document replaces the app's name with the file name, and an empty window brings it back", async () => {
  answers = [null, shownFrom(README, { showing: 1 }), null];
  await startWithTheDocumentView();

  let shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(titleBarAppName().hidden).toBe(true);
  expect(titleBarFileName().hidden).toBe(false);
  expect(titleBarFileName().textContent).toBe("README.md");

  shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(titleBarAppName().hidden).toBe(false);
  expect(titleBarFileName().hidden).toBe(true);
});

test("a window opened on a document never shows the app's name, also before the document arrives", async () => {
  answers = [shownFrom(README)];
  startTitleBar(stop.signal);
  expect(titleBarAppName().hidden).toBe(true);

  await startDocumentView(0);
  expect(titleBarAppName().hidden).toBe(true);
});

test("the app's name is plain text the window is dragged by: no button, no tooltip, nothing on a click", async () => {
  answers = [null];
  await startWithTheDocumentView();
  const name = titleBarAppName();

  expect(name.closest("button")).toBeNull();
  expect(name.title).toBe("");
  // Only the bar's items are kept out of the drag region (title-bar.css).
  expect(name.classList.contains("bar-item")).toBe(false);
  const before = askedOfTheApp().length;
  name.click();
  await settle();
  expect(askedOfTheApp().slice(before)).toEqual([]);
});

test("a click on the name asks the app to reveal the document's file", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();

  titleBarFileName().click();
  await settle();
  expect(
    askedOfTheApp().filter((command) => command === "reveal_document"),
  ).toEqual(["reveal_document"]);
});

test("a file the app can't reveal shows why in a notice", async () => {
  const why =
    "Can't find C:\\work\\docs\\README.md, so it can't be shown in its folder.";
  reveal = () => Promise.reject(new Error(why));
  answers = [shownFrom(README)];
  await startWithTheDocumentView();

  titleBarFileName().click();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toContain(why);
  });
});

// Open and Edit (window design ticket 05).

test("the bar's buttons come in groups set apart by separators: Open and Edit, Full width, Keep on top and Settings, then the window's own", () => {
  startTitleBar(stop.signal);
  expect(titleBarButtonRow()).toEqual([
    "Open",
    "Edit",
    "|",
    "Full width",
    "Keep on top",
    "Settings",
    "|",
    "Minimize",
    "Maximize",
    "Close",
  ]);
});

test("Open and Edit run the commands of Ctrl+O and Ctrl+E", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  startCommands(stop.signal);

  titleBarButton("Open").click();
  titleBarButton("Edit").click();
  for (const key of ["o", "e"]) {
    // Once the first open dialog has closed: Open opens no second one while it is up.
    await settle();
    document.body.dispatchEvent(
      new KeyboardEvent("keydown", {
        ctrlKey: true,
        key,
        code: `Key${key.toUpperCase()}`,
        bubbles: true,
        cancelable: true,
      }),
    );
  }
  await settle();
  const asked = askedOfTheApp().filter(
    (command) => command === "open_document" || command === "edit_document",
  );
  expect(asked).toEqual([
    "open_document",
    "edit_document",
    "open_document",
    "edit_document",
  ]);
});

test("Open's and Edit's tooltips name the command and its shortcut", () => {
  startTitleBar(stop.signal);
  expect(titleBarButton("Open").title).toBe("Open (Ctrl+O)");
  expect(titleBarButton("Edit").title).toBe("Edit (Ctrl+E)");
});

test("Edit is disabled in a window without a document, and Open isn't", async () => {
  answers = [null];
  startCommands(stop.signal);
  await startWithTheDocumentView();
  expect(titleBarButton("Edit").disabled).toBe(true);
  expect(titleBarButton("Open").disabled).toBe(false);
});

test("Edit is enabled once the window shows a document, and disabled again without one", async () => {
  answers = [null, shownFrom(README, { showing: 1 }), null];
  startCommands(stop.signal);
  await startWithTheDocumentView();

  let shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(titleBarButton("Edit").disabled).toBe(false);

  shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(titleBarButton("Edit").disabled).toBe(true);
});

test("Edit is enabled for a document whose file can't be read, which the editor can still open", async () => {
  answers = [
    {
      showing: 0,
      file: README,
      notice: null,
      steps: { back: false, forward: false },
      place: "show",
      heading: null,
      error: "Can't read README.md: access is denied.",
    },
  ];
  startCommands(stop.signal);
  await startWithTheDocumentView();
  expect(titleBarButton("Edit").disabled).toBe(false);
});

test("Open and Edit are reached with Tab, unlike the window's own buttons", () => {
  startTitleBar(stop.signal);
  for (const label of ["Open", "Edit"]) {
    expect(titleBarButton(label).tabIndex, label).toBe(0);
  }
});

test("the logo and the name leave the focus in the document", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  for (const element of [titleBarLogo(), titleBarFileName()]) {
    expect(element.tabIndex, element.className).toBe(-1);
    const press = new MouseEvent("mousedown", {
      bubbles: true,
      cancelable: true,
    });
    element.dispatchEvent(press);
    expect(press.defaultPrevented, element.className).toBe(true);
  }
});

// The zoom level beside the file name (window design ticket 04, v1 ticket 16).

test("the zoom level is hidden at 100%", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  expect(titleBarZoomLevel().hidden).toBe(true);
});

test("the zoom level shows beside the name when it isn't 100%, and follows each change", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();

  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 125 });
  expect(titleBarZoomLevel().hidden).toBe(false);
  expect(titleBarZoomLevel().textContent).toBe("125%");

  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 90 });
  expect(titleBarZoomLevel().textContent).toBe("90%");

  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 100 });
  expect(titleBarZoomLevel().hidden).toBe(true);
});

test("a window that opens zoomed shows its zoom level at once", async () => {
  openPageAt(
    `?titleBar&settings=${encodeURIComponent(JSON.stringify({ ...DEFAULT_SETTINGS, zoom: 150 }))}`,
  );
  await startSettings();
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  expect(titleBarZoomLevel().hidden).toBe(false);
  expect(titleBarZoomLevel().textContent).toBe("150%");
});

test("the zoom level is named as Reset zoom, and says its shortcut", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 125 });
  // The level a reader sees is part of its name, as WCAG's label in name asks.
  expect(titleBarZoomLevel().getAttribute("aria-label")).toBe(
    "125%, Reset zoom",
  );
  expect(titleBarZoomLevel().title).toBe("Reset zoom (Ctrl+0)");
});

test("a click on the zoom level asks the app to reset the zoom", async () => {
  const changes: ArgsOf<"change_setting">[] = [];
  fakeTheApp({
    render_document: () => nextInTurn(answers) ?? null,
    change_setting: (args) => {
      changes.push(args);
      return null;
    },
  });
  await startSettings();
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 125 });

  titleBarZoomLevel().click();
  await settle();
  expect(changes).toEqual([{ change: { setting: "zoom", change: "reset" } }]);
});

test("the empty window shows no zoom level, as it shows no file name", async () => {
  answers = [null];
  await startWithTheDocumentView();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 125 });
  expect(titleBarZoomLevel().hidden).toBe(true);
});

test("the zoom level leaves the focus in the document, as the logo and the name do", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 125 });
  expect(titleBarZoomLevel().tabIndex).toBe(-1);
  expect(titleBarZoomLevel().classList.contains("bar-item")).toBe(true);
});
