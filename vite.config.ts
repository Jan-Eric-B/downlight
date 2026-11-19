/// <reference types="vitest/config" />
import { readFileSync } from "node:fs";
import { defineConfig } from "vite";

/** The license of Fluent UI System Icons, whose glyphs the title bar and the find bar inline
 * (title-bar.ts, find.ts). The package ships no license file of its own, so the text is kept in the
 * page for the banner below and for the third-party notices (scripts/notices.js, ticket 36). */
const FLUENT_ICONS_LICENSE = readFileSync(
  new URL("page/fluent-icons-license.txt", import.meta.url),
  "utf8",
);

const FLUENT_ICONS_NOTICE = [
  "/*! Fluent UI System Icons (@fluentui/svg-icons)",
  " *",
  ...FLUENT_ICONS_LICENSE.trimEnd()
    .split("\n")
    .map((line) => (line === "" ? " *" : ` * ${line}`)),
  " */",
].join("\n");

export default defineConfig({
  root: "page",
  // Keeps Tauri's output visible when both run in one terminal.
  clearScreen: false,
  server: {
    // Tauri's devUrl expects this port.
    port: 5173,
    strictPort: true,
  },
  build: {
    // The bundled packages' licenses ship in the installer's third-party notices, beside the exe
    // rather than inside it (ticket 36): scripts/notices.js turns build.license on for its own
    // build, which writes nothing to page/dist.
    license: false,
    rolldownOptions: {
      output: {
        // The notice inside the code it covers: minifying keeps a `/*!` comment at the top of the
        // bundle.
        postBanner: FLUENT_ICONS_NOTICE,
      },
    },
  },
  test: {
    // The page's tests run without a webview (spec: Testing Decisions).
    environment: "happy-dom",
    // Vitest empties every stylesheet unless told otherwise, and the accent color's test reads
    // every stylesheet of the page as text (accent.test.ts). Only those imports are kept.
    css: { include: [/\/page\/src\/[^/]+\.css\?raw$/] },
  },
});
