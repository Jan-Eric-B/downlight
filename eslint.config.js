import js from "@eslint/js";
import { defineConfig, globalIgnores } from "eslint/config";
import tseslint from "typescript-eslint";

export default defineConfig(
  globalIgnores(["page/dist/", "target/"]),
  {
    files: ["**/*.{js,ts}"],
    extends: [js.configs.recommended, tseslint.configs.strictTypeChecked],
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    files: ["**/*.js"],
    extends: [tseslint.configs.disableTypeChecked],
  },
  {
    // Only the page's app module speaks to the app, and the tests' fake app answers in its types
    // (ticket 27).
    files: ["page/src/**/*.ts"],
    ignores: [
      "page/src/app.ts",
      "page/src/app.test.ts",
      "page/src/test-support.ts",
    ],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: [
                "@tauri-apps/api/core",
                "@tauri-apps/api/event",
                "@tauri-apps/api/webviewWindow",
              ],
              message:
                "Ask the app and listen to it through page/src/app.ts, and fake it with test-support.ts.",
            },
          ],
        },
      ],
    },
  },
  {
    // The repo's scripts run in Node.
    files: ["scripts/**/*.js"],
    languageOptions: {
      globals: {
        Buffer: "readonly",
        console: "readonly",
        fetch: "readonly",
        process: "readonly",
        WebSocket: "readonly",
      },
    },
  },
);
