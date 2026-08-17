use crate::external::ExternalTool;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

/// Run a command through the SubprocessRegistry if available, otherwise fall back to .output().
/// `label` is used as a human-readable operation ID prefix for cancellation tracking.
fn run_cancellable(label: &str, mut cmd: std::process::Command) -> Result<std::process::Output> {
    if let Some(registry) = crate::get_subprocess_registry() {
        let op_id = format!("qpdf:{label}:{}", std::process::id());
        registry
            .spawn_and_wait(&op_id, cmd)
            .with_context(|| format!("执行 qpdf {label} 失败"))
    } else {
        cmd.output()
            .with_context(|| format!("执行 qpdf {label} 失败"))
    }
}

pub(crate) fn status_is_success(status: &ExitStatus) -> bool {
    status.success() || status.code() == Some(3)
}

pub struct InspectResult {
    pub encrypted: bool,
    pub pages: Option<u32>,
}

pub struct UnlockResult {
    pub output_path: String,
    pub skipped: bool,
}

pub struct PdfOutputResult {
    pub output_path: String,
    pub input_size: u64,
    pub output_size: u64,
}

pub struct OptimizeResult {
    pub output_path: String,
    pub input_size: u64,
    pub output_size: u64,
    pub changed: bool,
}

pub fn inspect(path: &str) -> Result<InspectResult> {
    let encrypted = is_encrypted(path)?;
    let pages = page_count(path).ok();

    Ok(InspectResult { encrypted, pages })
}

fn is_encrypted(path: &str) -> Result<bool> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let mut cmd = crate::external::hidden_command(&bin);
    cmd.arg("--is-encrypted").arg(path);
    let output = run_cancellable("加密检测", cmd)?;
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
    if !is_encrypted(&input.to_string_lossy())? {
        return Ok(UnlockResult {
            output_path: input.display().to_string(),
            skipped: true,
        });
    }

    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let output_path = unique_output_path(input, "_unlocked");
    let mut cmd = crate::external::hidden_command(&bin);
    cmd.arg("--decrypt")
        .arg("--password=")
        .arg(input)
        .arg(&output_path);
    let output = run_cancellable("解锁", cmd)?;

    if !status_is_success(&output.status) {
        anyhow::bail!(
            "qpdf 解锁失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&output)
        );
    }

    Ok(UnlockResult {
        output_path: output_path.display().to_string(),
        skipped: false,
    })
}

pub fn merge(inputs: &[String], output: &str) -> Result<String> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output_path = unique_available_path(Path::new(output));

    let mut cmd = crate::external::hidden_command(&bin);
    // Merging is a print-oriented operation. Flatten annotation appearances
    // before the structural optimization so signature/stamp graphics become
    // reachable page content before unused resources are removed.
    add_merge_args(&mut cmd);
    cmd.arg("--empty").arg("--pages");
    for input in inputs {
        cmd.arg(input);
    }
    cmd.arg("--").arg(&output_path);

    let output = run_cancellable("合并", cmd)?;
    if !status_is_success(&output.status) {
        anyhow::bail!(
            "qpdf 合并失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&output)
        );
    }

    Ok(output_path.display().to_string())
}

/// 压缩 PDF，并通过回调报告阶段。回调只用于 UI 反馈，不改变处理策略。
pub fn compress_with_progress<F>(
    input: &str,
    output_dir: Option<&str>,
    level: Option<u8>,
    mut progress: F,
) -> Result<PdfOutputResult>
where
    F: FnMut(super::compress::CompressProgress),
{
    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("PDF 文件不存在: {}", input);
    }

    let input_size = std::fs::metadata(input_path)?.len();

    // 默认只做无损结构整理。图片解码/重编码必须由用户明确开启，
    // 否则大扫描件会进入很长的图片处理流程，和证据处理的快速无损路径不一致。
    let Some(level) = level else {
        progress(super::compress::CompressProgress::phase("按页重建 PDF"));
        let output_path = unique_output_path_in_dir(input_path, output_dir, "_compressed");
        // Keep the standalone default identical to the evidence workflow.
        // A plain qpdf rewrite only recompresses reachable streams; rebuilding
        // the page tree also drops otherwise unreachable objects left by common
        // scanners and PDF editors, which is where large evidence files often
        // gain most of their size back.
        rebuild_pages(input_path, &output_path)?;
        let output_size = std::fs::metadata(&output_path)?.len();
        progress(super::compress::CompressProgress::done());
        return Ok(PdfOutputResult {
            output_path: output_path.display().to_string(),
            input_size,
            output_size,
        });
    };

    let options = super::compress::CompressOptions::from_level(level);

    // Step 1: 图片重编码压缩
    let temp_path = unique_output_path_in_dir(input_path, output_dir, "_imgtmp");
    progress(super::compress::CompressProgress::phase(
        "读取 PDF 并分析图片",
    ));
    super::compress::compress_pdf_with_progress(input_path, &temp_path, &options, &mut progress)?;

    // Step 2: qpdf 按页重建 + 结构优化（丢弃不可达对象，收益通常大于单纯重写）
    let output_path = unique_output_path_in_dir(input_path, output_dir, "_compressed");
    progress(super::compress::CompressProgress::phase(
        "按页重建并整理结构",
    ));
    rebuild_pages(&temp_path, &output_path)?;

    // 清理临时文件
    let _ = std::fs::remove_file(&temp_path);

    let output_size = std::fs::metadata(&output_path)?.len();
    progress(super::compress::CompressProgress::done());
    Ok(PdfOutputResult {
        output_path: output_path.display().to_string(),
        input_size,
        output_size,
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
    let mut command = crate::external::hidden_command(&bin);
    command.arg("--empty").arg("--pages");
    // qpdf --pages 里每个文件名后只认一个页段，多页需重复文件名
    for page in pages {
        command.arg(input_path).arg(page.to_string());
    }
    command.arg("--").arg(&output_path);

    let command_output = run_cancellable("页面提取", command)?;
    if !status_is_success(&command_output.status) {
        anyhow::bail!(
            "qpdf 页面提取失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&command_output)
        );
    }
    if !output_path.exists() {
        anyhow::bail!("qpdf 未生成页面提取输出文件");
    }

    let input_size = std::fs::metadata(input_path)?.len();
    let output_size = std::fs::metadata(&output_path)?.len();
    Ok(PdfOutputResult {
        output_path: output_path.display().to_string(),
        input_size,
        output_size,
    })
}

/// Create a losslessly rebuilt copy in `output_dir` when supplied. Evidence
/// processing uses this before applying overlays so temporary working copies
/// remain alongside the eventual output instead of next to the source PDF.
pub fn optimize_lossless_to_dir(input: &str, output_dir: Option<&Path>) -> Result<OptimizeResult> {
    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("PDF 文件不存在: {}", input);
    }
    let input_size = std::fs::metadata(input_path)?.len();

    let parent =
        output_dir.unwrap_or_else(|| input_path.parent().unwrap_or_else(|| Path::new(".")));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("创建优化输出目录失败: {}", parent.display()))?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("pdf");
    let output_path =
        crate::util::fs::unique_output_path(parent, &format!("{stem}（已优化）"), ext);

    rebuild_pages(input_path, &output_path)?;

    let output_size = std::fs::metadata(&output_path)?.len();
    if output_size >= input_size {
        // 优化无效：删除输出，回退原路径
        let _ = std::fs::remove_file(&output_path);
        return Ok(OptimizeResult {
            output_path: input.to_string(),
            input_size,
            output_size: input_size,
            changed: false,
        });
    }

    Ok(OptimizeResult {
        output_path: output_path.display().to_string(),
        input_size,
        output_size,
        changed: true,
    })
}

/// 就地无损优化：重建到同目录临时文件，有收益则替换原文件（同一路径），
/// 无收益或失败则保持原文件不动。用于导出结果收尾，不产生副本文件。
pub fn optimize_in_place(input: &str) -> Result<OptimizeResult> {
    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("PDF 文件不存在: {}", input);
    }
    let input_size = std::fs::metadata(input_path)?.len();

    // 临时文件必须与原文件同目录：std::fs::rename 不能跨卷/跨盘移动。
    let temp_path = crate::util::fs::sibling_temp_path(input_path, "docsy-optimizing");
    let guard = crate::util::fs::TempPathGuard::new(temp_path.clone());
    rebuild_pages(input_path, guard.path())?;

    let output_size = std::fs::metadata(guard.path())?.len();
    if output_size >= input_size {
        return Ok(OptimizeResult {
            output_path: input.to_string(),
            input_size,
            output_size: input_size,
            changed: false,
        });
    }

    crate::util::fs::replace_file(guard.path(), input_path).context("替换优化后的文件失败")?;
    Ok(OptimizeResult {
        output_path: input.to_string(),
        input_size,
        output_size,
        changed: true,
    })
}

/// qpdf 按页重建：--empty --pages <input> 1-z，丢弃页树不可达对象并施加优化参数。
fn rebuild_pages(input_path: &Path, output_path: &Path) -> Result<()> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let mut cmd = crate::external::hidden_command(&bin);
    add_optimization_args(&mut cmd);
    cmd.arg("--empty")
        .arg("--pages")
        .arg(input_path)
        .arg("1-z")
        .arg("--")
        .arg(output_path);
    let output = run_cancellable("无损优化", cmd)?;
    if !status_is_success(&output.status) {
        anyhow::bail!(
            "qpdf 无损优化失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&output)
        );
    }
    if !output_path.exists() {
        anyhow::bail!("qpdf 未生成无损优化输出文件");
    }
    Ok(())
}

pub(crate) fn add_optimization_args(command: &mut std::process::Command) {
    command
        .arg("--object-streams=generate")
        .arg("--compress-streams=y")
        .arg("--recompress-flate")
        .arg("--compression-level=9")
        .arg("--remove-unreferenced-resources=yes");
}

/// Keep page appearance stable for printable outputs.
///
/// qpdf uses each annotation's existing appearance stream, so this preserves
/// vector/text/image quality while making signature and stamp graphics part of
/// the page content. It intentionally gives up annotation interactivity and
/// signature validity, which is the desired boundary for print-only outputs.
pub(crate) fn add_print_preservation_args(command: &mut std::process::Command) {
    command.arg("--flatten-annotations=all");
}

/// Merge with print-safe appearance preservation and the existing lossless
/// structural optimization. Keep flattening first so resources used by a
/// signature/stamp appearance are visible to the resource cleanup pass.
pub(crate) fn add_merge_args(command: &mut std::process::Command) {
    add_print_preservation_args(command);
    add_optimization_args(command);
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

pub fn page_count(input: &str) -> Result<u32> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let mut cmd = crate::external::hidden_command(&bin);
    cmd.arg("--show-npages").arg(input);
    let output = run_cancellable("读取页数", cmd)?;
    if status_is_success(&output.status) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(count) = stdout.trim().parse::<u32>() {
            return Ok(count);
        }
    }

    // Some repaired PDFs return no value for --show-npages even though the
    // richer JSON command can still resolve every page correctly.
    if let Ok(page_infos) = super::page_info::get_page_infos(input) {
        return u32::try_from(page_infos.len()).context("PDF 页数超过支持范围");
    }

    anyhow::bail!(
        "qpdf 读取页数失败（{}）：{}",
        bin.display(),
        crate::external::command_failure_detail(&output)
    )
}

fn unique_output_path(input: &Path, suffix: &str) -> PathBuf {
    unique_output_path_in_dir(input, None, suffix)
}

// 收敛说明：统一委托 crate::util::fs::unique_output_path。与原本地实现的差异仅在
// 撞名场景：原实现从 `-1` 起编号且无上限，现从 `-2` 起、超过 10 000 回退时间戳命名。
fn unique_output_path_in_dir(input: &Path, output_dir: Option<&str>, suffix: &str) -> PathBuf {
    let parent = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")).to_path_buf());
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("pdf");
    crate::util::fs::unique_output_path(&parent, &format!("{stem}{suffix}"), ext)
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

    #[test]
    fn accepts_qpdf_warning_exit_status() {
        assert!(status_is_success(&exit_status(0)));
        assert!(status_is_success(&exit_status(3)));
        assert!(!status_is_success(&exit_status(2)));
    }
}
