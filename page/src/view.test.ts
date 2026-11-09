import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { startDocumentView } from "./view";

let commands: string[] = [];

/** What `render_document` gives, in turn; the last answer repeats. */
let documents: (() => Promise<string | null>)[] = [];

beforeEach(() => {
  mockWindows("document-0");
  commands = [];
  documents = [];
  mockIPC(
    (command) => {
      commands.push(command);
      if (command !== "render_document") return undefined;
      const next = documents.length > 1 ? documents.shift() : documents[0];
      return next?.();
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
  document.body.replaceChildren();
});

/** Lets the page's pending IPC calls, such as starting to listen, finish. */
function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function heading(): string | null | undefined {
  return document.querySelector("article.markdown-body h1")?.textContent;
}

function emptyWindow(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".empty-window");
}

test("a window without a document offers to open one", async () => {
  documents = [() => Promise.resolve(null)];
  await startDocumentView(0);

  const empty = emptyWindow();
  expect(empty?.hidden).toBe(false);
  expect(empty?.querySelector("button")?.textContent).toBe("Open file");
  expect(empty?.textContent).toContain("or drop a file here");

  empty?.querySelector("button")?.click();
  expect(commands).toContain("open_document");
});

test("a document shown in the window replaces the one before", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  expect(heading()).toBe("Lamplighter");
  await settle();

  await emit("document-shown");
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  expect(document.body.textContent).not.toContain("Lamplighter");
});

test("an empty window shows the document it is given", async () => {
  documents = [() => Promise.resolve(null), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();

  await emit("document-shown");
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  expect(emptyWindow()?.hidden).toBe(true);
});

test("of two documents shown in quick succession, the later one stays", async () => {
  let sendEarlier: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    () =>
      new Promise((resolve) => {
        sendEarlier = resolve;
      }),
    () => Promise.resolve(notes),
  ];
  await startDocumentView(0);
  await settle();

  await emit("document-shown");
  await emit("document-shown");
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  sendEarlier(readme);
  await settle();
  expect(heading()).toBe("Field notes");
});

test("a document that can't be read shows why", async () => {
  documents = [
    () =>
      Promise.reject(
        new Error(
          "Can't read C:\\docs\\gone.md: The system cannot find the file specified.",
        ),
      ),
  ];
  await startDocumentView(0);

  expect(document.querySelector("article")?.textContent).toContain(
    "Can't read C:\\docs\\gone.md",
  );
});

test("a notice from the app shows for a few seconds", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();

  vi.useFakeTimers();
  await emit("notice", "Downlight opens Markdown files (.md).");
  const notice = document.querySelector<HTMLElement>(".notice");
  expect(notice?.textContent).toBe("Downlight opens Markdown files (.md).");
  expect(notice?.getAttribute("role")).toBe("status");
  expect(notice?.hidden).toBe(false);

  vi.advanceTimersByTime(5000);
  expect(notice?.hidden).toBe(true);
});
