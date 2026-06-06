//! 视频信息读取

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::detect::{build_cmd, resolve_ffprobe_command};

/// 视频信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub path: String,
    pub duration: f64, // 秒
    pub width: u32,
    pub height: u32,
    pub fps: f64, // 帧率
    pub codec: String,
    pub size_bytes: u64,
    pub format_name: String,
}

/// ffprobe 输出的 JSON 结构
#[derive(Debug, Deserialize)]
struct ProbeOutput {
    streams: Option<Vec<ProbeStream>>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    codec_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
    size: Option<String>,
    format_name: Option<String>,
}

/// 读取视频信息
pub fn probe_video(path: &Path) -> Result<VideoInfo, String> {
    if !path.exists() {
        return Err(format!("视频文件不存在：{}", path.display()));
    }

    let program = resolve_ffprobe_command();
    let mut cmd = build_cmd(&program);
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
    ]);
    cmd.arg(path);

    let out = cmd.output().map_err(|e| format!("ffprobe 调用失败：{e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("ffprobe 失败：{stderr}"));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let probe: ProbeOutput =
        serde_json::from_str(&stdout).map_err(|e| format!("解析 ffprobe 输出失败：{e}"))?;

    // 查找视频流
    let video_stream = probe.streams.as_ref().and_then(|streams| {
        streams
            .iter()
            .find(|s| s.codec_type.as_deref() == Some("video"))
    });

    let width = video_stream
        .and_then(|s| s.width)
        .ok_or("无法获取视频宽度")?;

    let height = video_stream
        .and_then(|s| s.height)
        .ok_or("无法获取视频高度")?;

    let fps = video_stream
        .and_then(|s| s.r_frame_rate.as_deref())
        .and_then(parse_fps)
        .unwrap_or(30.0); // 默认 30fps

    let codec = video_stream
        .and_then(|s| s.codec_name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let format = probe.format.unwrap_or_default();
    let duration = format
        .duration
        .as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let size_bytes = format
        .size
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let format_name = format.format_name.unwrap_or_else(|| "unknown".to_string());

    Ok(VideoInfo {
        path: path.display().to_string(),
        duration,
        width,
        height,
        fps,
        codec,
        size_bytes,
        format_name,
    })
}

/// 解析帧率字符串（如 "30/1"、"24000/1001"）
fn parse_fps(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().ok()?;
        let den: f64 = parts[1].parse().ok()?;
        if den > 0.0 {
            return Some(num / den);
        }
    } else {
        return s.parse::<f64>().ok();
    }
    None
}

impl Default for ProbeFormat {
    fn default() -> Self {
        Self {
            duration: None,
            size: None,
            format_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fps() {
        assert_eq!(parse_fps("30/1"), Some(30.0));
        assert_eq!(parse_fps("24000/1001"), Some(23.976023976023978));
        assert_eq!(parse_fps("60"), Some(60.0));
        assert_eq!(parse_fps("0/0"), None);
    }
}
