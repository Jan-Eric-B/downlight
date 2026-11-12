import { invoke } from "@tauri-apps/api/core";
import { listenToApp } from "./events";
import { findNext, findPrevious, openFind } from "./find";
import { showNotice } from "./notice";
import { open } from "./open";

/**
 * The reader's commands: each one's keyboard shortcut and what it does. A new command is added
 * to the `commands` table below, and to the `menu` list beside it if it belongs in the right-click
 * menu, and nowhere else in the page or the app.
 */
interface Command {
  /** The letter pressed with Ctrl, in lower case. */
  ctrlLetter?: string;
  /**
   * A named key, such as `F5`, pressed without Ctrl, Alt or Meta, and with Shift only if `shift`
   * says so. The menu shows it before Ctrl+letter.
   */
  key?: string;
  /** Whether `key` is pressed with Shift. */
  shift?: boolean;
  run(): void;
}

const commands = {
  reload: {
    ctrlLetter: "r",
    key: "F5",
    // The app tells the page to reload, and the page shows why if the file can't be read.
    run: () => void invoke("reload_document"),
  },
  find: { ctrlLetter: "f", run: openFind },
  findNext: { key: "F3", run: findNext },
  findPrevious: { key: "F3", shift: true, run: findPrevious },
  open: { ctrlLetter: "o", run: () => void open() },
  close: {
    ctrlLetter: "w",
    // Closing the window can't fail in a way the reader could act on: the window stays open.
    run: () => void invoke("close_window"),
  },
} satisfies Record<string, Command>;

type CommandId = keyof typeof commands;

/**
 * A line of the right-click menu: a command with its text in the menu, a separator, or the
 * system's Copy. The text sits on the line, so a command can't reach the menu without one.
 */
type MenuLine = { command: CommandId; label: string } | "separator" | "copy";

/** The right-click menu, top to bottom. */
const menu: MenuLine[] = [
  { command: "reload", label: "Reload" },
  { command: "find", label: "Find" },
  "separator",
  { command: "open", label: "Open..." },
  "separator",
  "copy",
];

/** The menu as the app's `show_document_menu` takes it. */
function menuEntries() {
  return menu.map((line) => {
    if (line === "separator" || line === "copy") return { kind: line };
    return {
      kind: "command",
      id: line.command,
      label: line.label,
      shortcut: shortcutOf(commands[line.command]),
    };
  });
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
      const command = Object.values(commands).find((candidate: Command) =>
        pressesShortcutOf(event, candidate),
      );
      if (!command) return;
      event.preventDefault();
      command.run();
    },
    { signal },
  );

  addEventListener(
    "contextmenu",
    (event) => {
      // The app shows a native menu in place of the webview's own.
      event.preventDefault();
      invoke("show_document_menu", { entries: menuEntries() }).catch(
        (error: unknown) => {
          showNotice(String(error));
        },
      );
    },
    { signal },
  );

  listenToApp("menu-command", (id) => {
    if (isCommandId(id)) commands[id].run();
  });
}
