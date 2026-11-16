import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { ArgsOf } from "./app";
import { startCommands } from "./commands";
import {
  askedOfTheApp,
  fakeTheApp,
  fakeTitleBarLayout,
  openPageWith,
  settle,
  tell,
  titleBarButton,
} from "./test-support";
import { startTitleBar } from "./title-bar";

/** Each `place_maximize_button` request's arguments, in turn. */
let placed: ArgsOf<"place_maximize_button">[];

/** Lays the title bar out, as the webview does when the window's size or DPI changes. */
let layOutTitleBar: () => void;

let stop: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  placed = [];
  layOutTitleBar = fakeTitleBarLayout();
  fakeTheApp({
    place_maximize_button: (args) => {
      placed.push(args);
      return null;
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
