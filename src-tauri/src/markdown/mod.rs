//! Markdown ↔ Office 文档转换。
//!
//! - `.md` / `.markdown` → `.docx` / `.xlsx` / `.pptx` / `.html`
//! - Office / OpenDocument / RTF / CSV / EPUB → `.md`
//! - `.doc` → 两条引擎，由前端让用户选择：
//!   - `word`：本机 Word/WPS 自动化另存临时 `.docx`（高保真），失败报错提示手动另存；
//!   - `extract`（默认）：AnyDoc 直接提取结构化 Markdown（不执行宏）。

mod docx_to_md;
mod md_to_docx;
pub(crate) mod pdf_to_md;

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::util::fs::{unique_output_path, TempPathGuard};

/// 转换方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    MdToDocx,
    MdToXlsx,
    MdToPptx,
    MdToHtml,
    OfficeToMd,
    PdfToMd,
}

#[derive(Debug, Serialize)]
pub struct ConvertResult {
    pub output_path: String,
    pub direction: Direction,
    pub source_format: String,
    pub output_format: String,
    pub input_size: u64,
    pub output_size: u64,
    /// 有损语义转换时的用户提示（如宏、精确版式或动画无法写入 Markdown）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Markdown 能生成的输出格式：三种 Office 格式 + HTML 网页。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeOutputFormat {
    Docx,
    Xlsx,
    Pptx,
    Html,
}

impl OfficeOutputFormat {
    fn parse(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("docx") {
            "docx" => Ok(Self::Docx),
            "xlsx" => Ok(Self::Xlsx),
            "pptx" => Ok(Self::Pptx),
            "html" => Ok(Self::Html),
            other => anyhow::bail!("不支持的 Markdown 输出格式: {other}"),
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Xlsx => "xlsx",
            Self::Pptx => "pptx",
            Self::Html => "html",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Xlsx => "xlsx",
            Self::Pptx => "pptx",
            Self::Html => "html",
        }
    }

    fn direction(self) -> Direction {
        match self {
            Self::Docx => Direction::MdToDocx,
            Self::Xlsx => Direction::MdToXlsx,
            Self::Pptx => Direction::MdToPptx,
            Self::Html => Direction::MdToHtml,
        }
    }
}

/// .doc 转换引擎：`extract` = AnyDoc 直接提取；`word` = 本机 Word/WPS
/// 自动化另存 docx（高保真，需安装了 Word 或 WPS）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocEngine {
    Extract,
    Word,
}

fn parse_doc_engine(value: Option<&str>) -> Result<DocEngine> {
    match value.unwrap_or("extract") {
        "extract" => Ok(DocEngine::Extract),
        "word" => Ok(DocEngine::Word),
        other => anyhow::bail!("未知的 .doc 转换引擎: {other}"),
    }
}

/// 按扩展名判定转换方向；不支持的类型报错。
fn source_extension(input: &Path) -> String {
    input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

fn is_markdown_input(input: &Path) -> bool {
    matches!(
        source_extension(input).as_str(),
        "md" | "markdown" | "mdown" | "mkdn" | "mdwn" | "mdtxt"
    )
}

fn is_office_input(input: &Path) -> bool {
    matches!(
        source_extension(input).as_str(),
        "doc"
            | "docx"
            | "docm"
            | "xls"
            | "xlsx"
            | "xlsm"
            | "xlsb"
            | "ppt"
            | "pptx"
            | "pptm"
            | "pps"
            | "ppsx"
            | "ppsm"
            | "pot"
            | "potx"
            | "potm"
            | "odt"
            | "ods"
            | "odp"
            | "rtf"
            | "csv"
            | "epub"
    )
}

fn is_pdf_input(input: &Path) -> bool {
    source_extension(input) == "pdf"
}

fn detect_direction(input: &Path) -> Result<Direction> {
    let ext = source_extension(input);
    match ext.as_str() {
        "md" | "markdown" | "mdown" | "mkdn" | "mdwn" | "mdtxt" => Ok(Direction::MdToDocx),
        "pdf" => Ok(Direction::PdfToMd),
        _ if is_office_input(input) => Ok(Direction::OfficeToMd),
        _ => anyhow::bail!("不支持的文件类型: {}", input.display()),
    }
}

/// 计算输出路径：默认放输入同目录，文件名同 stem 换后缀，撞名时自动加序号。
pub(crate) fn output_path_for(
    input: &Path,
    output_dir: Option<&str>,
    extension: &str,
) -> Result<PathBuf> {
    let dir = match output_dir {
        Some(d) => {
            let dir = PathBuf::from(d);
            if !dir.is_dir() {
                anyhow::bail!("输出目录不存在: {}", dir.display());
            }
            dir
        }
        None => input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .context("无法解析输入文件名")?;
    Ok(unique_output_path(&dir, stem, extension))
}

/// 旧版 .doc → 临时 .docx，走本机 Word/WPS 自动化（高保真）。
/// 用户显式选择此引擎，失败直接报错（提示手动另存），不回退纯文本。
fn convert_doc_to_temp_docx_with_word(input: &Path) -> Result<TempPathGuard> {
    let guard_path = crate::util::fs::temp_named_path("docsy-doc2docx-word", "docx");
    word_save_as_docx(input, &guard_path).with_context(|| {
        format!(
            "Word/WPS 自动转换失败: {}。可以改用纯文本转换，或用 Word/WPS 手动另存为 .docx 后再转换。",
            input.display()
        )
    })?;
    if !guard_path.is_file() {
        anyhow::bail!(
            "Word/WPS 未生成预期输出: {}。可以改用纯文本转换，或手动另存为 .docx 后再转换。",
            guard_path.display()
        );
    }
    Ok(TempPathGuard::new(guard_path))
}

/// Windows：COM 自动化，先 Word.Application，失败再 KWPS.Application（WPS 文字）。
/// FileFormat 16 = wdFormatXMLDocument (.docx)。
#[cfg(windows)]
fn word_save_as_docx(input: &Path, output: &Path) -> Result<()> {
    let escape = |p: &Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop';\
         $app=$null;$doc=$null;\
         try {{\
           try {{ $app=New-Object -ComObject Word.Application }} catch {{ $app=New-Object -ComObject KWPS.Application }};\
           $app.Visible=$false;\
           try {{ $app.DisplayAlerts=0 }} catch {{ }};\
           $doc=$app.Documents.Open('{input}', $false, $true);\
           try {{ $doc.SaveAs2('{output}', 16) }} catch {{ $doc.SaveAs('{output}', 16) }};\
         }} finally {{\
           if ($doc -ne $null) {{ try {{ $doc.Close([ref]$false) | Out-Null }} catch {{ }} }};\
           if ($app -ne $null) {{ try {{ $app.Quit() | Out-Null }} catch {{ }} }};\
           [System.GC]::Collect();\
           [System.GC]::WaitForPendingFinalizers();\
         }}",
        input = escape(input),
        output = escape(output),
    );
    let mut cmd = crate::external::hidden_command("powershell");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &script,
    ]);
    let result =
        crate::external::command_output_with_timeout(&mut cmd, std::time::Duration::from_secs(120))
            .context("执行 Word/WPS 转换进程失败")?;
    if !result.status.success() {
        anyhow::bail!(
            "Word/WPS 进程返回错误: {}",
            crate::external::command_failure_detail(&result)
        );
    }
    Ok(())
}

/// macOS：AppleScript 驱动 Microsoft Word 另存（format document default = .docx）。
/// WPS for Mac 不支持 AppleScript，与证据模块的 doc→pdf 保持一致只走 Word。
#[cfg(target_os = "macos")]
fn word_save_as_docx(input: &Path, output: &Path) -> Result<()> {
    let input = std::fs::canonicalize(input).context("读取 .doc 文件失败")?;
    let script = r#"
on run argv
  set inputPath to item 1 of argv
  set outputPath to item 2 of argv
  set inputHfsPath to POSIX file inputPath as text
  set outputFile to POSIX file outputPath
  set docRef to missing value
  tell application "Microsoft Word"
    set visible to false
    try
      open file inputHfsPath
      set docRef to active document
      save as docRef file name outputFile file format format document default
    on error errMsg number errNum
      try
        if docRef is not missing value then close docRef saving no
      end try
      error errMsg number errNum
    end try
    close docRef saving no
  end tell
end run
"#;
    let mut command = crate::external::hidden_command("osascript");
    command
        .arg("-e")
        .arg(script)
        .arg(input.display().to_string())
        .arg(output.display().to_string());
    let result = crate::external::command_output_with_timeout(
        &mut command,
        std::time::Duration::from_secs(120),
    )
    .context("执行 Microsoft Word 转换进程失败")?;
    if !result.status.success() {
        anyhow::bail!(
            "Microsoft Word 进程返回错误: {}",
            crate::external::command_failure_detail(&result)
        );
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn word_save_as_docx(_input: &Path, _output: &Path) -> Result<()> {
    anyhow::bail!("当前平台不支持 Word/WPS 自动转换，请手动另存为 .docx")
}

fn markdown_to_office(
    input: &Path,
    output: &Path,
    target: OfficeOutputFormat,
    style: md_to_docx::DocxStylePreset,
) -> Result<()> {
    let markdown = std::fs::read_to_string(input)
        .with_context(|| format!("无法读取 Markdown 文件: {}", input.display()))?;
    match target {
        OfficeOutputFormat::Docx => md_to_docx::convert(input, output, style),
        OfficeOutputFormat::Xlsx => office_oxide::create::create_from_markdown(
            &markdown,
            office_oxide::DocumentFormat::Xlsx,
            output,
        )
        .map_err(|error| anyhow::anyhow!("生成 Excel 失败: {error}")),
        OfficeOutputFormat::Pptx => office_oxide::create::create_from_markdown(
            &markdown,
            office_oxide::DocumentFormat::Pptx,
            output,
        )
        .map_err(|error| anyhow::anyhow!("生成 PowerPoint 失败: {error}")),
        OfficeOutputFormat::Html => {
            let title = input.file_stem().and_then(|s| s.to_str()).unwrap_or("文档");
            std::fs::write(output, markdown_to_html_document(title, &markdown))
                .with_context(|| format!("无法写入 HTML 文件: {}", output.display()))
        }
    }
}

/// 将 AnyDoc 可识别的 Office 文档提取为 Markdown。
///
/// 此路径只读取内容，绝不执行 Office 宏；导出的 Markdown 保留语义样式而非页面像素布局。
#[derive(Debug, Default)]
struct OfficeImportFacts {
    asset_count: usize,
    note_count: usize,
}

fn office_asset_dir(output: &Path) -> Result<PathBuf> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .context("无法解析 Markdown 输出文件名")?;
    Ok(parent.join(format!("{stem}_assets")))
}

fn office_to_markdown(input: &Path, output: &Path) -> Result<OfficeImportFacts> {
    let asset_dir = office_asset_dir(output)?;
    let asset_dir_name = asset_dir
        .file_name()
        .and_then(|value| value.to_str())
        .context("无法解析 Markdown 资源目录名")?;
    let converted = docsy_anydoc::docsy::extract_office_markdown(input, asset_dir_name)
        .map_err(|error| anyhow::anyhow!("读取 Office 文档失败: {error}"))?;
    if converted.markdown.trim().is_empty() {
        anyhow::bail!("未从文件中提取到可转换的文本内容: {}", input.display());
    }
    if !converted.assets.is_empty() {
        std::fs::create_dir_all(&asset_dir)
            .with_context(|| format!("无法创建资源目录: {}", asset_dir.display()))?;
        for asset in &converted.assets {
            let asset_path = asset_dir.join(&asset.file_name);
            std::fs::write(&asset_path, &asset.bytes)
                .with_context(|| format!("无法导出资源文件: {}", asset_path.display()))?;
        }
    }
    std::fs::write(output, converted.markdown)
        .with_context(|| format!("无法写入 Markdown 文件: {}", output.display()))?;
    Ok(OfficeImportFacts {
        asset_count: converted.asset_count,
        note_count: converted.note_count,
    })
}

fn office_input_warning(extension: &str, facts: &OfficeImportFacts) -> Option<String> {
    let base = match extension {
        "docm" => Some("文档中的宏不会执行，也不会写入 Markdown 输出。".to_string()),
        "ppt" | "pptx" | "pps" | "ppsx" | "pot" | "potx" => Some(
            "已提取幻灯片文字、列表、表格和讲者备注；动画、精确布局与图形效果无法在 Markdown 中保留。"
                .to_string(),
        ),
        "pptm" | "ppsm" | "potm" => Some(
            "文档中的宏不会执行；已提取幻灯片文字、列表、表格和讲者备注，动画与精确布局无法在 Markdown 中保留。"
                .to_string(),
        ),
        "xls" | "xlsx" | "xlsb" => Some(
            "已提取工作表结构和单元格内容；公式显示结果、图表、条件格式和精确列宽无法完整映射到 Markdown。"
                .to_string(),
        ),
        "xlsm" => Some(
            "文档中的宏不会执行；已提取工作表结构和单元格内容，图表、条件格式和精确列宽无法完整映射到 Markdown。"
                .to_string(),
        ),
        "odt" | "ods" | "odp" | "rtf" => Some(
            "已提取可表达的正文结构和文字样式；精确页面版式可能与源文件不同。".to_string(),
        ),
        "epub" => Some(
            "已提取电子书章节正文结构；内嵌样式、字体和精确排版不会在 Markdown 中保留。"
                .to_string(),
        ),
        _ => None,
    };
    let mut notices = base.into_iter().collect::<Vec<_>>();
    if facts.asset_count > 0 {
        notices.push(format!(
            "已导出 {} 个嵌入资源到同级 assets 目录。",
            facts.asset_count
        ));
    }
    if facts.note_count > 0 {
        notices.push(format!("已保留 {} 条脚注或尾注。", facts.note_count));
    }
    (!notices.is_empty()).then(|| notices.join(" "))
}

pub fn convert(
    input: &str,
    output_dir: Option<&str>,
    doc_engine: Option<&str>,
    output_format: Option<&str>,
    docx_style: Option<&str>,
) -> Result<ConvertResult> {
    let input_path = PathBuf::from(input);
    if !input_path.is_file() {
        anyhow::bail!("输入文件不存在: {input}");
    }
    let ext = source_extension(&input_path);
    let markdown_input = is_markdown_input(&input_path);
    let pdf_input = is_pdf_input(&input_path);
    let detected_direction = detect_direction(&input_path)?;
    // Office → Markdown 不消费 Markdown 输出参数，避免旧调用方传入无关值导致失败。
    let target = if markdown_input {
        OfficeOutputFormat::parse(output_format)?
    } else {
        OfficeOutputFormat::Docx
    };
    let style = if markdown_input {
        md_to_docx::DocxStylePreset::parse(docx_style)?
    } else {
        md_to_docx::DocxStylePreset::Professional
    };
    let output_extension = if markdown_input {
        target.extension()
    } else {
        "md"
    };
    let output_path = output_path_for(&input_path, output_dir, output_extension)?;
    let input_size = std::fs::metadata(&input_path)
        .with_context(|| format!("无法读取输入文件信息: {input}"))?
        .len();
    let mut warning: Option<String> = None;

    let (direction, source_format, output_format) = if markdown_input {
        markdown_to_office(&input_path, &output_path, target, style)?;
        (
            target.direction(),
            "markdown".to_string(),
            target.label().to_string(),
        )
    } else if pdf_input {
        // 非命令调用同样走流式实现；此路径没有可取消任务时传 None。
        let extracted =
            pdf_to_md::convert_pdf_text_layer(input, output_dir, None, None, None, None)?;
        return Ok(ConvertResult {
            output_path: extracted.output_path,
            direction: Direction::PdfToMd,
            source_format: "pdf".to_string(),
            output_format: "markdown".to_string(),
            input_size: extracted.input_size,
            output_size: extracted.output_size,
            warning: extracted.warning,
        });
    } else {
        match ext.as_str() {
            "docx" | "docm" => {
                if let Err(error) = docx_to_md::convert(&input_path, &output_path) {
                    let facts =
                        office_to_markdown(&input_path, &output_path).with_context(|| {
                            format!("Docsy Word 读取器失败（{error}），AnyDoc 兜底读取也失败")
                        })?;
                    warning = office_input_warning(&ext, &facts).or_else(|| {
                        Some(
                            "已使用兼容读取器导出 Markdown；个别图片或细节格式可能简化。"
                                .to_string(),
                        )
                    });
                }
            }
            "doc" => {
                let engine = parse_doc_engine(doc_engine)?;
                if engine == DocEngine::Word {
                    let guard = convert_doc_to_temp_docx_with_word(&input_path)?;
                    docx_to_md::convert(guard.path(), &output_path)?;
                } else {
                    let facts = office_to_markdown(&input_path, &output_path)?;
                    let base =
                        "已直接读取旧版 .doc；复杂图文排版建议改用 Word/WPS 中转以获得更完整的结构。"
                            .to_string();
                    warning = office_input_warning(&ext, &facts)
                        .map(|notice| format!("{base} {notice}"))
                        .or(Some(base));
                }
            }
            _ => {
                let facts = office_to_markdown(&input_path, &output_path)?;
                warning = office_input_warning(&ext, &facts);
            }
        }
        if warning.is_none() && ext == "docm" {
            warning = office_input_warning(&ext, &OfficeImportFacts::default());
        }
        (detected_direction, ext, "markdown".to_string())
    };

    let output_size = std::fs::metadata(&output_path)
        .with_context(|| format!("输出文件生成失败: {}", output_path.display()))?
        .len();
    Ok(ConvertResult {
        output_path: output_path.display().to_string(),
        direction,
        source_format,
        output_format,
        input_size,
        output_size,
        warning,
    })
}

/// `convert_markdown_text` 的返回结果（snake_case JSON）。
#[derive(Debug, Serialize)]
pub struct ConvertTextResult {
    pub output_path: String,
    pub format: String,
    pub input_size: u64,
    pub output_size: u64,
}

/// 粘贴即转的输出目录：显式指定 > 用户 Downloads > 系统临时目录。
fn text_output_dir(output_dir: Option<&str>) -> Result<PathBuf> {
    match output_dir {
        Some(d) => {
            let dir = PathBuf::from(d);
            if !dir.is_dir() {
                anyhow::bail!("输出目录不存在: {}", dir.display());
            }
            Ok(dir)
        }
        None => Ok(dirs::download_dir().unwrap_or_else(std::env::temp_dir)),
    }
}

/// 粘贴文本的文件名主干：用户指定（做安全清洗），否则 `文档-<时间戳>`。
fn text_file_stem(file_stem: Option<&str>) -> String {
    match file_stem.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => crate::util::fs::safe_file_stem(s),
        None => format!("文档-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")),
    }
}

/// 最小 HTML 骨架：utf-8 meta + 简单排版样式。
fn wrap_html_fragment(title: &str, fragment: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\">\n\
<title>{title}</title>\n<style>\n\
body {{ max-width: 820px; margin: 2em auto; padding: 0 1em; \
font-family: -apple-system, \"Segoe UI\", \"PingFang SC\", \"Microsoft YaHei\", sans-serif; \
line-height: 1.7; }}\n\
code {{ background: #f5f5f5; padding: 0.15em 0.35em; border-radius: 4px; \
font-family: Consolas, Menlo, monospace; }}\n\
pre code {{ display: block; padding: 0.8em; overflow-x: auto; }}\n\
table {{ border-collapse: collapse; }}\n\
th, td {{ border: 1px solid #ddd; padding: 0.35em 0.8em; }}\n\
blockquote {{ margin: 0; padding-left: 1em; border-left: 4px solid #ddd; color: #666; }}\n\
</style>\n</head>\n<body>\n{fragment}</body>\n</html>\n"
    )
}

/// Markdown → 完整 HTML 文档（含骨架样式）。文件队列与粘贴即转共用。
fn markdown_to_html_document(title: &str, markdown: &str) -> String {
    let options = pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_STRIKETHROUGH
        | pulldown_cmark::Options::ENABLE_TASKLISTS;
    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let mut fragment = String::new();
    pulldown_cmark::html::push_html(&mut fragment, parser);
    wrap_html_fragment(title, &fragment)
}

/// 粘贴 Markdown 文本直接转换为 Office / HTML 文件。
pub fn convert_text(
    text: &str,
    format: &str,
    output_dir: Option<&str>,
    file_stem: Option<&str>,
    docx_style: Option<&str>,
) -> Result<ConvertTextResult> {
    if text.trim().is_empty() {
        anyhow::bail!("粘贴内容为空，无法转换");
    }
    let ext = match format {
        "docx" | "xlsx" | "pptx" | "html" => format,
        other => anyhow::bail!("不支持的输出格式: {other}（支持 docx / xlsx / pptx / html）"),
    };
    let dir = text_output_dir(output_dir)?;
    let stem = text_file_stem(file_stem);
    let output_path = unique_output_path(&dir, &stem, ext);

    match ext {
        "docx" | "xlsx" | "pptx" => {
            // 复用 md → docx 管线：先落临时 .md，转换后由守卫自动清理
            let temp_path = crate::util::fs::temp_named_path("docsy-paste-md", "md");
            std::fs::write(&temp_path, text).context("写入临时 Markdown 文件失败")?;
            let guard = TempPathGuard::new(temp_path);
            let target = OfficeOutputFormat::parse(Some(ext))?;
            let style = md_to_docx::DocxStylePreset::parse(docx_style)?;
            markdown_to_office(guard.path(), &output_path, target, style)?;
        }
        "html" => {
            std::fs::write(&output_path, markdown_to_html_document(&stem, text))
                .with_context(|| format!("无法写入 HTML 文件: {}", output_path.display()))?;
        }
        _ => unreachable!(),
    }

    let output_size = std::fs::metadata(&output_path)
        .with_context(|| format!("输出文件生成失败: {}", output_path.display()))?
        .len();
    Ok(ConvertTextResult {
        output_path: output_path.display().to_string(),
        format: ext.to_string(),
        input_size: text.len() as u64,
        output_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_direction_by_extension() {
        assert_eq!(
            detect_direction(Path::new("/tmp/a.md")).unwrap(),
            Direction::MdToDocx
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.markdown")).unwrap(),
            Direction::MdToDocx
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.DOCX")).unwrap(),
            Direction::OfficeToMd
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.doc")).unwrap(),
            Direction::OfficeToMd
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.xlsm")).unwrap(),
            Direction::OfficeToMd
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.odp")).unwrap(),
            Direction::OfficeToMd
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.pdf")).unwrap(),
            Direction::PdfToMd
        );
        assert!(detect_direction(Path::new("/tmp/a.txt")).is_err());
        assert!(detect_direction(Path::new("/tmp/noext")).is_err());
    }

    #[test]
    fn output_path_uses_input_dir_and_swapped_ext() {
        let path = output_path_for(
            Path::new("/tmp/docsy-nonexistent-dir-xyz/合同.md"),
            None,
            "docx",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("/tmp/docsy-nonexistent-dir-xyz/合同.docx")
        );

        let path = output_path_for(
            Path::new("/tmp/docsy-nonexistent-dir-xyz/合同.doc"),
            None,
            "md",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("/tmp/docsy-nonexistent-dir-xyz/合同.md")
        );
    }

    #[test]
    fn output_dir_must_exist() {
        assert!(output_path_for(
            Path::new("/tmp/a.md"),
            Some("/tmp/docsy-nonexistent-dir-xyz"),
            "docx",
        )
        .is_err());
    }

    #[test]
    fn parse_doc_engine_values() {
        assert_eq!(parse_doc_engine(None).unwrap(), DocEngine::Extract);
        assert_eq!(
            parse_doc_engine(Some("extract")).unwrap(),
            DocEngine::Extract
        );
        assert_eq!(parse_doc_engine(Some("word")).unwrap(), DocEngine::Word);
        assert!(parse_doc_engine(Some("libreoffice")).is_err());
    }

    #[test]
    fn convert_rejects_missing_input() {
        assert!(convert("/tmp/docsy-nonexistent-file-xyz.md", None, None, None, None).is_err());
    }

    /// 手动 QA：评估 AnyDoc 对 .doc 的提取质量。
    /// 用法：DOCSY_DOC_FIXTURE=/path/to/文件.doc cargo test -- --ignored --nocapture
    #[test]
    #[ignore = "manual QA: set DOCSY_DOC_FIXTURE to a .doc path"]
    fn doc_extract_manual_quality_probe() {
        let path = std::env::var("DOCSY_DOC_FIXTURE").expect("set DOCSY_DOC_FIXTURE");
        let result =
            docsy_anydoc::docsy::extract_office_markdown(Path::new(&path), "probe_assets").unwrap();
        println!(
            "--- format: {:?}, assets: {}, notes: {} ---\n{}",
            result.format, result.asset_count, result.note_count, result.markdown
        );
        assert!(!result.markdown.trim().is_empty());
    }

    #[test]
    fn convert_text_rejects_empty_and_bad_format() {
        assert!(convert_text("  ", "docx", None, None, None).is_err());
        assert!(convert_text("# 标题", "pdf", None, None, None).is_err());
    }

    #[test]
    fn convert_text_to_docx_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-paste-docx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let result = convert_text(
            "# 粘贴标题\n\n正文 **加粗**\n",
            "docx",
            Some(dir.to_str().unwrap()),
            Some("粘贴文档"),
            Some("professional"),
        )
        .unwrap();
        assert_eq!(result.format, "docx");
        assert!(result.output_path.ends_with("粘贴文档.docx"));
        assert!(result.input_size > 0 && result.output_size > 0);
        // 输出能被 docx-rs 读回且包含关键内容
        let bytes = std::fs::read(&result.output_path).unwrap();
        let md = crate::markdown::docx_to_md::docx_bytes_to_md(&bytes).unwrap();
        assert!(md.contains("# 粘贴标题"), "md:\n{md}");
        assert!(md.contains("**加粗**"), "md:\n{md}");
        // serde 字段为契约约定的 snake_case
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("output_path").is_some());
        assert_eq!(json["format"], "docx");
        assert!(json.get("output_size").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_text_to_excel_and_powerpoint_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-paste-office-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let markdown = "# 标题\n\n正文 **加粗**\n\n| 项目 | 金额 |\n| --- | ---: |\n| A | 100 |\n";

        for format in ["xlsx", "pptx"] {
            let result = convert_text(markdown, format, Some(dir.to_str().unwrap()), None, None)
                .unwrap_or_else(|error| panic!("{format} 生成失败: {error}"));
            assert_eq!(result.format, format);
            assert!(result.output_size > 0, "{format} 输出为空");
            office_oxide::Document::open(&result.output_path)
                .unwrap_or_else(|error| panic!("{format} 输出包无法读回: {error}"));

            let back = convert(&result.output_path, None, None, None, None)
                .unwrap_or_else(|error| panic!("{format} → Markdown 失败: {error}"));
            let back_markdown = std::fs::read_to_string(back.output_path).unwrap();
            assert!(
                !back_markdown.trim().is_empty(),
                "{format} → Markdown 结果为空"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_text_to_html_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-paste-html-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // 不指定文件名：回退到 文档-<时间戳>
        let result = convert_text(
            "# 标题\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n",
            "html",
            Some(dir.to_str().unwrap()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(result.format, "html");
        assert!(result.output_path.ends_with(".html"));
        let html = std::fs::read_to_string(&result.output_path).unwrap();
        assert!(html.contains("<meta charset=\"utf-8\">"), "html:\n{html}");
        assert!(html.contains("<h1>标题</h1>"), "html:\n{html}");
        assert!(html.contains("<table>"), "html:\n{html}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_md_file_to_html_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-md-html-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("网页.md");
        std::fs::write(&input, "# 标题\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n").unwrap();

        let result = convert(input.to_str().unwrap(), None, None, Some("html"), None).unwrap();
        assert_eq!(result.direction, Direction::MdToHtml);
        assert_eq!(result.output_format, "html");
        assert!(result.output_path.ends_with("网页.html"));
        let html = std::fs::read_to_string(&result.output_path).unwrap();
        assert!(html.contains("<h1>标题</h1>"), "html:\n{html}");
        assert!(html.contains("<table>"), "html:\n{html}");
        // serde 输出为契约约定的 snake_case
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["direction"], "md_to_html");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_md_file_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-md-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("测试.md");
        std::fs::write(&input, "# 标题\n\n正文 **加粗**\n").unwrap();

        let result = convert(input.to_str().unwrap(), None, None, None, None).unwrap();
        assert_eq!(result.direction, Direction::MdToDocx);
        assert!(result.output_path.ends_with("测试.docx"));
        assert!(result.input_size > 0 && result.output_size > 0);
        // serde 输出必须是契约约定的 snake_case 字段
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["direction"], "md_to_docx");
        assert!(json.get("output_path").is_some());
        assert!(json.get("input_size").is_some());
        assert!(json.get("output_size").is_some());

        // 生成的 docx 再转回 md
        let back = convert(&result.output_path, None, None, None, None).unwrap();
        assert_eq!(back.direction, Direction::OfficeToMd);
        let md = std::fs::read_to_string(&back.output_path).unwrap();
        assert!(md.contains("# 标题"), "md:\n{md}");
        assert!(md.contains("**加粗**"), "md:\n{md}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
