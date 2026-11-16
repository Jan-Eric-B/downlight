import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { AnswerOf, ArgsOf, DocumentFile, DocumentToShow } from "./app";
import { startCommands } from "./commands";
import { startLinks } from "./links";
import {
  askedOfTheApp,
  fakeTheApp,
  fakeTitleBarLayout,
  giveAnotherDocument,
  nextInTurn,
  nextShown,
  openPageWith,
  settle,
  tell,
  titleBarButton,
  titleBarFileName,
  titleBarLogo,
} from "./test-support";
import { startTitleBar } from "./title-bar";
import { startDocumentView } from "./view";

/** Each `place_maximize_button` request's arguments, in turn. */
let placed: ArgsOf<"place_maximize_button">[];

/** The app's answers to `render_document`, in turn; the last one repeats. */
let answers: DocumentToShow[];

/** How the app answers `reveal_document`. */
let reveal: () => Promise<AnswerOf<"reveal_document">>;

/** Lays the title bar out, as the webview does when the window's size or DPI changes. */
let layOutTitleBar: () => void;

let stop: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  placed = [];
  answers = [];
  reveal = () => Promise.resolve(null);
  layOutTitleBar = fakeTitleBarLayout();
  fakeTheApp({
    place_maximize_button: (args) => {
      placed.push(args);
      return null;
    },
    render_document: () => nextInTurn(answers) ?? null,
    reveal_document: () => reveal(),
    // A link shows another document in this window: the app tells the page, which asks for it.
    follow_link: async () => {
      await giveAnotherDocument();
      return { kind: "nothing" };
    },
  });
  // The app's flag for a window that has no native frame.
  openPageWith("titleBar");
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  openPageWith();
  document.body.replaceChildren();
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

/** The app's answer for a window that shows `file`, its `showing`th document. */
function shownFrom(file: DocumentFile, showing = 0): DocumentToShow {
  return {
    showing,
    file,
    notice: null,
    steps: { back: false, forward: false },
    place: "show",
    heading: null,
    document: `<h1>${file.name}</h1>\n<p><a href="guide.md">The guide</a></p>\n`,
  };
}

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

test("the title bar names the shown document's file, with its full path on hover", async () => {
  answers = [shownFrom(README)];
  await startWithTheDocumentView();

  expect(titleBarFileName().hidden).toBe(false);
  expect(titleBarFileName().textContent).toBe("README.md");
  expect(titleBarFileName().title).toBe("C:\\work\\docs\\README.md");
});

test("the name follows the document a link shows", async () => {
  answers = [shownFrom(README, 0), shownFrom(GUIDE, 1)];
  startLinks(stop.signal);
  await startWithTheDocumentView();
  await settle();

  const shown = nextShown();
  document.querySelector<HTMLAnchorElement>('a[href="guide.md"]')?.click();
  await shown;
  expect(titleBarFileName().textContent).toBe("guide.md");
  expect(titleBarFileName().title).toBe("C:\\work\\docs\\guide.md");
});

test("the empty window shows only the logo", async () => {
  answers = [null];
  await startWithTheDocumentView();

  expect(titleBarFileName().hidden).toBe(true);
  expect(titleBarLogo().hidden).toBe(false);
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
