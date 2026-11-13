// Helpers shared by the page's tests. The name has no `.test`, so Vitest doesn't run it as a test
// file, and nothing the app runs imports it.
import type { InvokeArgs } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { mockIPC, type MockIPCOptions } from "@tauri-apps/api/mocks";
import { vi } from "vitest";
import {
  whenDocumentShown,
  type DocumentToShow,
  type Place,
  type RenderDocumentArgs,
} from "./view";

/** A document shown at a heading, as the app hands it over for a link like `a.md#install`. */
export interface AtHeading {
  document: string;
  heading: string;
}

/**
 * What reading the window's document gives the fake app: its HTML, its HTML at a heading, or
 * null while the window shows no document. A read that rejects is a file that can't be read.
 */
export type Read = () => Promise<string | AtHeading | null>;

/** The showing the fake app's window is at (`Showing` in `app/src/documents.rs`). */
let currentShowing = 0;

/**
 * Mocks the app's IPC with `handle`, as `mockIPC` does, with the fake app's window back at its
 * first showing, so that no test inherits the showings another test gave its window.
 */
export function mockTheApp(
  handle: Parameters<typeof mockIPC>[0],
  options?: MockIPCOptions,
): void {
  currentShowing = 0;
  mockIPC(handle, options);
}

/**
 * The page's `render_document` arguments as the IPC hands them over. They are checked rather than
 * cast, so a page that stops sending its showing fails its tests, as the app would reject it.
 */
function renderDocumentArgs(args: InvokeArgs | undefined): RenderDocumentArgs {
  const showing =
    args !== undefined && "showing" in args ? args.showing : undefined;
  if (typeof showing === "number" || showing === null) return { showing };
  throw new Error("render_document needs its showing, a number or null");
}

/**
 * Gives the fake app's window another document, as Ctrl+O, a drop or a link does, and tells the
 * page, which then asks for it.
 */
export async function giveAnotherDocument(): Promise<void> {
  currentShowing += 1;
  await emit("document-shown");
}

/**
 * Answers `render_document` as the app does (`Documents::handover`): a page whose view shows the
 * current showing reloads its document, and any other page shows it, at its heading or the top.
 * Decided when the page asks, before the read, as the app does. `args` are the request's, as the
 * IPC hands them over.
 */
export async function answerAsTheApp(
  args: InvokeArgs | undefined,
  read: Read,
): Promise<DocumentToShow> {
  const showing = currentShowing;
  const reloads = renderDocumentArgs(args).showing === showing;
  const placeAt = (heading: string | null): Place =>
    reloads ? { place: "reload" } : { place: "show", heading };
  let content: Awaited<ReturnType<Read>>;
  try {
    content = await read();
  } catch (error) {
    const why = error instanceof Error ? error.message : String(error);
    return { showing, ...placeAt(null), error: why };
  }
  if (content === null) return null;
  return typeof content === "string"
    ? { showing, ...placeAt(null), document: content }
    : { showing, ...placeAt(content.heading), document: content.document };
}

/** Lets the page's pending IPC calls, such as starting to listen, finish. */
export function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/** Resolves once the next shown document is in place, as `whenDocumentShown` says. */
export function nextShown(): Promise<void> {
  return new Promise((resolve) => {
    const stop = whenDocumentShown(() => {
      stop();
      resolve();
    });
  });
}

/** The first and last line of the file that `element` comes from, by its `data-sourcepos`. */
export function linesOf(element: Element): [number, number] | undefined {
  const own = element.getAttribute("data-sourcepos");
  if (own) {
    const [start, end] = own.split("-").map((point) => Number.parseInt(point));
    return [start ?? 0, end ?? 0];
  }
  // A wrapper, such as a code block's, sits where the block inside it does; inline text sits in
  // its block.
  const inside = element.querySelector("[data-sourcepos]");
  const around = element.parentElement?.closest("[data-sourcepos]");
  const block = inside ?? around;
  return block ? linesOf(block) : undefined;
}

/**
 * Lays the document out, which happy-dom doesn't: each line of the file is 20 px high, and
 * `topLine` is at the top of the view (1.5 is halfway down the first line). A range sits where
 * the element holding its start does.
 */
export function layOut(topLine: number): void {
  const rectOf = (element: Element | null) => {
    const [start, end] = (element && linesOf(element)) ?? [
      topLine,
      topLine - 1,
    ];
    const top = (start - topLine) * 20;
    return new DOMRect(0, top, 800, (end - start + 1) * 20);
  };
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
    function (this: Element) {
      return rectOf(this);
    },
  );
  vi.spyOn(Range.prototype, "getBoundingClientRect").mockImplementation(
    function (this: Range) {
      return rectOf(this.startContainer.parentElement);
    },
  );
}
