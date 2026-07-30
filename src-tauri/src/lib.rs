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

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use tauri::Emitter;

/// Global app handle for emitting events from non-command contexts.
static APP_HANDLE: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

pub fn get_app_handle() -> Option<&'static tauri::AppHandle> {
    APP_HANDLE.get()
}

/// Shared state for long-running conversion operations.
/// When a COM conversion times out, the backend sets `timed_out` and waits for
/// the frontend to respond via `respond_conversion_timeout`.
pub struct ConversionState {
    /// Set to true when the conversion has timed out and is waiting for user response.
    pub timed_out: AtomicBool,
    /// User response: 0 = waiting, 1 = continue, 2 = cancel.
    pub response: AtomicU8,
    /// Process ID of the running conversion (for potential kill).
    pub pid: AtomicU64,
}

impl ConversionState {
    pub fn new() -> Self {
        Self {
            timed_out: AtomicBool::new(false),
            response: AtomicU8::new(0),
            pid: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.timed_out.store(false, Ordering::SeqCst);
        self.response.store(0, Ordering::SeqCst);
        self.pid.store(0, Ordering::SeqCst);
    }

    /// Called by backend: signal timeout and wait for user response.
    /// Returns true if user chose to continue, false if cancel.
    pub fn wait_for_user_response(&self, app: &tauri::AppHandle) -> bool {
        self.timed_out.store(true, Ordering::SeqCst);
        self.response.store(0, Ordering::SeqCst);

        // Emit event to frontend
        let _ = app.emit("docsy-conversion-timeout", ());

        // Poll for response (check every 500ms)
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            match self.response.load(Ordering::SeqCst) {
                1 => {
                    self.timed_out.store(false, Ordering::SeqCst);
                    return true; // continue
                }
                2 => {
                    self.timed_out.store(false, Ordering::SeqCst);
                    return false; // cancel
                }
                _ => continue, // still waiting
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_log::install_panic_hook();
    app_log::init();

    // Clean the previous WebKit NetworkCache without delaying the first window.
    // PDF.js workers can otherwise accumulate hundreds of megabytes across
    // sessions, but cache cleanup is not on the application critical path.
    std::thread::spawn(cleanup_webkit_cache);

    let conversion_state = Arc::new(ConversionState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(conversion_state)
        .setup(|app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            Ok(())
        })
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
