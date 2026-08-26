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
pub async fn compose_log_email() -> Result<serde_json::Value, DocsyError> {
    crate::commands::run_blocking(compose_log_email_impl)
        .await
        .map_err(|message| DocsyError::Unknown { message })
}

fn compose_log_email_impl() -> anyhow::Result<serde_json::Value> {
    let source_log_path = crate::app_log::log_file_path().map_err(anyhow::Error::msg)?;
    if !source_log_path.exists() {
        anyhow::bail!("当前日志文件尚未生成");
    }
    let log_path =
        crate::app_log::create_sanitized_log_copy(&source_log_path).map_err(anyhow::Error::msg)?;

    let recipient = "oonlyxin@outlook.com";
    let subject = format!(
        "Docsy {} 运行日志（{}）",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    let body = "你好，附件是 Docsy 自动生成的脱敏运行日志，文件路径和文件名已替换。\n\n问题描述：\n复现步骤：\n期望结果：\n实际结果：\n";

    if compose_email_with_attachment(recipient, &subject, body, &log_path).is_ok() {
        return Ok(serde_json::json!({
            "attached": true,
            "logPath": log_path.display().to_string(),
            "message": "已创建邮件草稿并附加脱敏日志，请检查后发送"
        }));
    }

    open_email_fallback(recipient, &subject, body, &log_path)?;
    Ok(serde_json::json!({
        "attached": false,
        "logPath": log_path.display().to_string(),
        "message": "邮件客户端不支持自动添加附件，已创建邮件草稿并定位脱敏日志，请手动添加后发送"
    }))
}

#[cfg(target_os = "macos")]
fn compose_email_with_attachment(
    recipient: &str,
    subject: &str,
    body: &str,
    log_path: &std::path::Path,
) -> anyhow::Result<()> {
    let script = r#"
on run argv
    set recipientAddress to item 1 of argv
    set messageSubject to item 2 of argv
    set messageBody to item 3 of argv
    set attachmentFile to POSIX file (item 4 of argv)
    tell application "Mail"
        set draftMessage to make new outgoing message with properties {subject:messageSubject, content:messageBody, visible:true}
        tell draftMessage
            make new to recipient at end of to recipients with properties {address:recipientAddress}
            tell content
                make new attachment with properties {file name:attachmentFile} at after last paragraph
            end tell
        end tell
        activate
    end tell
end run
"#;
    let output = crate::external::hidden_command("osascript")
        .arg("-e")
        .arg(script)
        .arg(recipient)
        .arg(subject)
        .arg(body)
        .arg(log_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(crate::external::command_failure_detail(&output));
    }
    Ok(())
}

#[cfg(windows)]
fn compose_email_with_attachment(
    recipient: &str,
    subject: &str,
    body: &str,
    log_path: &std::path::Path,
) -> anyhow::Result<()> {
    fn ps_literal(value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $outlook=New-Object -ComObject Outlook.Application; \
         $mail=$outlook.CreateItem(0); \
         $mail.To={}; $mail.Subject={}; $mail.Body={}; \
         [void]$mail.Attachments.Add({}); $mail.Display()",
        ps_literal(recipient),
        ps_literal(subject),
        ps_literal(body),
        ps_literal(&log_path.display().to_string()),
    );
    let output = crate::external::hidden_command("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(&script)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(crate::external::command_failure_detail(&output));
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn compose_email_with_attachment(
    _recipient: &str,
    _subject: &str,
    _body: &str,
    _log_path: &std::path::Path,
) -> anyhow::Result<()> {
    anyhow::bail!("当前平台不支持自动添加邮件附件")
}

fn open_email_fallback(
    recipient: &str,
    subject: &str,
    body: &str,
    log_path: &std::path::Path,
) -> anyhow::Result<()> {
    let mut mailto = reqwest::Url::parse(&format!("mailto:{recipient}"))?;
    mailto
        .query_pairs_mut()
        .append_pair("subject", subject)
        .append_pair("body", body);
    open::that(mailto.as_str())?;

    #[cfg(target_os = "macos")]
    {
        let _ = crate::external::hidden_command("open")
            .arg("-R")
            .arg(log_path)
            .spawn();
    }
    #[cfg(windows)]
    {
        let argument = format!("/select,{}", log_path.display());
        let _ = crate::external::hidden_command("explorer")
            .arg(argument)
            .spawn();
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    if let Some(parent) = log_path.parent() {
        let _ = open::that(parent);
    }
    Ok(())
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
pub async fn read_image_data_url(path: String, max_edge: Option<u32>) -> Result<String, String> {
    crate::commands::run_blocking(move || preview_image_data_url(&path, max_edge)).await
}

fn preview_image_data_url(path: &str, max_edge: Option<u32>) -> anyhow::Result<String> {
    use base64::Engine;
    use std::io::Cursor;

    const DEFAULT_PREVIEW_EDGE: u32 = 1600;
    const MAX_SOURCE_PIXELS: u64 = 64_000_000;
    // 源文件大小上限，防止前端传任意大文件读爆内存
    const MAX_SOURCE_BYTES: u64 = 50 * 1024 * 1024;

    let path = std::path::PathBuf::from(path);
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // 扩展名白名单（大小写不敏感）：仅允许常见图片格式
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "ico" | "tif" | "tiff" => {}
        _ => anyhow::bail!("不支持的图片格式"),
    };
    let file_size = std::fs::metadata(&path)
        .map_err(|error| anyhow::anyhow!("无法读取图片文件: {error}"))?
        .len();
    if file_size > MAX_SOURCE_BYTES {
        anyhow::bail!("图片文件过大（超过 50MB），无法生成预览");
    }
    let (width, height) = image::image_dimensions(&path)
        .map_err(|error| anyhow::anyhow!("无法读取图片尺寸: {error}"))?;
    if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
        anyhow::bail!("图片像素过大，无法安全生成预览缩略图");
    }
    let image = image::open(&path).map_err(|error| anyhow::anyhow!("读取图片失败: {error}"))?;
    let preview_edge = max_edge
        .unwrap_or(DEFAULT_PREVIEW_EDGE)
        .clamp(160, DEFAULT_PREVIEW_EDGE);
    let preview = image.thumbnail(preview_edge, preview_edge);
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

/// Close the application after the frontend has confirmed that running work
/// may be interrupted. The first native close request is always intercepted in
/// `lib.rs`; this command marks the retry as approved and performs coordinated
/// cancellation before asking the window to close again.
#[tauri::command]
pub fn confirm_app_close(
    window: tauri::Window,
    close_state: tauri::State<'_, std::sync::Arc<crate::CloseRequestState>>,
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    registry: tauri::State<'_, std::sync::Arc<crate::SubprocessRegistry>>,
    conversion_state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
) -> Result<(), DocsyError> {
    manager.cancel_all();
    registry.cancel_all();
    if conversion_state
        .timed_out
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        conversion_state.set_response(2);
    }
    close_state.allow_next_close();
    if let Err(err) = window.close() {
        // Do not leave a one-shot close permission armed after a failed close.
        close_state.take_allowed_close();
        return Err(DocsyError::Unknown {
            message: format!("关闭应用失败：{err}"),
        });
    }
    Ok(())
}
