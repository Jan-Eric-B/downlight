import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import code from "../../core/tests/fixtures/code.html?raw";
import { startDocumentView } from "./view";

beforeEach(() => {
  mockWindows("document-0");
  mockIPC((command) =>
    command === "render_document"
      ? { document: code, heading: null }
      : undefined,
  );
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

function view(): HTMLElement {
  const article = document.querySelector<HTMLElement>("article.markdown-body");
  if (!article) throw new Error("The document view isn't there.");
  return article;
}

/** The copy button of the code block whose code starts with `start`. */
function copyButtonOf(start: string): HTMLButtonElement {
  const block = [...view().querySelectorAll("pre")].find((pre) =>
    pre.querySelector("code")?.textContent.startsWith(start),
  );
  const button = block?.parentElement?.querySelector("button");
  if (!button)
    throw new Error(`No copy button on the block starting ${start}.`);
  return button;
}

/** Lets the click's pending promises, such as writing to the clipboard, finish. */
function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

test("each code block has one copy button", async () => {
  await startDocumentView(0);

  const blocks = view().querySelectorAll("pre");
  expect(blocks.length).toBe(9);
  for (const block of blocks) {
    // Beside the block rather than in it, as on GitHub, since a block scrolls sideways.
    expect(block.parentElement?.querySelectorAll("button")).toHaveLength(1);
  }
  expect(view().querySelectorAll("button")).toHaveLength(9);
});

test("the copy button copies exactly the code's text", async () => {
  const writeText = vi.spyOn(navigator.clipboard, "writeText");
  await startDocumentView(0);

  copyButtonOf("/// Says hello.").click();
  copyButtonOf("\tindented").click();
  await settle();

  // As written in code.md, with the highlighting's spans gone.
  expect(writeText.mock.calls).toEqual([
    [
      '/// Says hello.\nfn main() {\n    let name = "<world> & co";\n    println!("Hello, {name}!");\n}\n',
    ],
    ["\tindented with a tab\ntrailing spaces   \n"],
  ]);
});

test("a copy button shows it copied for 2 s after its last copy", async () => {
  await startDocumentView(0);
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  const button = copyButtonOf("[package]");
  const shows = () => ({
    copied: button.classList.contains("copied"),
    label: button.getAttribute("aria-label"),
    title: button.title,
  });
  const copied = { copied: true, label: "Copied", title: "Copied" };
  const ready = { copied: false, label: "Copy code", title: "Copy code" };

  button.click();
  // The async variant also lets the clipboard's promise settle.
  await vi.advanceTimersByTimeAsync(0);
  expect(shows()).toEqual(copied);

  // A second copy restarts the 2 s.
  await vi.advanceTimersByTimeAsync(1500);
  button.click();
  await vi.advanceTimersByTimeAsync(1500);
  expect(shows()).toEqual(copied);

  await vi.advanceTimersByTimeAsync(500);
  expect(shows()).toEqual(ready);
});

test("copy buttons add no text to the document", async () => {
  await startDocumentView(0);

  // Find and selecting text see the document's text only, never a button's.
  const rendered = document.createElement("div");
  rendered.innerHTML = code;
  expect(view().textContent).toBe(rendered.textContent);
});

test("copy buttons are named for screen readers", async () => {
  await startDocumentView(0);

  const button = copyButtonOf("[package]");
  expect(button.type).toBe("button");
  expect(button.getAttribute("aria-label")).toBe("Copy code");
});

test("a copy that fails is shown to the reader", async () => {
  vi.spyOn(navigator.clipboard, "writeText").mockRejectedValue(
    new Error("Clipboard is busy"),
  );
  await startDocumentView(0);

  copyButtonOf("[package]").click();
  await settle();

  expect(document.querySelector(".notice")?.textContent).toBe(
    "Can't copy the code: Error: Clipboard is busy",
  );
});
