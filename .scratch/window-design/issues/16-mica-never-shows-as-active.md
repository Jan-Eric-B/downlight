# 16: Mica never shows as active

**What to build:** Find why Downlight's window always shows Mica's inactive look (a flat neutral color) even while it is the active window, and fix it so the active window shows Mica tinted by the wallpaper, as other Windows 11 apps do. Acrylic and Mica Alt most likely share the cause.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** needs-info

- [x] The cause found, by the experiments below or others, each recorded here with its result.
- [ ] Fixed in the per-OS module (`app/src/platform/windows.rs`), keeping ticket 13's fix (no white behind a dark page while a window opens) and ticket 01's snap layouts flyout.
- [ ] Checked by hand by the repo owner: Mica, Mica Alt and Acrylic, light and dark, active and inactive, next to another app on the same wallpaper; ten launches in dark mode with no white frame (ticket 13's check); resize and maximize with no black or see-through edges; the snap layouts flyout on the maximize button.

## Comments

**2026-11-19, found by the repo owner** while comparing backdrops for ticket 15. Two windows side by side on the same dark wallpaper (dark blue on the left, red lines on the right), both active, both in dark mode with Mica:

- **WinDown** (the owner's earlier WinUI 3 Markdown viewer) shows Mica as expected: a dark backdrop with a visible blue-to-red tint from the wallpaper behind it.
- **Downlight** (empty window, Backdrop Mica) is a flat neutral gray, about `#202020`, the color Mica falls back to for an inactive window in dark mode. It looks the same active and inactive, and the same as WinDown when WinDown is inactive.
- Earlier the same day, with Backdrop Acrylic in dark mode, the owner saw the window as "really gray", a flat mid-gray. That fits Acrylic stuck in its inactive fallback too.

Machine: Windows 11 Enterprise build 26200, transparency effects on (WinDown shows Mica, so the system's own fallback conditions don't apply). Versions from `Cargo.lock`: tauri 2.12.5, tao 0.37.2, wry 0.57.2, window-vibrancy 0.8.1, webview2-com 0.39.1.

**How Downlight builds a window** (`open_window` in `app/src/window.rs`, `with_backdrop` and `set_backdrop` in `app/src/platform/windows.rs`):

- `.decorations(false)`: the page draws the title bar (ADR 0005). tao keeps `WS_CAPTION` on a frameless window (`to_window_styles` in tao's `window_state.rs`) but answers `WM_NCCALCSIZE` with 0 and the whole window, or the window less tao's insets when Tauri's `shadow` is on (the default, `MARKER_UNDECORATED_SHADOW`), so the window has no non-client area (tao's `event_loop.rs`, `WM_NCCALCSIZE`).
- `.visible(false)`: the window is built hidden and shown at first paint (ticket 04), or after 5 s at the latest.
- `.transparent(true).no_redirection_bitmap(true)` (ticket 13), then `.effects(...)` with the reader's backdrop. Tauri applies the effect through window-vibrancy (`tauri/src/vibrancy/windows.rs`): Mica is `apply_mica(hwnd, None)`, which sets `DWMWA_SYSTEMBACKDROP_TYPE` to `DWMSBT_MAINWINDOW`; Mica Alt is `DWMSBT_TABBEDWINDOW`; Acrylic is `DWMSBT_TRANSIENTWINDOW`. A live change from the settings flyout calls `set_effects` again.
- `.theme(...)`: tao's theme handling sets `DWMWA_USE_IMMERSIVE_DARK_MODE`, and its `dark_mode.rs` sends `WM_NCACTIVATE` pairs through `DefWindowProcW` to refresh the frame.
- The page draws no background on a backdrop (`:root.backdrop { background: transparent }` in `page/src/document.css`), so the gray isn't the page's: without a backdrop the page draws GitHub's `#0d1117`, which is bluish.
- Two kinds of child window sit over the client area: WebView2's (from another process) and Downlight's small window over the maximize button (`app/src/platform/windows/title_bar.rs`), plus Tauri's drag-resize border windows.
- Nothing calls `DwmExtendFrameIntoClientArea`: not Downlight, not tao 0.37.2, not Tauri's effects.

**Ruled out:**

- **The system's fallback conditions** (transparency effects off, Remote Desktop, weak graphics, battery saver, high contrast): WinDown shows active Mica on the same screen at the same time.
- **tao swallowing activation:** tao passes `WM_NCACTIVATE` on to `DefWindowProc` (`event_loop.rs`, after recording the active state), and keeps `WS_CAPTION`.
- **Downlight's own message handling:** the only window procedure of Downlight's own is the maximize button's child window, which handles `WM_NCHITTEST`, mouse messages and `WM_NCDESTROY`, nothing about activation.
- **The missing redirection surface (ticket 13) as the only cause.** Experiment 1, 2026-11-19: a release build without `.no_redirection_bitmap(true)` (so tao calls `DwmEnableBlurBehindWindow` with an empty region instead) still showed flat gray, active and inactive. The line is back; the experiment wasn't committed.

**Not known:** whether Mica ever looked active in Downlight. Ticket 02 (native frame, before ADR 0005) recorded "Mica shows in the native title bar" and DWM reporting `DWMSBT_MAINWINDOW`, and the hand checks of tickets 04, 13 and 08 recorded Mica behind the page, but none compared the active look against another app, so each may have seen the fallback.

**Suspects, most likely first, each with an experiment.** Each experiment is a release build (`npm run tauri build`) the owner looks at next to another app with Mica, active and inactive; no test can observe the backdrop (see ticket 13), and agents run only the project's own commands (no scripts of their own that capture the screen or drive windows). Revert each experiment after it.

1. **No frame extended into the client area.** The common recipe for DWM's system backdrop under a custom frame calls `DwmExtendFrameIntoClientArea` (margins `-1`, or at least the top) besides `DWMWA_SYSTEMBACKDROP_TYPE`; Downlight's window has no non-client area and no extended frame, so DWM may draw the backdrop with its fallback brush. Experiment 2: after `build()`, call `DwmExtendFrameIntoClientArea(hwnd, &MARGINS { -1, -1, -1, -1 })` in the per-OS module (the `windows` crate's `Win32_Graphics_Dwm` feature), keeping everything else. Also try it with ticket 13's line removed, since tao's blur-behind and an extended frame interact.
2. **The frameless window itself.** Experiment 3: `.decorations(true)` for one build (the page's title bar then sits under the native one, which is fine for a look). If the native caption and the client area show active Mica, the cause is the frameless setup (suspects 1 or 4); if not, it is in the effect or the webview.
3. **Tauri's shadow option** changes tao's `WM_NCCALCSIZE` (insets or none). Experiment 4: `.shadow(false)`, then `.shadow(true)` explicitly, each with experiment 2's call.
4. **The effect set while the window is hidden.** The backdrop is applied at build time, before the window is first shown and activated. Experiment 5: call `set_backdrop` again right after the first show and on the first `Focused(true)`. Whether a live Backdrop change in the flyout ever showed active Mica isn't recorded; the owner can check that without a build: switch Backdrop to Acrylic and back to Mica while the window is active.
5. **Something painting over the backdrop.** The gray could also be a surface of its own (WebView2's default background or tao's `WM_ERASEBKGND` background color) that only looks like Mica's fallback. Checks without a build: with Theme Light, Mica's inactive fallback is about `#F3F3F3`, and with transparency effects off DWM draws a solid color of its own. If Downlight's gray doesn't change between Mica, Mica Alt and Acrylic, or with these switches, it isn't the backdrop. With Backdrop None the page draws `#0d1117`, which should look clearly bluer.

**Useful for comparing:** WinDown uses the Windows App SDK's `MicaController`, which composes its own Mica, not DWM's `DWMSBT_MAINWINDOW`, so its tint can differ a little from DWM's even once this is fixed. The question here is only active against inactive: DWM's Mica must change between the two, and Downlight's doesn't. A window inspector (Spy++ from Visual Studio, if installed, run by the owner) can show the styles of Downlight's window and its children next to WinDown's.

Related: ticket 15 (the backdrops against Windows' own) waits for this one; ticket 13 (white backdrop behind a dark page) must keep working.

**2026-11-19, the activation path checked from inside the app.** A temporary module (`app/src/platform/windows/backdrop_experiment.rs`, switched by `DOWNLIGHT_BACKDROP_EXPERIMENT`, to be reverted) logged the window's messages and DWM's attributes on the release build:

- The window has `WS_CAPTION | WS_SYSMENU | WS_SIZEBOX | WS_MINIMIZEBOX | WS_MAXIMIZEBOX` (style `0x14cf0000` once shown) and `WS_EX_NOREDIRECTIONBITMAP | WS_EX_APPWINDOW | WS_EX_ACCEPTFILES | WS_EX_WINDOWEDGE` (`0x240110`).
- DWM reads back `DWMWA_SYSTEMBACKDROP_TYPE` = 2 (`DWMSBT_MAINWINDOW`) from the build on, and `DWMWA_NCRENDERING_ENABLED` = 1.
- On activation the window gets `WM_NCACTIVATE(TRUE)`, which reaches `DefWindowProc` and returns 1, and `GetForegroundWindow()` is the window. On deactivation it gets `WM_NCACTIVATE(FALSE)`, `WM_ACTIVATE(WA_INACTIVE)` and `WM_ACTIVATEAPP(FALSE)`.
- So Windows and DWM are told the window is active, and the attribute is set: what Electron fixed (Chromium never passing `WM_NCACTIVATE` on to `DefWindowProc` for a frameless window, [electron#39708](https://github.com/electron/electron/pull/39708)) isn't the cause here. Left: how DWM draws this window (suspects 1 to 3), something drawn over the backdrop (suspect 5), or Windows itself.
- The same day, Personalize's `ColorPrevalence` (accent color on title bars) was 0 and `EnableTransparency` 1, and there are no DWM policies, so neither forces a solid caption.

**Reported elsewhere:** [WebView2Feedback#5409](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5409) (October 2025) is a Tauri app with window-vibrancy on build 26200 whose Mica stopped showing while the window is focused after a Windows update; it was closed without a cause. [electron#29937](https://github.com/electron/electron/issues/29937) saw Mica only inactive on frameless transparent windows (fixed in Electron by the activation patch above). [mygo#63](https://github.com/egoist/mygo/pull/63) needed `WS_EX_NOREDIRECTIONBITMAP`, which Downlight already has.

**The experiment build.** One release build (`npm run tauri build -- --no-bundle`) took the switches, comma-separated, from `DOWNLIGHT_BACKDROP_EXPERIMENT`: `nowebview` (hid the webview 2 s after showing, leaving the bare backdrop), `frame` (experiment 3), `extend` (experiment 2), `redirect` (ticket 13's line off), `noshadow` (experiment 4), `reapply` (experiment 5, after the first show), `opaque` (neither transparent nor without the redirection surface), `notheme` (no theme set on the window) and `log`. Reverted after the runs below; it wasn't committed.

**2026-11-19, the runs, checked by the repo owner** on Windows 11 25H2, build 26200.9711, dark mode, Backdrop Mica, each window clicked and then WinDown clicked, on the same wallpaper:

| Run | Switches | Result |
| --- | --- | --- |
| 1 | `nowebview` | Flat gray active and inactive: nothing is drawn over the backdrop (suspect 5 ruled out). |
| 2 | `frame` | Flat gray, the native title bar too (suspects 1 to 3, the frameless setup, ruled out). |
| 3 | `frame,redirect` | Flat gray: not the missing redirection surface. |
| 4 | `frame,reapply` | Flat gray: not the backdrop set while hidden (suspect 4 ruled out). |
| 5 | `frame,opaque,notheme` | The plainest window Tauri makes with Mica: its native title bar is still flat ("something there, but flat"), with TranslucentTB running and after quitting it. |

- **File Explorer is flat too**, active and inactive, while **Calculator and WinDown show active Mica**. Explorer gets its backdrop from DWM, as Downlight does (`DWMWA_SYSTEMBACKDROP_TYPE`); Calculator (WinUI 2) and WinDown (Windows App SDK's `MicaController`) compose their own Mica in the app from the blurred wallpaper. So on this machine DWM's system backdrop doesn't draw active Mica for any app, and Downlight looks as Explorer does.
- **Cause:** Windows, not Downlight's window. It fits the reports of Mica broken on 25H2 after an update ([WebView2Feedback#5409](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5409), a Tauri app on 26200; [a Stardock forum thread](https://forums.stardock.com/541590/latest-windows-update-screwed-up-the-mica-effect) on 26200.8457 that also names File Explorer). Not known: whether every 25H2 machine shows it, or only this one.
- Nothing in `app/src/platform/windows.rs` changes. Experiments 2 (`extend`) and 4 (`noshadow`) weren't run: with the native frame flat too, they can't help.

**Open, for the repo owner:** compare File Explorer and Calculator on another Windows 11 machine. If Explorer's Mica is active there, Downlight's will be too, and this ticket closes as Windows'. Then decide: keep DWM's backdrop (Downlight looks as Explorer does wherever Windows draws Mica), or compose Downlight's own Mica in the app as Calculator and WinDown do: Windows.UI.Composition's blurred wallpaper brush (`TryCreateBlurredWallpaperBackdropBrush`, in Windows 11 itself, no Windows App Runtime) with a tint, on a `DesktopWindowTarget` behind the webview, in the per-OS module, following activation, theme, transparency effects and high contrast itself. It would also give the tint control ticket 15 lacks, but it is a ticket of its own, and on machines where DWM works it looks a little different from Explorer.
