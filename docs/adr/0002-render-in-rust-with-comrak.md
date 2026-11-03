# Render in Rust with comrak, not in the page

Downlight turns a document into HTML in the core crate, in Rust with comrak, sanitizes it there with ammonia, and sends the finished HTML to the page, which only inserts it. No Markdown renderer runs in the page. The performance spike (ticket 02) measured what this costs: reading and rendering take about 2 ms for a 100 KB document and about 15 ms for 1 MB, on a background thread, out of a first paint of about 0.7 to 0.8 s; nearly all the rest is the webview starting. Rendering elsewhere would save nothing worth having, and rendering in Rust keeps three things the spec needs:

- **Untrusted HTML never reaches the page unsanitized.** The sanitizing happens before the page sees any of the document, in a crate whose tests check what is removed and what is kept. A renderer in the page would put the document's raw HTML and the sanitizer in the same place as the scripts it must keep out.
- **The page's main thread stays free.** The 1 MB target is that the window never stops answering for more than 100 ms. Rendering in Rust runs off the page entirely, so that budget goes to inserting and laying out the HTML alone; a renderer in the page would spend the same thread's time on parsing Markdown too.
- **The rendering is tested without a window.** The core's public interface is the main test seam, and its tests run with `cargo test` in seconds (spec, user story 85).

## Considered options

- **A JavaScript renderer in the page** (markdown-it or marked, with DOMPurify). One language for rendering and showing, and no HTML crossing the IPC. But rendering and sanitizing would run on the page's main thread, the renderer's script would load before every first paint, and its tests would need a DOM.
- **A JavaScript renderer in a Web Worker.** Keeps the main thread free, but still sanitizes in the page, adds a worker to start in every window, and splits the document logic (line numbers, heading anchors, image paths) across two languages.

## Consequences

- The rendered HTML crosses the IPC as one string. For 1 MB of Markdown that took about 15 ms between the end of rendering and the page receiving it.
- Syntax highlighting also runs in Rust (syntect through comrak's plugin), emitting CSS classes so that switching between light and dark needs no new render.
- Read and render start only once the page asks, after the webview has started. Starting them at process start would save their 2 to 15 ms at most, so they stay where they are until a measurement says otherwise.
- A further format (CSV, SVG) is a new Rust module behind the core's render entry point.
