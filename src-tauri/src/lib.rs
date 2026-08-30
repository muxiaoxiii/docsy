mod app_log;
mod commands;
mod docx_template;
mod error;
mod external;
mod ffmpeg;
mod image_paddler;
mod markdown;
mod operations;
mod pdf;
mod services;
mod sort_utils;
mod template_history;
mod util;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

/// Registry of active subprocess PIDs for cancellation support.
/// Frontend can call `cancel_operation` to kill a subprocess by its operation ID.
#[derive(Default)]
pub struct SubprocessRegistry {
    /// Map from operation ID (e.g. "merge_pdfs:42") to subprocess PID.
    pids: Mutex<HashMap<String, u32>>,
}

impl SubprocessRegistry {
    pub fn new() -> Self {
        Self {
            pids: Mutex::new(HashMap::new()),
        }
    }

    /// Register a subprocess PID under the given operation ID.
    pub fn register(&self, operation_id: &str, pid: u32) {
        if let Ok(mut map) = self.pids.lock() {
            map.insert(operation_id.to_string(), pid);
        }
    }

    /// Unregister a subprocess (call when operation completes normally).
    pub fn unregister(&self, operation_id: &str) {
        if let Ok(mut map) = self.pids.lock() {
            map.remove(operation_id);
        }
    }

    /// Kill a subprocess by operation ID. Returns true if the process was found and killed.
    pub fn cancel(&self, operation_id: &str) -> bool {
        let pid = {
            if let Ok(mut map) = self.pids.lock() {
                map.remove(operation_id)
            } else {
                return false;
            }
        };
        if let Some(pid) = pid {
            kill_pid(pid);
            return true;
        }
        false
    }

    /// Check if an operation has been cancelled.
    pub fn is_cancelled(&self, operation_id: &str) -> bool {
        if let Ok(map) = self.pids.lock() {
            !map.contains_key(operation_id)
        } else {
            false
        }
    }

    pub fn has_active(&self) -> bool {
        self.pids.lock().map(|map| !map.is_empty()).unwrap_or(false)
    }

    /// Spawn a command, register its PID, wait for completion, unregister.
    /// This is the primary entry point for cancellable subprocesses.
    pub fn spawn_and_wait(
        &self,
        operation_id: &str,
        mut cmd: std::process::Command,
    ) -> std::io::Result<std::process::Output> {
        let child = cmd.spawn()?;
        let pid = child.id();
        self.register(operation_id, pid);
        let result = child.wait_with_output();
        self.unregister(operation_id);
        result
    }

    /// Kill all registered subprocesses (for app shutdown cleanup).
    pub fn cancel_all(&self) {
        let pids: Vec<u32> = if let Ok(mut map) = self.pids.lock() {
            map.drain().map(|(_, pid)| pid).collect()
        } else {
            return;
        };
        for pid in pids {
            kill_pid(pid);
        }
    }
}

/// Kill a process by PID. SIGTERM first (graceful), SIGKILL after 2s if still alive.
fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(2);
        while start.elapsed() < timeout {
            if let Ok(status) = std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
            {
                if !status.success() {
                    return; // Process has exited
                }
            } else {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let _ = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
}

/// Global app handle for emitting events from non-command contexts.
static APP_HANDLE: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// Global subprocess registry for cancellation support.
/// Set once during app initialization; qpdf/ffmpeg can access it without Tauri state.
static SUBPROCESS_REGISTRY: std::sync::OnceLock<Arc<SubprocessRegistry>> =
    std::sync::OnceLock::new();

/// Coordinates the asynchronous close confirmation shown by the frontend.
/// Every native close request is intercepted once; the confirmed retry is
/// allowed through without prompting again.
#[derive(Default)]
pub struct CloseRequestState {
    allow_next_close: AtomicBool,
}

impl CloseRequestState {
    pub fn allow_next_close(&self) {
        self.allow_next_close.store(true, Ordering::SeqCst);
    }

    pub fn take_allowed_close(&self) -> bool {
        self.allow_next_close.swap(false, Ordering::SeqCst)
    }
}

pub fn get_subprocess_registry() -> Option<&'static Arc<SubprocessRegistry>> {
    SUBPROCESS_REGISTRY.get()
}

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
    /// MDG-001: 改用 Condvar 替代忙等待轮询。
    response: std::sync::Mutex<u8>,
    response_cvar: std::sync::Condvar,
    /// Process ID of the running conversion (for potential kill).
    pub pid: AtomicU64,
}

impl ConversionState {
    pub fn new() -> Self {
        Self {
            timed_out: AtomicBool::new(false),
            response: std::sync::Mutex::new(0),
            response_cvar: std::sync::Condvar::new(),
            pid: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.timed_out.store(false, Ordering::SeqCst);
        *self.response.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        self.pid.store(0, Ordering::SeqCst);
    }

    /// 设置用户响应并通知等待线程。
    pub fn set_response(&self, value: u8) {
        let mut state = self.response.lock().unwrap_or_else(|e| e.into_inner());
        *state = value;
        self.response_cvar.notify_all();
    }

    /// Called by backend: signal timeout and wait for user response.
    /// Returns true if user chose to continue, false if cancel.
    /// MDG-001: 使用 Condvar 阻塞等待，不再忙等待轮询。
    pub fn wait_for_user_response(&self, app: &tauri::AppHandle) -> bool {
        self.timed_out.store(true, Ordering::SeqCst);
        *self.response.lock().unwrap_or_else(|e| e.into_inner()) = 0;

        // Emit event to frontend
        let _ = app.emit("docsy-conversion-timeout", ());

        // 此方法只从 spawn_blocking 工作线程调用；等待用户明确选择，避免
        // 用户暂时离开时被静默当作“取消转换”。
        let mut state = self.response.lock().unwrap_or_else(|e| e.into_inner());
        while *state == 0 {
            state = self
                .response_cvar
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
        self.timed_out.store(false, Ordering::SeqCst);
        *state == 1 // true = continue, false = cancel
    }
}

impl Default for ConversionState {
    fn default() -> Self {
        Self::new()
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
    let subprocess_registry = Arc::new(SubprocessRegistry::new());
    let operation_manager = Arc::new(operations::OperationManager::new());
    let close_request_state = Arc::new(CloseRequestState::default());
    let operation_manager_for_setup = operation_manager.clone();
    let operation_manager_for_close = operation_manager.clone();
    let subprocess_registry_for_close = subprocess_registry.clone();
    let close_request_state_for_event = close_request_state.clone();
    let _ = SUBPROCESS_REGISTRY.set(subprocess_registry.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(conversion_state)
        .manage(subprocess_registry)
        .manage(operation_manager)
        .manage(close_request_state)
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if close_request_state_for_event.take_allowed_close() {
                    return;
                }

                api.prevent_close();
                let backend_active = !operation_manager_for_close.list_active().is_empty()
                    || subprocess_registry_for_close.has_active();
                let _ = window.emit(
                    "docsy-close-requested",
                    serde_json::json!({ "backendActive": backend_active }),
                );
            }
        })
        .setup(move |app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            operation_manager_for_setup.set_app_handle(app.handle().clone());
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

#[cfg(test)]
mod close_request_tests {
    use super::*;

    #[test]
    fn confirmed_close_is_consumed_once() {
        let state = CloseRequestState::default();
        assert!(!state.take_allowed_close());
        state.allow_next_close();
        assert!(state.take_allowed_close());
        assert!(!state.take_allowed_close());
    }

    #[test]
    fn subprocess_registry_reports_active_work() {
        let registry = SubprocessRegistry::new();
        assert!(!registry.has_active());
        registry.register("test", 42);
        assert!(registry.has_active());
        registry.unregister("test");
        assert!(!registry.has_active());
    }
}
