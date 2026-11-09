import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  test,
} from "vitest";
import { closeOnCtrlW } from "./window";

let commands: string[] = [];

beforeAll(() => {
  closeOnCtrlW();
});

beforeEach(() => {
  commands = [];
  mockIPC((command) => {
    commands.push(command);
  });
});

afterEach(() => {
  clearMocks();
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
    expect(commands).toEqual(["close_window"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("on AZERTY, by the key that types w, which sits in Z's place", () => {
    press({ ctrlKey: true, key: "w", code: "KeyZ" });
    expect(commands).toEqual(["close_window"]);
  });

  test("on a Cyrillic layout, by the key in W's place", () => {
    press({ ctrlKey: true, key: "ц", code: "KeyW" });
    expect(commands).toEqual(["close_window"]);
  });
});

describe("the window stays open", () => {
  test("on AZERTY's key in W's place, which types z", () => {
    const event = press({ ctrlKey: true, key: "z", code: "KeyW" });
    expect(commands).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });

  test("on W without Ctrl", () => {
    press({ key: "w", code: "KeyW" });
    expect(commands).toEqual([]);
  });

  test.each([
    ["Shift", { shiftKey: true, key: "W" }],
    ["Alt", { altKey: true, key: "w" }],
    ["Meta", { metaKey: true, key: "w" }],
  ])("on Ctrl+W with %s held too", (_modifier, modifier) => {
    press({ ctrlKey: true, code: "KeyW", ...modifier });
    expect(commands).toEqual([]);
  });
});
