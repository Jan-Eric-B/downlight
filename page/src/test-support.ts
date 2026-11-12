// Helpers shared by the page's tests. The name has no `.test`, so Vitest doesn't run it as a test
// file, and nothing the app runs imports it.
import { vi } from "vitest";
import { whenDocumentShown } from "./view";

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
