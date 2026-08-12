//! PDF 文本层 → Markdown。
//!
//! 这是 MD 转换中的轻量路径：只读取 PDF 已有文本层，不执行 OCR，不重采样原件。
//! 扫描件、复杂表格和公式由后续可选的本地 AI worker 处理，不能在这里静默降级。

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::external::PopplerTool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfTextExtraction {
    pub markdown: String,
    pub pages_with_text: usize,
    /// 范围内无文本层的页码（原始页码，1-based）。
    pub empty_pages: Vec<u32>,
}

/// `convert_pdf_text_layer` 命令的业务结果（命令层再做 serde 封装）。
#[derive(Debug)]
pub struct PdfTextLayerOutput {
    pub output_path: String,
    pub pages_with_text: usize,
    pub empty_pages: Vec<u32>,
    pub input_size: u64,
    pub output_size: u64,
    pub warning: Option<String>,
}

/// 页段参数校验：start 从 1 开始，end 不早于 start。
fn validate_page_range(start_page: Option<u32>, end_page: Option<u32>) -> Result<()> {
    if let Some(start) = start_page {
        if start == 0 {
            anyhow::bail!("起始页必须从 1 开始");
        }
    }
    if let Some(end) = end_page {
        if end == 0 {
            anyhow::bail!("结束页必须从 1 开始");
        }
        if let Some(start) = start_page {
            if end < start {
                anyhow::bail!("结束页（{end}）不能早于起始页（{start}）");
            }
        }
    }
    Ok(())
}

/// pdftotext 参数（纯函数，便于测试）。页段为 1-based 闭区间。
fn build_pdftotext_args(
    input: &Path,
    output: &Path,
    start_page: Option<u32>,
    end_page: Option<u32>,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["-layout".into(), "-enc".into(), "UTF-8".into()];
    if let Some(start) = start_page {
        args.push("-f".into());
        args.push(start.to_string().into());
    }
    if let Some(end) = end_page {
        args.push("-l".into());
        args.push(end.to_string().into());
    }
    args.push("--".into());
    args.push(input.as_os_str().to_os_string());
    args.push(output.as_os_str().to_os_string());
    args
}

/// 通过 SubprocessRegistry 运行 pdftotext，前端 `cancel_operation` 可杀进程；
/// 无 registry（单元测试等场景）时回退 `.output()`。
fn run_pdftotext_cancellable(mut cmd: std::process::Command) -> Result<std::process::Output> {
    if let Some(registry) = crate::get_subprocess_registry() {
        let op_id = format!("pdf_text_layer:{}", std::process::id());
        registry
            .spawn_and_wait(&op_id, cmd)
            .context("执行 pdftotext 失败")
    } else {
        cmd.output().context("执行 pdftotext 失败")
    }
}

/// 提取 PDF 文本层为 Markdown。`start_page`/`end_page` 为 None 时处理全文。
pub fn extract_pdf_text_pages(
    input: &Path,
    start_page: Option<u32>,
    end_page: Option<u32>,
) -> Result<PdfTextExtraction> {
    validate_page_range(start_page, end_page)?;
    let pdftotext = PopplerTool::binary_path_for("pdftotext")
        .context("未找到 pdftotext，无法读取 PDF 文本层")?;
    let output = crate::util::fs::temp_named_path("docsy_pdf_text_layer", "txt");
    let args = build_pdftotext_args(input, &output, start_page, end_page);
    let mut cmd = crate::external::hidden_command(&pdftotext);
    cmd.args(&args);
    let command_result = run_pdftotext_cancellable(cmd);

    let text_result = std::fs::read_to_string(&output).context("读取 PDF 文本层输出失败");
    let _ = std::fs::remove_file(&output);
    let command_result = command_result?;
    if !command_result.status.success() {
        anyhow::bail!(
            "PDF 文本层提取失败：{}",
            crate::external::command_failure_detail(&command_result)
        );
    }

    let text = text_result?;
    // 借用切分，避免再持有一份完整文本拷贝；末尾空段是 pdftotext 结尾 \x0c 的产物，丢弃。
    let mut pages: Vec<&str> = text.split('\u{000c}').map(str::trim).collect();
    if pages.last().is_some_and(|page| page.is_empty()) {
        pages.pop();
    }

    let first_page = start_page.unwrap_or(1);
    let mut empty_pages = Vec::new();
    let mut pages_with_text = 0usize;
    for (index, page) in pages.iter().enumerate() {
        if page.is_empty() {
            empty_pages.push(first_page + index as u32);
        } else {
            pages_with_text += 1;
        }
    }
    if pages_with_text == 0 {
        anyhow::bail!(
            "没有读取到可复制的 PDF 文本层。该文件可能是扫描件；请在后续版本安装本地 AI 文档解析包后重新处理。"
        );
    }

    Ok(PdfTextExtraction {
        markdown: markdown_from_pages(&pages, first_page),
        pages_with_text,
        empty_pages,
    })
}

pub fn extract_pdf_text_markdown(input: &Path) -> Result<PdfTextExtraction> {
    extract_pdf_text_pages(input, None, None)
}

/// `first_page` 为第一段的原始页码（有页段时保持原页码偏移）。
fn markdown_from_pages(pages: &[&str], first_page: u32) -> String {
    let mut markdown = String::from(
        "<!-- 由 Docsy 从 PDF 现有文本层提取；不包含 OCR、版面重建或表格识别。 -->\n\n",
    );
    for (index, page) in pages.iter().enumerate() {
        let page_number = first_page + index as u32;
        markdown.push_str(&format!("## 第 {page_number} 页\n\n"));
        if page.is_empty() {
            markdown.push_str("> （本页无可提取的文本层，可能为扫描件或纯图片页。）");
        } else {
            markdown.push_str(page);
        }
        markdown.push_str("\n\n");
    }
    markdown
}

/// 空页提示文案（多页合并列出原始页码）。
pub(crate) fn empty_page_notice(empty_pages: &[u32]) -> Option<String> {
    if empty_pages.is_empty() {
        return None;
    }
    let list = empty_pages
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("、");
    Some(format!(
        "第 {list} 页无文本层，可能为扫描件，建议后续用 AI 文档解析包处理。"
    ))
}

/// `convert_pdf_text_layer` 命令的业务实现：页段校验、页数校验、文本层提取、落盘。
pub fn convert_pdf_text_layer(
    input: &str,
    output_dir: Option<&str>,
    start_page: Option<u32>,
    end_page: Option<u32>,
) -> Result<PdfTextLayerOutput> {
    validate_page_range(start_page, end_page)?;
    let input_path = PathBuf::from(input);
    if !input_path.is_file() {
        anyhow::bail!("输入文件不存在: {input}");
    }
    // 页数总数校验：qpdf 不可用时跳过，不阻塞转换。
    if let Ok(total) = crate::pdf::qpdf::page_count(input) {
        if let Some(end) = end_page {
            if end > total {
                anyhow::bail!("结束页（{end}）超出 PDF 总页数（{total}）");
            }
        }
    }

    let extracted = extract_pdf_text_pages(&input_path, start_page, end_page)?;
    let output_path = crate::markdown::output_path_for(&input_path, output_dir, "md")?;
    std::fs::write(&output_path, &extracted.markdown)
        .with_context(|| format!("写入 PDF Markdown 失败: {}", output_path.display()))?;

    let input_size = std::fs::metadata(&input_path)
        .with_context(|| format!("无法读取输入文件信息: {input}"))?
        .len();
    let output_size = std::fs::metadata(&output_path)
        .with_context(|| format!("输出文件生成失败: {}", output_path.display()))?
        .len();

    let mut warning = format!(
        "已从 {} 页可复制文本生成 Markdown；这是文本层提取，扫描件、表格和复杂版面请使用本地 AI 文档解析。",
        extracted.pages_with_text
    );
    if let Some(notice) = empty_page_notice(&extracted.empty_pages) {
        warning.push(' ');
        warning.push_str(&notice);
    }

    Ok(PdfTextLayerOutput {
        output_path: output_path.display().to_string(),
        pages_with_text: extracted.pages_with_text,
        empty_pages: extracted.empty_pages,
        input_size,
        output_size,
        warning: Some(warning),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_explicit_page_boundaries() {
        let markdown = markdown_from_pages(&["第一页", "第二页"], 1);
        assert!(markdown.contains("## 第 1 页\n\n第一页"));
        assert!(markdown.contains("## 第 2 页\n\n第二页"));
        assert!(markdown.starts_with("<!-- 由 Docsy"));
    }

    #[test]
    fn pdftotext_args_without_page_range() {
        let args = build_pdftotext_args(
            Path::new("/tmp/in.pdf"),
            Path::new("/tmp/out.txt"),
            None,
            None,
        );
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            ["-layout", "-enc", "UTF-8", "--", "/tmp/in.pdf", "/tmp/out.txt"]
        );
    }

    #[test]
    fn pdftotext_args_with_page_range() {
        let args = build_pdftotext_args(
            Path::new("/tmp/in.pdf"),
            Path::new("/tmp/out.txt"),
            Some(3),
            Some(9),
        );
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "-layout", "-enc", "UTF-8", "-f", "3", "-l", "9", "--", "/tmp/in.pdf",
                "/tmp/out.txt"
            ]
        );
    }

    #[test]
    fn page_range_validation_rejects_bad_values() {
        assert!(validate_page_range(Some(0), None).is_err());
        assert!(validate_page_range(None, Some(0)).is_err());
        assert!(validate_page_range(Some(5), Some(3)).is_err());
        assert!(validate_page_range(Some(1), Some(1)).is_ok());
        assert!(validate_page_range(None, Some(10)).is_ok());
        assert!(validate_page_range(Some(2), None).is_ok());
        assert!(validate_page_range(None, None).is_ok());
    }

    #[test]
    fn empty_pages_get_placeholder_and_offset_page_numbers() {
        let markdown = markdown_from_pages(&["第三页内容", "", "第五页内容"], 3);
        assert!(markdown.contains("## 第 3 页\n\n第三页内容"));
        assert!(markdown.contains("## 第 4 页\n\n> （本页无可提取的文本层"));
        assert!(markdown.contains("## 第 5 页\n\n第五页内容"));
    }

    #[test]
    fn empty_page_notice_lists_original_page_numbers() {
        assert_eq!(empty_page_notice(&[]), None);
        let notice = empty_page_notice(&[2, 5]).unwrap();
        assert!(notice.contains("第 2、5 页无文本层"));
    }

    /// 端到端：需要本机安装 poppler（pdftotext）。
    #[test]
    #[ignore = "requires poppler"]
    fn extract_fixture_pdf_end_to_end() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test-pdf/测试文件1.pdf");
        let extracted = extract_pdf_text_pages(&fixture, None, None).unwrap();
        assert!(extracted.pages_with_text > 0);
        assert!(extracted.markdown.contains("## 第 1 页"));

        // 页段：页码保持原始偏移
        let partial = extract_pdf_text_pages(&fixture, Some(2), Some(3)).unwrap();
        assert!(partial.markdown.contains("## 第 2 页"));
        assert!(!partial.markdown.contains("## 第 1 页"));

        assert!(extract_pdf_text_pages(&fixture, Some(4), Some(2)).is_err());
    }
}
