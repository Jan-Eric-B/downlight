//! Downlight's app: the process, its windows and the commands the page calls. The app reads the
//! document's file and passes its bytes to the core, which renders them.

// A GUI app in release builds, so `downlight <file>` opens no console and doesn't hold the shell.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod document;
mod platform;
mod timing;

fn main() -> tauri::Result<()> {
    let timing = timing::Timing::from_env();
    let command_line_documents = document::CommandLineDocuments::from_args(std::env::args_os());

    tauri::Builder::default()
        .manage(timing)
        .manage(command_line_documents)
        .invoke_handler(tauri::generate_handler![
            document::render_document,
            timing::report_first_paint,
            timing::report_long_task,
        ])
        .setup(|app| {
            document::open_first_window(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
}
