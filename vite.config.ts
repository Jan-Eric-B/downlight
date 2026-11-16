/// <reference types="vitest/config" />
import { defineConfig } from "vite";

/** The license of Fluent UI System Icons, whose glyphs the title bar inlines (title-bar.ts). */
const FLUENT_ICONS_NOTICE = `/*! Fluent UI System Icons (@fluentui/svg-icons)
 *
 * MIT License
 *
 * Copyright (c) 2020 Microsoft Corporation
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */`;

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
    rolldownOptions: {
      output: {
        // @fluentui/svg-icons ships no license file, so licenses.md names it without its text;
        // its notice goes at the top of the bundle, where minifying keeps it.
        postBanner: FLUENT_ICONS_NOTICE,
      },
    },
  },
  test: {
    // The page's tests run without a webview (spec: Testing Decisions).
    environment: "happy-dom",
    // Vitest empties every stylesheet unless told otherwise, and the accent scale's test reads the
    // scale's stylesheet as text (accent.test.ts). Only that import is kept.
    css: { include: [/\/page\/src\/accent\.css\?raw$/] },
  },
});
