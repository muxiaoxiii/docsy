use crate::external::ExternalTool;
use anyhow::{Context, Result};
use std::time::Duration;

const FFPROBE_IDLE_TIMEOUT: Duration = Duration::from_secs(45);

pub fn probe_video(path: &str) -> Result<serde_json::Value> {
    let ffmpeg = crate::external::FfmpegTool;
    let bin = ffmpeg.binary_path()?;
    let ffprobe_name = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    let ffprobe = bin
        .parent()
        .unwrap_or(&std::path::PathBuf::from("."))
        .join(ffprobe_name);

    let mut command = std::process::Command::new(&ffprobe);
    command
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path);
    let output =
        crate::external::command_output_with_idle_timeout(&mut command, FFPROBE_IDLE_TIMEOUT)?;

    if !output.status.success() {
        anyhow::bail!("ffprobe 失败");
    }

    let info: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    flatten_probe_info(&info, path)
}

fn flatten_probe_info(info: &serde_json::Value, path: &str) -> Result<serde_json::Value> {
    let streams = info
        .get("streams")
        .and_then(|v| v.as_array())
        .context("ffprobe 输出缺少 streams")?;
    let format = info.get("format").unwrap_or(&serde_json::Value::Null);
    let video = streams
        .iter()
        .find(|stream| {
            stream
                .get("codec_type")
                .and_then(|v| v.as_str())
                .is_some_and(|v| v == "video")
        })
        .context("未找到视频流")?;

    let duration = format
        .get("duration")
        .or_else(|| video.get("duration"))
        .and_then(parse_f64)
        .unwrap_or(0.0);
    let size = format.get("size").and_then(parse_u64).unwrap_or(0);
    let width = video.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
    let height = video.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
    let fps = video
        .get("avg_frame_rate")
        .or_else(|| video.get("r_frame_rate"))
        .and_then(|v| v.as_str())
        .and_then(parse_ratio)
        .unwrap_or(0.0);
    let codec = video
        .get("codec_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(serde_json::json!({
        "path": path,
        "duration": duration,
        "width": width,
        "height": height,
        "fps": fps,
        "codec": codec,
        "size": size,
        "raw": info,
    }))
}

fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn parse_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<u64>().ok()))
}

fn parse_ratio(value: &str) -> Option<f64> {
    let (num, den) = value.split_once('/')?;
    let num = num.parse::<f64>().ok()?;
    let den = den.parse::<f64>().ok()?;
    if den == 0.0 {
        return None;
    }
    Some(num / den)
}
