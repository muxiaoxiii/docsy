use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitMergedArgs {
    #[serde(alias = "input")]
    input_path: String,
    output_dir: String,
    items: Vec<SplitRange>,
    #[serde(default)]
    cleanup: SplitCleanup,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitCleanup {
    #[serde(default)]
    header_enabled: bool,
    #[serde(default)]
    footer_enabled: bool,
    #[serde(default = "default_header_height_mm")]
    header_height_mm: f32,
    #[serde(default = "default_footer_height_mm")]
    footer_height_mm: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitRange {
    name: String,
    page_start: u32,
    page_end: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitMergedResult {
    total_pages: u32,
    warnings: Vec<String>,
    outputs: Vec<SplitOutput>,
    failed: Vec<SplitFailure>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitOutput {
    name: String,
    page_start: u32,
    page_end: u32,
    output_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitFailure {
    name: String,
    page_start: u32,
    page_end: u32,
    message: String,
}

pub fn split_merged(args: &serde_json::Value) -> Result<SplitMergedResult> {
    let args: SplitMergedArgs =
        serde_json::from_value(args.clone()).context("解析合并 PDF 拆分参数失败")?;
    if args.items.is_empty() {
        anyhow::bail!("缺少拆分页段");
    }
    if !Path::new(&args.input_path).exists() {
        anyhow::bail!("合并 PDF 不存在: {}", args.input_path);
    }
    std::fs::create_dir_all(&args.output_dir).context("创建拆分输出目录失败")?;

    let total_pages = super::qpdf::page_count(&args.input_path)?;
    let warnings = validate_split_layout(&args.items, total_pages);
    let mut outputs = Vec::new();
    let mut failed = Vec::new();

    for item in args.items {
        match validate_range(&item, total_pages)
            .and_then(|_| extract_range(&args.input_path, &args.output_dir, &item, &args.cleanup))
        {
            Ok(output_path) => outputs.push(SplitOutput {
                name: item.name,
                page_start: item.page_start,
                page_end: item.page_end,
                output_path,
            }),
            Err(err) => failed.push(SplitFailure {
                name: item.name,
                page_start: item.page_start,
                page_end: item.page_end,
                message: err.to_string(),
            }),
        }
    }

    Ok(SplitMergedResult {
        total_pages,
        warnings,
        outputs,
        failed,
    })
}

fn validate_range(item: &SplitRange, total_pages: u32) -> Result<()> {
    if item.page_start == 0 || item.page_end == 0 {
        anyhow::bail!("页码必须从 1 开始");
    }
    if item.page_start > item.page_end {
        anyhow::bail!("起始页不能大于结束页");
    }
    if item.page_end > total_pages {
        anyhow::bail!("结束页超过 PDF 总页数 {total_pages}");
    }
    Ok(())
}

fn validate_split_layout(items: &[SplitRange], total_pages: u32) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut ranges: Vec<&SplitRange> = items.iter().collect();
    ranges.sort_by_key(|item| (item.page_start, item.page_end));

    let mut cursor = 1_u32;
    for item in ranges {
        if item.page_start == 0 || item.page_end == 0 || item.page_start > item.page_end {
            continue;
        }
        if item.page_start > cursor {
            warnings.push(format!(
                "第 {cursor}-{} 页未包含在任何拆分页段中",
                item.page_start.saturating_sub(1)
            ));
        } else if item.page_start < cursor {
            warnings.push(format!("页段「{}」与前面的页段存在重叠", item.name.trim()));
        }
        cursor = cursor.max(item.page_end.saturating_add(1));
    }

    if cursor <= total_pages {
        warnings.push(format!(
            "第 {cursor}-{total_pages} 页未包含在任何拆分页段中"
        ));
    }
    warnings
}

fn extract_range(
    input_path: &str,
    output_dir: &str,
    item: &SplitRange,
    cleanup: &SplitCleanup,
) -> Result<String> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output_path = unique_output_path(output_dir, &safe_file_stem(&item.name));
    let qpdf_output_path = if cleanup.header_enabled || cleanup.footer_enabled {
        temp_named_path("docsy_split_range", "pdf")
    } else {
        output_path.clone()
    };
    let range = format!("{}-{}", item.page_start, item.page_end);
    let status = std::process::Command::new(&bin)
        .arg("--empty")
        .arg("--pages")
        .arg(input_path)
        .arg(range)
        .arg("--")
        .arg(&qpdf_output_path)
        .status()
        .context("执行 qpdf 页段拆分失败")?;

    if !status.success() {
        anyhow::bail!("qpdf 页段拆分失败");
    }
    if cleanup.header_enabled || cleanup.footer_enabled {
        let cleanup_result = super::header_footer::overlay_text(&serde_json::json!({
            "inputPath": qpdf_output_path.to_string_lossy(),
            "outputPath": output_path.to_string_lossy(),
            "cleanup": {
                "headerEnabled": cleanup.header_enabled,
                "footerEnabled": cleanup.footer_enabled,
                "headerHeightMm": cleanup.header_height_mm,
                "footerHeightMm": cleanup.footer_height_mm
            }
        }));
        let _ = std::fs::remove_file(&qpdf_output_path);
        cleanup_result?;
    }
    Ok(output_path.to_string_lossy().to_string())
}

fn default_header_height_mm() -> f32 {
    18.0
}

fn default_footer_height_mm() -> f32 {
    18.0
}

fn safe_file_stem(name: &str) -> String {
    let mut value = name
        .trim()
        .trim_end_matches(".pdf")
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();
    if value.is_empty() {
        value = "split".to_string();
    }
    value
}

fn unique_output_path(output_dir: &str, stem: &str) -> PathBuf {
    let dir = Path::new(output_dir);
    let mut path = dir.join(format!("{stem}.pdf"));
    let mut index = 1;
    while path.exists() {
        path = dir.join(format!("{stem}-{index}.pdf"));
        index += 1;
    }
    path
}

fn temp_named_path(prefix: &str, extension: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_page_ranges() {
        assert!(validate_range(
            &SplitRange {
                name: "a".to_string(),
                page_start: 1,
                page_end: 3,
            },
            5,
        )
        .is_ok());
        assert!(validate_range(
            &SplitRange {
                name: "a".to_string(),
                page_start: 4,
                page_end: 3,
            },
            5,
        )
        .is_err());
    }

    #[test]
    fn sanitizes_split_file_names() {
        assert_eq!(safe_file_stem("证据/1:合同.pdf"), "证据_1_合同");
        assert_eq!(safe_file_stem(""), "split");
    }

    #[test]
    fn split_cleanup_defaults_are_conservative() {
        let cleanup = SplitCleanup::default();
        assert!(!cleanup.header_enabled);
        assert!(!cleanup.footer_enabled);
    }

    #[test]
    fn warns_for_split_gaps_and_overlaps() {
        let warnings = validate_split_layout(
            &[
                SplitRange {
                    name: "a".to_string(),
                    page_start: 1,
                    page_end: 3,
                },
                SplitRange {
                    name: "b".to_string(),
                    page_start: 3,
                    page_end: 5,
                },
                SplitRange {
                    name: "c".to_string(),
                    page_start: 7,
                    page_end: 8,
                },
            ],
            10,
        );
        assert_eq!(warnings.len(), 3);
        assert!(warnings[0].contains("重叠"));
        assert!(warnings[1].contains("第 6-6 页"));
        assert!(warnings[2].contains("第 9-10 页"));
    }

    #[test]
    fn builds_unique_output_path_candidate() {
        let path = unique_output_path("/tmp", "evidence");
        assert_eq!(path, Path::new("/tmp").join("evidence.pdf"));
    }
}
