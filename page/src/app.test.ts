import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, expect, test, vi } from "vitest";
import { listenToApp } from "./app";

afterEach(() => {
  clearMocks();
});

// The mocks deliver an event to every listener, but Tauri delivers an event the app sends one
// window only to listeners that name that window, so this checks the name.
test("listens to what the app sends this window", async () => {
  mockWindows("document-3");
  const listens: unknown[] = [];
  mockIPC((command, args) => {
    if (command === "plugin:event|listen") listens.push(args);
    return 0;
  });

  listenToApp("document-shown", () => {});

  await vi.waitFor(() => {
    expect(listens).toEqual([
      expect.objectContaining({
        event: "document-shown",
        target: { kind: "WebviewWindow", label: "document-3" },
      }),
    ]);
  });
});
