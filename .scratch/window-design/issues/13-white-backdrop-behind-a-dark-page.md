# 13: A window sometimes shows a white backdrop behind a dark page

**What to build:** Every window shows its document legibly from its first frame: the backdrop behind the page always matches the page's theme, also when several windows open at once. Today a window can come up with white text on a white background until something makes Windows draw it again.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] The cause is found and written down here: which window, which order of events, and why the backdrop and the page's theme disagree.
- [ ] A test reproduces it where a test can see it (for example what `open_window` asks of the windows port when several windows open in one launch); if only a real window shows it, say so and why.
- [ ] After the fix, a launch with several documents shows each window with a backdrop that matches its page, in light and dark.
- [ ] Checked by hand: ten launches of `downlight.exe` with three documents, in dark mode and in light mode, each window legible from its first frame.

## Comments

**2026-11-17, seen during the hand checks of tickets 04 to 07.** The release build (commit f325a45) was started from `target/hand-checks/` with three documents at once: `downlight.exe 00-start-here.md title-bar-and-zoom.md full-width.md`. The window of the first document, `00-start-here.md`, showed white text on a white background; the other two looked normal. A little later it was fine without the reader knowing what changed it. It hasn't been reproduced yet.

What the symptom says: white text means the page took the dark theme and drew no background of its own, as it does when the app says the window has a backdrop (`backdrop` flag, `page/src/backdrop.ts`). The white came from behind the page, so the window's Mica was light, or not drawn yet, while the page was dark. Then something (focus, activation, a resize or a theme message) made Windows draw it right.

Where to look:

- `open_window` in `app/src/window.rs` builds every window hidden (`.visible(false)`, shown once the page has drawn its first frame by `show_window`) and transparent with Mica (`platform::with_backdrop` in `app/src/platform/windows.rs`, `Effect::Mica`, which "follows the system's light or dark mode"). Whether Mica on a window built hidden takes the system's dark mode before it is first shown, and whether that differs for the first window of a launch, is unchecked.
- The first window of a launch opens from `main.rs`'s setup, before the event loop runs; the others follow it. A race between the windows, or between a window's dark-mode attribute and its first show, fits "only the first of several".
- The page's theme comes from `prefers-color-scheme` in the webview, the backdrop's from the window. They are decided in two places, so nothing keeps them in step today; ticket 08's Theme setting will make the app decide both (see 08's comment on `Settings::window_look()`), which may be the natural place for the fix.

Questions for the reader if it comes back: was Windows in dark mode, what made it right (clicking into the window, switching windows, a resize, nothing), and is it always the first window of a launch?

Use `mattpocock-skills:diagnosing-bugs` for the loop: reproduce first, then a failing test where one can see it, then the fix.
