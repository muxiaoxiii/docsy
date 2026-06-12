//! 视频抽帧执行

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::detect::{build_cmd, resolve_ffmpeg_command};

/// 抽帧参数
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractArgs {
    pub input: String,
    pub output_dir: Option<String>,
    pub fps_mode: String, // "per_second" / "interval"
    pub fps_value: f64,
    pub format: String,                  // "jpg" / "png"
    pub quality: u8,                     // 1-100
    pub filename_prefix: Option<String>, // 自定义文件名前缀
    pub timestamp: TimestampConfig,
}

/// 时间轴配置
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampConfig {
    pub enabled: bool,
    pub x_percent: f64, // 0-100
    pub y_percent: f64, // 0-100
    pub font_size: u32,
    pub font_color: String,
    pub font_file: String, // 字体文件路径，空则用默认
    pub bg_color: String,  // 如 "black@0.6"
    pub bg_padding: u32,
}

/// 抽帧结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResult {
    pub output_dir: String,
    pub total_frames: usize,
    pub extracted_frames: usize,
    pub output_files: Vec<String>,
    pub command: String, // 执行的 ffmpeg 命令
}

/// 执行抽帧
pub fn extract_frames(args: &ExtractArgs) -> Result<ExtractResult, String> {
    let input = Path::new(&args.input);
    if !input.exists() {
        return Err(format!("视频文件不存在：{}", input.display()));
    }

    // 输出目录
    let output_dir = args
        .output_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")).join("frames"));

    fs::create_dir_all(&output_dir).map_err(|e| format!("创建输出目录失败：{e}"))?;

    // 计算 ffmpeg fps 参数
    let fps_param = match args.fps_mode.as_str() {
        "interval" => {
            if args.fps_value <= 0.0 {
                return Err("间隔时间必须大于 0".to_string());
            }
            format!("1/{}", args.fps_value)
        }
        _ => {
            // per_second
            if args.fps_value <= 0.0 {
                return Err("每秒帧数必须大于 0".to_string());
            }
            format!("{}", args.fps_value)
        }
    };

    // 构建 filter
    let mut filters = Vec::new();
    filters.push(format!("fps={}", fps_param));

    if args.timestamp.enabled {
        let drawtext = build_drawtext_filter(&args.timestamp)?;
        filters.push(drawtext);
    }

    let filter_complex = filters.join(",");

    // 输出文件名模式：使用自定义前缀或源文件名作为前缀
    let ext = if args.format == "png" { "png" } else { "jpg" };
    let source_stem = args
        .filename_prefix
        .as_ref()
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| {
            input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("video")
                .to_string()
        });
    let output_pattern = output_dir.join(format!("{}_%05d.{}", source_stem, ext));

    // 构建 ffmpeg 命令
    let program = resolve_ffmpeg_command();
    let mut cmd = build_cmd(&program);

    cmd.arg("-i").arg(input);
    cmd.arg("-vf").arg(&filter_complex);

    // JPEG 质量
    if args.format != "png" {
        let q = 31 - (args.quality as u32 * 31 / 100).max(1);
        cmd.arg("-q:v").arg(q.to_string());
    }

    cmd.arg("-y"); // 覆盖已有文件
    cmd.arg(&output_pattern);

    // 记录命令字符串（用于显示）
    let command_str = format!("{:?}", cmd);

    // 执行
    let out = cmd.output().map_err(|e| format!("ffmpeg 调用失败：{e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("ffmpeg 失败：{stderr}"));
    }

    // 统计输出文件
    let output_files = list_output_files(&output_dir, ext);

    Ok(ExtractResult {
        output_dir: output_dir.display().to_string(),
        total_frames: output_files.len(),
        extracted_frames: output_files.len(),
        output_files,
        command: command_str,
    })
}

/// 构建 drawtext 过滤器
fn build_drawtext_filter(config: &TimestampConfig) -> Result<String, String> {
    // 位置：百分比转小数
    let x_expr = format!("w*{}", config.x_percent / 100.0);
    let y_expr = format!("h*{}", config.y_percent / 100.0);

    // 背景颜色解析
    let (bg_color, bg_alpha) = parse_color_alpha(&config.bg_color);

    let mut parts = vec![
        format!("text='%{{pts\\:gmtime\\:0\\:%T}}'"),
        format!("x={}", x_expr),
        format!("y={}", y_expr),
        format!("fontsize={}", config.font_size),
        format!("fontcolor={}", config.font_color),
        format!("box=1"),
        format!("boxcolor={}@{}", bg_color, bg_alpha),
        format!("boxborderw={}", config.bg_padding),
    ];

    // 字体文件
    if !config.font_file.is_empty() {
        parts.push(format!("fontfile={}", config.font_file));
    }

    Ok(format!("drawtext={}", parts.join(":")))
}

/// 解析颜色和透明度（如 "black@0.6" -> ("black", 0.6)）
fn parse_color_alpha(s: &str) -> (&str, f64) {
    if let Some(at_pos) = s.find('@') {
        let color = &s[..at_pos];
        let alpha: f64 = s[at_pos + 1..].parse().unwrap_or(1.0);
        (color, alpha)
    } else {
        (s, 1.0)
    }
}

/// 列出输出文件
fn list_output_files(dir: &Path, ext: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_ext) = path.extension().and_then(|s| s.to_str()) {
                    if file_ext.eq_ignore_ascii_case(ext) {
                        files.push(path.display().to_string());
                    }
                }
            }
        }
    }
    files.sort();
    files
}

/// 格式化时长（保留供前端展示用）
#[allow(dead_code)]
pub fn format_duration(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}

/// 格式化文件大小（保留供前端展示用）
#[allow(dead_code)]
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "00:00");
        assert_eq!(format_duration(65.0), "01:05");
        assert_eq!(format_duration(3661.0), "01:01:01");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_parse_color_alpha() {
        assert_eq!(parse_color_alpha("black@0.6"), ("black", 0.6));
        assert_eq!(parse_color_alpha("red"), ("red", 1.0));
    }

    #[test]
    fn test_build_drawtext_filter() {
        let config = TimestampConfig {
            enabled: true,
            x_percent: 3.0,
            y_percent: 3.0,
            font_size: 32,
            font_color: "red".to_string(),
            font_file: String::new(),
            bg_color: "black@0.6".to_string(),
            bg_padding: 8,
        };
        let filter = build_drawtext_filter(&config).unwrap();
        assert!(filter.starts_with("drawtext="));
        assert!(filter.contains("x=w*0.03"));
        assert!(filter.contains("y=h*0.03"));
    }
}
