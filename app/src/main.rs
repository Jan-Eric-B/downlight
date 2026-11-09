//! Downlight's app: the process, its windows and the commands the page calls. The app reads the
//! document's file and passes its bytes to the core, which renders them.

// A GUI app in release builds, so `downlight <file>` opens no console and doesn't hold the shell.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod command_line;
mod menu;
mod open;
mod platform;
mod render;
mod timing;
mod window;

use std::path::Path;

use tauri::Manager as _;

use window::OpenWindows;

fn main() -> tauri::Result<()> {
    let timing = timing::Timing::from_env();

    tauri::Builder::default()
        // First, so that a second launch hands over its documents and exits before anything
        // else starts (ADR 0001).
        .plugin(tauri_plugin_single_instance::init(|app, args, folder| {
            // The second launch waits until this returns, so `downlight <file>` returns at once
            // only if the window opens later, off the main thread.
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                // Ticket 19 shows these to the reader; release builds have no console.
                for failure in window::open_launch(&app, args, Path::new(&folder)) {
                    eprintln!("{failure}");
                }
            });
        }))
        .manage(timing)
        .manage(OpenWindows::default())
        .invoke_handler(tauri::generate_handler![
            render::render_document,
            open::open_document,
            menu::show_document_menu,
            window::close_window,
            window::show_window,
            timing::report_first_paint,
            timing::report_long_task,
        ])
        .on_menu_event(menu::forward_choice)
        .on_window_event(|window, event| {
            window::forget_when_destroyed(window, event);
            open::show_when_dropped(window, event);
        })
        .setup(|app| {
            // Without the current folder a relative path stays relative, and reading it reports
            // the problem to the reader.
            let folder = std::env::current_dir().unwrap_or_default();
            let failures = window::open_launch(app.handle(), std::env::args_os(), &folder);
            // Ticket 19 shows these to the reader; release builds have no console.
            for failure in &failures {
                eprintln!("{failure}");
            }
            // Only closing the last window ends the process, so without one it would never end.
            // No window open means every window failed, so the failures say why.
            if app.webview_windows().is_empty() {
                let message = failures
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(message.into());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
}
