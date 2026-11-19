// The window's title bar, which the page draws in place of the native one (ADR 0005, window
// design tickets 01, 04 and 05): Downlight's logo, the shown document's file name and its zoom
// level when it isn't 100%, a strip to drag the window by, the buttons of the reader's commands
// in groups, and minimize, maximize or restore, and close.
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
import editGlyph from "@fluentui/svg-icons/icons/edit_16_regular.svg?raw";
import fullWidthGlyph from "@fluentui/svg-icons/icons/arrow_bidirectional_left_right_16_regular.svg?raw";
import openGlyph from "@fluentui/svg-icons/icons/folder_open_16_regular.svg?raw";
import keepOnTopGlyph from "@fluentui/svg-icons/icons/pin_16_regular.svg?raw";
import settingsGlyph from "@fluentui/svg-icons/icons/settings_16_regular.svg?raw";
import maximizeGlyph from "@fluentui/svg-icons/icons/square_16_regular.svg?raw";
import restoreGlyph from "@fluentui/svg-icons/icons/square_multiple_16_regular.svg?raw";
import minimizeGlyph from "@fluentui/svg-icons/icons/subtract_16_regular.svg?raw";
// Inlined rather than loaded: the content security policy's img-src has no 'self' (window design
// spec). Its ids are prefixed for sharing the page's namespace.
import logo from "../../app/icons/icon.svg?raw";
import {
  askApp,
  DEFAULT_SETTINGS,
  listenToApp,
  pageFlags,
  type DocumentFile,
} from "./app";
import { commandButton, type CommandId } from "./commands";
import { showingWhy } from "./notice";
import { settingsNow, whenSettingsChange } from "./settings";
import { showsDocument, whenDocumentShown } from "./view";

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
 * The class of everything on the bar that takes clicks, which the window isn't dragged by
 * (`app-region` in title-bar.css).
 */
const BAR_ITEM_CLASS = "bar-item";

/**
 * One of the bar's own items at its left, which never takes the focus, as the window buttons
 * don't.
 */
function barItem(className: string, label: string): HTMLButtonElement {
  const item = document.createElement("button");
  item.type = "button";
  item.classList.add(BAR_ITEM_CLASS, className);
  item.title = label;
  item.tabIndex = -1;
  return item;
}

/** Downlight's logo, which opens the window menu as Alt+Space does (window design spec). */
function createLogo(): HTMLButtonElement {
  const button = barItem("logo", "Downlight");
  button.setAttribute("aria-label", "Downlight");
  button.setAttribute("aria-haspopup", "menu");
  // The SVG is the app's own icon, bundled with the page.
  button.innerHTML = logo;
  button.addEventListener("click", () => {
    showingWhy(askApp("show_window_menu"));
  });
  return button;
}

/**
 * The shown document's file name, with its full path as the tooltip, which reveals the file in
 * its folder. Only the name shows, so the bar stays short; the window's OS title keeps its folder
 * for Alt+Tab and the taskbar (v1 story 10).
 */
function createFileName(): {
  element: HTMLButtonElement;
  show(file: DocumentFile | null): void;
} {
  const element = barItem("file-name", "");
  // A child of its own, which the stylesheet cuts with an ellipsis before it pushes the buttons.
  const name = document.createElement("span");
  element.append(name);
  element.hidden = true;
  element.addEventListener("click", () => {
    // The app reveals the window's document, never a path from the page.
    showingWhy(askApp("reveal_document"));
  });
  return {
    element,
    show(file) {
      element.hidden = file === null;
      name.textContent = file?.name ?? "";
      element.title = file?.path ?? "";
    },
  };
}

/**
 * The zoom level beside the file name while it isn't 100%, so a reader notices a zoomed window
 * (window design ticket 04). It is the Reset zoom command, so a click resets the zoom and its
 * tooltip teaches Ctrl+0, but it sits with the logo and the name, and as they do, it never takes
 * the focus and isn't a part of the window to drag it by. `refresh` shows it as the settings and
 * the document view are now, whenever either may have changed.
 */
function createZoomLevel(): {
  element: HTMLButtonElement;
  refresh: () => void;
} {
  // No glyph: `refresh` writes the level as the text and into the name, as both change with it.
  const element = commandButton("resetZoom", "");
  element.classList.add(BAR_ITEM_CLASS, "zoom-level");
  element.tabIndex = -1;
  element.hidden = true;
  return {
    element,
    refresh: () => {
      const { zoom } = settingsNow();
      // Hidden at the level Reset gives, and in the empty window, which shows only the logo: there
      // is no document zoomed.
      element.hidden = zoom === DEFAULT_SETTINGS.zoom || !showsDocument();
      element.textContent = `${String(zoom)}%`;
      // The level the reader sees is part of its name (WCAG's label in name).
      element.setAttribute("aria-label", `${String(zoom)}%, Reset zoom`);
    },
  };
}

/**
 * The buttons of the reader's commands, in the spec's groups, each group followed by a thin
 * separator, the last one setting them apart from the window's own buttons (window design ticket
 * 05). Unlike the window's buttons they are reached with Tab, as a toolbar's are: they have no
 * system key such as Alt+Space, only shortcuts a reader may not know yet. A click still leaves the
 * focus in the document (the bar's `mousedown`).
 */
function createCommandButtons(): HTMLElement {
  const groups: [CommandId, string][][] = [
    [
      ["open", openGlyph],
      ["edit", editGlyph],
    ],
    [
      ["fullWidth", fullWidthGlyph],
      ["keepOnTop", keepOnTopGlyph],
      // The gear, which opens the settings flyout (settings-flyout.ts, window design ticket 08).
      ["settings", settingsGlyph],
    ],
  ];
  const row = document.createElement("div");
  row.className = "command-buttons";
  for (const group of groups) {
    for (const [id, glyph] of group) {
      const button = commandButton(id, glyph);
      button.classList.add(BAR_ITEM_CLASS, "command-button");
      row.append(button);
    }
    const separator = document.createElement("div");
    separator.className = "separator";
    // Only drawn: the buttons' names say what they do.
    separator.setAttribute("aria-hidden", "true");
    row.append(separator);
  }
  return row;
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
    button.classList.add(BAR_ITEM_CLASS, "window-button");
    // Never in the focus, as Windows' own buttons aren't, so the keys go on scrolling the document
    // after a click; the keyboard has Alt+Space for the window menu.
    button.tabIndex = -1;
  }
  const buttons = document.createElement("div");
  buttons.className = "window-buttons";
  buttons.append(minimize, maximize, close);
  const fileName = createFileName();
  const zoomLevel = createZoomLevel();
  bar.append(
    createLogo(),
    fileName.element,
    zoomLevel.element,
    createCommandButtons(),
    buttons,
  );
  document.body.prepend(bar);

  // The name follows whatever the window shows: a link, Open, a drop, and Back and Forward
  // (v1 ticket 12) all go through the document view.
  const stopNaming = whenDocumentShown(({ file }) => {
    fileName.show(file);
    zoomLevel.refresh();
  });
  signal?.addEventListener("abort", stopNaming);

  // The zoom level follows every change of the settings, from this window or another.
  const stopFollowingSettings = whenSettingsChange(zoomLevel.refresh);
  signal?.addEventListener("abort", stopFollowingSettings);

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
