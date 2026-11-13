import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import {
  answerAsTheApp,
  giveAnotherDocument,
  layOut,
  linesOf,
  mockTheApp,
  nextShown,
  settle,
  type AtHeading,
  type Read,
} from "./test-support";
import { startDocumentView, whenDocumentShown } from "./view";

let commands: string[] = [];

/**
 * A `render_document` call that fails before the app can answer it, so `invoke` itself rejects,
 * with why as Tauri gives it.
 */
interface NoAnswer {
  noAnswer: string;
}

/**
 * What reading the window's document gives `render_document`, in turn: the HTML, the HTML at a
 * heading, or null for no document; or no answer at all. The last one repeats.
 */
let documents: (Read | NoAnswer)[] = [];

/** The page's text when it asked the app to show its window, or undefined until then. */
let textWhenShown: string | undefined;

beforeEach(() => {
  mockWindows("document-0");
  commands = [];
  documents = [];
  textWhenShown = undefined;
  mockTheApp(
    (command, args) => {
      commands.push(command);
      if (command === "show_window") {
        textWhenShown = document.body.textContent;
        return undefined;
      }
      if (command !== "render_document") return undefined;
      const next = documents.length > 1 ? documents.shift() : documents[0];
      if (typeof next === "object")
        // Tauri rejects a call it can't hand to the command (a denied or malformed one) with a
        // string, not an Error.
        // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors -- as Tauri does
        return Promise.reject(next.noAnswer);
      return next && answerAsTheApp(args, next);
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

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

  await giveAnotherDocument();
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  expect(document.body.textContent).not.toContain("Lamplighter");
});

test("an empty window shows the document it is given", async () => {
  documents = [() => Promise.resolve(null), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();

  await giveAnotherDocument();
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

  await giveAnotherDocument();
  await giveAnotherDocument();
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

test("the window shows once its document is in it", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);

  await vi.waitFor(() => {
    expect(textWhenShown).toContain("Lamplighter");
  });
  expect(commands.filter((c) => c === "show_window")).toHaveLength(1);
});

test("a window without a document shows once it offers to open one", async () => {
  documents = [() => Promise.resolve(null)];
  await startDocumentView(0);

  await vi.waitFor(() => {
    expect(textWhenShown).toContain("Open file");
  });
});

test("a window whose document can't be read shows once it says why", async () => {
  documents = [() => Promise.reject(new Error("Can't read C:\\docs\\gone.md"))];
  await startDocumentView(0);

  await vi.waitFor(() => {
    expect(textWhenShown).toContain("Can't read C:\\docs\\gone.md");
  });
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

/** The first line of each block the page scrolled to the top of the view, in turn. */
function spyOnScrolls(): () => number[] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () =>
    scrolls.mock.contexts.map((block) => linesOf(block as Element)?.[0] ?? 0);
}

/** Shows `first`, then reloads it as `reloaded` with the view's top at `topLine`. */
async function reload(
  first: string,
  reloaded: string,
  topLine: number,
): Promise<() => number[]> {
  documents = [() => Promise.resolve(first), () => Promise.resolve(reloaded)];
  await startDocumentView(0);
  await settle();
  layOut(topLine);
  const scrolledTo = spyOnScrolls();
  const shown = nextShown();
  await emit("document-changed", "reload");
  await shown;
  return scrolledTo;
}

function goneBar(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".file-gone");
}

test("a reload puts the block that was at the top of the view back at the top", async () => {
  // Halfway down "## Options", on line 15.
  const scrolledTo = await reload(readme, readme, 15.5);
  expect(scrolledTo().length).toBeGreaterThan(0);
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
});

test("a reload anchors to the innermost block crossing the top of the view", async () => {
  // Halfway down the list's second item, on line 8; the list starts on line 7.
  const scrolledTo = await reload(readme, readme, 8.5);
  expect(new Set(scrolledTo())).toEqual(new Set([8]));
});

test("a reload anchors to the last block starting before that line when none starts on it", async () => {
  const reloaded =
    '<p data-sourcepos="10:1-14:5">Long paragraph</p>\n<p data-sourcepos="20:1-20:5">Later</p>';
  const scrolledTo = await reload(readme, reloaded, 15.5);
  expect(new Set(scrolledTo())).toEqual(new Set([10]));
});

test("a reload at the top of the document stays at the top", async () => {
  const scrolledTo = await reload(readme, readme, 0);
  expect(scrolledTo()).toEqual([]);
});

test("a reload shows the file's new text", async () => {
  await reload(readme, notes, 15.5);
  expect(heading()).toBe("Field notes");
});

test("a reload that can't read the file keeps the last render and says why", async () => {
  documents = [
    () => Promise.resolve(readme),
    () => Promise.reject(new Error("Can't read C:\\docs\\readme.md: in use")),
  ];
  await startDocumentView(0);
  await settle();

  await emit("document-changed", "reload");
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toContain(
      "Can't read C:\\docs\\readme.md",
    );
  });
  expect(heading()).toBe("Lamplighter");
});

test("a deleted file leaves the last render under a 'File deleted or moved' bar", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();

  await emit("document-changed", "gone");
  const bar = goneBar();
  expect(bar?.hidden).toBe(false);
  expect(bar?.textContent).toBe("File deleted or moved");
  expect(bar?.getAttribute("role")).toBe("status");
  expect(heading()).toBe("Lamplighter");
});

test("a file that comes back reloads and the bar goes away", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();
  await emit("document-changed", "gone");

  const shown = nextShown();
  await emit("document-changed", "reload");
  await shown;
  expect(heading()).toBe("Field notes");
  expect(goneBar()?.hidden).toBe(true);
});

test("a file deleted while its reload is on the way keeps the bar", async () => {
  let sendReload: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    () =>
      new Promise((resolve) => {
        sendReload = resolve;
      }),
  ];
  await startDocumentView(0);
  await settle();

  const shown = nextShown();
  await emit("document-changed", "reload");
  await emit("document-changed", "gone");
  sendReload(notes);
  await shown;
  expect(heading()).toBe("Field notes");
  expect(goneBar()?.hidden).toBe(false);
});

test("showing another document removes the bar", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();
  await emit("document-changed", "gone");

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(heading()).toBe("Field notes");
  expect(goneBar()?.hidden).toBe(true);
});

test("a reload runs the whenDocumentShown listeners once its render is in place and anchored", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();
  const seen: { heading: string | null | undefined; scrolls: number }[] = [];
  const stop = whenDocumentShown(() => {
    seen.push({ heading: heading(), scrolls: scrolledTo().length });
  });

  await emit("document-changed", "reload");
  await vi.waitFor(() => {
    expect(seen).toHaveLength(1);
  });
  await settle();
  stop();
  expect(seen).toEqual([
    { heading: "Field notes", scrolls: scrolledTo().length },
  ]);
  expect(seen[0]?.scrolls).toBeGreaterThan(0);
});

test("showing another document runs the whenDocumentShown listeners", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();
  const headings: (string | null | undefined)[] = [];
  const stop = whenDocumentShown(() => {
    headings.push(heading());
  });

  await giveAnotherDocument();
  await vi.waitFor(() => {
    expect(headings).toEqual(["Field notes"]);
  });
  stop();
});

test("a reload overtaken by a reload that can't read the file still runs the whenDocumentShown listeners", async () => {
  documents = [
    () => Promise.resolve(readme),
    () => Promise.resolve(notes),
    () => Promise.reject(new Error("Can't read C:\\docs\\readme.md: in use")),
  ];
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  // The second reload starts while the first one anchors its render, which overtakes the first.
  let overtaken = false;
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(() => {
    if (overtaken) return;
    overtaken = true;
    void emit("document-changed", "reload");
  });
  const headings: (string | null | undefined)[] = [];
  const stop = whenDocumentShown(() => {
    headings.push(heading());
  });

  await emit("document-changed", "reload");
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toContain(
      "Can't read C:\\docs\\readme.md",
    );
  });
  await vi.waitFor(() => {
    expect(headings).toEqual(["Field notes"]);
  });
  stop();
});

test("a reload overtaken by a newer document-shown shows the newer document", async () => {
  let sendReload: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    () =>
      new Promise((resolve) => {
        sendReload = resolve;
      }),
    () => Promise.resolve(notes),
  ];
  await startDocumentView(0);
  await settle();

  await emit("document-changed", "reload");
  await giveAnotherDocument();
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  sendReload(readme);
  await settle();
  expect(heading()).toBe("Field notes");
});

/** The ids of the elements the page scrolled to the top of the view, in turn. */
function spyOnHeadingScrolls(): () => string[] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () => scrolls.mock.contexts.map((element) => (element as Element).id);
}

test("a reload for the document the window left, arriving after another was shown, shows the new one at its heading", async () => {
  // Ticket 23: the app sent `reload` for the document the window showed, and a link then gave the
  // window another one at a heading, so `reload` reaches the page after `document-shown`.
  let sendShown: (answer: AtHeading) => void = () => {};
  const atOptions = { document: readme, heading: "options" };
  documents = [
    () => Promise.resolve(notes),
    () =>
      new Promise((resolve) => {
        sendShown = resolve;
      }),
    () => Promise.resolve(atOptions),
  ];
  await startDocumentView(0);
  await settle();
  // The old document's reading position, partway down it.
  layOut(3.5);
  const scrolledTo = spyOnHeadingScrolls();

  const shown = nextShown();
  await giveAnotherDocument();
  await emit("document-changed", "reload");
  sendShown(atOptions);
  await shown;
  await settle();
  expect(heading()).toBe("Lamplighter");
  expect(new Set(scrolledTo())).toEqual(new Set(["options"]));
});

test("a 'gone' for the document the window left, arriving after another was shown, leaves no bar over the new one", async () => {
  // The watch on the old document reported its file gone just as the window was given another.
  let sendShown: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(notes),
    () =>
      new Promise((resolve) => {
        sendShown = resolve;
      }),
  ];
  await startDocumentView(0);
  await settle();

  const shown = nextShown();
  await giveAnotherDocument();
  await emit("document-changed", "gone");
  sendShown(readme);
  await shown;
  expect(heading()).toBe("Lamplighter");
  expect(goneBar()?.hidden).toBe(true);
});

test("document shown at a heading scrolls to that heading", async () => {
  documents = [
    () => Promise.resolve(notes),
    () => Promise.resolve({ document: readme, heading: "options" }),
  ];
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(heading()).toBe("Lamplighter");
  expect(scrolledTo().length).toBeGreaterThan(0);
  expect(new Set(scrolledTo())).toEqual(new Set(["options"]));
});

test("document shown without a heading starts at the top", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();
  const toTop = vi.spyOn(window, "scrollTo").mockImplementation(() => {});

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(toTop).toHaveBeenCalledWith(0, 0);
  expect(scrolledTo()).toEqual([]);
});

test("a new window opened at a heading scrolls to it on its first show", async () => {
  documents = [() => Promise.resolve({ document: readme, heading: "install" })];
  const scrolledTo = spyOnHeadingScrolls();

  await startDocumentView(0);
  expect(new Set(scrolledTo())).toEqual(new Set(["install"]));
  expect(commands).toContain("show_window");
});

test("scroll-to-heading from the app scrolls an open document to that heading", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();

  await emit("scroll-to-heading", "before-a-release");
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("before-a-release");
  });
  expect(heading()).toBe("Lamplighter");
});

test("a reload of a document shown at a heading keeps the reading position", async () => {
  // Shown at a heading first; the app answers the reload without it.
  const atOptions = { document: readme, heading: "options" };
  documents = [() => Promise.resolve(atOptions)];
  await startDocumentView(0);
  await settle();
  layOut(8.5);
  const scrolledTo = spyOnScrolls();

  const shown = nextShown();
  await emit("document-changed", "reload");
  await shown;
  expect(new Set(scrolledTo())).toEqual(new Set([8]));
});

test("a document that can't be read runs the whenDocumentShown listeners", async () => {
  documents = [
    () => Promise.resolve(readme),
    () => Promise.reject(new Error("Can't read gone.md: it is in use")),
  ];
  await startDocumentView(0);
  await settle();

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(document.querySelector("article")?.textContent).toContain(
    "Can't read gone.md",
  );
});

/** Why the app gives no answer in the tests below, as Tauri words a denied command. */
const NOT_ALLOWED = "render_document not allowed. Command not found";

test("a window whose app gives no answer for its first document shows why", async () => {
  documents = [{ noAnswer: NOT_ALLOWED }];
  const shown = nextShown();
  await startDocumentView(0);
  await shown;

  const view = document.querySelector<HTMLElement>("article.markdown-body");
  expect(view?.hidden).toBe(false);
  expect(view?.textContent).toBe(NOT_ALLOWED);
  expect(emptyWindow()?.hidden).toBe(true);
  expect(textWhenShown).toContain(NOT_ALLOWED);
});

test("a shown document stays when the app gives no answer, and a notice says why", async () => {
  documents = [() => Promise.resolve(readme), { noAnswer: NOT_ALLOWED }];
  await startDocumentView(0);
  await settle();

  await emit("document-changed", "reload");
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toBe(NOT_ALLOWED);
  });
  expect(heading()).toBe("Lamplighter");
});

test("a reload overtaken by a request the app gives no answer to still runs the whenDocumentShown listeners", async () => {
  documents = [
    () => Promise.resolve(readme),
    () => Promise.resolve(notes),
    { noAnswer: NOT_ALLOWED },
  ];
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  // The second request starts while the reload anchors its render, which overtakes the reload.
  let overtaken = false;
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(() => {
    if (overtaken) return;
    overtaken = true;
    void emit("document-changed", "reload");
  });
  const headings: (string | null | undefined)[] = [];
  const stop = whenDocumentShown(() => {
    headings.push(heading());
  });

  await emit("document-changed", "reload");
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toBe(NOT_ALLOWED);
  });
  await vi.waitFor(() => {
    expect(headings).toEqual(["Field notes"]);
  });
  stop();
});
