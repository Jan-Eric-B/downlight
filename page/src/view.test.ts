import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import {
  deleteTheFile,
  askedOfTheApp,
  emptyWindow,
  fakeTheAppReading,
  giveAnotherDocument,
  goneBar,
  heading,
  layOut,
  nextShown,
  saveTheFile,
  scrollAsTheApp,
  scroller,
  settle,
  spyOnHeadingScrolls,
  spyOnScrollsToTop,
  tell,
  type AtHeading,
  type FakeDocumentApp,
  type NoAnswer,
  type Read,
} from "./test-support";
import { startDocumentView, whenDocumentShown, type ShownReason } from "./view";

// The document view's own tests. Those of the window's first show, the wait note and the reload
// are in `first-show.test.ts`, `wait-note.test.ts` and `reload.test.ts`.

/**
 * What reading the window's document gives `render_document`, in turn: the HTML, the HTML at a
 * heading, or null for no document; or no answer at all. The last one repeats.
 */
let documents: (Read | NoAnswer)[] = [];

let app: FakeDocumentApp;

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  app = fakeTheAppReading(() => documents);
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
  expect(empty?.querySelector("p")?.textContent).toBe(
    "or drop a file here, or paste Markdown (Ctrl+V)",
  );

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
  expect(app.textWhenShown()).toContain(NOT_ALLOWED);
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
