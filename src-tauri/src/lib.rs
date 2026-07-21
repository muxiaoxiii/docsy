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

    // Clean WebKit NetworkCache on startup to prevent unbounded growth.
    // pdfjs-dist workers (~1-2MB each) are cached on every PDF preview load.
    cleanup_webkit_cache();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(commands::build_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn cleanup_webkit_cache() {
    let cache_dir = dirs::cache_dir()
        .or_else(|| dirs::home_dir().map(|p| p.join("Library").join("Caches")))
        .unwrap_or_default()
        .join("docsy")
        .join("WebKit")
        .join("NetworkCache");
    if cache_dir.exists() {
        let _ = std::fs::remove_dir_all(&cache_dir);
    }
}
