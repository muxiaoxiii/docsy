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
