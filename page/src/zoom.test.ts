import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { ZoomChange } from "./app";
import { startCommands } from "./commands";
import { startSettings } from "./settings";
import {
  answerAsTheApp,
  documentZoom,
  failingWith,
  fakeTheApp,
  layOut,
  linesOf,
  openPageAt,
  settle,
  tell,
  type FakeApp,
} from "./test-support";
import { startDocumentView } from "./view";
import { startZoom, ZOOM_VARIABLE } from "./zoom";

/** How the fake app answers; a test sets `change_zoom` to make it fail. */
let app: FakeApp;

/** The changes of the zoom the page asked the app for, in turn. */
let asked: ZoomChange[];

let stop: AbortController;
let stopZoom: () => void;

beforeEach(async () => {
  mockWindows("document-0");
  asked = [];
  app = {
    change_zoom: ({ change }) => {
      asked.push(change);
      return null;
    },
    render_document: (args) =>
      answerAsTheApp(args, () => Promise.resolve(readme)),
  };
  fakeTheApp(app);
  stop = new AbortController();
  startCommands(stop.signal);
  await startSettings();
  stopZoom = startZoom();
});

afterEach(() => {
  stop.abort();
  stopZoom();
  vi.restoreAllMocks();
  clearMocks();
  openPageAt("");
  document.documentElement.style.removeProperty(ZOOM_VARIABLE);
  document.body.replaceChildren();
});

/** Presses a key with Ctrl, as the reader would, and returns the event. */
function pressWithCtrl(init: KeyboardEventInit): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    ctrlKey: true,
    ...init,
    bubbles: true,
    cancelable: true,
  });
  document.body.dispatchEvent(event);
  return event;
}

/** Turns the mouse wheel over the document, as the reader would, and returns the event. */
function turnWheel(init: WheelEventInit): WheelEvent {
  const event = new WheelEvent("wheel", {
    ...init,
    bubbles: true,
    cancelable: true,
  });
  // happy-dom's WheelEvent is a UIEvent rather than a MouseEvent, so it drops the modifier keys.
  Object.defineProperty(event, "ctrlKey", { value: init.ctrlKey ?? false });
  document.body.dispatchEvent(event);
  return event;
}

describe("Ctrl+plus zooms in", () => {
  test("by its own key, as on a German layout or the number pad", () => {
    const event = pressWithCtrl({ key: "+", code: "NumpadAdd" });
    expect(asked).toEqual(["in"]);
    expect(event.defaultPrevented).toBe(true);
  });

  test("by Shift and =, as on a US layout", () => {
    pressWithCtrl({ shiftKey: true, key: "+", code: "Equal" });
    expect(asked).toEqual(["in"]);
  });

  test("by = without Shift, as browsers take it", () => {
    pressWithCtrl({ key: "=", code: "Equal" });
    expect(asked).toEqual(["in"]);
  });

  test("again on each repeat while held", () => {
    pressWithCtrl({ key: "+" });
    pressWithCtrl({ key: "+", repeat: true });
    expect(asked).toEqual(["in", "in"]);
  });
});

test("Ctrl+minus zooms out", () => {
  const event = pressWithCtrl({ key: "-", code: "Minus" });
  expect(asked).toEqual(["out"]);
  expect(event.defaultPrevented).toBe(true);
});

describe("Ctrl+0 resets the zoom", () => {
  test("on a QWERTY layout", () => {
    pressWithCtrl({ key: "0", code: "Digit0" });
    expect(asked).toEqual(["reset"]);
  });

  test("on AZERTY, where the 0 key types à without Shift", () => {
    pressWithCtrl({ key: "à", code: "Digit0" });
    expect(asked).toEqual(["reset"]);
  });

  test("once while held down", () => {
    pressWithCtrl({ key: "0", code: "Digit0" });
    pressWithCtrl({ key: "0", code: "Digit0", repeat: true });
    expect(asked).toEqual(["reset"]);
  });
});

describe("Ctrl+wheel", () => {
  test("zooms in a step for each notch up and out for each notch down", () => {
    const up = turnWheel({ ctrlKey: true, deltaY: -100 });
    turnWheel({ ctrlKey: true, deltaY: -100 });
    turnWheel({ ctrlKey: true, deltaY: 100 });
    expect(asked).toEqual(["in", "in", "out"]);
    expect(up.defaultPrevented).toBe(true);
  });

  test("zooms a single step for a notch of a large display scale", () => {
    turnWheel({ ctrlKey: true, deltaY: -250 });
    expect(asked).toEqual(["in"]);
  });

  test("zooms once a touchpad's pinch has added up to a step", () => {
    for (let step = 0; step < 4; step++)
      turnWheel({ ctrlKey: true, deltaY: -10 });
    expect(asked).toEqual([]);
    turnWheel({ ctrlKey: true, deltaY: -10 });
    expect(asked).toEqual(["in"]);
  });

  test("starts adding up again when it turns the other way", () => {
    turnWheel({ ctrlKey: true, deltaY: -40 });
    turnWheel({ ctrlKey: true, deltaY: 40 });
    expect(asked).toEqual([]);
  });

  test("in lines rather than px zooms a step for each turn", () => {
    turnWheel({ ctrlKey: true, deltaY: -3, deltaMode: 1 });
    expect(asked).toEqual(["in"]);
  });

  test("without Ctrl leaves the wheel to scroll the document", () => {
    const event = turnWheel({ deltaY: -100 });
    expect(asked).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });
});

test("Zoom in chosen in the right-click menu zooms in", async () => {
  await settle();
  await tell("menu-command", "zoomIn");
  await tell("menu-command", "zoomOut");
  await tell("menu-command", "resetZoom");
  expect(asked).toEqual(["in", "out", "reset"]);
});

test("a zoom that can't be saved says why", async () => {
  app.change_zoom = () =>
    failingWith("Downlight's settings can't be saved. The zoom holds.");
  pressWithCtrl({ key: "+" });
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toBe(
      "Downlight's settings can't be saved. The zoom holds.",
    );
  });
});

test("the document view starts at 100% when the reader never zoomed", () => {
  expect(documentZoom()).toBe("1");
});

test("the document view starts at the zoom level in the page's URL", async () => {
  stopZoom();
  openPageAt(`?settings=${encodeURIComponent(JSON.stringify({ zoom: 150 }))}`);
  await startSettings();
  stopZoom = startZoom();
  expect(documentZoom()).toBe("1.5");
});

test("a page whose URL has settings it can't read starts at 100%", async () => {
  stopZoom();
  openPageAt("?settings=%7Bzoom");
  await startSettings();
  stopZoom = startZoom();
  expect(documentZoom()).toBe("1");
});

test("a new zoom level from the app zooms the document view", async () => {
  await tell("settings", { zoom: 125 });
  expect(documentZoom()).toBe("1.25");
});

/** The first line of each block the page scrolled to the top of the view, in turn. */
function spyOnScrolls(): () => number[] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () =>
    scrolls.mock.contexts.map((block) => linesOf(block as Element)?.[0] ?? 0);
}

test("zooming keeps the block that was at the top of the view at the top", async () => {
  await startDocumentView(0);
  await settle();
  // Halfway down "## Options", on line 15.
  layOut(15.5);
  const scrolledTo = spyOnScrolls();
  await tell("settings", { zoom: 150 });
  await vi.waitFor(() => {
    expect(scrolledTo().length).toBeGreaterThan(0);
  });
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
  expect(documentZoom()).toBe("1.5");
});

test("zooming at the top of the document stays at the top", async () => {
  await startDocumentView(0);
  await settle();
  layOut(1);
  const scrolledTo = spyOnScrolls();
  await tell("settings", { zoom: 150 });
  await settle();
  expect(scrolledTo()).toEqual([]);
});

test("a settings message that leaves the zoom as it was doesn't move the view", async () => {
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();
  await tell("settings", { zoom: 100 });
  await settle();
  expect(scrolledTo()).toEqual([]);
});
