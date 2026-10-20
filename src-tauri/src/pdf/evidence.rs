use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::sort_utils::natural_cmp;

use super::fnv1a_hash;
use super::safe_file_stem;
use crate::ConversionState;

use super::word_convert::convert_word_to_pdf;

const SUPPORTED_EXTS: &[&str] = &["pdf", "doc", "docx", "docm"];

#[derive(Debug, Clone, Copy)]
enum FileType {
    Pdf,
    Word,
}

impl FileType {
    fn from_ext(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "doc" | "docx" | "docm" => Some(Self::Word),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Word => "word",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OverlayConfig {
    header: Option<HeaderConfig>,
    footer: Option<FooterConfig>,
}

#[derive(Debug, Deserialize)]
struct HeaderConfig {
    enabled: bool,
    #[serde(default)]
    content: String,
    custom_text: Option<String>,
    start_number: Option<u32>,
    font_size: Option<f64>,
    y_offset: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct FooterConfig {
    enabled: bool,
    #[serde(default)]
    content: String,
    font_size: Option<f64>,
    y_offset: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct IdentityConfig {
    prefix: Option<String>,
    start_number: Option<u32>,
}

fn has_supported_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn collect_supported_files(
    dir: &Path,
    out: &mut Vec<(String, PathBuf, FileType, u64)>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            // Evidence folders are often assembled with Finder/Explorer aliases.
            // Never follow them: a link to an ancestor would recurse forever and
            // a link outside the selected folder should not be imported silently.
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with('_') || name.starts_with('.')) {
                continue;
            }
            collect_supported_files(&path, out)?;
            continue;
        }
        if !has_supported_ext(&path) {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if let Some(ft) = FileType::from_ext(&ext) {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            out.push((name, path, ft, size));
        }
    }
    Ok(())
}

pub fn scan_folder(root: &str) -> Result<serde_json::Value> {
    let root_path = Path::new(root);
    if !root_path.is_dir() {
        anyhow::bail!("路径不是有效目录: {}", root);
    }

    let mut groups: BTreeMap<String, Vec<(String, PathBuf, FileType, u64)>> = BTreeMap::new();

    for entry in fs::read_dir(root_path)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if dir_name.starts_with('_') || dir_name.starts_with('.') {
            continue;
        }

        let mut files = Vec::new();
        collect_supported_files(&path, &mut files)?;
        if files.is_empty() {
            continue;
        }
        files.sort_by(|a, b| natural_cmp(&a.0, &b.0));
        groups.insert(dir_name, files);
    }

    {
        let mut root_files = Vec::new();
        collect_supported_files(root_path, &mut root_files)?;
        // Filter out files in subdirectories (only keep root-level files)
        root_files.retain(|(_, path, _, _)| path.parent().is_some_and(|p| p == root_path));
        root_files.sort_by(|a, b| natural_cmp(&a.0, &b.0));
        if !root_files.is_empty() {
            let group_name = root_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("证据")
                .to_string();
            let mut unique_name = group_name.clone();
            let mut suffix = 1;
            while groups.contains_key(&unique_name) {
                unique_name = format!("{group_name}（根目录文件{suffix}）");
                suffix += 1;
            }
            groups.insert(unique_name, root_files);
        }
    }

    let mut groups_json = Vec::new();
    for (group_name, files) in &groups {
        let group_id = format!("{:016x}", fnv1a_hash(group_name));
        let files_json: Vec<serde_json::Value> = files
            .iter()
            .map(|(name, path, ft, size)| {
                let id = format!("{:016x}", fnv1a_hash(&path.display().to_string()));
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "path": path.display().to_string(),
                    "fileType": ft.label(),
                    "size": size,
                })
            })
            .collect();
        groups_json.push(serde_json::json!({
            "id": group_id,
            "name": group_name,
            "files": files_json,
        }));
    }

    Ok(serde_json::json!({
        "groups": groups_json,
        "root": root,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGroupPdfsArgs {
    pub root: String,
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupConfig {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub files: Vec<FileConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConfig {
    pub path: String,
    pub file_type: String,
}

pub fn build_group_pdfs(
    args: &BuildGroupPdfsArgs,
    conversion_state: &Arc<ConversionState>,
) -> Result<serde_json::Value> {
    let evidence_dir = Path::new(&args.root).join("_evidence_output");
    fs::create_dir_all(&evidence_dir)?;

    let mut results = Vec::new();
    let mut failed_conversions = Vec::new();
    let mut failed_groups = Vec::new();

    for group in &args.groups {
        crate::operations::check_current_cancelled()?;
        let group_name = &group.name;
        let group_id = &group.id;

        let mut pdf_paths: Vec<String> = Vec::new();

        for file in &group.files {
            let file_path = &file.path;
            let file_type = &file.file_type;

            match file_type.as_str() {
                "pdf" => {
                    pdf_paths.push(file_path.to_string());
                }
                "word" => match convert_word_to_pdf(file_path, &evidence_dir, conversion_state) {
                    Ok(converted) => pdf_paths.push(converted),
                    Err(err) => failed_conversions.push(serde_json::json!({
                        "groupId": group_id,
                        "groupName": group_name,
                        "path": file_path,
                        "name": Path::new(file_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(file_path),
                        "reason": err.to_string(),
                    })),
                },
                _ => {}
            }
        }

        if pdf_paths.is_empty() {
            continue;
        }

        let group_output = evidence_dir.join(format!(
            "{}-{:016x}.pdf",
            safe_file_stem(group_name),
            fnv1a_hash(group_id)
        ));
        match merge_pdfs_print(&pdf_paths, &group_output, Some(conversion_state)) {
            Ok((merged_path, page_count)) => {
                results.push(serde_json::json!({
                    "groupId": group_id,
                    "name": group_name,
                    "outputPath": merged_path.display().to_string(),
                    "pageCount": page_count,
                    "conversionFailures": failed_conversions
                        .iter()
                        .filter(|item| item["groupId"].as_str() == Some(group_id.as_str()))
                        .cloned()
                        .collect::<Vec<_>>(),
                }));
            }
            Err(err) => {
                if crate::operations::check_current_cancelled().is_err() {
                    return Err(err);
                }
                failed_groups.push(serde_json::json!({
                    "groupId": group_id,
                    "name": group_name,
                    "reason": err.to_string(),
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "evidenceDir": evidence_dir.display().to_string(),
        "results": results,
        "failedConversions": failed_conversions,
        "failedGroups": failed_groups,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeAllArgs {
    pub evidence_dir: String,
    pub group_pdfs: Vec<String>,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub identity: Option<IdentityConfig>,
    #[serde(default)]
    pub overlay: Option<OverlayConfig>,
}

pub fn merge_all(args: &MergeAllArgs) -> Result<String> {
    if args.group_pdfs.is_empty() {
        anyhow::bail!("没有可合并的分组 PDF");
    }

    let output_path_str = args.output_path.clone().unwrap_or_else(|| {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Path::new(&args.evidence_dir)
            .join(format!("evidence_merged_{}.pdf", ts))
            .display()
            .to_string()
    });

    let inputs: Vec<String> = args.group_pdfs.clone();

    let renamed_paths = if let Some(ref ident) = args.identity {
        apply_identity_rename(&inputs, &args.evidence_dir, ident)?
    } else {
        inputs
    };

    let overlaid_paths = if let Some(ref cfg) = args.overlay {
        apply_overlay_batch(&renamed_paths, &args.evidence_dir, cfg)?
    } else {
        renamed_paths
    };

    let (merged_path, _) =
        merge_pdfs_print(&overlaid_paths, Path::new(&output_path_str), None)?;

    Ok(merged_path.display().to_string())
}

fn merge_pdfs_print(
    inputs: &[String],
    output: &Path,
    conversion_state: Option<&Arc<ConversionState>>,
) -> Result<(PathBuf, u32)> {
    if inputs.is_empty() {
        anyhow::bail!("没有可合并的 PDF");
    }

    let policy = super::backend::policy::OutputPolicy {
        merge_policy: super::backend::policy::MergePolicy::Print,
        duplex_separate: false,
        ..Default::default()
    };
    let req = super::backend::MergeRequest {
        inputs: inputs.to_vec(),
        output_path: output.to_path_buf(),
        policy,
        preferred_backend: None,
        conversion_state: conversion_state.cloned(),
    };
    let res = super::backend::dispatch_merge(&req).map_err(|err| anyhow::anyhow!("{err}"))?;
    let total_pages = if res.total_pages > 0 {
        res.total_pages
    } else {
        super::qpdf::page_count(&res.output_path.to_string_lossy())?
    };
    Ok((res.output_path, total_pages))
}

fn apply_identity_rename(
    inputs: &[String],
    output_dir: &str,
    identity: &IdentityConfig,
) -> Result<Vec<String>> {
    let raw_prefix = identity.prefix.as_deref().unwrap_or("evidence");
    let prefix = crate::util::fs::safe_file_stem(raw_prefix);
    let prefix = if prefix.is_empty() { "evidence" } else { &prefix };
    let start = identity.start_number.unwrap_or(1);

    let rename_dir = Path::new(output_dir).join("_renamed");
    fs::create_dir_all(&rename_dir)?;

    let mut result = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        let num = start.saturating_add(i as u32);
        let new_name = format!("{}{:04}.pdf", prefix, num);
        let new_path = rename_dir.join(&new_name);
        fs::copy(input, &new_path)?;
        result.push(new_path.display().to_string());
    }

    Ok(result)
}

fn apply_overlay_batch(
    inputs: &[String],
    output_dir: &str,
    config: &OverlayConfig,
) -> Result<Vec<String>> {
    use crate::pdf::header_footer;

    let overlay_dir = Path::new(output_dir).join("_overlaid");
    fs::create_dir_all(&overlay_dir)?;

    let init_seq = config
        .header
        .as_ref()
        .and_then(|h| {
            if h.content == "sequence" {
                h.start_number
            } else {
                None
            }
        })
        .unwrap_or(1);

    let mut jobs = Vec::new();
    let mut global_seq = init_seq;

    for input in inputs {
        let stem = Path::new(input)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("page");
        let output = overlay_dir.join(format!(
            "{}-{:016x}_overlay.pdf",
            safe_file_stem(stem),
            fnv1a_hash(input)
        ));

        let mut job = serde_json::json!({
            "inputPath": input,
            "outputPath": output.display().to_string(),
            "pageStart": 1,
            "normalizeA4": false,
            "a4Orientation": "portrait",
            "a4ContentRotation": "none",
            "a4ContentMarginMm": 10,
            "rasterDpi": 300,
            "cleanup": {},
            "extraOverlays": [],
            "bookmarks": [],
            "bookmarkRemoveExisting": false,
        });

        if let Some(ref header) = config.header {
            if header.enabled {
                let text = match header.content.as_str() {
                    "filename" => Path::new(input)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string(),
                    "custom" => header.custom_text.clone().unwrap_or_default(),
                    "sequence" => global_seq.to_string(),
                    _ => String::new(),
                };
                job["header"] = serde_json::json!({
                    "text": text,
                    "region": "header",
                    "fontFamily": "",
                    "fontSize": header.font_size.unwrap_or(10.0) as f32,
                    "marginMm": pt_to_mm(header.y_offset.unwrap_or(30.0)) as f32,
                    "align": "left",
                    "offsetXMm": 0.0,
                    "color": "#000000",
                    "numberStyle": if header.content == "sequence" { "arabic" } else { "" },
                    "numberOffset": 0,
                });
            }
        }

        if let Some(ref footer) = config.footer {
            if footer.enabled {
                let text = match footer.content.as_str() {
                    "page_total" => "{page} / {total}".to_string(),
                    _ => String::new(),
                };
                job["footer"] = serde_json::json!({
                    "text": text,
                    "region": "footer",
                    "fontFamily": "",
                    "fontSize": footer.font_size.unwrap_or(9.0) as f32,
                    "marginMm": pt_to_mm(footer.y_offset.unwrap_or(20.0)) as f32,
                    "align": "left",
                    "offsetXMm": 0.0,
                    "color": "#000000",
                });
            }
        }

        jobs.push(job);

        if config.header.as_ref().map(|h| h.content.as_str()) == Some("sequence") {
            global_seq += 1;
        }
    }

    let jobs: Vec<header_footer::HeaderFooterJob> = jobs
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<_, _>>()
        .context("构建旧版证据页眉页脚任务失败")?;
    let result = header_footer::batch_overlay(&jobs)?;

    let output_paths: Vec<String> = result
        .results
        .iter()
        .map(|r| r.output_path.clone())
        .collect();

    if output_paths.is_empty() && !inputs.is_empty() {
        if !result.failed.is_empty() {
            let details = result
                .failed
                .iter()
                .map(|failure| format!("{}: {}", failure.path, failure.message))
                .collect::<Vec<_>>()
                .join("；");
            anyhow::bail!("旧版证据页眉页脚处理失败: {details}");
        }
        anyhow::bail!("旧版证据页眉页脚处理未生成任何输出文件");
    }

    Ok(output_paths)
}

fn pt_to_mm(pt: f64) -> f64 {
    pt * 25.4 / 72.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_includes_root_files_and_excludes_nested_outputs() {
        let root = super::super::temp_named_path("docsy_scan_audit", "dir");
        fs::create_dir_all(root.join("group/_evidence_output")).unwrap();
        fs::write(root.join("root.pdf"), b"fixture").unwrap();
        fs::write(root.join("group/10.pdf"), b"fixture").unwrap();
        fs::write(root.join("group/2.pdf"), b"fixture").unwrap();
        fs::write(root.join("group/_evidence_output/old.pdf"), b"fixture").unwrap();
        let scanned = scan_folder(root.to_str().unwrap()).unwrap();
        let groups = scanned["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 2);
        let group = groups.iter().find(|group| group["name"] == "group").unwrap();
        assert_eq!(group["files"].as_array().unwrap().len(), 2);
        assert_eq!(group["files"][0]["name"], "2.pdf");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn safe_file_stem_blocks_path_segments() {
        assert_eq!(safe_file_stem("../证据1/../../x"), "证据1_______x");
    }

    #[test]
    fn safe_file_stem_keeps_common_chinese_names() {
        assert_eq!(safe_file_stem("证据 1（聊天记录）"), "证据 1_聊天记录");
    }

    #[test]
    fn apply_identity_rename_blocks_path_traversal() {
        let temp_dir = std::env::temp_dir().join(format!("docsy_rename_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let sample_pdf = temp_dir.join("sample.pdf");
        std::fs::write(&sample_pdf, b"%PDF-1.4 dummy").unwrap();

        let identity = IdentityConfig {
            prefix: Some("../../escaped".into()),
            start_number: Some(1),
            ..Default::default()
        };

        let result = apply_identity_rename(
            &[sample_pdf.display().to_string()],
            &temp_dir.display().to_string(),
            &identity,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        let path = Path::new(&result[0]);
        assert!(path.starts_with(temp_dir.join("_renamed")));
        assert_eq!(path.file_name().unwrap(), "escaped0001.pdf");
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
