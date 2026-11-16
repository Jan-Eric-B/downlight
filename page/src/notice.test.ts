import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Notice } from "./app";
import { showNotice } from "./notice";
import { fakeTheApp } from "./test-support";

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  fakeTheApp();
});

afterEach(() => {
  // Lets the last notice's timer hide it, so no test inherits another's lines.
  vi.runAllTimers();
  vi.useRealTimers();
  clearMocks();
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

/** A notice that names `delete-me.md`, missing from `folder`, as the app sends it. */
function missingFile(
  folderOpens: boolean,
  folder: string | null = "C:\\Users\\me\\docs",
): Notice {
  return {
    before: "Can't find ",
    file: { name: "delete-me.md", folder, folderOpens },
    after: ".",
  };
}

/** The notice's link to a folder, if it has one. */
function folderLink(): HTMLElement | null {
  return document.querySelector<HTMLElement>('.notice [role="link"]');
}

test("a notice that names a file shows its name in bold and its folder after it", () => {
  showNotice(missingFile(false));

  expect(shownLines()).toEqual([
    "Can't find delete-me.md in C:\\Users\\me\\docs.",
  ]);
  expect(document.querySelector(".notice strong")?.textContent).toBe(
    "delete-me.md",
  );
});

test("the folder of a missing file is a link, reachable with Tab, that names what it does", () => {
  showNotice(missingFile(true));

  const link = folderLink();
  expect(link?.textContent).toBe("C:\\Users\\me\\docs");
  expect(link?.tabIndex).toBe(0);
  expect(link?.getAttribute("aria-label")).toBe(
    "Open the folder C:\\Users\\me\\docs",
  );
});

test("a folder that is gone too is plain text, and nothing in the notice is a link", () => {
  showNotice(missingFile(false));

  expect(folderLink()).toBeNull();
  expect(notice()?.querySelector("a, button, [tabindex]")).toBeNull();
});

test("a file without a folder, such as a drive's root, shows only its name", () => {
  showNotice(missingFile(false, null));

  expect(shownLines()).toEqual(["Can't find delete-me.md."]);
});

test("a click on the folder, or Enter on it, asks the app to open the folder the notice named", async () => {
  const opened: string[] = [];
  fakeTheApp({
    open_named_folder: ({ folder }) => {
      opened.push(folder);
      return null;
    },
  });
  showNotice(missingFile(true));

  folderLink()?.click();
  folderLink()?.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
  );
  await vi.waitFor(() => {
    expect(opened).toEqual(["C:\\Users\\me\\docs", "C:\\Users\\me\\docs"]);
  });
});

test("a folder the app can't open shows why", async () => {
  fakeTheApp({
    open_named_folder: () => {
      throw new Error("Can't find docs in C:\\Users\\me.");
    },
  });
  showNotice(missingFile(true));

  folderLink()?.click();
  await vi.waitFor(() => {
    expect(shownLines().at(-1)).toContain("Can't find docs in C:\\Users\\me.");
  });
});

test("a press on the folder leaves the focus where it is", () => {
  showNotice(missingFile(true));

  const press = new MouseEvent("mousedown", {
    bubbles: true,
    cancelable: true,
  });
  folderLink()?.dispatchEvent(press);
  expect(press.defaultPrevented).toBe(true);
});

test("a long folder breaks only after its separators, and the name never", () => {
  showNotice(missingFile(true, "C:\\Users\\me\\long-folder-name\\docs"));

  const unbreakable = [
    ...(notice()?.querySelectorAll<HTMLElement>(".unbreakable") ?? []),
  ].map((part) => part.textContent);
  expect(unbreakable).toEqual([
    "delete-me.md",
    "C:\\",
    "Users\\",
    "me\\",
    "long-folder-name\\",
    "docs",
  ]);
  // A break opportunity between each two parts of the folder, and nowhere else.
  expect(folderLink()?.querySelectorAll("wbr")).toHaveLength(4);
});

test("a notice with a link stays while the pointer is on it, and goes 4 s after it leaves", () => {
  showNotice(missingFile(true));

  notice()?.dispatchEvent(new PointerEvent("pointerenter"));
  vi.advanceTimersByTime(10_000);
  expect(shownLines()).toHaveLength(1);

  notice()?.dispatchEvent(new PointerEvent("pointerleave"));
  vi.advanceTimersByTime(3999);
  expect(shownLines()).toHaveLength(1);
  vi.advanceTimersByTime(1);
  expect(shownLines()).toEqual([]);
});

test("a notice with a link stays while its link has the focus, and goes 4 s after it leaves", () => {
  showNotice(missingFile(true));

  folderLink()?.focus();
  vi.advanceTimersByTime(10_000);
  expect(shownLines()).toHaveLength(1);

  folderLink()?.blur();
  vi.advanceTimersByTime(3999);
  expect(shownLines()).toHaveLength(1);
  vi.advanceTimersByTime(1);
  expect(shownLines()).toEqual([]);
});

test("a notice without a link goes after 4 s with the pointer on it", () => {
  showNotice(missingFile(false));

  notice()?.dispatchEvent(new PointerEvent("pointerenter"));
  vi.advanceTimersByTime(4000);
  expect(shownLines()).toEqual([]);
});
