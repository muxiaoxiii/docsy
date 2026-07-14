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
pub fn read_image_data_url(path: String) -> Result<String, String> {
    use base64::Engine;

    let path = std::path::PathBuf::from(&path);
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    };
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

#[tauri::command]
pub async fn get_diagnostic_info() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(build_diagnostic_info)
        .await
        .map_err(|e| e.to_string())?
}

fn build_diagnostic_info() -> Result<serde_json::Value, String> {
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

    Ok(serde_json::Value::Object(info))
}

#[tauri::command]
pub fn list_system_fonts() -> Result<Vec<String>, String> {
    crate::ffmpeg::detect::list_system_fonts().map_err(|e| e.to_string())
}
