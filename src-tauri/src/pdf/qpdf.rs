use crate::external::ExternalTool;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

pub struct InspectResult {
    pub encrypted: bool,
    pub pages: Option<u32>,
}

pub struct UnlockResult {
    pub output_path: String,
}

pub struct PdfOutputResult {
    pub output_path: String,
}

pub fn inspect(path: &str) -> Result<InspectResult> {
    let encrypted = is_encrypted(path)?;
    let pages = page_count(path).ok();

    Ok(InspectResult { encrypted, pages })
}

fn is_encrypted(path: &str) -> Result<bool> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output = std::process::Command::new(&bin)
        .arg("--is-encrypted")
        .arg(path)
        .output()
        .context("执行 qpdf 加密检测失败")?;
    parse_is_encrypted_status(output.status, &output.stderr)
}

fn parse_is_encrypted_status(status: ExitStatus, stderr: &[u8]) -> Result<bool> {
    match status.code() {
        Some(0) => Ok(true),
        Some(2) if !looks_like_qpdf_hard_error(stderr) => Ok(false),
        _ => {
            let message = String::from_utf8_lossy(stderr);
            anyhow::bail!("qpdf 加密检测失败: {}", message.trim())
        }
    }
}

fn looks_like_qpdf_hard_error(stderr: &[u8]) -> bool {
    let message = String::from_utf8_lossy(stderr).to_lowercase();
    message.contains("error")
        || message.contains("invalid")
        || message.contains("not a pdf")
        || message.contains("password")
        || message.contains("missing")
}

pub fn unlock(input: &Path) -> Result<UnlockResult> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let output_path = unique_output_path(input, "_unlocked");
    let status = std::process::Command::new(&bin)
        .arg("--decrypt")
        .arg("--password=")
        .arg(input)
        .arg(&output_path)
        .status()?;

    if !status.success() {
        anyhow::bail!("qpdf 解锁失败");
    }

    Ok(UnlockResult {
        output_path: output_path.display().to_string(),
    })
}

pub fn merge(inputs: &[String], output: &str) -> Result<String> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output_path = unique_available_path(Path::new(output));

    let mut cmd = std::process::Command::new(&bin);
    add_optimization_args(&mut cmd);
    cmd.arg("--empty").arg("--pages");
    for input in inputs {
        cmd.arg(input);
    }
    cmd.arg("--").arg(&output_path);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("qpdf 合并失败");
    }

    Ok(output_path.display().to_string())
}

pub fn optimize_to(input: &Path, output: &Path) -> Result<()> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let mut command = std::process::Command::new(&bin);
    add_optimization_args(&mut command);
    let command_output = command
        .arg(input)
        .arg(output)
        .output()
        .context("执行 qpdf 压缩整理失败")?;

    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        anyhow::bail!("qpdf 压缩整理失败: {}", stderr.trim());
    }

    Ok(())
}

pub fn compress(input: &str, output_dir: Option<&str>) -> Result<PdfOutputResult> {
    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("PDF 文件不存在: {}", input);
    }
    let output_path = unique_output_path_in_dir(input_path, output_dir, "_compressed");
    optimize_to(input_path, &output_path)?;
    Ok(PdfOutputResult {
        output_path: output_path.display().to_string(),
    })
}

pub fn extract_pages(
    input: &str,
    pages: &[u32],
    output_dir: Option<&str>,
) -> Result<PdfOutputResult> {
    if pages.is_empty() {
        anyhow::bail!("缺少要提取的页码");
    }
    if pages.contains(&0) {
        anyhow::bail!("页码必须从 1 开始");
    }
    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("PDF 文件不存在: {}", input);
    }
    let total_pages = page_count(input)?;
    if let Some(page) = pages.iter().find(|page| **page > total_pages) {
        anyhow::bail!("页码 {page} 超过 PDF 总页数 {total_pages}");
    }

    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output_path = unique_output_path_in_dir(input_path, output_dir, "_pages");
    let mut command = std::process::Command::new(&bin);
    command.arg("--empty").arg("--pages").arg(input_path);
    for page in pages {
        command.arg(page.to_string());
    }
    command.arg("--").arg(&output_path);

    let command_output = command.output().context("执行 qpdf 页面提取失败")?;
    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        anyhow::bail!("qpdf 页面提取失败: {}", stderr.trim());
    }
    if !output_path.exists() {
        anyhow::bail!("qpdf 未生成页面提取输出文件");
    }

    Ok(PdfOutputResult {
        output_path: output_path.display().to_string(),
    })
}

fn add_optimization_args(command: &mut std::process::Command) {
    command
        .arg("--object-streams=generate")
        .arg("--compress-streams=y")
        .arg("--recompress-flate")
        .arg("--compression-level=9")
        .arg("--remove-unreferenced-resources=yes");
}

fn unique_available_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("output");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for index in 1..10_000 {
        let name = if extension.is_empty() {
            format!("{stem}-{index}")
        } else {
            format!("{stem}-{index}.{extension}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

pub fn split(input: &str, output_dir: &str) -> Result<Vec<String>> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let input_path = Path::new(input);
    std::fs::create_dir_all(output_dir)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let pages = page_count(input)?;
    let token = unique_suffix();
    let output_pattern = Path::new(output_dir).join(format!("{stem}-split-{token}-%d.pdf"));
    let command_output = std::process::Command::new(&bin)
        .arg("--split-pages")
        .arg(input)
        .arg(&output_pattern)
        .output()?;
    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        anyhow::bail!("qpdf 拆分失败: {}", stderr.trim());
    }

    let prefix = format!("{stem}-split-{token}-");
    let mut outputs = std::fs::read_dir(output_dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".pdf"))
        })
        .collect::<Vec<_>>();
    outputs.sort();
    if outputs.len() != pages as usize {
        anyhow::bail!(
            "qpdf 拆分输出页数异常：预期 {pages} 个文件，实际 {} 个",
            outputs.len()
        );
    }

    Ok(outputs
        .into_iter()
        .map(|path| path.display().to_string())
        .collect())
}

pub fn page_count(input: &str) -> Result<u32> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output = std::process::Command::new(&bin)
        .arg("--show-npages")
        .arg(input)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("qpdf 读取页数失败: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.trim().parse::<u32>()?;
    Ok(count)
}

fn unique_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("pdf");

    let mut path = parent.join(format!("{}{}.{}", stem, suffix, ext));
    let mut i = 1;
    while path.exists() {
        path = parent.join(format!("{}{}-{}.{}", stem, suffix, i, ext));
        i += 1;
    }
    path
}

fn unique_output_path_in_dir(input: &Path, output_dir: Option<&str>, suffix: &str) -> PathBuf {
    let parent = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")).to_path_buf());
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("pdf");
    let mut path = parent.join(format!("{stem}{suffix}.{ext}"));
    let mut i = 1;
    while path.exists() {
        path = parent.join(format!("{stem}{suffix}-{i}.{ext}"));
        i += 1;
    }
    path
}

fn unique_suffix() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{millis}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }

    #[test]
    fn parses_qpdf_is_encrypted_exit_status() {
        assert!(parse_is_encrypted_status(exit_status(0), b"").unwrap());
        assert!(!parse_is_encrypted_status(exit_status(2), b"").unwrap());
        assert!(parse_is_encrypted_status(exit_status(2), b"missing file").is_err());
    }
}
