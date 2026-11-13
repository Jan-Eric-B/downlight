import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { showNotice } from "./notice";

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});

afterEach(() => {
  // Lets the last notice's timer hide it, so no test inherits another's lines.
  vi.runAllTimers();
  vi.useRealTimers();
  document.body.replaceChildren();
});

function notice(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".notice");
}

/** The messages the notice shows, top to bottom, or none while it is hidden. */
function shownLines(): string[] {
  const shown = notice();
  if (!shown || shown.hidden) return [];
  return [...shown.children].map((line) => line.textContent);
}

test("a notice shows its message for 4 s", () => {
  showNotice("Downlight opens Markdown files (.md).");
  expect(shownLines()).toEqual(["Downlight opens Markdown files (.md)."]);
  expect(notice()?.getAttribute("role")).toBe("status");

  vi.advanceTimersByTime(3999);
  expect(shownLines()).toEqual(["Downlight opens Markdown files (.md)."]);
  vi.advanceTimersByTime(1);
  expect(shownLines()).toEqual([]);
});

test("a second notice within 4 s shows below the first, and both stay 4 s after the second", () => {
  showNotice("Can't watch a.md for changes.");
  vi.advanceTimersByTime(3000);
  showNotice("Downlight opens Markdown files (.md).");
  expect(shownLines()).toEqual([
    "Can't watch a.md for changes.",
    "Downlight opens Markdown files (.md).",
  ]);

  vi.advanceTimersByTime(3999);
  expect(shownLines()).toHaveLength(2);
  vi.advanceTimersByTime(1);
  expect(shownLines()).toEqual([]);
});

test("a notice shows at most 3 messages, dropping the oldest", () => {
  for (const message of ["one", "two", "three", "four"]) showNotice(message);
  expect(shownLines()).toEqual(["two", "three", "four"]);
});

test("a notice after the last one has gone shows alone", () => {
  showNotice("one");
  vi.advanceTimersByTime(4000);
  showNotice("two");
  expect(shownLines()).toEqual(["two"]);
});
