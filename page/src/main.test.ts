import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { answerAsTheApp, fakeTheApp } from "./test-support";

afterEach(() => {
  clearMocks();
  document.body.replaceChildren();
});

test("shows the document the core rendered", async () => {
  mockWindows("document-0");
  fakeTheApp({
    render_document: (args) =>
      answerAsTheApp(args, () => Promise.resolve(readme)),
  });

  // The page starts when its script loads, and the import waits until it shows the document.
  await import("./main");

  const view = document.querySelector("article.markdown-body");
  expect(view?.querySelector("h1")?.textContent).toBe("Lamplighter");
  expect(view?.querySelector("pre code")?.textContent).toBe("lamp --version\n");
});
