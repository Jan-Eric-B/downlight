import { afterEach, beforeEach, expect, test } from "vitest";
import headings from "../../core/tests/fixtures/headings.html?raw";
import { DEFAULT_SETTINGS, type PageSettings } from "./app";
import { afterNextFrame } from "./reading-position";
import {
  clickStrip,
  closeRulerWindow,
  currentMarkTop,
  documentView,
  drawn,
  findStatus,
  LONG,
  marks,
  onStrip,
  openRulerWindow,
  press,
  readInTurn,
  readSentLater,
  resize,
  showInTurn,
  spyOnScrolls,
  startShowing,
  typeInFind,
} from "./overview-ruler-test-support";
import {
  giveAnotherDocument,
  layOut,
  nextShown,
  saveTheFile,
  scroller,
  settle,
  spyOnHeadingScrolls,
  tell,
} from "./test-support";
import { canStepThroughHistory, stepThroughHistory } from "./view";

// Clicking and hovering the overview ruler's heading marks (overview ruler ticket 04): a click
// goes to the heading as a heading link does, and hovering names it. The layout fake makes each
// line of the file 20 px high, and the scroller 736 px high.

beforeEach(openRulerWindow);

afterEach(closeRulerWindow);

/** Resolves once a scroll the page just started has landed, as a heading link's does. */
async function landed(): Promise<void> {
  await settle();
  await afterNextFrame();
  await settle();
}

/**
 * Moves the pointer over the strip `row` px below its top, in its left or right half, with the
 * mouse buttons `buttons` holds held down, none unless it says.
 */
function hover(half: "left" | "right", row: number, buttons = 0): void {
  documentView().dispatchEvent(
    new MouseEvent("mousemove", { ...onStrip(half, row), buttons }),
  );
}

/** The hover label while it shows. */
function shownLabel(): HTMLElement | undefined {
  const label = document.querySelector<HTMLElement>(".overview-ruler-label");
  return label && !label.closest("[hidden]") ? label : undefined;
}

/** The lines of the hover label while it shows, or undefined while it doesn't. */
function hoverLabel(): string[] | undefined {
  const label = shownLabel();
  return label && [...label.children].map((line) => line.textContent);
}

/** The pointer the reader sees over the strip, where the document view lies under it. */
function pointer(): string {
  return getComputedStyle(documentView()).cursor;
}

test("a click on a heading mark goes to its heading, as a heading link does, and Back returns to where the view was", async () => {
  // The reader is at the h2 on line 11, row 73, and clicks the mark of the one on line 51, row 368.
  layOut(11);
  showInTurn(
    LONG.replace(
      "\n",
      '\n<h2 id="first-way" data-sourcepos="11:1-11:9">First way</h2>\n',
    ),
  );
  await startShowing();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  const pressed = clickStrip("left", 369);
  await landed();
  expect(pressed.defaultPrevented).toBe(true);
  expect(scrolledTo().at(-1)).toBe("half-way");
  expect(canStepThroughHistory("back")).toBe(true);

  stepThroughHistory("back");
  await landed();
  expect(scrolledTo().at(-1)).toBe("first-way");
  expect(canStepThroughHistory("back")).toBe(false);
});

test("a click on the mark of a heading in a closed section opens the section and goes there", async () => {
  const rawHtml: Record<string, [number, number]> = {
    details: [39, 46],
    "h2:not([id])": [48, 48],
  };
  layOut(1, rawHtml);
  showInTurn(headings);
  await startShowing();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true, rawHtml });

  // The closed section's mark is in row 499.
  clickStrip("left", 500);
  await landed();
  expect(scrolledTo().at(-1)).toBe("oil-in-a-closed-section");
  expect(document.querySelector("details")?.open).toBe(true);
});

/**
 * A document of 1000 lines, 20,000 px, about 27 px of it in each px of the strip: the h4 on line
 * 12 and the h2 on line 13 share one mark in row 8.
 */
const MERGED = [
  '<h4 id="small" data-sourcepos="12:1-12:5">Small</h4>',
  '<h2 id="large" data-sourcepos="13:1-13:5">Large</h2>',
  '<h3 id="alone" data-sourcepos="500:1-500:5">Alone</h3>',
  '<p data-sourcepos="1000:1-1000:4">End</p>',
].join("\n");

test("a click on a mark that stands for several headings goes to the first of them", async () => {
  layOut(1);
  showInTurn(MERGED);
  await startShowing();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  clickStrip("left", 9);
  await landed();
  expect(scrolledTo()).toContain("small");
  expect(scrolledTo()).not.toContain("large");
});

test("a click on the strip near no mark leaves the view and History as they were", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  const pressed = clickStrip("left", 200);
  await landed();
  expect(pressed.defaultPrevented).toBe(false);
  expect(scrolledTo()).toEqual([]);
  expect(canStepThroughHistory("back")).toBe(false);
});

test("a selection dragged from the text and let go beside a heading mark leaves the view and History as they were", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  // Pressed in the h1's text and let go beside the h2's mark: the webview gives the click to the
  // view, which holds both.
  document
    .querySelector("#long")
    ?.dispatchEvent(
      new MouseEvent("mousedown", { ...onStrip("left", 10), clientX: 400 }),
    );
  documentView().dispatchEvent(new MouseEvent("mouseup", onStrip("left", 369)));
  documentView().dispatchEvent(new MouseEvent("click", onStrip("left", 369)));
  await landed();
  expect(scrolledTo()).toEqual([]);
  expect(canStepThroughHistory("back")).toBe(false);
});

/**
 * `LONG` with "lamp" in its h1, row 0, twice in the paragraph on lines 11 to 20, rows 73 and 110,
 * and in its h2 on line 51, row 368, whose mark shares the row with that match's.
 */
const LAMP_ROOM = [
  '<h1 id="lamps" data-sourcepos="1:1-1:5">Lamps</h1>',
  '<p data-sourcepos="11:1-20:20">lamp wick lamp glass</p>',
  '<h2 id="lamp-room" data-sourcepos="51:1-51:9">Lamp room</h2>',
  '<p data-sourcepos="100:1-100:4">End</p>',
].join("\n");

test("in the strip's left half a heading mark wins over a match mark in its row, and in the right half the match mark", async () => {
  layOut(1);
  showInTurn(LAMP_ROOM);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  clickStrip("left", 369);
  await landed();
  expect(scrolledTo().at(-1)).toBe("lamp-room");
  expect(findStatus()).toBe("1 of 4");

  clickStrip("right", 369);
  await landed();
  expect(findStatus()).toBe("4 of 4");
});

test("a click in the strip's left half beside a match mark with no heading mark near makes that match current", async () => {
  layOut(1);
  showInTurn(LAMP_ROOM);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolled = spyOnScrolls();

  // 1.6 px below the match 110.4 px down the strip.
  expect(clickStrip("left", 112).defaultPrevented).toBe(true);
  expect(findStatus()).toBe("3 of 4");
  expect(scrolled.mock.calls.at(-1)).toEqual([{ block: "center" }]);
  expect(canStepThroughHistory("back")).toBe(false);
});

test("a click in the strip's right half on a heading mark with no match mark near goes to the heading", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  clickStrip("right", 1);
  await landed();
  expect(scrolledTo().at(-1)).toBe("long");
  expect(canStepThroughHistory("back")).toBe(true);
});

test("hovering a heading mark names its heading in a label, which goes when the pointer moves off the mark", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  expect(hoverLabel()).toBeUndefined();

  hover("left", 369);
  expect(hoverLabel()).toEqual(["Half way"]);
  hover("left", 200);
  expect(hoverLabel()).toBeUndefined();
});

test("the hover label lists the headings of a mark that stands for several, up to five and then how many more", async () => {
  layOut(1);
  // 10,000 lines, 200,000 px, about 272 px of it in each px of the strip: the headings on lines 1
  // to 7 all fall in row 0.
  showInTurn(
    [
      ...["One", "Two", "Three", "Four", "Five", "Six", "Seven"].map(
        (text, index) =>
          `<h2 id="${text.toLowerCase()}" data-sourcepos="${String(index + 1)}:1-${String(index + 1)}:9">${text}</h2>`,
      ),
      '<p data-sourcepos="10000:1-10000:4">End</p>',
    ].join("\n"),
  );
  await startShowing();

  hover("left", 1);
  expect(hoverLabel()).toEqual([
    "One",
    "Two",
    "Three",
    "Four",
    "Five",
    "and 2 more",
  ]);
});

test("the hover label goes when the pointer leaves the document", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  hover("left", 369);
  expect(hoverLabel()).toEqual(["Half way"]);
  scroller()?.dispatchEvent(new MouseEvent("mouseleave"));
  expect(hoverLabel()).toBeUndefined();
});

test("no hover label shows over a match mark or the empty strip", async () => {
  layOut(1);
  showInTurn(LAMP_ROOM);
  await startShowing();
  typeInFind("lamp");
  await drawn();

  hover("right", 111);
  expect(hoverLabel()).toBeUndefined();
  // The h2 shares row 368 with a match, whose mark wins in the right half.
  hover("right", 369);
  expect(hoverLabel()).toBeUndefined();
  hover("left", 250);
  expect(hoverLabel()).toBeUndefined();
});

test("the pointer is a hand over a heading mark and a match mark, and an arrow beside them", async () => {
  layOut(1);
  showInTurn(LAMP_ROOM);
  await startShowing();
  typeInFind("lamp");
  await drawn();

  hover("left", 369);
  expect(pointer()).toBe("pointer");
  hover("left", 250);
  expect(pointer()).not.toBe("pointer");
  hover("right", 111);
  expect(pointer()).toBe("pointer");
  scroller()?.dispatchEvent(new MouseEvent("mouseleave"));
  expect(pointer()).not.toBe("pointer");
});

test("the hover label goes when the document scrolls", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  hover("left", 369);
  scroller()?.dispatchEvent(new Event("scroll"));
  expect(hoverLabel()).toBeUndefined();
});

test("the hover label goes when another document is shown", async () => {
  layOut(1);
  showInTurn(LONG, LONG.replace("Half way", "Other way"));
  await startShowing();

  hover("left", 369);
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(hoverLabel()).toBeUndefined();
  await drawn();
  expect(hoverLabel()).toBeUndefined();
});

test("the hover label goes when the ruler is switched off", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();
  const off: PageSettings = { ...DEFAULT_SETTINGS, overviewRuler: false };

  hover("left", 369);
  await tell("settings", off);
  expect(hoverLabel()).toBeUndefined();
  await tell("settings", DEFAULT_SETTINGS);
  await drawn();
  expect(hoverLabel()).toBeUndefined();
  expect(pointer()).not.toBe("pointer");
});

/**
 * `LONG` with "lamp" in its h1, row 0, and in its h2 on line 51, row 368, halfway through its text:
 * 1,010 px down, 371.68 px down the strip, in row 371, 2.68 px below the middle of the h2's mark.
 */
const FAR_LAMP = [
  '<h1 id="lamps" data-sourcepos="1:1-1:5">Lamps</h1>',
  '<h2 id="far-lamp" data-sourcepos="51:1-51:8">Far lamp</h2>',
  '<p data-sourcepos="100:1-100:4">End</p>',
].join("\n");

test("a click in the strip's right half on a heading mark goes to the heading, though a match mark a few pixels away is in reach", async () => {
  layOut(1);
  showInTurn(FAR_LAMP);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  // On the middle of the h2's mark, 2.68 px above the match.
  clickStrip("right", 369);
  await landed();
  expect(scrolledTo().at(-1)).toBe("far-lamp");
  expect(findStatus()).toBe("1 of 2");
  expect(canStepThroughHistory("back")).toBe(true);
});

test("a click in the strip's left half on the current match's mark makes that match current, though a heading mark a few pixels away is in reach", async () => {
  layOut(1);
  showInTurn(FAR_LAMP);
  await startShowing();
  typeInFind("lamp");
  press({ key: "Enter", code: "Enter" });
  await drawn();
  // The current match's full-width mark covers rows 371 to 373.
  expect(currentMarkTop()).toBe(371);
  const scrolled = spyOnScrolls();

  // 1.32 px below the match, and 4 px below the middle of the h2's mark.
  expect(clickStrip("left", 373).defaultPrevented).toBe(true);
  await landed();
  expect(findStatus()).toBe("2 of 2");
  expect(scrolled.mock.calls.at(-1)).toEqual([{ block: "center" }]);
  expect(canStepThroughHistory("back")).toBe(false);
});

test("a click on a heading mark in the frame between a count and the strip's redraw of it does nothing", async () => {
  layOut(1);
  showInTurn(LAMP_ROOM);
  await startShowing();
  typeInFind("lamp");
  await drawn();
  const scrolledTo = spyOnHeadingScrolls({ laidOut: true });

  // "lamp g" has no match in the h1, whose mark the strip still shows in row 0.
  typeInFind("lamp g");
  expect(clickStrip("left", 1).defaultPrevented).toBe(false);
  await landed();
  expect(scrolledTo()).not.toContain("lamps");
  expect(canStepThroughHistory("back")).toBe(false);
});

test("during a reload the old heading marks neither name nor go to their headings until the new render's list is in", async () => {
  // Scrolled to the h2, so the reload lands back at it, which takes a frame.
  layOut(51);
  let deliver: (html: string) => void = () => {};
  readInTurn(
    () => Promise.resolve(LONG),
    readSentLater((send) => (deliver = send)),
  );
  await startShowing();
  await saveTheFile();
  await drawn();

  const shown = nextShown();
  // The h2 moved to line 26, row 184.
  deliver(LONG.replace("51:1-51:9", "26:1-26:9"));
  await shown;
  // The strip keeps the old document's marks until the new render's list of headings is in.
  expect(marks()).toContainEqual({ kind: "h2", top: 368 });
  hover("left", 369);
  expect(hoverLabel()).toBeUndefined();
  expect(pointer()).not.toBe("pointer");
  expect(clickStrip("left", 369).defaultPrevented).toBe(false);
  await landed();
  expect(canStepThroughHistory("back")).toBe(false);

  await drawn();
  hover("left", 185);
  expect(hoverLabel()).toEqual(["Half way"]);
});

test("the hover label comes back on the same mark after a scroll took it away", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  hover("left", 369);
  scroller()?.dispatchEvent(new Event("scroll"));
  expect(hoverLabel()).toBeUndefined();
  hover("left", 369);
  expect(hoverLabel()).toEqual(["Half way"]);
});

test("no hover label and no hand show while a mouse button is held, as during a drag", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  hover("left", 369, 1);
  expect(hoverLabel()).toBeUndefined();
  expect(pointer()).not.toBe("pointer");
  hover("left", 369);
  expect(hoverLabel()).toEqual(["Half way"]);
});

test("the hover label and the hand go when a resize redraws the strip, and the next move finds the mark again", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  hover("left", 369);
  expect(hoverLabel()).toEqual(["Half way"]);
  resize();
  await drawn();
  expect(hoverLabel()).toBeUndefined();
  expect(pointer()).not.toBe("pointer");
  hover("left", 369);
  expect(hoverLabel()).toEqual(["Half way"]);
  expect(pointer()).toBe("pointer");
});

test("the hover label runs down from a mark in the strip's upper half and up from one in its lower half", async () => {
  layOut(1);
  showInTurn(LONG);
  await startShowing();

  // The h1's mark is in row 0 of the 736 px strip, and the h2's in row 368.
  hover("left", 1);
  expect(shownLabel()?.style.top).toBe("0px");
  expect(shownLabel()?.style.bottom).toBe("");
  hover("left", 369);
  expect(shownLabel()?.style.top).toBe("");
  expect(shownLabel()?.style.bottom).toBe("368px");
});

test("the hover label leaves out a heading with no text, such as one of only an image", async () => {
  layOut(1);
  // As `MERGED`: the h4 on line 12 and the h2 on line 13 share one mark in row 8.
  showInTurn(
    MERGED.replace('id="small"', 'id="logo"').replace(
      ">Small<",
      '><img alt="Logo"><',
    ),
  );
  await startShowing();

  hover("left", 9);
  expect(hoverLabel()).toEqual(["Large"]);
});
