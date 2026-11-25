# 01: Print, and PDF through the print dialog

**What to build:** The reader can **Print** the shown document, on paper or into a PDF, through WebView2's Print Preview, laid out for paper in GitHub's light colors.

- **How it starts:** Ctrl+P, and "Print..." in the right-click menu, after Paste in the Edit / Open... / Paste group. No title bar button. It can't run in the empty window, nor while only a **Wait note** is in view (a document on its way after another prints the one in view).
- **The dialog:** WebView2's Print Preview (the Browser kind), which previews the pages and offers printers, "Save as PDF" and "Microsoft Print to PDF". PDF is only a destination there: no separate Save as PDF command, so Downlight writes no file of its own. The preview covers the whole webview, the drawn **Title bar** included, until it closes; that is accepted. `window.print()` from the page should open it (WebView2's browser keys are off, `platform/windows.rs`, but that disables only Ctrl+P's own handling, not the call); if it doesn't, the app opens it with `ICoreWebView2_16::ShowPrintUI` in the Windows module.
- **What paper looks like:** GitHub's light colors (github-markdown-css's light values, blue links, GitHub's checkboxes), whatever the **Theme**, **Backdrop** or **Accent color**; zoom and **Full width** don't apply. None of what Downlight adds on screen prints: the title bar, the find bar, Find's matches (CSS Highlights), notices, the wait note, the selection bubble, copy buttons. The theme reaches the page only as `prefers-color-scheme`, so the print styles set the light values themselves.
- **Laid out for paper:** long code lines wrap; a wide table is a real table at the page's width with wrapping cells (github-markdown-css makes tables scroll, which on paper clips them); no page break inside an image, a table row or a short code block; a heading never ends a page; link addresses are not printed after links.
- **The showing as it is:** **Ticks** (the file's and the reader's) and a **Sort** print as shown. Every `<details>` is opened for printing and closed again after, so the screen is unchanged once the dialog closes.
- **Page edges:** while printing, `document.title` is the file's name (it is always "Downlight" now), so Save as PDF suggests `a.pdf`. CSS page margin boxes print the file's name at the top of each page and "3 / 12" at the bottom; the browser's own header and footer (date, "Downlight", `http://tauri.localhost/`) stay off. If WebView2 doesn't support margin boxes, or they don't replace its header and footer, fall back to the title alone and say so in the ticket.
- **ADR 0003's skipped blocks:** the print styles give the document's blocks `content-visibility: visible`, so blocks never laid out print at their real size, not at their 3em estimate.
- **Large documents:** no cap and no wait note: every document prints, however large. The time to the preview is measured and recorded; a follow-up (a warning above a size, say) is a ticket of its own if the numbers are bad.
- **Not shared with export-html/01:** Export's grilling (2026-11-25) gave the exported file both themes and GitHub's scrolling code and tables, and it leaves out Downlight's additions by stripping a clone of the document, so the two share only github-markdown-css. The print stylesheet is Print's alone.

Rejected: Windows' own print dialog (no preview); a separate "Save as PDF..." command through `PrintToPdf` and a save dialog (more app code, and Downlight writing a file, export-html/01's question); paper following the file instead of the screen for ticks and a sort (it drops what the reader sees in the preview just before printing; Copy as Markdown copies the file, paper is what the reader sees); collapsed sections printed collapsed (paper can't open them).

Spec: none; grilled on 2026-11-25 instead (`/grill-with-docs`). From the v1 spec's Out of Scope, v1.x ("printing and PDF through the webview's print dialog").

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A Print command with Ctrl+P, in the right-click menu after Paste, that can't run in the empty window or with only a wait note in view; it opens WebView2's Print Preview.
- [ ] Checked in the release build: the Print Preview offers "Save as PDF" (and "Microsoft Print to PDF"), and Save as PDF suggests the file's name.
- [ ] A print stylesheet: GitHub's light colors in either theme and with any backdrop or accent color; zoom and Full width ignored; nothing of Downlight's on screen printed; wrapped code; real tables; the page-break rules; no link addresses.
- [ ] Before printing every `<details>` opens and `document.title` becomes the file's name; after printing both are as they were. Ticks and a sort print as shown.
- [ ] Margin boxes with the file's name at the top and "page / pages" at the bottom, replacing the browser's header and footer; or the fallback, noted here.
- [ ] Skipped blocks print at their real size: hand check on the 1 MB test document (`npm run documents`) that a PDF of it has the same pages as one made after scrolling through it.
- [ ] Measured with the timing switch on the 1 MB and 100 MB documents (`npm run documents -- large`): the page's long tasks when Print runs, and the time until the preview shows its pages; recorded under Results here.
- [ ] Page tests (written first, Vitest): Ctrl+P and the menu entry run Print, at its place in the menu, and not in the empty window or over a wait note; `beforeprint` opens the closed `<details>` and sets the title to the file's name, `afterprint` closes those it opened and gives the title back.
- [ ] Hand check: print to PDF a document with code, a wide table, images, ticked and unticked items (from the file and the reader), a sorted table and a closed `<details>`, in the dark theme with Mica and a non-amber accent color, zoomed in, with Find's matches showing.
- [ ] `npm run check` passes.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (v1.x). mdhero has print styles and PDF (`.scratch/mdhero-comparison/features.md`).

**2026-11-25.** Grilled with the repo owner; decisions above. The glossary has its **Print** entry. No ADR: none of it is hard to reverse.
