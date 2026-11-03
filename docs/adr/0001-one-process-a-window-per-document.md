# One process, a window per document

Downlight runs as a single process. Every document opened from the file manager, the command line or a Ctrl+click on a link gets its own window in that process, and opening a document that is already open brings its window to the front. There are no tabs. We chose this because each process builds its own system webview environment, which cost about half a second per first window in an earlier WebView2 spike, while a further window in a running process reuses the one already warm; it also lets `downlight <file>` in a script hand the file to the running process and return at once.

## Considered options

- **A process per window.** Isolates crashes, but pays the webview's startup cost for every window.
- **Tabs in one window.** Not wanted for v1. A window that holds several documents stays possible later, since all windows already live in one process.

## Consequences

- One crash closes every open window.
- The process exits when its last window closes; there is no tray icon or background process.
