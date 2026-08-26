use crate::error::DocsyError;
use crate::external::ExternalTool;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FfmpegStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub has_drawtext: bool,
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<FfmpegStatus, DocsyError> {
    tauri::async_runtime::spawn_blocking(build_ffmpeg_status)
        .await
        .map_err(|e| DocsyError::Unknown {
            message: e.to_string(),
        })
}

fn build_ffmpeg_status() -> FfmpegStatus {
    let tool = crate::external::FfmpegTool;
    let status = tool.check();
    FfmpegStatus {
        available: status.available,
        path: status.path,
        version: status.version,
        has_drawtext: crate::ffmpeg::detect::has_drawtext().unwrap_or(false),
    }
}

#[tauri::command]
pub async fn probe_video(path: String) -> Result<serde_json::Value, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || crate::ffmpeg::probe::probe_video(&path))
        .await
        .map_err(|e| DocsyError::Unknown {
            message: e.to_string(),
        })?
        .map_err(|e| DocsyError::Unknown {
            message: e.to_string(),
        })
}

#[tauri::command]
pub async fn extract_frames(
    args: serde_json::Value,
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
) -> Result<serde_json::Value, String> {
    let operation_id = args
        .get("operation_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    super::run_managed(&manager, "extract_frames", operation_id, move |token| {
        crate::ffmpeg::extract::extract(&args, &token)
    })
    .await
}

#[tauri::command]
pub async fn list_output_frames(dir: String) -> Result<Vec<String>, String> {
    super::run_blocking(move || crate::ffmpeg::extract::list_output_frames(&dir)).await
}

#[tauri::command]
pub async fn analyze_frame_selection(
    args: crate::ffmpeg::selection::FrameSelectionArgs,
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
) -> Result<crate::ffmpeg::selection::FrameSelectionResult, String> {
    let operation_id = "analyze_frame_selection:auto".to_string();
    let progress_operation_id = operation_id.clone();
    let progress_manager = std::sync::Arc::clone(manager.inner());
    super::run_managed(
        &manager,
        "analyze_frame_selection",
        Some(operation_id),
        move |token| {
            let cancel_token = token.clone();
            crate::ffmpeg::selection::analyze_with_progress(
                args,
                move || cancel_token.is_cancelled(),
                move |phase, current, total| {
                    let label = match phase {
                        "features" => format!("正在读取图片特征… {current}/{total}"),
                        "compare" => format!("正在判断重复与前后连续性… {current}/{total}"),
                        _ => "正在整理智能筛选结果…".to_string(),
                    };
                    progress_manager.update(&progress_operation_id, label);
                },
            )
        },
    )
    .await
}
