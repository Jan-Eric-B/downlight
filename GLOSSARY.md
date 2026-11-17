# Downlight

A small, fast viewer for Markdown files, meant to be the default app that opens them. Reading only; changing a file happens in another program.

## Language

**Document**:
The Markdown file a window shows.
_Avoid_: Page, note

**Editor**:
The program that Edit opens a document in. It is never Downlight itself.
_Avoid_: Handler, default app

**Text file program**:
The program the system opens plain text (`.txt`) files with. Edit opens the document in it when the reader chose no editor, or the chosen one is gone; Edit skips it when it is Downlight.
_Avoid_: Handler, default editor

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

**History**:
The documents a window showed and the headings a link scrolled it to, in the order the reader visited them, which Back and Forward step through, each step returning to the reading position the view had there. It belongs to its window, isn't saved and is gone when the window closes. Going somewhere new after Back drops what Forward would have gone to.
_Avoid_: Navigation, back stack

**Notice**:
A short message a window shows the reader for a few seconds, such as why a dropped file didn't open or why the document's file isn't watched. One that comes before the window's page is ready is held until the page first asks for its document.
_Avoid_: Toast, alert, popup

**Reading position**:
The block at the top of the view, kept as the line number it starts on. Reload, and Back and Forward, return to it, and Find's first current match is the first one from it on.
_Avoid_: Scroll position, anchor

**Showing**:
One time a window was given a document. Given the same document again after another, the window has a new showing; Back and Forward give it back the showing it had, so it returns to where it was. The page names the showing its view shows when it asks for its document or follows a link, and the app names the showing a message about the document is for, so that one about a document the window has left changes nothing.
_Avoid_: Visit, session, generation

**Rendered document**:
A document's file turned into the sanitized HTML the page shows. Only the core makes one.
_Avoid_: Output, HTML string

**Wait note**:
What a window shows while its document takes longer than half a second to arrive, as a large one does: "Showing **a.md** (100 MB) ...", the file's name and size, until the document replaces it. A reload shows none: the document in view stays until its new render replaces it. Every document is shown, however large.
_Avoid_: Spinner, loading screen, progress

**Highlight limit**:
How much code a document has highlighted. A code block that is too large or has a line too long shows as plain code, as GitHub shows a large file, and so does every block after the document's code has used up the limit, in size or in time.
_Avoid_: Highlight budget

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
An image in a document whose file is on this computer. The page loads it through Downlight's own image scheme, never from the file system directly. An image on a network share is never a local image and doesn't load, nor is one whose path goes through a symbolic link to a network share.
_Avoid_: Asset, file image

**Network share**:
A folder that another computer, its host, shares, named by a path such as `\\host\share`. Looking at anything on one connects to the host, and Windows may sign in there as the reader, so Downlight never loads an image from one, and following a link that leads to one, by its path or through a symbolic link, first asks the reader, naming the host, unless the shown document is on a share of that same host.
_Avoid_: UNC path, network drive, server

**Link target**:
Where a link in a document points, as the document writes it (its `href`). It comes from the document, so it is untrusted until link resolution turns it into a link action.
_Avoid_: URL, href

**Link action**:
The one thing following a link does, as the core's link resolution returns it: show a document (optionally at a heading), scroll to a heading, reveal a file, open a folder, hand a URL to the system, or nothing.
_Avoid_: Navigation, link handler

**Title bar**:
The strip at the top of a window with the logo, the document's file name and the window's buttons. The page draws it (ADR 0005); the window's title that Alt+Tab and the taskbar show is a separate thing.
_Avoid_: Header, toolbar, caption

**Accent color**:
The color scale, in seven shades, that marks what Downlight adds to a document: links, the selection, Find's matches, ticks and switched-on buttons. Amber unless the reader picks Windows' own accent color. The icon always stays amber.
_Avoid_: Theme color, highlight color, brand color

**Theme**:
Light or dark. It follows Windows unless the reader picks one in the settings.
_Avoid_: Mode, color scheme

**Backdrop**:
The material Windows draws behind a window, showing through wherever the page draws no background: Mica, Mica Alt, Acrylic, or none, which means a solid background.
_Avoid_: Background, material, transparency

**Full width**:
The document filling the window's width instead of GitHub's reading width. One choice for every window, kept in the settings.
_Avoid_: Fixed width, wide mode

**Active window**:
The Downlight window the reader was last in, while it is open. A new window opens down and to the right of it, at the size of the last window that closed.
_Avoid_: Current window, focused window

**Keep on top**:
A window staying above all others. It belongs to one window and is gone when the window closes.
_Avoid_: Pin, always on top

**Settings**:
The reader's choices that every window shares (editor, zoom, theme, backdrop, accent color, Full width), kept in one file and changed from the settings flyout under the title bar's gear.
_Avoid_: Preferences, options, config
