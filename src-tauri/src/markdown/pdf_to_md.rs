//! PDF 文本层 → Markdown。
//!
//! 这是 MD 转换中的轻量路径：只读取 PDF 已有文本层，不执行 OCR，不重采样原件。
//! 处理过程分两遍流式读取临时文本：第一遍识别跨页重复的页首/页尾噪声，第二遍逐页
//! 写 Markdown。因此大型 PDF 不会同时把全文文本层和 Markdown 保留在内存中。

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::external::PopplerTool;

const EDGE_LINE_COUNT: usize = 3;
const MAX_EDGE_LINE_CHARS: usize = 240;

#[derive(Debug)]
pub struct PdfTextLayerOutput {
    pub output_path: String,
    pub pages_with_text: usize,
    /// 范围内去除重复页首/页尾文本后无正文的页码（原始页码，1-based）。
    pub empty_pages: Vec<u32>,
    pub input_size: u64,
    pub output_size: u64,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PageEdge {
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EdgeLineKey {
    edge: PageEdge,
    fingerprint: String,
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

/// 用与 OperationManager 相同的 ID 注册子进程；取消 token 后立即终止 pdftotext。
fn run_pdftotext_cancellable(
    mut cmd: Command,
    operation_id: Option<&str>,
    token: Option<&CancellationToken>,
) -> Result<Output> {
    cmd.stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = cmd.spawn().context("执行 pdftotext 失败")?;
    let mut stderr_reader = child
        .stderr
        .take()
        .map(crate::external::spawn_bounded_output_reader);
    let registry = crate::get_subprocess_registry();
    if let (Some(registry), Some(operation_id)) = (registry, operation_id) {
        registry.register(operation_id, child.id());
    }

    loop {
        if token.is_some_and(|token| token.is_cancelled()) {
            if let (Some(registry), Some(operation_id)) = (registry, operation_id) {
                let _ = registry.cancel(operation_id);
            } else {
                let _ = child.kill();
            }
            let _ = child.wait();
            crate::external::finish_bounded_output_reader(stderr_reader.take());
            anyhow::bail!("操作已取消");
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let stderr = crate::external::finish_bounded_output_reader(stderr_reader.take());
                if let (Some(registry), Some(operation_id)) = (registry, operation_id) {
                    registry.unregister(operation_id);
                }
                return Ok(Output {
                    status,
                    stdout: Vec::new(),
                    stderr,
                });
            }
            Ok(None) => {}
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                crate::external::finish_bounded_output_reader(stderr_reader.take());
                if let (Some(registry), Some(operation_id)) = (registry, operation_id) {
                    registry.unregister(operation_id);
                }
                return Err(err).context("检查 pdftotext 进程状态失败");
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn run_pdftotext_to_file(
    input: &Path,
    output: &Path,
    start_page: Option<u32>,
    end_page: Option<u32>,
    operation_id: Option<&str>,
    token: Option<&CancellationToken>,
) -> Result<()> {
    let pdftotext = PopplerTool::binary_path_for("pdftotext")
        .context("未找到 pdftotext，无法读取 PDF 文本层")?;
    let mut cmd = crate::external::hidden_command(&pdftotext);
    cmd.args(build_pdftotext_args(input, output, start_page, end_page));
    let command_result = run_pdftotext_cancellable(cmd, operation_id, token)?;
    if !command_result.status.success() {
        anyhow::bail!(
            "PDF 文本层提取失败：{}",
            crate::external::command_failure_detail(&command_result)
        );
    }
    if !output.is_file() {
        anyhow::bail!("PDF 文本层提取未生成输出文件");
    }
    crate::util::fs::set_private_permissions(output).ok();
    Ok(())
}

/// 按分页符读取单页。pdftotext 末尾通常带一个空页段，调用方负责忽略。
fn for_each_text_page(
    path: &Path,
    first_page: u32,
    mut visit: impl FnMut(u32, Vec<String>) -> Result<()>,
) -> Result<u32> {
    let mut reader = BufReader::new(File::open(path).context("读取 PDF 文本层输出失败")?);
    let mut buffer = Vec::new();
    let mut page_number = first_page;
    let mut pages = 0_u32;
    loop {
        buffer.clear();
        let read = reader.read_until(b'\x0c', &mut buffer)?;
        if read == 0 {
            break;
        }
        if buffer.last() == Some(&b'\x0c') {
            buffer.pop();
        }
        // pdftotext 使用 UTF-8 输出；遇到损坏字节时使用替换字符保留其他可读内容。
        let raw = String::from_utf8_lossy(&buffer);
        if raw.is_empty() && read > 0 && reader.fill_buf()?.is_empty() {
            break;
        }
        let lines = raw.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        visit(page_number, lines)?;
        page_number = page_number.saturating_add(1);
        pages = pages.saturating_add(1);
    }
    Ok(pages)
}

fn normalized_edge_fingerprint(line: &str) -> Option<String> {
    let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() || collapsed.chars().count() > MAX_EDGE_LINE_CHARS {
        return None;
    }
    let mut result = String::with_capacity(collapsed.len());
    let mut previous_was_number = false;
    for ch in collapsed.chars() {
        let is_number = ch.is_ascii_digit() || ('０'..='９').contains(&ch);
        if is_number {
            if !previous_was_number {
                result.push('#');
            }
        } else {
            result.push(ch);
        }
        previous_was_number = is_number;
    }
    Some(result)
}

fn edge_line_keys(lines: &[String]) -> Vec<(usize, EdgeLineKey)> {
    let nonempty = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let mut keys = Vec::new();
    for (index, line) in nonempty.iter().take(EDGE_LINE_COUNT) {
        if let Some(fingerprint) = normalized_edge_fingerprint(line) {
            keys.push((
                *index,
                EdgeLineKey {
                    edge: PageEdge::Top,
                    fingerprint,
                },
            ));
        }
    }
    for (index, line) in nonempty.iter().rev().take(EDGE_LINE_COUNT) {
        if let Some(fingerprint) = normalized_edge_fingerprint(line) {
            keys.push((
                *index,
                EdgeLineKey {
                    edge: PageEdge::Bottom,
                    fingerprint,
                },
            ));
        }
    }
    keys
}

fn detect_repeating_edge_lines(path: &Path, first_page: u32) -> Result<HashSet<EdgeLineKey>> {
    let mut occurrences: HashMap<EdgeLineKey, HashSet<u32>> = HashMap::new();
    let page_count = for_each_text_page(path, first_page, |page, lines| {
        for (_, key) in edge_line_keys(&lines) {
            occurrences.entry(key).or_default().insert(page);
        }
        Ok(())
    })?;
    if page_count < 2 {
        return Ok(HashSet::new());
    }
    Ok(occurrences
        .into_iter()
        .filter_map(|(key, pages)| (pages.len() >= 2).then_some(key))
        .collect())
}

fn clean_page_lines(lines: &[String], repeating_edges: &HashSet<EdgeLineKey>) -> Vec<String> {
    let indexes_to_remove = edge_line_keys(lines)
        .into_iter()
        .filter_map(|(index, key)| repeating_edges.contains(&key).then_some(index))
        .collect::<HashSet<_>>();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (!indexes_to_remove.contains(&index)).then_some(line.clone()))
        .collect()
}

fn is_page_text_empty(lines: &[String]) -> bool {
    lines.iter().all(|line| line.trim().is_empty())
}

fn render_markdown(
    text_path: &Path,
    markdown_path: &Path,
    first_page: u32,
    repeating_edges: &HashSet<EdgeLineKey>,
    token: Option<&CancellationToken>,
) -> Result<(usize, Vec<u32>)> {
    let output = File::create(markdown_path)
        .with_context(|| format!("写入 PDF Markdown 失败: {}", markdown_path.display()))?;
    let mut writer = BufWriter::new(output);
    writer.write_all(
        "<!-- 由 Docsy 从 PDF 现有文本层提取；已滤除跨页重复的页首和页尾文本；不包含 OCR、版面重建或表格识别。 -->\n\n".as_bytes(),
    )?;
    let mut pages_with_text = 0usize;
    let mut empty_pages = Vec::new();
    for_each_text_page(text_path, first_page, |page, lines| {
        if token.is_some_and(|token| token.is_cancelled()) {
            anyhow::bail!("操作已取消");
        }
        let lines = clean_page_lines(&lines, repeating_edges);
        writeln!(writer, "## 第 {page} 页\n")?;
        if is_page_text_empty(&lines) {
            empty_pages.push(page);
            writeln!(
                writer,
                "> （本页无可靠的可提取文本层，可能为扫描件或纯图片页。）\n"
            )?;
        } else {
            pages_with_text += 1;
            for line in lines {
                writeln!(writer, "{line}")?;
            }
            writeln!(writer)?;
        }
        Ok(())
    })?;
    writer.flush()?;
    Ok((pages_with_text, empty_pages))
}

/// PDF 文本层 → Markdown 的生产入口。全文仅落在受限临时文件和最终输出中。
pub fn convert_pdf_text_layer(
    input: &str,
    output_dir: Option<&str>,
    start_page: Option<u32>,
    end_page: Option<u32>,
    operation_id: Option<&str>,
    token: Option<&CancellationToken>,
) -> Result<PdfTextLayerOutput> {
    validate_page_range(start_page, end_page)?;
    // A cancelled task must not be reported as a missing input/tool error.
    // This also keeps queued conversions cancellable before any disk access.
    if token.is_some_and(CancellationToken::is_cancelled) {
        anyhow::bail!("操作已取消");
    }
    let input_path = PathBuf::from(input);
    if !input_path.is_file() {
        anyhow::bail!("输入文件不存在: {input}");
    }
    let first_page = start_page.unwrap_or(1);
    let temp_path = crate::util::fs::temp_named_path("docsy_pdf_text_layer", "txt");
    let temp_guard = crate::util::fs::TempPathGuard::new(temp_path.clone());
    run_pdftotext_to_file(
        &input_path,
        temp_guard.path(),
        start_page,
        end_page,
        operation_id,
        token,
    )?;
    let repeating_edges = detect_repeating_edge_lines(temp_guard.path(), first_page)?;
    let output_path = crate::markdown::output_path_for(&input_path, output_dir, "md")?;
    let render_result = render_markdown(
        temp_guard.path(),
        &output_path,
        first_page,
        &repeating_edges,
        token,
    );
    let (pages_with_text, empty_pages) = match render_result {
        Ok(result) => result,
        Err(error) => {
            let _ = std::fs::remove_file(&output_path);
            return Err(error);
        }
    };
    if pages_with_text == 0 {
        let _ = std::fs::remove_file(&output_path);
        anyhow::bail!("没有读取到可靠的 PDF 文本层。该文件可能是扫描件，当前无法转换为 Markdown。");
    }

    let input_size = std::fs::metadata(&input_path)
        .with_context(|| format!("无法读取输入文件信息: {input}"))?
        .len();
    let output_size = std::fs::metadata(&output_path)
        .with_context(|| format!("输出文件生成失败: {}", output_path.display()))?
        .len();
    let mut warning = format!(
        "已从 {pages_with_text} 页可靠文本生成 Markdown；表格、公式和复杂版面可能无法完整保留。"
    );
    if !empty_pages.is_empty() {
        let list = empty_pages
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("、");
        warning.push_str(&format!(" 第 {list} 页无可靠文本层，已保留页码占位。"));
    }
    if !repeating_edges.is_empty() {
        warning.push_str(" 已滤除跨页重复的页首/页尾文本。");
    }
    Ok(PdfTextLayerOutput {
        output_path: output_path.display().to_string(),
        pages_with_text,
        empty_pages,
        input_size,
        output_size,
        warning: Some(warning),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_text_fixture_pdf(path: &Path) {
        use lopdf::{
            content::{Content, Operation},
            dictionary, Document, Object, Stream,
        };

        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = document.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let mut page_ids = Vec::new();
        for (index, text) in ["First page text", "Second page text"].iter().enumerate() {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                    Operation::new(
                        "Tm",
                        vec![
                            1.into(),
                            0.into(),
                            0.into(),
                            1.into(),
                            72.into(),
                            (760 - index as i32 * 20).into(),
                        ],
                    ),
                    Operation::new("Tj", vec![Object::string_literal(*text)]),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id =
                document.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = document.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            });
            page_ids.push(page_id);
        }
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => page_ids.into_iter().map(Into::into).collect::<Vec<Object>>(),
                "Count" => 2,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);
        document.save(path).unwrap();
    }

    fn temp_test_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "docsy-pdf-md-{label}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn pdftotext_args_with_page_range() {
        let args = build_pdftotext_args(
            Path::new("/tmp/in.pdf"),
            Path::new("/tmp/out.txt"),
            Some(3),
            Some(9),
        );
        let args = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "-layout",
                "-enc",
                "UTF-8",
                "-f",
                "3",
                "-l",
                "9",
                "--",
                "/tmp/in.pdf",
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
    }

    #[test]
    fn normalizes_varying_page_numbers_without_content_specific_rules() {
        assert_eq!(
            normalized_edge_fingerprint("证据 13"),
            Some("证据 #".into())
        );
        assert_eq!(normalized_edge_fingerprint("— 2 —"), Some("— # —".into()));
    }

    #[test]
    fn only_repeated_edge_text_is_removed() {
        let repeated = EdgeLineKey {
            edge: PageEdge::Top,
            fingerprint: "证据 #".into(),
        };
        let mut remove = HashSet::new();
        remove.insert(repeated);
        let lines = vec!["证据 13".into(), "正文内容".into(), "— 2 —".into()];
        let cleaned = clean_page_lines(&lines, &remove);
        assert_eq!(cleaned, vec!["正文内容", "— 2 —"]);
    }

    #[test]
    fn repeating_page_edges_leave_header_only_pages_empty() {
        let path = std::env::temp_dir().join(format!(
            "docsy-pdf-md-edges-{}-{}.txt",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::write(&path, "证据 13\n\n— 1 —\u{000c}证据 14\n\n— 2 —\u{000c}").unwrap();

        let repeated = detect_repeating_edge_lines(&path, 1).unwrap();
        let page_one = vec!["证据 13".into(), "".into(), "— 1 —".into()];
        let page_two = vec!["证据 14".into(), "".into(), "— 2 —".into()];
        assert!(is_page_text_empty(&clean_page_lines(&page_one, &repeated)));
        assert!(is_page_text_empty(&clean_page_lines(&page_two, &repeated)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn end_to_end_extracts_fixture_pdf() {
        let dir = temp_test_dir("fixture");
        std::fs::create_dir_all(&dir).unwrap();
        let fixture = dir.join("fixture.pdf");
        create_text_fixture_pdf(&fixture);
        let result = convert_pdf_text_layer(
            fixture.to_str().unwrap(),
            Some(dir.to_str().unwrap()),
            Some(2),
            Some(3),
            None,
            None,
        )
        .unwrap();
        let markdown = std::fs::read_to_string(&result.output_path).unwrap();
        assert!(markdown.contains("## 第 2 页"));
        assert!(!markdown.contains("## 第 1 页"));
        assert!(markdown.contains("Second page text"));
        let _ = std::fs::remove_file(&result.output_path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancellation_stops_before_pdf_text_is_rendered() {
        let dir = temp_test_dir("cancel");
        std::fs::create_dir_all(&dir).unwrap();
        let fixture = dir.join("fixture.pdf");
        create_text_fixture_pdf(&fixture);
        let token = CancellationToken::new();
        token.cancel();
        let error = convert_pdf_text_layer(
            fixture.to_str().unwrap(),
            Some(dir.to_str().unwrap()),
            None,
            None,
            None,
            Some(&token),
        )
        .unwrap_err();
        assert!(error.to_string().contains("操作已取消"));
        let _ = std::fs::remove_dir(&dir);
    }
}
