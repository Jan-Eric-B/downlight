// The window's title bar, which the page draws in place of the native one (ADR 0005, window
// design ticket 01): a strip to drag the window by, and minimize, maximize or restore, and close.
// The app turns the native frame off and says so with the `titleBar` flag; without it the window
// keeps its native title bar, as on other systems until their ports.
//
// Dragging, double-clicking and the window menu on a right-click come from WebView2 itself: the
// bar is `app-region: drag` in the stylesheet. On Windows the pointer over the maximize button
// reaches the app's window behind it, which brings back the snap layouts flyout, so the app tells
// the page how to show that button and where it has to be.
// Microsoft's Fluent UI System Icons, so the glyphs look like Windows 11's on Windows 10 and the
// other systems too. Their MIT notice goes into the bundle from vite.config.ts.
import closeGlyph from "@fluentui/svg-icons/icons/dismiss_16_regular.svg?raw";
import maximizeGlyph from "@fluentui/svg-icons/icons/square_16_regular.svg?raw";
import restoreGlyph from "@fluentui/svg-icons/icons/square_multiple_16_regular.svg?raw";
import minimizeGlyph from "@fluentui/svg-icons/icons/subtract_16_regular.svg?raw";
import { askApp, listenToApp, pageFlags } from "./app";
import { commandButton } from "./commands";

/**
 * Tells the app where the maximize button is, in device pixels, so its window behind the button
 * covers it exactly at any DPI and zoom.
 */
function placeMaximizeButton(button: HTMLElement): void {
  const { left, top, right, bottom } = button.getBoundingClientRect();
  const x = Math.round(left * devicePixelRatio);
  const y = Math.round(top * devicePixelRatio);
  askApp("place_maximize_button", {
    button: {
      x,
      y,
      width: Math.round(right * devicePixelRatio) - x,
      height: Math.round(bottom * devicePixelRatio) - y,
    },
  }).catch(() => {
    // Without it only the snap layouts flyout is missing; the page's button still maximizes,
    // and the next layout tries again.
  });
}

/**
 * Draws the title bar when the app left it to the page, and keeps it in step with the window:
 * maximize or restore, the maximize button's look while the pointer is on it, and dimmed while the
 * window isn't active. The signal removes the window listeners again, for the tests.
 */
export function startTitleBar(signal?: AbortSignal): void {
  if (!pageFlags().titleBar) return;
  const bar = document.createElement("header");
  bar.className = "title-bar";
  // Commands, so a button, its shortcut and its menu entry run the same thing (window design spec).
  const minimize = commandButton("minimize", minimizeGlyph);
  const maximize = commandButton("maximize", maximizeGlyph);
  maximize.dataset.state = "normal";
  const close = commandButton("close", closeGlyph);
  close.classList.add("close");
  for (const button of [minimize, maximize, close]) {
    button.classList.add("window-button");
    // Never in the focus, as Windows' own buttons aren't, so the keys go on scrolling the document
    // after a click; the keyboard has Alt+Space for the window menu.
    button.tabIndex = -1;
  }
  const buttons = document.createElement("div");
  buttons.className = "window-buttons";
  buttons.append(minimize, maximize, close);
  bar.append(buttons);
  document.body.prepend(bar);

  // A press anywhere on the bar leaves the focus in the document.
  bar.addEventListener("mousedown", (event) => {
    event.preventDefault();
  });

  // The webview has the focus exactly while the window is active: wry hands it the focus.
  bar.classList.toggle("inactive", !document.hasFocus());
  addEventListener(
    "focus",
    () => {
      bar.classList.remove("inactive");
    },
    { signal },
  );
  addEventListener(
    "blur",
    () => {
      bar.classList.add("inactive");
    },
    { signal },
  );

  // The bar's size in device pixels changes with the window's size, its DPI and the zoom, and
  // each moves the button.
  new ResizeObserver(() => {
    placeMaximizeButton(maximize);
  }).observe(bar, { box: "device-pixel-content-box" });

  // A window opens restored. The app sends the state on every resize, so the button changes only
  // when the state does, rather than parse its glyph again each time.
  let maximized = false;
  listenToApp("window-state", (state) => {
    if (maximized === (state === "maximized")) return;
    maximized = !maximized;
    const label = maximized ? "Restore" : "Maximize";
    maximize.setAttribute("aria-label", label);
    maximize.title = label;
    maximize.innerHTML = maximized ? restoreGlyph : maximizeGlyph;
  });
  listenToApp("maximize-button", (state) => {
    maximize.dataset.state = state;
  });
}
