use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{annotations, header_footer, qpdf, same_path};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// 最终返回给前端的结构化结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRulesResult {
    pub session: Option<Value>,
    pub results: Vec<header_footer::HeaderFooterResult>,
    pub failed: Vec<header_footer::HeaderFooterFailure>,
    pub merge: MergeResult,
    pub summary: ApplyRulesSummary,
    pub optimize: OptimizeSummary,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRulesSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_intermediates: Option<usize>,
}

impl MergeResult {
    fn disabled() -> Self {
        Self {
            enabled: false,
            status: None,
            output_path: None,
            output_mode: None,
            message: None,
            removed_intermediates: None,
        }
    }
    fn skipped(message: impl Into<String>) -> Self {
        Self {
            enabled: true,
            status: Some("skipped".into()),
            output_path: None,
            output_mode: None,
            message: Some(message.into()),
            removed_intermediates: None,
        }
    }
    fn done(output: String, mode: String, removed: usize) -> Self {
        Self {
            enabled: true,
            status: Some("done".into()),
            output_path: Some(output),
            output_mode: Some(mode),
            message: None,
            removed_intermediates: Some(removed),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeSummary {
    pub count: u64,
    pub input_size: u64,
    pub output_size: u64,
}

#[derive(Debug, Clone, Default)]
struct MergeSpec {
    enabled: bool,
    output_path: String,
    output_mode: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationRule {
    #[serde(default, alias = "removeAnnotations", alias = "remove")]
    pub remove: bool,
    #[serde(default)]
    pub kinds: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public entry
// ---------------------------------------------------------------------------

pub fn apply_rules_cancellable(
    args: &crate::commands::pdf::ApplyEvidencePdfRulesArgs,
    token: &tokio_util::sync::CancellationToken,
    progress: &dyn Fn(String),
) -> Result<ApplyRulesResult> {
    if token.is_cancelled() {
        anyhow::bail!("操作已取消");
    }

    // Extract typed data from the args struct
    let items = extract_job_items_from_args(args)?;
    let annotation_rule = extract_annotation_rule_from_args(args);
    let merge = extract_merge_from_args(args);
    let optimize_output = args
        .extra
        .get("optimizeOutput")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut temp_paths = Vec::new();
    let mut original_input_by_temp = BTreeMap::new();
    let mut annotation_failed: Vec<header_footer::HeaderFooterFailure> = Vec::new();

    let prepared_items = prepare_items_for_processing(
        items,
        &annotation_rule,
        &mut temp_paths,
        &mut original_input_by_temp,
        &mut annotation_failed,
        progress,
    );

    let batch_result: header_footer::BatchHeaderFooterResult = if prepared_items.is_empty() {
        header_footer::BatchHeaderFooterResult::empty()
    } else {
        let overlay_progress = |index: usize, total: usize, input: &str| {
            let original = original_input_by_temp
                .get(input)
                .map(String::as_str)
                .unwrap_or(input);
            progress(format!(
                "正在处理 {index}/{total}:{}",
                file_name_of(original)
            ));
        };
        let batch =
            header_footer::batch_overlay_cancellable(&prepared_items, token, &overlay_progress);
        match batch {
            Ok(result) => result,
            Err(err) => {
                cleanup_temp_paths(temp_paths);
                return Err(err);
            }
        }
    };

    let mut results = restore_original_inputs(batch_result.results, &original_input_by_temp);
    let mut failed = batch_result.failed;
    failed.extend(annotation_failed);

    if batch_result.cancelled || token.is_cancelled() {
        cleanup_temp_paths(temp_paths);
        return Ok(ApplyRulesResult {
            session: args.session.clone(),
            summary: ApplyRulesSummary {
                total: results.len() + failed.len(),
                success: results.len(),
                failed: failed.len(),
            },
            results,
            failed,
            merge: MergeResult::skipped("操作已取消，未合并未处理的 PDF"),
            optimize: OptimizeSummary::default(),
            cancelled: true,
        });
    }

    let mut optimize_summary = OptimizeSummary::default();
    if optimize_output {
        optimize_summary = optimize_result_outputs(&mut results, Some(token), progress);
        if token.is_cancelled() {
            cleanup_temp_paths(temp_paths);
            anyhow::bail!("操作已取消");
        }
    }

    if merge.enabled {
        progress("正在合并 PDF 并写入书签".to_string());
    }
    let merge_result = match apply_merge_if_requested(&merge, &prepared_items, &results, &failed) {
        Ok(value) => value,
        Err(err) => {
            cleanup_temp_paths(temp_paths);
            return Err(err);
        }
    };
    cleanup_temp_paths(temp_paths);

    let session = args.session.clone();

    Ok(ApplyRulesResult {
        session,
        summary: ApplyRulesSummary {
            total: results.len() + failed.len(),
            success: results.len(),
            failed: failed.len(),
        },
        results,
        failed,
        merge: merge_result,
        optimize: optimize_summary,
        cancelled: false,
    })
}

// ---------------------------------------------------------------------------
// Extraction helpers (typed from args)
// ---------------------------------------------------------------------------

fn extract_job_items_from_args(
    args: &crate::commands::pdf::ApplyEvidencePdfRulesArgs,
) -> Result<Vec<header_footer::HeaderFooterJob>> {
    let raw_items = args
        .items
        .as_ref()
        .or(args.jobs.as_ref())
        .context("缺少证据 PDF 处理任务 items")?;
    if raw_items.is_empty() {
        anyhow::bail!("证据 PDF 处理任务为空");
    }
    Ok(raw_items.clone())
}

fn extract_annotation_rule_from_args(
    args: &crate::commands::pdf::ApplyEvidencePdfRulesArgs,
) -> AnnotationRule {
    // Try session.annotationRule first, then annotationRule
    let rule_value = args
        .session
        .as_ref()
        .and_then(|s| s.get("annotationRule"))
        .or(args.annotation_rule.as_ref());
    let Some(value) = rule_value else {
        return AnnotationRule::default();
    };
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn extract_merge_from_args(args: &crate::commands::pdf::ApplyEvidencePdfRulesArgs) -> MergeSpec {
    let Some(merge_value) = args.merge.as_ref() else {
        return MergeSpec::default();
    };
    MergeSpec {
        enabled: merge_value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        output_path: merge_value
            .get("outputPath")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        output_mode: merge_value
            .get("outputMode")
            .or_else(|| {
                args.session
                    .as_ref()
                    .and_then(|s| s.get("outputRule"))
                    .and_then(|r| r.get("outputMode"))
            })
            .and_then(Value::as_str)
            .unwrap_or("files_and_merge")
            .to_string(),
    }
}

// ---------------------------------------------------------------------------
// Processing pipeline
// ---------------------------------------------------------------------------

fn prepare_items_for_processing(
    items: Vec<header_footer::HeaderFooterJob>,
    rule: &AnnotationRule,
    temp_paths: &mut Vec<PathBuf>,
    original_input_by_temp: &mut BTreeMap<String, String>,
    failed: &mut Vec<header_footer::HeaderFooterFailure>,
    progress: &dyn Fn(String),
) -> Vec<header_footer::HeaderFooterJob> {
    if !rule.remove {
        return items;
    }

    let total = items.len();
    let mut prepared = Vec::new();
    for (index, mut job) in items.into_iter().enumerate() {
        if job.input_path.is_empty() {
            failed.push(header_footer::HeaderFooterFailure {
                path: String::new(),
                message: "缺少批注删除输入路径".to_string(),
            });
            continue;
        }

        progress(format!(
            "正在删除批注 {}/{total}:{}",
            index + 1,
            file_name_of(&job.input_path)
        ));
        match annotations::delete_annotations_to_temp(&job.input_path, &rule.kinds) {
            Ok(temp_path) => {
                let temp_path_string = temp_path.to_string_lossy().to_string();
                original_input_by_temp.insert(temp_path_string.clone(), job.input_path.clone());
                job.input_path = temp_path_string;
                temp_paths.push(temp_path);
                prepared.push(job);
            }
            Err(err) => failed.push(header_footer::HeaderFooterFailure {
                path: job.input_path,
                message: format!("删除批注失败: {err}"),
            }),
        }
    }
    prepared
}

fn restore_original_inputs(
    results: Vec<header_footer::HeaderFooterResult>,
    original_input_by_temp: &BTreeMap<String, String>,
) -> Vec<header_footer::HeaderFooterResult> {
    results
        .into_iter()
        .map(|mut result| {
            if let Some(original) = original_input_by_temp.get(&result.input_path) {
                result.input_path = original.clone();
            }
            result
        })
        .collect()
}

fn optimize_result_outputs(
    results: &mut [header_footer::HeaderFooterResult],
    token: Option<&tokio_util::sync::CancellationToken>,
    progress: &dyn Fn(String),
) -> OptimizeSummary {
    let mut summary = OptimizeSummary::default();
    let total = results.len();
    for (index, result) in results.iter_mut().enumerate() {
        if token.is_some_and(|t| t.is_cancelled()) {
            break;
        }
        if result.output_path.is_empty() {
            continue;
        }
        progress(format!(
            "正在优化导出 {}/{total}:{}",
            index + 1,
            file_name_of(&result.output_path)
        ));
        match qpdf::optimize_in_place(&result.output_path) {
            Ok(opt) if opt.changed => {
                summary.count += 1;
                summary.input_size += opt.input_size;
                summary.output_size += opt.output_size;
                result.warnings.push(format!(
                    "已优化体积: {} → {}",
                    human_size(opt.input_size),
                    human_size(opt.output_size)
                ));
            }
            Ok(_) => {}
            Err(err) => {
                result
                    .warnings
                    .push(format!("体积优化失败，已保留未优化结果: {err}"));
            }
        }
    }
    summary
}

// ---------------------------------------------------------------------------
// Merge
// ---------------------------------------------------------------------------

fn apply_merge_if_requested(
    merge: &MergeSpec,
    items: &[header_footer::HeaderFooterJob],
    results: &[header_footer::HeaderFooterResult],
    failed: &[header_footer::HeaderFooterFailure],
) -> Result<MergeResult> {
    if !merge.enabled {
        return Ok(MergeResult::disabled());
    }
    if !failed.is_empty() {
        return Ok(MergeResult::skipped("存在处理失败的 PDF，已跳过合并"));
    }
    if merge.output_path.trim().is_empty() {
        return Ok(MergeResult::skipped("未设置合并输出路径"));
    }

    let inputs: Vec<String> = results
        .iter()
        .map(|r| r.output_path.clone())
        .filter(|p| !p.is_empty())
        .collect();
    if inputs.is_empty() {
        return Ok(MergeResult::skipped("没有可合并的处理结果"));
    }

    if let Some(parent) = Path::new(&merge.output_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).context("创建合并 PDF 输出目录失败")?;
        }
    }
    let output = qpdf::merge(&inputs, &merge.output_path)?;

    let merge_bookmarks = collect_merge_bookmarks(items, results);
    let remove_existing = items
        .first()
        .map(|j| j.bookmark_remove_existing)
        .unwrap_or(false);
    if !merge_bookmarks.is_empty() || remove_existing {
        header_footer::apply_bookmarks(Path::new(&output), &merge_bookmarks, remove_existing)
            .context("合并 PDF 写入书签失败")?;
    }

    let removed = if merge.output_mode == "merge_only" {
        remove_intermediate_outputs(results, &merge.output_path)
    } else {
        0
    };

    Ok(MergeResult::done(
        output,
        merge.output_mode.clone(),
        removed,
    ))
}

fn collect_merge_bookmarks(
    items: &[header_footer::HeaderFooterJob],
    results: &[header_footer::HeaderFooterResult],
) -> Vec<header_footer::BookmarkConfig> {
    let mut bookmarks = Vec::new();
    let mut page_offset: u32 = 0;

    for (item, result) in items.iter().zip(results.iter()) {
        for bm in &item.bookmarks {
            if bm.enabled && !bm.label.is_empty() {
                let mut adjusted = bm.clone();
                adjusted.page_index += page_offset;
                bookmarks.push(adjusted);
            }
        }
        page_offset += result.pages;
    }

    bookmarks
}

fn remove_intermediate_outputs(
    results: &[header_footer::HeaderFooterResult],
    merge_output_path: &str,
) -> usize {
    let mut removed = 0_usize;
    for result in results {
        if result.output_path.is_empty() {
            continue;
        }
        if same_path(Path::new(&result.output_path), Path::new(merge_output_path)) {
            continue;
        }
        if fs::remove_file(&result.output_path).is_ok() {
            removed += 1;
        }
    }
    removed
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn cleanup_temp_paths(paths: Vec<PathBuf>) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn file_name_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn file_name_of_takes_last_path_segment() {
        assert_eq!(file_name_of("/tmp/dir/a.pdf"), "a.pdf");
        assert_eq!(file_name_of("C:\\dir\\b.pdf"), "b.pdf");
        assert_eq!(file_name_of("c.pdf"), "c.pdf");
    }

    #[test]
    fn annotation_rule_from_json() {
        let rule: AnnotationRule = serde_json::from_value(json!({
            "removeAnnotations": true,
            "kinds": ["Highlight", "Underline"]
        }))
        .unwrap();
        assert!(rule.remove);
        assert_eq!(rule.kinds, vec!["Highlight", "Underline"]);
    }

    #[test]
    fn merge_result_serializes_correctly() {
        let r = MergeResult::skipped("测试");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["enabled"], true);
        assert_eq!(v["status"], "skipped");
        assert_eq!(v["message"], "测试");
    }

    #[test]
    fn skips_merge_when_item_failed() {
        let merge = MergeSpec {
            enabled: true,
            output_path: "/tmp/final.pdf".to_string(),
            output_mode: "files_and_merge".to_string(),
        };
        let failed = vec![header_footer::HeaderFooterFailure {
            path: "/tmp/a.pdf".to_string(),
            message: "test".to_string(),
        }];
        let result = apply_merge_if_requested(&merge, &[], &[], &failed).unwrap();
        assert_eq!(result.enabled, true);
        assert_eq!(result.status.as_deref(), Some("skipped"));
    }

    #[test]
    fn extract_annotation_rule_from_session_field() {
        let args = crate::commands::pdf::ApplyEvidencePdfRulesArgs {
            items: None,
            jobs: None,
            merge: None,
            session: Some(json!({
                "annotationRule": {
                    "remove": true,
                    "kinds": ["Stamp"]
                }
            })),
            annotation_rule: None,
            extra: std::collections::HashMap::new(),
        };
        let rule = extract_annotation_rule_from_args(&args);
        assert!(rule.remove);
        assert_eq!(rule.kinds, vec!["Stamp"]);
    }

    #[test]
    fn extract_merge_from_args_basic() {
        let args = crate::commands::pdf::ApplyEvidencePdfRulesArgs {
            items: None,
            jobs: None,
            merge: Some(json!({
                "enabled": true,
                "outputPath": "/tmp/out.pdf",
                "outputMode": "merge_only"
            })),
            session: None,
            annotation_rule: None,
            extra: std::collections::HashMap::new(),
        };
        let merge = extract_merge_from_args(&args);
        assert!(merge.enabled);
        assert_eq!(merge.output_path, "/tmp/out.pdf");
        assert_eq!(merge.output_mode, "merge_only");
    }
}
