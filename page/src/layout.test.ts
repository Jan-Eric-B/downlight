import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { DEFAULT_SETTINGS } from "./app";
import { FULL_WIDTH_CLASS } from "./full-width";
import { startLayout } from "./layout";
import { startSettings } from "./settings";
import {
  answerAsTheApp,
  documentZoom,
  fakeTheApp,
  layOut,
  linesOf,
  openPageAt,
  settle,
  tell,
} from "./test-support";
import { startDocumentView } from "./view";
import { ZOOM_VARIABLE } from "./zoom";

let stopLayout: () => void;

beforeEach(async () => {
  mockWindows("document-0");
  fakeTheApp({
    render_document: (args) =>
      answerAsTheApp(args, () => Promise.resolve(readme)),
  });
  await startSettings();
  stopLayout = startLayout();
});

afterEach(() => {
  stopLayout();
  vi.restoreAllMocks();
  clearMocks();
  openPageAt("");
  document.documentElement.style.removeProperty(ZOOM_VARIABLE);
  document.documentElement.classList.remove(FULL_WIDTH_CLASS);
  document.body.replaceChildren();
});

/** What the view looked like each time the page scrolled a block to its top. */
interface Scroll {
  line: number;
  zoom: string;
  fullWidth: boolean;
}

function spyOnScrolls(): () => Scroll[] {
  const scrolls: Scroll[] = [];
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(function (
    this: Element,
  ) {
    scrolls.push({
      line: linesOf(this)?.[0] ?? 0,
      zoom: documentZoom(),
      fullWidth: document.documentElement.classList.contains(FULL_WIDTH_CLASS),
    });
  });
  return () => scrolls;
}

// One layout change for both, so the 1 MB document wraps anew once rather than twice, and the view
// never lands on a layout with only one of them in place.
test("a snapshot that changes the zoom and Full width at once keeps the reading position with both in place", async () => {
  await startDocumentView(0);
  await settle();
  // Halfway down "## Options", on line 15.
  layOut(15.5);
  const scrolls = spyOnScrolls();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 150, fullWidth: true });
  await vi.waitFor(() => {
    expect(scrolls().length).toBeGreaterThan(0);
  });
  await settle();

  for (const scroll of scrolls())
    expect(scroll).toEqual({ line: 15, zoom: "1.5", fullWidth: true });
});
