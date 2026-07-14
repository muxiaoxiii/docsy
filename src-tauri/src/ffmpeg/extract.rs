use crate::external::ExternalTool;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn extract(args: &serde_json::Value) -> Result<serde_json::Value> {
    let started = Instant::now();
    let ffmpeg = crate::external::FfmpegTool;
    let bin = ffmpeg.binary_path()?;

    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .context("缺少视频文件")?;
    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("视频文件不存在: {}", input);
    }

    let output_dir = output_dir_for(input_path, args)?;
    std::fs::create_dir_all(&output_dir).context("创建抽帧输出目录失败")?;

    let format = normalized_format(args);
    let prefix = output_prefix_for(input_path, args);
    let output_pattern = output_dir.join(format!("{prefix}_frame_%04d.{format}"));
    let fps = args.get("fps").and_then(|v| v.as_f64()).unwrap_or(1.0);
    if !fps.is_finite() || fps <= 0.0 {
        anyhow::bail!("抽帧频率必须大于 0");
    }

    let mut filters = vec![format!("fps={fps}")];
    if let Some(drawtext) = drawtext_filter(args) {
        filters.push(drawtext);
    }

    let mut cmd = std::process::Command::new(&bin);
    cmd.arg("-hide_banner")
        .arg("-y")
        .arg("-i")
        .arg(input_path)
        .arg("-vf")
        .arg(filters.join(","));

    match format.as_str() {
        "jpg" => {
            let quality = args.get("quality").and_then(|v| v.as_u64()).unwrap_or(90);
            cmd.arg("-q:v").arg(jpeg_qscale(quality).to_string());
        }
        "png" => {
            cmd.arg("-compression_level").arg("6");
        }
        _ => {}
    }

    let output = cmd.arg(&output_pattern).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg 抽帧失败: {}", stderr.trim());
    }

    let frames = list_output_frames_path(&output_dir)?;
    Ok(serde_json::json!({
        "output_dir": output_dir.display().to_string(),
        "count": frames.len(),
        "elapsed": started.elapsed().as_millis(),
        "frames": frames,
    }))
}

pub fn list_output_frames(dir: &str) -> Result<Vec<String>> {
    list_output_frames_path(Path::new(dir))
}

fn output_dir_for(input: &Path, args: &serde_json::Value) -> Result<PathBuf> {
    if let Some(dir) = args
        .get("output_dir")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
    {
        return Ok(PathBuf::from(dir));
    }

    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    Ok(parent
        .join("_docsy_video_frames")
        .join(format!("{}_{}", sanitize_name(stem), ts)))
}

fn output_prefix_for(input: &Path, args: &serde_json::Value) -> String {
    if let Some(prefix) = args
        .get("filename_prefix")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return sanitize_name(prefix);
    }

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    sanitize_name(&format!("{stem}_{ts}"))
}

fn normalized_format(args: &serde_json::Value) -> String {
    match args.get("format").and_then(|v| v.as_str()) {
        Some("png") => "png".into(),
        _ => "jpg".into(),
    }
}

fn jpeg_qscale(quality: u64) -> u64 {
    let quality = quality.clamp(1, 100);
    31 - ((quality - 1) * 29 / 99)
}

fn drawtext_filter(args: &serde_json::Value) -> Option<String> {
    let ts = args.get("timestamp")?;
    if !ts.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }

    let color = match ts.get("color").and_then(|v| v.as_str()).unwrap_or("white") {
        "black" => "black",
        "red" => "red",
        "yellow" => "yellow",
        "green" => "lime",
        _ => "white",
    };
    let (x, y) = match ts
        .get("position")
        .and_then(|v| v.as_str())
        .unwrap_or("top-left")
    {
        "top-right" => ("w-tw-16", "16"),
        "bottom-left" => ("16", "h-th-16"),
        "bottom-right" => ("w-tw-16", "h-th-16"),
        _ => ("16", "16"),
    };

    Some(format!(
        "drawtext=text='%{{pts\\:hms}}':x={x}:y={y}:fontcolor={color}:fontsize=24:box=1:boxcolor=black@0.45:boxborderw=6"
    ))
}

fn list_output_frames_path(dir: &Path) -> Result<Vec<String>> {
    if !dir.is_dir() {
        anyhow::bail!("输出目录不存在: {}", dir.display());
    }

    let mut frames = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "jpg" | "jpeg" | "png") {
            frames.push(path.display().to_string());
        }
    }
    frames.sort();
    Ok(frames)
}

fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric()
                || matches!(ch, '-' | '_')
                || ('\u{4e00}'..='\u{9fff}').contains(&ch)
            {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "video".into()
    } else {
        sanitized
    }
}
