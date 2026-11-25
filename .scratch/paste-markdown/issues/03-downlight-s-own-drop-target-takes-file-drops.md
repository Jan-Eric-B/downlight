# 03: Downlight's own drop target takes file drops

**What to build:** On Windows, file drops come through a drop target of Downlight's own instead of Tauri's native drop, with no change the reader can see: a file dropped onto a window shows there exactly as before, with the same Notices for a shortcut, a binary file and a drop of folders. It is the ground for text drops (ticket 04), which wry's drop target can't take.

Spec: [Paste Markdown](../spec.md) (Text drops), [ADR 0007](../../../docs/adr/0007-downlight-takes-drops-with-its-own-drop-target.md)

**Blocked by:** None (can start immediately)

**Status:** built, hand checks left

- [x] Tauri's native drop is off for every window on Windows (`dragDropEnabled: false`); other systems keep it, and `open::show_when_dropped` there.
- [x] The Windows module registers an `IDropTarget` of its own on the webview's child windows, as wry 0.57's `DragDropController` does (revoke WebView2's target, register ours), built with the `windows` crate's `implement` support (no new dependency). The `unsafe` code stays in the Windows module, behind one platform interface.
- [x] It reads a drag's file paths as wry does: `CF_HDROP`, then the shell item array when that fails (a path of 260 characters or longer from Explorer). A drag with paths gets the copy cursor (`DROPEFFECT_COPY`, never a move) and any other the no-drop cursor; no highlight.
- [x] A drop's paths go to today's logic (the first text file, its shortcut, binary and no-file Notices) on a background task, so the main thread only copies the paths out.
- [x] The drop target lives as long as its window and goes with it.
- [x] The documents tests of the drop logic still pass unchanged (they take paths, not Tauri's event).
- [ ] Hand checks in the release build: a Markdown file from Explorer; a path of 260 characters or longer; several files with the text file not first; a shortcut; a binary file; a folder; a file onto the empty window and onto a document still on its way; a text drag still gets the no-drop cursor.
- [x] `npm run check` passes.

## Comments

**2026-11-20.** Split off while grilling Paste Markdown: the owner wants it, but not at the cost of holding Paste back.

**2026-11-25.** Grilled with the owner (spec's Text drops, ADR 0007): Downlight takes drops with its own drop target in the Windows module, rather than HTML5 drops in the page, whose files come without paths. Split in two so a regression in file drops shows on its own: this ticket moves file drops onto the new drop target, ticket 04 adds text.

**2026-11-25.** Built; only the hand checks are left.

- **The interface.** `platform::with_drop_target(builder)` turns Tauri's native drop off for the window (`disable_drag_drop_handler`, the code's `dragDropEnabled: false`, since windows are built in `window::open_window`, not in `tauri.conf.json`), and `platform::add_drop_target(&window, dropped: impl Fn(Vec<PathBuf>) + Send + 'static)` registers the drop target. Other systems' fallbacks do nothing, so Tauri's native drop and `open::show_when_dropped` stay there. `open::take_drops`, called from `open_window`, hands the paths to the same `show_dropped` the documents tests drive, on a blocking task (`show_dropped_off_the_main_thread`, shared with `show_when_dropped`).
- **The drop target.** `app/src/platform/windows/drop_target.rs`, with the `windows` crate's `#[implement(IDropTarget, Agile = false)]` (it holds `Rc`s and cells and is only called on the main thread); the crate gained Win32 features (Foundation, Graphics_Gdi, System_Com, System_Com_StructuredStorage, System_Ole, System_SystemServices, System_Threading, UI_Shell, UI_WindowsAndMessaging), and Cargo.lock and the notices are unchanged. In `with_webview` (the main thread) it takes the controller's parent window, enumerates the windows inside it that belong to this thread (WebView2's other processes' windows take neither a drop target nor a subclass), and on each revokes WebView2's target and registers its own, as wry does. It also sets `SetAllowExternalDrop(false)`, as wry does. The paths are read at DragEnter, for the cursor, and kept in the target for Drop (ticket 04): `CF_HDROP` through a medium released with `ReleaseStgMedium` (wry calls `DragFinish`, and leaks the medium it reads at DragEnter), and the shell item array when that fails. Drop sets the effect it answers too (wry leaves the source's allowed effects there, a move among them), and every call answers a copy only when the source allows one.
- **Its lifetime.** OLE holds the only reference to each target, from `RegisterDragDrop` until `RevokeDragDrop`, which a subclass of the window (`SetWindowSubclass`, set before registering and removed again should registering fail) calls in its `WM_DESTROY`, whichever way the window goes; the closure, and the app handle it holds, goes with the window's last target. OLE is initialized on the main thread by tao, for every window it creates with its own drag and drop on, which Tauri leaves on.
- **For ticket 04.** The drag is read in one place, `dragged_paths` (with `Medium::get(data, format)`, which 04 can use for `CF_UNICODETEXT`); `DropTarget` keeps the effect chosen at DragEnter for DragOver, and would keep the window's label for the tag. `add_drop_target`'s callback takes paths only; 04 widens it to a file or text drop.
- **Left.** The hand checks.
