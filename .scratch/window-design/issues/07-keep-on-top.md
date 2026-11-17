# 07: Keep on top

**What to build:** **Keep on top** keeps one window above all others. A toggle button in the title bar, Ctrl+Alt+T and a right-click menu entry switch it. It belongs to the window and is gone when the window closes.

Spec: [Window design](../spec.md)

**Blocked by:** 03 (The accent color in the page), 05 (Open and Edit buttons, and the separators)

**Status:** ready-for-human

- [x] The button, Ctrl+Alt+T and the menu entry run one command, and the button shows in the accent color while it is on. (Built and tested up to `aria-pressed`; the color is hand check 4.)
- [ ] The window stays above other windows while it is on, and only that window. (Built; only a real window shows it, so hand checks 1 to 3 tick it.)
- [x] It is not saved: a new window, or the same document opened again after closing, starts without it. (Tested in the page; hand check 5.)
- [x] Page tests: the command asks the app to switch it, and the button shows the state the app answers with.
- [ ] Checked by hand: above other apps, above another Downlight window, and while maximized.

## Comments

**2026-11-16, decided after the review of tickets 03 and 04.** The switched-on button looks as ticket 06's comment says: its glyph in `--accent-emphasis`, no fill, dimmed with the bar in an inactive window. Whichever of 06 and 07 comes first adds the toggle state to `commandButton` once (see ticket 06's comment).

**2026-11-17, built; the hand checks are left.** What was built, and what it decided:

- **Window state in the app, not a setting.** The new command `switch_keep_on_top` (`app/src/keep_on_top.rs`) takes nothing and flips Tauri's always-on-top of the calling window, and only that one, and answers whether it is kept on top now. Flipped in the app rather than set to what the page shows, as Full width is, so the page always shows the window's own state. Nothing saves it, and `window::open_window` builds every window with Tauri's default of off, so a new window, and a document opened again after its window closed, start without it. A switch that fails rejects with "Keep on top didn't switch: ...", shown as a notice.
- **No code in the per-OS module.** Tauri 2.12.5 has `WebviewWindow::is_always_on_top` and `set_always_on_top` (read in its source at the version in Cargo.lock); on Windows tao keeps the state in its window flags and sets `HWND_TOPMOST`. That is Tauri's cross-platform API, with no OS call or `unsafe` of Downlight's own, so "One module per OS" doesn't apply, and the macOS and Linux ports get it from Tauri too. No capability changed: the command is the app's own.
- **One change in the Windows module all the same:** `platform::show_error`'s box belongs to no window, so it would open behind a window kept on top, as a second launch's failures (`downlight missing.md` while a window is kept on top) do. It is `MB_TOPMOST` now. `ask`'s box and the open dialogs belong to their window, and Windows keeps them above it.
- **The page** has `page/src/keep-on-top.ts`: `switchKeepOnTop()` asks the app and keeps its answer, which `isKeptOnTop()` gives, so the button and the menu's check mark show what the app answered, never what the page expected. The command `keepOnTop` in `page/src/commands.ts`: "Keep on top", Ctrl+Alt+T, `pressed: isKeptOnTop`; it runs `refreshCommands()` once the answer is in, since no message tells the page about the window's own state. A settings change runs the refresh too and leaves it as it was. The state lives in the page module for the page's life, which is the window's (the page never navigates). Its button is the 16 px regular `pin` glyph after Full width in the bar's second group, as Windows apps mark keeping a window on top with a pin; its menu line is after Full width, with a check mark (`checked`). The switched-on look is ticket 06's, styled once for every toggle, so no CSS changed. AltGr+T that types a character isn't the shortcut, by 06's `pressesKey`.
- **Contract:** `switch_keep_on_top` is the first command to answer with a flag. Its answer has a sample, `app/tests/fixtures/keep_on_top.json` (`switchedOn`, `switchedOff`, of the type `KeptOnTop` the command returns), which a contract test drives through the page's button; the test support's fake app must be told how to answer it, as `follow_link`. `npm run fixtures` added only that file.
- **Tests changed:** `title-bar.test.ts`'s row of buttons and `commands.test.ts`'s right-click menu both list every entry, and Keep on top is now one of them in the spec's place.
- **Tests** (written first, failing): `page/src/keep-on-top.test.ts` (the button, Ctrl+Alt+T and the menu entry each ask the app; the tooltip; a new window starts unpressed and unticked; the button and the check mark follow the app's answers both ways; the button shows the app's answer, not the expected one; a settings change leaves it; a failed switch says why and leaves the button); `contract.test.ts` (the sampled answers press the button and release it). No `cargo test` for the command: it is two calls to the window, which only a real window can answer (Tauri's mock runtime always says off), as for minimize and maximize.

**Hand checks** (on the release build, `npm run tauri build`), to record here:

1. Keep on top's button, Ctrl+Alt+T and the right-click menu's Keep on top each keep the window above another app (Explorer, a browser) when that app is clicked, and switch it off again; the tooltip says "Keep on top (Ctrl+Alt+T)".
2. With two Downlight windows, only the one switched stays above the other and above other apps; the other's button is not on. A new window opened from the one kept on top (Ctrl+click a link) is not kept on top and may open partly behind it.
3. Maximized and then restored, the window stays on top; the taskbar stays visible above a maximized window.
4. While on, the pin glyph is amber (dark amber in light, light amber in dark), with no fill, and dims when another window is active; the menu entry has a check mark. A real contrast theme: the glyph is in the highlight color.
5. Closed while on, the same document opened again (and a second window opened meanwhile) starts without it; nothing about it is in the settings file.
6. While a window is kept on top, `downlight missing.md` from a terminal shows its failure box above that window.
7. AltGr+W and AltGr+T on US-International type å and þ in Find and switch nothing.

**2026-11-17, review fixes.** Keep on top's page state gets a start function, as the title bar's and the commands' do: `startKeepOnTop(signal)` in `page/src/keep-on-top.ts` starts the page not kept on top, and the signal starts it so again. `main.ts` runs it before the title bar; the tests run it in their setup, so `keep-on-top.test.ts` no longer clicks the button off through the fake app after each test, and `contract.test.ts` no longer has to switch it off last, which let a failure midway leak into later tests. `switchKeepOnTop` handles a rejection once: `showingWhy` shows it, and the state stays as it was (`await switched.catch(() => keptOnTop)`).

**2026-11-17, review fixes.** A switch answered after the page's state started afresh (`startKeepOnTop`, or its signal aborting) no longer writes its answer into the fresh state: `switchKeepOnTop` notes the start it was asked in and drops an answer from an earlier one, which in the tests leaked Keep on top on into the next test. New test, written first and failing: `keep-on-top.test.ts`, a switch the app answers after the window's page started again leaves Keep on top off.
