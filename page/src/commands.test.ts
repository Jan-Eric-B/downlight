import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { startCommands } from "./commands";

let commands: string[] = [];
let stop: AbortController;

/** Answers the page's commands; the tests replace it to make one fail or wait. */
let answer: (command: string, args: unknown) => unknown;

beforeEach(() => {
  mockWindows("document-0");
  commands = [];
  answer = () => undefined;
  mockIPC(
    (command, args) => {
      commands.push(command);
      return answer(command, args);
    },
    { shouldMockEvents: true },
  );
  stop = new AbortController();
  startCommands(stop.signal);
});

afterEach(() => {
  stop.abort();
  clearMocks();
  document.body.replaceChildren();
});

/** The app's commands the page invoked, without the event plugin's own. */
function appCommands(): string[] {
  return commands.filter((command) => !command.startsWith("plugin:"));
}

/** Lets the page's pending IPC calls, such as starting to listen, finish. */
function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

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
    expect(appCommands()).toEqual(["close_window"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("on AZERTY, by the key that types w, which sits in Z's place", () => {
    press({ ctrlKey: true, key: "w", code: "KeyZ" });
    expect(appCommands()).toEqual(["close_window"]);
  });

  test("on a Cyrillic layout, by the key in W's place", () => {
    press({ ctrlKey: true, key: "ц", code: "KeyW" });
    expect(appCommands()).toEqual(["close_window"]);
  });
});

describe("the window stays open", () => {
  test("on AZERTY's key in W's place, which types z", () => {
    const event = press({ ctrlKey: true, key: "z", code: "KeyW" });
    expect(appCommands()).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });

  test("on W without Ctrl", () => {
    press({ key: "w", code: "KeyW" });
    expect(appCommands()).toEqual([]);
  });

  test.each([
    ["Shift", { shiftKey: true, key: "W" }],
    ["Alt", { altKey: true, key: "w" }],
    ["Meta", { metaKey: true, key: "w" }],
  ])("on Ctrl+W with %s held too", (_modifier, modifier) => {
    press({ ctrlKey: true, code: "KeyW", ...modifier });
    expect(appCommands()).toEqual([]);
  });
});

describe("Ctrl+O opens a document", () => {
  test("on a QWERTY layout", () => {
    const event = press({ ctrlKey: true, key: "o", code: "KeyO" });
    expect(appCommands()).toEqual(["open_document"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("on a Cyrillic layout, by the key in O's place", () => {
    press({ ctrlKey: true, key: "щ", code: "KeyO" });
    expect(appCommands()).toEqual(["open_document"]);
  });

  test("but not with Shift held too", () => {
    press({ ctrlKey: true, shiftKey: true, key: "O", code: "KeyO" });
    expect(appCommands()).toEqual([]);
  });

  test("once while the dialog is open", async () => {
    let closeDialog = () => {};
    answer = (command) =>
      command === "open_document"
        ? new Promise<void>((resolve) => {
            closeDialog = resolve;
          })
        : undefined;
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    expect(appCommands()).toEqual(["open_document"]);
    closeDialog();
    await settle();
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    expect(appCommands()).toEqual(["open_document", "open_document"]);
    closeDialog();
    await settle();
  });

  test("and shows why it couldn't", async () => {
    answer = (command) => {
      if (command === "open_document") {
        throw new Error("Can't show C:\\docs\\a.md: window not found");
      }
    };
    press({ ctrlKey: true, key: "o", code: "KeyO" });
    await vi.waitFor(() => {
      expect(document.querySelector(".notice")?.textContent).toContain(
        "Can't show C:\\docs\\a.md",
      );
    });
  });
});

describe("Reload", () => {
  test("F5 reloads", () => {
    const event = press({ key: "F5", code: "F5" });
    expect(appCommands()).toEqual(["reload_document"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("Ctrl+R reloads on a QWERTY layout", () => {
    const event = press({ ctrlKey: true, key: "r", code: "KeyR" });
    expect(appCommands()).toEqual(["reload_document"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("Ctrl+R reloads on a Cyrillic layout, by the key in R's place", () => {
    press({ ctrlKey: true, key: "к", code: "KeyR" });
    expect(appCommands()).toEqual(["reload_document"]);
  });

  // WebView2 would reload the page itself, leaving the window without its document.
  test.each([
    ["Ctrl+F5", { ctrlKey: true, key: "F5", code: "F5" }],
    ["Shift+F5", { shiftKey: true, key: "F5", code: "F5" }],
    ["Ctrl+Shift+R", { ctrlKey: true, shiftKey: true, key: "R", code: "KeyR" }],
  ])("%s is swallowed", (_keys, keys) => {
    const event = press(keys);
    expect(appCommands()).toEqual([]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("F5 with Alt held does nothing", () => {
    const event = press({ altKey: true, key: "F5", code: "F5" });
    expect(appCommands()).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });
});

describe("the right-click menu", () => {
  test("replaces the webview's menu with Reload, Open and Copy", () => {
    let entries: unknown;
    answer = (command, args) => {
      if (command === "show_document_menu") {
        entries = (args as { entries: unknown }).entries;
      }
    };
    const event = new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
    });
    document.body.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(entries).toEqual([
      { kind: "command", id: "reload", label: "Reload", shortcut: "F5" },
      { kind: "separator" },
      { kind: "command", id: "open", label: "Open...", shortcut: "Ctrl+O" },
      { kind: "separator" },
      { kind: "copy" },
    ]);
  });

  test("runs the command the reader chose", async () => {
    await settle();
    await emit("menu-command", "open");
    expect(appCommands()).toEqual(["open_document"]);
  });

  test("runs Reload chosen in the menu", async () => {
    await settle();
    await emit("menu-command", "reload");
    expect(appCommands()).toEqual(["reload_document"]);
  });

  test("ignores a command it doesn't know", async () => {
    await settle();
    await emit("menu-command", "toString");
    expect(appCommands()).toEqual([]);
  });
});
