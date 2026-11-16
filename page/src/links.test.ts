import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import github from "../../core/tests/fixtures/github.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import type { ArgsOf, Followed } from "./app";
import { startLinks } from "./links";
import {
  fakeTheApp,
  settle,
  spyOnHeadingScrolls,
  spyOnScrollsToTop,
} from "./test-support";
import { SCROLLER_SELECTOR } from "./view";

/** Each `follow_link` call's arguments, in turn. */
let follows: ArgsOf<"follow_link">[] = [];

/** What `follow_link` answers. */
let answer: () => Promise<Followed> = () =>
  Promise.resolve({ kind: "nothing" });

let stopLinks: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  follows = [];
  answer = () => Promise.resolve({ kind: "nothing" });
  fakeTheApp({
    follow_link: (args) => {
      follows.push(args);
      return answer();
    },
  });
  stopLinks = new AbortController();
});

afterEach(() => {
  stopLinks.abort();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/**
 * Shows `html` as the document, in its scroller, as the document view does, and starts following
 * its links.
 */
function showDocument(html: string): HTMLElement {
  const view = document.createElement("article");
  view.className = "markdown-body";
  view.innerHTML = html;
  const scroller = document.createElement("div");
  scroller.className = SCROLLER_SELECTOR.slice(1);
  scroller.append(view);
  document.body.append(scroller);
  startLinks(stopLinks.signal);
  return view;
}

/** Clicks `element` as a mouse would and tells whether the webview would still act on it. */
function click(element: Element, init: MouseEventInit = {}): boolean {
  return element.dispatchEvent(
    new MouseEvent("click", { bubbles: true, cancelable: true, ...init }),
  );
}

function link(view: HTMLElement, href: string): HTMLAnchorElement {
  const found = [...view.querySelectorAll("a")].find(
    (a) => a.getAttribute("href") === href,
  );
  if (!found) throw new Error(`no link to ${href}`);
  return found;
}

function bubble(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".link-target");
}

test("clicking a link follows its target and never navigates the webview", async () => {
  const view = showDocument(
    '<p><a href="docs/my%20guide.md#install">Guide</a></p>',
  );

  const navigates = click(link(view, "docs/my%20guide.md#install"));
  await settle();

  expect(navigates).toBe(false);
  expect(follows).toEqual([
    { target: "docs/my%20guide.md#install", newWindow: false },
  ]);
});

test("ctrl+click asks for a new window", async () => {
  const view = showDocument('<p><a href="guide.md">Guide</a></p>');

  const navigates = click(link(view, "guide.md"), { ctrlKey: true });
  await settle();

  expect(navigates).toBe(false);
  expect(follows).toEqual([{ target: "guide.md", newWindow: true }]);
});

test("heading outcome scrolls to the element with that id in the document", async () => {
  answer = () => Promise.resolve({ kind: "scrollTo", heading: "options" });
  const view = showDocument(readme);
  const scrolledTo = spyOnHeadingScrolls();

  click(link(view, "#options"));
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("options");
  });
  expect(new Set(scrolledTo())).toEqual(new Set(["options"]));
});

test("heading outcome scrolls to a footnote", async () => {
  answer = () => Promise.resolve({ kind: "scrollTo", heading: "fn-paper" });
  const view = showDocument(github);
  const scrolledTo = spyOnHeadingScrolls();

  click(link(view, "#fn-paper"));
  await vi.waitFor(() => {
    expect(scrolledTo()).toContain("fn-paper");
  });
});

test("heading outcome finds an anchor by its name", async () => {
  answer = () => Promise.resolve({ kind: "scrollTo", heading: "old anchor" });
  const view = showDocument(
    '<p><a href="#old%20anchor">Up</a></p><p><a name="old anchor">Here</a></p>',
  );
  const scrolls = vi
    .spyOn(Element.prototype, "scrollIntoView")
    .mockImplementation(() => {});

  click(link(view, "#old%20anchor"));
  await vi.waitFor(() => {
    expect(scrolls).toHaveBeenCalled();
  });
  expect((scrolls.mock.contexts[0] as Element).textContent).toBe("Here");
});

test("heading outcome looks for the heading in the document only", async () => {
  answer = () => Promise.resolve({ kind: "scrollTo", heading: "outside" });
  const outside = document.createElement("div");
  outside.id = "outside";
  document.body.append(outside);
  const view = showDocument('<p><a href="#outside">Out</a></p>');
  const scrolledTo = spyOnHeadingScrolls();

  click(link(view, "#outside"));
  await settle();
  await settle();
  expect(scrolledTo()).toEqual([]);
});

test("heading outcome without a heading scrolls to the top", async () => {
  answer = () => Promise.resolve({ kind: "scrollTo", heading: null });
  const view = showDocument('<p><a href="#">Top</a></p>');
  const scrollsToTop = spyOnScrollsToTop();

  click(link(view, "#"));
  await vi.waitFor(() => {
    expect(scrollsToTop()).toBe(1);
  });
});

test("link to a missing file shows the app's message", async () => {
  answer = () => Promise.reject(new Error("Can't find C:\\docs\\gone.md."));
  const view = showDocument('<p><a href="gone.md">Gone</a></p>');

  click(link(view, "gone.md"));
  await vi.waitFor(() => {
    expect(document.querySelector(".notice")?.textContent).toContain(
      "Can't find C:\\docs\\gone.md.",
    );
  });
});

test("hovering or focusing a link shows its target, leaving hides it", () => {
  const view = showDocument(
    '<p><a href="docs/my%20guide.md#install">Guide</a> <a href="https://example.com/">Web</a></p>',
  );
  const guide = link(view, "docs/my%20guide.md#install");

  guide.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  expect(bubble()?.hidden).toBe(false);
  expect(bubble()?.textContent).toBe("docs/my%20guide.md#install");

  guide.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
  expect(bubble()?.hidden).toBe(true);

  const web = link(view, "https://example.com/");
  web.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  expect(bubble()?.hidden).toBe(false);
  expect(bubble()?.textContent).toBe("https://example.com/");

  web.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
  expect(bubble()?.hidden).toBe(true);
});

/** The target the bubble shows while the pointer is over a link to `href`. */
function shownTargetOf(href: string): string | null | undefined {
  const view = showDocument("<p><a>Link</a></p>");
  const anchor = view.querySelector("a");
  if (!anchor) throw new Error("no link");
  anchor.setAttribute("href", href);
  anchor.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  return bubble()?.textContent;
}

// The reader's browser goes to the punycode host, so a look-alike letter must not pass for the
// real one.
test("the target of a web link with a look-alike letter in its host shows the host in punycode", () => {
  const shown = shownTargetOf("https://ex\u0430mple.com/");
  expect(shown).toMatch(/^https:\/\/xn--[a-z0-9-]+\.com\/$/);
});

test.each([
  [
    "a bidi control",
    "https://example.com/\u202Egnp.exe",
    "https://example.com/%E2%80%AEgnp.exe",
  ],
  ["a space", "http://example.com/a b", "http://example.com/a%20b"],
  [
    "a mail address with a look-alike letter",
    "mailto:ann@ex\u0430mple.com",
    "mailto:ann@ex%D0%B0mple.com",
  ],
  ["capitals", "HTTPS://Example.COM", "https://example.com/"],
])(
  "the target of a web link with %s shows as the URL that is opened",
  (_what, href, shown) => {
    expect(shownTargetOf(href)).toBe(shown);
  },
);

test.each([
  "docs/my guide.md",
  "C:\\docs\\\u00e4.md",
  "\\\\server\\share\\a.md",
  "file:///C:/docs/\u00e4.md",
  "#\u00fcber",
  "https://[not a host/",
])("the target %s shows as the document wrote it", (href) => {
  expect(shownTargetOf(href)).toBe(href);
});

test("moving within a link keeps its target shown", () => {
  const view = showDocument(
    '<p><a href="guide.md"><em>Guide</em> here</a></p>',
  );
  const guide = link(view, "guide.md");
  const inner = guide.querySelector("em");

  guide.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  guide.dispatchEvent(
    new MouseEvent("mouseout", { bubbles: true, relatedTarget: inner }),
  );
  expect(bubble()?.hidden).toBe(false);
});

test("hovering a link asks the app nothing", async () => {
  const view = showDocument(
    '<p><a href="\\\\server\\share\\a.md">Share</a></p>',
  );

  const share = link(view, "\\\\server\\share\\a.md");
  share.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  share.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  await settle();

  expect(follows).toEqual([]);
});

test("link without a target does nothing", async () => {
  const view = showDocument("<p><a>Link</a></p>");

  const anchor = view.querySelector("a");
  if (!anchor) throw new Error("no link");
  click(anchor);
  anchor.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  await settle();

  expect(follows).toEqual([]);
  expect(bubble()?.hidden ?? true).toBe(true);
});

test("middle and shift clicks are stopped", async () => {
  const view = showDocument('<p><a href="https://example.com/">Web</a></p>');
  const web = link(view, "https://example.com/");

  const middle = web.dispatchEvent(
    new MouseEvent("auxclick", { bubbles: true, cancelable: true, button: 1 }),
  );
  const shift = click(web, { shiftKey: true });
  await settle();

  expect(middle).toBe(false);
  expect(shift).toBe(false);
  expect(follows).toEqual([]);
});

test("clicks outside the document's links are left alone", async () => {
  const view = showDocument("<p>Text</p>");
  const outside = document.createElement("a");
  outside.href = "#elsewhere";
  document.body.append(outside);

  const onText = click(view.querySelector("p") ?? view);
  const onOutside = click(outside);
  await settle();

  expect(onText).toBe(true);
  expect(onOutside).toBe(true);
  expect(follows).toEqual([]);
});
