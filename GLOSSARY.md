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

**Tick**:
A reader's mark on a task list item. It belongs to the window and is never written to the file.
_Avoid_: Check, complete, done

**Reload**:
Showing the document again after its file changed, at the same reading position.
_Avoid_: Refresh

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
