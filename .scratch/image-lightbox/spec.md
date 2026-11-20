# Image lightbox

Status: ready-for-agent

## Problem Statement

The documents I read are full of screenshots and diagrams, and the reading width shrinks most of them until their text is too small to read. To see one properly today I have to find its file and open it in another program, or widen the window and switch on Full width, which changes the whole document's layout for one picture. A small image, such as an icon or a pixel-art sprite, I can't make bigger at all. In a browser on GitHub I can at least open the image on its own; in Downlight a click on an image does nothing.

## Solution

**Lightbox**: a click on an image in the document shows it on its own over the document, fitted to the window but never larger than its full size (one image pixel per screen pixel, whatever Windows' scaling and Downlight's zoom). The wheel zooms it around the pointer, larger or smaller, as far as 16 times its full size, so a small image can always be made bigger; double-click switches between fitted and full size, and dragging pans it. The arrow keys, PageUp and PageDown, Home and End step to the document's other images. The file name at the top reveals a local image's file in Explorer or opens a remote image in the browser; at the bottom are "3 of 12" and the alt text. Esc, the Close button or a click beside the image closes it, and the document is where the reader left it. An image inside a link follows its link, as on GitHub, unless the link points at that same image; Alt+click or "Show image" in the right-click menu opens any image, linked or not. A local animated image that has played as often as its file says gets a "Play again" button.

## User Stories

1. As a reader, I want a click on an image in the document to show it large over the document, so that I can read a screenshot the reading width shrinks.
2. As a reader, I want the image to open fitted to the window, so that I see all of it at once.
3. As a reader, I want an image to open no larger than its full size, so that a small image isn't blown up blurry before I ask for it.
4. As a reader, I want full size to be one image pixel per screen pixel, so that a screenshot taken on this screen comes back exactly as it was taken, sharp at any Windows scaling.
5. As a reader, I want Downlight's zoom to leave the lightbox's sizes alone, so that the document's zoom never makes an image soft.
6. As a reader, I want an SVG to fill the window, so that a diagram without a pixel size is shown as large as it can be.
7. As a reader, I want the wheel to zoom the image in and out around the pointer, so that I zoom into the part I'm looking at.
8. As a reader, I want to zoom a small image well beyond its full size, so that an icon or a sprite can always be made bigger.
9. As a reader, I want zooming to stop at 16 times full size and at a tenth of the fitted size, so that the image never gets lost at an absurd size.
10. As a reader, I want an image zoomed beyond its full size to be smoothed, so that it looks as images usually do when enlarged.
11. As a reader, I want double-click to switch between fitted and full size, so that one gesture takes me to the image's real pixels and back.
12. As a reader, I want to drag an image that is larger than the window, so that I can look at every part of it.
13. As a reader, I want a drag that pans never to count as a click beside the image, so that panning never closes the lightbox.
14. As a reader, I want Ctrl+wheel and a touchpad's pinch to zoom the image while the lightbox is open, so that the zoom I'm used to acts on what I'm looking at.
15. As a reader, I want Ctrl+Plus and Ctrl+Minus to zoom the image while the lightbox is open, so that the keyboard can zoom it too.
16. As a reader, I want Ctrl+0 to fit the image again while the lightbox is open, so that I can get back to the whole image at once.
17. As a reader, I want zooming in the lightbox never to change Downlight's zoom setting, so that the document behind it and my other windows stay as they were.
18. As a reader, I want a zoom percentage to show briefly while I zoom, so that I know how large the image is.
19. As a reader, I want Right and Left to step to the next and previous image while the image fits the window, so that I can look through a document's screenshots in a row.
20. As a reader, I want the arrow keys to pan an image zoomed larger than the window, so that the keyboard can look around it, as in Windows Photos.
21. As a reader, I want PageDown and PageUp to step whatever the zoom, so that I can always move on.
22. As a reader, I want Home and End to go to the first and last image, so that I can jump to either end.
23. As a reader, I want stepping to stop at the first and the last image, so that I know when I've seen them all.
24. As a reader, I want Previous and Next buttons at the sides, each hidden at its end, so that the mouse can step too.
25. As a reader, I want each image I step to to start at its opening size, so that a zoom on one never leaves the next oddly sized.
26. As a reader, I want the steps to go through every image of the document in its order, linked or not, so that stepping shows them all.
27. As a reader, I want an image that failed to load, one on a network share and one the theme hides to be left out of the steps and to open nothing, so that the lightbox never shows a broken image.
28. As a reader, I want a `<picture>` to show the source the document shows, so that the lightbox shows the same picture, in the right theme.
29. As a reader, I want "3 of 12" at the bottom, so that I know where I am among the images.
30. As a reader, I want the image's alt text at the bottom, so that I read what the author says about it.
31. As a reader, I want the image's file name at the top, so that I know which file I'm looking at.
32. As a reader, I want the full path or URL as the name's tooltip, so that I can tell two files of the same name apart.
33. As a reader, I want a click on a local image's name to reveal its file in Explorer, selected, so that I can open it in an image editor.
34. As a reader, I want a click on a remote image's name to open it in my browser, so that I can save it or see where it lives.
35. As a reader, I want a remote image's name to be the last part of its URL's path, or its host when the path is empty, so that the name is short and familiar.
36. As a reader, I want an image embedded in the document to show no name, so that the lightbox doesn't name a file that doesn't exist.
37. As a reader, I want the lightbox to stay open while Explorer or the browser comes forward, so that I can return to it.
38. As a reader, I want a Notice when the local file is gone by the time I click its name, so that the click never silently fails.
39. As a reader, I want a Close button at the top right, so that the mouse has a way out I can see.
40. As a reader, I want Esc to close the lightbox, so that the key I try first works.
41. As a reader, I want a click on the backdrop beside the image to close it, so that a click away goes back to the document.
42. As a reader, I want a click on the image itself to do nothing, so that a single click while looking never closes it.
43. As a reader, I want closing to leave the document where it was before I opened the lightbox, so that I am back where I was reading, not at the last image I stepped to.
44. As a reader, I want the document not to scroll behind the lightbox, so that the wheel only zooms the image.
45. As a reader, I want the lightbox's backdrop to be the theme's background, nearly opaque, so that the light theme stays light.
46. As a reader, I want the lightbox to cover the document area below the title bar, so that I can still move, minimize and close the window.
47. As a reader, I want the lightbox to cover the whole screen in full screen, so that full screen shows only the image.
48. As a reader, I want the title bar's buttons to keep working while the lightbox is open, so that it never traps the window.
49. As a reader, I want commands about the document (Find, Open, Back, Forward, Paste, Edit) to close the lightbox and then run, so that a command does what it says without a step in between.
50. As a reader, I want Keep on top, Full screen and the settings flyout to leave the lightbox open, so that changing the window doesn't lose my image.
51. As a reader, I want Esc to close the settings flyout first when it is open over the lightbox, so that Esc closes what is nearest first.
52. As a reader, I want Esc to close the lightbox before Find, Stop and full screen, so that leaving the image doesn't also close Find or leave full screen.
53. As a reader, I want an Esc that cancels an IME's composition, or comes with a modifier held, to leave the lightbox open, so that Esc does only what it means.
54. As a reader, I want an image inside a link to follow its link when clicked, as on GitHub, so that badges and linked images keep working.
55. As a reader, I want a link that points at its own image, such as `[![shot](shot.png)](shot.png)`, to open the lightbox instead, so that an author's "click for the full image" link shows the image, not Explorer.
56. As a reader, I want the same for a remote image linked to itself, so that a README's linked screenshots work the same wherever they live.
57. As a reader, I want Alt+click on any image, linked or not, to open the lightbox, so that I can always see a linked image large.
58. As a reader, I want "Show image" in the right-click menu on an image, so that I can find the lightbox without knowing Alt+click, and open a linked image.
59. As a reader, I want the pointer over an image that opens the lightbox to show that it zooms, so that I know a click will do something.
60. As a keyboard reader, I want unlinked images to be tab stops that Enter or Space opens, so that I can open the lightbox without a mouse.
61. As a keyboard reader, I want Shift+F10 or the menu key on a focused image or link to offer "Show image", so that the keyboard can open a linked image too.
62. As a keyboard reader, I want the focus to move into the lightbox when it opens and to stay inside it while it is open, so that Tab never wanders into the document behind it.
63. As a keyboard reader, I want the focus to return to the image I opened the lightbox from when it closes, even after stepping, so that I continue from where I was.
64. As a keyboard reader, I want the image's name to be a button in the lightbox's Tab order, so that I can reveal or open the file without a mouse.
65. As a screen reader user, I want the lightbox announced as a dialog named by the image's alt text, or its file name when it has none, so that I know what opened.
66. As a screen reader user, I want the buttons named for what they do (Close, Previous image, Next image, Play again), so that I hear what each does.
67. As a reader, I want the right-click menu inside the lightbox to offer only Close, so that it offers nothing that doesn't apply to the image.
68. As a reader, I want a Reload while the lightbox is open to keep it on the same image when the reloaded document still has it, so that an edit elsewhere in the file doesn't take the image away.
69. As a reader, I want the lightbox to close on a Reload when its image is gone from the document, so that it never shows an image the document no longer has.
70. As a reader, I want "3 of 12" to count the reloaded document's images, so that the count stays true.
71. As a reader, I want the lightbox to close when the window shows another document, by Back, Forward, a drop, the command line or Paste, so that it never shows an image of a document I've left.
72. As a reader, I want a fitted image to fit the window again when I resize it, so that it stays whole.
73. As a reader, I want a zoomed image to keep its zoom when I resize the window, so that resizing doesn't undo my zoom.
74. As a reader on a touch screen, I want a pinch to zoom the image and a swipe to step, so that the lightbox works without a mouse or keyboard.
75. As a reader, I want an animated image to start from its first frame whenever the lightbox shows it, so that I see the whole animation.
76. As a reader, I want the document's copy of an animated image to play on undisturbed, so that the lightbox changes nothing in the document.
77. As a reader, I want a "Play again" button beside "3 of 12" once a local animated image has played as often as its file says, so that one that plays only once can be watched again.
78. As a reader, I want Space to play it again while the lightbox has the focus and the button shows, so that the keyboard can replay it too.
79. As a reader, I want no "Play again" on an animated image that loops forever, so that the button shows only when it means something.
80. As a reader, I want the lightbox to add no network request, so that opening a remote image doesn't load it again.
81. As a reader with a large document, I want opening the lightbox to answer within 100 ms on the 1 MB test document, so that a document with hundreds of images doesn't hold up the window.
82. As a reader who prefers reduced motion, I want the lightbox to open, close and step without animating, so that it follows my system's setting.
83. As a reader, I want the lightbox to belong to its window and the moment, so that nothing about it is saved or shown in other windows.

## Implementation Decisions

- **Glossary.** The term is **Lightbox** (`GLOSSARY.md`, added during the grilling), with its full size defined there. Avoid image viewer, overlay, modal, preview (the Explorer preview pane's word) and zoom (the setting's word).
- **A page module for the lightbox.** It owns the dialog, its state (which image of which list it shows, its zoom and pan) and its input. It starts with the page's other per-window modules, gets the view's shown document as Find does, and its signal removes its listeners for the tests. Its interface to the rest of the page is small: open on an image, whether it is open (for Esc's order and the commands), close, and keep or close across a new render.
- **Which images.** The list of images is built when the lightbox opens and again after a Reload: every `<img>` in the shown document in document order whose image has loaded and which the theme doesn't hide (`#gh-dark-mode-only` and `#gh-light-mode-only`, and the sources of a `<picture>` through its `currentSrc`). An image without a source (one on a network share, which rendering leaves without one) or one that failed to load is never in it. Building the list reads no layout, so it never forces the layout of a block that ADR 0003 skips; the loaded state and the natural size are known without it.
- **Opening.** A click (button 0, no modifier) on an image outside a link opens the lightbox on it. A click on an image inside a link goes on to the link's handling, unless the link is marked as pointing at its own image, which then behaves as an unlinked image (opened by a click, Enter or Space, no link target bubble). Alt+click on any image opens it, linked or not; the link handling already ignores Alt+click. The pointer over an image that a plain click opens is the zoom-in cursor.
- **The core marks a link that points at its own image.** At render time, after sanitizing, the core marks a link whose only content is an image and whose target names the same file or URL as the image's source, read the way both are already read (the shared reading of link targets and image sources), so no file system lookup is needed. The mark is an attribute only the core writes; the sanitizer strips any the document brings. This is a rendering change: a fixture gets such a link, and `npm run fixtures` writes the HTML again.
- **"Show image" in the right-click menu.** A new command, available only when the menu was opened on an image of the list (by the pointer, or by Shift+F10 or the menu key on a focused image or a link around one), which opens the lightbox on that image. Inside the lightbox the menu has only Close. The menu entries are the page's, so the app's menu needs no new shape.
- **Keyboard opening.** Unlinked images, and images in self-linked links, get a tab stop, and Enter or Space on one opens the lightbox. A linked image keeps its link's tab stop.
- **Sizes.** Full size is the image's natural size divided by the screen pixels per CSS pixel, so one image pixel is one screen pixel; the lightbox sits outside the document view, so Downlight's zoom (CSS `zoom` on the view) never applies to it. The fitted size is the largest that fits the lightbox's area. An image opens at the smaller of the two; an SVG, which has no full size, opens at the fitted size. Zoom runs from a tenth of the fitted size to 16 times full size (for an SVG, 16 times fitted), and the browser's smoothing is kept above full size.
- **Zoom input.** The wheel zooms around the pointer. While the lightbox is open, the commands' Ctrl+wheel handling (which a touchpad's pinch also sends) and Zoom in, Zoom out and Reset zoom act on the image instead of the setting; Reset zoom fits it. Double-click switches between fitted and full size around the pointer. A touch pinch zooms. The zoom percentage, relative to full size, shows briefly after each change.
- **Panning.** Dragging pans an image larger than the area, and the arrow keys pan it while it is. A pointer that moved beyond a small threshold between down and up is a drag, never a click.
- **Stepping.** Right and Left step while the image fits the area; PageDown and PageUp always step; Home and End go to the first and last; a touch swipe steps. Stepping stops at the ends. Each image stepped to opens at its opening size.
- **The commands while it is open.** Every command that changes or leaves the document (Find and its next and previous, Open, Back, Forward, Reload by key, Paste, Edit, Close) closes the lightbox first and then runs. Keep on top, Full screen, Full width, Minimize, Maximize and the settings flyout leave it open, and the zoom commands act on the image as above.
- **Esc.** The lightbox's own Esc listener closes it while it is open, the event isn't composing, no modifier is held, the event isn't already taken and the settings flyout isn't open. The listeners of Find, Stop and full screen each defer to the lightbox while it is open, so the order is the settings flyout, the lightbox, Find, Stop, full screen.
- **Layout.** The lightbox covers the area below the title bar, the Find bar, the Overview ruler and the Counts bubble included, and the whole screen in full screen (where the title bar is hidden and slides in over it as over the document). The title bar keeps working. The backdrop is the theme's background, nearly opaque. The name sits at the top, the Close button at the top right, Previous and Next at the sides (each hidden at its end), and "3 of 12", the alt text and "Play again" at the bottom. Buttons are drawn as the title bar's are, with Fluent icons, and tooltips name their keys. Under reduced motion nothing animates.
- **Closing.** Esc, the Close button, the menu's Close and a click on the backdrop close it. A click on the image does nothing. The document view is never scrolled while it is open or when it closes, and the focus returns to the image it was opened from (or, after a Reload, to that image's counterpart, and to the view when it is gone).
- **Focus and screen readers.** The lightbox is a modal dialog (`aria-modal`), named by the alt text of the image it shows, or by its file name, or "Image" when it has neither. The focus moves into it on opening and Tab cycles inside it.
- **The name and its click.** For a local image the name is the file's name, for a remote one the last segment of its URL's path, decoded, or the host when the path is empty, and a `data:` image has none. The tooltip is the full path or URL. A click asks the app with the image's source as the lightbox shows it; the app hands it to a new core function that returns a **Link action**: a source of the image scheme becomes revealing its file (through the core's existing mapping of the scheme's URLs to files, with its network share rules), an `http` or `https` URL becomes opening the URL, and anything else becomes nothing. The app performs it as it performs a link's, without asking, and a file that is gone gives the Notice a broken link gives. The lightbox stays open.
- **Animated images.** The lightbox shows each image through its own `<img>`, so the document's copy is never touched. For a local image, the page asks the app for its animation when the lightbox shows it. The app reads the file on a background task and passes the bytes to a new core function that reads, from a GIF, a WebP or an APNG, how many times the file says it plays (or that it loops forever, or isn't animated) and how long one play lasts. When it plays a finite number of times, the page starts it from its first frame and shows "Play again" once that time has passed; pressing it, or Space while it shows, starts it again. Whether a new `<img>` for the same URL restarts the animation in WebView2 is a hand check; if Chromium plays every copy of a URL in step, the lightbox's copy of a local image gets its own URL (a query the image scheme ignores), which reads the local file again but adds no network request. A remote image is shown as it is and gets no "Play again", since restarting it would load it again from the network.
- **Reading animations is the core's, from bytes.** Whether the reader is hand-written (the three formats keep their loop count and frame delays in a few chunks) or uses maintained crates (`gif`, `png`, `image-webp`) is decided when it is built, by how much code a crate saves; any crate is added with `cargo add`, checked by cargo-deny and the notices.
- **Reload and new showings.** After a Reload the lightbox, if open, looks in the new render for the image with the same source at the same occurrence of that source; it stays on it, keeping its zoom, with the list rebuilt, or closes when it is gone. A new showing closes it at once.
- **Resize.** A fitted image fits the new area; a zoomed one keeps its zoom and pan, kept within the image's bounds.
- **App contract.** Two new app commands, each taking only an image source and validating it: the image's link action, performed by the app (its answer is the existing link answer), and the animation of a local image (its answer: not animated, loops forever, or plays n times and one play lasts so many milliseconds). Each answer gets a contract sample, the page's command types gain the entries, and only the page's app module calls them.

## Testing Decisions

- A good test drives the page as the reader does (a click, Alt+click, a key, the wheel, a drag, the right-click menu) and checks what the reader can observe: whether the lightbox is open and which image it shows, "3 of 12", the alt text and the name, the focus, the size the page gives the image, the menu's entries, and what the page asks the app. It never checks the module's internals. Test names state the behavior, as `a click on a linked image follows the link` would.
- **The page (Vitest with happy-dom, `fakeTheApp`).** The seam for nearly everything: opening by a click, Alt+click, Enter or Space and "Show image"; a linked image follows its link and a self-linked one opens; which images are in the list (broken, sourceless and theme-hidden ones aren't); the opening size from the natural size and the screen pixels per CSS pixel, and the SVG case; the wheel, Ctrl+wheel and Ctrl+Plus, Minus and 0 zoom the image within the limits and never ask the app to change the zoom setting; double-click; a drag isn't a click; stepping with each key, stopping at the ends, resetting the zoom; the name, its tooltip and its click asking the app; a gone file's Notice; Esc's order with the settings flyout, Find, Stop and full screen, and composing and modifiers; which commands close it first and which leave it; focus in, Tab inside and back to the opening image; the dialog's name; a Reload keeping the same image or closing; a new showing closing it; "Play again" after the animation's time, and none for an image that loops forever or a remote one. happy-dom loads no images and lays nothing out, so the tests give each image its loaded state and natural size and the lightbox its area, as `fakeTheApp` stands in for the app, and check the size the page sets rather than pixels. Prior art: `links.test.ts` (clicks and modifiers on the document's links), `find.test.ts` and `wait-note.test.ts` (Esc's order), `full-screen.test.ts` and `keep-on-top.test.ts` (a module that starts with the page and asks the app), `commands.test.ts` (the right-click menu's entries and the zoom keys and wheel), `reload.test.ts` (what survives a Reload).
- **The core (`cargo test -p downlight-core`).** At its public interface: rendering marks a link that points at its own image (local and remote) and no other, and strips the mark a document brings, checked against a fixture; an image's source gives the right link action (a local image reveals its file, a network share's never, `http` and `https` open the URL, `data:` and others do nothing); reading the animation of small GIF, WebP and APNG test files (not animated, loops forever, plays n times, the length of one play), and a truncated or malformed file reads as not animated without a panic. Prior art: the tests in the core's image, links and render modules.
- **The contract samples.** The app's tests write a sample of the animation answer (the image's link action reuses the link answer's sample), and the page's contract tests drive it through the page. `npm run fixtures` writes them after the shape is added.
- **Hand checks in the release build.** Full size at 100% and 150% Windows scaling is one image pixel per screen pixel, and Downlight's zoom doesn't change it; smoothing above full size; the wheel zooms around the pointer; pinch on a touchpad and on a touch screen, and a swipe; Explorer opens with the file selected; a remote name opens the browser; "Play again" restarts a GIF that plays once, and the document's copy is untouched (and whether a second `<img>` restarts it, which decides the own-URL fallback); the lightbox in full screen; Esc's order with everything open at once; a screen reader announces the dialog and its buttons.
- **Measured.** Building the list of images when the lightbox opens, on the 1 MB test document with the timing switch: it shows no task over 100 ms.

## Out of Scope

- Copying or saving the image, rotating it, printing it, or a slideshow timer.
- Forcing an animated image to loop, and controls for an animation beyond "Play again" (pause, frame stepping).
- "Play again" for a remote animated image.
- Opening the lightbox on anything but an `<img>`: a Mermaid diagram (`.scratch/mermaid-and-math/`), an SVG shown as a document (`.scratch/other-formats/`), a code block or a table.
- Remembering the lightbox's zoom between images or between openings.
- A setting to switch the lightbox off, or to make a click follow every linked image's link.
- Other systems: their ports decide whether their webviews show it the same.

## Further Notes

- From ticket `issues/01-image-lightbox.md`, picked from the mdhero comparison (`.scratch/mdhero-comparison/features.md`), grilled with the repo owner on 2026-11-20.
- Esc's order now has five owners (the settings flyout, the lightbox, Find, Stop, full screen), each listener deferring to the ones before it. The full screen spec's note applies: if the lightbox makes that hard to follow, one place that owns Esc's order may be worth it now.
- The slideshow (`.scratch/slideshow/`) and other formats' SVG as an image (`.scratch/other-formats/`) may want to reuse the lightbox's zoom and pan.
- A remote image setting (`.scratch/remote-images/`), when it blocks remote images, leaves them out of the lightbox's list as it leaves them unloaded.
