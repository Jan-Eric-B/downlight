import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import {
  answerAsTheApp,
  deleteTheFile,
  askedOfTheApp,
  emptyWindow,
  giveAnotherDocument,
  goneBar,
  heading,
  layOut,
  linesOf,
  fakeTheApp,
  nextInTurn,
  nextShown,
  saveTheFile,
  scrollAsTheApp,
  settle,
  spyOnHeadingScrolls,
  spyOnScrollsToTop,
  tell,
  type AtHeading,
  type Read,
} from "./test-support";
import { afterNextFrame } from "./reading-position";
import {
  requestDocument,
  SCROLLER_SELECTOR,
  startDocumentView,
  whenDocumentShown,
  type ShownReason,
} from "./view";

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
  documents = [];
  textWhenShown = undefined;
  fakeTheApp({
    show_window: () => {
      textWhenShown = document.body.textContent;
      return null;
    },
    render_document: (args) => {
      const next = nextInTurn(documents);
      if (typeof next === "object")
        // Tauri rejects a call it can't hand to the command (a denied or malformed one) with a
        // string, not an Error.
        // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors -- as Tauri does
        return Promise.reject(next.noAnswer);
      // No document given yet answers as a window that shows none.
      return next ? answerAsTheApp(args, next) : null;
    },
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

test("a window without a document offers to open one", async () => {
  documents = [() => Promise.resolve(null)];
  await startDocumentView(0);

  const empty = emptyWindow();
  expect(empty?.hidden).toBe(false);
  expect(empty?.querySelector("button")?.textContent).toBe("Open file");
  expect(empty?.textContent).toContain("or drop a file here");

  empty?.querySelector("button")?.click();
  expect(askedOfTheApp()).toContain("open_document");
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
  expect(askedOfTheApp().filter((c) => c === "show_window")).toHaveLength(1);
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

/** The element the document scrolls in, below the title bar. */
function scroller(): HTMLElement | null {
  return document.querySelector<HTMLElement>(SCROLLER_SELECTOR);
}

test("the document scrolls in its own element, which holds the view", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);

  expect(scroller()?.contains(document.querySelector("article"))).toBe(true);
});

test("the document has the keys from the start, so they scroll it", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);

  expect(document.activeElement).toBe(scroller());
});

test("the keys go back to the document when the focus leaves a part of the page for nowhere", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);
  // Find's box, which Esc blurs.
  const box = document.createElement("input");
  document.body.append(box);
  box.focus();

  box.blur();
  await settle();
  expect(document.activeElement).toBe(scroller());
});

test("a notice from the app shows for a few seconds", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();

  vi.useFakeTimers();
  await tell("notice", "Downlight opens Markdown files (.md).");
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
  await saveTheFile();
  await shown;
  return scrolledTo;
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

test("a reload never anchors to a footnote that starts before the reading position", async () => {
  // A footnote's definition on line 5, and the reader at the paragraph on line 7. A line added to
  // the first paragraph moves that paragraph to line 8 and the footnote to line 6, while the
  // footnotes still come last in the view.
  const first = `<p data-sourcepos="1:1-1:5">First</p>
<p data-sourcepos="3:1-3:6">Second<sup class="footnote-ref"><a href="#fn-1" id="fnref-1" data-footnote-ref="">1</a></sup></p>
<p data-sourcepos="7:1-7:5">Third</p>
<section data-sourcepos="5:1-5:10" class="footnotes" data-footnotes="">
<ol>
<li data-sourcepos="5:1-5:10" id="fn-1">
<p data-sourcepos="5:7-5:10">note</p>
</li>
</ol>
</section>`;
  const reloaded = `<p data-sourcepos="1:1-2:4">First
more</p>
<p data-sourcepos="4:1-4:6">Second<sup class="footnote-ref"><a href="#fn-1" id="fnref-1" data-footnote-ref="">1</a></sup></p>
<p data-sourcepos="8:1-8:5">Third</p>
<section data-sourcepos="6:1-6:10" class="footnotes" data-footnotes="">
<ol>
<li data-sourcepos="6:1-6:10" id="fn-1">
<p data-sourcepos="6:7-6:10">note</p>
</li>
</ol>
</section>`;
  const scrolledTo = await reload(first, reloaded, 7.5);
  expect(new Set(scrolledTo())).toEqual(new Set([4]));
});

test("a reload in the footnotes anchors to the footnote that starts on the reading position", async () => {
  // The footnote's definition is the file's last line, so the fake layout, which goes by lines,
  // puts the footnotes last, as the view does.
  const withFootnote = `<p data-sourcepos="1:1-1:6">First<sup class="footnote-ref"><a href="#fn-1" id="fnref-1" data-footnote-ref="">1</a></sup></p>
<p data-sourcepos="3:1-3:5">Second</p>
<section data-sourcepos="5:1-5:10" class="footnotes" data-footnotes="">
<ol>
<li data-sourcepos="5:1-5:10" id="fn-1">
<p data-sourcepos="5:7-5:10">note</p>
</li>
</ol>
</section>`;
  const scrolledTo = await reload(withFootnote, withFootnote, 5.5);
  expect(new Set(scrolledTo())).toEqual(new Set([5]));
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

  await saveTheFile();
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

  await deleteTheFile();
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
  await deleteTheFile();

  const shown = nextShown();
  await saveTheFile();
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
  await saveTheFile();
  await deleteTheFile();
  sendReload(notes);
  await shown;
  expect(heading()).toBe("Field notes");
  expect(goneBar()?.hidden).toBe(false);
});

test("showing another document removes the bar", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0);
  await settle();
  await deleteTheFile();

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

  await saveTheFile();
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
    void saveTheFile();
  });
  const headings: (string | null | undefined)[] = [];
  const stop = whenDocumentShown(() => {
    headings.push(heading());
  });

  await saveTheFile();
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

  await saveTheFile();
  await giveAnotherDocument();
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  sendReload(readme);
  await settle();
  expect(heading()).toBe("Field notes");
});

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
  await saveTheFile();
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
  const left = await giveAnotherDocument();
  await deleteTheFile(left);
  sendShown(readme);
  await shown;
  expect(heading()).toBe("Lamplighter");
  expect(goneBar()?.hidden).toBe(true);
});

test("a 'gone' for the document the window left, arriving once the new one is in view, leaves no bar", async () => {
  // Ticket 26: the `gone` names the showing it is about, so the page needs no clearing on each
  // show to drop a stale one, however late it comes.
  documents = [() => Promise.resolve(notes), () => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();

  const shown = nextShown();
  const left = await giveAnotherDocument();
  await shown;
  await deleteTheFile(left);
  expect(heading()).toBe("Lamplighter");
  expect(goneBar()?.hidden).toBe(true);
});

test("a 'gone' for the document the window was just given, arriving before it is in view, puts the bar over it", async () => {
  // Ticket 26: the file was deleted just after its read, and the `gone` reached the page while its
  // view still showed the old document.
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
  await deleteTheFile();
  expect(goneBar()?.hidden).toBe(true);
  sendShown(readme);
  await shown;
  expect(heading()).toBe("Lamplighter");
  expect(goneBar()?.hidden).toBe(false);
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
  const scrollsToTop = spyOnScrollsToTop();

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(scrollsToTop()).toBe(1);
  expect(scrolledTo()).toEqual([]);
});

test("a new window opened at a heading scrolls to it on its first show", async () => {
  documents = [() => Promise.resolve({ document: readme, heading: "install" })];
  const scrolledTo = spyOnHeadingScrolls();

  await startDocumentView(0);
  expect(new Set(scrolledTo())).toEqual(new Set(["install"]));
  expect(askedOfTheApp()).toContain("show_window");
});

test("scroll-to-heading from the app scrolls an open document to that heading", async () => {
  documents = [() => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();

  await scrollAsTheApp("before-a-release");
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("before-a-release");
  });
  expect(heading()).toBe("Lamplighter");
});

test("scroll-to-heading for the document the window was just given, arriving before it is in view, scrolls it once it is", async () => {
  // From the review of ticket 12: the app sends it once the page has asked for that document, and
  // a large render can still be on its way then, so it was looked up in the old document.
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
  const scrolledTo = spyOnHeadingScrolls();

  const shown = nextShown();
  await giveAnotherDocument();
  await scrollAsTheApp("options");
  expect(scrolledTo()).toEqual([]);
  sendShown(readme);
  await shown;
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("options");
  });
  expect(heading()).toBe("Lamplighter");
});

test("scroll-to-heading for the document the window left, arriving once another is in view, scrolls nothing", async () => {
  // From the review of ticket 12: it was looked up in the document in view instead.
  documents = [() => Promise.resolve(notes), () => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();
  const shown = nextShown();
  const left = await giveAnotherDocument();
  await shown;
  const scrolledTo = spyOnHeadingScrolls();

  await scrollAsTheApp("options", left);
  await settle();
  expect(scrolledTo()).toEqual([]);
});

/** A document with a heading in a closed section and one in that section's summary. */
const withClosedSection = `<h1 id="guide" data-sourcepos="1:1-1:7">Guide</h1>
<details>
<summary><h2 id="more" data-sourcepos="4:10-4:16">More</h2></summary>
<h2 id="hidden-step" data-sourcepos="6:1-6:14">Hidden step</h2>
</details>`;

test("scroll-to-heading into a closed section opens it", async () => {
  documents = [() => Promise.resolve(withClosedSection)];
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();
  const section = document.querySelector("details");
  expect(section?.open).toBe(false);

  await scrollAsTheApp("hidden-step");
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("hidden-step");
  });
  expect(section?.open).toBe(true);
});

test("a document shown at a heading in a closed section opens it", async () => {
  documents = [
    () =>
      Promise.resolve({ document: withClosedSection, heading: "hidden-step" }),
  ];
  await startDocumentView(0);
  expect(document.querySelector("details")?.open).toBe(true);
});

test("scroll-to-heading into a closed section's summary leaves it closed", async () => {
  documents = [() => Promise.resolve(withClosedSection)];
  await startDocumentView(0);
  await settle();
  const scrolledTo = spyOnHeadingScrolls();

  await scrollAsTheApp("more");
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("more");
  });
  expect(document.querySelector("details")?.open).toBe(false);
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
  await saveTheFile();
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

  await saveTheFile();
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
    void saveTheFile();
  });
  const headings: (string | null | undefined)[] = [];
  const stop = whenDocumentShown(() => {
    headings.push(heading());
  });

  await saveTheFile();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toBe(NOT_ALLOWED);
  });
  await vi.waitFor(() => {
    expect(headings).toEqual(["Field notes"]);
  });
  stop();
});

test("the whenDocumentShown listeners hear whether another document was shown or the document reloaded", async () => {
  documents = [
    () => Promise.resolve(readme),
    () => Promise.resolve(notes),
    () => Promise.resolve(notes),
  ];
  await startDocumentView(0);
  await settle();
  const reasons: ShownReason[] = [];
  const stop = whenDocumentShown(({ reason }) => {
    reasons.push(reason);
  });

  let shown = nextShown();
  await saveTheFile();
  await shown;
  shown = nextShown();
  await giveAnotherDocument();
  await shown;
  stop();
  expect(reasons).toEqual(["reload", "shown"]);
});

test("a reload overtaken after its swap by a reload that can't read the file is heard as a reload", async () => {
  documents = [
    () => Promise.resolve(readme),
    () => Promise.resolve(notes),
    () => Promise.reject(new Error("Can't read C:\\docs\\readme.md: in use")),
  ];
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  let overtaken = false;
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(() => {
    if (overtaken) return;
    overtaken = true;
    void saveTheFile();
  });
  const reasons: ShownReason[] = [];
  const stop = whenDocumentShown(({ reason }) => {
    reasons.push(reason);
  });

  await saveTheFile();
  await vi.waitFor(() => {
    expect(reasons).toEqual(["reload"]);
  });
  stop();
});

test("asking for the document again reloads it at the reading position", async () => {
  let reads = 0;
  documents = [
    () => {
      reads += 1;
      return Promise.resolve(readme);
    },
  ];
  await startDocumentView(0);
  await settle();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();

  await requestDocument();
  expect(reads).toBe(2);
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
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
  expect(textWhenShown).toContain("Lamplighter");
  expect(heading()).toBe("Lamplighter");
});

test("a reload under the 'File deleted or moved' bar keeps the block just below the bar", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(readme)];
  await startDocumentView(0);
  await settle();
  await deleteTheFile();
  // The page's stylesheet, which the tests don't load, sets this on the document's scroller while
  // the bar shows.
  scroller()?.style.setProperty("scroll-padding-top", "41px");
  // Line 13.5 is at the top of the scroller, below the title bar, and "## Options" on line 15 at
  // the "File deleted or moved" bar's lower edge.
  layOut(13.5);
  const scrolledTo = spyOnScrolls();

  const shown = nextShown();
  await saveTheFile();
  await shown;
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
});
