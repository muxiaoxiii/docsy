use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAnnotationsArgs {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(alias = "output")]
    output_path: String,
    #[serde(default)]
    kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAnnotationsResult {
    input_path: String,
    output_path: String,
    removed: usize,
    pages_touched: usize,
}

pub fn delete_annotations(args: &serde_json::Value) -> Result<DeleteAnnotationsResult> {
    let args: DeleteAnnotationsArgs =
        serde_json::from_value(args.clone()).context("解析批注删除参数失败")?;
    delete_annotations_file(&args.input_path, &args.output_path, &args.kinds)
}

pub fn delete_annotations_file(
    input_path: &str,
    output_path: &str,
    kinds: &[String],
) -> Result<DeleteAnnotationsResult> {
    let input = Path::new(input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }
    let output = Path::new(output_path);
    if same_path(input, output) {
        anyhow::bail!("批注删除输出路径不能和原始 PDF 相同");
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("创建批注删除输出目录失败")?;
        }
    }

    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let targets = AnnotationKinds::from_user_values(kinds);
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    let mut removed = 0_usize;
    let mut pages_touched = 0_usize;

    for page_id in page_ids {
        let Some(annots) = page_annotations(&doc, page_id) else {
            continue;
        };
        let original_len = annots.len();
        let kept: Vec<Object> = annots
            .into_iter()
            .filter(|annot| !targets.should_remove(&doc, annot))
            .collect();
        let removed_on_page = original_len.saturating_sub(kept.len());
        if removed_on_page == 0 {
            continue;
        }
        removed += removed_on_page;
        pages_touched += 1;

        let page = doc
            .get_object_mut(page_id)
            .context("读取 PDF 页面对象失败")?
            .as_dict_mut()
            .context("PDF 页面对象不是字典")?;
        if kept.is_empty() {
            page.remove(b"Annots");
        } else {
            page.set("Annots", Object::Array(kept));
        }
    }

    doc.prune_objects();
    doc.save(output).context("保存删除批注后的 PDF 失败")?;

    Ok(DeleteAnnotationsResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        removed,
        pages_touched,
    })
}

pub fn delete_annotations_to_temp(input_path: &str, kinds: &[String]) -> Result<PathBuf> {
    let output = temp_named_path("docsy_annotations_removed", "pdf");
    delete_annotations_file(input_path, &output.to_string_lossy(), kinds)?;
    Ok(output)
}

fn page_annotations(doc: &Document, page_id: ObjectId) -> Option<Vec<Object>> {
    let page = doc.get_object(page_id).ok()?.as_dict().ok()?;
    let annots = page.get(b"Annots").ok()?;
    match annots {
        Object::Array(items) => Some(items.clone()),
        Object::Reference(id) => doc.get_object(*id).ok()?.as_array().ok().cloned(),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct AnnotationKinds {
    values: BTreeSet<String>,
}

impl AnnotationKinds {
    fn from_user_values(values: &[String]) -> Self {
        let values = if values.is_empty() {
            vec![
                "Text".to_string(),
                "FreeText".to_string(),
                "Highlight".to_string(),
                "Underline".to_string(),
                "StrikeOut".to_string(),
                "Squiggly".to_string(),
                "Ink".to_string(),
                "Stamp".to_string(),
                "Square".to_string(),
                "Circle".to_string(),
                "Line".to_string(),
                "Polygon".to_string(),
                "PolyLine".to_string(),
                "Caret".to_string(),
            ]
        } else {
            values.to_vec()
        };
        Self {
            values: values.into_iter().map(normalize_kind).collect(),
        }
    }

    fn should_remove(&self, doc: &Document, annot: &Object) -> bool {
        let Some(kind) = annotation_subtype(doc, annot) else {
            return false;
        };
        self.values.contains(&normalize_kind(&kind))
    }
}

fn annotation_subtype(doc: &Document, annot: &Object) -> Option<String> {
    let dict = match annot {
        Object::Dictionary(dict) => dict,
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok()?,
        _ => return None,
    };
    let subtype = dict.get(b"Subtype").ok()?;
    match subtype {
        Object::Name(name) => Some(String::from_utf8_lossy(name).to_string()),
        _ => None,
    }
}

fn normalize_kind(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase()
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
    use lopdf::Dictionary;

    #[test]
    fn normalizes_annotation_kinds() {
        assert_eq!(normalize_kind("/Highlight"), "highlight");
        assert_eq!(normalize_kind("FreeText"), "freetext");
    }

    #[test]
    fn default_kinds_remove_review_annotations_not_links() {
        let doc = Document::with_version("1.7");
        let targets = AnnotationKinds::from_user_values(&[]);
        let mut highlight = Dictionary::new();
        highlight.set("Subtype", Object::Name(b"Highlight".to_vec()));
        let mut link = Dictionary::new();
        link.set("Subtype", Object::Name(b"Link".to_vec()));

        assert!(targets.should_remove(&doc, &Object::Dictionary(highlight)));
        assert!(!targets.should_remove(&doc, &Object::Dictionary(link)));
    }
}
