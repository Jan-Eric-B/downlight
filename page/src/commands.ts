import { invoke } from "@tauri-apps/api/core";
import { listenToApp } from "./events";
import { showNotice } from "./notice";

/**
 * The reader's commands: each one's keyboard shortcut and what it does. A new command is added
 * to the `commands` table below, and to the `menu` list beside it if it belongs in the right-click
 * menu, and nowhere else in the page or the app.
 */
interface Command {
  /** The letter pressed with Ctrl, in lower case. */
  ctrlLetter: string;
  run(): void;
}

let opening = false;

/** Open: the open dialog, then the chosen document in this window. */
export async function open(): Promise<void> {
  // A second Ctrl+O before the dialog is up would open a second dialog.
  if (opening) return;
  opening = true;
  try {
    await invoke("open_document");
  } catch (error) {
    showNotice(String(error));
  } finally {
    opening = false;
  }
}

const commands = {
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
  { command: "open", label: "Open..." },
  "separator",
  "copy",
];

/** The menu as the app's `show_document_menu` takes it. */
function menuEntries() {
  return menu.map((line) => {
    if (line === "separator" || line === "copy") return { kind: line };
    const { ctrlLetter } = commands[line.command];
    return {
      kind: "command",
      id: line.command,
      label: line.label,
      shortcut: `Ctrl+${ctrlLetter.toUpperCase()}`,
    };
  });
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
      const command = Object.values(commands).find((candidate) =>
        pressesCtrlLetter(event, candidate.ctrlLetter),
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
