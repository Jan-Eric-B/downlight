import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import notes from "../../core/tests/fixtures/notes.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { WaitNote } from "./app";
import { openFind, startFind } from "./find";
import {
  askedOfTheApp,
  emptyWindow,
  fakeTheAppReading,
  giveAnotherDocument,
  heading,
  scrollAsTheApp,
  saveTheFile,
  settle,
  tell,
  shownWaitNote,
  spyOnHeadingScrolls,
  stopDocumentSample,
  waitNoteAsTheApp,
  type FakeDocumentApp,
  type NoAnswer,
  type Read,
} from "./test-support";
import {
  showingInView,
  startDocumentView,
  stepThroughHistory,
  visitHeading,
} from "./view";
import { WAIT_NOTE_MS } from "./wait-note";

/** What reading the window's document gives `render_document`, in turn; the last one repeats. */
let documents: (Read | NoAnswer)[] = [];

let app: FakeDocumentApp;

/** What the fake app's wait note names. */
const BIG_DOCUMENT: WaitNote = {
  file: { name: "big.md", path: "C:\\docs\\big.md" },
  size: "100 MB",
};

/** What the fake app's wait note names now. */
let waitedFor: WaitNote = BIG_DOCUMENT;

/** The showings the page sent with `stop_document`, in turn. */
let stops: (number | null)[] = [];

/** The showings the page sent with `document_arrived`, in turn. */
let arrivals: number[] = [];

/** What the fake app answers `stop_document`: whether it stopped. */
let stopAnswer = stopDocumentSample("stopped");

/** Stops the test's view, whose Esc would otherwise reach the next test's. */
let stopView: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  documents = [];
  waitedFor = BIG_DOCUMENT;
  stops = [];
  arrivals = [];
  stopAnswer = stopDocumentSample("stopped");
  stopView = new AbortController();
  app = fakeTheAppReading(() => documents, {
    wait_note: (args) => waitNoteAsTheApp(args, waitedFor),
    stop_document: ({ showing }) => {
      stops.push(showing);
      return stopAnswer;
    },
    document_arrived: ({ showing }) => {
      arrivals.push(showing);
      return null;
    },
  });
  // The note's half second passes when a test says, never late on a loaded machine; time still
  // runs on its own, for the page's frames and the IPC.
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  stopView.abort();
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** A read that gives the HTML the test sends through the function it hands `send`. */
function readSentLater(send: (deliver: (html: string) => void) => void): Read {
  return () =>
    new Promise((resolve) => {
      send(resolve);
    });
}

test("a document that takes longer than half a second to arrive shows a wait note naming its file and size", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [readSentLater((send) => (deliver = send))];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);

  await vi.waitFor(() => {
    expect(shownWaitNote()?.textContent).toBe("Showing big.md (100 MB) ...");
  });
  expect(shownWaitNote()?.querySelector("strong")?.textContent).toBe("big.md");
  expect(shownWaitNote()?.getAttribute("role")).toBe("status");
  deliver(readme);
  await vi.waitFor(() => {
    expect(document.querySelector("article")?.textContent).toContain(
      "Lamplighter",
    );
  });
  expect(shownWaitNote()).toBeNull();
});

test("a window whose document is slow to arrive shows with the wait note rather than staying hidden", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [readSentLater((send) => (deliver = send))];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);

  await vi.waitFor(() => {
    expect(app.textWhenShown()).toContain("Showing big.md (100 MB) ...");
  });
  deliver(readme);
  await settle();
  expect(askedOfTheApp().filter((c) => c === "show_window")).toHaveLength(1);
});

test("a document that arrives within half a second shows no wait note", async () => {
  documents = [
    () =>
      new Promise((resolve) => {
        setTimeout(() => {
          resolve(readme);
        }, 100);
      }),
  ];
  await startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS + 100);

  expect(askedOfTheApp()).not.toContain("wait_note");
  expect(shownWaitNote()).toBeNull();
});

test("a document given while another is in view shows the wait note over it until it arrives", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0, stopView.signal);
  await settle();

  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()).not.toBeNull();
  });
  expect(document.querySelector("article")?.textContent).toContain(
    "Lamplighter",
  );
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  expect(shownWaitNote()).toBeNull();
});

test("a reload slower than half a second shows no wait note and keeps the document in view", async () => {
  // The owner's decision on ticket 31: only a window waiting for another document shows the note.
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0, stopView.signal);
  await settle();

  await saveTheFile();
  await vi.advanceTimersByTimeAsync(2 * WAIT_NOTE_MS);
  expect(shownWaitNote()).toBeNull();
  expect(heading()).toBe("Lamplighter");
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
});

/** The text of the shown wait note, or undefined while none shows. */
function noteText(): string | undefined {
  return shownWaitNote()?.textContent ?? undefined;
}

/** The shown wait note's Stop button, or null. */
function stopButton(): HTMLButtonElement | null {
  return shownWaitNote()?.querySelector("button") ?? null;
}

/** Presses Esc where the page has the focus, as the reader does. */
function pressEsc(): void {
  (document.activeElement ?? document.body).dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Escape",
      bubbles: true,
      cancelable: true,
    }),
  );
}

/**
 * A window showing readme.md, given another document whose read is on its way, with the wait
 * note shown: the HTML the test sends through the function it gives back.
 */
async function loadingAnotherDocument(): Promise<(html: string) => void> {
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0, stopView.signal);
  await settle();
  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()).not.toBeNull();
  });
  return (html) => {
    deliver(html);
  };
}

test("the wait note counts the whole seconds since the request, from the first on", async () => {
  documents = [readSentLater(() => {})];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(noteText()).toBe("Showing big.md (100 MB) ...");
  });

  await vi.advanceTimersByTimeAsync(1000 - WAIT_NOTE_MS);
  expect(noteText()).toBe("Showing big.md (100 MB) ... 1 s");
  await vi.advanceTimersByTimeAsync(11_000);
  expect(noteText()).toBe("Showing big.md (100 MB) ... 12 s");
});

test("the wait note's seconds stay out of what a screen reader is told of it", async () => {
  documents = [readSentLater(() => {})];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(2000);
  const seconds = shownWaitNote()?.querySelector("[aria-hidden='true']");
  expect(seconds?.textContent).toBe(" 2 s");
});

test("a save during the load keeps the count, and another document starts it again", async () => {
  documents = [readSentLater(() => {})];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(3000);
  expect(noteText()).toBe("Showing big.md (100 MB) ... 3 s");

  await saveTheFile();
  await vi.advanceTimersByTimeAsync(1000);
  expect(noteText()).toBe("Showing big.md (100 MB) ... 4 s");

  waitedFor = {
    file: { name: "other.md", path: "C:\\docs\\other.md" },
    size: "20 MB",
  };
  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(noteText()).toBe("Showing other.md (20 MB) ...");
  });
  await vi.advanceTimersByTimeAsync(1000 - WAIT_NOTE_MS);
  expect(noteText()).toBe("Showing other.md (20 MB) ... 1 s");
});

test("the wait note says the document is laid out, without its Stop, before the document replaces it", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [readSentLater((send) => (deliver = send))];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(stopButton()).not.toBeNull();
  });
  // The frames wait until the test draws them, so it sees the page between the two.
  const frames: FrameRequestCallback[] = [];
  const held = vi
    .spyOn(window, "requestAnimationFrame")
    .mockImplementation((frame) => {
      frames.push(frame);
      return frames.length;
    });

  deliver(readme);
  await vi.waitFor(() => {
    expect(noteText()).toBe("Laying out big.md ...");
  });
  expect(stopButton()).toBeNull();
  await vi.advanceTimersByTimeAsync(100);
  expect(document.querySelector("article")?.textContent).not.toContain(
    "Lamplighter",
  );
  held.mockRestore();
  for (const frame of frames.splice(0)) frame(performance.now());
  await vi.waitFor(() => {
    expect(heading()).toBe("Lamplighter");
  });
  expect(shownWaitNote()).toBeNull();
});

test("the wait note's Stop is named for the reader with its key", async () => {
  documents = [readSentLater(() => {})];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(stopButton()).not.toBeNull();
  });
  expect(stopButton()?.getAttribute("aria-label")).toBe("Stop");
  expect(stopButton()?.title).toBe("Stop (Esc)");
  expect(stopButton()?.getAttribute("aria-keyshortcuts")).toBe("Escape");
});

test("Stop keeps the document in view and tells the app, and the stopped document never shows", async () => {
  const deliver = await loadingAnotherDocument();
  const inView = showingInView();

  stopButton()?.click();
  await settle();
  expect(stops).toEqual([inView]);
  expect(shownWaitNote()).toBeNull();
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).toBe("Lamplighter");
  expect(showingInView()).toBe(inView);
});

test("Esc stops the document on its way", async () => {
  const deliver = await loadingAnotherDocument();
  const inView = showingInView();

  pressEsc();
  await settle();
  expect(stops).toEqual([inView]);
  expect(shownWaitNote()).toBeNull();
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).toBe("Lamplighter");
});

test("Esc with Find open closes Find and the document stays on its way", async () => {
  const stopFind = new AbortController();
  startFind(stopFind.signal);
  const deliver = await loadingAnotherDocument();
  openFind();

  pressEsc();
  await settle();
  expect(stops).toEqual([]);
  expect(shownWaitNote()).not.toBeNull();
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  stopFind.abort();
});

test("Stop in a window that showed no document shows the empty window", async () => {
  documents = [readSentLater(() => {}), () => Promise.resolve(null)];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(stopButton()).not.toBeNull();
  });

  stopButton()?.click();
  await vi.waitFor(() => {
    expect(emptyWindow()?.hidden).toBe(false);
  });
  expect(stops).toEqual([null]);
  expect(shownWaitNote()).toBeNull();
});

test("a link to a heading in the document in view stops the document on its way", async () => {
  const deliver = await loadingAnotherDocument();
  const inView = showingInView();

  await visitHeading("install");
  expect(stops).toEqual([inView]);
  expect(shownWaitNote()).toBeNull();
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).toBe("Lamplighter");
});

test("Back within the document in view stops the document on its way", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0, stopView.signal);
  await settle();
  await visitHeading("install");
  const inView = showingInView();
  await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(shownWaitNote()).not.toBeNull();
  });

  stepThroughHistory("back");
  await settle();
  expect(stops).toEqual([inView]);
  expect(shownWaitNote()).toBeNull();
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).toBe("Lamplighter");
});

test("the page tells the app each render it placed, a show and a reload alike", async () => {
  documents = [() => Promise.resolve(readme), () => Promise.resolve(notes)];
  await startDocumentView(0, stopView.signal);
  await settle();
  const shown = showingInView();
  expect(arrivals).toEqual([shown]);

  await saveTheFile();
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  await settle();
  expect(arrivals).toEqual([shown, shown]);
});

/** A read that fails, as for a binary file or a folder named like a document. */
const unreadable: Read = () =>
  Promise.reject(new Error("big.md isn't a text file."));

/** How often the page has asked the app for its document. */
function requests(): number {
  return askedOfTheApp().filter((command) => command === "render_document")
    .length;
}

test("the page tells the app it placed why a document given can't be shown", async () => {
  // Ticket 38's review: else the app holds the document before it as in view, for good.
  documents = [() => Promise.resolve(readme), unreadable];
  await startDocumentView(0, stopView.signal);
  await settle();
  const shown = showingInView();

  await giveAnotherDocument();
  await vi.waitFor(() => {
    expect(arrivals).toHaveLength(2);
  });
  expect(arrivals[0]).toBe(shown);
  expect(arrivals[1]).not.toBe(shown);
});

test("the page tells the app it took a reload that can't be read, so the reload isn't left pending", async () => {
  documents = [() => Promise.resolve(readme), unreadable];
  await startDocumentView(0, stopView.signal);
  await settle();
  const shown = showingInView();

  await saveTheFile();
  await vi.waitFor(() => {
    expect(arrivals).toEqual([shown, shown]);
  });
  expect(heading()).toBe("Lamplighter");
});

test("Stop after a document that can't be shown names that answer's showing and asks for nothing again", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    unreadable,
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0, stopView.signal);
  await settle();
  await giveAnotherDocument();
  await vi.waitFor(() => {
    expect(arrivals).toHaveLength(2);
  });
  const failed = await giveAnotherDocument();
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(stopButton()).not.toBeNull();
  });
  const asked = requests();

  stopButton()?.click();
  await settle();
  expect(stops).toEqual([failed]);
  expect(requests()).toBe(asked);
  expect(shownWaitNote()).toBeNull();
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).not.toBe("Field notes");
});

test("Stop that the app didn't carry out asks for nothing again", async () => {
  stopAnswer = stopDocumentSample("notStopped");
  documents = [readSentLater(() => {}), () => Promise.resolve(null)];
  void startDocumentView(0, stopView.signal);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  await vi.waitFor(() => {
    expect(stopButton()).not.toBeNull();
  });

  stopButton()?.click();
  await settle();
  await settle();
  expect(stops).toEqual([null]);
  expect(requests()).toBe(1);
  expect(emptyWindow()?.hidden).toBe(true);
});

test("the app's word that the reader asked for the document in view stops the document on its way", async () => {
  // Ctrl+O or a drop of the document in view, as Stop (ticket 38's review).
  const deliver = await loadingAnotherDocument();
  const inView = showingInView();
  if (inView === null) throw new Error("No document is in view.");

  await tell("stop-document", inView);
  await settle();
  expect(stops).toEqual([inView]);
  expect(shownWaitNote()).toBeNull();
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).toBe("Lamplighter");
  expect(showingInView()).toBe(inView);
});

test("the app's word to stop before the wait note shows stops the document on its way", async () => {
  let deliver: (html: string) => void = () => {};
  documents = [
    () => Promise.resolve(readme),
    readSentLater((send) => (deliver = send)),
  ];
  await startDocumentView(0, stopView.signal);
  await settle();
  const inView = await giveAnotherDocument();

  await tell("stop-document", inView);
  await settle();
  expect(stops).toEqual([inView]);
  deliver(notes);
  await vi.advanceTimersByTimeAsync(WAIT_NOTE_MS);
  expect(heading()).toBe("Lamplighter");
  expect(shownWaitNote()).toBeNull();
});

test("the app's word to stop for a showing the view doesn't show changes nothing", async () => {
  const deliver = await loadingAnotherDocument();
  const inView = showingInView() ?? 0;

  await tell("stop-document", inView + 100);
  await settle();
  expect(stops).toEqual([]);
  expect(shownWaitNote()).not.toBeNull();
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
});

test("a link from another window to a heading in the document in view stops the load and scrolls there", async () => {
  const deliver = await loadingAnotherDocument();
  const inView = showingInView();
  if (inView === null) throw new Error("No document is in view.");
  const scrolledTo = spyOnHeadingScrolls();

  // As the app sends them for such a link.
  await tell("stop-document", inView);
  await scrollAsTheApp("install", inView);
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("install");
  });
  expect(stops).toEqual([inView]);
  deliver(notes);
  await vi.advanceTimersByTimeAsync(100);
  expect(heading()).toBe("Lamplighter");
});

test("Esc with Find open closes Find first, whichever started first", async () => {
  // Ticket 38's review: the view's Esc doesn't rely on Find's listener running before it.
  const deliver = await loadingAnotherDocument();
  const stopFind = new AbortController();
  startFind(stopFind.signal);
  openFind();

  pressEsc();
  await settle();
  expect(stops).toEqual([]);
  expect(shownWaitNote()).not.toBeNull();
  deliver(notes);
  await vi.waitFor(() => {
    expect(heading()).toBe("Field notes");
  });
  stopFind.abort();
});
