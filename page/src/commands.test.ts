import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { ArgsOf, MenuEntry } from "./app";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startSettings } from "./settings";
import {
  answerAsTheApp,
  askedOfTheApp,
  fakeTheApp,
  settle,
  tell,
  type FakeApp,
} from "./test-support";
import { startDocumentView } from "./view";

let stop: AbortController;

/** How the fake app answers; the tests set a command's answer to make it fail or wait. */
let app: FakeApp;

beforeEach(async () => {
  mockWindows("document-0");
  app = {};
  fakeTheApp(app);
  stop = new AbortController();
  await startSettings();
  startCommands(stop.signal);
});

afterEach(() => {
  stop.abort();
  clearMocks();
  document.body.replaceChildren();
});

/** Presses a key in the document, as the reader would, and returns the event. */
function press(init: KeyboardEventInit): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    ...init,
    bubbles: true,
    cancelable: true,
  });
  document.body.dispatchEvent(event);
  return event;
}

describe("Ctrl+W closes the window", () => {
  test("on a QWERTY layout", () => {
    const event = press({ ctrlKey: true, key: "w", code: "KeyW" });
    expect(askedOfTheApp()).toEqual(["close_window"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("on AZERTY, by the key that types w, which sits in Z's place", () => {
    press({ ctrlKey: true, key: "w", code: "KeyZ" });
    expect(askedOfTheApp()).toEqual(["close_window"]);
  });

  test("on a Cyrillic layout, by the key in W's place", () => {
    press({ ctrlKey: true, key: "ц", code: "KeyW" });
    expect(askedOfTheApp()).toEqual(["close_window"]);
  });

  // Held down, the repeats would close the next Downlight window that takes the focus.
  test("once while held down", () => {
    press({ ctrlKey: true, key: "w", code: "KeyW" });
    const repeat = press({
      ctrlKey: true,
      key: "w",
      code: "KeyW",
      repeat: true,
    });
    expect(askedOfTheApp()).toEqual(["close_window"]);
    expect(repeat.defaultPrevented).toBe(true);
  });

  test("and shows why it couldn't", async () => {
    app.close_window = () => {
      throw new Error("Can't close the window");
    };
    press({ ctrlKey: true, key: "w", code: "KeyW" });
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toContain(
        "Can't close the window",
      );
    });
  });
});

describe("the window stays open", () => {
  test("on AZERTY's key in W's place, which types z", () => {
    const event = press({ ctrlKey: true, key: "z", code: "KeyW" });
    expect(askedOfTheApp()).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });

  test("on W without Ctrl", () => {
    press({ key: "w", code: "KeyW" });
    expect(askedOfTheApp()).toEqual([]);
  });

  test.each([
    ["Shift", { shiftKey: true, key: "W" }, []],
    // Ctrl+Alt+W is Full width's (window design ticket 06).
    ["Alt", { altKey: true, key: "w" }, ["change_setting"]],
    ["Meta", { metaKey: true, key: "w" }, []],
  ])("on Ctrl+W with %s held too", (_modifier, modifier, asked) => {
    press({ ctrlKey: true, code: "KeyW", ...modifier });
    expect(askedOfTheApp()).toEqual(asked);
  });
});

describe("Ctrl+O opens a document", () => {
  test("on a QWERTY layout", () => {
    const event = press({ ctrlKey: true, key: "o", code: "KeyO" });
    expect(askedOfTheApp()).toEqual(["open_document"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("on a Cyrillic layout, by the key in O's place", () => {
    press({ ctrlKey: true, key: "щ", code: "KeyO" });
    expect(askedOfTheApp()).toEqual(["open_document"]);
  });

  test("but not with Shift held too", () => {
    press({ ctrlKey: true, shiftKey: true, key: "O", code: "KeyO" });
    expect(askedOfTheApp()).toEqual([]);
  });

  test("once while the dialog is open", async () => {
    let closeDialog = () => {};
    app.open_document = () =>
      new Promise((resolve) => {
        closeDialog = () => {
          resolve(null);
        };
      });
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    expect(askedOfTheApp()).toEqual(["open_document"]);
    closeDialog();
    await settle();
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    expect(askedOfTheApp()).toEqual(["open_document", "open_document"]);
    closeDialog();
    await settle();
  });

  test("and shows why it couldn't", async () => {
    app.open_document = () => {
      throw new Error("Can't show C:\\docs\\a.md: window not found");
    };
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toContain(
        "Can't show C:\\docs\\a.md",
      );
    });
  });
});

describe("Ctrl+E opens the document in the editor", () => {
  test("on a QWERTY layout", () => {
    const event = press({ ctrlKey: true, key: "e", code: "KeyE" });
    expect(askedOfTheApp()).toEqual(["edit_document"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("on a Cyrillic layout, by the key in E's place", () => {
    press({ ctrlKey: true, key: "у", code: "KeyE" });
    expect(askedOfTheApp()).toEqual(["edit_document"]);
  });

  // Held down, each repeat would start the editor again.
  test("once while held down", () => {
    press({ ctrlKey: true, key: "e", code: "KeyE" });
    press({ ctrlKey: true, key: "e", code: "KeyE", repeat: true });
    expect(askedOfTheApp()).toEqual(["edit_document"]);
  });

  test("and a failed edit shows a notice", async () => {
    app.edit_document = () => {
      throw new Error("There is no document to edit.");
    };
    press({ ctrlKey: true, key: "e", code: "KeyE" });
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toContain(
        "There is no document to edit.",
      );
    });
  });
});

describe("Choose editor...", () => {
  test("runs from the menu", async () => {
    await settle();
    await tell("menu-command", "chooseEditor");
    expect(askedOfTheApp()).toEqual(["choose_editor"]);
  });

  test("a second Choose editor before the dialog closes opens no second dialog", async () => {
    let closeDialog = () => {};
    app.choose_editor = () =>
      new Promise((resolve) => {
        closeDialog = () => {
          resolve(null);
        };
      });
    await settle();
    await tell("menu-command", "chooseEditor");
    await tell("menu-command", "chooseEditor");
    expect(askedOfTheApp()).toEqual(["choose_editor"]);
    closeDialog();
    await settle();
    await tell("menu-command", "chooseEditor");
    expect(askedOfTheApp()).toEqual(["choose_editor", "choose_editor"]);
    closeDialog();
    await settle();
  });

  test("shows why a choice wasn't kept", async () => {
    app.choose_editor = () => {
      throw new Error("Downlight can't be the editor");
    };
    await settle();
    await tell("menu-command", "chooseEditor");
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toContain(
        "Downlight can't be the editor",
      );
    });
  });
});

// A reload asks the document view for the window's document directly, with the showing the view
// shows, so the app answers it with a reload (ticket 26).
describe("Reload", () => {
  /** Each `render_document` request's arguments since the view showed its document. */
  let requested: ArgsOf<"render_document">[];

  beforeEach(async () => {
    app.render_document = (args) => {
      requested.push(args);
      return answerAsTheApp(args, () => Promise.resolve(readme));
    };
    requested = [];
    await startDocumentView(0);
    requested = [];
  });

  test("F5 reloads", () => {
    const event = press({ key: "F5", code: "F5" });
    expect(requested).toEqual([{ showing: 0 }]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("Ctrl+R reloads on a QWERTY layout", () => {
    const event = press({ ctrlKey: true, key: "r", code: "KeyR" });
    expect(requested).toEqual([{ showing: 0 }]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("Ctrl+R reloads on a Cyrillic layout, by the key in R's place", () => {
    press({ ctrlKey: true, key: "к", code: "KeyR" });
    expect(requested).toEqual([{ showing: 0 }]);
  });

  test("F5 held down reloads once", () => {
    press({ key: "F5", code: "F5" });
    const repeat = press({ key: "F5", code: "F5", repeat: true });
    press({ key: "F5", code: "F5", repeat: true });
    expect(requested).toEqual([{ showing: 0 }]);
    // The webview never gets the repeats either, so it doesn't reload the page itself.
    expect(repeat.defaultPrevented).toBe(true);
  });

  test("a reload the app can't answer keeps the document and shows why", async () => {
    app.render_document = () => {
      throw new Error("Can't reload: window not found");
    };
    press({ key: "F5", code: "F5" });
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toContain(
        "Can't reload: window not found",
      );
    });
    expect(document.querySelector("article h1")?.textContent).toBe(
      "Lamplighter",
    );
  });

  test("runs Reload chosen in the menu", async () => {
    await settle();
    await tell("menu-command", "reload");
    expect(requested).toEqual([{ showing: 0 }]);
  });

  // WebView2's own reloads on these keys are off with all its browser keys (ticket 29), and the
  // page gives them no command.
  test.each([
    ["Ctrl+F5", { ctrlKey: true, key: "F5", code: "F5" }],
    ["Shift+F5", { shiftKey: true, key: "F5", code: "F5" }],
    ["Ctrl+Shift+R", { ctrlKey: true, shiftKey: true, key: "R", code: "KeyR" }],
  ])("%s does nothing", (_keys, keys) => {
    const event = press(keys);
    expect(requested).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });

  test("F5 with Alt held does nothing", () => {
    const event = press({ altKey: true, key: "F5", code: "F5" });
    expect(requested).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });
});

// With nowhere to go in the document in view, a step goes to the app (`history.test.ts` covers
// where it lands).
describe("Back and Forward", () => {
  /** Each `step_history` call's step since the view showed its document. */
  let steps: ArgsOf<"step_history">["step"][];

  beforeEach(async () => {
    app.render_document = (args) =>
      answerAsTheApp(args, () => Promise.resolve(readme));
    app.step_history = ({ step }) => {
      steps.push(step);
      return null;
    };
    steps = [];
    await startDocumentView(0);
  });

  test.each([
    ["Alt+Left", { altKey: true, key: "ArrowLeft", code: "ArrowLeft" }],
    ["the Browser Back key", { key: "BrowserBack", code: "BrowserBack" }],
  ])("%s goes back", (_keys, keys) => {
    const event = press(keys);
    expect(steps).toEqual(["back"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test.each([
    ["Alt+Right", { altKey: true, key: "ArrowRight", code: "ArrowRight" }],
    [
      "the Browser Forward key",
      { key: "BrowserForward", code: "BrowserForward" },
    ],
  ])("%s goes forward", (_keys, keys) => {
    const event = press(keys);
    expect(steps).toEqual(["forward"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test.each([
    ["back", 3],
    ["forward", 4],
  ])("the mouse's %s button goes %s", (step, button) => {
    const event = new MouseEvent("mouseup", {
      button,
      bubbles: true,
      cancelable: true,
    });
    document.body.dispatchEvent(event);
    expect(steps).toEqual([step]);
    expect(event.defaultPrevented).toBe(true);
  });

  test.each([0, 1, 2])("mouse button %i goes nowhere", (button) => {
    const event = new MouseEvent("mouseup", {
      button,
      bubbles: true,
      cancelable: true,
    });
    document.body.dispatchEvent(event);
    expect(steps).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });

  // The arrow keys alone scroll the document.
  test("Left and Right without Alt go nowhere", () => {
    const event = press({ key: "ArrowLeft", code: "ArrowLeft" });
    press({ key: "ArrowRight", code: "ArrowRight" });
    expect(steps).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });
});

// Holding F3 steps on through Find's matches, as in a browser.
test.each([
  ["F3", { key: "F3", code: "F3" }],
  ["Shift+F3", { shiftKey: true, key: "F3", code: "F3" }],
])("%s held down keeps running", (_keys, keys) => {
  startFind(stop.signal);
  const repeat = press({ ...keys, repeat: true });
  expect(repeat.defaultPrevented).toBe(true);
  // With the bar closed, a step opens Find.
  expect(document.querySelector<HTMLElement>(".find-bar")?.hidden).toBe(false);
});

// Minimize and maximize are commands for the title bar's buttons, with no shortcut of their own.
test("a key that names no shortcut never minimizes or maximizes the window", () => {
  for (const init of [{}, { key: "Unidentified" }, { ctrlKey: true }]) {
    expect(press(init).defaultPrevented).toBe(false);
  }
  expect(askedOfTheApp()).toEqual([]);
});

describe("the right-click menu", () => {
  /** The entries the page last asked the app to show. */
  let entries: MenuEntry[];

  beforeEach(() => {
    entries = [];
    app.show_document_menu = (args) => {
      entries = args.entries;
      return null;
    };
  });

  /** Right-clicks the document, as the reader would, and returns the event. */
  function rightClick(): MouseEvent {
    const event = new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
    });
    document.body.dispatchEvent(event);
    return event;
  }

  /** Whether the menu's entry for the command `id` can be chosen. */
  function enabledInMenu(id: string): boolean | undefined {
    rightClick();
    const entry = entries.find(
      (candidate) => candidate.kind === "command" && candidate.id === id,
    );
    return entry?.kind === "command" ? entry.enabled : undefined;
  }

  test("replaces the webview's menu with Back, Forward, Reload, Find, Edit, Choose editor..., Open, Zoom in, Zoom out, Reset zoom, Full width, Keep on top and Copy", async () => {
    // A window without a document.
    app.render_document = () => null;
    await startDocumentView(0);
    const event = rightClick();
    expect(event.defaultPrevented).toBe(true);
    // With nowhere to go back or forward to, Back and Forward are unavailable, and without a
    // document, Edit (window design ticket 05).
    const command = (id: string, label: string, shortcut?: string) => ({
      kind: "command",
      id,
      label,
      shortcut,
      enabled: !["back", "forward", "edit"].includes(id),
    });
    expect(entries).toEqual([
      command("back", "Back", "Alt+Left"),
      command("forward", "Forward", "Alt+Right"),
      command("reload", "Reload", "F5"),
      command("find", "Find", "Ctrl+F"),
      { kind: "separator" },
      command("edit", "Edit", "Ctrl+E"),
      // Without a shortcut, the menu shows none beside it.
      command("chooseEditor", "Choose editor..."),
      command("open", "Open...", "Ctrl+O"),
      { kind: "separator" },
      // As Windows names the keys, as in Notepad's menu.
      command("zoomIn", "Zoom in", "Ctrl+Plus"),
      command("zoomOut", "Zoom out", "Ctrl+Minus"),
      command("resetZoom", "Reset zoom", "Ctrl+0"),
      { kind: "separator" },
      // Off, with its check mark not ticked (window design ticket 06).
      { ...command("fullWidth", "Full width", "Ctrl+Alt+W"), checked: false },
      // A new window starts without it (window design ticket 07).
      {
        ...command("keepOnTop", "Keep on top", "Ctrl+Alt+T"),
        checked: false,
      },
      { kind: "separator" },
      { kind: "copy" },
    ]);
  });

  test("offers Edit once the window shows a document", async () => {
    app.render_document = (args) =>
      answerAsTheApp(args, () => Promise.resolve(readme));
    await startDocumentView(0);
    expect(enabledInMenu("edit")).toBe(true);
  });

  test("runs the command the reader chose", async () => {
    await settle();
    await tell("menu-command", "open");
    expect(askedOfTheApp()).toEqual(["open_document"]);
  });

  test("runs Find chosen in the menu", async () => {
    startFind(stop.signal);
    await settle();
    await tell("menu-command", "find");
    const bar = document.querySelector<HTMLElement>(".find-bar");
    expect(bar?.hidden).toBe(false);
    expect(document.activeElement).toBe(bar?.querySelector("input"));
  });

  test("ignores a command it doesn't know", async () => {
    await settle();
    await tell("menu-command", "toString");
    expect(askedOfTheApp()).toEqual([]);
  });
});
