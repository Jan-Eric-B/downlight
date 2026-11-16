# 11: A missing file's notice opens its folder

**What to build:** A notice that names a file shows the file's name in bold and its folder as a link apart from the rest of the text, and when the file is missing but its folder is there, a click on the folder opens it in Explorer. For example: "Can't find **delete-me.md** in [C:\Users\me\docs](#)."

Spec: [Window design](../spec.md)

**Blocked by:** 04 (The logo and the file name in the title bar)

**Status:** ready-for-agent

- [ ] Every notice that names a file shows its name in bold and its folder after it, instead of the whole path as plain text. This covers at least following a link to a missing document (`FollowError::NotFound` in `app/src/links.rs`) and clicking the title bar's file name when the file is gone (`RevealError::NotFound` in `app/src/title_bar.rs`); find the other errors and notices that name a path (open, drop, launch, reload) and give them the same shape.
- [ ] When the file is missing and its folder exists, the folder is a link in the link role's color (`--accent-link`), and a click opens the folder in Explorer. When the folder is gone too, it is plain text and nothing is clickable.
- [ ] A notice with a link stays while the pointer is on it or it has the focus, and goes its usual few seconds after that. The link is reachable with Tab and has an accessible name.
- [ ] A long path breaks only at its backslashes, never inside a name (today "delete-me.md" breaks after "delete-").
- [ ] The page never sends the app a path of its own choosing to open: the folder to open is one the app named in the notice, and the app checks it (an existing folder) before opening it. Opening uses the same platform call as a link's "open a folder" action.
- [ ] The notice's shape is typed in `page/src/app.ts`, with contract samples (`npm run fixtures`) for the message and for the errors that carry a file.
- [ ] Page tests: the name is bold and the folder a link when the folder exists, plain when it doesn't; a click on the folder sends the request; the notice stays while hovered or focused.
- [ ] App tests: each missing-file error carries the name, the folder and whether the folder exists; the open-folder command refuses anything that isn't an existing folder.
- [ ] Checked by hand: follow a link to a deleted document, and click the title bar's name after deleting the shown file; each time the notice's folder opens Explorer there. With the folder deleted too, the folder is plain text.

## Comments

**2026-11-16, from ticket 04's hand check.** The reader asked for a way to open the folder of a missing file and for the path to stand apart from the rest of a notice. Decided with them:

- The folder is a link inside the notice, not a button, and the notice doesn't take a click anywhere else: a notice vanishes after a few seconds, so it holds at most one small action, and it stays while that action is in reach.
- No "Create" for a missing file. Downlight is reading-only (glossary: changing a file happens in another program), and a created empty file would show an empty window; creating files is the Editor's job.
- The look: the file name in bold, the folder as a link, e.g. "Can't find **delete-me.md** in C:\\...\\handcheck."
