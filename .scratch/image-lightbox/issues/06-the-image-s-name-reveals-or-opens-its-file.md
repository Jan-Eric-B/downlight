# 06: The image's name reveals or opens its file

**What to build:** The **Lightbox** shows the image's file name at the top, with its full path or URL as the tooltip. A click on the name reveals a **Local image**'s file in Explorer, selected, or opens a remote image in the browser, without asking; the lightbox stays open. A file gone by then gives a **Notice**. An image embedded in the document (`data:`) shows no name.

Spec: [Image lightbox](../spec.md)

**Blocked by:** 02 (A click on an image shows it in the lightbox)

**Status:** ready-for-agent

- [ ] The name: a local image's file name, a remote image's last URL path segment, decoded, or its host when the path is empty; a `data:` image has none. The tooltip is the full path or URL. The name is a button in the lightbox's Tab order.
- [ ] A new core function turns an image's source into a **Link action**: a source of the image scheme becomes revealing its file, through the core's existing mapping of the scheme's URLs to files and its network share rules; an `http` or `https` URL becomes opening it; anything else is nothing.
- [ ] A new app command takes only the image's source, validates it, asks the core and performs the action as it performs a link's, and answers as a link does; a gone file gives the Notice a broken link gives. Only the page's app module calls it, and its types sit with the page's other command types.
- [ ] The lightbox stays open after the click.
- [ ] Core tests (written first): a local image's source reveals its file; one whose file is on a network share, directly or through a symbolic link, never does; `http` and `https` open the URL; `data:`, `javascript:` and others do nothing.
- [ ] Page tests (written first): the name and tooltip for a local, a remote and a `data:` image; a click asks the app with the image's source; a failure shows the Notice; the lightbox stays open.
- [ ] Contract: the answer reuses the link answer's sample; if a new shape is needed, the app's tests write its sample and `npm run fixtures` writes it.
- [ ] Hand check in the release build: Explorer opens with the file selected; a remote name opens the browser; a deleted file gives the Notice.
- [ ] `npm run check` passes.
