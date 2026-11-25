// The page's side of the `copy_markdown.json` contract samples (ticket 27, beside
// `contract.test.ts`): the pieces the page sends for Copy as Markdown are checked against the
// app's samples of the pieces it takes, so that a change to a piece's shape on either side fails
// a test.
import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { Piece } from "./app";
import { startCommands } from "./commands";
import { startSettings } from "./settings";
import {
  copyMarkdownSamples,
  fakeTheAppReading,
  keysSent,
  settle,
  tell,
} from "./test-support";
import { startDocumentView } from "./view";

let stopPage: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  stopPage = new AbortController();
});

afterEach(() => {
  stopPage.abort();
  document.getSelection()?.removeAllRanges();
  clearMocks();
  document.body.replaceChildren();
});

/** The kind of each piece and the keys it reaches the app with, such as `lines: first, last, piece`. */
function piecesShape(pieces: Piece[]): string[] {
  return pieces.map((piece) => `${piece.piece}: ${keysSent(piece).join(", ")}`);
}

test("the page's pieces for Copy as Markdown are those the app takes, each with the keys it takes", async () => {
  const sent: Piece[][] = [];
  fakeTheAppReading(() => [() => Promise.resolve(readme)], {
    copy_markdown: ({ pieces }) => {
      sent.push(pieces);
      return null;
    },
  });
  await startSettings();
  startCommands(stopPage.signal);
  await startDocumentView(0, stopPage.signal);
  await settle();

  // Nothing selected, for the whole file, then a paragraph's lines, then a table cell's span.
  await tell("menu-command", "copyAsMarkdown");
  const paragraph = document.querySelector("article.markdown-body p");
  if (!paragraph) throw new Error("The document has no paragraph.");
  document.getSelection()?.selectAllChildren(paragraph);
  await tell("menu-command", "copyAsMarkdown");
  const cell = document.querySelector("article.markdown-body td");
  if (!cell) throw new Error("The document has no table cell.");
  document.getSelection()?.selectAllChildren(cell);
  await tell("menu-command", "copyAsMarkdown");
  expect(sent.map(piecesShape).toSorted()).toEqual(
    Object.values(copyMarkdownSamples()).map(piecesShape).toSorted(),
  );
});
