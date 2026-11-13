// What the page and the app say to each other (ticket 27), the page's counterpart to
// `app/src/message.rs`: every command the page asks the app for, with its arguments and answer,
// and every message the app sends, with its payload. Tauri's IPC is the adapter here; the page's
// tests answer through the same types (`fakeTheApp` in `test-support.ts`), and read the samples
// the app's tests write (`app/src/contract.rs`), so the two sides can't drift apart. No other page
// module calls `invoke` or `listen`.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { showNotice } from "./notice";

/**
 * What the page does with a render, as the app decides it from the showing the page's view shows
 * (`Place` in `app/src/documents.rs`): show the document at the heading a link like
 * `guide.md#install` named, or at its top for null, or reload it at the reading position.
 */
export type Place =
  { place: "show"; heading: string | null } | { place: "reload" };

/**
 * The app's answer when the page asks for its document (`DocumentToShow` in
 * `app/src/documents.rs`), or null while the window shows no document: the showing the window is
 * at, for the next request, what to do with the render, and the rendered document as its HTML or
 * why it can't be read.
 */
export type DocumentToShow =
  | ({ showing: number } & Place & ({ document: string } | { error: string }))
  | null;

/** What `follow_link` leaves for the page to do (`Followed` in `app/src/links.rs`). */
export type Followed =
  { kind: "scrollTo"; heading: string | null } | { kind: "nothing" };

/**
 * A line of the right-click menu as `show_document_menu` takes it (`MenuEntry` in
 * `app/src/menu.rs`): a command the page runs when it is chosen, a separator, or the system's
 * Copy.
 */
export type MenuEntry =
  | { kind: "command"; id: string; label: string; shortcut?: string }
  | { kind: "separator" }
  | { kind: "copy" };

/** The marks on the way to first paint, each in ms since the Unix epoch (`app/src/timing.rs`). */
export type FirstPaintMarks = {
  navigationStart: number;
  scriptStart: number;
  documentRequested: number;
  documentReceived: number;
  documentInserted: number;
  /** When the frame after the document was inserted was done. */
  firstFrame: number;
};

/** The marks while a reload is shown, each in ms since the Unix epoch (`app/src/timing.rs`). */
export type ReloadMarks = {
  /** When the reloaded document's HTML arrived. */
  received: number;
  inserted: number;
  /** When the view was back at the reading position. */
  anchored: number;
};

/** A page task or frame that took over 100 ms (`LongTask` in `app/src/timing.rs`). */
export type LongTask = {
  kind: "task" | "frame";
  /** In ms since the Unix epoch. */
  start: number;
  /** In ms. */
  duration: number;
};

/**
 * Every command the app has (`generate_handler!` in `app/src/main.rs`), with the arguments the
 * page passes and the app's answer; a command whose Rust function returns `()` answers null.
 */
export type Commands = {
  render_document: {
    /** The showing whose render the view shows, or null while it shows no render. */
    args: { showing: number | null };
    answer: DocumentToShow;
  };
  /** Until ticket 26 removes it. */
  reload_document: { args: undefined; answer: null };
  follow_link: {
    /** The link target as the document wrote it; Ctrl+click opens a new window. */
    args: { target: string; newWindow: boolean };
    answer: Followed;
  };
  open_document: { args: undefined; answer: null };
  show_document_menu: { args: { entries: MenuEntry[] }; answer: null };
  close_window: { args: undefined; answer: null };
  show_window: { args: undefined; answer: null };
  report_first_paint: { args: { marks: FirstPaintMarks }; answer: null };
  report_reload: { args: { marks: ReloadMarks }; answer: null };
  report_long_task: { args: { task: LongTask }; answer: null };
};

export type Command = keyof Commands;
export type ArgsOf<C extends Command> = Commands[C]["args"];
export type AnswerOf<C extends Command> = Commands[C]["answer"];

/** A command's arguments as `askApp` takes them: none for a command that takes none. */
type ArgumentList<C extends Command> =
  ArgsOf<C> extends undefined ? [] : [args: ArgsOf<C>];

/**
 * Asks the app for `command` and resolves with its answer. Rejects with why, as Tauri gives it,
 * when the command fails or the app couldn't be asked.
 */
export function askApp<C extends Command>(
  command: C,
  ...args: ArgumentList<C>
): Promise<AnswerOf<C>> {
  const [given] = args;
  return invoke<AnswerOf<C>>(command, given);
}

/**
 * Every message the app sends a window's page (`Message` in `app/src/message.rs`), by the event
 * it goes as, with its payload.
 */
export type Messages = {
  /** The window shows another document now; the page asks for it. */
  "document-shown": null;
  /** The document's file changed: its new render is ready to ask for, or the file is gone. */
  "document-changed": "reload" | "gone";
  /** The heading a link in another window named, to scroll to. */
  "scroll-to-heading": string;
  /** A notice to show the reader. */
  notice: string;
  /** The id of the command the reader chose in the right-click menu. */
  "menu-command": string;
};

export type Message = keyof Messages;

/**
 * Runs `handler` on each `message` the app sends this window. The app sends each such event to
 * one window, and Tauri delivers it only to listeners that name that window, never to a plain
 * `listen`.
 */
export function listenToApp<M extends Message>(
  message: M,
  handler: (payload: Messages[M]) => void,
): void {
  getCurrentWebviewWindow()
    .listen<Messages[M]>(message, ({ payload }) => {
      handler(payload);
    })
    .catch((error: unknown) => {
      // Without the listener the window would miss what the app tells it.
      showNotice(String(error));
    });
}
