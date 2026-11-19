import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import find from "../../core/tests/fixtures/find.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { ArgsOf } from "./app";
import { startCommands } from "./commands";
import { startSettings } from "./settings";
import { fakeTheAppReading, layOut, settle } from "./test-support";
import { startDocumentView } from "./view";

// Edit opens the document in the editor at the reading position's line (ticket 35).

let stop: AbortController;

/** Each `edit_document` call's arguments, in turn. */
let edited: ArgsOf<"edit_document">[];

beforeEach(() => {
  mockWindows("document-0");
  edited = [];
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** Shows the rendered document `html` in the window, whose app keeps each Edit's arguments. */
async function show(html: string): Promise<void> {
  fakeTheAppReading(() => [() => Promise.resolve(html)], {
    edit_document: (args) => {
      edited.push(args);
      return null;
    },
  });
  await startSettings();
  startCommands(stop.signal);
  await startDocumentView(0);
  await settle();
}

/** Presses Ctrl+E, as the reader would. */
function pressEdit(): void {
  document.body.dispatchEvent(
    new KeyboardEvent("keydown", {
      ctrlKey: true,
      key: "e",
      code: "KeyE",
      bubbles: true,
      cancelable: true,
    }),
  );
}

test("Edit sends the line of the block at the top of the view", async () => {
  await show(readme);
  // Halfway down "## Options", on line 15.
  layOut(15.5);
  pressEdit();
  expect(edited).toEqual([{ line: 15 }]);
});

test("Edit sends the innermost block's line", async () => {
  await show(readme);
  // Halfway down the list's second item, on line 8; the list starts on line 7.
  layOut(8.5);
  pressEdit();
  expect(edited).toEqual([{ line: 8 }]);
});

test("Edit at the start of the document sends the first line", async () => {
  await show(readme);
  layOut(1);
  pressEdit();
  expect(edited).toEqual([{ line: 1 }]);
});

test("Edit in raw HTML, which has no line number, sends no line", async () => {
  await show(find);
  // Halfway down the raw HTML summary "Storing" on line 21, in the raw HTML details on lines 20
  // to 25. The paragraph inside has line 23, which would be a wrong line to open at.
  layOut(21.5, { details: [20, 25], summary: [21, 21] });
  pressEdit();
  expect(edited).toEqual([{ line: null }]);
});
