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
 * A document's lines and words (`Counts` in `core/src/counts.rs`, window design ticket 10), which
 * the core counts while it renders: the file's lines, as line numbers count them, and the words of
 * the text the reader sees.
 */
export type DocumentCounts = { lines: number; words: number };

/**
 * A document as the core rendered it (`RenderedDocument` in `core/src/render.rs`): its sanitized
 * HTML, and its counts, which the title bar's file name tooltip shows.
 */
export type RenderedDocument = { html: string; counts: DocumentCounts };

/**
 * The app's answer when the page asks for its document (`DocumentToShow` in
 * `app/src/documents.rs`), or null while the window shows no document: the showing the window is
 * at, for the next request, the document's file, what to do with the render, the rendered
 * document or why it can't be read, a notice the app held until the page first asked, or null,
 * and whether the window has a document to go back or forward to.
 */
export type DocumentToShow =
  | ({
      showing: number;
      file: DocumentFile;
      notice: Notice | null;
      steps: Steps;
    } & Place &
      ({ document: RenderedDocument } | { error: Notice }))
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

/**
 * A piece of the document's file that Copy as Markdown copies (`Piece` in `app/src/copy.rs`,
 * copy-as-markdown tickets 01 to 03): a range of whole lines, the first and the last, 1-based as
 * the rendered blocks' line numbers count them, or a span within one line. The lines of the blocks
 * a selection runs over, a code block's fence lines and the code lines the selection touches, and
 * a table's header, delimiter and touched rows are all ranges. A selection within one table cell
 * is the cell's span: its line, and its first and last column as its `data-sourcepos` counts
 * them (1-based, in bytes of the line, both included), which the app copies without the spaces
 * around it.
 */
export type Piece =
  | { piece: "lines"; first: number; last: number }
  | { piece: "span"; line: number; first: number; last: number };

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
  /**
   * The system can show a backdrop (Mica on Windows 11) and the window sits on it, so the page
   * draws no background of its own.
   */
  backdropSupported: boolean;
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
    backdropSupported: query.has("backdropSupported"),
    titleBar: query.has("titleBar"),
  };
}

/**
 * The label of the window the page is in, as the app named it when it opened the window. It
 * holds for the page's life.
 */
export function windowLabel(): string {
  return getCurrentWebviewWindow().label;
}

/**
 * The `dataTransfer` type the page tags each drag that starts in it with, its window's label the
 * tag's data (`DRAG_TAG` in `app/src/open.rs`): the app's drop target refuses a drag tagged with
 * its own window, so dropping a selection, a link or an image back onto the page does nothing
 * (paste-markdown ticket 04).
 */
export const DRAG_TAG = "application/x-downlight-window";

/**
 * Every Theme the reader can pick (`Theme` in `app/src/settings/look.rs`, window design ticket
 * 08), in the order the settings flyout offers them.
 */
export const THEMES = ["system", "light", "dark"] as const;
export type Theme = (typeof THEMES)[number];

/** Every Backdrop the reader can pick (`Backdrop` in `app/src/settings/look.rs`), in order. */
export const BACKDROPS = ["mica", "micaAlt", "acrylic", "none"] as const;
export type Backdrop = (typeof BACKDROPS)[number];

/**
 * Every Accent color the reader can pick (`AccentColor` in `app/src/settings/look.rs`), in order:
 * amber, the page's own, and Windows' (window design ticket 09), offered where the OS has one.
 */
export const ACCENT_COLORS = ["amber", "windows"] as const;
export type AccentColor = (typeof ACCENT_COLORS)[number];

/**
 * The accent scale's seven shades, Light 3 to Dark 3 (spec: Accent color), each the
 * `--accent-<shade>` variable `accent.css` defines, and each a key of the scale the app sends
 * (`AccentScale` in `app/src/settings/look.rs`). The one list of their names.
 */
export const ACCENT_SHADES = [
  "light-3",
  "light-2",
  "light-1",
  "base",
  "dark-1",
  "dark-2",
  "dark-3",
] as const;
export type AccentShade = (typeof ACCENT_SHADES)[number];

/** An accent color's seven shades, each a CSS color `#rrggbb`. */
export type AccentScale = Record<AccentShade, string>;

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
  /** Whether the page shows the overview ruler beside the scroll bar (overview ruler ticket 02). */
  overviewRuler: boolean;
  /**
   * The reader's theme, which the settings flyout shows. The app sets it on the window, which
   * gives the page its `prefers-color-scheme`, so nothing else in the page reads it.
   */
  theme: Theme;
  /** The reader's backdrop: with None the page draws its solid background (`backdrop.ts`). */
  backdrop: Backdrop;
  accentColor: AccentColor;
  /**
   * Windows' accent scale now, which the page colors with while the reader's accent color is
   * Windows' (`accent.ts`); a new snapshot comes whenever Windows' changes. Null where the OS has
   * no accent color of its own, and the settings flyout then offers no choice of accent color.
   */
  windowsAccent: AccentScale | null;
};

/**
 * The settings of a reader who changed none, for a URL without a snapshot the page can read. The
 * app's `PageSettings::default` (`app/src/settings.rs`) is the source; a contract test checks this
 * against it (the page URL's `defaults` sample).
 */
export const DEFAULT_SETTINGS: PageSettings = {
  zoom: 100,
  fullWidth: false,
  overviewRuler: true,
  theme: "system",
  backdrop: "mica",
  accentColor: "amber",
  windowsAccent: null,
};

/** Whether `value` is one of `values`. */
function isOneOf<T extends string>(
  values: readonly T[],
  value: unknown,
): value is T {
  return (values as readonly unknown[]).includes(value);
}

function isPageSettings(value: unknown): value is PageSettings {
  return (
    typeof value === "object" &&
    value !== null &&
    "zoom" in value &&
    typeof value.zoom === "number" &&
    Number.isFinite(value.zoom) &&
    value.zoom > 0 &&
    "fullWidth" in value &&
    typeof value.fullWidth === "boolean" &&
    "overviewRuler" in value &&
    typeof value.overviewRuler === "boolean" &&
    "theme" in value &&
    isOneOf(THEMES, value.theme) &&
    "backdrop" in value &&
    isOneOf(BACKDROPS, value.backdrop) &&
    "accentColor" in value &&
    isOneOf(ACCENT_COLORS, value.accentColor) &&
    "windowsAccent" in value &&
    (value.windowsAccent === null || isAccentScale(value.windowsAccent))
  );
}

/** A CSS color as the app writes a shade, which the page sets as it is. */
const SHADE_COLOR = /^#[0-9a-f]{6}$/;

function isAccentScale(value: unknown): value is AccentScale {
  return (
    typeof value === "object" &&
    value !== null &&
    ACCENT_SHADES.every((shade) => {
      const color: unknown = (value as Record<string, unknown>)[shade];
      return typeof color === "string" && SHADE_COLOR.test(color);
    })
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
 * in `app/src/settings/change.rs`): a zoom step (ticket 16), Full width switched (window design
 * ticket 06), switched rather than set so that two windows switching it at once each switch it, a
 * value the reader picked in the settings flyout (ticket 08), set rather than switched, or Edit
 * with's Default, which forgets the chosen editor.
 */
export type SettingChange =
  | { setting: "zoom"; change: ZoomChange }
  | { setting: "switchFullWidth" }
  | { setting: "theme"; theme: Theme }
  | { setting: "backdrop"; backdrop: Backdrop }
  | { setting: "accentColor"; accentColor: AccentColor }
  | { setting: "overviewRuler"; overviewRuler: boolean }
  | { setting: "clearEditor" };

/**
 * What Edit with in the settings flyout offers (`EditorChoices` in `app/src/edit.rs`): Default,
 * named after the program Edit opens a document in without a chosen editor, or null when the
 * system names none, and the editor the reader chose, while Edit would open it, with its file
 * for the tooltip.
 */
export type EditorChoices = {
  defaultName: string | null;
  editor: { name: string; path: string } | null;
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
  /**
   * What the wait note names, asked for when `render_document` hasn't answered within half a
   * second, with the showing the request sent; null while the window shows no document, or when
   * the window is still at that showing, so the request is a reload, which keeps the document in
   * view instead.
   */
  wait_note: { args: { showing: number | null }; answer: WaitNote | null };
  /**
   * The reader stopped the document on its way, from the wait note or with Esc (ticket 38), while
   * the page's last placed answer was for this showing (`DocumentToShow`'s, also for an answer
   * that says why the document can't be shown), or null for the empty window: the window goes
   * back to what it was before it was given the document. Answers whether it did, which it
   * doesn't once the document has arrived or the showing isn't the one it holds.
   */
  stop_document: { args: { showing: number | null }; answer: boolean };
  /**
   * The page placed the answer for this showing, a show, a return or a reload, its render or why
   * it can't be shown, so the document joins the window's history (ticket 38).
   */
  document_arrived: { args: { showing: number }; answer: null };
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
   * Paste (paste-markdown ticket 01): the clipboard's text, which the app reads itself, written to a
   * file of its own and given to the window as Open gives it a file. Rejects with why, such as a
   * clipboard that holds no text, and the window stays as it was.
   */
  paste_clipboard: { args: undefined; answer: null };
  /**
   * Copy as Markdown (copy-as-markdown ticket 01): the Markdown of these pieces of the file the
   * view's render, shown as this showing, was made from, or of the whole file for none, which the
   * app puts on the clipboard itself, so a large copy never passes through the page. A showing the
   * window has left copies nothing. Rejects with why, such as a clipboard another program holds.
   */
  copy_markdown: { args: { showing: number; pieces: Piece[] }; answer: null };
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
   * Edit: the window's document opened in the reader's editor (ticket 15), at `line`, the reading
   * position's line, for an editor the app knows to take one (ticket 35); null when the block at
   * the top of the view has no line, as raw HTML has none, which opens the document as the editor
   * opens it without one. Rejects with why, such as a window without a document or no editor that
   * started.
   */
  edit_document: { args: { line: number | null }; answer: null };
  /**
   * Choose editor...: a file dialog for a program, kept in the settings as the reader's editor.
   * Choosing nothing changes nothing; a choice that can't be the editor rejects with why.
   */
  choose_editor: { args: undefined; answer: null };
  /** What Edit with in the settings flyout offers now, asked for each time the flyout opens. */
  editor_choices: { args: undefined; answer: EditorChoices };
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
  /**
   * Full screen (full screen ticket 01), for the asking window only: a borderless window covering
   * its monitor if it wasn't full screen, and back to how it was if it was. Answers whether the
   * window is full screen now. Rejects with why when the window couldn't be switched.
   */
  switch_full_screen: { args: undefined; answer: boolean };
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
  /**
   * The reader asked for the document the view shows as this showing (the showing of the last
   * answer placed) while the window's next document is on its way, by Ctrl+O, a drop, a link or
   * a step in another window: the page stops the load, as Stop does (ticket 38's review).
   */
  "stop-document": number;
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
