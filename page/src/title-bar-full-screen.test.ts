// The title bar in full screen (full screen tickets 03 to 05): hidden, with no room kept for it,
// sliding in while the reader reaches for it, and its maximize button leaving full screen.
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import readme from "../../core/tests/fixtures/readme.html?raw";
import { type AnswerOf, type ArgsOf, type DocumentToShow } from "./app";
import { startCommands } from "./commands";
import { startFullScreen } from "./full-screen";
import { startSettingsFlyout } from "./settings-flyout";
import {
  askedOfTheApp,
  closeTitleBarWindow,
  failingWith,
  layOut,
  linesOf,
  movePointer,
  nextInTurn,
  openPageWith,
  openWithTitleBar,
  settle,
  SHOWN_FILE,
  shownFrom,
  tell,
  titleBarButton,
  titleBarFileName,
  titleBarLogo,
  titleBarRoom,
  titleBarShows,
  titleBarTopZone,
} from "./test-support";
import { startTitleBar } from "./title-bar";
import { startDocumentView } from "./view";

/** Each `place_maximize_button` request's arguments, in turn. */
let placed: ArgsOf<"place_maximize_button">[];

/** The app's answers to `render_document`, in turn; the last one repeats. */
let answers: DocumentToShow[];

/** Lays the title bar out, as the webview does when the window's size or DPI changes. */
let layOutTitleBar: () => void;

/** Whether the fake app's window is full screen, which its `switch_full_screen` flips. */
let fullScreen: boolean;

/** How the app answers `switch_full_screen`. */
let switchFullScreen: () =>
  AnswerOf<"switch_full_screen"> | Promise<AnswerOf<"switch_full_screen">>;

let stop: AbortController;

beforeEach(async () => {
  answers = [];
  fullScreen = false;
  switchFullScreen = () => (fullScreen = !fullScreen);
  ({ placed, layOut: layOutTitleBar } = await openWithTitleBar({
    switch_full_screen: () => switchFullScreen(),
    render_document: () => nextInTurn(answers) ?? null,
  }));
  stop = new AbortController();
  startFullScreen(stop.signal);
});

afterEach(() => {
  stop.abort();
  closeTitleBarWindow();
});

function bar(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".title-bar");
  if (!found) throw new Error("The page draws no title bar.");
  return found;
}

function pressF11(): void {
  document.body.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "F11",
      code: "F11",
      bubbles: true,
      cancelable: true,
    }),
  );
}

/** How often the page asked the app to switch full screen. */
function fullScreenSwitches(): number {
  return askedOfTheApp().filter((command) => command === "switch_full_screen")
    .length;
}

/** How often the page asked the app for its right-click menu. */
function menusShown(): number {
  return askedOfTheApp().filter((command) => command === "show_document_menu")
    .length;
}

/** Starts the title bar and the commands, whose F11 switches full screen. */
function startWithFullScreen(): void {
  startTitleBar(stop.signal);
  startCommands(stop.signal);
}

/** Starts the title bar and the commands, and goes full screen, where the bar is hidden. */
async function startInFullScreen(): Promise<void> {
  startWithFullScreen();
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
}

/** Right-clicks `target`, as the webview tells the page, and says whether the page took it. */
function rightClick(target: Element): boolean {
  const click = new MouseEvent("contextmenu", {
    bubbles: true,
    cancelable: true,
  });
  target.dispatchEvent(click);
  return click.defaultPrevented;
}

// Full screen (full screen ticket 03): the bar is hidden and the page keeps no room for it, as the
// app last answered.

test("a new window's title bar shows, with its room kept", () => {
  startWithFullScreen();
  expect(titleBarShows()).toBe(true);
  expect(titleBarRoom()).toBeUndefined();
});

test("in full screen the title bar is hidden and the page keeps no room for it", async () => {
  startWithFullScreen();
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  expect(titleBarRoom()).toBe("0px");
});

test("leaving full screen shows the title bar again and keeps its room again", async () => {
  startWithFullScreen();
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(true);
  });
  expect(titleBarRoom()).toBeUndefined();
});

test("the title bar shows the full screen the app answers with, not the one the page expected", async () => {
  // The window was full screen all along, as far as the app knows.
  switchFullScreen = () => true;
  startWithFullScreen();
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  pressF11();
  await vi.waitFor(() => {
    expect(fullScreenSwitches()).toBe(2);
  });
  await settle();
  expect(titleBarShows()).toBe(false);
  expect(titleBarRoom()).toBe("0px");
});

test("a full screen the app couldn't switch leaves the title bar and its room as they were", async () => {
  const why = "Full screen didn't switch: the window is gone.";
  switchFullScreen = () => failingWith(why);
  startWithFullScreen();
  pressF11();
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toContain(why);
  });
  expect(titleBarShows()).toBe(true);
  expect(titleBarRoom()).toBeUndefined();
});

test("going full screen and leaving keep the reading position, with the room for the bar already changed", async () => {
  answers = [shownFrom(SHOWN_FILE, { rendered: readme })];
  startTitleBar(stop.signal);
  await startDocumentView(0);
  startCommands(stop.signal);
  await settle();
  // Halfway down "## Options", on line 15.
  layOut(15.5);
  const scrolls: { line: number; room: string | undefined }[] = [];
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(function (
    this: Element,
  ) {
    scrolls.push({ line: linesOf(this)?.[0] ?? 0, room: titleBarRoom() });
  });

  pressF11();
  await vi.waitFor(() => {
    expect(scrolls.length).toBeGreaterThan(0);
  });
  await settle();
  for (const scroll of scrolls)
    expect(scroll).toEqual({ line: 15, room: "0px" });

  scrolls.length = 0;
  pressF11();
  await vi.waitFor(() => {
    expect(scrolls.length).toBeGreaterThan(0);
  });
  await settle();
  for (const scroll of scrolls)
    expect(scroll).toEqual({ line: 15, room: undefined });
});

test("without the page's title bar, full screen leaves the room at the top as it is", async () => {
  openPageWith();
  startWithFullScreen();
  pressF11();
  await vi.waitFor(() => {
    expect(fullScreenSwitches()).toBe(1);
  });
  await settle();
  expect(fullScreen).toBe(true);
  expect(titleBarRoom()).toBeUndefined();
});

test("a title bar stopped in full screen leaves the room at the top to the stylesheet", async () => {
  await startInFullScreen();
  expect(titleBarRoom()).toBe("0px");
  stop.abort();
  expect(titleBarRoom()).toBeUndefined();
});

// Full screen (full screen ticket 04): the hidden bar slides in while the pointer is at the top
// edge of the screen or over it, the focus is in it or the settings flyout is open.

test("in full screen the pointer at the top edge shows the title bar, and leaving the zone and the bar hides it", async () => {
  await startInFullScreen();
  const zone = titleBarTopZone();
  const name = titleBarFileName();

  movePointer(null, zone);
  expect(titleBarShows()).toBe(true);
  // Shown, the bar is under the pointer: moving onto it keeps it.
  movePointer(zone, name);
  expect(titleBarShows()).toBe(true);
  movePointer(name, null);
  expect(titleBarShows()).toBe(false);
});

test("in full screen the focus moving into the title bar shows it, and moving out hides it", async () => {
  await startInFullScreen();
  const open = titleBarButton("Open");
  const fullWidth = titleBarButton("Full width");

  // As Tab moves it there, and on along the bar.
  open.focus();
  expect(titleBarShows()).toBe(true);
  fullWidth.focus();
  expect(titleBarShows()).toBe(true);
  fullWidth.blur();
  expect(titleBarShows()).toBe(false);
});

test("in full screen the open settings flyout keeps the title bar, and closing it with the pointer and the focus elsewhere hides it", async () => {
  await startInFullScreen();
  startSettingsFlyout(stop.signal);
  const gear = titleBarButton("Settings");
  const flyout = document.querySelector<HTMLElement>(".settings-flyout");
  if (!flyout) throw new Error("The page draws no settings flyout.");

  // From the top edge to the gear, a click, and on into the flyout, which hangs from the bar.
  movePointer(null, titleBarTopZone());
  movePointer(titleBarTopZone(), gear);
  gear.click();
  expect(flyout.hidden).toBe(false);
  movePointer(gear, flyout);
  expect(titleBarShows()).toBe(true);

  // A click on the document closes the flyout.
  movePointer(flyout, document.body);
  document.body.dispatchEvent(
    new PointerEvent("pointerdown", { bubbles: true }),
  );
  expect(flyout.hidden).toBe(true);
  expect(titleBarShows()).toBe(false);
});

test("in full screen Ctrl+, opens the settings flyout with the title bar in view, and Esc leaves the bar to the focus on its gear", async () => {
  await startInFullScreen();
  startSettingsFlyout(stop.signal);

  document.body.dispatchEvent(
    new KeyboardEvent("keydown", {
      ctrlKey: true,
      key: ",",
      code: "Comma",
      bubbles: true,
      cancelable: true,
    }),
  );
  expect(titleBarShows()).toBe(true);

  // Esc closes the flyout and gives the focus back to the gear, which the reader has to see.
  document.activeElement?.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Escape",
      code: "Escape",
      bubbles: true,
      cancelable: true,
    }),
  );
  expect(document.activeElement).toBe(titleBarButton("Settings"));
  expect(titleBarShows()).toBe(true);
  titleBarButton("Settings").blur();
  expect(titleBarShows()).toBe(false);
});

test("in full screen the title bar slides in over the document: the page keeps no room for it while it shows", async () => {
  await startInFullScreen();

  movePointer(null, titleBarTopZone());
  expect(titleBarShows()).toBe(true);
  expect(titleBarRoom()).toBe("0px");
  titleBarButton("Open").focus();
  expect(titleBarRoom()).toBe("0px");
});

test("out of full screen the title bar shows whatever the pointer and the focus do", async () => {
  startWithFullScreen();
  const open = titleBarButton("Open");

  movePointer(null, open);
  open.focus();
  movePointer(open, null);
  open.blur();
  expect(titleBarShows()).toBe(true);

  // Left with the bar in view under the pointer, and then the pointer leaves it.
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  movePointer(null, titleBarTopZone());
  pressF11();
  await vi.waitFor(() => {
    expect(fullScreenSwitches()).toBe(2);
  });
  await settle();
  movePointer(titleBarTopZone(), null);
  expect(titleBarShows()).toBe(true);
  expect(titleBarRoom()).toBeUndefined();
});

// Full screen (full screen ticket 05): the maximize button leaves full screen, and a right-click
// on the strip opens nothing. The strip being no drag region is `app-region` in title-bar.css,
// which happy-dom doesn't compute: ticket 05's hand check covers it.

test("in full screen the maximize button is Leave full screen, with the restore glyph and a tooltip naming F11", async () => {
  startWithFullScreen();
  await settle();
  await tell("window-state", "maximized");
  const restoreGlyph = titleBarButton("Restore").innerHTML;
  await tell("window-state", "restored");

  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  const leave = titleBarButton("Leave full screen");
  expect(leave.title).toBe("Leave full screen (F11)");
  expect(leave.innerHTML).toBe(restoreGlyph);
});

test("a click on Leave full screen asks the app to switch full screen, not to maximize", async () => {
  await startInFullScreen();
  titleBarButton("Leave full screen").click();
  await vi.waitFor(() => {
    expect(fullScreenSwitches()).toBe(2);
  });
  expect(askedOfTheApp()).not.toContain("toggle_maximize_window");
  await vi.waitFor(() => {
    expect(titleBarButton("Maximize").title).toBe("Maximize");
  });
});

test("in full screen the window's maximized state leaves the button Leave full screen", async () => {
  await startInFullScreen();
  // The app tells a maximized window's state on every resize, full screen's included.
  await tell("window-state", "maximized");
  expect(titleBarButton("Leave full screen").title).toBe(
    "Leave full screen (F11)",
  );
});

test("leaving full screen brings back restore for a maximized window, and maximize for any other", async () => {
  startWithFullScreen();
  await settle();
  await tell("window-state", "maximized");
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarButton("Restore").title).toBe("Restore");
  });

  await tell("window-state", "restored");
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(false);
  });
  pressF11();
  await vi.waitFor(() => {
    expect(titleBarButton("Maximize").title).toBe("Maximize");
  });
});

test("leaving full screen tells the app where the maximize button is once the bar is back, though the window's resize laid it out while away", async () => {
  await startInFullScreen();
  // Its place moves with the bar's slide, as `getBoundingClientRect` gives it.
  vi.spyOn(
    titleBarButton("Leave full screen"),
    "getBoundingClientRect",
  ).mockImplementation(
    () => new DOMRect(908, titleBarShows() ? 0 : -32, 46, 32),
  );
  // The app sizes the window back before it answers, while the bar is still away.
  switchFullScreen = () => {
    layOutTitleBar();
    return (fullScreen = !fullScreen);
  };

  pressF11();
  await vi.waitFor(() => {
    expect(titleBarShows()).toBe(true);
  });
  await settle();
  expect(placed.at(-1)).toEqual({
    button: { x: 908, y: 0, width: 46, height: 32 },
  });
});

test("in full screen a right-click on the strip opens no menu", async () => {
  await startInFullScreen();
  movePointer(null, titleBarTopZone());

  // Taken, so the webview shows no menu of its own either.
  expect(rightClick(bar())).toBe(true);
  expect(rightClick(titleBarTopZone())).toBe(true);
  await settle();
  expect(menusShown()).toBe(0);
});

test("in full screen a right-click on the bar's items opens the page's menu, as out of full screen", async () => {
  await startInFullScreen();
  movePointer(null, titleBarTopZone());

  for (const item of [
    titleBarLogo(),
    titleBarButton("Open"),
    titleBarButton("Leave full screen"),
  ])
    expect(rightClick(item), item.className).toBe(true);
  await settle();
  expect(menusShown()).toBe(3);
});

test("out of full screen the title bar leaves a right-click on its strip alone", async () => {
  startWithFullScreen();
  // WebView2 opens the window menu on the drag region itself; one that reaches the page goes on
  // to its own menu, as anywhere else.
  rightClick(bar());
  await settle();
  expect(menusShown()).toBe(1);
});
