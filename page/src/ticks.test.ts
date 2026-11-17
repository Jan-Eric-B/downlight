import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import github from "../../core/tests/fixtures/github.html?raw";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startLinks } from "./links";
import { afterNextFrame } from "./reading-position";
import { startSettings } from "./settings";
import {
  answerAsTheApp,
  askedOfTheApp,
  fakeTheApp,
  forgetForwardAsTheApp,
  giveAnotherDocument,
  heading,
  nextInTurn,
  nextShown,
  saveTheFile,
  settle,
  stepAsTheApp,
  type Read,
} from "./test-support";
import { DOCUMENT_VIEW_SELECTOR, startDocumentView } from "./view";

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: Read[] = [];

let stop: AbortController;

beforeEach(async () => {
  mockWindows("document-0");
  documents = [];
  fakeTheApp({
    render_document: (args) => {
      const next = nextInTurn(documents);
      return next ? answerAsTheApp(args, next) : null;
    },
    step_history: stepAsTheApp,
    forget_forward: forgetForwardAsTheApp,
  });
  stop = new AbortController();
  await startSettings();
  startCommands(stop.signal);
  startLinks(stop.signal);
  startFind(stop.signal);
});

afterEach(() => {
  stop.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** The files the window reads, in turn, each as its HTML. */
function readInTurn(...html: string[]): void {
  documents = html.map((each) => () => Promise.resolve(each));
}

/** Resolves once the view has shown its first document and its IPC calls are done. */
async function started(): Promise<void> {
  await startDocumentView(0);
  await settle();
  await afterNextFrame();
  await settle();
}

/** The shown document's task list checkboxes, in the document's order. */
function taskItems(): HTMLInputElement[] {
  return [
    ...document.querySelectorAll<HTMLInputElement>(
      `${DOCUMENT_VIEW_SELECTOR} input.task-list-item-checkbox`,
    ),
  ];
}

/** Whether each task item shows ticked, in the document's order. */
function ticked(): boolean[] {
  return taskItems().map((item) => item.checked);
}

/** Clicks the task item at `index`, as the reader would. */
function clickTaskItem(index: number): void {
  const item = taskItems()[index];
  if (!item) throw new Error(`The document has no task item ${String(index)}.`);
  item.click();
}

/** Resolves once `change` has run and the document it brings is in place. */
async function shownAfter(change: () => Promise<unknown>): Promise<void> {
  const shown = nextShown();
  await change();
  await shown;
}

/** Presses `key`, with Alt if `alt`, on the element that has the focus, as the reader would. */
function press(key: string, alt = false): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    key,
    altKey: alt,
    bubbles: true,
    cancelable: true,
  });
  (document.activeElement ?? document.body).dispatchEvent(event);
  return event;
}

/** Back, as Alt+Left gives it, once the document it brings is in place. */
function back(): Promise<void> {
  return shownAfter(() => Promise.resolve(press("ArrowLeft", true)));
}

/** Forward, as Alt+Right gives it, once the document it brings is in place. */
function forward(): Promise<void> {
  return shownAfter(() => Promise.resolve(press("ArrowRight", true)));
}

// readme's task items: "Update the changelog" ([x] in the file), "Tag the version" and "Publish
// the notes". github's are four, the first [x].

test("a click ticks a task item, and another click unticks it", async () => {
  readInTurn(readme);
  await started();
  expect(ticked()).toEqual([true, false, false]);

  clickTaskItem(1);
  expect(ticked()).toEqual([true, true, false]);
  clickTaskItem(1);
  expect(ticked()).toEqual([true, false, false]);
});

test("a click unticks a task item the file has ticked", async () => {
  readInTurn(readme);
  await started();

  clickTaskItem(0);
  expect(ticked()).toEqual([false, false, false]);
});

test("a task item takes the keyboard's focus, and the page leaves Space to tick it", async () => {
  readInTurn(readme);
  await started();
  const item = taskItems()[1];

  item?.focus();
  expect(document.activeElement).toBe(item);
  // The webview ticks a focused checkbox on Space with a click of its own, which no page test can
  // give, so this checks that nothing in the page takes the key from it.
  expect(press(" ").defaultPrevented).toBe(false);
});

test("ticking a task item asks nothing of the app, so the file is never changed", async () => {
  readInTurn(readme);
  await started();
  const askedBefore = askedOfTheApp();

  clickTaskItem(1);
  clickTaskItem(0);
  await settle();
  expect(askedOfTheApp()).toEqual(askedBefore);
});

test("a reload with the same number of task items keeps the ticks", async () => {
  readInTurn(readme, readme);
  await started();
  clickTaskItem(0);
  clickTaskItem(1);

  await shownAfter(saveTheFile);
  expect(ticked()).toEqual([false, true, false]);
});

test("a reload with a different number of task items clears the ticks", async () => {
  readInTurn(readme, github);
  await started();
  clickTaskItem(1);

  await shownAfter(saveTheFile);
  expect(ticked()).toEqual([true, false, false, false]);
});

test("a reload shows what the file now says of the task items the reader didn't tick", async () => {
  const publishedInTheFile = readme.replace(
    '<input type="checkbox" class="task-list-item-checkbox" disabled=""> Publish the notes',
    '<input type="checkbox" class="task-list-item-checkbox" checked="" disabled=""> Publish the notes',
  );
  expect(publishedInTheFile).not.toBe(readme);
  readInTurn(readme, publishedInTheFile);
  await started();
  clickTaskItem(1);

  await shownAfter(saveTheFile);
  expect(ticked()).toEqual([true, true, true]);
});

test("Back restores the ticks the document had when the window left it", async () => {
  readInTurn(readme, notes, readme);
  await started();
  clickTaskItem(0);
  clickTaskItem(2);
  await shownAfter(giveAnotherDocument);
  expect(heading()).toBe("Field notes");

  await back();
  expect(heading()).toBe("Lamplighter");
  expect(ticked()).toEqual([false, false, true]);
});

test("Forward restores the ticks of the document Back left, and Back again those of the one before", async () => {
  readInTurn(readme, github, readme, github, readme);
  await started();
  clickTaskItem(1);
  await shownAfter(giveAnotherDocument);
  clickTaskItem(3);

  await back();
  expect(ticked()).toEqual([true, true, false]);
  clickTaskItem(2);
  await forward();
  expect(ticked()).toEqual([true, false, false, true]);
  await back();
  expect(ticked()).toEqual([true, true, true]);
});

test("Back to a document whose number of task items changed meanwhile shows it without ticks", async () => {
  // The file of the first document gains a task item while the window shows the second.
  readInTurn(readme, notes, github);
  await started();
  clickTaskItem(1);
  await shownAfter(giveAnotherDocument);

  await back();
  expect(ticked()).toEqual([true, false, false, false]);
});

test("a document shown anew, as when it is opened again, starts without ticks", async () => {
  readInTurn(readme, notes, readme);
  await started();
  clickTaskItem(1);
  await shownAfter(giveAnotherDocument);

  await shownAfter(giveAnotherDocument);
  expect(heading()).toBe("Lamplighter");
  expect(ticked()).toEqual([true, false, false]);
});
