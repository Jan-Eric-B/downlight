import { askApp, listenToApp, type MenuEntry } from "./app";
import { findNext, findPrevious, openFind } from "./find";
import { showNotice } from "./notice";
import { open } from "./open";

/**
 * The reader's commands: each one's name, keyboard shortcut and what it does. A new command is
 * added to the `commands` table below, to the `menu` list beside it if it belongs in the
 * right-click menu, and to the title bar with `commandButton` if it has a button there, and
 * nowhere else in the page or the app. The menu entry, the button's tooltip and the shortcut then
 * all come from the one entry (window design spec).
 */
interface Command {
  /** The command's name, as the right-click menu and its button's tooltip show it. */
  label: string;
  /** The letter pressed with Ctrl, in lower case. */
  ctrlLetter?: string;
  /**
   * A named key, such as `F5`, pressed without Ctrl, Alt or Meta, and with Shift only if `shift`
   * says so. The menu shows it before Ctrl+letter.
   */
  key?: string;
  /** Whether `key` is pressed with Shift. */
  shift?: boolean;
  /**
   * Whether the command runs again on each repeat while its shortcut is held down. Off unless a
   * command asks for it: a held Ctrl+W would go on to close the next window that takes the focus,
   * and a held F5 would start a render per repeat.
   */
  repeatsWhileHeld?: boolean;
  run(): void;
}

/** Shows the reader why, if the app couldn't do what it was `asked`. */
function showingWhy(asked: Promise<unknown>): void {
  asked.catch((error: unknown) => {
    showNotice(String(error));
  });
}

const commands = {
  reload: {
    label: "Reload",
    ctrlLetter: "r",
    key: "F5",
    // The app tells the page to reload, and the page shows why if the file can't be read.
    run: () => {
      showingWhy(askApp("reload_document"));
    },
  },
  find: { label: "Find", ctrlLetter: "f", run: openFind },
  findNext: {
    label: "Find next",
    key: "F3",
    repeatsWhileHeld: true,
    run: findNext,
  },
  findPrevious: {
    label: "Find previous",
    key: "F3",
    shift: true,
    repeatsWhileHeld: true,
    run: findPrevious,
  },
  open: { label: "Open...", ctrlLetter: "o", run: () => void open() },
  close: {
    label: "Close",
    ctrlLetter: "w",
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

type CommandId = keyof typeof commands;

/** A line of the right-click menu: a command, a separator, or the system's Copy. */
type MenuLine = CommandId | "separator" | "copy";

/** The right-click menu, top to bottom. */
const menu: MenuLine[] = [
  "reload",
  "find",
  "separator",
  "open",
  "separator",
  "copy",
];

/** The menu as the app's `show_document_menu` takes it. */
function menuEntries(): MenuEntry[] {
  return menu.map((line): MenuEntry => {
    if (line === "separator" || line === "copy") return { kind: line };
    const command: Command = commands[line];
    return {
      kind: "command",
      id: line,
      label: command.label,
      shortcut: shortcutOf(command),
    };
  });
}

/**
 * A button that runs the command `id`, showing `glyph`, one of the page's own bundled SVGs. A
 * screen reader names it by the command's label, and its tooltip adds the shortcut, as in
 * "Close (Ctrl+W)", so the button teaches the key.
 */
export function commandButton(id: CommandId, glyph: string): HTMLButtonElement {
  const command: Command = commands[id];
  const shortcut = shortcutOf(command);
  const button = document.createElement("button");
  button.type = "button";
  button.setAttribute("aria-label", command.label);
  button.title = shortcut ? `${command.label} (${shortcut})` : command.label;
  button.innerHTML = glyph;
  button.addEventListener("click", () => {
    command.run();
  });
  return button;
}

function shortcutOf(command: Command): string | undefined {
  if (command.key) return command.shift ? `Shift+${command.key}` : command.key;
  return command.ctrlLetter && `Ctrl+${command.ctrlLetter.toUpperCase()}`;
}

function pressesShortcutOf(event: KeyboardEvent, command: Command): boolean {
  if (
    command.ctrlLetter !== undefined &&
    pressesCtrlLetter(event, command.ctrlLetter)
  ) {
    return true;
  }
  if (event.ctrlKey || event.altKey || event.metaKey) return false;
  return (
    command.key !== undefined &&
    event.key === command.key &&
    event.shiftKey === (command.shift ?? false)
  );
}

/**
 * The webview's own reloads (Ctrl+F5, Shift+F5, Ctrl+Shift+R), which would load the page anew
 * and lose the reading position. On Windows the app already turns them off in WebView2
 * (`leave_reload_and_find_keys_to_the_page` in `app/src/platform/windows.rs`), so this check
 * matters only if the app couldn't add that handler (it carries on without it), and on other
 * systems until their ports.
 */
function isWebviewReload(event: KeyboardEvent): boolean {
  if (event.altKey || event.metaKey) return false;
  const f5 = event.key === "F5" && (event.ctrlKey || event.shiftKey);
  const ctrlShiftR = event.ctrlKey && event.shiftKey && event.code === "KeyR";
  return f5 || ctrlShiftR;
}

/**
 * Whether the key pressed is Ctrl and `letter`. On a layout that types Latin letters it goes by
 * the letter the key types, so it follows AZERTY and the like; on other layouts, such as
 * Cyrillic, by the key's place, as other Windows apps do.
 */
function pressesCtrlLetter(event: KeyboardEvent, letter: string): boolean {
  if (!event.ctrlKey || event.altKey || event.shiftKey || event.metaKey) {
    return false;
  }
  const isLatinLetter = /^[a-z]$/i.test(event.key);
  return isLatinLetter
    ? event.key.toLowerCase() === letter
    : event.code === `Key${letter.toUpperCase()}`;
}

function isCommandId(id: string): id is CommandId {
  return Object.hasOwn(commands, id);
}

/**
 * Runs a command when its shortcut is pressed or it is chosen in the right-click menu, and shows
 * that menu on a right-click. The signal removes the key and right-click listeners again, for the
 * tests.
 */
export function startCommands(signal?: AbortSignal): void {
  addEventListener(
    "keydown",
    (event) => {
      if (isWebviewReload(event)) {
        event.preventDefault();
        return;
      }
      const command: Command | undefined = Object.values(commands).find(
        (candidate: Command) => pressesShortcutOf(event, candidate),
      );
      if (!command) return;
      // A repeat is still taken from the webview, so a held F5 never reloads the page itself.
      event.preventDefault();
      if (event.repeat && !command.repeatsWhileHeld) return;
      command.run();
    },
    { signal },
  );

  addEventListener(
    "contextmenu",
    (event) => {
      // The app shows a native menu in place of the webview's own.
      event.preventDefault();
      showingWhy(askApp("show_document_menu", { entries: menuEntries() }));
    },
    { signal },
  );

  listenToApp("menu-command", (id) => {
    if (isCommandId(id)) commands[id].run();
  });
}
