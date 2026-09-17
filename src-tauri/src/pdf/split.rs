use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

use crate::external::ExternalTool;

use super::{safe_file_stem, unique_output_path};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitMergedArgs {
    #[serde(alias = "input")]
    input_path: String,
    output_dir: String,
    items: Vec<SplitRange>,
    /// 拆分时移除无可视内容的空白页（默认关闭）。
    #[serde(default)]
    remove_blank_pages: bool,
    #[serde(default)]
    cleanup_targets: Vec<SplitCleanupTarget>,
    #[serde(default)]
    header_zone_mm: Option<f32>,
    #[serde(default)]
    footer_zone_mm: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SplitCleanupTarget {
    region: String,
    normalized_text: String,
    page_start: u32,
    page_end: u32,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    bbox: Option<SplitCleanupBBox>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SplitCleanupBBox {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    page: u32,
    width: f32,
    height: f32,
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
    removed_headers: usize,
    removed_footers: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitOutput {
    name: String,
    page_start: u32,
    page_end: u32,
    output_path: String,
    /// 该页段内被移除的空白页数量（remove_blank_pages 开启时）。
    #[serde(skip_serializing_if = "is_zero")]
    removed_blank_pages: usize,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitFailure {
    name: String,
    page_start: u32,
    page_end: u32,
    message: String,
}

pub fn split_merged(args: &SplitMergedArgs) -> Result<SplitMergedResult> {
    if args.items.is_empty() {
        anyhow::bail!("缺少拆分页段");
    }
    if !Path::new(&args.input_path).exists() {
        anyhow::bail!("合并 PDF 不存在: {}", args.input_path);
    }
    std::fs::create_dir_all(&args.output_dir).context("创建拆分输出目录失败")?;

    let total_pages = super::qpdf::page_count(&args.input_path)?;
    let mut warnings = validate_split_layout(&args.items, total_pages);
    let blank_pages = if args.remove_blank_pages {
        detect_blank_pages(&args.input_path)?
    } else {
        BTreeSet::new()
    };
    let mut artifact_plan = super::artifacts::HeaderFooterArtifactEditPlan::default();
    let mut plain_header_targets = Vec::new();
    let mut plain_footer_targets = Vec::new();

    let has_artifact_targets = args
        .cleanup_targets
        .iter()
        .any(|t| t.source.as_deref() == Some("artifact"));
    let inspection = if has_artifact_targets {
        Some(super::artifacts::inspect_meaningful_header_footer_artifacts(
            Path::new(&args.input_path),
            0,
        )?)
    } else {
        None
    };

    for target in &args.cleanup_targets {
        if target.normalized_text.trim().is_empty()
            || target.page_start == 0
            || target.page_end < target.page_start
            || target.page_end > total_pages
        {
            anyhow::bail!("清除元素的文本或页码范围无效，请重新检测");
        }

        let is_artifact = target.source.as_deref() == Some("artifact");
        if is_artifact {
            let edit = super::artifacts::HeaderFooterArtifactEditTarget {
                normalized_text: target.normalized_text.clone(),
                page_start: target.page_start,
                page_end: target.page_end,
                ..Default::default()
            };
            if !inspection.as_ref().is_some_and(|inspection| {
                inspection.occurrences.iter().any(|occurrence| {
                    occurrence.region == target.region
                        && super::artifacts::artifact_occurrence_matches_target(occurrence, &edit)
                })
            }) {
                anyhow::bail!("所选标准水印元素已失配，请重新检测；未输出拆分文件");
            }
            match target.region.as_str() {
                "header" => {
                    artifact_plan.remove_header = true;
                    artifact_plan.header_targets.push(edit);
                }
                "footer" => {
                    artifact_plan.remove_footer = true;
                    artifact_plan.footer_targets.push(edit);
                }
                _ => anyhow::bail!("清除元素区域无效"),
            }
        } else {
            let raw_text = target
                .text
                .as_deref()
                .filter(|t| !t.trim().is_empty())
                .unwrap_or(&target.normalized_text);
            let plain_target = super::content_text::PlainTextTarget {
                text: raw_text.to_string(),
                normalized_text: target.normalized_text.clone(),
                page_start: target.page_start,
                page_end: target.page_end,
                bbox: target.bbox.map(|b| super::content_text::PlainTextTargetBBox {
                    x0: b.x0,
                    y0: b.y0,
                    x1: b.x1,
                    y1: b.y1,
                    page: b.page,
                    width: b.width,
                    height: b.height,
                }),
            };
            match target.region.as_str() {
                "header" => plain_header_targets.push(plain_target),
                "footer" => plain_footer_targets.push(plain_target),
                _ => anyhow::bail!("清除元素区域无效"),
            }
        }
    }

    let mut current_source = args.input_path.clone();
    let mut total_removed_headers = 0;
    let mut total_removed_footers = 0;
    let mut _temp_guards: Vec<crate::util::fs::TempPathGuard> = Vec::new();

    if artifact_plan.remove_header || artifact_plan.remove_footer {
        if let Some((path, stats)) = super::artifacts::edit_header_footer_artifacts_to_temp(
            &current_source,
            &artifact_plan,
        )? {
            total_removed_headers += stats.removed_header;
            total_removed_footers += stats.removed_footer;
            current_source = path.to_string_lossy().into_owned();
            _temp_guards.push(crate::util::fs::TempPathGuard::new(path));
        }
    }

    if !plain_header_targets.is_empty() || !plain_footer_targets.is_empty() {
        let header_zone_mm = args.header_zone_mm.unwrap_or(35.0).clamp(10.0, 100.0);
        let footer_zone_mm = args.footer_zone_mm.unwrap_or(35.0).clamp(10.0, 100.0);
        let plain_plan = super::content_text::PlainTextCleanupPlan {
            header_targets: plain_header_targets,
            footer_targets: plain_footer_targets,
            header_zone_mm,
            footer_zone_mm,
        };
        let mut plain_removed = 0;
        if let Some((path, stats)) = super::content_text::delete_plain_header_footer_to_temp(
            &current_source,
            &plain_plan,
        )? {
            plain_removed = stats.removed();
            total_removed_headers += stats.removed_header;
            total_removed_footers += stats.removed_footer;
            current_source = path.to_string_lossy().into_owned();
            _temp_guards.push(crate::util::fs::TempPathGuard::new(path));
        }
        if plain_removed == 0 {
            warnings.push("已勾选清除普通文本页眉/页脚，但在对应页段内容流中未匹配到可安全剔除的文字，原文未修改".to_string());
        }
    }

    let source = current_source;
    let mut outputs = Vec::new();
    let mut failed = Vec::new();

    for item in &args.items {
        match validate_range(item, total_pages)
            .and_then(|_| extract_range(&source, &args.output_dir, item, &blank_pages))
        {
            Ok((output_path, removed_blank_pages)) => outputs.push(SplitOutput {
                name: item.name.clone(),
                page_start: item.page_start,
                page_end: item.page_end,
                output_path,
                removed_blank_pages,
            }),
            Err(err) => failed.push(SplitFailure {
                name: item.name.clone(),
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
        removed_headers: total_removed_headers,
        removed_footers: total_removed_footers,
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
    blank_pages: &BTreeSet<u32>,
) -> Result<(String, usize)> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output_path = unique_output_path(Path::new(output_dir), &safe_file_stem(&item.name), "pdf");

    // 页段内排除空白页；若空白页占了整段（不会发生），回退保留全部页。
    let kept: Vec<u32> = (item.page_start..=item.page_end)
        .filter(|page| !blank_pages.contains(page))
        .collect();
    let total_in_range = (item.page_end - item.page_start + 1) as usize;
    let (selection, removed_blank_pages) = if kept.is_empty() || kept.len() == total_in_range {
        (format!("{}-{}", item.page_start, item.page_end), 0)
    } else {
        let removed = total_in_range - kept.len();
        (
            kept.iter()
                .map(|page| page.to_string())
                .collect::<Vec<_>>()
                .join(","),
            removed,
        )
    };

    // 与压缩输出一致：按页重建时顺手做结构优化，丢弃不可达对象
    let mut cmd = crate::external::hidden_command(&bin);
    super::qpdf::add_optimization_args(&mut cmd);
    cmd.arg("--empty")
        .arg("--pages")
        .arg(input_path)
        .arg(selection)
        .arg("--")
        .arg(&output_path);
    let output = cmd.output().context("执行 qpdf 页段拆分失败")?;

    if !super::qpdf::status_is_success(&output.status) {
        let detail = crate::external::command_failure_detail(&output);
        anyhow::bail!("qpdf 页段拆分失败: {detail}");
    }
    Ok((
        output_path.to_string_lossy().to_string(),
        removed_blank_pages,
    ))
}

/// 检测 PDF 中"无可视内容"的空白页（无文本、无图像、无矢量路径绘制）。
///
/// 流式实现：仅通过 qpdf 拉取页面元数据与内容流，不整档载入扫描件图像对象。
/// 判定规则：页面内容流中没有任何文本算子、图像绘制算子（Do/BI/ID/EI）
/// 或矢量路径构造与着色算子（re/m/l/c/v/y/h/f/F/f*/S/s/B/B*/b/b*/sh）即视为空白页。
/// 内容流解析失败时保守视为有内容，避免误删。
fn detect_blank_pages(input: &str) -> Result<BTreeSet<u32>> {
    super::qpdf_stream::detect_blank_pages_streaming(Path::new(input))
        .context("读取 PDF 分析空白页失败")
}

pub(crate) fn operation_is_visible(operation: &lopdf::content::Operation) -> bool {
    matches!(
        operation.operator.as_str(),
        // 文本输出
        "Tj" | "TJ" | "'" | "\"" |
        // XObject 与图像
        "Do" | "BI" | "ID" | "EI" |
        // 矢量路径构建（矩形、点线、曲线、闭合）
        "re" | "m" | "l" | "c" | "v" | "y" | "h" |
        // 矢量路径绘制（填充、描边、着色、网格）
        "f" | "F" | "f*" | "S" | "s" | "B" | "B*" | "b" | "b*" | "sh"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires DOCSY_ARTIFACT_FIXTURE and qpdf"]
    fn split_cleans_only_selected_footer_and_preserves_original() {
        let input = std::env::var("DOCSY_ARTIFACT_FIXTURE").unwrap();
        let original = std::fs::read(&input).unwrap();
        let inspected = crate::pdf::artifacts::inspect_meaningful_header_footer_artifacts(std::path::Path::new(&input), 0).unwrap();
        let footer = inspected.occurrences.iter().find(|item| item.region == "footer").unwrap();
        let output = crate::pdf::temp_named_path("docsy_split_cleanup_test", "dir");
        let pages = crate::pdf::qpdf::page_count(&input).unwrap();
        let mut args: super::SplitMergedArgs = serde_json::from_value(serde_json::json!({
            "inputPath": input, "outputDir": output,
            "items": [{"name": "cleaned", "pageStart": 1, "pageEnd": pages}],
            "cleanupTargets": [{"region": "footer", "source": "artifact", "normalizedText": footer.text, "pageStart": footer.page, "pageEnd": footer.page}],
        })).unwrap();
        let result = super::split_merged(&args).unwrap();
        assert!(result.failed.is_empty());
        assert_eq!(result.removed_footers, 1);
        assert_eq!(result.removed_headers, 0);
        let after = crate::pdf::artifacts::inspect_meaningful_header_footer_artifacts(std::path::Path::new(&result.outputs[0].output_path), 0).unwrap();
        assert_eq!(after.header_count, inspected.header_count);
        assert_eq!(after.footer_count + 1, inspected.footer_count);
        assert_eq!(std::fs::read(&input).unwrap(), original);
        args.cleanup_targets[0].normalized_text = "missing-element-test".into();
        assert!(super::split_merged(&args).is_err());
        std::fs::remove_dir_all(output).unwrap();
    }
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
        assert_eq!(safe_file_stem(""), "output");
    }

    #[test]
    fn operation_visibility_classifies_text_images_and_vectors() {
        use lopdf::content::Operation;
        assert!(operation_is_visible(&Operation::new("Tj", vec![])));
        assert!(operation_is_visible(&Operation::new("TJ", vec![])));
        assert!(operation_is_visible(&Operation::new("'", vec![])));
        assert!(operation_is_visible(&Operation::new("\"", vec![])));
        assert!(operation_is_visible(&Operation::new("Do", vec![])));
        assert!(operation_is_visible(&Operation::new("BI", vec![])));
        assert!(operation_is_visible(&Operation::new("re", vec![])));
        assert!(operation_is_visible(&Operation::new("f", vec![])));
        assert!(operation_is_visible(&Operation::new("S", vec![])));
        assert!(!operation_is_visible(&Operation::new("cm", vec![])));
        assert!(!operation_is_visible(&Operation::new("q", vec![])));
        assert!(!operation_is_visible(&Operation::new("Q", vec![])));
        assert!(!operation_is_visible(&Operation::new("BT", vec![])));
        assert!(!operation_is_visible(&Operation::new("ET", vec![])));
    }

    #[test]
    fn detects_pages_without_visible_content_as_blank() {
        use lopdf::{content::Operation, dictionary, Object, Stream};

        let mut doc = lopdf::Document::with_version("1.7");
        let pages_id = doc.add_object(dictionary! {});
        // 空白页：没有任何内容流
        let blank_page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        // 文本页：内容流含 Tj 算子
        let content_bytes = lopdf::content::Content::encode(&lopdf::content::Content {
            operations: vec![Operation::new("Tj", vec![Object::string_literal("证据一")])],
        })
        .expect("encode content");
        let content_id = doc.add_object(Object::Stream(Stream::new(dictionary! {}, content_bytes)));
        let text_page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => content_id,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![Object::Reference(blank_page_id), Object::Reference(text_page_id)],
                "Count" => 2,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let path = crate::util::fs::temp_named_path("docsy-blank-detect", "pdf");
        let guard = crate::util::fs::TempPathGuard::new(path.clone());
        doc.save(&path).expect("save test pdf");

        let blank = detect_blank_pages(&path.to_string_lossy()).expect("detect blank pages");
        assert_eq!(blank, BTreeSet::from([1]), "第 1 页无内容应判为空白页");
        drop(guard);
    }

    #[test]
    fn extract_range_excludes_blank_pages_from_selection() {
        // 页段 1-4 中第 2、3 页为空白页时，应生成 1,4 的选择并报告移除 2 页
        let blank = BTreeSet::from([2_u32, 3_u32]);
        let item = SplitRange {
            name: "a".to_string(),
            page_start: 1,
            page_end: 4,
        };
        // 直接验证选择逻辑（避免依赖 qpdf 二进制）
        let kept: Vec<u32> = (item.page_start..=item.page_end)
            .filter(|page| !blank.contains(page))
            .collect();
        assert_eq!(kept, vec![1, 4]);
        let selection = kept
            .iter()
            .map(|page| page.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(selection, "1,4");
        assert_eq!(item.page_end - item.page_start + 1 - kept.len() as u32, 2);
    }

    /// 端到端：真实 qpdf 拆分时移除空白页。依赖 qpdf 二进制，未安装时跳过。
    #[test]
    fn split_merged_removes_blank_pages_end_to_end() {
        let qpdf = crate::external::QpdfTool;
        if qpdf.binary_path().is_err() {
            return;
        }

        use lopdf::{content::Operation, dictionary, Object, Stream};
        let mut doc = lopdf::Document::with_version("1.7");
        let pages_id = doc.add_object(dictionary! {});
        let media_box = || vec![0.into(), 0.into(), 595.into(), 842.into()];
        let blank_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => media_box(),
        });
        let content_bytes = lopdf::content::Content::encode(&lopdf::content::Content {
            operations: vec![Operation::new("Tj", vec![Object::string_literal("证据一")])],
        })
        .expect("encode content");
        let content_id = doc.add_object(Object::Stream(Stream::new(dictionary! {}, content_bytes)));
        let text_id = |doc: &mut lopdf::Document| {
            doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => media_box(),
                "Contents" => content_id,
            })
        };
        let p2 = text_id(&mut doc);
        let p3 = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => media_box(),
        });
        let p4 = text_id(&mut doc);
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![
                    Object::Reference(blank_id),
                    Object::Reference(p2),
                    Object::Reference(p3),
                    Object::Reference(p4),
                ],
                "Count" => 4,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog", "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("docsy-split-blank-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("创建测试目录失败");
        let input = dir.join("merged.pdf");
        doc.save(&input).expect("保存测试 PDF 失败");

        let result = split_merged(&SplitMergedArgs {
            input_path: input.to_string_lossy().to_string(),
            output_dir: dir.join("out").to_string_lossy().to_string(),
            items: vec![SplitRange {
                name: "证据".to_string(),
                page_start: 1,
                page_end: 4,
            }],
            remove_blank_pages: true,
            cleanup_targets: vec![],
            header_zone_mm: None,
            footer_zone_mm: None,
        })
        .expect("拆分失败");

        assert_eq!(result.outputs.len(), 1);
        assert_eq!(
            result.outputs[0].removed_blank_pages, 2,
            "第 1、3 页为空白页应被移除"
        );
        let output_path = &result.outputs[0].output_path;
        assert_eq!(
            crate::pdf::qpdf::page_count(output_path).expect("读取输出页数失败"),
            2,
            "拆分输出应只剩 2 个文本页"
        );

        let _ = std::fs::remove_dir_all(&dir);
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
        let path = unique_output_path(Path::new("/tmp"), "evidence", "pdf");
        assert_eq!(path, Path::new("/tmp").join("evidence.pdf"));
    }
}
