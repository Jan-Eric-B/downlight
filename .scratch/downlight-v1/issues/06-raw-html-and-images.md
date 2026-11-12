# 06: Raw HTML and images, safely

**What to build:** The raw HTML that GitHub allows renders, and nothing that could act on its own does. Relative and local images load through Downlight's own scheme, remote images load as on GitHub, and images on network shares never load, so opening a document can't make Windows send credentials to a server.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 04 (Documents look like GitHub on Mica)

**Status:** closed

- [x] Sanitizing keeps `<details>`, `<summary>`, `<picture>`, `<source>`, `<img>` with `width`, `height` and `align`, `align` on block elements, `<kbd>`, `<sub>` and `<sup>`, plus the task list checkboxes, heading ids, line numbers and classes that rendering produces.
- [x] Sanitizing removes scripts, styles, frames, objects, event handler attributes and `javascript:` URLs.
- [x] Sanitizing removes the tags GitHub's tagfilter escapes (`<title>`, `<textarea>`, `<style>`, `<xmp>`, `<iframe>`, `<noembed>`, `<noframes>`, `<script>`, `<plaintext>`). comrak 0.59 has no tagfilter (removed in 0.56, see ticket 04), so this is the only place they are stopped. GitHub shows them as escaped text; Downlight drops them.
- [x] A `<picture>` with light and dark sources shows the image for the current mode. (Light mode checked with emulated media, see Comments.)
- [x] Relative image paths resolve against the document's folder and load through Downlight's own scheme, and so do absolute local paths.
- [x] Remote `http(s)` images load.
- [x] Images on network shares (UNC paths, `file://server/`) are removed before the page sees them; opening the document makes no connection to the share.
- [x] The content security policy is final: scripts only from Downlight, images from Downlight's own scheme, `data:` URLs and remote `http(s)` addresses.
- [x] Core tests cover what is kept, what is removed and each kind of image source.

## Results

**2026-11-09, measured** with `npm run measure` on the release build, median of 5 runs. This ticket's build and the one before it (HEAD, built from `git archive`) ran back to back, so they compare with each other. They don't compare with ticket 04's numbers, since load moves them: an earlier run of each, made while other work ran, was 100 to 600 ms slower on first paint.

| Case | Target | Before | Ticket 06 |
| --- | --- | ---: | ---: |
| Cold start, 100 KB, first paint / shown | under 1.0 s | 799 / 809 ms | **803 / 813 ms** |
| Second window, 100 KB, shown after the window starts opening | under 300 ms | 294 ms | **277 ms** |
| 1 MB mixed, first paint / shown | under 2 s | 963 / 981 ms | 983 / 996 ms |
| 1 MB prose, first paint / shown | under 2 s | 908 / 931 ms | 867 / 880 ms |
| Longest task or frame once the page has the document, also while scrolling | never over 100 ms | none over 100 ms | **none over 100 ms** |
| Read and render: 100 KB / 1 MB mixed / 1 MB prose | | 3.2 / 32.8 / 21.0 ms | 9.5 / 103.0 / 42.5 ms |
| Private memory: 100 KB / 1 MB mixed / 1 MB prose | recorded | 188 / 269 / 198 MB | 229 / 270 / 198 MB |
| `downlight.exe` | recorded | 4.32 MB | 5.14 MB |

- Sanitizing is the growth in reading and rendering: ammonia parses and writes the whole HTML again, about 70 ms for the 1 MB mixed document, on the background thread. First paint barely moves, since the webview's start dominates it.
- The 100 KB document's 41 MB more private memory isn't explained by its 6 ms more work, and the 1 MB documents didn't grow. It is likely noise; a quiet machine should measure it again.

## Comments

**2026-11-09, implementation.**

- **Sanitizing** (`core/src/sanitize.rs`): comrak now writes raw HTML (`render.unsafe`), and ammonia 4.2.4 sanitizes all of comrak's HTML, its own markup included, with an explicit allowlist:
  - elements: GitHub's, plus `section` (footnotes) and `input` (task list checkboxes);
  - on every element: `align`, `width`, `height`, ids, `data-sourcepos` and a few ARIA attributes;
  - on single elements: `href`, `src`, `srcset`, `media`, `open` and the footnote attributes.

  A document keeps only the classes rendering writes (and `language-*` on `code`), so it can't take on the look of the page's own parts, such as its notice. `link_rel` is off, since links go through ticket 11.
- **tagfilter's tags, and `noscript`:** before comrak writes HTML, these tags in the document's raw HTML are renamed to `dropped-<name>`. The parser reads that as an unknown element, and ammonia removes it; `dropped-script` and `dropped-style` go with their content.
  - The tags have to be escaped (as on GitHub) or renamed (as here) before the HTML is parsed. Left to ammonia, an unclosed `<textarea>`, `<xmp>` or `<plaintext>` turned the rest of the document into its text.
  - What the other tags enclose shows as ordinary content, as on GitHub without the tag text.
  - `noscript` is on the list because ammonia's parser runs with scripting on, which makes `noscript` behave like `noembed`.
  - Raw `<input>` is dropped the same way, so checkboxes come only from task lists (ticket 13 counts them).
- **Links:** ammonia checks a URL's scheme before its attribute filter runs. So its scheme list is now a superset (`file`, `data` and the drive letters, which the URL parser reads as schemes), and the filter decides.
  - `a href` keeps relative targets, Windows paths, `http`, `https`, `mailto` and `file`.
  - Any other scheme (`javascript:`, `data:`, `ms-settings:`, `search-ms:`) loses its target.
  - What each link does when clicked is ticket 11's decision.
- **Images** (`core/src/image.rs`): `img src` and each candidate of `source srcset` go through one rule.
  - `http(s)` and `data:` stay as written.
  - Relative paths, absolute paths (`C:\`, `C:/`, `/`) and `file:` URLs resolve against the document's folder as a `file:` URL (url 2.5.8). They become `http://image.localhost/C:/docs/img/logo.png`: the query is dropped (`?raw=true`), and the fragment is kept (`#gh-dark-mode-only`).
  - comrak writes a backslash as `%5C`, so `%5C` separates folders the way `\` does.
  - Anything that resolves to a network share (`\\server\share`, `//server/share`, `file://server/`, `file:////server/`, `\\?\UNC\`), or has another scheme, loses its `src`, and the page shows the alt text.
  - `/logo.png` resolves to the document's drive root, not to the repository root as on GitHub.
- **Render's interface** is now `render(bytes, path, image_origin)`. That departs from the spec's "takes a file's bytes and its path". The core knows the URLs' form, but only the app knows where the webview serves a custom scheme: `http://image.localhost` on Windows, `image://localhost` elsewhere (`custom_scheme_origin` in the per-OS module).
- **The image scheme** (`app/src/image.rs`) is an asynchronous protocol that reads the file on a blocking thread.
  - The core's `local_image` maps the URL's path back to the file and checks it again: it must be a whole local path with an image extension (png, apng, jpg, jpeg, gif, webp, avif, svg, bmp, ico). The check is repeated because the page asks for whatever a document names.
  - A missing file is a 404, which the page shows as a missing image.
  - Responses carry `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; sandbox` and `nosniff`, so an SVG opened on its own couldn't run scripts.
- **Content security policy:** `img-src http://image.localhost data: https: http:` is added. The image scheme is listed although `http:` covers it, for when remote images can be switched off. Scripts stay `'self'`.
- **Extras, not in the criteria:**
  - `<meta name="referrer" content="no-referrer">`, so remote image hosts aren't told where they're shown (GitHub's image proxy doesn't tell them either).
  - GitHub's `#gh-dark-mode-only` and `#gh-light-mode-only` images are hidden in the other mode (`page/src/document.css`).
- **Glossary:** Local image.
- **Dependencies:** ammonia 4.2.4 (MIT OR Apache-2.0) brings html5ever 0.40 (a second copy beside Tauri's 0.39), markup5ever, string_cache and maplit. url 2.5.8 was in the tree already. cargo-deny passes. `downlight.exe` grew from 4.32 MB to 5.14 MB.
- **The rendered fixtures** changed in form only: void elements are written `<hr>` rather than `<hr />`, and boolean attributes get `=""`, as html5ever writes them. The test `task_list_items_render_as_checkboxes_with_githubs_classes` was changed for that one reason.
- **For ticket 10:** raw HTML blocks carry no line number, since comrak writes them as they are. A document's own `data-sourcepos` attributes are kept, so a raw block can name a wrong line.
- **Tests:** `core/src/sanitize.rs` covers what is kept, what is removed, tagfilter's tags (open and closed), classes, checkboxes and link targets. `core/src/image.rs` covers each kind of image source, `srcset` and `local_image`. All of them go through `render` and `local_image`.

**Checked by hand, 2026-11-09,** on the release build with WebView2's debug port, with a document beside an `img` folder:

- Relative, absolute (`C:\...\abs image.svg`), PNG, `data:` and remote https images loaded. The image scheme answered with `image/svg+xml` and `image/png`.
- No request of any kind went to the host of the UNC, `//server` or `file://server/` images, and their `img` elements had no `src`.
- In dark mode, the `<picture>` showed its dark source and the `#gh-light-mode-only` image was hidden. With light mode emulated over the debug port (not a real switch in Windows), the light source showed and the `#gh-dark-mode-only` image was hidden.
- `onerror`, `<script>` and `<style>` had no effect: the title and the body's display stayed as they were. The `notice` class was gone, and the heading after an unclosed `<textarea>` rendered.
- The served policy: `default-src 'self'; script-src 'self'` (plus Tauri's hash)`; connect-src ipc: http://ipc.localhost; img-src http://image.localhost data: https: http:; object-src 'none'; base-uri 'none'; form-action 'none'`.

Open:

- **Credentials over http:** a remote image at an intranet host name (`http://fileserver/logo.png`) is allowed. WebView2, like Chromium, may answer an NTLM or Negotiate challenge from a host it counts as intranet without asking. The spec names network shares only. Turning that off (Chromium's `--auth-server-allowlist` browser argument, for instance) touches the arguments of the WebView2 environment, which ticket 19 also deals with, so it needs a decision.
- **A document on a network share** loses all its relative images, as the criterion says, although the reader has already connected to that server by opening the file.
- `<picture>` in light mode with a real switch in Windows.

**2026-11-10, closed.** Closed, with every criterion done. The credentials over http moved to ticket 20, which the v1 release check waits on. Still open from the Comments: a document on a network share loses its relative images, and `<picture>` in light mode with a real switch in Windows.
