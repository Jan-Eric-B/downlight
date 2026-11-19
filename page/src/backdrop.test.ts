import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test } from "vitest";
import { DEFAULT_SETTINGS, type PageSettings } from "./app";
import { startBackdrop } from "./backdrop";
import { startSettings } from "./settings";
import { fakeTheApp, openPageAt, openPageWith, tell } from "./test-support";

let stopBackdrop: () => void;

beforeEach(() => {
  mockWindows("document-0");
  fakeTheApp();
});

afterEach(() => {
  stopBackdrop();
  clearMocks();
  openPageAt("");
  document.documentElement.classList.remove("backdrop");
});

/** Whether the page draws no background of its own, so the window's backdrop shows. */
function onBackdrop(): boolean {
  return document.documentElement.classList.contains("backdrop");
}

/** Opens the page as the app does where the OS can show a backdrop, with `settings`. */
async function startOnWindows11(settings: PageSettings): Promise<void> {
  openPageAt(
    `?backdropSupported&settings=${encodeURIComponent(JSON.stringify(settings))}`,
  );
  await startSettings();
  stopBackdrop = startBackdrop();
}

test("where the OS can show a backdrop, the page draws no background on the reader's Mica", async () => {
  await startOnWindows11(DEFAULT_SETTINGS);
  expect(onBackdrop()).toBe(true);
});

test("a window that opens with Backdrop None draws its solid background", async () => {
  await startOnWindows11({ ...DEFAULT_SETTINGS, backdrop: "none" });
  expect(onBackdrop()).toBe(false);
});

test("a settings message with Backdrop None brings back the solid background, and another backdrop takes it away", async () => {
  await startOnWindows11(DEFAULT_SETTINGS);
  await tell("settings", { ...DEFAULT_SETTINGS, backdrop: "none" });
  expect(onBackdrop()).toBe(false);
  await tell("settings", { ...DEFAULT_SETTINGS, backdrop: "acrylic" });
  expect(onBackdrop()).toBe(true);
});

test("where the OS shows no backdrop, as on Windows 10, the page draws its solid background whatever the reader's Backdrop", async () => {
  openPageWith();
  await startSettings();
  stopBackdrop = startBackdrop();
  expect(onBackdrop()).toBe(false);
  await tell("settings", { ...DEFAULT_SETTINGS, backdrop: "micaAlt" });
  expect(onBackdrop()).toBe(false);
});
