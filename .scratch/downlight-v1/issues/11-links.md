# 11: Links

**What to build:** Clicking a link does the one safe, expected thing. A Markdown document shows in the same window (or in a new one with Ctrl+click), headings scroll, web and mail links go to the system, other local files are shown in Explorer and never run, and other schemes do nothing. Every link shows its target on hover.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 04 (Documents look like GitHub on Mica), 08 (One process, a window per document)

**Status:** ready-for-human

- [x] The core's link resolution takes the document's path and a link target and returns one action: show a document in this window (optionally at a heading), scroll to a heading, reveal a file in the file manager, open a folder, hand a URL to the system, or nothing.
- [x] A link to a Markdown file (one of the six extensions) shows that document in the same window, at its heading if the link has one.
- [x] A link to a heading in this document scrolls to it.
- [x] Ctrl+click on a link to a Markdown file opens it in a new window, or brings its window to the front if it is open.
- [x] `http`, `https` and `mailto` links open in the system's browser or mail app.
- [x] A link to a local file that isn't Markdown shows the file selected in Explorer; Downlight never opens or runs it.
- [x] A link to a folder opens it in Explorer.
- [x] A link with any other scheme does nothing.
- [x] Paths on network shares are handled like local paths, and only when clicked.
- [x] Hovering any link shows its target.
- [x] A link to a missing file shows a message.
- [x] Core tests are a table of document paths and link targets with the action each one gives.

## Plan

**Approach:** A pure `links` feature in the core: `resolve_link(document, target, kind_of) -> Result<LinkAction, LinkError>`. It works on path syntax and gets file and folder facts only from an injected lookup (`Fn(&Path) -> Option<PathKind>`, `PathKind` is `File` or `Folder`, `None` is missing), so the table tests touch no disk or network and the core reads no files. The app adds one command, `follow_link(window, target, new_window)`, in `app/src/links.rs`. It takes the document path from `OpenWindows` (never from the page), resolves on a blocking thread, and either shows the document through the existing `window::show` or calls the per-OS `reveal_file`, `open_folder` or `open_url`, returning a typed `Followed` outcome that the page carries out. Showing another document in this window adds no new path: `window::show` -> `OpenWindows::claim` already replaces the window's `ShownDocument`, which is where ticket 10's watch lives, so the watch follows a link for free. Ticket 12 adds history where the page handles `Followed` and `document-shown`.

**Tests first:**

- core `links.rs` (`#[cfg(all(test, windows))]`, a fake lookup, a table of document, target and expected action per test):
  - `markdown_link_shows_the_document_in_this_window` - `guide.md`, `./sub/a.MD`, `..\b.mdown`, `my%20doc.md`, `C:/x.md`, `file:///C:/x.md`, `/top.md` (drive root)
  - `markdown_link_with_a_heading_shows_the_document_at_that_heading` - `guide.md#install`; `a.md#gr%C3%B6%C3%9Fe` gives the decoded `größe`
  - `heading_link_scrolls_to_the_heading` - `#install`, `#fn-1` (footnotes), a percent-encoded fragment
  - `web_and_mail_links_go_to_the_system` - `http:`, `HTTPS:`, `mailto:`, re-serialized by `url` so quotes and spaces are encoded
  - `local_file_that_is_not_markdown_is_revealed` - `.txt`, `.exe`, `.bat`, `.lnk`, no extension, `notes.md.exe`
  - `link_to_a_folder_opens_it` - `docs/`, `..`, `C:\`, a folder named `x.md`
  - `links_with_other_schemes_do_nothing` - `ftp:`, `ms-settings:`, `search-ms:`, `javascript:`, `data:`, `""`, `?q`
  - `network_share_paths_resolve_like_local_paths` - `\\server\share\a.md`, `//server/share/b.txt`, `file://server/share/docs/`, a relative link in a document on a share
  - `link_to_a_missing_file_is_an_error_naming_it` - `LinkError::NotFound { path }` for a missing `.md` and `.txt`
  - `heading_and_web_links_look_up_no_file` - the lookup panics if called
- core `image.rs`: the existing image tests stay green after the shared path parsing moves out.
- app `window.rs`: `document_claimed_for_its_own_window_is_already_there` (a link to the shown document scrolls instead of rendering again); `window_document_path_is_the_one_it_shows`.
- app `links.rs`: `path_kind_tells_files_folders_and_missing_apart` (real temp files); `missing_file_message_names_the_file`; `page_navigates_only_within_its_own_origin` (pure `is_own_page(url)`).
- page `links.test.ts`:
  - "clicking a link follows its target and never navigates the webview" - invokes `follow_link {target: raw href, newWindow: false}`, default prevented
  - "ctrl+click asks for a new window"
  - "heading outcome scrolls to the element with that id in the document"
  - "link to a missing file shows the app's message"
  - "hovering or focusing a link shows its target, leaving hides it"
  - "hovering a link asks the app nothing" - network shares are touched only when clicked
  - "link without a target does nothing" - sanitized `<a>Link</a>`
  - "middle and shift clicks are stopped"
- page `view.test.ts`: "document shown at a heading scrolls to that heading" (`document-shown` payload `"install"`); "document shown without a heading starts at the top"; "a new window opened at a heading scrolls to it on its first show" (`render_document` answers with a heading); "scroll-to-heading from the app scrolls an open document to that heading".
- app `window.rs`: `new_window_for_a_heading_link_keeps_the_heading` - the heading given to `show` with `Target::NewWindow` comes back from the window's first render.

**Changes:**

1. `GLOSSARY.md` - entries for **Link target** (as written in the document, untrusted) and **Link action** (what link resolution returns).
2. `core/src/file_url.rs` (new, crate-private) - move `local_source`'s parsing out of `image.rs` (the `%5C` fix, drive paths, joining against the folder URL); `image.rs` keeps its `is_local` filter (no shares). `core/Cargo.toml` gets `percent-encoding` (already in the lockfile through `url`) for fragments.
3. `core/src/links.rs` (new) and `core/src/lib.rs` - `LinkAction { ShowDocument { path, heading: Option<String> }, ScrollToHeading { heading }, RevealFile(PathBuf), OpenFolder(PathBuf), OpenUrl(String), Nothing }`, `PathKind`, `LinkError::NotFound { path }`, `resolve_link`; Markdown is decided by the existing `is_markdown`.
4. `app/Cargo.toml` - windows-sys features `Win32_UI_Shell`, `Win32_UI_Shell_Common`, `Win32_System_Com`.
5. `app/src/platform/windows.rs` - `reveal_file` (COM through the shared `with_com` helper, `ILCreateFromPathW`, `SHOpenFolderAndSelectItems(pidl, 0, null, 0)`, `ILFree`); `open_folder` (`ShellExecuteW` verb `explore`, which fails on a file and so never runs it); `open_url` (`ShellExecuteW` verb `open`). Each returns `io::Result<()>`; every `unsafe` block has a SAFETY comment.
6. `app/src/platform/mod.rs` - non-Windows fallbacks returning `io::ErrorKind::Unsupported`, shown as a notice.
7. `app/src/window.rs` - `show(app, document, target, heading: Option<&str>) -> tauri::Result<Shown>` (`Opened`, `Changed`, `AlreadyOpen { label }`); the heading travels as the `document-shown` payload (`""` is the top); Open and drop pass `None`. For `Target::NewWindow`, `ShownDocument` keeps the heading and `render_document` returns it with the HTML, so a new window opens at the heading. `OpenWindows::document_path(label)`. In `open_window`: `.on_navigation(is_own_page)` and `.on_new_window(|_, _| NewWindowResponse::Deny)` as a safety net.
8. `app/src/links.rs` (new) and `app/src/main.rs` - async `follow_link(window, target: String, new_window: bool) -> Result<Followed, FollowError>`, registered in `generate_handler!`. `Followed` is `scrollTo { heading } | shown | nothing` (`#[serde(tag = "kind")]`). `ShowDocument` with Ctrl gives `Target::NewWindow`, without it `Target::Window(own)`; `AlreadyOpen` in the window's own label becomes `scrollTo`. `AlreadyOpen` in another window brings it to the front and, only if the link has a heading, emits `scroll-to-heading` with it to that window; without a heading that window keeps its reading position. `FollowError` (NotFound, Reveal, OpenFolder, OpenUrl, Show, Stopped) carries the reader's messages, e.g. "Can't find C:\docs\x.md." No capability change.
9. `page/src/links.ts` (new) - delegated `click` and `auxclick` on the article's `a[href]`: `preventDefault`, invoke, act. Scrolling looks up the id with `CSS.escape`, then `a[name]`, inside the article only. A status bubble at the bottom left shows the raw `getAttribute("href")` on hover and focus (WebView2's own status bar is off in wry 0.57.2).
10. `page/src/view.ts`, `page/src/main.ts`, `page/src/document.css` - the `document-shown` handler scrolls to the payload's heading or to the top, through `showDocument`; start the links module; `.link-target` style.

**Fits with other tickets:**

- `OpenWindows::document_path(label)` is the one accessor for a window's document; ticket 15's Edit uses it too. Whichever lands first adds it.
- COM setup in `app/src/platform/windows.rs` goes through one `with_com` helper (`CoInitializeEx` apartment-threaded, `CoUninitialize` only if init succeeded), shared with ticket 15's editor launch. The `Win32_UI_Shell` and `Win32_System_Com` features are added once.
- Ticket 10 restarts the watch on `claim`'s New/Changed path; this ticket's `show` change keeps that path, so after a link the new document is watched and the old one isn't.
- A document shown by a link goes through `showDocument`, so ticket 14's `whenDocumentShown` listeners run and Find recounts.

**Verification:** `npm run check`, then `cargo test -p downlight-core links`. By hand on the release build with a test folder: `guide.md#install` shows in the same window at the heading and the title changes; `#heading` and footnote links scroll; Ctrl+click opens a new window, or brings the open one to the front (also minimized); https opens the browser and mailto the mail app; `.exe`, `.bat` and `.txt` are only selected in Explorer; a folder link opens Explorer; `ms-settings:` and `ftp:` do nothing; a missing file shows the notice; hovering shows targets; a `\\server\share` link connects only on click (Process Monitor or a slow share); Shift-click and middle-click open no browser window; with ticket 10 merged, after following a link, saving the new document reloads it and saving the old one doesn't.

**Decisions (2026-11-12):**

- Ctrl+click on `other.md#heading` opens the new window at that heading, in this ticket (not left for ticket 12): `render_document` returns the heading with the HTML.
- A link to a document already open in another window brings that window to the front and scrolls it to the heading if the link has one; without a heading, that window keeps its reading position.
- The tickets are built one at a time, so each shared piece in "Fits with other tickets" is added by the first ticket that needs it.

## Results

Built 2026-11-12. `npm run check` passes.

**Beyond the plan:**

- The heading reaches the page one way only: `ShownDocument` keeps it and `render_document` answers `{ document, heading }` for every window, new or changed, and `document-shown` stays without a payload. The plan sent it two ways (the event's payload for a changed window, the render for a new one). The heading is cloned, not taken, so of two links followed in quick succession the page's latest request still gets it; the page ignores it on a reload. Tests `window_shown_a_document_at_a_heading_hands_it_over_with_every_render` and "a reload of a document shown at a heading keeps the reading position".
- The `url` crate leaves quotes and spaces in a `mailto:` URL, so `OpenUrl` percent-encodes spaces, quotes, `<`, `>`, backticks and control characters on top, and a URL can't break the browser's or mail app's command line.
- The page finds a heading with `getElementById`, then `getElementsByName` for `a[name]`, both kept to the article, rather than a selector with `CSS.escape`: no escaping, and happy-dom rejects escaped id selectors that browsers accept.
- The navigation guard sits in `window.rs` next to `page_origin`, with no `is_own_page` in `links.rs`: a URL may load only if its origin is the page's own, the `devUrl`'s (Vite's dev server) under `cfg!(dev)`, otherwise `http://tauri.localhost`, which Tauri also serves in development without a `devUrl`. `page_origin` always gives an origin, so no configuration leaves a blank window.
- A link to a document open in another window whose page hasn't asked for its document yet (a window still starting, say after two quick Ctrl+clicks on `b.md#x` and `b.md#y`) doesn't send that page `scroll-to-heading`, which it couldn't hear yet: `OpenWindows::claim` holds the heading on the window's `ShownDocument`, under the same lock, and the page's first render gets it, as `hold_notice` does for a notice. `show` then answers `Shown::AlreadyOpen { window, scroll_to }`, and `follow_link` sends only `scroll_to`. Tests `heading_for_an_open_window_whose_page_has_not_asked_is_held_for_its_first_render` and `link_to_a_document_open_in_another_window_scrolls_it_only_with_a_heading`.
- `Followed` is `scrollTo { heading: string | null } | nothing`: the page has nothing to do after a link showed another document, which `document-shown` already renders, so the plan's `shown` was dropped. `show` answers `Shown::Given` for a new or changed window, and takes the document with its heading as one `DocumentAt`. Serde shape tests (`followed_serializes_as_the_page_reads_it`, `document_to_show_serializes_as_the_page_reads_it`) keep the app's answers and the page's types in step.
- `#` alone, and a link to the shown document without a heading, scroll to the top. An empty target and `?q` do nothing.
- The top of the document is a missing heading everywhere, never an empty one: `LinkAction::ShowDocument` and `LinkAction::ScrollToHeading` both hold `heading: Option<String>` (the core drops an empty fragment for both), as do `Followed::ScrollTo`, `DocumentAt`, `Handover` and `render_document`'s answer, and the page's `scrollToHeading(heading: string | null)` scrolls to the top for null. `scroll-to-heading` is sent only with a heading. Tests `empty_heading_link_scrolls_to_the_top` and "heading outcome without a heading scrolls to the top".
- The lookup is `Fn(&Path) -> io::Result<Option<PathKind>>` rather than `Option<PathKind>`: a path that can't be looked at (an offline share, a folder the reader may not open) gives `LinkError::Unreachable { path, source }`, shown as "Can't reach \\server\share\a.md: " and the system's reason rather than "Can't find", since the file may well exist. Only `ERROR_FILE_NOT_FOUND`, `ERROR_PATH_NOT_FOUND` and `ERROR_INVALID_DRIVE` count as missing (`platform::is_missing`): Rust's `ErrorKind::NotFound` also covers an offline server (`ERROR_BAD_NETPATH`) and a missing share (`ERROR_BAD_NET_NAME`). Tests `link_to_a_path_that_cannot_be_reached_is_an_error_naming_it`, `unreachable_file_message_names_the_file_and_why` and `only_a_missing_file_folder_or_drive_is_missing`.
- Opening a folder stays `ShellExecuteW` with `explore`. A file put where the folder was isn't run because Windows registers `explore` for folders (`HKCR\Folder\shell\explore`) and for no file type; that rests on the registry, though, and a program can add an `explore` verb to a file type for one user without admin rights, which the comment on `open_folder` now says. `ShellExecuteExW` with `SEE_MASK_CLASSNAME` and the class `Directory` was considered in the review and dropped: `explore` isn't a verb of `HKCR\Directory`, so the call fails with `ERROR_NO_ASSOCIATION` on a real folder too. Checked against Microsoft's `SHELLEXECUTEINFOW` docs, this machine's registry and one call outside the repo, and compiled; not clicked in the app.
- `OpenWindows::document_path` replaced the window tests' own `document_of` helper.

**Measured** (release build, 5 runs): following a heading link 80% into the 1 MB mixed document took 45 ms (median) from the click until the heading stayed at the top of the view, with no task or frame over 100 ms.

**Checked in the release build** over WebView2's debug port with real clicks: the page loads (the navigation guard lets its own origin through); `#heading`, footnote and `README.md#heading` links in README.md scroll; `gone.md` shows "Can't find ...\gone.md."; hovering shows the target; `guide.md#install` shows the guide at Install; afterwards a save to guide.md reloads it and a save to README.md doesn't; `location.href = "https://example.com/"` leaves the page where it is, and `window.open` gives null.

**Not yet checked by hand:** the window title after a link; Ctrl+click (a new window, at a heading, and an open document's window brought to the front, also minimized); https in the browser and mailto in the mail app; `.exe`, `.bat` and `.txt` only selected in Explorer; a folder link; `ms-settings:` and `ftp:`; a `\\server\share` link connecting only on click; a link on an offline share or into a folder without access showing "Can't reach ..."; Shift-click and middle-click opening no browser window.

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- Put the app side in `app/src/links.rs` and keep `window.rs` to the window table and `show`. Ticket 10 moved Reload into `app/src/reload.rs` for the same reason. `window.rs` is about 470 lines, and its tests are in `app/src/window/tests.rs`; the tests this plan lists for `window.rs` go there.
- A new window's first heading has a ready path. `OpenWindows::render_of` returns a `Handover`: the `render`, plus a `notice` held on `ShownDocument` until the page first asks. Hold the heading on `ShownDocument` the same way and add it to `Handover`, rather than building a second way to hand data to a new window's page.
- `Claim::New` and `Claim::Changed` are `{ window, key }` now, and `show` passes the key to `reload::watch_document`. Keep that when `show` returns `Shown`, so the watch follows a link without working out the key again.
- Messages to the reader go through `notice::show_notice`. A message for a window whose page may not be listening yet goes through `OpenWindows::hold_notice`; sent directly, it is lost.
- `whenDocumentShown` in `page/src/view.ts` already runs for every shown document and every reload, so a document shown by a link needs nothing extra for Find.
- Two of ticket 10's review findings apply here too: a return value only the tests read, and the same value worked out in two places. Test the effect a caller sees, and pass a value that is computed once.
