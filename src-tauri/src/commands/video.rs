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
pub async fn check_ffmpeg() -> Result<FfmpegStatus, String> {
    tauri::async_runtime::spawn_blocking(build_ffmpeg_status)
        .await
        .map_err(|e| e.to_string())
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
pub async fn probe_video(path: String) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || crate::ffmpeg::probe::probe_video(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_frames(args: serde_json::Value) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || crate::ffmpeg::extract::extract(&args))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_output_frames(dir: String) -> Result<Vec<String>, String> {
    crate::ffmpeg::extract::list_output_frames(&dir).map_err(|e| e.to_string())
}
