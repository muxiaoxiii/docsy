use crate::external::ExternalTool;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const FFMPEG_EXTRACT_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub fn extract(args: &serde_json::Value, token: &tokio_util::sync::CancellationToken) -> Result<serde_json::Value> {
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
    let run_id = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let temp_prefix = format!(".docsy_tmp_{run_id}");
    let output_pattern = output_dir.join(format!("{temp_prefix}_%06d.{format}"));
    let fps = args.get("fps").and_then(|v| v.as_f64()).unwrap_or(1.0);
    if !fps.is_finite() || fps <= 0.0 {
        anyhow::bail!("抽帧频率必须大于 0");
    }
    let time_range = time_range_args(args)?;

    let mut filters = vec![format!("fps={fps}")];
    if let Some(drawtext) = drawtext_filter(args) {
        if !crate::ffmpeg::detect::has_drawtext()? {
            anyhow::bail!("当前 FFmpeg 不支持 drawtext，无法添加时间戳水印。请关闭水印或更换支持 drawtext 的 FFmpeg");
        }
        filters.push(drawtext);
    }

    let mut cmd = crate::external::hidden_command(&bin);
    cmd.arg("-hide_banner").arg("-y");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(start) = time_range.start {
        cmd.arg("-ss").arg(format_seconds_arg(start));
    }
    cmd.arg("-i").arg(input_path);
    if let Some(duration) = time_range.duration {
        cmd.arg("-t").arg(format_seconds_arg(duration));
    }
    cmd.arg("-vf").arg(filters.join(","));

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

    cmd.arg(&output_pattern);

    let mut child = cmd.spawn().context("启动 ffmpeg 进程失败")?;
    let stderr = child.stderr.take().context("无法捕获 ffmpeg stderr")?;

    // 独立线程逐行读取 stderr
    let (tx, rx) = mpsc::channel::<String>();
    let stderr_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 主循环：检查取消 + 空闲超时 + 进度解析
    let mut last_activity = Instant::now();
    let mut exit_status = None;
    let mut stderr_lines: Vec<String> = Vec::new();

    loop {
        // 检查取消
        if token.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_thread.join();
            cleanup_temp_files(&output_dir, &temp_prefix);
            return Ok(serde_json::json!({
                "cancelled": true,
                "count": 0,
                "elapsed": started.elapsed().as_millis(),
            }));
        }

        // 非阻塞接收 stderr 行
        match rx.try_recv() {
            Ok(line) => {
                last_activity = Instant::now();
                stderr_lines.push(line);
            }
            Err(mpsc::TryRecvError::Empty) => {
                // 检查空闲超时
                if last_activity.elapsed() > FFMPEG_EXTRACT_IDLE_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stderr_thread.join();
                    cleanup_temp_files(&output_dir, &temp_prefix);
                    anyhow::bail!("ffmpeg 抽帧超时：{}秒内无输出", FFMPEG_EXTRACT_IDLE_TIMEOUT.as_secs());
                }

                // 检查子进程是否已退出
                match child.try_wait() {
                    Ok(Some(status)) => {
                        exit_status = Some(status);
                        break;
                    }
                    Ok(None) => {
                        // 进程仍在运行，短暂休眠避免忙等
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        let _ = child.kill();
                        let _ = stderr_thread.join();
                        anyhow::bail!("检查 ffmpeg 进程状态失败: {}", e);
                    }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // stderr 线程结束，等待子进程退出
                match child.wait() {
                    Ok(status) => {
                        exit_status = Some(status);
                        break;
                    }
                    Err(e) => anyhow::bail!("等待 ffmpeg 进程退出失败: {}", e),
                }
            }
        }
    }

    let _ = stderr_thread.join();

    let status = exit_status.context("ffmpeg 进程状态未知")?;
    if !status.success() {
        let stderr_text = stderr_lines.join("\n");
        anyhow::bail!("ffmpeg 抽帧失败: {}", stderr_text.trim());
    }

    let frames = rename_extracted_frames(
        &output_dir,
        &temp_prefix,
        &prefix,
        &format,
        fps,
        time_range.start.unwrap_or(0.0),
    )?;
    Ok(serde_json::json!({
        "output_dir": output_dir.display().to_string(),
        "count": frames.len(),
        "elapsed": started.elapsed().as_millis(),
        "frames": frames,
        "cancelled": false,
    }))
}

fn cleanup_temp_files(output_dir: &Path, temp_prefix: &str) {
    if let Ok(entries) = std::fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(temp_prefix) {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TimeRange {
    start: Option<f64>,
    duration: Option<f64>,
}

fn time_range_args(args: &serde_json::Value) -> Result<TimeRange> {
    let start = parse_time_arg(
        args.get("start_time").or_else(|| args.get("startTime")),
        "开始时间",
    )?;
    let end = parse_time_arg(
        args.get("end_time").or_else(|| args.get("endTime")),
        "结束时间",
    )?;
    if let Some(value) = start {
        if !value.is_finite() || value < 0.0 {
            anyhow::bail!("开始时间不能为负数");
        }
    }
    if let Some(value) = end {
        if !value.is_finite() || value < 0.0 {
            anyhow::bail!("结束时间不能为负数");
        }
    }
    let duration = match (start, end) {
        (Some(start), Some(end)) if end <= start => anyhow::bail!("结束时间必须晚于开始时间"),
        (Some(start), Some(end)) => Some(end - start),
        (None, Some(end)) => Some(end),
        _ => None,
    };
    Ok(TimeRange { start, duration })
}

fn parse_time_arg(value: Option<&serde_json::Value>, label: &str) -> Result<Option<f64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(number) = value.as_f64() {
        return Ok(Some(number));
    }
    let text = value
        .as_str()
        .with_context(|| format!("{label}格式无效，请输入秒数或 HH:MM:SS"))?
        .trim();
    if text.is_empty() {
        return Ok(None);
    }
    parse_time_text(text)
        .map(Some)
        .with_context(|| format!("{label}格式无效，请输入秒数或 HH:MM:SS"))
}

fn parse_time_text(text: &str) -> Option<f64> {
    if let Ok(seconds) = text.parse::<f64>() {
        return seconds.is_finite().then_some(seconds);
    }
    let parts: Vec<&str> = text.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let values: Vec<f64> = parts
        .iter()
        .map(|part| part.parse::<f64>().ok())
        .collect::<Option<_>>()?;
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return None;
    }
    if values[values.len() - 1] >= 60.0 || values[values.len() - 2] >= 60.0 {
        return None;
    }
    Some(match values.as_slice() {
        [minutes, seconds] => minutes * 60.0 + seconds,
        [hours, minutes, seconds] => hours * 3600.0 + minutes * 60.0 + seconds,
        _ => return None,
    })
}

fn format_seconds_arg(seconds: f64) -> String {
    format!("{:.3}", seconds.max(0.0))
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
    sanitize_name(stem)
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

fn rename_extracted_frames(
    output_dir: &Path,
    temp_prefix: &str,
    prefix: &str,
    format: &str,
    fps: f64,
    timeline_offset_seconds: f64,
) -> Result<Vec<String>> {
    let mut temp_frames = Vec::new();
    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        if name.starts_with(temp_prefix) {
            temp_frames.push(path);
        }
    }
    temp_frames.sort();

    let mut frames = Vec::with_capacity(temp_frames.len());
    for (idx, path) in temp_frames.into_iter().enumerate() {
        let seq = idx + 1;
        let seconds = timeline_offset_seconds + idx as f64 / fps;
        let time = format_frame_time(seconds);
        let file_name = format!("{prefix}_{time}_frame_{seq:04}.{format}");
        let target = unique_frame_path(output_dir, &file_name);
        std::fs::rename(&path, &target).with_context(|| {
            format!(
                "重命名抽帧文件失败: {} -> {}",
                path.display(),
                target.display()
            )
        })?;
        frames.push(target.display().to_string());
    }
    Ok(frames)
}

fn unique_frame_path(output_dir: &Path, file_name: &str) -> PathBuf {
    let candidate = output_dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or("frame");
    let ext = path.extension().and_then(|v| v.to_str()).unwrap_or("jpg");
    for idx in 2.. {
        let candidate = output_dir.join(format!("{stem}_{idx}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn format_frame_time(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let secs = (total_ms % 60_000) / 1000;
    let millis = total_ms % 1000;
    format!("{hours:02}_{minutes:02}_{secs:02}_{millis:03}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_prefix_uses_video_name_without_run_timestamp() {
        let args = serde_json::json!({});
        let prefix = output_prefix_for(Path::new("/tmp/证据 视频.mp4"), &args);
        assert_eq!(prefix, "证据_视频");
    }

    #[test]
    fn frame_time_is_filename_safe_and_precise() {
        assert_eq!(format_frame_time(0.0), "00_00_00_000");
        assert_eq!(format_frame_time(3.5), "00_00_03_500");
        assert_eq!(format_frame_time(3661.25), "01_01_01_250");
    }

    #[test]
    fn parses_video_time_range() {
        let range = time_range_args(&serde_json::json!({
            "startTime": "01:30:00",
            "endTime": "01:35:30"
        }))
        .unwrap();
        assert_eq!(range.start, Some(5400.0));
        assert_eq!(range.duration, Some(330.0));
    }

    #[test]
    fn rejects_reversed_video_time_range() {
        let err = time_range_args(&serde_json::json!({
            "startTime": "10",
            "endTime": "5"
        }))
        .unwrap_err();
        assert!(err.to_string().contains("结束时间必须晚于开始时间"));
    }

    #[test]
    fn rejects_nonempty_invalid_video_time() {
        let err = time_range_args(&serde_json::json!({ "startTime": "tomorrow" })).unwrap_err();
        assert!(err.to_string().contains("开始时间格式无效"));
        let err = time_range_args(&serde_json::json!({ "endTime": "01:99" })).unwrap_err();
        assert!(err.to_string().contains("结束时间格式无效"));
    }
}
