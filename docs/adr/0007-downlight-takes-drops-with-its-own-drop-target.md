# Downlight takes drops with its own drop target

On Windows, Tauri's native drag and drop is off, and the app's Windows module registers a drop target of its own on the webview's windows, as wry does, which takes a drag's file paths and, when it has none, its plain text. We chose this so text dragged from another program can be shown (**Paste**), while a dropped file still arrives with its path and the page still never carries the dropped text: wry's drop target, which Tauri's native drop puts in place of WebView2's own, reads file paths only and refuses every other drag, and its events carry nothing else ([Paste Markdown](../../.scratch/paste-markdown/spec.md)).

## Considered options

- **HTML5 drag and drop in the page**, with Tauri's native drop off so WebView2's own drop target is back. The page gets the text, but a dropped file comes as a `File` with no path: the page would post it to the app with WebView2's `postMessageWithAdditionalObjects`, the app would read its path through a WebView2 message handler of its own beside Tauri's IPC, and the dragged text, which can be megabytes, would cross the IPC, which Paste avoids by never sending text from the page.
- **Keeping wry's drop target**: no text drops.

## Consequences

- File drops are Downlight's own code now: reading `CF_HDROP`, and the shell item array for paths Explorer can't put there (260 characters or longer), copied from wry's approach. A fix to wry's drop target no longer reaches Downlight by itself.
- The drop target covers the whole webview, so a text field never takes a drop: text dropped onto the Find box shows as a document, as a dropped file does.
- A drop is always taken as a copy, so an editor that moves dragged text never loses it.
- Other systems keep Tauri's native drop: file drops only, until their ports.
