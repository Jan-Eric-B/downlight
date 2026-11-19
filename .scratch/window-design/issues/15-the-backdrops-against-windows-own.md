# 15: The backdrops against Windows' own

**What to build:** Find out whether Downlight's Mica and Acrylic look as Windows draws them, and decide what to do where the reader would expect otherwise.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** needs-info

- [ ] Compared side by side by the repo owner (below), with the result recorded here.
- [ ] Decided: keep the system's materials as they are, darken Acrylic in the dark theme, or drop Acrylic.

## Comments

**2026-11-19, from the repo owner's hand check of ticket 08.** With Theme Dark and Backdrop Acrylic, the window looks very gray. Mica looks much less transparent than Mica in WinUI 3 apps.

**What Downlight draws.** The page draws no background on a backdrop (`:root.backdrop { background: transparent }` in `page/src/document.css`), so all the color comes from Windows. window-vibrancy 0.8.1 sets `DWMWA_SYSTEMBACKDROP_TYPE`: Mica is `DWMSBT_MAINWINDOW`, Mica Alt `DWMSBT_TABBEDWINDOW`, Acrylic `DWMSBT_TRANSIENTWINDOW` (from Windows 11 build 22621). DWM draws these with its own tint values, and no attribute changes the tint or opacity. window-vibrancy's Acrylic color is used only on Windows 10's older path.

**What Microsoft documents** ([DWM_SYSTEMBACKDROP_TYPE](https://learn.microsoft.com/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type), [Mica](https://learn.microsoft.com/windows/apps/design/style/mica), [Acrylic](https://learn.microsoft.com/windows/apps/design/style/acrylic), [system backdrops](https://learn.microsoft.com/windows/apps/develop/ui/system-backdrops), checked on 2026-11-19):

- `DWMSBT_TRANSIENTWINDOW` is "Desktop Acrylic, also known as Background Acrylic, in its brightest variant". That fits the gray: it is the material of context menus and flyouts, not of a long-lived window. The Acrylic guidance says "Don't put desktop acrylic on large background surfaces of your app".
- Mica is "an opaque material": it samples only the desktop wallpaper, never the windows behind, and falls back to a neutral color while the window is inactive. Mica that shows other windows through it is Acrylic, not Mica.
- WinUI 3's `MicaBackdrop` and `DesktopAcrylicBackdrop` use the Windows App SDK's `MicaController` and `DesktopAcrylicController` with their default light and dark values. Those controllers let an app set `TintColor`, `TintOpacity`, `LuminosityOpacity` and Acrylic's `Thin` kind, so a WinUI 3 app may look more transparent because it chose to. DWM's system backdrops offer none of these.

**The side-by-side check** for the owner: put Downlight and the WinUI 3 app next to each other over the same part of a colorful wallpaper, nothing else behind them, both in the same theme, both on Mica (not Mica Alt), and click each before comparing (an inactive window shows the neutral fallback). Then the same for Acrylic, against a WinUI 3 app on Desktop Acrylic over the same busy window. Note which app was compared and whether it sets its own tint.

**Options, if they differ:**

1. **Keep the system's materials** (recommended for Mica). They are what Windows draws for every app that asks DWM, and they follow the reader's theme, transparency effects and contrast settings by themselves.
2. **Darken Acrylic in the dark theme**: the page draws a translucent dark layer (`rgb(0 0 0 / x%)`) on `:root.backdrop` while Backdrop is Acrylic. Cheap, but a tint of Downlight's own over the system's that has to be tuned by eye.
3. **The Windows App SDK's controllers** for tint control like WinUI 3's: a dependency on the Windows App Runtime and composition code under a WebView2 window in the per-OS module. Not worth it for a tint.
4. **Drop Acrylic**, following Microsoft's guidance for long-lived windows. It removes a choice a reader may like.
