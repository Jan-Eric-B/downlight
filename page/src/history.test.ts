import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { ArgsOf, Followed, MenuEntry } from "./app";
import { startCommands } from "./commands";
import { startLinks } from "./links";
import { afterNextFrame } from "./reading-position";
import { startSettings } from "./settings";
import {
  answerAsTheApp,
  askedOfTheApp,
  fakeTheApp,
  giveAnotherDocument,
  heading,
  layOut,
  linesOf,
  nextInTurn,
  nextShown,
  settle,
  forgetForwardAsTheApp,
  stepAsTheApp,
  scrollAsTheApp,
  tell,
  type Read,
} from "./test-support";
import {
  DOCUMENT_VIEW_SELECTOR,
  startDocumentView,
  whenDocumentShown,
  type ShownReason,
} from "./view";

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: Read[] = [];

/** Each `step_history` call's arguments, in turn. */
let steps: ArgsOf<"step_history">[] = [];

/** The entries of the last right-click menu the page asked the app to show. */
let menu: MenuEntry[] = [];

let stop: AbortController;

beforeEach(async () => {
  mockWindows("document-0");
  documents = [];
  steps = [];
  menu = [];
  fakeTheApp({
    render_document: (args) => {
      const next = nextInTurn(documents);
      return next ? answerAsTheApp(args, next) : null;
    },
    step_history: (args) => {
      steps.push(args);
      return stepAsTheApp(args);
    },
    forget_forward: forgetForwardAsTheApp,
    follow_link: ({ target }): Followed => ({
      kind: "scrollTo",
      heading: target.slice(1),
    }),
    show_document_menu: ({ entries }) => {
      menu = entries;
      return null;
    },
  });
  stop = new AbortController();
  await startSettings();
  startCommands(stop.signal);
  startLinks(stop.signal);
});

afterEach(() => {
  stop.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** The documents the window is given, in turn, each read as its HTML. */
function showInTurn(...html: string[]): void {
  documents = html.map((each) => () => Promise.resolve(each));
}

/** Presses `key`, with Alt if `alt`, as the reader would. */
function press(key: string, alt = false): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    key,
    altKey: alt,
    bubbles: true,
    cancelable: true,
  });
  document.body.dispatchEvent(event);
  return event;
}

/** Back, as Alt+Left gives it. */
function back(): void {
  press("ArrowLeft", true);
}

/** Forward, as Alt+Right gives it. */
function forward(): void {
  press("ArrowRight", true);
}

/** The first line of each block the page scrolled to the top of the view since the spy started. */
function spyOnScrolls(): () => number[] {
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});
  return () =>
    scrolls.mock.contexts.map((block) => linesOf(block as Element)?.[0] ?? 0);
}

/** Resolves once `steps` has run and the document it brings is in place. */
async function stepped(steps: () => void): Promise<void> {
  const shown = nextShown();
  steps();
  await shown;
}

/**
 * Resolves once a scroll the page just started has landed: its IPC calls are done, and the frame
 * after which it corrects has passed.
 */
async function landed(): Promise<void> {
  await settle();
  await afterNextFrame();
  await settle();
}

/** Clicks the document's link to `#id`, as the reader would. */
function clickLinkTo(id: string): void {
  const link = document.querySelector<HTMLElement>(
    `${DOCUMENT_VIEW_SELECTOR} a[href="#${id}"]`,
  );
  if (!link) throw new Error(`The document has no link to #${id}.`);
  link.click();
}

/** Whether the right-click menu offers Back and Forward now. */
async function offered(): Promise<{ back: boolean; forward: boolean }> {
  document.body.dispatchEvent(
    new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
  );
  await landed();
  const enabled = (id: string) =>
    menu.some(
      (entry) => entry.kind === "command" && entry.id === id && entry.enabled,
    );
  return { back: enabled("back"), forward: enabled("forward") };
}

/** Shows readme, then notes, read with the view's top at `leftAt` in readme when it was left. */
async function readmeLeftAt(leftAt: number): Promise<void> {
  showInTurn(readme, notes, readme);
  await startDocumentView(0);
  await landed();
  layOut(leftAt);
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(heading()).toBe("Field notes");
}

test("Back shows the document the window showed before, back at the reading position it was left at", async () => {
  // Halfway down "## Options", on line 15.
  await readmeLeftAt(15.5);
  const scrolledTo = spyOnScrolls();

  await stepped(back);
  expect(steps).toEqual([{ step: "back" }]);
  expect(heading()).toBe("Lamplighter");
  expect(scrolledTo().length).toBeGreaterThan(0);
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
});

test("Back to a document left at its top shows its top", async () => {
  await readmeLeftAt(0);
  const scrolledTo = spyOnScrolls();
  const scrollsToTop = vi
    .spyOn(Element.prototype, "scrollTo")
    .mockImplementation(() => {});

  await stepped(back);
  expect(heading()).toBe("Lamplighter");
  expect(scrolledTo()).toEqual([]);
  expect(scrollsToTop).toHaveBeenCalledWith(0, 0);
});

test("Forward after Back shows the document Back left, back at its reading position", async () => {
  await readmeLeftAt(15.5);
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  // Notes has a block on line 3.
  layOut(3.5);
  await stepped(back);
  const scrolledTo = spyOnScrolls();

  await stepped(forward);
  expect(steps).toEqual([{ step: "back" }, { step: "forward" }]);
  expect(heading()).toBe("Field notes");
  expect(new Set(scrolledTo())).toEqual(new Set([3]));
});

test("the whenDocumentShown listeners hear that Back returned to a document", async () => {
  await readmeLeftAt(15.5);
  const reasons: ShownReason[] = [];
  const stopListening = whenDocumentShown(({ reason }) => {
    reasons.push(reason);
  });

  await stepped(back);
  stopListening();
  expect(reasons).toEqual(["return"]);
});

test("Back from a link to a heading returns to where the view was in the document, without the app", async () => {
  showInTurn(readme);
  await startDocumentView(0);
  await landed();
  layOut(8.5);
  clickLinkTo("options");
  await landed();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();

  back();
  await vi.waitFor(() => {
    expect(scrolledTo().length).toBeGreaterThan(0);
  });
  expect(new Set(scrolledTo())).toEqual(new Set([8]));
  expect(steps).toEqual([]);
});

test("Forward after Back from a heading returns to where the view was at the heading", async () => {
  showInTurn(readme);
  await startDocumentView(0);
  await landed();
  layOut(8.5);
  clickLinkTo("options");
  await landed();
  layOut(17.5);
  back();
  await landed();
  layOut(8.5);
  const scrolledTo = spyOnScrolls();

  forward();
  await vi.waitFor(() => {
    expect(scrolledTo().length).toBeGreaterThan(0);
  });
  expect(new Set(scrolledTo())).toEqual(new Set([17]));
  expect(steps).toEqual([]);
});

test("a heading another window's link scrolled to is a step Back returns from", async () => {
  showInTurn(readme);
  await startDocumentView(0);
  await landed();
  layOut(8.5);
  await scrollAsTheApp("options");
  await landed();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();

  back();
  await vi.waitFor(() => {
    expect(scrolledTo().length).toBeGreaterThan(0);
  });
  expect(new Set(scrolledTo())).toEqual(new Set([8]));
});

test("a link to a heading the document doesn't have is no step", async () => {
  showInTurn(readme);
  await startDocumentView(0);
  await landed();
  await scrollAsTheApp("nowhere");
  await landed();

  expect(await offered()).toEqual({ back: false, forward: false });
});

test("Back past the first heading visited goes to the document before", async () => {
  await readmeLeftAt(15.5);
  documents = [() => Promise.resolve(readme)];
  layOut(1.5);
  await scrollAsTheApp("field-notes");
  await landed();
  back();
  await landed();
  expect(steps).toEqual([]);
  const scrolledTo = spyOnScrolls();

  await stepped(back);
  expect(steps).toEqual([{ step: "back" }]);
  expect(heading()).toBe("Lamplighter");
  expect(new Set(scrolledTo())).toEqual(new Set([15]));
});

test("Back to a document returns to the heading visited last in it, and Back again to where the view was before it", async () => {
  showInTurn(readme, notes, readme);
  await startDocumentView(0);
  await landed();
  layOut(8.5);
  clickLinkTo("options");
  await landed();
  layOut(17.5);
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  const scrolledTo = spyOnScrolls();

  await stepped(back);
  await landed();
  expect(new Set(scrolledTo())).toEqual(new Set([17]));
  const before = scrolledTo().length;
  back();
  await landed();
  expect(new Set(scrolledTo().slice(before))).toEqual(new Set([8]));
  expect(steps).toEqual([{ step: "back" }]);
});

test("a document given after Back leaves no heading of the document before to go forward to", async () => {
  // As in a browser: going somewhere new drops what Forward would have gone to.
  showInTurn(readme, notes, readme, notes);
  await startDocumentView(0);
  await landed();
  layOut(8.5);
  clickLinkTo("options");
  await landed();
  back();
  await landed();
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  await stepped(back);

  expect(await offered()).toEqual({ back: false, forward: true });
  await stepped(forward);
  expect(heading()).toBe("Field notes");
});

test("the menu offers Back and Forward only when there is somewhere to go", async () => {
  showInTurn(readme, notes, readme);
  await startDocumentView(0);
  await landed();
  expect(await offered()).toEqual({ back: false, forward: false });

  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  expect(await offered()).toEqual({ back: true, forward: false });

  await stepped(back);
  expect(await offered()).toEqual({ back: false, forward: true });

  clickLinkTo("options");
  await landed();
  expect(await offered()).toEqual({ back: true, forward: false });
});

test("a heading visited after Back leaves no document to go forward to, in the app too", async () => {
  // As in a browser: going somewhere new drops what Forward would have gone to.
  await readmeLeftAt(15.5);
  documents = [() => Promise.resolve(readme)];
  await stepped(back);

  clickLinkTo("options");
  await landed();
  expect(askedOfTheApp()).toContain("forget_forward");
  forward();
  await landed();
  expect(steps).toEqual([{ step: "back" }, { step: "forward" }]);
  expect(heading()).toBe("Lamplighter");
});

test("Back and Forward chosen in the menu go back and forward", async () => {
  await readmeLeftAt(15.5);
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];

  await stepped(() => {
    void tell("menu-command", "back");
  });
  expect(heading()).toBe("Lamplighter");
  await stepped(() => {
    void tell("menu-command", "forward");
  });
  expect(heading()).toBe("Field notes");
});

test("Back with nowhere to go leaves the document where it is", async () => {
  showInTurn(readme);
  await startDocumentView(0);
  await landed();
  layOut(15.5);
  const scrolledTo = spyOnScrolls();

  back();
  await landed();
  expect(heading()).toBe("Lamplighter");
  expect(scrolledTo()).toEqual([]);
  expect(
    askedOfTheApp().filter((asked) => asked === "render_document"),
  ).toEqual(["render_document"]);
});

test("of Back and another document given in quick succession, the later one stays", async () => {
  // Back goes through the same request as every show, where the latest request wins.
  let sendReturned: (html: string) => void = () => {};
  await readmeLeftAt(15.5);
  documents = [
    () =>
      new Promise((resolve) => {
        sendReturned = resolve;
      }),
    () => Promise.resolve(notes),
  ];

  back();
  await landed();
  const shown = nextShown();
  await giveAnotherDocument();
  await shown;
  sendReturned(readme);
  await landed();
  expect(heading()).toBe("Field notes");
});
