//! FFmpeg 检测与安装

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// FFmpeg 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub has_drawtext: bool,
    pub error: Option<String>,
}

/// 系统字体信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    pub name: String,
    pub path: String,
}

/// 下载源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadConfig {
    pub macos_arm: String,
    pub macos_x64: String,
    pub windows: String,
    pub private_url: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            macos_arm: "https://only:6688@share.mars4.muxiaoxi.top:44/ffmpeg-8.1.1.zip".to_string(),
            macos_x64: "https://only:6688@share.mars4.muxiaoxi.top:44/ffmpeg-8.1.1.zip".to_string(),
            windows: "https://only:6688@share.mars4.muxiaoxi.top:44/ffmpeg-win64-gpl.zip"
                .to_string(),
            private_url: String::new(),
        }
    }
}

/// ffmpeg 可执行文件名
fn ffmpeg_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// ffprobe 可执行文件名
fn ffprobe_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffprobe.exe"
    } else {
        "ffprobe"
    }
}

/// 获取 ffmpeg 存储目录
pub fn ffmpeg_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join("Docsy").join("ffmpeg"))
}

/// 查找 ffmpeg 可执行文件
pub fn resolve_ffmpeg_command() -> PathBuf {
    let filename = ffmpeg_filename();

    // 1. 检查 Docsy 数据目录（用户下载的）
    if let Some(dir) = ffmpeg_dir() {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return candidate;
        }
    }

    // 2. 检查可执行文件同目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(filename);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // 3. 检查常见安装路径
    let prefixes: Vec<&str> = if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\ffmpeg\bin",
            r"C:\ffmpeg\bin",
            r"C:\Program Files (x86)\ffmpeg\bin",
        ]
    } else {
        vec!["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"]
    };

    for prefix in &prefixes {
        let candidate = Path::new(prefix).join(filename);
        if candidate.exists() {
            return candidate;
        }
    }

    // 4. 回退到 PATH
    PathBuf::from(filename)
}

/// 查找 ffprobe 可执行文件
pub fn resolve_ffprobe_command() -> PathBuf {
    let filename = ffprobe_filename();

    // 1. 检查 Docsy 数据目录
    if let Some(dir) = ffmpeg_dir() {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return candidate;
        }
    }

    // 2. 检查可执行文件同目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(filename);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // 3. 检查常见安装路径
    let prefixes: Vec<&str> = if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\ffmpeg\bin",
            r"C:\ffmpeg\bin",
            r"C:\Program Files (x86)\ffmpeg\bin",
        ]
    } else {
        vec!["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"]
    };

    for prefix in &prefixes {
        let candidate = Path::new(prefix).join(filename);
        if candidate.exists() {
            return candidate;
        }
    }

    // 4. 回退到 PATH
    PathBuf::from(filename)
}

/// 构建命令（跨平台处理）
pub fn build_cmd(program: &Path) -> Command {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}

/// 检测 ffmpeg 状态
pub fn check_ffmpeg() -> FfmpegStatus {
    let program = resolve_ffmpeg_command();
    let mut cmd = build_cmd(&program);
    cmd.arg("-version");

    match cmd.output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let version = parse_ffmpeg_version(&stdout);
            let has_drawtext = check_drawtext_support(&program);

            FfmpegStatus {
                available: true,
                version,
                path: Some(program.display().to_string()),
                has_drawtext,
                error: None,
            }
        }
        Ok(out) => FfmpegStatus {
            available: false,
            version: None,
            path: None,
            has_drawtext: false,
            error: Some(format!(
                "ffmpeg 运行失败：{}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        },
        Err(err) => FfmpegStatus {
            available: false,
            version: None,
            path: None,
            has_drawtext: false,
            error: Some(format!("未检测到 ffmpeg：{err}")),
        },
    }
}

/// 尝试通过 brew 安装 ffmpeg（macOS）
///
/// brew install ffmpeg 默认安装完整版（包含 drawtext）
/// 如果需要精简版：brew install ffmpeg --without-drawtext
pub fn try_brew_install_ffmpeg() -> Result<String, String> {
    if !cfg!(target_os = "macos") {
        return Err("brew 安装仅支持 macOS".to_string());
    }

    // 检查 brew 是否可用
    let brew_path = if Path::new("/opt/homebrew/bin/brew").exists() {
        "/opt/homebrew/bin/brew"
    } else if Path::new("/usr/local/bin/brew").exists() {
        "/usr/local/bin/brew"
    } else {
        return Err("未检测到 Homebrew，请先安装：https://brew.sh".to_string());
    };

    // 检查是否已安装 ffmpeg
    let mut check_cmd = build_cmd(Path::new(brew_path));
    check_cmd.args(["list", "ffmpeg"]);
    if let Ok(out) = check_cmd.output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("ffmpeg") {
                // 已安装，检查是否有 drawtext
                let ffmpeg_path = resolve_ffmpeg_command();
                let has_drawtext = check_drawtext_support(&ffmpeg_path);
                if has_drawtext {
                    return Ok("ffmpeg 已通过 brew 安装（完整版）".to_string());
                } else {
                    // 已安装但是精简版，需要重新安装
                    return Err(
                        "当前 ffmpeg 是精简版，不支持 drawtext。请运行：brew reinstall ffmpeg"
                            .to_string(),
                    );
                }
            }
        }
    }

    // 尝试安装 ffmpeg（完整版，包含 drawtext）
    let mut install_cmd = build_cmd(Path::new(brew_path));
    install_cmd.args(["install", "ffmpeg"]);

    let out = install_cmd
        .output()
        .map_err(|e| format!("brew 调用失败：{e}"))?;

    if out.status.success() {
        Ok("brew 安装 ffmpeg 成功（完整版）".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(format!("brew 安装失败：{stderr}"))
    }
}

/// 尝试通过 brew 安装 qpdf（macOS）
pub fn try_brew_install_qpdf() -> Result<String, String> {
    if !cfg!(target_os = "macos") {
        return Err("brew 安装仅支持 macOS".to_string());
    }

    // 检查 brew 是否可用
    let brew_path = if Path::new("/opt/homebrew/bin/brew").exists() {
        "/opt/homebrew/bin/brew"
    } else if Path::new("/usr/local/bin/brew").exists() {
        "/usr/local/bin/brew"
    } else {
        return Err("未检测到 Homebrew，请先安装：https://brew.sh".to_string());
    };

    // 检查是否已安装 qpdf
    let mut check_cmd = build_cmd(Path::new(brew_path));
    check_cmd.args(["list", "qpdf"]);
    if let Ok(out) = check_cmd.output() {
        if out.status.success() {
            return Ok("qpdf 已通过 brew 安装".to_string());
        }
    }

    // 尝试安装 qpdf
    let mut install_cmd = build_cmd(Path::new(brew_path));
    install_cmd.args(["install", "qpdf"]);

    let out = install_cmd
        .output()
        .map_err(|e| format!("brew 调用失败：{e}"))?;

    if out.status.success() {
        Ok("brew 安装 qpdf 成功".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(format!("brew 安装失败：{stderr}"))
    }
}

/// 解析 ffmpeg 版本
fn parse_ffmpeg_version(s: &str) -> Option<String> {
    // 例：ffmpeg version 7.0 Copyright (c) ...
    s.lines().next().and_then(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "ffmpeg" && parts[1] == "version" {
            Some(parts[2].to_string())
        } else {
            None
        }
    })
}

/// 检查是否支持 drawtext
fn check_drawtext_support(program: &Path) -> bool {
    let mut cmd = build_cmd(program);
    cmd.arg("-filters");

    match cmd.output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("drawtext")
        }
        _ => false,
    }
}

/// 获取系统字体列表
pub fn list_system_fonts() -> Vec<FontInfo> {
    let mut fonts = Vec::new();

    if cfg!(target_os = "macos") {
        // macOS 字体目录
        let dirs = [
            "/Library/Fonts",
            "/System/Library/Fonts",
            "/Network/Library/Fonts",
        ];
        if let Some(home) = dirs::home_dir() {
            let user_fonts = home.join("Library/Fonts");
            for dir in dirs
                .iter()
                .chain(std::iter::once(&user_fonts.to_str().unwrap_or("")))
            {
                scan_font_dir(Path::new(dir), &mut fonts);
            }
        } else {
            for dir in &dirs {
                scan_font_dir(Path::new(dir), &mut fonts);
            }
        }
    } else if cfg!(target_os = "windows") {
        // Windows 字体目录
        if let Some(windows) = std::env::var("WINDIR")
            .ok()
            .or_else(|| Some("C:\\Windows".to_string()))
        {
            let fonts_dir = Path::new(&windows).join("Fonts");
            scan_font_dir(&fonts_dir, &mut fonts);
        }
        // 用户字体
        if let Some(appdata) = dirs::data_dir() {
            let user_fonts = appdata.join("Microsoft").join("Windows").join("Fonts");
            scan_font_dir(&user_fonts, &mut fonts);
        }
    }

    fonts.sort_by(|a, b| a.name.cmp(&b.name));
    fonts
}

/// 扫描字体目录
fn scan_font_dir(dir: &Path, fonts: &mut Vec<FontInfo>) {
    if !dir.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if matches!(ext.as_str(), "ttf" | "otf" | "ttc") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();

            fonts.push(FontInfo {
                name,
                path: path.display().to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ffmpeg_version() {
        let input = "ffmpeg version 7.0 Copyright (c) 2000-2024 the FFmpeg developers";
        assert_eq!(parse_ffmpeg_version(input), Some("7.0".to_string()));
    }

    #[test]
    fn test_list_system_fonts() {
        let fonts = list_system_fonts();
        // 至少应该能找到一些字体
        println!("Found {} fonts", fonts.len());
    }
}
