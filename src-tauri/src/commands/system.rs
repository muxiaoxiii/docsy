use crate::external::ExternalTool;

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_frontend_log(level: String, target: String, message: String, context: Option<String>) {
    let ctx = context
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null);
    let _ = crate::app_log::write_frontend(crate::app_log::FrontendLogEntry {
        level,
        target,
        message,
        context: Some(ctx),
    });
}

#[tauri::command]
pub fn get_log_file_path() -> Result<String, String> {
    crate::app_log::log_file_path().map(|p| p.display().to_string())
}

#[tauri::command]
pub fn open_log_file() -> Result<(), String> {
    let path = crate::app_log::log_file_path()?;
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let path = crate::app_log::log_dir()?;
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_diagnostic_info() -> Result<serde_json::Value, String> {
    let mut info = serde_json::Map::new();
    info.insert(
        "version".into(),
        serde_json::Value::String(env!("CARGO_PKG_VERSION").into()),
    );
    info.insert(
        "os".into(),
        serde_json::Value::String(std::env::consts::OS.into()),
    );
    info.insert(
        "arch".into(),
        serde_json::Value::String(std::env::consts::ARCH.into()),
    );

    let qpdf = crate::external::QpdfTool.check();
    info.insert(
        "qpdf".into(),
        serde_json::json!({ "available": qpdf.available, "version": qpdf.version }),
    );

    let ffmpeg = crate::external::FfmpegTool.check();
    info.insert(
        "ffmpeg".into(),
        serde_json::json!({ "available": ffmpeg.available, "version": ffmpeg.version }),
    );

    let poppler = crate::external::PopplerTool.check();
    info.insert(
        "poppler".into(),
        serde_json::json!({ "available": poppler.available, "version": poppler.version }),
    );

    Ok(serde_json::Value::Object(info))
}

#[tauri::command]
pub fn list_system_fonts() -> Result<Vec<String>, String> {
    crate::ffmpeg::detect::list_system_fonts().map_err(|e| e.to_string())
}
