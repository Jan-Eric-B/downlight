//! Downlight's app: the process, its window and the commands the page calls. The app reads the
//! document's file and passes its bytes to the core, which renders them.

// A GUI app in release builds, so `downlight <file>` opens no console and doesn't hold the shell.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod document;
mod platform;

fn main() -> tauri::Result<()> {
    let command_line_document = document::CommandLineDocument::from_args(std::env::args_os());

    tauri::Builder::default()
        .manage(command_line_document)
        .invoke_handler(tauri::generate_handler![document::render_document])
        .setup(|app| {
            document::open_window(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
}
