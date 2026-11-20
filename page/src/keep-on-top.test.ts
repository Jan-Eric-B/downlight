import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { DEFAULT_SETTINGS, type MenuEntry } from "./app";
import { startCommands } from "./commands";
import { startKeepOnTop } from "./keep-on-top";
import { startSettings } from "./settings";
import {
  askedOfTheApp,
  failingWith,
  fakeTheApp,
  fakeTitleBarLayout,
  menuEntryOf,
  openPageAt,
  settle,
  tell,
  titleBarButton,
  type FakeApp,
} from "./test-support";
import { startTitleBar } from "./title-bar";

/** How the fake app answers; each test sets how it answers `switch_keep_on_top`. */
let app: FakeApp;

/** Whether the fake app's window is kept on top, which its `switch_keep_on_top` flips. */
let keptOnTop: boolean;

let stop: AbortController;

beforeEach(async () => {
  mockWindows("document-0");
  keptOnTop = false;
  app = {
    switch_keep_on_top: () => (keptOnTop = !keptOnTop),
  };
  fakeTheApp(app);
  fakeTitleBarLayout();
  openPageAt("?titleBar");
  stop = new AbortController();
  await startSettings();
  startKeepOnTop(stop.signal);
  startTitleBar(stop.signal);
  startCommands(stop.signal);
});

afterEach(() => {
  stop.abort();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  openPageAt("");
  document.body.replaceChildren();
});

function keepOnTopButton(): HTMLButtonElement {
  return titleBarButton("Keep on top");
}

/** Whether Keep on top's button shows it on. */
function pressed(): boolean {
  return keepOnTopButton().getAttribute("aria-pressed") === "true";
}

/** How often the page asked the app to switch Keep on top. */
function switches(): number {
  return askedOfTheApp().filter((command) => command === "switch_keep_on_top")
    .length;
}

/** Presses T with Ctrl and Alt, as the reader would. */
function pressCtrlAltT(): void {
  document.body.dispatchEvent(
    new KeyboardEvent("keydown", {
      ctrlKey: true,
      altKey: true,
      key: "t",
      code: "KeyT",
      bubbles: true,
      cancelable: true,
    }),
  );
}

/** Whether the right-click menu's Keep on top has its check mark, or undefined without one. */
function checkedInMenu(): boolean | undefined {
  let entries: MenuEntry[] = [];
  app.show_document_menu = (args) => {
    entries = args.entries;
    return null;
  };
  document.body.dispatchEvent(
    new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
  );
  return menuEntryOf(entries, "keepOnTop")?.checked;
}

test("the button, Ctrl+Alt+T and the menu entry each ask the app to switch Keep on top", async () => {
  keepOnTopButton().click();
  await settle();
  pressCtrlAltT();
  await settle();
  await tell("menu-command", "keepOnTop");
  expect(switches()).toBe(3);
});

// Windows reports AltGr as Ctrl+Alt, so the key is the letter it types, not the shortcut.
test("AltGr+T that types þ on US-International types it and switches nothing", async () => {
  const event = new KeyboardEvent("keydown", {
    ctrlKey: true,
    altKey: true,
    key: "þ",
    code: "KeyT",
    bubbles: true,
    cancelable: true,
  });
  document.body.dispatchEvent(event);
  await settle();
  expect(switches()).toBe(0);
  expect(event.defaultPrevented).toBe(false);
});

test("the button's tooltip names Keep on top and its shortcut", () => {
  expect(keepOnTopButton().title).toBe("Keep on top (Ctrl+Alt+T)");
});

test("a new window starts without Keep on top: its button isn't pressed and its menu entry isn't ticked", () => {
  expect(pressed()).toBe(false);
  expect(checkedInMenu()).toBe(false);
});

test("the button shows Keep on top on once the app answers that it switched it on, and off once it answers off", async () => {
  keepOnTopButton().click();
  await vi.waitFor(() => {
    expect(pressed()).toBe(true);
  });
  expect(checkedInMenu()).toBe(true);

  pressCtrlAltT();
  await vi.waitFor(() => {
    expect(pressed()).toBe(false);
  });
  expect(checkedInMenu()).toBe(false);
});

test("the button shows the state the app answers with, not the one the page expected", async () => {
  // The window was kept on top all along, as far as the app knows.
  app.switch_keep_on_top = () => true;
  keepOnTopButton().click();
  await vi.waitFor(() => {
    expect(pressed()).toBe(true);
  });
  keepOnTopButton().click();
  await settle();
  await settle();
  expect(switches()).toBe(2);
  expect(pressed()).toBe(true);
});

test("Keep on top isn't a setting: a settings change from any window leaves it as the app answered", async () => {
  keepOnTopButton().click();
  await vi.waitFor(() => {
    expect(pressed()).toBe(true);
  });
  await tell("settings", { ...DEFAULT_SETTINGS, fullWidth: true });
  expect(pressed()).toBe(true);
  await tell("settings", DEFAULT_SETTINGS);
  expect(pressed()).toBe(true);
});

test("a switch the app answers after the window's page started again leaves Keep on top off", async () => {
  let answer: (keptOnTop: boolean) => void = () => {};
  app.switch_keep_on_top = () =>
    new Promise<boolean>((resolve) => {
      answer = resolve;
    });
  keepOnTopButton().click();
  await settle();

  stop.abort();
  document.body.replaceChildren();
  stop = new AbortController();
  startKeepOnTop(stop.signal);
  startTitleBar(stop.signal);
  startCommands(stop.signal);
  answer(true);
  await settle();

  expect(pressed()).toBe(false);
  expect(checkedInMenu()).toBe(false);
});

test("a Keep on top the app couldn't switch says why, and the button stays as it was", async () => {
  app.switch_keep_on_top = () =>
    failingWith("Keep on top didn't switch: the window is gone.");
  keepOnTopButton().click();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toBe(
      "Keep on top didn't switch: the window is gone.",
    );
  });
  expect(pressed()).toBe(false);
});
