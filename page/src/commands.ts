import { askApp, listenToApp, type MenuEntry } from "./app";
import { canCopyAsMarkdown, copyAsMarkdown } from "./copy-as-markdown";
import { edit } from "./edit";
import { isEditable } from "./editable";
import { findNext, findPrevious, isFindOpen, openFind } from "./find";
import { isFullScreen, switchFullScreen } from "./full-screen";
import { isFullWidth, switchFullWidth } from "./full-width";
import { isKeptOnTop, switchKeepOnTop } from "./keep-on-top";
import { showingWhy } from "./notice";
import { open } from "./open";
import { whenSettingsChange } from "./settings";
import { isSettingsFlyoutOpen, toggleSettingsFlyout } from "./settings-flyout";
import {
  canStepThroughHistory,
  canStop,
  requestDocument,
  showsDocument,
  stepThroughHistory,
  whenDocumentShown,
} from "./view";
import { changeZoom } from "./zoom";

/**
 * The reader's commands: each one's name, keyboard shortcut and what it does. A new command is
 * added to the `commands` table below, to the `menu` list beside it if it belongs in the
 * right-click menu, and to the title bar with `commandButton` if it has a button there, and
 * nowhere else in the page or the app. The menu entry, the button's tooltip and the shortcut then
 * all come from the one entry (window design spec). The title bar's logo and file name are not
 * commands: they have no shortcut or menu entry, and their name and tooltip come from the window
 * and its document (title-bar.ts). Its zoom level is the Reset zoom command, showing the level.
 */
interface Command {
  /** The command's name, as the right-click menu and its button's tooltip show it. */
  label: string;
  /**
   * The keys that run it. The page owns every key: the app turns WebView2's own browser keys off
   * (`leave_keys_to_the_page` in `app/src/platform/windows.rs`, ticket 29), so this table is the
   * one place that says what a key does. The menu and the button's tooltip show the first.
   */
  shortcuts?: Shortcut[];
  /**
   * Whether the command runs again on each repeat while its shortcut is held down. Off unless a
   * command asks for it: a held Ctrl+W would go on to close the next window that takes the focus,
   * and a held F5 would start a render per repeat.
   */
  repeatsWhileHeld?: boolean;
  /**
   * Whether its shortcut is left to an editable field that has the focus, such as the Find box,
   * so the field does with the key what it always does: Ctrl+V pastes into the field (paste-markdown
   * ticket 01). Off unless a command asks for it, so Ctrl+W still closes the window from the Find
   * box. Such a command isn't offered in the right-click menu over a field, or while one has the
   * focus, either: chosen there, the reader would take it for the field's own.
   */
  leavesEditableFields?: boolean;
  /**
   * The mouse button that runs it, by `MouseEvent.button`: 3 is the side button for Back and 4
   * the one for Forward. Any other button, and a command without one, does what it did.
   */
  mouseButton?: 3 | 4;
  /** Which way the mouse wheel runs it while Ctrl is held: up for Zoom in, down for Zoom out. */
  ctrlWheel?: "up" | "down";
  /**
   * Whether it can run now, which the right-click menu and its button show; always, for a command
   * without it. Its button reads it again whenever the window shows a document or the settings
   * change (`startCommands`), so a command whose state comes from elsewhere refreshes the buttons
   * itself, as Keep on top does with the app's answer.
   */
  available?(): boolean;
  /**
   * Whether a command that switches something on and off has it on now, such as Full width, which
   * its button shows in the accent color and the right-click menu with a check mark (window design
   * tickets 05 to 07). A command without it switches nothing. Its button reads it again as it
   * reads `available`.
   */
  pressed?(): boolean;
  run(): void;
}

/**
 * A key pressed with exactly the modifiers named, and no others. `key` is a letter in lower case,
 * which follows the keyboard's layout (`pressesKey`), a digit, or a character or named key as
 * `KeyboardEvent.key` gives it, such as `+`, `F5` or `ArrowLeft`.
 */
interface Shortcut {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

const commands = {
  back: {
    label: "Back",
    shortcuts: [{ alt: true, key: "ArrowLeft" }, { key: "BrowserBack" }],
    mouseButton: 3,
    available: () => canStepThroughHistory("back"),
    run: () => {
      stepThroughHistory("back");
    },
  },
  forward: {
    label: "Forward",
    shortcuts: [{ alt: true, key: "ArrowRight" }, { key: "BrowserForward" }],
    mouseButton: 4,
    available: () => canStepThroughHistory("forward"),
    run: () => {
      stepThroughHistory("forward");
    },
  },
  reload: {
    label: "Reload",
    shortcuts: [{ key: "F5" }, { ctrl: true, key: "r" }],
    // Asks for the document with the showing the view shows, which the app answers with a reload
    // (ticket 26); the view shows why if the file can't be read.
    run: () => {
      void requestDocument();
    },
  },
  find: {
    label: "Find",
    shortcuts: [{ ctrl: true, key: "f" }],
    run: openFind,
  },
  findNext: {
    label: "Find next",
    shortcuts: [{ key: "F3" }],
    repeatsWhileHeld: true,
    run: findNext,
  },
  findPrevious: {
    label: "Find previous",
    shortcuts: [{ shift: true, key: "F3" }],
    repeatsWhileHeld: true,
    run: findPrevious,
  },
  open: { label: "Open...", shortcuts: [{ ctrl: true, key: "o" }], run: open },
  // The app reads the clipboard itself, so a large paste never passes through the page, and
  // Win+V's clipboard history, which pastes with Ctrl+V, works too (paste-markdown spec).
  paste: {
    label: "Paste",
    shortcuts: [{ ctrl: true, key: "v" }],
    leavesEditableFields: true,
    run: () => {
      showingWhy(askApp("paste_clipboard"));
    },
  },
  // The app writes the clipboard itself, so a large copy never passes through the page
  // (copy-as-markdown spec); only a selection within one code block, whose Markdown is the text
  // selected, the page writes, as the copy buttons do. Its shortcut is left to a field, which
  // keeps its keys.
  copyAsMarkdown: {
    label: "Copy as Markdown",
    shortcuts: [{ ctrl: true, shift: true, key: "c" }],
    leavesEditableFields: true,
    available: canCopyAsMarkdown,
    run: copyAsMarkdown,
  },
  // Without a document the app has nothing to open; Ctrl+E still asks it, which says so.
  edit: {
    label: "Edit",
    shortcuts: [{ ctrl: true, key: "e" }],
    available: showsDocument,
    run: edit,
  },
  // Ctrl+plus as each layout types it: its own key (German, the number pad), Shift+= (US), and =
  // without Shift, as browsers take it too.
  zoomIn: {
    label: "Zoom in",
    shortcuts: [
      { ctrl: true, key: "+" },
      { ctrl: true, shift: true, key: "+" },
      { ctrl: true, key: "=" },
    ],
    ctrlWheel: "up",
    repeatsWhileHeld: true,
    run: () => {
      changeZoom("in");
    },
  },
  zoomOut: {
    label: "Zoom out",
    shortcuts: [{ ctrl: true, key: "-" }],
    ctrlWheel: "down",
    repeatsWhileHeld: true,
    run: () => {
      changeZoom("out");
    },
  },
  resetZoom: {
    label: "Reset zoom",
    shortcuts: [{ ctrl: true, key: "0" }],
    run: () => {
      changeZoom("reset");
    },
  },
  fullWidth: {
    label: "Full width",
    shortcuts: [{ ctrl: true, alt: true, key: "w" }],
    pressed: isFullWidth,
    run: switchFullWidth,
  },
  // The window's own state, which no message tells the page: its button shows the app's answer.
  keepOnTop: {
    label: "Keep on top",
    shortcuts: [{ ctrl: true, alt: true, key: "t" }],
    pressed: isKeptOnTop,
    run: () => {
      void switchKeepOnTop().then(refreshCommands);
    },
  },
  // The window's own state too, as Keep on top's, with no title bar button of its own (full
  // screen spec): in full screen the title bar's maximize button runs it (full screen ticket 05).
  fullScreen: {
    label: "Full screen",
    shortcuts: [{ key: "F11" }],
    pressed: isFullScreen,
    run: () => {
      void switchFullScreen().then(refreshCommands);
    },
  },
  // The gear's: opens the settings flyout under it, or closes it (window design ticket 08).
  settings: {
    label: "Settings",
    shortcuts: [{ ctrl: true, key: "," }],
    run: toggleSettingsFlyout,
  },
  close: {
    label: "Close",
    shortcuts: [{ ctrl: true, key: "w" }],
    run: () => {
      showingWhy(askApp("close_window"));
    },
  },
  minimize: {
    label: "Minimize",
    run: () => {
      showingWhy(askApp("minimize_window"));
    },
  },
  // Restores a maximized window; the title bar relabels its button while the window is maximized.
  maximize: {
    label: "Maximize",
    run: () => {
      showingWhy(askApp("toggle_maximize_window"));
    },
  },
} satisfies Record<string, Command>;

export type CommandId = keyof typeof commands;

/**
 * Runs the command `id`, for a button that runs one command or another by the window's state, as
 * the title bar's maximize button runs Full screen in full screen (full screen ticket 05). Every
 * other button is a `commandButton`.
 */
export function runCommand(id: CommandId): void {
  commands[id].run();
}

/** A line of the right-click menu: a command, a separator, or the system's Copy. */
type MenuLine = CommandId | "separator" | "copy";

/** The right-click menu, top to bottom. */
const menu: MenuLine[] = [
  "back",
  "forward",
  "reload",
  "find",
  "separator",
  "edit",
  "open",
  "paste",
  "separator",
  "zoomIn",
  "zoomOut",
  "resetZoom",
  "separator",
  "fullWidth",
  "keepOnTop",
  "fullScreen",
  "separator",
  "copy",
  // Directly after the system's Copy, which copies the text the reader sees (copy-as-markdown
  // spec).
  "copyAsMarkdown",
  // Last (window design spec); Choose editor... is in its Edit with now.
  "separator",
  "settings",
];

/** The menu as the app's `show_document_menu` takes it, right-clicked on `target`. */
function menuEntries(target: EventTarget | null): MenuEntry[] {
  const inField = isEditable(target) || isEditable(document.activeElement);
  return menu.map((line): MenuEntry => {
    if (line === "separator" || line === "copy") return { kind: line };
    const command: Command = commands[line];
    return {
      kind: "command",
      id: line,
      label: command.label,
      shortcut: shortcutOf(command),
      enabled:
        !(inField && command.leavesEditableFields) &&
        (command.available?.() ?? true),
      // Only a command that switches something has a check mark, ticked or not.
      ...(command.pressed && { checked: command.pressed() }),
    };
  });
}

/**
 * A button that runs the command `id`, showing `glyph`, one of the page's own bundled SVGs. A
 * screen reader names it by the command's label, and its tooltip adds the shortcut, as in
 * "Close (Ctrl+W)", so the button teaches the key. The label's ellipsis, which marks a menu entry
 * that asks for more before it acts ("Open..."), is the menu's alone, as in Windows' own tooltips.
 * It is disabled while its command isn't available, and a command that switches something on and
 * off makes a toggle button (`aria-pressed`), each as `refreshCommands` last found it.
 */
export function commandButton(id: CommandId, glyph: string): HTMLButtonElement {
  const command: Command = commands[id];
  const label = command.label.replace(/\.\.\.$/, "");
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.command = id;
  button.setAttribute("aria-label", label);
  button.title = tooltipOf(id, label);
  button.innerHTML = glyph;
  button.addEventListener("click", () => {
    command.run();
  });
  showState(button, command);
  return button;
}

/**
 * The tooltip of a button that runs the command `id` under the name `label`: the name and the
 * command's shortcut, as in "Close (Ctrl+W)", or the name alone for a command without one. Also
 * for a button that runs a command under another name, as the title bar's maximize button runs
 * Full screen as "Leave full screen (F11)" (full screen ticket 05).
 */
export function tooltipOf(id: CommandId, label: string): string {
  const command: Command = commands[id];
  const shortcut = shortcutOf(command);
  return shortcut ? `${label} (${shortcut})` : label;
}

/** Shows on `button` whether `command` is available, and whether it is on. */
function showState(button: HTMLButtonElement, command: Command): void {
  button.disabled = !(command.available?.() ?? true);
  if (command.pressed)
    button.setAttribute("aria-pressed", String(command.pressed()));
}

/**
 * Shows on every command button whether its command is available now, and whether it is on: run
 * whenever either may have changed, as when the window shows another document, the settings
 * change (`startCommands`) or the app answers that a window's own state switched (window design
 * ticket 05). The right-click menu reads them when it opens, so it needs no refresh.
 */
function refreshCommands(): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>(
    "button[data-command]",
  )) {
    const id = button.dataset.command ?? "";
    if (isCommandId(id)) showState(button, commands[id]);
  }
}

/** The named keys that Windows writes otherwise than `KeyboardEvent.key` names them. */
const WINDOWS_KEY_NAMES: Partial<Record<string, string>> = {
  ArrowLeft: "Left",
  ArrowRight: "Right",
  "+": "Plus",
  "-": "Minus",
};

/** The command's first shortcut as Windows writes it, such as `Ctrl+O` or `Alt+Left`. */
function shortcutOf(command: Command): string | undefined {
  const shortcut = command.shortcuts?.[0];
  if (!shortcut) return undefined;
  const names: string[] = [];
  if (shortcut.ctrl) names.push("Ctrl");
  if (shortcut.alt) names.push("Alt");
  if (shortcut.shift) names.push("Shift");
  const { key } = shortcut;
  names.push(
    isLetter(key) ? key.toUpperCase() : (WINDOWS_KEY_NAMES[key] ?? key),
  );
  return names.join("+");
}

function pressesShortcutOf(event: KeyboardEvent, command: Command): boolean {
  return (command.shortcuts ?? []).some(
    (shortcut) =>
      event.ctrlKey === (shortcut.ctrl ?? false) &&
      event.altKey === (shortcut.alt ?? false) &&
      event.shiftKey === (shortcut.shift ?? false) &&
      !event.metaKey &&
      pressesKey(event, shortcut.key),
  );
}

/**
 * Whether the key pressed is `key`. A letter goes by the a-z letter the key types, so it follows
 * AZERTY and the like; a key that types a letter outside the Latin script, as on Cyrillic, goes by
 * its place, as other Windows apps do. Any other key is never a letter's shortcut: Windows reports
 * AltGr as Ctrl+Alt, so AltGr+W, which types `|` on Czech and `å` on US-International or is a dead
 * key, would otherwise be Ctrl+Alt+W and lose what the reader typed (window design ticket 06). A
 * digit goes by the digit it types or by the digit key it is on, since AZERTY types `à` on the 0
 * key without Shift. A named key goes by its name.
 */
function pressesKey(event: KeyboardEvent, key: string): boolean {
  if (/^\d$/.test(key))
    return event.key === key || event.code === `Digit${key}`;
  if (!isLetter(key)) return event.key === key;
  if (/^[a-z]$/i.test(event.key)) return event.key.toLowerCase() === key;
  return (
    typesANonLatinLetter(event) && event.code === `Key${key.toUpperCase()}`
  );
}

/** Whether the key pressed types one letter of a script other than Latin, such as Cyrillic. */
function typesANonLatinLetter(event: KeyboardEvent): boolean {
  return /^\p{L}$/u.test(event.key) && !/^\p{Script=Latin}$/u.test(event.key);
}

function isLetter(key: string): boolean {
  return /^[a-z]$/.test(key);
}

/**
 * How far the wheel turns with Ctrl held, in px, to run Zoom in or Zoom out once: half a mouse
 * wheel's notch, so every notch zooms whatever the display's scale.
 */
const WHEEL_STEP = 50;

function isCommandId(id: string): id is CommandId {
  return Object.hasOwn(commands, id);
}

/**
 * Runs a command when its shortcut is pressed, its mouse button released or it is chosen in the
 * right-click menu, shows that menu on a right-click, and keeps the command buttons in step with
 * the document and the settings. After `startSettings`. The signal removes the listeners again,
 * for the tests.
 */
export function startCommands(signal?: AbortSignal): void {
  // Edit can run only with a document, and Full width is a setting: the table knows what its
  // commands' states follow, so the title bar needn't (review fixes of window design ticket 05).
  const stopFollowingDocument = whenDocumentShown(refreshCommands);
  signal?.addEventListener("abort", stopFollowingDocument);
  const stopFollowingSettings = whenSettingsChange(refreshCommands);
  signal?.addEventListener("abort", stopFollowingSettings);

  addEventListener(
    "keydown",
    (event) => {
      const command: Command | undefined = Object.values(commands).find(
        (candidate: Command) => pressesShortcutOf(event, candidate),
      );
      if (!command) return;
      if (command.leavesEditableFields && isEditable(event.target)) return;
      // A repeat is taken from the webview too, for webviews whose browser keys are still on: the
      // other systems' until their ports, and a window whose keys the app couldn't turn off.
      event.preventDefault();
      if (event.repeat && !command.repeatsWhileHeld) return;
      command.run();
    },
    { signal },
  );

  // Esc leaves full screen, last (full screen ticket 02). It isn't the command's shortcut, which
  // would take Esc from everything else: Esc closes what is nearest first, in the order the
  // settings flyout, Find, Stop, then full screen, and the coming image lightbox goes before full
  // screen too. Each feature's listener defers to the ones before it, whichever runs first, so
  // this one asks each of them rather than relying on `defaultPrevented` alone: the open flyout
  // hears Esc only with the focus inside it, and a click on it where nothing takes the focus
  // leaves it open with the focus outside. It runs the command, so what shows full screen's state
  // refreshes with the app's answer.
  addEventListener(
    "keydown",
    (event) => {
      if (event.key !== "Escape" || event.isComposing) return;
      if (event.ctrlKey || event.altKey || event.metaKey || event.shiftKey)
        return;
      if (event.defaultPrevented || !isFullScreen()) return;
      if (isSettingsFlyoutOpen() || isFindOpen() || canStop()) return;
      event.preventDefault();
      // The app switches rather than sets, so a repeat before its answer would switch back.
      if (event.repeat) return;
      commands.fullScreen.run();
    },
    { signal },
  );

  addEventListener(
    "mouseup",
    (event) => {
      const command: Command | undefined = Object.values(commands).find(
        (candidate: Command) => candidate.mouseButton === event.button,
      );
      if (!command) return;
      // Taken from the webview, so that it does nothing of its own with the button. On the
      // release rather than on `auxclick`, which comes only when the button is pressed and
      // released on the same element.
      event.preventDefault();
      command.run();
    },
    { signal },
  );

  /**
   * How far the wheel has turned with Ctrl held since its command last ran, in px, up below zero.
   * A mouse wheel turns 100 px or more a notch, which runs its command once, never more; a
   * touchpad's pinch, which the webview sends as Ctrl+wheel in many small steps, runs it once
   * every `WHEEL_STEP`.
   */
  let wheelTurned = 0;
  addEventListener(
    "wheel",
    (event) => {
      if (!event.ctrlKey || event.altKey || event.shiftKey || event.metaKey)
        return;
      // Ctrl+wheel never scrolls the document, and WebView2 has its own zoom off.
      event.preventDefault();
      if (event.deltaY === 0) return;
      // In lines or pages rather than px, a turn is at least a notch.
      const turned =
        event.deltaMode === 0
          ? event.deltaY
          : Math.sign(event.deltaY) * WHEEL_STEP;
      if (Math.sign(turned) !== Math.sign(wheelTurned)) wheelTurned = 0;
      wheelTurned += turned;
      if (Math.abs(wheelTurned) < WHEEL_STEP) return;
      const way = wheelTurned < 0 ? "up" : "down";
      wheelTurned = 0;
      const command: Command | undefined = Object.values(commands).find(
        (candidate: Command) => candidate.ctrlWheel === way,
      );
      command?.run();
    },
    { signal, passive: false },
  );

  addEventListener(
    "contextmenu",
    (event) => {
      // The app shows a native menu in place of the webview's own.
      event.preventDefault();
      showingWhy(
        askApp("show_document_menu", { entries: menuEntries(event.target) }),
      );
    },
    { signal },
  );

  listenToApp("menu-command", (id) => {
    if (isCommandId(id)) commands[id].run();
  });
}
