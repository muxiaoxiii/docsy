use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{annotations, header_footer, qpdf, same_path};

pub fn apply_rules_cancellable(
    args: &Value,
    token: &tokio_util::sync::CancellationToken,
    progress: &dyn Fn(String),
) -> Result<Value> {
    apply_rules_inner(args, Some(token), progress)
}

fn apply_rules_inner(
    args: &Value,
    token: Option<&tokio_util::sync::CancellationToken>,
    progress: &dyn Fn(String),
) -> Result<Value> {
    if token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        anyhow::bail!("操作已取消");
    }
    let items = extract_job_items(args)?;
    let annotation_rule = extract_annotation_rule(args);
    let mut temp_paths = Vec::new();
    let mut original_input_by_temp = BTreeMap::new();
    let mut annotation_failed = Vec::new();
    let prepared_items = prepare_items_for_processing(
        items,
        &annotation_rule,
        &mut temp_paths,
        &mut original_input_by_temp,
        &mut annotation_failed,
        progress,
    );
    let merge = extract_merge(args);
    let batch_result = if prepared_items.is_empty() {
        json!({ "results": [], "failed": [] })
    } else {
        let batch_args = json!({ "items": prepared_items });
        // 批注删除后输入是临时文件，进度展示时还原为原始文件名
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
        let batch = if let Some(token) = token {
            header_footer::batch_overlay_cancellable(&batch_args, token, &overlay_progress)
        } else {
            header_footer::batch_overlay(&batch_args)
        };
        match batch {
            Ok(value) => value,
            Err(err) => {
                cleanup_temp_paths(temp_paths);
                return Err(err);
            }
        }
    };
    let mut results = restore_original_inputs(
        batch_result
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        &original_input_by_temp,
    );
    let mut failed = batch_result
        .get("failed")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    failed.extend(annotation_failed);
    if token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        cleanup_temp_paths(temp_paths);
        anyhow::bail!("操作已取消");
    }
    let mut optimize_summary = Value::Null;
    if extract_optimize_output(args) {
        optimize_summary = optimize_result_outputs(&mut results, token, progress);
        if token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
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

    Ok(json!({
        "session": args.get("session").cloned().unwrap_or(Value::Null),
        "results": results,
        "failed": failed,
        "merge": merge_result,
        "summary": {
            "total": results.len() + failed.len(),
            "success": results.len(),
            "failed": failed.len()
        },
        "optimize": optimize_summary
    }))
}

/// 读取导出优化开关：前端在 payload 根级传 `optimizeOutput: true`。
fn extract_optimize_output(args: &Value) -> bool {
    args.get("optimizeOutput")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// 对每个成功结果的输出文件就地无损优化（丢弃页树不可达对象）。
/// 优化失败不阻断导出，只在该结果的 warnings 里追加提示。
/// 返回 { count, inputSize, outputSize } 汇总，供前端展示体积收益。
fn optimize_result_outputs(
    results: &mut [Value],
    token: Option<&tokio_util::sync::CancellationToken>,
    progress: &dyn Fn(String),
) -> Value {
    let mut count = 0_u64;
    let mut input_size = 0_u64;
    let mut output_size = 0_u64;
    let total = results.len();
    for (index, item) in results.iter_mut().enumerate() {
        if token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
            break;
        }
        let Some(output_path) = item
            .get("outputPath")
            .and_then(Value::as_str)
            .map(ToString::to_string)
        else {
            continue;
        };
        progress(format!(
            "正在优化导出 {}/{total}:{}",
            index + 1,
            file_name_of(&output_path)
        ));
        match qpdf::optimize_in_place(&output_path) {
            Ok(result) if result.changed => {
                count += 1;
                input_size += result.input_size;
                output_size += result.output_size;
                item["optimize"] = json!({
                    "changed": true,
                    "inputSize": result.input_size,
                    "outputSize": result.output_size,
                });
            }
            Ok(_) => {}
            Err(err) => {
                let warnings = item
                    .get("warnings")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut warnings = warnings;
                warnings.push(json!(format!("体积优化失败，已保留未优化结果: {err}")));
                item["warnings"] = Value::Array(warnings);
            }
        }
    }
    json!({
        "count": count,
        "inputSize": input_size,
        "outputSize": output_size,
    })
}

#[derive(Debug, Clone, Default)]
struct MergeSpec {
    enabled: bool,
    output_path: String,
    output_mode: String,
}

#[derive(Debug, Clone, Default)]
struct AnnotationRule {
    remove: bool,
    kinds: Vec<String>,
}

fn extract_job_items(args: &Value) -> Result<Vec<Value>> {
    let items = args
        .get("items")
        .or_else(|| args.get("jobs"))
        .and_then(Value::as_array)
        .cloned()
        .context("缺少证据 PDF 处理任务 items")?;

    if items.is_empty() {
        anyhow::bail!("证据 PDF 处理任务为空");
    }

    Ok(items)
}

fn extract_merge(args: &Value) -> MergeSpec {
    let Some(merge) = args.get("merge") else {
        return MergeSpec::default();
    };
    MergeSpec {
        enabled: merge
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        output_path: merge
            .get("outputPath")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        output_mode: merge
            .get("outputMode")
            .or_else(|| {
                args.get("session")
                    .and_then(|session| session.get("outputRule"))
                    .and_then(|rule| rule.get("outputMode"))
            })
            .and_then(Value::as_str)
            .unwrap_or("files_and_merge")
            .to_string(),
    }
}

fn extract_annotation_rule(args: &Value) -> AnnotationRule {
    let rule = args
        .get("session")
        .and_then(|session| session.get("annotationRule"))
        .or_else(|| args.get("annotationRule"));
    let Some(rule) = rule else {
        return AnnotationRule::default();
    };
    AnnotationRule {
        remove: rule
            .get("removeAnnotations")
            .or_else(|| rule.get("remove"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        kinds: rule
            .get("kinds")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn prepare_items_for_processing(
    items: Vec<Value>,
    rule: &AnnotationRule,
    temp_paths: &mut Vec<PathBuf>,
    original_input_by_temp: &mut BTreeMap<String, String>,
    failed: &mut Vec<Value>,
    progress: &dyn Fn(String),
) -> Vec<Value> {
    if !rule.remove {
        return items;
    }

    let total = items.len();
    let mut prepared = Vec::new();
    for (index, mut item) in items.into_iter().enumerate() {
        let input = item
            .get("inputPath")
            .or_else(|| item.get("input"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if input.is_empty() {
            failed.push(json!({
                "path": "",
                "message": "缺少批注删除输入路径"
            }));
            continue;
        }

        progress(format!(
            "正在删除批注 {}/{total}:{}",
            index + 1,
            file_name_of(&input)
        ));
        match annotations::delete_annotations_to_temp(&input, &rule.kinds) {
            Ok(temp_path) => {
                let temp_path_string = temp_path.to_string_lossy().to_string();
                original_input_by_temp.insert(temp_path_string.clone(), input);
                item["inputPath"] = Value::String(temp_path_string);
                temp_paths.push(temp_path);
                prepared.push(item);
            }
            Err(err) => failed.push(json!({
                "path": input,
                "message": format!("删除批注失败: {err}")
            })),
        }
    }
    prepared
}

fn restore_original_inputs(
    results: Vec<Value>,
    original_input_by_temp: &BTreeMap<String, String>,
) -> Vec<Value> {
    results
        .into_iter()
        .map(|mut item| {
            let original = item
                .get("inputPath")
                .and_then(Value::as_str)
                .and_then(|input| original_input_by_temp.get(input))
                .cloned();
            if let Some(original) = original {
                item["inputPath"] = Value::String(original);
            }
            item
        })
        .collect()
}

fn apply_merge_if_requested(
    merge: &MergeSpec,
    items: &[Value],
    results: &[Value],
    failed: &[Value],
) -> Result<Value> {
    if !merge.enabled {
        return Ok(json!({ "enabled": false }));
    }
    if !failed.is_empty() {
        return Ok(json!({
            "enabled": true,
            "status": "skipped",
            "message": "存在处理失败的 PDF，已跳过合并"
        }));
    }
    if merge.output_path.trim().is_empty() {
        return Ok(json!({
            "enabled": true,
            "status": "skipped",
            "message": "未设置合并输出路径"
        }));
    }

    let inputs: Vec<String> = results
        .iter()
        .filter_map(|item| item.get("outputPath").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect();
    if inputs.is_empty() {
        return Ok(json!({
            "enabled": true,
            "status": "skipped",
            "message": "没有可合并的处理结果"
        }));
    }

    if let Some(parent) = Path::new(&merge.output_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).context("创建合并 PDF 输出目录失败")?;
        }
    }
    let output = qpdf::merge(&inputs, &merge.output_path)?;

    // Apply bookmarks to the merged PDF: collect all bookmarks from items,
    // adjusting page_index to be global in the merged PDF.
    let merge_bookmarks = collect_merge_bookmarks(items, results);
    let remove_existing = items
        .first()
        .and_then(|item| item.get("bookmarkRemoveExisting"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !merge_bookmarks.is_empty() || remove_existing {
        header_footer::apply_bookmarks(Path::new(&output), &merge_bookmarks, remove_existing)
            .context("合并 PDF 写入书签失败")?;
    }

    let removed_intermediates = if merge.output_mode == "merge_only" {
        remove_intermediate_outputs(results, &merge.output_path)
    } else {
        0
    };
    Ok(json!({
        "enabled": true,
        "status": "done",
        "outputPath": output,
        "outputMode": merge.output_mode,
        "removedIntermediates": removed_intermediates
    }))
}

/// Collect all bookmarks from items, adjusting page_index to global position
/// in the merged PDF. Each file's bookmarks get an offset equal to the sum of
/// page counts of all preceding files.
fn collect_merge_bookmarks(
    items: &[Value],
    results: &[Value],
) -> Vec<header_footer::BookmarkConfig> {
    let mut bookmarks = Vec::new();
    let mut page_offset: u32 = 0;

    for (item, result) in items.iter().zip(results.iter()) {
        let pages = result.get("pages").and_then(Value::as_u64).unwrap_or(0) as u32;

        // Collect bookmarks from the bookmarks array
        if let Some(bms) = item.get("bookmarks").and_then(Value::as_array) {
            for bm_value in bms {
                if let Ok(bm) =
                    serde_json::from_value::<header_footer::BookmarkConfig>(bm_value.clone())
                {
                    if bm.enabled && !bm.label.is_empty() {
                        let mut adjusted = bm;
                        adjusted.page_index += page_offset;
                        bookmarks.push(adjusted);
                    }
                }
            }
        }

        page_offset += pages;
    }

    bookmarks
}

fn remove_intermediate_outputs(results: &[Value], merge_output_path: &str) -> usize {
    let mut removed = 0_usize;
    for item in results {
        let Some(output_path) = item.get("outputPath").and_then(Value::as_str) else {
            continue;
        };
        if same_path(Path::new(output_path), Path::new(merge_output_path)) {
            continue;
        }
        if fs::remove_file(output_path).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn cleanup_temp_paths(paths: Vec<PathBuf>) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

/// 取路径末段作为进度展示的文件名，兼容 Windows/Unix 分隔符。
fn file_name_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
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
    fn extracts_optimize_output_flag() {
        assert!(extract_optimize_output(&json!({ "optimizeOutput": true })));
        assert!(!extract_optimize_output(&json!({ "optimizeOutput": false })));
        assert!(!extract_optimize_output(&json!({})));
    }

    #[test]
    fn extracts_items_from_business_payload() {
        let items = extract_job_items(&json!({
            "session": { "totalPages": 3 },
            "items": [{ "inputPath": "/tmp/a.pdf" }]
        }))
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["inputPath"], "/tmp/a.pdf");
    }

    #[test]
    fn rejects_empty_business_payload() {
        let err = extract_job_items(&json!({ "items": [] })).unwrap_err();
        assert!(err.to_string().contains("任务为空"));
    }

    #[test]
    fn skips_merge_when_item_failed() {
        let merge = MergeSpec {
            enabled: true,
            output_path: "/tmp/final.pdf".to_string(),
            output_mode: "files_and_merge".to_string(),
        };
        let value =
            apply_merge_if_requested(&merge, &[], &[], &[json!({ "path": "/tmp/a.pdf" })]).unwrap();
        assert_eq!(value["status"], "skipped");
    }

    #[test]
    fn extracts_annotation_rule_from_session() {
        let rule = extract_annotation_rule(&json!({
            "session": {
                "annotationRule": {
                    "removeAnnotations": true,
                    "kinds": ["Highlight", "Underline"]
                }
            }
        }));
        assert!(rule.remove);
        assert_eq!(rule.kinds, vec!["Highlight", "Underline"]);
    }

    #[test]
    fn extracts_merge_only_output_mode() {
        let merge = extract_merge(&json!({
            "merge": {
                "enabled": true,
                "outputPath": "/tmp/final.pdf",
                "outputMode": "merge_only"
            }
        }));
        assert!(merge.enabled);
        assert_eq!(merge.output_mode, "merge_only");
    }
}
