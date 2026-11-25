import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import code from "../../core/tests/fixtures/code.html?raw";
import find from "../../core/tests/fixtures/find.html?raw";
import github from "../../core/tests/fixtures/github.html?raw";
import imagesAndLinks from "../../core/tests/fixtures/images-and-links.html?raw";
import readme from "../../core/tests/fixtures/readme.html?raw";
import tables from "../../core/tests/fixtures/tables.html?raw";
import { startCleanCopy } from "./clean-copy";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startSettings } from "./settings";
import { fakeTheAppReading, settle } from "./test-support";
import { startDocumentView } from "./view";

// The clean copy (copy-as-markdown ticket 04): Ctrl+C, and the menu's Copy, put the plain text the
// webview copied and clean HTML on the clipboard. The tests raise the copy event as the webview
// does and read what the page put on its clipboard data, in the core's own HTML of the fixture
// documents.

let stop: AbortController;

beforeEach(() => {
  mockWindows("document-0");
  stop = new AbortController();
});

afterEach(() => {
  stop.abort();
  document.getSelection()?.removeAllRanges();
  vi.restoreAllMocks();
  clearMocks();
  document.body.replaceChildren();
});

/** Shows the rendered document `html` in the window. */
async function show(html: string): Promise<void> {
  fakeTheAppReading(() => [() => Promise.resolve(html)], {
    copy_markdown: () => null,
  });
  await startSettings();
  startFind(stop.signal);
  startCommands(stop.signal);
  startCleanCopy(stop.signal);
  await startDocumentView(0);
  await settle();
}

function view(): HTMLElement {
  const article = document.querySelector<HTMLElement>("article.markdown-body");
  if (!article) throw new Error("The document view isn't there.");
  return article;
}

/** What a copy event ended with: whether the page took the copy over, and what it put on. */
interface Copied {
  /** True when the page cancelled the webview's own copy. */
  taken: boolean;
  text: string;
  html: string;
}

/** Raises the copy event on `target`, as Ctrl+C and the menu's Copy do. */
function copy(target: EventTarget = document.body): Copied {
  const event = new ClipboardEvent("copy", {
    clipboardData: new DataTransfer(),
    bubbles: true,
    cancelable: true,
  });
  target.dispatchEvent(event);
  return {
    taken: event.defaultPrevented,
    text: event.clipboardData?.getData("text/plain") ?? "",
    html: event.clipboardData?.getData("text/html") ?? "",
  };
}

/** Selects the whole document, as Ctrl+A and then Ctrl+C would copy it from its view. */
function selectDocument(): void {
  const range = document.createRange();
  range.selectNodeContents(view());
  document.getSelection()?.removeAllRanges();
  document.getSelection()?.addRange(range);
}

/** Selects everything in the page, as Ctrl+A does: the document and what the page has beside it. */
function selectPage(): void {
  const range = document.createRange();
  range.selectNodeContents(document.body);
  document.getSelection()?.removeAllRanges();
  document.getSelection()?.addRange(range);
}

/** The element of the shown document that `selector` selects and whose text holds `text`. */
function elementWith(selector: string, text: string): Element {
  const found = [
    ...document.querySelectorAll(`article.markdown-body ${selector}`),
  ].find((element) => element.textContent.includes(text));
  if (!found) throw new Error(`The document has no ${selector} with ${text}.`);
  return found;
}

/** The text node in `element` that holds `text`, and where in it `text` starts. */
function pointAt(element: Element, text: string): [Text, number] {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const at = node.textContent?.indexOf(text) ?? -1;
    if (at >= 0) return [node as Text, at];
  }
  throw new Error(`No text ${text} in the element.`);
}

/** Selects from the start of `from`'s text `fromText` to the end of `to`'s text `toText`. */
function select(
  [from, fromText]: [Element, string],
  [to, toText]: [Element, string],
): void {
  const range = document.createRange();
  const [start, startOffset] = pointAt(from, fromText);
  const [end, endOffset] = pointAt(to, toText);
  range.setStart(start, startOffset);
  range.setEnd(end, endOffset + toText.length);
  document.getSelection()?.removeAllRanges();
  document.getSelection()?.addRange(range);
}

describe("Ctrl+C of the document view", () => {
  test("puts the selection's text on the clipboard as the webview copied it", async () => {
    await show(tables);
    const note = elementWith("td", "Trim it");
    select([note, "Trim"], [note, "before"]);

    const { taken, text } = copy();

    expect(taken).toBe(true);
    expect(text).toBe("Trim it before");
  });

  test("puts the text of a selection across blocks on the clipboard", async () => {
    await show("<h1>Lamp</h1>\n<p>one two</p>");
    select(
      [elementWith("h1", "Lamp"), "amp"],
      [elementWith("p", "one"), "one"],
    );

    const { text } = copy();

    expect(text).toBe("amp\r\none");
  });

  test("writes the plain text's line breaks as Windows does, as CRLF", async () => {
    await show("<pre><code>a\nb\nc</code></pre>");
    selectDocument();

    const { text } = copy();

    expect(text).toBe("a\r\nb\r\nc");
  });

  test("puts the text of the whole selection on the clipboard, though the HTML is cut at the view", async () => {
    await show("<p>shown</p>");
    const hint = document.querySelector(".empty-window p");
    if (!hint?.textContent)
      throw new Error("The empty window's hint isn't there.");
    selectPage();

    const { text, html } = copy();

    // The text is the selection's, as the webview copies it; only the HTML leaves the page's own
    // parts out.
    expect(text).toContain(hint.textContent);
    expect(text).toContain("shown");
    expect(html).toBe("<p>shown</p>");
  });

  test("puts only the cut blocks of a selection across paragraphs in the HTML", async () => {
    await show(readme);
    select(
      [elementWith("h1", "Lamplighter"), "lighter"],
      [elementWith("p", "small tool"), "printable"],
    );

    const { html } = copy();

    expect(html).toBe(
      "<h1>lighter</h1>\n<p>A small tool that turns <em>Markdown</em> notes into <strong>printable</strong></p>",
    );
  });

  test("leaves out the colors, classes, ids and line numbers of the page", async () => {
    await show(github);
    selectDocument();

    const { html } = copy();

    expect(html).toContain("<h1>Lamplighter on GitHub</h1>");
    expect(html).not.toMatch(/\s(class|style|id|data-[a-z-]+|aria-[a-z]+)=/);
  });

  test("leaves out what the page has beside the document when everything is selected", async () => {
    await show(github);
    selectPage();

    const { taken, html } = copy();

    expect(taken).toBe(true);
    expect(html).toContain("<h1>Lamplighter on GitHub</h1>");
    // The empty window's button and hint are in the page, hidden, beside the document view.
    expect(html).not.toContain("Open file");
    expect(html).not.toContain("<button");
  });

  test("leaves out Find's matches highlighted, keeping their text", async () => {
    await show(find);
    const box = document.querySelector<HTMLInputElement>(".find-bar input");
    if (!box) throw new Error("The Find box isn't there.");
    box.value = "lamp";
    box.dispatchEvent(new Event("input", { bubbles: true }));
    selectDocument();

    const { html } = copy();

    expect(html).toContain("The lamp stands by the door.");
    expect(html).not.toMatch(/<mark|<span|\s(class|style)=/);
  });

  test("leaves out the copy buttons and gives code as plain monospace", async () => {
    await show(code);
    selectDocument();

    const { html } = copy();

    expect(html).not.toMatch(/<button|<svg|<div|<span|zeroclipboard/i);
    expect(html).toContain(
      '<pre><code>/// Says hello.\nfn main() {\n    let name = "&lt;world&gt; &amp; co";\n',
    );
  });

  test("leaves out the heading anchors", async () => {
    await show(github);
    selectDocument();

    const { html } = copy();

    expect(html).toContain("<h2>Alerts</h2>");
    expect(html).not.toContain("Link to heading");
    expect(html).not.toContain("#alerts");
  });

  test("gives an alert as a quote whose first paragraph is its title", async () => {
    await show(github);
    selectDocument();

    const { html } = copy();

    expect(html).toContain(
      "<blockquote>\n<p>Note</p>\n<p>Cards print on A6 and A7.</p>\n</blockquote>",
    );
    expect(html).toContain(
      "<blockquote>\n<p>Warning</p>\n<p>Glossy paper smudges.</p>\n</blockquote>",
    );
  });

  test("gives footnotes as a numbered list, without their way back", async () => {
    await show(github);
    selectDocument();

    const { html } = copy();

    expect(html).toContain(
      "<ol>\n<li>\n<p>200 g/m² or heavier. </p>\n</li>\n<li>\n<p>Check the tray's limit first. </p>\n</li>\n</ol>",
    );
    expect(html).toContain("matte paper.<sup>1</sup> Most printers");
    expect(html).not.toContain("↩");
  });

  test("gives ticked and unticked task items as ☑ and ☐, as the reader left them", async () => {
    await show(readme);
    // The file ticks "Update the changelog"; the reader unticks it and ticks "Tag the version".
    const boxes = view().querySelectorAll<HTMLInputElement>(
      "input.task-list-item-checkbox",
    );
    boxes[0]?.click();
    boxes[1]?.click();
    selectDocument();

    const { html } = copy();

    expect(html).toContain(
      "<ul>\n<li>☐ Update the changelog</li>\n<li>☑ Tag the version</li>\n<li>☐ Publish the notes</li>\n</ul>",
    );
    expect(html).not.toContain("<input");
  });

  test("gives the task items a selection reaches into, by their ticks", async () => {
    await show(readme);
    // From just before the second task item's box, as a drag from its left edge starts.
    const [, second] = view().querySelectorAll("input.task-list-item-checkbox");
    if (!second) throw new Error("The document has no second task item.");
    select(
      [elementWith("li", "Tag the version"), "Tag"],
      [elementWith("li", "Publish the notes"), "Publish"],
    );
    document.getSelection()?.getRangeAt(0).setStartBefore(second);

    const { html } = copy();

    expect(html).toContain("<li>☐ Tag the version</li>");
    expect(html).toContain("<li>☐ Publish");
    expect(html).not.toContain("Update the changelog");
  });

  test("gives a local image by its file path, and a web image by its URL", async () => {
    await show(imagesAndLinks);
    selectDocument();

    const { html } = copy();

    expect(html).toContain(
      '<img src="file:///C:/lamps/the%20lamp.png" alt="The lamp">',
    );
    expect(html).toContain(
      '<img src="https://example.com/sun.png" alt="The sun">',
    );
    expect(html).not.toContain("image.localhost");
  });

  test("gives an image that doesn't load as its alt text", async () => {
    await show(imagesAndLinks);
    selectDocument();

    const { html } = copy();

    expect(html).toContain("An image on a network share, The shared lamp,");
    expect(html).not.toContain('alt="The shared lamp"');
  });

  test("keeps an image inside its web link", async () => {
    await show(imagesAndLinks);
    selectDocument();

    const { html } = copy();

    expect(html).toContain(
      '<a href="https://example.com/badge"><img src="https://example.com/badge.svg" alt="The badge"></a>',
    );
  });

  test("keeps web and mail links as links", async () => {
    await show(imagesAndLinks);
    selectDocument();

    const { html } = copy();

    expect(html).toContain(
      '<a href="mailto:lamp@example.com">mailto:lamp@example.com</a>',
    );
    expect(html).toContain('<a href="https://example.com/lamp">the web</a>');
  });

  test("gives a link to a heading, a document or a file as its text", async () => {
    await show(imagesAndLinks);
    selectDocument();

    const { html } = copy();

    expect(html).toContain(", a heading, another document and a file.</p>");
    expect(html).not.toContain('href="#');
    expect(html).not.toContain("docs/guide.md");
    expect(html).not.toContain('href="lamp.png"');
  });

  test("gives a table's rows in the order the view shows them", async () => {
    await show(tables);
    const body = view().querySelector("tbody");
    if (!body) throw new Error("The document has no table body.");
    const row = (part: string) => elementWith("tr", part);
    body.replaceChildren(row("Base"), row("Wick"), row("Oil"), row("Glass"));
    selectDocument();

    const { html } = copy();

    const rows = ["Base", "Wick", "Oil", "Glass"].map((part) =>
      html.indexOf(`<td>${part}</td>`),
    );
    expect(rows.every((at) => at >= 0)).toBe(true);
    expect(rows).toEqual([...rows].sort((a, b) => a - b));
  });

  test("keeps a table's header, alignment and links", async () => {
    await show(tables);
    selectDocument();

    const { html } = copy();

    expect(html).toContain('<th align="right">Price</th>');
    expect(html).toContain('<td align="right">12 €</td>');
    expect(html).toContain("<td>Trim it <em>before</em> each lamp is lit</td>");
    // A link to the document's own heading is its text.
    expect(html).toContain("<td>Clean it when the lamp is cold</td>");
  });

  test("keeps the code of a selection within one code block as code", async () => {
    await show(code);
    const block = elementWith("pre", "Says hello");
    select([block, "Says hello"], [block, "name"]);

    const { text, html } = copy();

    expect(html).toBe(
      "<pre><code>Says hello.\nfn main() {\n    let name</code></pre>",
    );
    expect(text).toBe("Says hello.\r\nfn main() {\r\n    let name");
  });

  test("keeps the emphasis a selection is within", async () => {
    await show(tables);
    const emphasized = elementWith("td em", "before");
    select([emphasized, "bef"], [emphasized, "ore"]);

    const { html } = copy();

    expect(html).toBe("<em>before</em>");
  });

  test("escapes the document's text and keeps its non-breaking spaces", async () => {
    await show('<p data-sourcepos="1:1-1:9">a &lt;b&gt; &amp;\u00a0c</p>');
    selectDocument();

    const { html } = copy();

    expect(html).toBe("<p>a &lt;b&gt; &amp;&nbsp;c</p>");
  });
});

describe("Ctrl+C elsewhere than in the document view", () => {
  test("is left to the webview for the Find box", async () => {
    await show(find);
    const box = document.querySelector<HTMLInputElement>(".find-bar input");
    if (!box) throw new Error("The Find box isn't there.");
    box.value = "lamp";
    box.focus();
    box.select();
    // The reader selected text in the document too, before going to the box.
    select(
      [elementWith("p", "stands by"), "lamp"],
      [elementWith("p", "stands by"), "door"],
    );

    const { taken, text, html } = copy(box);

    expect(taken).toBe(false);
    expect(text).toBe("");
    expect(html).toBe("");
  });

  test("is left to the webview with nothing selected", async () => {
    await show(readme);

    expect(copy()).toEqual({ taken: false, text: "", html: "" });
  });

  test("is left to the webview for a selection outside the document view", async () => {
    await show(readme);
    const hint = document.querySelector(".empty-window p");
    if (!hint) throw new Error("The empty window's hint isn't there.");
    const range = document.createRange();
    range.selectNodeContents(hint);
    document.getSelection()?.addRange(range);

    expect(copy().taken).toBe(false);
  });
});

describe("Ctrl+C of images", () => {
  /** The HTML of the document view's one image, copied with everything selected. */
  async function copyImage(src: string, alt = "Lamp"): Promise<string> {
    await show(`<p><img src="${src}" alt="${alt}"></p>`);
    selectDocument();
    return copy().html;
  }

  test("gives an image at the image origin that names a network share as its alt text", async () => {
    const html = await copyImage(
      "http://image.localhost//attacker/share/x.png",
      "Planted",
    );

    expect(html).toBe("<p>Planted</p>");
  });

  test("gives the same network share image of the off-Windows image scheme as its alt text", async () => {
    const html = await copyImage(
      "image://localhost//attacker/share/x.png",
      "Planted",
    );

    expect(html).toBe("<p>Planted</p>");
  });

  test("gives an image at the image origin written with a trailing dot as its alt text", async () => {
    const html = await copyImage(
      "http://image.localhost.//attacker/share/x.png",
      "Planted",
    );

    expect(html).toBe("<p>Planted</p>");
  });

  test("gives an image at the image origin that is not a drive path as its alt text", async () => {
    const html = await copyImage("http://image.localhost/etc/passwd", "Other");

    expect(html).toBe("<p>Other</p>");
  });

  test("gives an image whose path leaves its drive by dots as its alt text", async () => {
    const html = await copyImage(
      "http://image.localhost/C:/../../attacker/x.png",
      "Dots",
    );

    expect(html).toBe("<p>Dots</p>");
  });

  test("gives a local image's percent-encoded path as its file URL, without the saved time", async () => {
    const html = await copyImage(
      "http://image.localhost/C:/lamps/100%25%20sure%20%231%20%C3%A4.png?modified=1700000000000000100",
    );

    expect(html).toBe(
      '<p><img src="file:///C:/lamps/100%25%20sure%20%231%20%C3%A4.png" alt="Lamp"></p>',
    );
  });

  test("keeps a data image as it is", async () => {
    const html = await copyImage("data:image/png;base64,iVBORw0KGgo=", "Dot");

    expect(html).toBe(
      '<p><img src="data:image/png;base64,iVBORw0KGgo=" alt="Dot"></p>',
    );
  });

  test("escapes an alt text and a link's target with quotes and angle brackets", async () => {
    const written =
      '<p><a href="https://example.com/?q=&quot;a&quot;&amp;b=&lt;c&gt;">go</a> ' +
      '<img src="https://example.com/i.png" alt="He said &quot;hi&quot; &lt;b&gt;"></p>';
    await show(written);
    selectDocument();

    const { html } = copy();

    expect(html).toBe(written);
  });

  test("gives a javascript link and a file link as their text", async () => {
    await show(
      '<p><a href="javascript:alert(1)">script</a> and <a href="file:///C:/x.txt">file</a></p>',
    );
    selectDocument();

    const { html } = copy();

    expect(html).toBe("<p>script and file</p>");
  });

  describe("shown by the theme", () => {
    const both =
      '<p><img src="http://image.localhost/C:/logo-dark.png#gh-dark-mode-only" alt="Dark">' +
      '<img src="http://image.localhost/C:/logo-light.png#gh-light-mode-only" alt="Light"></p>';

    /** Hides the image that `alt` names, as document.css does for the theme it isn't for. */
    function hide(alt: string): void {
      const image = view().querySelector<HTMLElement>(`img[alt="${alt}"]`);
      if (!image) throw new Error(`The document has no image ${alt}.`);
      image.style.display = "none";
    }

    test("leaves out the image the theme hides, with its alt text", async () => {
      await show(both);
      hide("Dark");
      selectDocument();

      const { html } = copy();

      expect(html).toBe(
        '<p><img src="file:///C:/logo-light.png" alt="Light"></p>',
      );
    });

    test("leaves out the other image in the other theme", async () => {
      await show(both);
      hide("Light");
      selectDocument();

      const { html } = copy();

      expect(html).toBe(
        '<p><img src="file:///C:/logo-dark.png" alt="Dark"></p>',
      );
    });

    test("keeps both when the theme hides neither", async () => {
      await show(both);
      selectDocument();

      const { html } = copy();

      expect(html).toContain('alt="Dark"');
      expect(html).toContain('alt="Light"');
    });
  });
});

describe("Ctrl+C of a closed section", () => {
  test("leaves out what a closed section doesn't show, keeping its summary", async () => {
    await show(
      "<details><summary>More</summary><p>Hidden body</p></details><p>After</p>",
    );
    selectDocument();

    const { html } = copy();

    expect(html).toBe("<p>More</p><p>After</p>");
  });

  test("gives what an open section shows", async () => {
    await show(
      "<details open><summary>More</summary><p>Shown body</p></details>",
    );
    selectDocument();

    const { html } = copy();

    expect(html).toBe("<p>More</p><p>Shown body</p>");
  });
});

describe("Ctrl+C of a selection within a structure", () => {
  test("wraps two list items in their list", async () => {
    await show("<ul><li>one</li><li>two</li><li>three</li></ul>");
    select(
      [elementWith("li", "one"), "one"],
      [elementWith("li", "two"), "two"],
    );

    const { html } = copy();

    expect(html).toBe("<ul><li>one</li><li>two</li></ul>");
  });

  test("wraps two items of an ordered list in the list, keeping its start", async () => {
    await show('<ol start="3"><li>c</li><li>d</li><li>e</li></ol>');
    select([elementWith("li", "c"), "c"], [elementWith("li", "d"), "d"]);

    const { html } = copy();

    expect(html).toBe('<ol start="3"><li>c</li><li>d</li></ol>');
  });

  test("wraps two table rows in their body and table", async () => {
    await show(
      "<table><thead><tr><th>Name</th></tr></thead><tbody><tr><td>alpha</td><td>beta</td></tr><tr><td>gamma</td><td>delta</td></tr><tr><td>eta</td><td>theta</td></tr></tbody></table>",
    );
    select(
      [elementWith("td", "alpha"), "alpha"],
      [elementWith("td", "delta"), "delta"],
    );

    const { html } = copy();

    expect(html).toBe(
      "<table><tbody><tr><td>alpha</td><td>beta</td></tr><tr><td>gamma</td><td>delta</td></tr></tbody></table>",
    );
  });

  test("wraps two cells of one row in their row, body and table", async () => {
    await show(
      "<table><tbody><tr><td>alpha</td><td>beta</td><td>gamma</td></tr></tbody></table>",
    );
    select(
      [elementWith("td", "alpha"), "alpha"],
      [elementWith("td", "beta"), "beta"],
    );

    const { html } = copy();

    expect(html).toBe(
      "<table><tbody><tr><td>alpha</td><td>beta</td></tr></tbody></table>",
    );
  });

  test("wraps the header and a row in their table", async () => {
    await show(
      "<table><thead><tr><th>Name</th></tr></thead><tbody><tr><td>alpha</td></tr></tbody></table>",
    );
    select(
      [elementWith("th", "Name"), "Name"],
      [elementWith("td", "alpha"), "alpha"],
    );

    const { html } = copy();

    expect(html).toBe(
      "<table><thead><tr><th>Name</th></tr></thead><tbody><tr><td>alpha</td></tr></tbody></table>",
    );
  });

  test("keeps the spans and alignment of cells", async () => {
    await show(
      '<table><thead><tr><th colspan="2">Both</th></tr></thead><tbody><tr><td rowspan="2" align="right">tall</td><td>x</td></tr><tr><td>y</td></tr></tbody></table>',
    );
    selectDocument();

    const { html } = copy();

    expect(html).toBe(
      '<table><thead><tr><th colspan="2">Both</th></tr></thead><tbody><tr><td align="right" rowspan="2">tall</td><td>x</td></tr><tr><td>y</td></tr></tbody></table>',
    );
  });

  test("wraps two paragraphs of a quote in the quote", async () => {
    await show("<blockquote><p>first</p><p>second</p></blockquote>");
    select(
      [elementWith("p", "first"), "first"],
      [elementWith("p", "second"), "second"],
    );

    const { html } = copy();

    expect(html).toBe("<blockquote><p>first</p><p>second</p></blockquote>");
  });

  test("wraps a term and its description in their list", async () => {
    await show("<dl><dt>Wick</dt><dd>Cotton</dd><dt>Oil</dt></dl>");
    select(
      [elementWith("dt", "Wick"), "Wick"],
      [elementWith("dd", "Cotton"), "Cotton"],
    );

    const { html } = copy();

    expect(html).toBe("<dl><dt>Wick</dt><dd>Cotton</dd></dl>");
  });

  test("does not wrap the text within one list item", async () => {
    await show("<ul><li>one two</li></ul>");
    select(
      [elementWith("li", "one"), "one"],
      [elementWith("li", "one"), "one"],
    );

    const { html } = copy();

    expect(html).toBe("one");
  });

  test("wraps the nested list a selection is within, and no further", async () => {
    await show(
      "<ul><li>outer<ul><li>inner one</li><li>inner two</li></ul></li></ul>",
    );
    select(
      [elementWith("li", "inner one"), "inner one"],
      [elementWith("li", "inner two"), "inner two"],
    );

    const { html } = copy();

    expect(html).toBe("<ul><li>inner one</li><li>inner two</li></ul>");
  });
});

describe("Ctrl+C of a very deep document", () => {
  test("copies a document nested deeper than a recursive walk could", async () => {
    await show("<p>x</p>");
    // Deep enough that a walk recursing once per level overflows the stack (it did, at this
    // depth), and shallow enough for happy-dom, which recurses itself too at 8,000 levels.
    const depth = 5_000;
    // Built from the top down: happy-dom recurses when a deep tree is attached whole.
    let innermost: Element = view();
    innermost.replaceChildren();
    for (let level = 0; level < depth; level++)
      innermost = innermost.appendChild(document.createElement("blockquote"));
    innermost.appendChild(document.createElement("p")).textContent = "deep";
    selectDocument();

    const { taken, html } = copy();

    expect(taken).toBe(true);
    expect(html).toBe(
      `${"<blockquote>".repeat(depth)}<p>deep</p>${"</blockquote>".repeat(depth)}`,
    );
  });
});
