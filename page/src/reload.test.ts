import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import {
  deleteTheFile,
  fakeTheAppReading,
  heading,
  layOut,
  linesOf,
  nextShown,
  saveTheFile,
  scroller,
  settle,
  type NoAnswer,
  type Read,
} from "./test-support";
import { requestDocument, startDocumentView, whenDocumentShown } from "./view";

// A reload shows the document's new render at the same reading position (ticket 23).

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: (Read | NoAnswer)[] = [];

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  fakeTheAppReading(() => documents);
});

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
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
