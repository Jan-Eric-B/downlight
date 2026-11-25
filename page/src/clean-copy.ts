import { isEditable } from "./editable";
import { DOCUMENT_VIEW_SELECTOR, selectionInView } from "./view";

/**
 * The clean copy (copy-as-markdown ticket 04): Ctrl+C, and the menu's Copy, put clean HTML on the
 * clipboard beside the plain text, so that a paste into Word or Outlook keeps the document's
 * formatting in the target's own styles, never Downlight's colors, theme or copy buttons.
 *
 * Both come from the copy event, which the webview raises for either way to copy. The page cancels
 * the webview's own copy, which would carry the page's look, and sets the two flavors itself:
 * the plain text is the selection's text, and the HTML is written from the selected part of the
 * view by the allowlist below. The selected part is walked once and written as it goes, never
 * cloned (a clone's images would start loading), so a large selection costs one pass over its
 * nodes. The pass reads no layout, apart from the style of the few images that the theme may hide;
 * the plain text, `Selection.toString()`, is the webview's own and does lay out.
 */

/** Where the page's own local images are loaded from, as the app's image scheme names its URLs. */
function isImageSchemeUrl(url: URL): boolean {
  // The origin is `http://image.localhost` on Windows and `image://localhost` elsewhere (the app's
  // `platform::custom_scheme_origin`); a trailing dot names the same host.
  const host = url.hostname.replace(/\.$/, "");
  return (
    (url.protocol === "http:" && host === "image.localhost" && !url.port) ||
    (url.protocol === "image:" && host === "localhost")
  );
}

/**
 * A drive path as the image scheme's URL path writes it, which the core makes from a local
 * image's `file:` URL (`/C:/lamps/a.png`). Any other path, such as `//host/share/a.png`, names no
 * file on this computer's drives: a `file:` URL made of it would lead a paste's target to a host.
 */
const DRIVE_PATH = /^\/[A-Za-z]:\//;

/** The links that stay links, since a target outside Downlight can follow them. */
const LINK_SCHEMES = new Set(["http:", "https:", "mailto:"]);

/** The images that load as they are: the web's and the document's own embedded ones. */
const WEB_IMAGE_SCHEMES = new Set(["http:", "https:", "data:"]);

/** GitHub's images for one theme only, which `document.css` hides in the other theme. */
const THEME_IMAGE = /#gh-(dark|light)-mode-only$/;

/** What an element is to a paste, by its tag. */
type Role =
  /** Goes with what is in it, which is not a document's text: the page's own parts, and what a browser never shows as text. */
  | "dropped"
  /** Comes over as it is, with no attributes; a selection inside it doesn't keep it. */
  | "structure"
  /**
   * Comes over as it is, and a selection of its children keeps it around them, since they are not
   * a paste without it (a row's cells, a list's items) or the quote or list is part of the copy.
   */
  | "container"
  /** Comes over as it is, and a selection inside it keeps it around its text, as a browser's copy does. */
  | "inline";

/**
 * The role of each tag a document's HTML can hold that a paste treats on its own; every other
 * element is left out, its children staying. Adding an element is one line here, and, when its
 * attributes or children need care, one case in `markupOf`.
 */
const ROLES: Readonly<Record<string, Role>> = {
  script: "dropped",
  style: "dropped",
  template: "dropped",
  button: "dropped",
  svg: "dropped",
  iframe: "dropped",
  object: "dropped",
  embed: "dropped",
  audio: "dropped",
  video: "dropped",
  canvas: "dropped",
  noscript: "dropped",
  select: "dropped",
  textarea: "dropped",
  source: "dropped",
  track: "dropped",
  h1: "structure",
  h2: "structure",
  h3: "structure",
  h4: "structure",
  h5: "structure",
  h6: "structure",
  p: "structure",
  br: "structure",
  hr: "structure",
  li: "structure",
  dt: "structure",
  dd: "structure",
  th: "structure",
  td: "structure",
  caption: "structure",
  blockquote: "container",
  ul: "container",
  ol: "container",
  dl: "container",
  table: "container",
  thead: "container",
  tbody: "container",
  tfoot: "container",
  tr: "container",
  em: "inline",
  strong: "inline",
  b: "inline",
  i: "inline",
  u: "inline",
  del: "inline",
  s: "inline",
  ins: "inline",
  sub: "inline",
  sup: "inline",
  small: "inline",
  kbd: "inline",
  samp: "inline",
  var: "inline",
  code: "inline",
  pre: "inline",
  a: "inline",
};

function roleOf(tag: string): Role | undefined {
  return Object.hasOwn(ROLES, tag) ? ROLES[tag] : undefined;
}

/** The kept elements that have no end tag. */
const VOID = new Set(["br", "hr"]);

/** The checkbox marks that stand for a task item's box, where a target drops checkboxes. */
const TICKED = "☑";
const UNTICKED = "☐";

/**
 * What an element of the document is in a paste: the tags around what is in it (the children are
 * written between them), or text alone, with what is in the element left out.
 */
type Markup =
  | { kind: "tags"; open: string; close: string }
  | { kind: "text"; text: string };

const AS_ITS_CHILDREN: Markup = { kind: "tags", open: "", close: "" };

function escapeText(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\u00a0", "&nbsp;");
}

function escapeAttribute(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/** ` name="value"` for each of `names` that `element` has, the only attributes a paste keeps. */
function attributesOf(element: Element, names: string[]): string {
  let attributes = "";
  for (const name of names) {
    const value = element.getAttribute(name);
    if (value !== null) attributes += ` ${name}="${escapeAttribute(value)}"`;
  }
  return attributes;
}

/** The start tag of `tag`, with no attributes, and its end tag. */
function plainTags(tag: string): Markup {
  return {
    kind: "tags",
    open: `<${tag}>`,
    close: VOID.has(tag) ? "" : `</${tag}>`,
  };
}

/** The start tag of `element`, with those of `names` that it has, and its end tag. */
function tagsWithAttributes(element: Element, names: string[]): Markup {
  const tag = element.localName;
  return {
    kind: "tags",
    open: `<${tag}${attributesOf(element, names)}>`,
    close: `</${tag}>`,
  };
}

/** A link as written for a target: a web or mail link stays one, any other is its text. */
function linkMarkup(link: Element): Markup | null {
  // A heading's anchor and a footnote's way back lead nowhere in a paste and are the page's own.
  if (
    link.classList.contains("anchor") ||
    link.hasAttribute("data-footnote-backref")
  )
    return null;
  const href = link.getAttribute("href") ?? "";
  const url = URL.parse(href);
  if (!url || !LINK_SCHEMES.has(url.protocol)) return AS_ITS_CHILDREN;
  return {
    kind: "tags",
    open: `<a href="${escapeAttribute(href)}">`,
    close: "</a>",
  };
}

/**
 * Whether the theme hides `image`, which GitHub's `#gh-dark-mode-only` and `#gh-light-mode-only`
 * images are in the other theme. Only those images have their style read.
 */
function isHiddenByTheme(image: Element): boolean {
  const src = image.getAttribute("src");
  return (
    src !== null &&
    THEME_IMAGE.test(src) &&
    getComputedStyle(image).display === "none"
  );
}

/**
 * An image as written for a target: a local image by its file's URL, so that Word and Outlook on
 * this computer can show it, a web image by its URL, and any other, which Downlight didn't load
 * either, as its alt text. One the theme hides is left out, as the page leaves it out.
 */
function imageMarkup(image: Element): Markup | null {
  if (isHiddenByTheme(image)) return null;
  const alt: Markup = {
    kind: "text",
    text: escapeText(image.getAttribute("alt") ?? ""),
  };
  const src = image.getAttribute("src");
  const url = src === null ? null : URL.parse(src);
  if (!src || !url) return alt;
  let source: string;
  if (isImageSchemeUrl(url)) {
    // The image scheme's path is the file's path, as its `file:` URL writes it, percent-encoded.
    // Only a drive path is one: any other, such as `//host/share/a.png`, would be a `file:` URL
    // to a network share (the core refuses such an image, and this holds if it ever didn't).
    if (!DRIVE_PATH.test(url.pathname)) return alt;
    source = `file://${url.pathname}`;
  } else if (WEB_IMAGE_SCHEMES.has(url.protocol)) source = src;
  else return alt;
  const sized = attributesOf(image, ["width", "height"]);
  const altAttribute = escapeAttribute(image.getAttribute("alt") ?? "");
  return {
    kind: "tags",
    open: `<img src="${escapeAttribute(source)}" alt="${altAttribute}"${sized}>`,
    close: "",
  };
}

/** A task item's checkbox as its mark, by the reader's tick, and the space that follows it. */
function checkboxMarkup(checkbox: HTMLInputElement): Markup {
  const mark = checkbox.checked ? TICKED : UNTICKED;
  const next = checkbox.nextSibling;
  // The core writes a space after the checkbox, which the mark then needs no other for.
  const spaced = next instanceof Text && /^\s/.test(next.data);
  return { kind: "text", text: spaced ? mark : `${mark} ` };
}

/** How `element` of the document is written, by the allowlist; null leaves it and its text out. */
function markupOf(element: Element): Markup | null {
  const tag = element.localName;
  switch (tag) {
    case "ol":
      return tagsWithAttributes(element, ["start"]);
    case "th":
    case "td":
      return tagsWithAttributes(element, ["align", "colspan", "rowspan"]);
    case "a":
      return linkMarkup(element);
    case "img":
      return imageMarkup(element);
    case "input":
      // Only a task item's checkbox is in a rendered document, and it is page markup to a paste.
      return element instanceof HTMLInputElement &&
        element.classList.contains("task-list-item-checkbox")
        ? checkboxMarkup(element)
        : null;
    case "div":
      // The page's own wrapper of the copy button, and an alert, which is a quote to a paste.
      if (element.classList.contains("zeroclipboard-container")) return null;
      return element.classList.contains("markdown-alert")
        ? plainTags("blockquote")
        : AS_ITS_CHILDREN;
    case "summary":
      return plainTags("p");
  }
  switch (roleOf(tag)) {
    case "dropped":
      return null;
    case undefined:
      return AS_ITS_CHILDREN;
    default:
      return plainTags(tag);
  }
}

/** A part of the document in the view, as the copy event's selection covers it. */
interface Clip {
  startContainer: Node;
  startOffset: number;
  endContainer: Node;
  endOffset: number;
  /** The start container and every node it is in, up to the document. */
  startPath: ReadonlySet<Node>;
  /** The end container and every node it is in, up to the document. */
  endPath: ReadonlySet<Node>;
}

function pathTo(node: Node): Set<Node> {
  const path = new Set<Node>();
  for (let next: Node | null = node; next; next = next.parentNode)
    path.add(next);
  return path;
}

function clipOf(range: Range): Clip {
  return {
    startContainer: range.startContainer,
    startOffset: range.startOffset,
    endContainer: range.endContainer,
    endOffset: range.endOffset,
    startPath: pathTo(range.startContainer),
    endPath: pathTo(range.endContainer),
  };
}

/** Writes the part of `text`, a text node, that the clip covers. */
function writeText(text: Text, clip: Clip, out: string[]): void {
  const from = text === clip.startContainer ? clip.startOffset : 0;
  const to = text === clip.endContainer ? clip.endOffset : text.length;
  out.push(escapeText(text.data.slice(from, to)));
}

/** Where the walk is in one element's children, and what to write when they are done. */
interface Frame {
  parent: Node;
  /** The child the walk looks at next, and its index in the parent. */
  next: ChildNode | null;
  index: number;
  /** True until the child the clip starts in: nothing before it is covered. */
  beforeStart: boolean;
  /** True when the clip ends in the parent. */
  hasEnd: boolean;
  /** True for a closed section, which shows only its summary. */
  onlySummary: boolean;
  /** The end tag of the parent. */
  close: string;
}

function frameOf(parent: Node, clip: Clip, close: string): Frame {
  return {
    parent,
    next: parent.firstChild,
    index: 0,
    beforeStart: clip.startPath.has(parent),
    hasEnd: clip.endPath.has(parent),
    onlySummary:
      parent instanceof Element &&
      parent.localName === "details" &&
      !parent.hasAttribute("open"),
    close,
  };
}

/**
 * The next child of the frame's parent that the clip covers and the page shows, or null when
 * there is none. A child the clip starts or ends in is covered, with only the part it covers.
 */
function nextCovered(frame: Frame, clip: Clip): ChildNode | null {
  while (frame.next) {
    const child = frame.next;
    const index = frame.index;
    frame.next = child.nextSibling;
    frame.index++;
    if (frame.beforeStart) {
      if (frame.parent === clip.startContainer) {
        if (index < clip.startOffset) continue;
        frame.beforeStart = false;
      } else if (clip.startPath.has(child)) frame.beforeStart = false;
      else continue;
    }
    if (frame.hasEnd) {
      if (frame.parent === clip.endContainer) {
        if (index >= clip.endOffset) return null;
      } else if (clip.endPath.has(child)) frame.next = null;
    }
    if (
      frame.onlySummary &&
      !(child instanceof Element && child.localName === "summary")
    )
      continue;
    return child;
  }
  return null;
}

/**
 * Writes the children of `root` that the clip covers, and theirs in turn. The walk keeps its place
 * in a stack of its own rather than in recursion, so a document nested deeper than the call stack
 * allows is copied like any other.
 */
function writeChildren(root: Node, clip: Clip, out: string[]): void {
  const stack = [frameOf(root, clip, "")];
  for (let frame = stack.at(-1); frame; frame = stack.at(-1)) {
    const child = nextCovered(frame, clip);
    if (!child) {
      out.push(frame.close);
      stack.pop();
    } else if (child instanceof Text) writeText(child, clip, out);
    else if (child instanceof Element) {
      const markup = markupOf(child);
      if (!markup) continue;
      if (markup.kind === "text") out.push(markup.text);
      else {
        out.push(markup.open);
        stack.push(frameOf(child, clip, markup.close));
      }
    }
  }
}

/**
 * The part of the document `view` shows that `range`, which lies in it, covers, as clean HTML, or
 * the empty string when a paste has nothing of it.
 */
function cleanHtmlOf(range: Range, view: Element): string {
  const within = range.commonAncestorContainer;
  const around: Extract<Markup, { kind: "tags" }>[] = [];
  const innermost = within instanceof Element ? within : within.parentElement;
  // Whether the elements so far, from the innermost, are all containers: a selection of a row's
  // cells or a list's items is wrapped in the row and list, as Chromium's copy does, and the
  // first element that is not one ends it.
  let wrapping = true;
  for (
    let element = innermost;
    element && element !== view;
    element = element.parentElement
  ) {
    const markup = markupOf(element);
    // A selection inside what a paste leaves out leaves out too.
    if (markup?.kind !== "tags") return "";
    const role = roleOf(element.localName);
    if (role !== "container") wrapping = false;
    if (role === "inline" || (role === "container" && wrapping))
      around.unshift(markup);
  }

  const out = around.map(({ open }) => open);
  const clip = clipOf(range);
  if (within instanceof Text) writeText(within, clip, out);
  else writeChildren(within, clip, out);
  for (const { close } of around.reverse()) out.push(close);
  return out.join("");
}

/**
 * Takes over the copy event for a selection in the document view: the plain text is the whole
 * selection's text (the webview's own copy writes the same, and the page's parts beside the view
 * stay in it), and the HTML is the clean one. Anything else is left to the webview, so a copy in
 * the Find box or another field copies its text as before.
 */
function copyCleanly(event: ClipboardEvent): void {
  const { clipboardData } = event;
  if (!clipboardData || event.defaultPrevented) return;
  if (isEditable(event.target) || isEditable(document.activeElement)) return;
  const range = selectionInView();
  const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
  if (!range || !view) return;
  const html = cleanHtmlOf(range, view);
  if (html === "") return;
  // `setData` writes the text as it is, where the webview's own copy gives Windows its CRLF line
  // breaks, as Notepad and the other Windows programs expect them.
  const text = document.getSelection()?.toString() ?? "";
  clipboardData.setData("text/plain", text.replace(/\r?\n/g, "\r\n"));
  clipboardData.setData("text/html", html);
  event.preventDefault();
}

/** Starts the clean copy. The signal removes the listener again, for the tests. */
export function startCleanCopy(signal?: AbortSignal): void {
  addEventListener("copy", copyCleanly, { signal });
}
