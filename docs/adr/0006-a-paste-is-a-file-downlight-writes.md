# A paste is a file Downlight writes

Paste writes the clipboard's text to a file in Downlight's own temporary folder (`%TEMP%\Downlight\Pasted\`), and the window shows that file as it shows any document. We chose this because every part of the app knows a document by its file: rendering and link resolution work against the file's folder, History keeps paths, windows are found by a document's key, and watching, Reload, Edit and the title bar's reveal all need the file. With a file, all of them work for a paste unchanged, and the reader can reveal the file from the title bar and keep a copy ([Paste Markdown](../../.scratch/paste-markdown/spec.md)).

## Considered options

- **Text with no file**, a new concept beside Document, held in memory. Nothing reaches the disk, but the path turns optional through the document table, History, the title, Edit, watching, notices, link resolution and the core's render, and every later feature (Copy as Markdown, Print, Recent documents) gets a branch for a document without one.

## Consequences

- Downlight now writes files, though never the reader's own: the clipboard's text is on disk while a window can return to it.
- A pasted file is deleted once no open window shows it or has it in its History, and the folder is emptied when Downlight starts, to clear up after a crash. A reader who edits a paste in an editor and closes its window loses the file under the editor; keeping a paste means saving a copy.
- Relative links and images in a paste resolve against the temporary folder, so they don't load or lead anywhere.
- Features that list or remember documents (Recent documents) skip pasted files.
