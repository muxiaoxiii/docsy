use crate::error::DocsyError;

#[tauri::command]
pub fn open_path(path: String) -> Result<(), DocsyError> {
    let path = std::path::PathBuf::from(&path);
    if !path.exists() {
        return Err(DocsyError::FileNotFound {
            path: path.display().to_string(),
        });
    }
    open::that(&path).map_err(|e| DocsyError::Unknown {
        message: e.to_string(),
    })
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), DocsyError> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| DocsyError::InvalidArgument {
        message: "下载地址无效".into(),
    })?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err(DocsyError::InvalidArgument {
            message: "只能打开 HTTPS 下载地址".into(),
        });
    }
    open::that(parsed.as_str()).map_err(|e| DocsyError::Unknown {
        message: e.to_string(),
    })
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
pub fn get_log_file_path() -> Result<String, DocsyError> {
    crate::app_log::log_file_path()
        .map(|p| p.display().to_string())
        .map_err(|e| DocsyError::Unknown { message: e })
}

#[tauri::command]
pub fn open_log_file() -> Result<(), DocsyError> {
    let path = crate::app_log::log_file_path().map_err(|e| DocsyError::Unknown { message: e })?;
    open::that(&path).map_err(|e| DocsyError::Unknown {
        message: e.to_string(),
    })
}

#[tauri::command]
pub fn open_log_dir() -> Result<(), DocsyError> {
    let path = crate::app_log::log_dir().map_err(|e| DocsyError::Unknown { message: e })?;
    open::that(&path).map_err(|e| DocsyError::Unknown {
        message: e.to_string(),
    })
}

#[tauri::command]
pub fn export_diagnostic_report(
    frontend_snapshots: Option<serde_json::Value>,
) -> Result<serde_json::Value, DocsyError> {
    let log_dir = crate::app_log::log_dir().map_err(|e| DocsyError::Unknown { message: e })?;
    let report_name = format!(
        "diagnostic-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let report_path = log_dir.join(&report_name);

    let system_info = get_diagnostic_info_internal();
    let recent_logs = collect_recent_logs(1000);

    let report = serde_json::json!({
        "generated": chrono::Local::now().to_rfc3339(),
        "system": system_info,
        "recentLogs": recent_logs,
        "frontendSnapshots": frontend_snapshots.unwrap_or(serde_json::Value::Null),
    });

    let content =
        serde_json::to_string_pretty(&report).map_err(|e| DocsyError::InvalidArgument {
            message: format!("序列化诊断报告失败: {e}"),
        })?;
    std::fs::write(&report_path, &content).map_err(|e| DocsyError::Unknown {
        message: format!("写入诊断报告失败: {e}"),
    })?;

    Ok(serde_json::json!({ "path": report_path.display().to_string() }))
}

fn get_diagnostic_info_internal() -> serde_json::Value {
    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "debug": cfg!(debug_assertions),
    })
}

fn collect_recent_logs(max_lines: usize) -> Vec<String> {
    let Ok(path) = crate::app_log::log_file_path() else {
        return vec![];
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return vec![];
    };
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].iter().map(|s| s.to_string()).collect()
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
pub async fn get_diagnostic_info() -> Result<serde_json::Value, DocsyError> {
    tauri::async_runtime::spawn_blocking(build_diagnostic_info)
        .await
        .map_err(|e| DocsyError::Unknown {
            message: e.to_string(),
        })?
}

fn build_diagnostic_info() -> Result<serde_json::Value, DocsyError> {
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
) -> Result<(), DocsyError> {
    state.set_response(if continue_waiting { 1 } else { 2 });
    Ok(())
}

/// Cancel a running operation by ID.
///
/// MDG-001: 优先使用 OperationManager（CancellationToken），
/// 统一取消入口。
/// OperationManager: CancellationToken-based 异步任务取消（run_managed 注册的任务）
/// SubprocessRegistry: PID-based 外部子进程 kill（qpdf 等外部命令）
#[tauri::command]
pub fn cancel_operation(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    registry: tauri::State<'_, std::sync::Arc<crate::SubprocessRegistry>>,
    operation_id: String,
) -> Result<bool, DocsyError> {
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
