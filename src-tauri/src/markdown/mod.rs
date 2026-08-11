//! Markdown ↔ docx 互转。
//!
//! - `.md` / `.markdown` → `.docx`（pulldown-cmark 解析，docx-rs 写入）
//! - `.docx` → `.md`（docx-rs reader 读取）
//! - `.doc` → 两条引擎，由前端让用户选择：
//!   - `word`：本机 Word/WPS 自动化另存临时 `.docx`（高保真），失败报错提示手动另存；
//!   - `extract`（默认）：office_oxide 转临时 `.docx`，仅纯文本（表格/图片/样式丢失，
//!     结果带 warning，前端转换前需用户确认）。

mod docx_to_md;
mod md_to_docx;

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::util::fs::{unique_output_path, TempPathGuard};

/// 转换方向，serde 输出为 snake_case（`md_to_docx` / `docx_to_md`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    MdToDocx,
    DocxToMd,
}

#[derive(Debug, Serialize)]
pub struct ConvertResult {
    pub output_path: String,
    pub direction: Direction,
    pub input_size: u64,
    pub output_size: u64,
    /// 降级转换时的用户提示（如 .doc 走了 office_oxide 纯文本兜底）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// .doc 转换引擎：`extract` = office_oxide 纯文本（有损）；`word` = 本机
/// Word/WPS 自动化另存 docx（高保真，需安装了 Word 或 WPS）。
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
fn detect_direction(input: &Path) -> Result<Direction> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => Ok(Direction::MdToDocx),
        "docx" | "doc" => Ok(Direction::DocxToMd),
        _ => anyhow::bail!("不支持的文件类型: {}", input.display()),
    }
}

/// 计算输出路径：默认放输入同目录，文件名同 stem 换后缀，撞名时自动加序号。
fn output_path_for(input: &Path, output_dir: Option<&str>, direction: Direction) -> Result<PathBuf> {
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
    let ext = match direction {
        Direction::MdToDocx => "docx",
        Direction::DocxToMd => "md",
    };
    Ok(unique_output_path(&dir, stem, ext))
}

/// office_oxide 的 doc→IR 只走 plain_text（表格/图片/样式全丢，
/// 标题靠全大写启发式猜测，对中文会误判），因此 .doc 转换始终带告警，
/// 由前端在转换前向用户确认。
const DOC_LOSSY_WARNING: &str =
    "旧版 .doc 转换仅保留纯文本，表格、图片和样式已丢失；建议先用 Word/WPS 另存为 .docx 再转换。";

/// 旧版 .doc → 临时 .docx（office_oxide），始终附带格式损失告警。
/// 临时文件由 TempPathGuard 负责清理。
fn convert_doc_to_temp_docx(input: &Path) -> Result<(TempPathGuard, Option<String>)> {
    let temp_path = crate::util::fs::temp_named_path("docsy-md2docx", "docx");
    let doc = office_oxide::Document::open(input.display().to_string())
        .with_context(|| format!("无法读取旧版 .doc 文件: {}", input.display()))?;
    doc.save_as(temp_path.display().to_string())
        .with_context(|| format!("转换 .doc → .docx 失败: {}", input.display()))?;
    Ok((
        TempPathGuard::new(temp_path),
        Some(DOC_LOSSY_WARNING.to_string()),
    ))
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
    let result = crate::external::command_output_with_timeout(&mut cmd, std::time::Duration::from_secs(120))
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
    let result = crate::external::command_output_with_timeout(&mut command, std::time::Duration::from_secs(120))
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

pub fn convert(input: &str, output_dir: Option<&str>, doc_engine: Option<&str>) -> Result<ConvertResult> {
    let input_path = PathBuf::from(input);
    if !input_path.is_file() {
        anyhow::bail!("输入文件不存在: {input}");
    }
    let direction = detect_direction(&input_path)?;
    let doc_engine = parse_doc_engine(doc_engine)?;
    let output_path = output_path_for(&input_path, output_dir, direction)?;
    let input_size = std::fs::metadata(&input_path)
        .with_context(|| format!("无法读取输入文件信息: {input}"))?
        .len();
    let mut warning: Option<String> = None;

    match direction {
        Direction::MdToDocx => {
            md_to_docx::convert(&input_path, &output_path)?;
        }
        Direction::DocxToMd => {
            let ext = input_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if ext == "doc" {
                // 先转临时 docx，守卫在作用域结束时自动删除
                let (guard, doc_warning) = match doc_engine {
                    DocEngine::Word => (convert_doc_to_temp_docx_with_word(&input_path)?, None),
                    DocEngine::Extract => convert_doc_to_temp_docx(&input_path)?,
                };
                warning = doc_warning;
                docx_to_md::convert(guard.path(), &output_path)?;
            } else {
                docx_to_md::convert(&input_path, &output_path)?;
            }
        }
    }

    let output_size = std::fs::metadata(&output_path)
        .with_context(|| format!("输出文件生成失败: {}", output_path.display()))?
        .len();
    Ok(ConvertResult {
        output_path: output_path.display().to_string(),
        direction,
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

/// 粘贴 Markdown 文本直接转换为 docx / html 文件。
pub fn convert_text(
    text: &str,
    format: &str,
    output_dir: Option<&str>,
    file_stem: Option<&str>,
) -> Result<ConvertTextResult> {
    if text.trim().is_empty() {
        anyhow::bail!("粘贴内容为空，无法转换");
    }
    let ext = match format {
        "docx" | "html" => format,
        other => anyhow::bail!("不支持的输出格式: {other}（仅支持 docx / html）"),
    };
    let dir = text_output_dir(output_dir)?;
    let stem = text_file_stem(file_stem);
    let output_path = unique_output_path(&dir, &stem, ext);

    match ext {
        "docx" => {
            // 复用 md → docx 管线：先落临时 .md，转换后由守卫自动清理
            let temp_path = crate::util::fs::temp_named_path("docsy-paste-md", "md");
            std::fs::write(&temp_path, text).context("写入临时 Markdown 文件失败")?;
            let guard = TempPathGuard::new(temp_path);
            md_to_docx::convert(guard.path(), &output_path)?;
        }
        "html" => {
            let options = pulldown_cmark::Options::ENABLE_TABLES
                | pulldown_cmark::Options::ENABLE_STRIKETHROUGH
                | pulldown_cmark::Options::ENABLE_TASKLISTS;
            let parser = pulldown_cmark::Parser::new_ext(text, options);
            let mut fragment = String::new();
            pulldown_cmark::html::push_html(&mut fragment, parser);
            std::fs::write(&output_path, wrap_html_fragment(&stem, &fragment))
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
            Direction::DocxToMd
        );
        assert_eq!(
            detect_direction(Path::new("/tmp/a.doc")).unwrap(),
            Direction::DocxToMd
        );
        assert!(detect_direction(Path::new("/tmp/a.txt")).is_err());
        assert!(detect_direction(Path::new("/tmp/noext")).is_err());
    }

    #[test]
    fn output_path_uses_input_dir_and_swapped_ext() {
        let path = output_path_for(
            Path::new("/tmp/docsy-nonexistent-dir-xyz/合同.md"),
            None,
            Direction::MdToDocx,
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("/tmp/docsy-nonexistent-dir-xyz/合同.docx")
        );

        let path = output_path_for(
            Path::new("/tmp/docsy-nonexistent-dir-xyz/合同.doc"),
            None,
            Direction::DocxToMd,
        )
        .unwrap();
        assert_eq!(path, PathBuf::from("/tmp/docsy-nonexistent-dir-xyz/合同.md"));
    }

    #[test]
    fn output_dir_must_exist() {
        assert!(output_path_for(
            Path::new("/tmp/a.md"),
            Some("/tmp/docsy-nonexistent-dir-xyz"),
            Direction::MdToDocx,
        )
        .is_err());
    }

    #[test]
    fn parse_doc_engine_values() {
        assert_eq!(parse_doc_engine(None).unwrap(), DocEngine::Extract);
        assert_eq!(parse_doc_engine(Some("extract")).unwrap(), DocEngine::Extract);
        assert_eq!(parse_doc_engine(Some("word")).unwrap(), DocEngine::Word);
        assert!(parse_doc_engine(Some("libreoffice")).is_err());
    }

    #[test]
    fn convert_rejects_missing_input() {
        assert!(convert("/tmp/docsy-nonexistent-file-xyz.md", None, None).is_err());
    }

    #[test]
    fn convert_text_rejects_empty_and_bad_format() {
        assert!(convert_text("  ", "docx", None, None).is_err());
        assert!(convert_text("# 标题", "pdf", None, None).is_err());
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
    fn convert_text_to_html_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-paste-html-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // 不指定文件名：回退到 文档-<时间戳>
        let result = convert_text(
            "# 标题\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n",
            "html",
            Some(dir.to_str().unwrap()),
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
    fn convert_md_file_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-md-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("测试.md");
        std::fs::write(&input, "# 标题\n\n正文 **加粗**\n").unwrap();

        let result = convert(input.to_str().unwrap(), None, None).unwrap();
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
        let back = convert(&result.output_path, None, None).unwrap();
        assert_eq!(back.direction, Direction::DocxToMd);
        let md = std::fs::read_to_string(&back.output_path).unwrap();
        assert!(md.contains("# 标题"), "md:\n{md}");
        assert!(md.contains("**加粗**"), "md:\n{md}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
