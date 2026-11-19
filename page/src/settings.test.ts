import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { DEFAULT_SETTINGS, type PageSettings } from "./app";
import {
  startSettings,
  whenSettingChanges,
  whenSettingsChange,
} from "./settings";
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
  openPageAt(
    `?settings=${encodeURIComponent(JSON.stringify({ ...DEFAULT_SETTINGS, zoom: 175 }))}`,
  );
  await startSettings();
  listen();
  expect(heard).toEqual([{ ...DEFAULT_SETTINGS, zoom: 175 }]);
});

test("a settings listener runs again on each settings message", async () => {
  await startSettings();
  listen();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 110 });
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 125 });
  expect(heard).toEqual([
    { ...DEFAULT_SETTINGS, zoom: 100 },
    { ...DEFAULT_SETTINGS, zoom: 110 },
    { ...DEFAULT_SETTINGS, zoom: 125 },
  ]);
});

test("a listener added after a settings message starts with that message's settings", async () => {
  await startSettings();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 90 });
  listen();
  expect(heard).toEqual([{ ...DEFAULT_SETTINGS, zoom: 90 }]);
});

test("a removed settings listener hears no more", async () => {
  await startSettings();
  listen();
  stopListening();
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 110 });
  expect(heard).toEqual([{ ...DEFAULT_SETTINGS, zoom: 100 }]);
});

test("a listener to one setting runs at once and then only when that setting changes", async () => {
  await startSettings();
  const backdrops: string[] = [];
  stopListening = whenSettingChanges("backdrop", (backdrop) => {
    backdrops.push(backdrop);
  });
  await tell("settings", { ...DEFAULT_SETTINGS, zoom: 110 });
  await tell("settings", { ...DEFAULT_SETTINGS, backdrop: "none" });
  await tell("settings", { ...DEFAULT_SETTINGS, backdrop: "none", zoom: 90 });
  await tell("settings", DEFAULT_SETTINGS);
  expect(backdrops).toEqual(["mica", "none", "mica"]);
});

test("a removed listener to one setting hears no more", async () => {
  await startSettings();
  const backdrops: string[] = [];
  whenSettingChanges("backdrop", (backdrop) => {
    backdrops.push(backdrop);
  })();
  stopListening = () => undefined;
  await tell("settings", { ...DEFAULT_SETTINGS, backdrop: "none" });
  expect(backdrops).toEqual(["mica"]);
});

test("a settings listener added before the page hears the settings fails", async () => {
  // A fresh module, which no other test has started.
  vi.resetModules();
  const fresh = await import("./settings");

  expect(() => fresh.whenSettingsChange(() => undefined)).toThrow(
    "startSettings()",
  );
});

test("reading the settings before the page hears them fails", async () => {
  vi.resetModules();
  const fresh = await import("./settings");

  expect(() => fresh.settingsNow()).toThrow("startSettings()");
});
