use serde::Serialize;
use crate::external::ExternalTool;

#[derive(Debug, Serialize)]
pub struct FfmpegStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub has_drawtext: bool,
}

#[tauri::command]
pub fn check_ffmpeg() -> FfmpegStatus {
    let tool = crate::external::FfmpegTool;
    let status = tool.check();
    FfmpegStatus {
        available: status.available,
        path: status.path,
        version: status.version,
        has_drawtext: false,
    }
}

#[tauri::command]
pub fn probe_video(path: String) -> Result<serde_json::Value, String> {
    crate::ffmpeg::probe::probe_video(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn extract_frames(args: serde_json::Value) -> Result<serde_json::Value, String> {
    crate::ffmpeg::extract::extract(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn try_brew_install_ffmpeg() -> Result<String, String> {
    crate::external::FfmpegTool.try_install().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn try_brew_install_qpdf() -> Result<String, String> {
    crate::external::QpdfTool.try_install().map_err(|e| e.to_string())
}
