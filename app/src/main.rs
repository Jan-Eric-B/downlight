//! Downlight's app: the process, its windows and the commands the page calls. The app reads the
//! document's file and passes its bytes to the core, which renders them.

// A GUI app in release builds, so `downlight <file>` opens no console and doesn't hold the shell.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod command_line;
#[cfg(test)]
mod contract;
mod disk;
mod documents;
mod edit;
mod image;
mod keep_on_top;
mod links;
mod menu;
mod message;
mod notice;
mod open;
mod platform;
mod render;
mod settings;
mod timing;
mod title_bar;
mod watch;
mod window;
mod zoom;

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use tauri::Manager as _;

use documents::Documents;
use settings::SettingsStore;

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
                let documents = app.state::<Documents>();
                let failures = window::open_launch(&documents, args, Path::new(&folder));
                // The second launch has returned by now, so this process tells the reader.
                if let Some(report) = window::failure_report(&failures) {
                    platform::show_error(&report);
                }
            });
        }))
        .register_asynchronous_uri_scheme_protocol(image::SCHEME, image::serve)
        .manage(timing)
        .invoke_handler(tauri::generate_handler![
            documents::handover::render_document,
            links::follow_link,
            open::open_document,
            menu::show_document_menu,
            window::close_window,
            window::show_window,
            timing::report_first_paint,
            timing::report_reload,
            timing::report_long_task,
            documents::history::step_history,
            documents::history::forget_forward,
            title_bar::minimize_window,
            title_bar::toggle_maximize_window,
            title_bar::place_maximize_button,
            title_bar::show_window_menu,
            title_bar::reveal_document,
            notice::open_named_folder,
            edit::edit_document,
            edit::choose_editor,
            settings::change::change_setting,
            keep_on_top::switch_keep_on_top,
        ])
        .on_menu_event(menu::forward_choice)
        .on_window_event(|window, event| {
            window::track_windows(window, event);
            open::show_when_dropped(window, event);
            title_bar::tell_window_state(window, event);
        })
        .setup(|app| {
            // Without the current folder a relative path stays relative, and reading it reports
            // the problem to the reader.
            let folder = std::env::current_dir().unwrap_or_default();
            let documents = app.state::<Documents>();
            let failures = window::open_launch(&documents, std::env::args_os(), &folder);
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
            // Its messages go to the pages, and its windows open, through the app's handle, which
            // exists only once the app is built. Managed before the event loop runs, so the
            // launches in `setup` and a second launch's documents find it. The settings load
            // before any window opens, since they shape every new window. Without a config folder
            // they hold until Downlight closes, and the first change says so.
            let (settings, problem) =
                SettingsStore::load(app.path().app_config_dir().ok().as_deref());
            if let Some(problem) = problem {
                // Off the main thread, so the windows open while the box is up.
                std::thread::spawn(move || platform::show_error(&problem.to_string()));
            }
            // One store: the documents module changes it, and tells every page, and the app's
            // other parts, such as Edit, read it from the managed state.
            let settings = Arc::new(settings);
            app.manage(Arc::clone(&settings));
            let handle = app.handle().clone();
            app.manage(Documents::new(handle.clone(), handle, settings));
            app.run(|_, _| {});
            ExitCode::SUCCESS
        }
        Err(error) => {
            platform::show_error(&format!("{APP_NAME} can't start: {error}"));
            ExitCode::FAILURE
        }
    }
}
