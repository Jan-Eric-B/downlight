import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { afterNextFrame } from "./reading-position";
import {
  askedOfTheApp,
  fakeTheAppReading,
  heading,
  saveTheFile,
  settle,
  type FakeDocumentApp,
  type NoAnswer,
  type Read,
} from "./test-support";
import { startDocumentView } from "./view";

// The window opens hidden and shows once its first frame is drawn (ticket 04). The window that
// shows with the wait note is in `wait-note.test.ts`.

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: (Read | NoAnswer)[] = [];

let app: FakeDocumentApp;

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  app = fakeTheAppReading(() => documents);
});

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

test("the window shows once its document is in it", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);

  await vi.waitFor(() => {
    expect(app.textWhenShown()).toContain("Lamplighter");
  });
  expect(askedOfTheApp().filter((c) => c === "show_window")).toHaveLength(1);
});

test("a window without a document shows once it offers to open one", async () => {
  documents = [() => Promise.resolve(null)];
  await startDocumentView(0);

  await vi.waitFor(() => {
    expect(app.textWhenShown()).toContain("Open file");
  });
});

test("a window whose document can't be read shows once it says why", async () => {
  documents = [() => Promise.reject(new Error("Can't read C:\\docs\\gone.md"))];
  await startDocumentView(0);

  await vi.waitFor(() => {
    expect(app.textWhenShown()).toContain("Can't read C:\\docs\\gone.md");
  });
});

test("the window shows with its document when a save overtakes its first request", async () => {
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
  const started = startDocumentView(0);
  await settle();

  await saveTheFile();
  sendFirst(notes);
  // Long enough for a window shown on the overtaken answer to be shown before the next arrives.
  await afterNextFrame();
  await afterNextFrame();
  sendSecond(readme);
  await started;
  expect(app.textWhenShown()).toContain("Lamplighter");
  expect(heading()).toBe("Lamplighter");
});
