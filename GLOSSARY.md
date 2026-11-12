# Downlight

A small, fast viewer for Markdown files, meant to be the default app that opens them. Reading only; changing a file happens in another program.

## Language

**Document**:
The Markdown file a window shows.
_Avoid_: Page, note

**Editor**:
The program that Edit opens a document in. It is never Downlight itself.
_Avoid_: Handler, default app

**Find**:
Looking for text in the shown document, and only there.
_Avoid_: Search

**Current match**:
The one match of Find that Enter and F3 step on from, scrolled into view and highlighted apart from the others. When the reader starts typing, it is the first match at or after the reading position.
_Avoid_: Active match, selected match

**Tick**:
A reader's mark on a task list item. It belongs to the window and is never written to the file.
_Avoid_: Check, complete, done

**Reload**:
Showing the document again after its file changed, at the same reading position.
_Avoid_: Refresh

**Reading position**:
The block at the top of the view, kept as the line number it starts on. Reload, and Back and Forward, return to it, and Find's first current match is the first one from it on.
_Avoid_: Scroll position, anchor

**Rendered document**:
A document's file turned into the sanitized HTML the page shows. Only the core makes one.
_Avoid_: Output, HTML string

**Line number**:
The line of the file where a rendered block starts, shown in the margin. The rendered page has no line numbers of its own.
_Avoid_: Row, source line

**Overview ruler**:
The strip beside the scroll bar that marks where headings and Find matches are in the document.
_Avoid_: Minimap, document map

**First paint**:
The moment the first screen of rendered document text is visible after a file is opened from the file manager. An empty or half-built window doesn't count.
_Avoid_: Startup time, load time

**Local image**:
An image in a document whose file is on this computer. The page loads it through Downlight's own image scheme, never from the file system directly. An image on a network share is never a local image and doesn't load.
_Avoid_: Asset, file image

**Link target**:
Where a link in a document points, as the document writes it (its `href`). It comes from the document, so it is untrusted until link resolution turns it into a link action.
_Avoid_: URL, href

**Link action**:
The one thing following a link does, as the core's link resolution returns it: show a document (optionally at a heading), scroll to a heading, reveal a file, open a folder, hand a URL to the system, or nothing.
_Avoid_: Navigation, link handler
