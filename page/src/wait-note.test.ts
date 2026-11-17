import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { WaitNote } from "./app";
import {
  askedOfTheApp,
  fakeTheAppReading,
  giveAnotherDocument,
  heading,
  saveTheFile,
  settle,
  shownWaitNote,
  waitNoteAsTheApp,
  type FakeDocumentApp,
  type NoAnswer,
  type Read,
} from "./test-support";
import { startDocumentView } from "./view";
import { WAIT_NOTE_MS } from "./wait-note";

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: (Read | NoAnswer)[] = [];

let app: FakeDocumentApp;

/** What the fake app's wait note names. */
const BIG_DOCUMENT: WaitNote = {
  file: { name: "big.md", path: "C:\\docs\\big.md" },
  size: "100 MB",
};

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  app = fakeTheAppReading(() => documents, {
    wait_note: (args) => waitNoteAsTheApp(args, BIG_DOCUMENT),
  });
  // The note's half second passes when a test says, never late on a loaded machine; time still
  // runs on its own, for the page's frames and the IPC.
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** A read that gives the HTML the test sends through the function it hands `send`. */
function readSentLater(send: (deliver: (html: string) => void) => void): Read {
  return () =>
    new Promise((resolve) => {
      send(resolve);
    });
}

test("a document that takes longer than half a second to arrive shows a wait note naming its file and size", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [readSentLater((send) => (deliver = send))];
  void startDocumentView(0);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);

  await vi.waitFor(() => {
    expect(shownWaitNote()?.textContent).toBe("Showing big.md (100 MB) ...");
  });
  expect(shownWaitNote()?.querySelector("strong")?.textContent).toBe("big.md");
  expect(shownWaitNote()?.getAttribute("role")).toBe("status");
  deliver(readme);
  await vi.waitFor(() => {
    expect(document.querySelector("article")?.textContent).toContain(
      "Lamplighter",
    );
  });
  expect(shownWaitNote()).toBeNull();
});

test("a window whose document is slow to arrive shows with the wait note rather than staying hidden", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [readSentLater((send) => (deliver = send))];
  void startDocumentView(0);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);

  await vi.waitFor(() => {
    expect(app.textWhenShown()).toContain("Showing big.md (100 MB) ...");
  });
  deliver(readme);
  await settle();
  expect(askedOfTheApp().filter((c) => c === "show_window")).toHaveLength(1);
});

test("a document that arrives within half a second shows no wait note", async () => {
  documents = [
    () =>
      new Promise((resolve) => {
        setTimeout(() => {
          resolve(readme);
        }, 100);
      }),
  ];
  await startDocumentView(0);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS + 100);

  expect(askedOfTheApp()).not.toContain("wait_note");
  expect(shownWaitNote()).toBeNull();
});

test("a document given while another is in view shows the wait note over it until it arrives", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0);
  await settle();

  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()).not.toBeNull();
  });
  expect(document.querySelector("article")?.textContent).toContain(
    "Lamplighter",
  );
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  expect(shownWaitNote()).toBeNull();
});

test("a reload slower than half a second shows no wait note and keeps the document in view", async () => {
  // The owner's decision on ticket 31: only a window waiting for another document shows the note.
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0);
  await settle();

  await saveTheFile();
  await vi.advanceTimersByTimeAsync(2 * WAIT_NOTE_MS);
  expect(shownWaitNote()).toBeNull();
  expect(heading()).toBe("Lamplighter");
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
});
