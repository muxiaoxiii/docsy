//! 大文件分段处理：超过页数/体积阈值时，按页段用 qpdf 切开、逐段处理再合并。
//! 避免 lopdf `Document::load` 把 1000+ 页扫描件整档读进 Rust 堆。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::temp_named_path;
use crate::external::ExternalTool;
use crate::util::fs::TempDirGuard;

/// 超过该页数改走分段（用户约定约 300 页）。
pub const CHUNK_PAGE_THRESHOLD: u32 = 300;
/// 超过该文件大小改走分段（约 100MB）。
pub const CHUNK_SIZE_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;
/// 分段时每段页数。
pub const CHUNK_SIZE_PAGES: u32 = 80;

pub fn file_size_bytes(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn should_chunk(path: &Path, page_count: u32) -> bool {
    page_count >= CHUNK_PAGE_THRESHOLD || file_size_bytes(path) >= CHUNK_SIZE_THRESHOLD_BYTES
}

/// 按页段处理整份 PDF。
///
/// `process_chunk` 收到 `(chunk_input, output, chunk_index, page_start, page_end)`，
/// 必须把处理结果写到 `output`。最终按顺序 qpdf 合并到 `final_output`。
pub fn process_pdf_chunked<F>(
    input: &Path,
    final_output: &Path,
    page_count: u32,
    mut process_chunk: F,
) -> Result<()>
where
    F: FnMut(&Path, &Path, usize, u32, u32) -> Result<()>,
{
    if page_count == 0 {
        anyhow::bail!("PDF 无页面，无法分段处理");
    }
    if let Some(parent) = final_output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("创建分段处理输出目录失败")?;
        }
    }

    super::qpdf::validate_recovery_backup(input)?;
    validate_chunk_catalog(input)?;
    crate::operations::check_current_cancelled()?;
    let work_dir = temp_named_path("docsy_chunk_work", "dir");
    std::fs::create_dir_all(&work_dir).context("创建分段工作目录失败")?;
    let _work_guard = TempDirGuard::new(work_dir.clone())?;

    let mut processed: Vec<PathBuf> = Vec::new();
    let mut chunk_index = 0_usize;
    let mut start = 1_u32;
    while start <= page_count {
        crate::operations::check_current_cancelled()?;
        let end = (start + CHUNK_SIZE_PAGES - 1).min(page_count);
        let chunk_input = work_dir.join(format!("chunk-{chunk_index:04}-in.pdf"));
        let chunk_output = work_dir.join(format!("chunk-{chunk_index:04}-out.pdf"));

        extract_page_range(input, &chunk_input, start, end)
            .with_context(|| format!("提取第 {start}-{end} 页分段失败"))?;
        process_chunk(&chunk_input, &chunk_output, chunk_index, start, end)
            .with_context(|| format!("处理第 {start}-{end} 页分段失败"))?;
        if !chunk_output.exists() {
            anyhow::bail!("第 {start}-{end} 页分段未生成输出");
        }
        processed.push(chunk_output);

        chunk_index += 1;
        start = end + 1;
    }

    crate::operations::check_current_cancelled()?;
    // Keep original Info (including anti-copy backup) and supported catalog metadata.
    let stage = crate::util::fs::sibling_temp_path(final_output, "chunk-final");
    let _stage_guard = crate::util::fs::TempPathGuard::new(stage.clone());
    let bin = crate::external::QpdfTool.binary_path()?;
    let mut cmd = crate::external::hidden_command(&bin);
    cmd.arg(input).arg("--pages");
    for path in &processed {
        cmd.arg(path).arg("1-z");
    }
    cmd.arg("--").arg(&stage);
    let result = super::qpdf::run_cancellable("合并分段", cmd)?;
    if !super::qpdf::status_is_success(&result.status) {
        anyhow::bail!(
            "合并分段处理结果失败：{}",
            crate::external::command_failure_detail(&result)
        );
    }
    crate::operations::check_current_cancelled()?;
    std::fs::rename(&stage, final_output).context("写入分段处理结果失败")?;
    Ok(())
}

fn extract_page_range(input: &Path, output: &Path, start: u32, end: u32) -> Result<()> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let selection = if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    };
    let mut cmd = crate::external::hidden_command(&bin);
    cmd.arg("--empty")
        .arg("--pages")
        .arg(input)
        .arg(selection)
        .arg("--")
        .arg(output);
    let result = super::qpdf::run_cancellable("分段提取", cmd)?;
    if !super::qpdf::status_is_success(&result.status) {
        anyhow::bail!(
            "qpdf 分段提取失败：{}",
            crate::external::command_failure_detail(&result)
        );
    }
    Ok(())
}

// A stop-version deliberately refuses page-referencing catalog features until
// their remapping has been verified. Inspect only two objects, not image streams.
pub(crate) fn read_qpdf_object(
    input: &Path,
    selector: &str,
    key: &str,
) -> Result<serde_json::Value> {
    let bin = crate::external::QpdfTool.binary_path()?;
    let mut cmd = crate::external::hidden_command(&bin);
    cmd.args(["--json", "--json-key=qpdf", "--json-stream-data=none"])
        .arg(format!("--json-object={selector}"))
        .arg(input);
    let output = super::qpdf::run_cancellable("检查分段文档信息", cmd)?;
    if !super::qpdf::status_is_success(&output.status) {
        anyhow::bail!("无法检查 PDF 文档信息，已停止分段处理");
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    json["qpdf"]
        .as_array()
        .and_then(|entries| entries.iter().find_map(|entry| entry.get(key)))
        .and_then(|object| object.get("value"))
        .cloned()
        .context("qpdf 未返回文档信息")
}

fn validate_chunk_catalog(input: &Path) -> Result<()> {
    let trailer = read_qpdf_object(input, "trailer", "trailer")?;
    let root = trailer["/Root"].as_str().context("PDF 缺少根目录")?;
    let fields: Vec<_> = root.split_whitespace().collect();
    if fields.len() != 3 {
        anyhow::bail!("PDF 根目录引用无效");
    }
    let catalog = read_qpdf_object(
        input,
        &format!("{},{}", fields[0], fields[1]),
        &format!("obj:{root}"),
    )?;
    let unsupported: Vec<_> = catalog
        .as_object()
        .context("PDF 根目录无效")?
        .keys()
        .filter(|key| {
            ![
                "/Type",
                "/Pages",
                "/Metadata",
                "/Version",
                "/Lang",
                "/ViewerPreferences",
            ]
            .contains(&key.as_str())
        })
        .cloned()
        .collect();
    if !unsupported.is_empty() {
        anyhow::bail!("此大 PDF 含尚未支持安全分段保留的文档结构（{}，可能为书签、表单、附件或签名），已停止处理并保留原文件。可使用无损优化，或先在专业 PDF 工具中处理文档结构。", unsupported.join("、"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_are_300_pages_or_100mb() {
        assert_eq!(CHUNK_PAGE_THRESHOLD, 300);
        assert_eq!(CHUNK_SIZE_THRESHOLD_BYTES, 100 * 1024 * 1024);
        assert_eq!(CHUNK_SIZE_PAGES, 80);
    }

    #[test]
    fn small_docs_do_not_chunk() {
        let path = std::env::temp_dir().join("docsy_chunk_threshold_test.pdf");
        std::fs::write(&path, b"%PDF-1.4 tiny").unwrap();
        assert!(!should_chunk(&path, 50));
        std::fs::remove_file(&path).ok();
    }
}
