//! Downlight's app: the process, its windows and the commands the page calls. The app reads the
//! document's file and passes its bytes to the core, which renders them.

// A GUI app in release builds, so `downlight <file>` opens no console and doesn't hold the shell.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod command_line;
#[cfg(test)]
mod contract;
mod documents;
mod image;
mod links;
mod menu;
mod message;
mod notice;
mod open;
mod platform;
mod render;
mod timing;
mod title_bar;
mod watch;
mod window;

use std::path::Path;
use std::process::ExitCode;

use tauri::Manager as _;

use documents::Documents;

const APP_NAME: &str = "Downlight";

fn main() -> ExitCode {
    let timing = timing::Timing::from_env();

    let built = tauri::Builder::default()
        // First, so that a second launch hands over its documents and exits before anything
        // else starts (ADR 0001).
        .plugin(tauri_plugin_single_instance::init(|app, args, folder| {
            // The second launch waits until this returns, so `downlight <file>` returns at once
            // only if the window opens later, off the main thread.
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let failures = window::open_launch(&app, args, Path::new(&folder));
                // The second launch has returned by now, so this process tells the reader.
                if let Some(report) = window::failure_report(&failures) {
                    platform::show_error(&report);
                }
            });
        }))
        .register_asynchronous_uri_scheme_protocol(image::SCHEME, image::serve)
        .manage(timing)
        .invoke_handler(tauri::generate_handler![
            documents::render_document,
            documents::reload_document,
            links::follow_link,
            open::open_document,
            menu::show_document_menu,
            window::close_window,
            window::show_window,
            timing::report_first_paint,
            timing::report_reload,
            timing::report_long_task,
            title_bar::minimize_window,
            title_bar::toggle_maximize_window,
            title_bar::place_maximize_button,
            title_bar::show_window_menu,
            title_bar::reveal_document,
            notice::open_named_folder,
        ])
        .on_menu_event(menu::forward_choice)
        .on_window_event(|window, event| {
            window::forget_when_destroyed(window, event);
            open::show_when_dropped(window, event);
            title_bar::tell_window_state(window, event);
        })
        .setup(|app| {
            // Without the current folder a relative path stays relative, and reading it reports
            // the problem to the reader.
            let folder = std::env::current_dir().unwrap_or_default();
            let failures = window::open_launch(app.handle(), std::env::args_os(), &folder);
            let Some(report) = window::failure_report(&failures) else {
                return Ok(());
            };
            // Only closing the last window ends the process, so without one it would never end.
            // The process ends here rather than by returning an error, which Tauri turns into a
            // panic that a release build, with no console, ends without a word (ticket 19).
            if app.webview_windows().is_empty() {
                platform::show_error(&report);
                std::process::exit(1);
            }
            // Off the main thread, so the windows that did open keep working while the box is up.
            std::thread::spawn(move || platform::show_error(&report));
            Ok(())
        })
        .build(tauri::generate_context!());
    match built {
        Ok(app) => {
            // Its messages go to the pages through the app's handle, which exists only once the
            // app is built. Managed before the event loop runs, so the launches in `setup` and a
            // second launch's documents find it.
            app.manage(Documents::new(app.handle().clone()));
            app.run(|_, _| {});
            ExitCode::SUCCESS
        }
        Err(error) => {
            platform::show_error(&format!("{APP_NAME} can't start: {error}"));
            ExitCode::FAILURE
        }
    }
}
