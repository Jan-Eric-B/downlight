// What the page and the app say to each other (ticket 27), the page's counterpart to
// `app/src/message.rs`: every command the page asks the app for, with its arguments and answer,
// every message the app sends, with its payload, and the flags it opens the page with in its URL.
// Tauri's IPC is the adapter here; the page's tests answer through the same types (`fakeTheApp`
// in `test-support.ts`), and read the samples the app's tests write (`app/src/contract.rs`), so
// the two sides can't drift apart. No other page module calls `invoke` or `listen`, or reads the
// page's URL.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { showNotice } from "./notice";

/**
 * What the page does with a render, as the app decides it from the showing the page's view shows
 * (`Place` in `app/src/documents.rs`): show the document at the heading a link like
 * `guide.md#install` named, or at its top for null, reload it at the reading position, or return
 * to where the view was when it left this showing, after Back or Forward (ticket 12).
 */
export type Place =
  | { place: "show"; heading: string | null }
  | { place: "reload" }
  | { place: "return" };

/** A step through the window's history (`Step` in `app/src/documents/history.rs`). */
export type Step = "back" | "forward";

/**
 * Whether the window has a document to go back to, and one to go forward to (`Steps` in
 * `app/src/documents/history.rs`).
 */
export type Steps = Record<Step, boolean>;

/**
 * The file of the document a window shows, as the title bar shows it (`DocumentFile` in
 * `app/src/documents.rs`): its name, and its full path for the name's tooltip.
 */
export type DocumentFile = { name: string; path: string };

/**
 * What the wait note names while the window's document is on its way (`WaitNote` in
 * `app/src/documents/handover.rs`, ticket 31): the document's file, and its size as the reader
 * is told it, such as "100 MB", or null when it can't be told.
 */
export type WaitNote = { file: DocumentFile; size: string | null };

/**
 * A file or folder a notice names (`NamedFile` in `app/src/notice.rs`, window design ticket 11):
 * its name, shown in bold, and its folder after it, or null for a drive's root. The folder opens
 * from the notice (`open_named_folder`) when `folderOpens`: the file is missing and its folder is
 * there.
 */
export type NamedFile = {
  name: string;
  folder: string | null;
  folderOpens: boolean;
};

/**
 * What the reader is told when something fails (`Notice` in `app/src/notice.rs`): plain text, or
 * text before and after the file it names. Every command that fails rejects with one, and a
 * document that can't be read says why with one.
 */
export type Notice =
  string | { before: string; file: NamedFile; after: string };

function isNamedFile(value: unknown): value is NamedFile {
  return (
    typeof value === "object" &&
    value !== null &&
    "name" in value &&
    typeof value.name === "string" &&
    "folder" in value &&
    (value.folder === null || typeof value.folder === "string") &&
    "folderOpens" in value &&
    typeof value.folderOpens === "boolean"
  );
}

/**
 * Whether a command's rejection is a notice in the app's shape, rather than something else Tauri
 * rejected with when the app couldn't be asked. It comes untyped, so the shape is checked, not
 * assumed.
 */
export function isNotice(value: unknown): value is Notice {
  if (typeof value === "string") return true;
  return (
    typeof value === "object" &&
    value !== null &&
    "before" in value &&
    typeof value.before === "string" &&
    "file" in value &&
    isNamedFile(value.file) &&
    "after" in value &&
    typeof value.after === "string"
  );
}

/**
 * The app's answer when the page asks for its document (`DocumentToShow` in
 * `app/src/documents.rs`), or null while the window shows no document: the showing the window is
 * at, for the next request, the document's file, what to do with the render, the rendered
 * document as its HTML or why it can't be read, a notice the app held until the page first
 * asked, or null, and whether the window has a document to go back or forward to.
 */
export type DocumentToShow =
  | ({
      showing: number;
      file: DocumentFile;
      notice: Notice | null;
      steps: Steps;
    } & Place &
      ({ document: string } | { error: Notice }))
  | null;

/** What `follow_link` leaves for the page to do (`Followed` in `app/src/links.rs`). */
export type Followed =
  { kind: "scrollTo"; heading: string | null } | { kind: "nothing" };

/**
 * A line of the right-click menu as `show_document_menu` takes it (`MenuEntry` in
 * `app/src/menu.rs`): a command the page runs when it is chosen, whether it can run now, and for
 * one that switches something on and off whether it is on, shown as a check mark, a
 * separator, or the system's Copy.
 */
export type MenuEntry =
  | {
      kind: "command";
      id: string;
      label: string;
      shortcut?: string;
      enabled: boolean;
      checked?: boolean;
    }
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
 * Where the title bar's maximize button is, in device pixels from the window's top left
 * (`ButtonRect` in `app/src/title_bar.rs`).
 */
export type ButtonRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

/**
 * What the app tells the page in its URL's query when it opens the window (`page_url` in
 * `app/src/window.rs`), each flag on when the query names it.
 */
export type PageFlags = {
  /** The timing switch is on, so the page sends its timing reports. */
  timing: boolean;
  /** The window sits on the system's backdrop, so the page draws no background of its own. */
  backdrop: boolean;
  /** The window has no native frame, so the page draws the title bar (ADR 0005). */
  titleBar: boolean;
};

/**
 * The flags the app opened the page with. The page never navigates (the app allows only the
 * page's own URL), so they hold for the page's life; they are read from the URL at each call so
 * the tests can open the page with other flags.
 */
export function pageFlags(): PageFlags {
  const query = new URLSearchParams(location.search);
  return {
    timing: query.has("timing"),
    backdrop: query.has("backdrop"),
    titleBar: query.has("titleBar"),
  };
}

/**
 * What the page shows of the reader's settings, one snapshot of all of them (`PageSettings` in
 * `app/src/settings.rs`, window design ticket 12): the first in the page's URL, and each change in
 * a `settings` message. The page hears both in one place (`whenSettingsChange` in `settings.ts`).
 */
export type PageSettings = {
  /** The document view's zoom level in percent (ticket 16). */
  zoom: number;
  /** Whether the document fills the window's width (window design ticket 06). */
  fullWidth: boolean;
};

/**
 * The settings of a reader who changed none, for a URL without a snapshot the page can read. The
 * app's `PageSettings::default` (`app/src/settings.rs`) is the source; a contract test checks this
 * against it (the page URL's `defaults` sample).
 */
export const DEFAULT_SETTINGS: PageSettings = { zoom: 100, fullWidth: false };

function isPageSettings(value: unknown): value is PageSettings {
  return (
    typeof value === "object" &&
    value !== null &&
    "zoom" in value &&
    typeof value.zoom === "number" &&
    Number.isFinite(value.zoom) &&
    value.zoom > 0 &&
    "fullWidth" in value &&
    typeof value.fullWidth === "boolean"
  );
}

/**
 * The settings the app opened the page with, from its URL's `settings`, read at each call so the
 * tests can open the page with other ones. A URL without them, or with ones the page can't read,
 * gives the defaults rather than a page that can't start.
 */
export function firstSettings(): PageSettings {
  const given = new URLSearchParams(location.search).get("settings");
  if (given === null) return DEFAULT_SETTINGS;
  try {
    const settings: unknown = JSON.parse(given);
    return isPageSettings(settings) ? settings : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

/** A change of the zoom level that every window shares (`ZoomChange` in `app/src/zoom.rs`). */
export type ZoomChange = "in" | "out" | "reset";

/**
 * A change of a setting every window shares, tagged with the setting it changes (`SettingChange`
 * in `app/src/settings/change.rs`): a zoom step (ticket 16), or Full width switched (window design
 * ticket 06), switched rather than set so that two windows switching it at once each switch it.
 */
export type SettingChange =
  { setting: "zoom"; change: ZoomChange } | { setting: "switchFullWidth" };

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
  /**
   * What the wait note names, asked for when `render_document` hasn't answered within half a
   * second, with the showing the request sent; null while the window shows no document, or when
   * the window is still at that showing, so the request is a reload, which keeps the document in
   * view instead.
   */
  wait_note: { args: { showing: number | null }; answer: WaitNote | null };
  follow_link: {
    /**
     * The showing whose document the link is in, the link target as the document wrote it, and
     * whether to open a new window (Ctrl+click). A link from a showing the window has left does
     * nothing.
     */
    args: { showing: number; target: string; newWindow: boolean };
    answer: Followed;
  };
  open_document: { args: undefined; answer: null };
  /**
   * Back or Forward to another document in the window's history, which the window is then given
   * as it is any other (`ask-for-document`). Nowhere to go does nothing.
   */
  step_history: { args: { step: Step }; answer: null };
  /**
   * A link scrolled the view, which shows this showing, to a heading: as in a browser, Forward has
   * nowhere to go any more. A showing the window has left changes nothing.
   */
  forget_forward: { args: { showing: number }; answer: null };
  show_document_menu: { args: { entries: MenuEntry[] }; answer: null };
  close_window: { args: undefined; answer: null };
  show_window: { args: undefined; answer: null };
  report_first_paint: { args: { marks: FirstPaintMarks }; answer: null };
  report_reload: { args: { marks: ReloadMarks }; answer: null };
  report_long_task: { args: { task: LongTask }; answer: null };
  minimize_window: { args: undefined; answer: null };
  toggle_maximize_window: { args: undefined; answer: null };
  /** The title bar was laid out, and its maximize button is at `button` now. */
  place_maximize_button: { args: { button: ButtonRect }; answer: null };
  /** The logo was clicked: the window menu, as Alt+Space opens it. */
  show_window_menu: { args: undefined; answer: null };
  /**
   * The file name was clicked: the window's document selected in its folder. Rejects with why,
   * such as a file that was deleted or moved.
   */
  reveal_document: { args: undefined; answer: null };
  /**
   * The folder of a notice's missing file was clicked: the folder, as the notice named it, opened
   * in the file manager. The app opens only a folder a notice named, and only while it is there.
   */
  open_named_folder: { args: { folder: string }; answer: null };
  /**
   * Edit: the window's document opened in the reader's editor (ticket 15). Rejects with why, such
   * as a window without a document or no editor that started.
   */
  edit_document: { args: undefined; answer: null };
  /**
   * Choose editor...: a file dialog for a program, kept in the settings as the reader's editor.
   * Choosing nothing changes nothing; a choice that can't be the editor rejects with why.
   */
  choose_editor: { args: undefined; answer: null };
  /**
   * A change of a shared setting, such as Zoom in or Full width, for every window: the app sends
   * each page the settings in a `settings` message, this one too. Rejects with why when the change
   * couldn't be saved; it holds until Downlight closes all the same.
   */
  change_setting: { args: { change: SettingChange }; answer: null };
  /**
   * Keep on top (window design ticket 07), for the asking window only: switched on if it was off,
   * and off if it was on. Answers whether the window is kept on top now. Rejects with why when the
   * window couldn't be switched.
   */
  switch_keep_on_top: { args: undefined; answer: boolean };
};

export type Command = keyof Commands;
export type ArgsOf<C extends Command> = Commands[C]["args"];
export type AnswerOf<C extends Command> = Commands[C]["answer"];

/** A command's arguments as `askApp` takes them: none for a command that takes none. */
type ArgumentList<C extends Command> =
  ArgsOf<C> extends undefined ? [] : [args: ArgsOf<C>];

/**
 * Asks the app for `command` and resolves with its answer. Rejects with why when the command
 * fails, as a `Notice`, or when the app couldn't be asked, as Tauri gives it (`showingWhy` in
 * `notice.ts` tells the two apart).
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
  /**
   * The window shows another document now, or the document's file changed: the page asks for it,
   * and the answer says whether to show it or reload it.
   */
  "ask-for-document": null;
  /** The file of the document the window shows as this showing was deleted or moved. */
  "document-gone": number;
  /**
   * The heading a link in another window named in the document the window shows as this showing,
   * to scroll to once the view shows that showing.
   */
  "scroll-to-heading": { showing: number; heading: string };
  /** A notice to show the reader. */
  notice: Notice;
  /** The id of the command the reader chose in the right-click menu. */
  "menu-command": string;
  /** The window's size changed, and whether it is maximized now. */
  "window-state": "maximized" | "restored";
  /**
   * How the maximize button is to look while the pointer is on it, which on Windows reaches the
   * app's window behind the button rather than the page.
   */
  "maximize-button": "normal" | "hovered" | "pressed";
  /** What the page shows of the settings, whole, since one of them changed. */
  settings: PageSettings;
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
  void startListeningToApp(message, handler);
}

/**
 * `listenToApp`, resolved once the page hears `message`: registering a listener is a round trip
 * to the app, and a message the app sends before it is done is lost. Never rejects.
 */
export async function startListeningToApp<M extends Message>(
  message: M,
  handler: (payload: Messages[M]) => void,
): Promise<void> {
  try {
    await getCurrentWebviewWindow().listen<Messages[M]>(
      message,
      ({ payload }) => {
        handler(payload);
      },
    );
  } catch (error: unknown) {
    // Without the listener the window would miss what the app tells it.
    showNotice(String(error));
  }
}
