//! Markdown ↔ docx 互转。
//!
//! - `.md` / `.markdown` → `.docx`（pulldown-cmark 解析，docx-rs 写入）
//! - `.docx` → `.md`（docx-rs reader 读取）
//! - `.doc` → 先用 office_oxide 转成临时 `.docx`，再转 `.md`

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

/// 旧版 .doc → 临时 .docx，临时文件由 TempPathGuard 负责清理。
fn convert_doc_to_temp_docx(input: &Path) -> Result<TempPathGuard> {
    let temp_path = crate::util::fs::temp_named_path("docsy-md2docx", "docx");
    let doc = office_oxide::Document::open(input.display().to_string())
        .with_context(|| format!("无法读取旧版 .doc 文件: {}", input.display()))?;
    doc.save_as(temp_path.display().to_string())
        .with_context(|| format!("转换 .doc → .docx 失败: {}", input.display()))?;
    Ok(TempPathGuard::new(temp_path))
}

pub fn convert(input: &str, output_dir: Option<&str>) -> Result<ConvertResult> {
    let input_path = PathBuf::from(input);
    if !input_path.is_file() {
        anyhow::bail!("输入文件不存在: {input}");
    }
    let direction = detect_direction(&input_path)?;
    let output_path = output_path_for(&input_path, output_dir, direction)?;
    let input_size = std::fs::metadata(&input_path)
        .with_context(|| format!("无法读取输入文件信息: {input}"))?
        .len();

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
                let guard = convert_doc_to_temp_docx(&input_path)?;
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
    fn convert_rejects_missing_input() {
        assert!(convert("/tmp/docsy-nonexistent-file-xyz.md", None).is_err());
    }

    #[test]
    fn convert_md_file_end_to_end() {
        let dir = std::env::temp_dir().join(format!("docsy-md-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("测试.md");
        std::fs::write(&input, "# 标题\n\n正文 **加粗**\n").unwrap();

        let result = convert(input.to_str().unwrap(), None).unwrap();
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
        let back = convert(&result.output_path, None).unwrap();
        assert_eq!(back.direction, Direction::DocxToMd);
        let md = std::fs::read_to_string(&back.output_path).unwrap();
        assert!(md.contains("# 标题"), "md:\n{md}");
        assert!(md.contains("**加粗**"), "md:\n{md}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
