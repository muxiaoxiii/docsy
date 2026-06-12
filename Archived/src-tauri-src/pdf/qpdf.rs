use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Error)]
pub enum QpdfError {
    #[error("qpdf 调用失败：{0}")]
    Spawn(String),
    #[error("qpdf 返回非零退出码：{stderr}")]
    NonZeroExit { stderr: String },
    #[error("输出文件未生成：{0}")]
    NoOutput(PathBuf),
    #[error("非法输入路径：{0}")]
    InvalidInput(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QpdfStatus {
    pub ok: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockResult {
    pub output_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub output_path: String,
    pub input_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectResult {
    pub encrypted: Option<bool>,
}

/// qpdf 下载配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct QpdfDownloadConfig {
    pub macos: String,
    pub windows: String,
    pub private_url: String,
}

impl Default for QpdfDownloadConfig {
    fn default() -> Self {
        Self {
            macos: "https://only:6688@share.mars4.muxiaoxi.top:44/qpdf-12.3.2-macos-arm64.zip"
                .to_string(),
            windows: "https://only:6688@share.mars4.muxiaoxi.top:44/qpdf-12.3.2-msvc64.exe"
                .to_string(),
            private_url: String::new(),
        }
    }
}

/// 获取 qpdf 存储目录
pub fn qpdf_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join("Docsy").join("qpdf"))
}

fn qpdf_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "qpdf.exe"
    } else {
        "qpdf"
    }
}

pub fn resolve_qpdf_command() -> PathBuf {
    let filename = qpdf_filename();

    // 1. 检查 Docsy 数据目录（用户下载的）
    if let Some(dir) = qpdf_dir() {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return candidate;
        }
        // Windows 可能在 bin 子目录
        let bin_candidate = dir.join("bin").join(filename);
        if bin_candidate.exists() {
            return bin_candidate;
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
            r"C:\Program Files\qpdf\bin",
            r"C:\Program Files (x86)\qpdf\bin",
        ]
    } else {
        vec!["/opt/homebrew/bin", "/usr/local/bin"]
    };

    for prefix in &prefixes {
        let candidate = Path::new(prefix).join(filename);
        if candidate.exists() {
            return candidate;
        }
    }

    // 4. 回退到 PATH 查找
    PathBuf::from(filename)
}

fn build_cmd(program: &Path) -> Command {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    cmd
}

pub fn check() -> QpdfStatus {
    let program = resolve_qpdf_command();
    let mut cmd = build_cmd(&program);
    cmd.arg("--version");

    match cmd.output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            QpdfStatus {
                ok: true,
                version: parse_version(&stdout),
                error: None,
            }
        }
        Ok(out) => QpdfStatus {
            ok: false,
            version: None,
            error: Some(format!(
                "qpdf 运行失败：{}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        },
        Err(err) => QpdfStatus {
            ok: false,
            version: None,
            error: Some(format!("未检测到 qpdf：{err}")),
        },
    }
}

fn parse_version(s: &str) -> Option<String> {
    s.split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|s| s.trim().to_string())
}

pub fn inspect(input: &Path) -> InspectResult {
    if !input.exists() {
        return InspectResult { encrypted: None };
    }

    let program = resolve_qpdf_command();
    let mut cmd = build_cmd(&program);
    cmd.arg("--show-encryption").arg(input);

    let Ok(out) = cmd.output() else {
        return InspectResult { encrypted: None };
    };
    if !out.status.success() {
        return InspectResult { encrypted: None };
    }

    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    let encrypted = if text.contains("file is not encrypted") || text.contains("not encrypted") {
        Some(false)
    } else if text.contains("file is encrypted")
        || text.contains("encryption")
        || text.contains("user password")
        || text.contains("owner password")
    {
        Some(true)
    } else {
        None
    };

    InspectResult { encrypted }
}

pub fn unlock(input: &Path) -> Result<UnlockResult, QpdfError> {
    if !input.exists() {
        return Err(QpdfError::InvalidInput(input.display().to_string()));
    }

    let output_dir = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let output_path = unique_output_path(&output_dir, stem);

    let program = resolve_qpdf_command();
    let mut cmd = build_cmd(&program);
    cmd.arg("--password=")
        .arg("--decrypt")
        .arg(input)
        .arg(&output_path);

    let out = cmd
        .output()
        .map_err(|err| QpdfError::Spawn(err.to_string()))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let _ = std::fs::remove_file(&output_path);
        return Err(QpdfError::NonZeroExit { stderr });
    }

    if !output_path.exists() {
        return Err(QpdfError::NoOutput(output_path));
    }

    Ok(UnlockResult {
        output_path: output_path.display().to_string(),
    })
}

pub fn merge(inputs: &[PathBuf], output: &Path) -> Result<MergeResult, QpdfError> {
    if inputs.is_empty() {
        return Err(QpdfError::InvalidInput("没有可合并的 PDF".to_string()));
    }
    for input in inputs {
        if !input.exists() {
            return Err(QpdfError::InvalidInput(input.display().to_string()));
        }
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|err| QpdfError::Spawn(err.to_string()))?;
    }

    let program = resolve_qpdf_command();
    let mut cmd = build_cmd(&program);
    cmd.arg("--empty").arg("--pages");
    for input in inputs {
        cmd.arg(input);
    }
    cmd.arg("--").arg(output);

    let out = cmd
        .output()
        .map_err(|err| QpdfError::Spawn(err.to_string()))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let _ = std::fs::remove_file(output);
        return Err(QpdfError::NonZeroExit { stderr });
    }

    if !output.exists() {
        return Err(QpdfError::NoOutput(output.to_path_buf()));
    }

    Ok(MergeResult {
        output_path: output.display().to_string(),
        input_count: inputs.len(),
    })
}

fn unique_output_path(dir: &Path, stem: &str) -> PathBuf {
    let base = format!("{stem}_unlocked");
    let first = dir.join(format!("{base}.pdf"));
    if !first.exists() {
        return first;
    }
    for i in 1..=9999 {
        let p = dir.join(format!("{base}_{i}.pdf"));
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!("{base}_overflow.pdf"))
}

/// 用 qpdf 的 --overlay 功能把 overlay_pdf 叠到 input_pdf 上。
///
/// ```text
/// qpdf input.pdf --overlay overlay.pdf -- output.pdf
/// ```
pub fn overlay(input: &Path, overlay_pdf: &Path, output: &Path) -> Result<MergeResult, QpdfError> {
    if !input.exists() {
        return Err(QpdfError::InvalidInput(input.display().to_string()));
    }
    if !overlay_pdf.exists() {
        return Err(QpdfError::InvalidInput(overlay_pdf.display().to_string()));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|err| QpdfError::Spawn(err.to_string()))?;
    }

    let program = resolve_qpdf_command();
    let mut cmd = build_cmd(&program);
    cmd.arg(input)
        .arg("--overlay")
        .arg(overlay_pdf)
        .arg("--")
        .arg(output);

    let out = cmd
        .output()
        .map_err(|err| QpdfError::Spawn(err.to_string()))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let _ = std::fs::remove_file(output);
        return Err(QpdfError::NonZeroExit { stderr });
    }

    if !output.exists() {
        return Err(QpdfError::NoOutput(output.to_path_buf()));
    }

    Ok(MergeResult {
        output_path: output.display().to_string(),
        input_count: 1,
    })
}
