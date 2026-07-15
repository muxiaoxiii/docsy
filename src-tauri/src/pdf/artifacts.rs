use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteHeaderFooterArtifactsArgs {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(alias = "output")]
    output_path: String,
    #[serde(default)]
    remove_header: bool,
    #[serde(default)]
    remove_footer: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteHeaderFooterArtifactsResult {
    input_path: String,
    output_path: String,
    removed: usize,
    removed_header: usize,
    removed_footer: usize,
    pages_touched: usize,
}

impl DeleteHeaderFooterArtifactsResult {
    pub(crate) fn removed_count(&self) -> usize {
        self.removed
    }

    pub(crate) fn removed_header_count(&self) -> usize {
        self.removed_header
    }

    pub(crate) fn removed_footer_count(&self) -> usize {
        self.removed_footer
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HeaderFooterArtifactTargets {
    pub header: bool,
    pub footer: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ArtifactRemovalStats {
    pub header: usize,
    pub footer: usize,
}

impl ArtifactRemovalStats {
    fn total(self) -> usize {
        self.header + self.footer
    }

    fn add_region(&mut self, region: ArtifactRegion) {
        match region {
            ArtifactRegion::Header => self.header += 1,
            ArtifactRegion::Footer => self.footer += 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ArtifactRegion {
    Header,
    Footer,
}

pub fn delete_header_footer_artifacts(
    args: &serde_json::Value,
) -> Result<DeleteHeaderFooterArtifactsResult> {
    let args: DeleteHeaderFooterArtifactsArgs =
        serde_json::from_value(args.clone()).context("解析标准页眉页脚删除参数失败")?;
    delete_header_footer_artifacts_file(
        &args.input_path,
        &args.output_path,
        HeaderFooterArtifactTargets {
            header: args.remove_header,
            footer: args.remove_footer,
        },
    )
}

pub fn delete_header_footer_artifacts_file(
    input_path: &str,
    output_path: &str,
    targets: HeaderFooterArtifactTargets,
) -> Result<DeleteHeaderFooterArtifactsResult> {
    let input = Path::new(input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }
    if !targets.header && !targets.footer {
        std::fs::copy(input, output_path).context("复制 PDF 失败")?;
        return Ok(DeleteHeaderFooterArtifactsResult {
            input_path: input_path.to_string(),
            output_path: output_path.to_string(),
            removed: 0,
            removed_header: 0,
            removed_footer: 0,
            pages_touched: 0,
        });
    }
    let output = Path::new(output_path);
    if same_path(input, output) {
        anyhow::bail!("标准页眉页脚删除输出路径不能和原始 PDF 相同");
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("创建标准页眉页脚删除输出目录失败")?;
        }
    }

    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    let mut removed = ArtifactRemovalStats::default();
    let mut pages_touched = 0_usize;

    for page_id in page_ids {
        let content = match doc.get_and_decode_page_content(page_id) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let properties = page_properties(&doc, page_id);
        let (filtered, removed_on_page) =
            remove_target_artifact_ranges(&content.operations, targets, &properties);
        if removed_on_page.total() == 0 {
            continue;
        }
        let encoded = Content {
            operations: filtered,
        }
        .encode()
        .context("编码删除标准页眉页脚后的内容流失败")?;
        doc.change_page_content(page_id, encoded)
            .context("写回删除标准页眉页脚后的内容流失败")?;
        removed.header += removed_on_page.header;
        removed.footer += removed_on_page.footer;
        pages_touched += 1;
    }

    doc.prune_objects();
    doc.save(output)
        .context("保存删除标准页眉页脚后的 PDF 失败")?;

    Ok(DeleteHeaderFooterArtifactsResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        removed: removed.total(),
        removed_header: removed.header,
        removed_footer: removed.footer,
        pages_touched,
    })
}

pub fn delete_header_footer_artifacts_to_temp(
    input_path: &str,
    targets: HeaderFooterArtifactTargets,
) -> Result<Option<(PathBuf, DeleteHeaderFooterArtifactsResult)>> {
    if !targets.header && !targets.footer {
        return Ok(None);
    }
    let output = temp_named_path("docsy_hf_artifacts_removed", "pdf");
    let result =
        delete_header_footer_artifacts_file(input_path, &output.to_string_lossy(), targets)?;
    if result.removed == 0 {
        let _ = std::fs::remove_file(&output);
        return Ok(None);
    }
    Ok(Some((output, result)))
}

fn remove_target_artifact_ranges(
    operations: &[Operation],
    targets: HeaderFooterArtifactTargets,
    properties: &Dictionary,
) -> (Vec<Operation>, ArtifactRemovalStats) {
    let mut filtered = Vec::with_capacity(operations.len());
    let mut removed = ArtifactRemovalStats::default();
    let mut index = 0_usize;

    while index < operations.len() {
        if let Some(region) = target_artifact_region(&operations[index], targets, properties) {
            if let Some(end) = matching_marked_content_end(operations, index) {
                removed.add_region(region);
                index = end + 1;
                continue;
            }
        }
        filtered.push(operations[index].clone());
        index += 1;
    }

    (filtered, removed)
}

fn matching_marked_content_end(operations: &[Operation], start: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (index, operation) in operations.iter().enumerate().skip(start) {
        if is_marked_content_start(operation) {
            depth += 1;
        } else if operation.operator == "EMC" {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn is_marked_content_start(operation: &Operation) -> bool {
    operation.operator == "BMC" || operation.operator == "BDC"
}

fn target_artifact_region(
    operation: &Operation,
    targets: HeaderFooterArtifactTargets,
    properties: &Dictionary,
) -> Option<ArtifactRegion> {
    if operation.operator != "BDC" {
        return None;
    }
    let tag = operation.operands.first().and_then(name_bytes)?;
    if tag != b"Artifact" {
        return None;
    }
    let property = operation.operands.get(1)?;
    let subtype = artifact_subtype(property, properties)?;
    if targets.header && subtype == b"Header" {
        Some(ArtifactRegion::Header)
    } else if targets.footer && subtype == b"Footer" {
        Some(ArtifactRegion::Footer)
    } else {
        None
    }
}

fn artifact_subtype<'a>(property: &'a Object, properties: &'a Dictionary) -> Option<&'a [u8]> {
    match property {
        Object::Dictionary(dict) => dict.get(b"Subtype").ok().and_then(name_bytes),
        Object::Name(name) => properties
            .get(name)
            .ok()
            .and_then(|object| object.as_dict().ok())
            .and_then(|dict| dict.get(b"Subtype").ok())
            .and_then(name_bytes),
        _ => None,
    }
}

fn page_properties(doc: &Document, page_id: ObjectId) -> Dictionary {
    let mut properties = Dictionary::new();
    if let Ok((resource_dict, resource_ids)) = doc.get_page_resources(page_id) {
        if let Some(resources) = resource_dict {
            merge_properties(doc, resources, &mut properties);
        }
        for id in resource_ids {
            if let Ok(resources) = doc.get_dictionary(id) {
                merge_properties(doc, resources, &mut properties);
            }
        }
    }
    properties
}

fn merge_properties(doc: &Document, resources: &Dictionary, output: &mut Dictionary) {
    let Ok(properties_obj) = resources.get(b"Properties") else {
        return;
    };
    let properties_dict = match properties_obj {
        Object::Dictionary(dict) => Some(dict),
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        _ => None,
    };
    if let Some(properties_dict) = properties_dict {
        for (name, value) in properties_dict.iter() {
            output.set(name.clone(), value.clone());
        }
    }
}

fn name_bytes(object: &Object) -> Option<&[u8]> {
    object.as_name().ok()
}

fn same_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normalize_path_components(&absolute)
}

fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
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
    use lopdf::dictionary;
    use lopdf::Stream;

    #[test]
    fn removes_standard_header_artifact_range() {
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Type" => "Pagination",
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("old header")]),
            Operation::new("EMC", vec![]),
            Operation::new("ET", vec![]),
        ];
        let (filtered, removed) = remove_target_artifact_ranges(
            &operations,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &Dictionary::new(),
        );
        assert_eq!(removed.total(), 1);
        assert_eq!(removed.header, 1);
        assert_eq!(
            filtered
                .iter()
                .map(|op| op.operator.as_str())
                .collect::<Vec<_>>(),
            vec!["BT", "ET"]
        );
    }

    #[test]
    fn keeps_footer_when_only_removing_header() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Subtype" => "Footer",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("1/2")]),
            Operation::new("EMC", vec![]),
        ];
        let (filtered, removed) = remove_target_artifact_ranges(
            &operations,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &Dictionary::new(),
        );
        assert_eq!(removed.total(), 0);
        assert_eq!(filtered.len(), operations.len());
    }

    #[test]
    fn resolves_named_artifact_properties() {
        let mut properties = Dictionary::new();
        properties.set(
            "HF1",
            Object::Dictionary(dictionary! {
                "Subtype" => "Header",
            }),
        );
        let operation = Operation::new(
            "BDC",
            vec![
                Object::Name(b"Artifact".to_vec()),
                Object::Name(b"HF1".to_vec()),
            ],
        );
        assert!(target_artifact_region(
            &operation,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &properties
        )
        .is_some());
    }

    #[test]
    fn refuses_unbalanced_marked_content_removal() {
        let operations = vec![Operation::new(
            "BDC",
            vec![
                Object::Name(b"Artifact".to_vec()),
                Object::Dictionary(dictionary! {
                    "Subtype" => "Header",
                }),
            ],
        )];
        let (filtered, removed) = remove_target_artifact_ranges(
            &operations,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &Dictionary::new(),
        );
        assert_eq!(removed.total(), 0);
        assert_eq!(filtered.len(), operations.len());
    }

    #[test]
    fn deletes_header_artifact_from_pdf_file() {
        let input = temp_named_path("docsy_artifact_test_input", "pdf");
        let output = temp_named_path("docsy_artifact_test_output", "pdf");
        create_artifact_test_pdf(&input);

        let result = delete_header_footer_artifacts_file(
            &input.to_string_lossy(),
            &output.to_string_lossy(),
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
        )
        .unwrap();
        assert_eq!(result.removed_count(), 1);
        assert_eq!(result.pages_touched, 1);

        let doc = Document::load(&output).unwrap();
        let page_id = doc.get_pages().into_values().next().unwrap();
        let content = doc.get_and_decode_page_content(page_id).unwrap();
        let text = content
            .operations
            .iter()
            .flat_map(|op| op.operands.iter())
            .filter_map(|object| object.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .collect::<Vec<_>>();
        assert!(!text.iter().any(|value| value.contains("old header")));
        assert!(text.iter().any(|value| value.contains("body text")));

        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    fn create_artifact_test_pdf(path: &Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });
        let content = Content {
            operations: vec![
                Operation::new(
                    "BDC",
                    vec![
                        Object::Name(b"Artifact".to_vec()),
                        Object::Dictionary(dictionary! {
                            "Type" => "Pagination",
                            "Subtype" => "Header",
                        }),
                    ],
                ),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
                Operation::new("Td", vec![450.into(), 800.into()]),
                Operation::new("Tj", vec![Object::string_literal("old header")]),
                Operation::new("ET", vec![]),
                Operation::new("EMC", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new("Td", vec![80.into(), 500.into()]),
                Operation::new("Tj", vec![Object::string_literal("body text")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }
}
