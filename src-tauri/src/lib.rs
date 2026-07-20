mod app_log;
mod commands;
mod docx_template;
mod external;
mod ffmpeg;
mod image_paddler;
mod pdf;
mod services;
mod sort_utils;
mod template_history;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_log::install_panic_hook();
    app_log::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(commands::build_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
