import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import {
  DEFAULT_SETTINGS,
  type EditorChoices,
  type PageSettings,
  type SettingChange,
} from "./app";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startSettings } from "./settings";
import { startSettingsFlyout } from "./settings-flyout";
import {
  askedOfTheApp,
  failingWith,
  fakeTheApp,
  fakeTitleBarLayout,
  openPageAt,
  settle,
  tell,
  titleBarButton,
  titleBarButtonRow,
  type FakeApp,
} from "./test-support";
import { startTitleBar } from "./title-bar";

/** How the fake app answers; a test changes an answer to make it fail or wait. */
let app: FakeApp;

/** Each change of the settings the page asked the app for, in turn. */
let changes: SettingChange[];

/** What the fake app answers for Edit with. */
let choices: EditorChoices;

let stop: AbortController;

const WITH_EDITOR: EditorChoices = {
  defaultName: "Notepad",
  editor: { name: "Notepad++", path: "C:\\Tools\\notepad++.exe" },
};

beforeEach(() => {
  mockWindows("document-0");
  changes = [];
  choices = WITH_EDITOR;
  app = {
    change_setting: ({ change }) => {
      changes.push(change);
      return null;
    },
    editor_choices: () => choices,
  };
  fakeTheApp(app);
  fakeTitleBarLayout();
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  vi.unstubAllGlobals();
  clearMocks();
  openPageAt("");
  document.body.replaceChildren();
});

/**
 * Starts the page's title bar, commands and settings flyout, as in a window where the OS can
 * show a backdrop unless `backdropSupported` is false, with `settings`.
 */
async function startPage(
  settings: PageSettings = DEFAULT_SETTINGS,
  backdropSupported = true,
): Promise<void> {
  const flags = backdropSupported ? "titleBar&backdropSupported" : "titleBar";
  openPageAt(
    `?${flags}&settings=${encodeURIComponent(JSON.stringify(settings))}`,
  );
  await startSettings();
  startTitleBar(stop.signal);
  startCommands(stop.signal);
  startSettingsFlyout(stop.signal);
  await settle();
}

function gear(): HTMLButtonElement {
  return titleBarButton("Settings");
}

function flyout(): HTMLElement {
  const found = document.querySelector<HTMLElement>('[role="dialog"]');
  if (!found) throw new Error("The page has no settings flyout.");
  return found;
}

function isOpen(): boolean {
  return !flyout().hidden;
}

/** The group of the flyout's controls named `legend`. */
function group(legend: string): HTMLFieldSetElement {
  const found = [
    ...flyout().querySelectorAll<HTMLFieldSetElement>("fieldset"),
  ].find(
    (fieldset) => fieldset.querySelector("legend")?.textContent === legend,
  );
  if (!found) throw new Error(`The flyout has no ${legend}.`);
  return found;
}

/** The choices of the group `legend`, by their labels' text. */
function choicesIn(legend: string): string[] {
  return [...group(legend).querySelectorAll("label")].map(
    (label) => label.textContent,
  );
}

/** The choice labelled `label` in the group `legend`. */
function choice(legend: string, label: string): HTMLInputElement {
  const found = [...group(legend).querySelectorAll("label")].find(
    (candidate) => candidate.textContent === label,
  );
  const input = found?.querySelector("input");
  if (!input) throw new Error(`${legend} has no choice ${label}.`);
  return input;
}

/** The label of the checked choice in the group `legend`, if one is checked. */
function checkedIn(legend: string): string | undefined {
  const checked = group(legend).querySelector("input:checked");
  return checked?.closest("label")?.textContent;
}

function chooseButton(): HTMLButtonElement {
  const found = [...group("Edit with").querySelectorAll("button")].find(
    (button) => button.textContent === "Choose...",
  );
  if (!found) throw new Error("Edit with has no Choose... button.");
  return found;
}

/** Presses `key` where the focus is, as the reader would, and returns the event. */
function press(key: string, init: KeyboardEventInit = {}): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    key,
    bubbles: true,
    cancelable: true,
    ...init,
  });
  (document.activeElement ?? document.body).dispatchEvent(event);
  return event;
}

/** Opens the flyout with the gear and waits until Edit with shows the app's choices. */
async function openWithTheGear(): Promise<void> {
  gear().click();
  await vi.waitFor(() => {
    expect(choicesIn("Edit with").length).toBeGreaterThan(0);
  });
}

test("Settings joins Full width and Keep on top in the title bar, before the window's own buttons", async () => {
  await startPage();
  expect(titleBarButtonRow().slice(-7)).toEqual([
    "Full width",
    "Keep on top",
    "Settings",
    "|",
    "Minimize",
    "Maximize",
    "Close",
  ]);
  expect(gear().title).toBe("Settings (Ctrl+,)");
});

test("the gear opens the flyout with Theme, Backdrop, Accent color and Edit with, above the document rather than in the title bar", async () => {
  await startPage();
  expect(isOpen()).toBe(false);
  expect(gear().getAttribute("aria-expanded")).toBe("false");

  await openWithTheGear();
  expect(isOpen()).toBe(true);
  expect(gear().getAttribute("aria-expanded")).toBe("true");
  expect(flyout().closest(".title-bar")).toBeNull();
  expect(
    [...flyout().querySelectorAll("legend")].map(
      (legend) => legend.textContent,
    ),
  ).toEqual(["Theme", "Backdrop", "Accent color", "Edit with"]);
  expect(choicesIn("Theme")).toEqual(["System", "Light", "Dark"]);
  expect(choicesIn("Backdrop")).toEqual([
    "Mica",
    "Mica Alt",
    "Acrylic",
    "None",
  ]);
  expect(choicesIn("Accent color")).toEqual(["Amber"]);
});

test("the gear again closes the flyout, and the focus returns to the gear", async () => {
  await startPage();
  await openWithTheGear();
  gear().click();
  expect(isOpen()).toBe(false);
  expect(gear().getAttribute("aria-expanded")).toBe("false");
  expect(document.activeElement).toBe(gear());
});

test("Ctrl+, opens the flyout and closes it again", async () => {
  await startPage();
  press(",", { ctrlKey: true, code: "Comma" });
  expect(isOpen()).toBe(true);
  press(",", { ctrlKey: true, code: "Comma" });
  expect(isOpen()).toBe(false);
  expect(document.activeElement).toBe(gear());
});

test("Esc closes the flyout and returns the focus to the gear, and leaves an open find bar open", async () => {
  await startPage();
  startFind(stop.signal);
  press("f", { ctrlKey: true, code: "KeyF" });
  await openWithTheGear();
  const event = press("Escape");
  expect(event.defaultPrevented).toBe(true);
  expect(isOpen()).toBe(false);
  expect(document.activeElement).toBe(gear());
  expect(document.querySelector<HTMLElement>(".find-bar")?.hidden).toBe(false);
});

test("a click outside closes the flyout, and a click inside doesn't", async () => {
  await startPage();
  await openWithTheGear();
  group("Theme").dispatchEvent(
    new MouseEvent("pointerdown", { bubbles: true }),
  );
  expect(isOpen()).toBe(true);
  document.body.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
  expect(isOpen()).toBe(false);
  expect(gear().getAttribute("aria-expanded")).toBe("false");
});

test("opening puts the focus on the reader's theme, and Tab moves through the controls and stays inside", async () => {
  await startPage({ ...DEFAULT_SETTINGS, theme: "dark", backdrop: "acrylic" });
  await openWithTheGear();
  const theme = choice("Theme", "Dark");
  expect(document.activeElement).toBe(theme);

  // One stop per group, at its checked choice, as the browser's own Tab does with radio buttons.
  const stops = [
    theme,
    choice("Backdrop", "Acrylic"),
    choice("Accent color", "Amber"),
    choice("Edit with", "Notepad++"),
    chooseButton(),
  ];
  for (const next of [...stops.slice(1), stops[0]]) {
    expect(press("Tab").defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(next);
  }
  press("Tab", { shiftKey: true });
  expect(document.activeElement).toBe(chooseButton());
});

test("each choice asks the app to set that value for every window, and the choice the settings hold asks nothing", async () => {
  await startPage();
  await openWithTheGear();
  choice("Theme", "Light").click();
  choice("Backdrop", "Mica Alt").click();
  // Checked already: Amber is the only accent color until window design ticket 09.
  choice("Accent color", "Amber").click();
  await settle();
  expect(changes).toEqual([
    { setting: "theme", theme: "light" },
    { setting: "backdrop", backdrop: "micaAlt" },
  ]);
});

test("the flyout shows the settings as they are, and follows a change from any window", async () => {
  await startPage({ ...DEFAULT_SETTINGS, theme: "light", backdrop: "none" });
  await openWithTheGear();
  expect([
    checkedIn("Theme"),
    checkedIn("Backdrop"),
    checkedIn("Accent color"),
  ]).toEqual(["Light", "None", "Amber"]);
  await tell("settings", {
    ...DEFAULT_SETTINGS,
    theme: "dark",
    backdrop: "mica",
  });
  expect([checkedIn("Theme"), checkedIn("Backdrop")]).toEqual(["Dark", "Mica"]);
});

test("where the OS shows no backdrop, Backdrop is disabled and says why", async () => {
  await startPage(DEFAULT_SETTINGS, false);
  await openWithTheGear();
  expect(group("Backdrop").disabled).toBe(true);
  expect(group("Backdrop").title).toBe("A backdrop needs Windows 11.");
});

test("Edit with offers Default by its program's name and the chosen editor, which is checked, with its file as the tooltip", async () => {
  await startPage();
  await openWithTheGear();
  expect(choicesIn("Edit with")).toEqual(["Default (Notepad)", "Notepad++"]);
  expect(checkedIn("Edit with")).toBe("Notepad++");
  expect(choice("Edit with", "Notepad++").closest("label")?.title).toBe(
    "C:\\Tools\\notepad++.exe",
  );
});

test("Edit with without a chosen editor checks Default, named alone when the system names no program", async () => {
  choices = { defaultName: null, editor: null };
  await startPage();
  await openWithTheGear();
  expect(choicesIn("Edit with")).toEqual(["Default"]);
  expect(checkedIn("Edit with")).toBe("Default");
});

test("Default asks the app to forget the chosen editor, and Edit with shows what the app has then", async () => {
  await startPage();
  await openWithTheGear();
  choices = { defaultName: "Notepad", editor: null };
  choice("Edit with", "Default (Notepad)").click();
  await vi.waitFor(() => {
    expect(choicesIn("Edit with")).toEqual(["Default (Notepad)"]);
  });
  expect(changes).toEqual([{ setting: "clearEditor" }]);
});

test("Choose... opens the app's dialog once while it is up, and Edit with shows the editor chosen in it", async () => {
  let closeDialog = () => {};
  app.choose_editor = () =>
    new Promise((resolve) => {
      closeDialog = () => {
        resolve(null);
      };
    });
  choices = { defaultName: "Notepad", editor: null };
  await startPage();
  await openWithTheGear();
  chooseButton().click();
  chooseButton().click();
  await settle();
  expect(
    askedOfTheApp().filter((command) => command === "choose_editor"),
  ).toHaveLength(1);

  choices = WITH_EDITOR;
  closeDialog();
  await vi.waitFor(() => {
    expect(checkedIn("Edit with")).toBe("Notepad++");
  });
});

test("an editor that couldn't be chosen says why", async () => {
  app.choose_editor = () =>
    failingWith(
      "Downlight can't be the editor: Edit opens the document in another program.",
    );
  await startPage();
  await openWithTheGear();
  chooseButton().click();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toContain(
      "Downlight can't be the editor",
    );
  });
});
