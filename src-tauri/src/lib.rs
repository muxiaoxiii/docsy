mod app_log;
mod commands;
mod docx;
mod external;
mod ffmpeg;
mod pdf;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_log::install_panic_hook();
    app_log::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(commands::build_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
