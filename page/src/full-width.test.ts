import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import {
  DEFAULT_SETTINGS,
  type MenuEntry,
  type PageSettings,
  type SettingChange,
} from "./app";
import { startCommands } from "./commands";
import { FULL_WIDTH_CLASS } from "./full-width";
import { startLayout } from "./layout";
import { startSettings } from "./settings";
import {
  answerAsTheApp,
  failingWith,
  fakeTheApp,
  fakeTitleBarLayout,
  layOut,
  linesOf,
  menuEntryOf,
  openPageAt,
  settle,
  tell,
  titleBarButton,
  type FakeApp,
} from "./test-support";
import { startTitleBar } from "./title-bar";
import { startDocumentView } from "./view";

/** How the fake app answers; a test sets `change_setting` to make it fail. */
let app: FakeApp;

/** The changes of the settings the page asked the app for, in turn. */
let changes: SettingChange[];

let stop: AbortController;
let stopFullWidth: () => void;

/** Opens the page as the app does, with the title bar and `settings` in its URL. */
function openPageWithSettings(settings: PageSettings): void {
  openPageAt(
    `?titleBar&settings=${encodeURIComponent(JSON.stringify(settings))}`,
  );
}

beforeEach(async () => {
  mockWindows("document-0");
  changes = [];
  app = {
    change_setting: ({ change }) => {
      changes.push(change);
      return null;
    },
    render_document: (args) =>
      answerAsTheApp(args, () => Promise.resolve(readme)),
  };
  fakeTheApp(app);
  fakeTitleBarLayout();
  openPageWithSettings(DEFAULT_SETTINGS);
  stop = new AbortController();
  await startSettings();
  stopFullWidth = startLayout();
  startTitleBar(stop.signal);
  startCommands(stop.signal);
});

afterEach(() => {
  stop.abort();
  stopFullWidth();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  openPageAt("");
  document.documentElement.classList.remove(FULL_WIDTH_CLASS);
  document.body.replaceChildren();
});

/** Whether the document view fills the window's width; happy-dom doesn't lay it out. */
function fullWidth(): boolean {
  return document.documentElement.classList.contains(FULL_WIDTH_CLASS);
}

function fullWidthButton(): HTMLButtonElement {
  return titleBarButton("Full width");
}

/** How often the page asked the app to switch Full width. */
function switches(): number {
  return changes.filter((change) => change.setting === "switchFullWidth")
    .length;
}

test("the button, Ctrl+Alt+W and the menu entry each ask the app to switch Full width", async () => {
  fullWidthButton().click();
  document.body.dispatchEvent(
    new KeyboardEvent("keydown", {
      ctrlKey: true,
      altKey: true,
      key: "w",
      code: "KeyW",
      bubbles: true,
      cancelable: true,
    }),
  );
  await settle();
  await tell("menu-command", "fullWidth");
  expect(switches()).toBe(3);
});

/** Presses W with Ctrl and Alt, as the reader would, typing `key`, and returns the event. */
function pressCtrlAltW(key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    ctrlKey: true,
    altKey: true,
    key,
    code: "KeyW",
    bubbles: true,
    cancelable: true,
  });
  document.body.dispatchEvent(event);
  return event;
}

test("Ctrl+Alt+W switches Full width on a Cyrillic layout, by the key in W's place", async () => {
  pressCtrlAltW("ц");
  await settle();
  expect(switches()).toBe(1);
});

// Windows types AltGr as Ctrl+Alt, so on a layout where AltGr+W types a character (Czech's |),
// the key is that character and not the shortcut, or Full width would eat it in Find's box.
test("AltGr+W that types a character on the reader's layout types it and switches nothing", async () => {
  const event = pressCtrlAltW("|");
  await settle();
  expect(switches()).toBe(0);
  expect(event.defaultPrevented).toBe(false);
});

// A Latin letter outside a-z is a letter the reader types, not a key in another script that the
// shortcut finds by its place, as on Cyrillic.
test("AltGr+W that types å on US-International types it and switches nothing", async () => {
  const event = pressCtrlAltW("å");
  await settle();
  expect(switches()).toBe(0);
  expect(event.defaultPrevented).toBe(false);
});

test("AltGr+W that is a dead key on the reader's layout switches nothing", async () => {
  const event = pressCtrlAltW("Dead");
  await settle();
  expect(switches()).toBe(0);
  expect(event.defaultPrevented).toBe(false);
});

test("the button's tooltip names Full width and its shortcut", () => {
  expect(fullWidthButton().title).toBe("Full width (Ctrl+Alt+W)");
});

test("the document keeps GitHub's reading width while Full width is off, and its button isn't pressed", () => {
  expect(fullWidth()).toBe(false);
  expect(fullWidthButton().getAttribute("aria-pressed")).toBe("false");
});

test("Full width switched on in the settings widens the view and presses the button, and off undoes both", async () => {
  await tell("settings", { ...DEFAULT_SETTINGS, fullWidth: true });
  expect(fullWidth()).toBe(true);
  expect(fullWidthButton().getAttribute("aria-pressed")).toBe("true");

  await tell("settings", { ...DEFAULT_SETTINGS, fullWidth: false });
  expect(fullWidth()).toBe(false);
  expect(fullWidthButton().getAttribute("aria-pressed")).toBe("false");
});

test("a new window starts with the saved choice in its URL", async () => {
  stop.abort();
  stopFullWidth();
  document.body.replaceChildren();
  stop = new AbortController();
  openPageWithSettings({ ...DEFAULT_SETTINGS, fullWidth: true });
  await startSettings();
  stopFullWidth = startLayout();
  startTitleBar(stop.signal);

  expect(fullWidth()).toBe(true);
  expect(fullWidthButton().getAttribute("aria-pressed")).toBe("true");
});

test("the right-click menu shows Full width with a check mark while it is on", async () => {
  let entries: MenuEntry[] = [];
  app.show_document_menu = (args) => {
    entries = args.entries;
    return null;
  };
  const checkedInMenu = () => {
    document.body.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
    );
    return menuEntryOf(entries, "fullWidth")?.checked;
  };

  expect(checkedInMenu()).toBe(false);
  await tell("settings", { ...DEFAULT_SETTINGS, fullWidth: true });
  expect(checkedInMenu()).toBe(true);
});

test("a Full width that can't be saved says why", async () => {
  app.change_setting = () =>
    failingWith(
      "Downlight's settings can't be saved. Full width holds until Downlight closes.",
    );
  fullWidthButton().click();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toBe(
      "Downlight's settings can't be saved. Full width holds until Downlight closes.",
    );
  });
});

/** The first line of each block the page scrolled to the top of the view, in turn. */
function spyOnScrolls(): () => number[] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () =>
    scrolls.mock.contexts.map((block) => linesOf(block as Element)?.[0] ?? 0);
}

test("switching Full width keeps the block that was at the top of the view at the top", async () => {
  await startDocumentView(0);
  await settle();
  // Halfway down "## Options", on line 15.
  layOut(15.5);
  const scrolledTo = spyOnScrolls();
  await tell("settings", { ...DEFAULT_SETTINGS, fullWidth: true });
  await vi.waitFor(() => {
    expect(scrolledTo().length).toBeGreaterThan(0);
  });
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
  expect(fullWidth()).toBe(true);
});

test("a settings message that leaves Full width as it was doesn't move the view", async () => {
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 100 });
  await settle();
  expect(scrolledTo()).toEqual([]);
});
