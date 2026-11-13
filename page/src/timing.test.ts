import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { Read } from "./test-support";

// The timing switch is read once, when `timing.ts` loads, so each test loads the page's modules
// afresh after turning the switch on, as the app does by opening the page with `?timing`.

/** The first paint reports the page sent. */
let firstPaints: unknown[] = [];

/** What reading the window's document gives `render_document`, in turn. */
let documents: Read[] = [];

/**
 * Loads the document view with the timing switch on and a fresh fake app, and returns its start.
 */
async function loadTimedView(): Promise<
  (scriptStart: number) => Promise<void>
> {
  const { answerAsTheApp, mockTheApp } = await import("./test-support");
  const { startDocumentView } = await import("./view");
  mockTheApp(
    (command, args) => {
      if (command === "report_first_paint") firstPaints.push(args);
      if (command !== "render_document") return undefined;
      const next = documents.shift();
      return next && answerAsTheApp(args, next);
    },
    { shouldMockEvents: true },
  );
  return startDocumentView;
}

/** Lets the page's pending IPC calls, such as starting to listen, finish. */
function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

beforeEach(() => {
  history.replaceState(null, "", "/?timing");
  vi.resetModules();
  mockWindows("document-0");
  firstPaints = [];
  documents = [];
});

afterEach(() => {
  clearMocks();
  document.body.replaceChildren();
  history.replaceState(null, "", "/");
});

test("first paint is reported once the window shows its document", async () => {
  documents = [() => Promise.resolve(readme)];
  const start = await loadTimedView();
  await start(0);

  expect(firstPaints).toHaveLength(1);
});

test("a first request that a save overtakes still reports first paint", async () => {
  let sendFirst: (html: string) => void = () => {};
  let sendSecond: (html: string) => void = () => {};
  documents = [
    () =>
      new Promise((resolve) => {
        sendFirst = resolve;
      }),
    () =>
      new Promise((resolve) => {
        sendSecond = resolve;
      }),
  ];
  const start = await loadTimedView();
  const started = start(0);
  await settle();

  await emit("document-changed", "reload");
  sendFirst(notes);
  await settle();
  sendSecond(readme);
  await started;
  expect(firstPaints).toHaveLength(1);
});

test("a window without a document reports no first paint", async () => {
  documents = [() => Promise.resolve(null)];
  const start = await loadTimedView();
  await start(0);

  expect(firstPaints).toHaveLength(0);
});
