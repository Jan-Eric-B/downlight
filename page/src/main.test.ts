import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { answerAsTheApp, mockTheApp } from "./test-support";

afterEach(() => {
  clearMocks();
  document.body.replaceChildren();
});

test("shows the document the core rendered", async () => {
  mockWindows("document-0");
  mockTheApp((command, args) =>
    command === "render_document"
      ? answerAsTheApp(args, () => Promise.resolve(readme))
      : undefined,
  );

  // The page starts when its script loads, and the import waits until it shows the document.
  await import("./main");

  const view = document.querySelector("article.markdown-body");
  expect(view?.querySelector("h1")?.textContent).toBe("Lamplighter");
  expect(view?.querySelector("pre code")?.textContent).toBe("lamp --version\n");
});
