# 32: Local images show their file's latest version after a reload

**What to build:** When a local image's file changes (a regenerated diagram, a new screenshot) and the document reloads, the window shows the new picture.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-human

- [ ] Checked first by hand in the release build: change a local image the document shows, save the document, and see whether the reload shows the new picture; then F5. Record the result here. If the new picture shows, close this ticket. (Decided from the code and from how Chromium treats images instead, see 2026-11-17 below; the hand check of the fix is left for the owner.)
- [x] If it doesn't: a reload loads each local image anew. A cache header alone likely won't do, since Chromium reuses an image its document has already loaded whatever the HTTP cache says. One way: the image scheme's URLs carry a per-render query, which `local_image` already ignores, written by the core from a value the app passes with each render, or added by the page to the template on a reload.
- [x] An unchanged image doesn't flicker on a reload. Check it on a document with many images. (Built so that an unchanged image keeps its URL; the check on a document with many images is a hand check.)
- [x] Tested first where a test can observe it: the URLs a render writes (core), or the page's reload (page tests).

## Comments

**2026-11-13, from the architecture review.** The image scheme answers without cache headers (`app/src/image.rs`), and a reload writes the same URL for an image as before (`core/src/image.rs`, `local_source`).

**2026-11-17, decided from the code instead of the first hand check.** The reload most likely shows the old picture, so the fix was built without waiting for the hand check:

- A reload replaces the view's content with new elements but stays in the same page: the page parses the render into a template and swaps it in (`prepare` in `page/src/view.ts`), and the webview never navigates. The core writes the same URL for an image on every render (`local_source` in `core/src/image.rs`).
- The HTML standard gives every page a list of available images, keyed by URL, and an `<img>` whose URL is on it takes the image from there without asking the network. The standard says the list exists "to avoid re-downloading images in the same document even when they don't allow caching per HTTP" ([HTML, the list of available images](https://html.spec.whatwg.org/multipage/images.html#list-of-available-images), and step "update the image data" there). Chromium does the same in Blink's resource fetcher: a resource the page has already loaded is used again for the same URL, "even if the cache policies would tell us to" reload it (`ResourceFetcher::DetermineRevalidationPolicy`, `third_party/blink/renderer/platform/loader/fetch/resource_fetcher.cc`). Raw requests are the only exception; images aren't. (Both quoted from memory of the standard and of Chromium's source; neither was fetched again for this ticket.)
- So the image scheme's missing cache headers don't matter, and adding `Cache-Control: no-store` wouldn't help: as long as the URL is the same, the new `<img>` gets the old picture. F5 is a reload of the same kind (the app renders again, the page swaps the content), so it behaves the same.

**2026-11-17, built.** A local image's URL names when its file was last saved, so a changed file gets a new URL and loads anew, and an unchanged one keeps its URL.

- **Approach.** The ticket's options were a per-render value (from the app, or added by the page on a reload) or a value that changes only with the file. A per-render value gives every local image a new URL on every reload, so every image loads anew: each new `<img>` stays empty until its file comes back from the image scheme, which shows as a flicker of every image in view, and an image without `width` and `height` has no height until then, so the text below it jumps while the reload anchors the reading position. The page alone can't do better, since it can't tell which files changed. So the URL carries the file's modified time instead, `http://image.localhost/C:/docs/img/logo.png?modified=1700000000000000100` (nanoseconds since 1970): it stays the same until the file is saved again, an unchanged image is taken from the page's list of available images with no request and no flicker, and only a changed image loads anew.
- **Who looks at the file.** The core still reads no files: `render` takes the caller's `FileSystem`, whose lookups already serve link resolution and the image scheme, and the trait has a new lookup, `modified` (the app's `Disk` answers it with the file's metadata, `app/src/disk.rs`). The lookup follows links, so the core asks it only once the image's path is known to lead to this computer (`leads`, ticket 20); an image that links to a share or elsewhere, or whose links can't be read, keeps its URL without a time, and the image scheme refuses it as before. A missing file's URL has no time either, so once the file appears the next reload gives it one and it loads.
- **The image scheme.** Unchanged: it serves the URL's path (`request.uri().path()`), so the query never reaches `local_image`, which still refuses anything but local images. The document's own query on an image is still dropped and its fragment kept after the time, so `#gh-dark-mode-only` still matches `document.css`.
- **Cost.** Each local image costs a render one link lookup per folder along its path plus one metadata lookup, on the render's blocking thread. The test documents that `npm run measure` uses have no images, so this work doesn't show in their timings and wasn't measured; if a document with hundreds of images renders slowly, the link lookups of a render could be kept per folder.
- **Changed in.** `core/src/files.rs` (`FileSystem::modified`, and the test fake's `modified_at`), `core/src/image.rs` (`ImageSources` holds the lookups; `local_source` writes the time), `core/src/render.rs` and `core/src/sanitize.rs` (`render` takes the lookups; the sanitizer's filter must be `Send`, `Sync` and `'static`, so they are owned and behind a lock), `app/src/disk.rs`, `app/src/render.rs` (passes `Disk`), and the callers in the tests. The rendered fixtures and contract samples don't change: the fixtures render through a file system with nothing in it, and no shape between the page and the app changed.
- **Tests, test-first.** In `core/src/image.rs`: `local_image_url_names_the_time_its_file_was_last_saved`, `local_image_url_stays_the_same_until_its_file_is_saved_again`, `image_whose_file_is_missing_loads_without_a_time`, `saved_time_comes_before_the_fragment_and_in_srcset_too`, `image_url_with_its_saved_time_leads_back_to_its_file`, `image_whose_path_links_off_this_computer_is_not_asked_when_it_was_saved`, `image_whose_links_cannot_be_read_is_not_asked_when_it_was_saved`. In `app/src/disk.rs`: `modified_tells_when_a_file_was_saved_and_nothing_for_a_missing_one`. No page test: the page doesn't change, and happy-dom has no list of available images, so a page test couldn't see the old or the new picture.
- **Not built.** A changed image above the reading position whose height changes may move the text a little after the reload has landed, since the reload doesn't wait for images; the hand checks look at it.

**Hand checks** for the owner, on the release build:

1. Open a document that shows a local image (`![Diagram](diagram.png)`). Replace `diagram.png` with another picture, then save the document: the reload shows the new picture.
2. Replace the picture again without touching the document and press F5: the new picture shows.
3. A document with many local images (twenty or more, some above the reading position): scroll to the middle, save the document without touching the images. No image flickers or blinks empty, and the view stays at the reading position.
4. In that document, replace one image above the reading position with a taller one and save the document: the new picture shows, and note whether the view stays at the reading position.
