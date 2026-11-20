import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { DEFAULT_SETTINGS, type MenuEntry } from "./app";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startFullScreen } from "./full-screen";
import { startSettings } from "./settings";
import { startSettingsFlyout } from "./settings-flyout";
import {
  askedOfTheApp,
  emptyWindow,
  failingWith,
  fakeTheApp,
  menuEntryOf,
  settle,
  tell,
  type FakeApp,
} from "./test-support";
import { startDocumentView } from "./view";

/** How the fake app answers; each test sets how it answers `switch_full_screen`. */
let app: FakeApp;

/** Whether the fake app's window is full screen, which its `switch_full_screen` flips. */
let fullScreen: boolean;

let stop: AbortController;

beforeEach(async () => {
  mockWindows("document-0");
  fullScreen = false;
  app = {
    switch_full_screen: () => (fullScreen = !fullScreen),
  };
  fakeTheApp(app);
  stop = new AbortController();
  await startSettings();
  startFullScreen(stop.signal);
  startCommands(stop.signal);
});

afterEach(() => {
  stop.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** How often the page asked the app to switch full screen. */
function switches(): number {
  return askedOfTheApp().filter((command) => command === "switch_full_screen")
    .length;
}

/** Presses a key where the page has the focus, as the reader would, and returns the event. */
function press(init: KeyboardEventInit): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    ...init,
    bubbles: true,
    cancelable: true,
  });
  (document.activeElement ?? document.body).dispatchEvent(event);
  return event;
}

function pressF11(): KeyboardEvent {
  return press({ key: "F11", code: "F11" });
}

function pressEsc(init: KeyboardEventInit = {}): KeyboardEvent {
  return press({ key: "Escape", code: "Escape", ...init });
}

/** Switches the window full screen with F11, once the app has answered that it is. */
async function goFullScreen(): Promise<void> {
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(true);
  });
}

/** The right-click menu's Full screen entry, as the page hands it to the app. */
function menuEntry(): ReturnType<typeof menuEntryOf> {
  let entries: MenuEntry[] = [];
  app.show_document_menu = (args) => {
    entries = args.entries;
    return null;
  };
  document.body.dispatchEvent(
    new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
  );
  return menuEntryOf(entries, "fullScreen");
}

/** Whether the right-click menu's Full screen has its check mark, or undefined without one. */
function checkedInMenu(): boolean | undefined {
  return menuEntry()?.checked;
}

/** The lines the notices show now, top to bottom. */
function noticeLines(): string[] {
  return [...document.querySelectorAll(".notice > div")].map(
    (line) => line.textContent,
  );
}

function findBar(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".find-bar");
}

function settingsFlyout(): HTMLElement | null {
  return document.querySelector<HTMLElement>('[role="dialog"]');
}

const LEAVE_NOTICE = "Press F11 or Esc to leave full screen";

test("F11 and the menu entry each ask the app to switch full screen", async () => {
  const event = pressF11();
  await settle();
  await tell("menu-command", "fullScreen");
  expect(switches()).toBe(2);
  expect(event.defaultPrevented).toBe(true);
});

test("the menu entry names Full screen and its key, after Keep on top", () => {
  let entries: MenuEntry[] = [];
  app.show_document_menu = (args) => {
    entries = args.entries;
    return null;
  };
  document.body.dispatchEvent(
    new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
  );
  const ids = entries.map((entry) =>
    entry.kind === "command" ? entry.id : entry.kind,
  );
  expect(ids.indexOf("fullScreen")).toBe(ids.indexOf("keepOnTop") + 1);
  expect(menuEntry()).toMatchObject({
    label: "Full screen",
    shortcut: "F11",
    enabled: true,
  });
});

test.each([
  ["Ctrl", { ctrlKey: true }],
  ["Alt", { altKey: true }],
  ["Shift", { shiftKey: true }],
  ["the Windows key", { metaKey: true }],
  // Windows reports AltGr as Ctrl+Alt.
  ["AltGr", { ctrlKey: true, altKey: true }],
])("F11 with %s switches nothing", async (_, modifiers) => {
  const event = press({ key: "F11", code: "F11", ...modifiers });
  await settle();
  expect(switches()).toBe(0);
  expect(event.defaultPrevented).toBe(false);
});

test("a new window starts out of full screen: its menu entry isn't ticked", () => {
  expect(checkedInMenu()).toBe(false);
});

test("the menu entry is ticked once the app answers that the window is full screen, and not once it answers that it left", async () => {
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(true);
  });
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(false);
  });
});

test("the menu entry shows the state the app answers with, not the one the page expected", async () => {
  // The window was full screen all along, as far as the app knows.
  app.switch_full_screen = () => true;
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(true);
  });
  pressF11();
  await settle();
  await settle();
  expect(switches()).toBe(2);
  expect(checkedInMenu()).toBe(true);
});

test("full screen isn't a setting: a settings change from any window leaves it as the app answered", async () => {
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(true);
  });
  await tell("settings", { ...DEFAULT_SETTINGS, fullWidth: true });
  expect(checkedInMenu()).toBe(true);
  await tell("settings", DEFAULT_SETTINGS);
  expect(checkedInMenu()).toBe(true);
});

test("a switch the app answers after the window's page started again leaves the window out of full screen and shows no Notice", async () => {
  let answer: (fullScreen: boolean) => void = () => {};
  app.switch_full_screen = () =>
    new Promise<boolean>((resolve) => {
      answer = resolve;
    });
  pressF11();
  await settle();

  stop.abort();
  document.body.replaceChildren();
  stop = new AbortController();
  startFullScreen(stop.signal);
  startCommands(stop.signal);
  answer(true);
  await settle();

  expect(checkedInMenu()).toBe(false);
  expect(noticeLines()).toEqual([]);
});

test("a full screen the app couldn't switch says why, and the menu entry stays as it was", async () => {
  app.switch_full_screen = () =>
    failingWith("Full screen didn't switch: the window is gone.");
  pressF11();
  await vi.waitFor(() => {
    expect(noticeLines()).toEqual([
      "Full screen didn't switch: the window is gone.",
    ]);
  });
  expect(checkedInMenu()).toBe(false);
});

test("each time the window goes full screen a Notice says how to leave, and leaving shows none", async () => {
  pressF11();
  await vi.waitFor(() => {
    expect(noticeLines()).toEqual([LEAVE_NOTICE]);
  });
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(false);
  });
  expect(noticeLines()).toEqual([LEAVE_NOTICE]);
  pressF11();
  await vi.waitFor(() => {
    expect(noticeLines()).toEqual([LEAVE_NOTICE, LEAVE_NOTICE]);
  });
});

test("F11 switches full screen in a window that shows no document", async () => {
  app.render_document = () => null;
  await startDocumentView(0, stop.signal);
  expect(emptyWindow()?.hidden).toBe(false);
  expect(menuEntry()).toMatchObject({ enabled: true });
  pressF11();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(true);
  });
  expect(switches()).toBe(1);
});

test("Esc leaves full screen", async () => {
  await goFullScreen();
  const event = pressEsc();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(false);
  });
  expect(switches()).toBe(2);
  expect(event.defaultPrevented).toBe(true);
});

test("Esc with Find open closes only Find, and a second Esc leaves full screen", async () => {
  startFind(stop.signal);
  await goFullScreen();
  press({ key: "f", code: "KeyF", ctrlKey: true });
  expect(findBar()?.hidden).toBe(false);

  pressEsc();
  await settle();
  expect(findBar()?.hidden).toBe(true);
  expect(switches()).toBe(1);
  expect(checkedInMenu()).toBe(true);

  pressEsc();
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(false);
  });
  expect(switches()).toBe(2);
});

test("Esc with the settings flyout open closes only the flyout", async () => {
  startSettingsFlyout(stop.signal);
  await goFullScreen();
  press({ key: ",", code: "Comma", ctrlKey: true });
  expect(settingsFlyout()?.hidden).toBe(false);

  pressEsc();
  await settle();
  expect(settingsFlyout()?.hidden).toBe(true);
  expect(switches()).toBe(1);
  expect(checkedInMenu()).toBe(true);
});

test("Esc with the settings flyout open and the focus outside it leaves full screen alone", async () => {
  startSettingsFlyout(stop.signal);
  await goFullScreen();
  press({ key: ",", code: "Comma", ctrlKey: true });
  // A click on the flyout where nothing takes the focus, such as its padding, leaves the flyout
  // open and takes the focus out of it, so the flyout's own Esc doesn't hear the next one.
  settingsFlyout()?.dispatchEvent(
    new PointerEvent("pointerdown", { bubbles: true }),
  );
  if (document.activeElement instanceof HTMLElement)
    document.activeElement.blur();
  expect(settingsFlyout()?.hidden).toBe(false);
  expect(document.activeElement).toBe(document.body);

  pressEsc();
  await settle();
  expect(switches()).toBe(1);
  expect(checkedInMenu()).toBe(true);
});

test("an Esc that cancels an IME's composition stays with the IME and asks the app nothing", async () => {
  await goFullScreen();
  const event = pressEsc({ isComposing: true });
  await settle();
  expect(switches()).toBe(1);
  expect(event.defaultPrevented).toBe(false);
  expect(checkedInMenu()).toBe(true);
});

test.each([
  ["Ctrl", { ctrlKey: true }],
  ["Alt", { altKey: true }],
  ["Shift", { shiftKey: true }],
  ["the Windows key", { metaKey: true }],
])("Esc with %s asks the app nothing", async (_, modifiers) => {
  await goFullScreen();
  const event = pressEsc(modifiers);
  await settle();
  expect(switches()).toBe(1);
  expect(event.defaultPrevented).toBe(false);
  expect(checkedInMenu()).toBe(true);
});

test("Esc out of full screen asks the app nothing", async () => {
  const event = pressEsc();
  await settle();
  expect(switches()).toBe(0);
  expect(event.defaultPrevented).toBe(false);
});

test("a held Esc leaves full screen once: its repeats ask the app nothing", async () => {
  await goFullScreen();
  // The repeats come before the app answers, while the page still thinks the window is full
  // screen, and the app switches rather than sets.
  pressEsc();
  pressEsc({ repeat: true });
  pressEsc({ repeat: true });
  await vi.waitFor(() => {
    expect(checkedInMenu()).toBe(false);
  });
  expect(switches()).toBe(2);
});
