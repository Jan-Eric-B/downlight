/// <reference types="vitest/config" />
import { defineConfig } from "vite";

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
    // Writes the bundled dependencies' license texts, such as github-markdown-css's MIT notice,
    // next to the page so they ship inside the app.
    license: { fileName: "licenses.md" },
  },
  test: {
    // The page's tests run without a webview (spec: Testing Decisions).
    environment: "happy-dom",
  },
});
