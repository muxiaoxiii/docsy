use crate::external::ExternalTool;

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_frontend_log(level: String, target: String, message: String, context: Option<String>) {
    match level.as_str() {
        "error" => log::error!(target: &target, "{}{}", message, context.as_deref().unwrap_or("")),
        "warn" => log::warn!(target: &target, "{}{}", message, context.as_deref().unwrap_or("")),
        "info" => log::info!(target: &target, "{}{}", message, context.as_deref().unwrap_or("")),
        _ => log::debug!(target: &target, "{}{}", message, context.as_deref().unwrap_or("")),
    }
}

#[tauri::command]
pub fn get_log_file_path() -> Result<String, String> {
    crate::app_log::current_log_path()
        .map(|p| p.display().to_string())
        .ok_or_else(|| "日志路径不可用".to_string())
}

#[tauri::command]
pub fn open_log_file() -> Result<(), String> {
    let path = crate::app_log::current_log_path()
        .ok_or_else(|| "日志文件不存在".to_string())?;
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let path = crate::app_log::log_dir()
        .ok_or_else(|| "日志目录不存在".to_string())?;
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_diagnostic_info() -> Result<serde_json::Value, String> {
    let mut info = serde_json::Map::new();
    info.insert("version".into(), serde_json::Value::String(env!("CARGO_PKG_VERSION").into()));
    info.insert("os".into(), serde_json::Value::String(std::env::consts::OS.into()));
    info.insert("arch".into(), serde_json::Value::String(std::env::consts::ARCH.into()));

    let qpdf = crate::external::QpdfTool.check();
    info.insert("qpdf".into(), serde_json::json!({ "available": qpdf.available, "version": qpdf.version }));

    let ffmpeg = crate::external::FfmpegTool.check();
    info.insert("ffmpeg".into(), serde_json::json!({ "available": ffmpeg.available, "version": ffmpeg.version }));

    Ok(serde_json::Value::Object(info))
}

#[tauri::command]
pub fn list_system_fonts() -> Result<Vec<String>, String> {
    crate::ffmpeg::detect::list_system_fonts().map_err(|e| e.to_string())
}
