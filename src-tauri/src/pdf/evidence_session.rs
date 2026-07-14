use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{annotations, header_footer, qpdf};

pub fn apply_rules(args: &Value) -> Result<Value> {
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
    );
    let merge = extract_merge(args);
    let batch_result = if prepared_items.is_empty() {
        json!({ "results": [], "failed": [] })
    } else {
        match header_footer::batch_overlay(&json!({ "items": prepared_items })) {
            Ok(value) => value,
            Err(err) => {
                cleanup_temp_paths(temp_paths);
                return Err(err);
            }
        }
    };
    let results = restore_original_inputs(
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
    let merge_result = match apply_merge_if_requested(&merge, &results, &failed) {
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
        }
    }))
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
) -> Vec<Value> {
    if !rule.remove {
        return items;
    }

    let mut prepared = Vec::new();
    for mut item in items {
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

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn cleanup_temp_paths(paths: Vec<PathBuf>) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
            apply_merge_if_requested(&merge, &[], &[json!({ "path": "/tmp/a.pdf" })]).unwrap();
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
