#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(path);
    if !path.exists() {
        return Err("要打开的文件不存在".into());
    }
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "下载地址无效".to_string())?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("只能打开 HTTPS 下载地址".into());
    }
    open::that(parsed.as_str()).map_err(|e| e.to_string())
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
pub async fn read_image_data_url(path: String) -> Result<String, String> {
    crate::commands::run_blocking(move || preview_image_data_url(&path)).await
}

fn preview_image_data_url(path: &str) -> anyhow::Result<String> {
    use base64::Engine;
    use std::io::Cursor;

    const MAX_PREVIEW_EDGE: u32 = 1600;
    const MAX_SOURCE_PIXELS: u64 = 64_000_000;

    let path = std::path::PathBuf::from(path);
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tif" | "tiff" => {}
        _ => anyhow::bail!("不支持的图片格式"),
    };
    let (width, height) = image::image_dimensions(&path)
        .map_err(|error| anyhow::anyhow!("无法读取图片尺寸: {error}"))?;
    if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
        anyhow::bail!("图片像素过大，无法安全生成预览缩略图");
    }
    let image = image::open(&path).map_err(|error| anyhow::anyhow!("读取图片失败: {error}"))?;
    let preview = image.thumbnail(MAX_PREVIEW_EDGE, MAX_PREVIEW_EDGE);
    let mut bytes = Vec::new();
    preview
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
        .map_err(|error| anyhow::anyhow!("生成图片缩略图失败: {error}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/jpeg;base64,{encoded}"))
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
pub async fn list_system_fonts() -> Result<Vec<String>, String> {
    crate::commands::run_blocking(crate::ffmpeg::detect::list_system_fonts).await
}

/// Respond to a conversion timeout event.
/// `continue_waiting`: true = keep waiting, false = cancel the conversion.
#[tauri::command]
pub fn respond_conversion_timeout(
    state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
    continue_waiting: bool,
) -> Result<(), String> {
    state.set_response(if continue_waiting { 1 } else { 2 });
    Ok(())
}

/// Cancel a running operation by ID.
///
/// MDG-001: 优先使用 OperationManager（CancellationToken），
/// 回退到 SubprocessRegistry（PID kill）用于尚未迁移的外部子进程。
#[tauri::command]
pub fn cancel_operation(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    registry: tauri::State<'_, std::sync::Arc<crate::SubprocessRegistry>>,
    operation_id: String,
) -> Result<bool, String> {
    // 优先尝试 OperationManager（异步任务取消）
    if manager.cancel(&operation_id) {
        return Ok(true);
    }
    // 回退到 SubprocessRegistry（外部子进程 kill）
    Ok(registry.cancel(&operation_id))
}

/// List all currently active operations with metadata (for debugging and UI).
#[tauri::command]
pub fn list_active_operations(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
) -> Vec<crate::operations::ActiveOperation> {
    manager.list_active()
}
