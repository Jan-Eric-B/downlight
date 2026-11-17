import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { PageSettings } from "./app";
import { startSettings, whenSettingsChange } from "./settings";
import { fakeTheApp, openPageAt, tell } from "./test-support";

/** The snapshots the listener heard, in turn. */
let heard: PageSettings[];
let stopListening: () => void;

beforeEach(() => {
  mockWindows("document-0");
  fakeTheApp();
  heard = [];
});

afterEach(() => {
  stopListening();
  clearMocks();
  openPageAt("");
});

function listen(): void {
  stopListening = whenSettingsChange((settings) => {
    heard.push(settings);
  });
}

test("a settings listener runs at once with the settings in the page's URL", async () => {
  openPageAt(`?settings=${encodeURIComponent('{"zoom":175}')}`);
  await startSettings();
  listen();
  expect(heard).toEqual([{ zoom: 175 }]);
});

test("a settings listener runs again on each settings message", async () => {
  await startSettings();
  listen();
  await tell("settings", { zoom: 110 });
  await tell("settings", { zoom: 125 });
  expect(heard).toEqual([{ zoom: 100 }, { zoom: 110 }, { zoom: 125 }]);
});

test("a listener added after a settings message starts with that message's settings", async () => {
  await startSettings();
  await tell("settings", { zoom: 90 });
  listen();
  expect(heard).toEqual([{ zoom: 90 }]);
});

test("a removed settings listener hears no more", async () => {
  await startSettings();
  listen();
  stopListening();
  await tell("settings", { zoom: 110 });
  expect(heard).toEqual([{ zoom: 100 }]);
});

test("a settings listener added before the page hears the settings fails", async () => {
  // A fresh module, which no other test has started.
  vi.resetModules();
  const fresh = await import("./settings");

  expect(() => fresh.whenSettingsChange(() => undefined)).toThrow(
    "startSettings()",
  );
});
