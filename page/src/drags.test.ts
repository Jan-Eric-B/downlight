import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test } from "vitest";
import { startTaggingDrags } from "./drags";
import { dragTagSample } from "./test-support";

let stopTagging: () => void;

beforeEach(() => {
  mockWindows("document-3");
  stopTagging = startTaggingDrags();
});

afterEach(() => {
  stopTagging();
  clearMocks();
  document.body.replaceChildren();
});

/** Starts a drag of `element`, as the webview does, and gives the drag's data. */
function startDragging(element: HTMLElement): DataTransfer {
  const data = new DataTransfer();
  data.setData("text/plain", element.textContent);
  const event = new Event("dragstart", { bubbles: true, cancelable: true });
  // happy-dom has no DragEvent, so the drag's data goes onto a plain event.
  Object.defineProperty(event, "dataTransfer", { value: data });
  element.dispatchEvent(event);
  return data;
}

test("a drag that starts in the page carries its window's tag, as the app's drop target reads it", () => {
  const link = document.createElement("a");
  link.href = "https://example.com/";
  link.textContent = "A link";
  document.body.append(link);

  const data = startDragging(link);

  expect(data.getData(dragTagSample())).toBe("document-3");
  // What the drag carries otherwise stays as it was.
  expect(data.getData("text/plain")).toBe("A link");
});

test("a drag that starts inside an element that stops its events still carries the tag", () => {
  const field = document.createElement("div");
  field.addEventListener("dragstart", (event) => {
    event.stopPropagation();
  });
  const selection = document.createElement("span");
  selection.textContent = "Selected text";
  field.append(selection);
  document.body.append(field);

  expect(startDragging(selection).getData(dragTagSample())).toBe("document-3");
});
