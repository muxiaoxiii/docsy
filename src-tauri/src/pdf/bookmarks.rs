//! PDF 书签（Outlines）增删查。
//! 读侧用 qpdf JSON（不拉流）；写侧用 qpdf `--update-from-json`（JSON v2 字符串须 `u:` / `b:` 前缀）。

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::external::ExternalTool;

use super::header_footer::BookmarkConfig;
use crate::util::fs::{replace_file, sibling_temp_path, TempPathGuard};

#[derive(Debug, Clone)]
struct BookmarksIndex {
    catalog_ref: String,
    catalog: Map<String, Value>,
    page_refs: Vec<String>,
    max_obj: u32,
}

fn load_bookmarks_index(path: &Path) -> Result<BookmarksIndex> {
    let qpdf_bin = crate::external::QpdfTool.binary_path()?;
    let output = crate::external::hidden_command(qpdf_bin)
        .arg("--json=1")
        .arg("--json-key=pages")
        .arg("--json-key=objects")
        .arg("--json-stream-data=none")
        .arg(path)
        .output()
        .with_context(|| format!("读取 PDF 书签信息失败: {}", path.display()))?;
    if !super::qpdf::status_is_success(&output.status) {
        anyhow::bail!(
            "读取 PDF 书签信息失败：{}",
            crate::external::command_failure_detail(&output)
        );
    }
    let json: Value = serde_json::from_slice(&output.stdout).context("解析 PDF 书签 JSON 失败")?;
    let objects = json
        .get("objects")
        .or_else(|| json.pointer("/qpdf/1"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let pages = json
        .get("pages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut catalog_ref = String::new();
    let mut catalog = Map::new();
    let mut max_obj = 0_u32;
    for (key, object) in &objects {
        if let Some(num) = object_number_from_key(key) {
            max_obj = max_obj.max(num);
        }
        let Some(dict) = object_dict_of(object) else {
            continue;
        };
        if dict
            .get("/Type")
            .and_then(Value::as_str)
            .is_some_and(|t| t.trim_start_matches('/').eq_ignore_ascii_case("catalog"))
        {
            catalog_ref = object_gen_from_key(key).unwrap_or_default();
            catalog = dict.clone();
        }
    }
    if catalog_ref.is_empty() {
        anyhow::bail!("找不到 PDF Catalog");
    }

    let page_refs = pages
        .iter()
        .filter_map(|page| page.get("object").and_then(Value::as_str))
        .map(|raw| raw.trim_end_matches(" R").trim().to_string())
        .collect();

    Ok(BookmarksIndex {
        catalog_ref,
        catalog,
        page_refs,
        max_obj,
    })
}

fn object_number_from_key(key: &str) -> Option<u32> {
    key.strip_prefix("obj:")
        .unwrap_or(key)
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// `"obj:12 0 R"` / `"12 0"` → `"12 0"`
fn object_gen_from_key(key: &str) -> Option<String> {
    let raw = key.strip_prefix("obj:").unwrap_or(key);
    let mut parts = raw.split_whitespace();
    let num = parts.next()?;
    let gen = parts.next().unwrap_or("0");
    Some(format!("{num} {gen}"))
}

fn object_dict_of(object: &Value) -> Option<&Map<String, Value>> {
    let map = object.as_object()?;
    if map.contains_key("/Type") || map.contains_key("/Outlines") || map.contains_key("/Pages") {
        return Some(map);
    }
    map.get("value").and_then(Value::as_object)
}

/// qpdf JSON v2 对象键：`obj:O G R`
fn obj_key(num: u32) -> String {
    format!("obj:{num} 0 R")
}

fn catalog_key(catalog_ref: &str) -> String {
    let trimmed = catalog_ref.trim_end_matches(" R").trim();
    format!("obj:{trimmed} R")
}

/// qpdf JSON v2：可无损表示为 Unicode 的 PDF 字符串必须加 `u:` 前缀。
fn qpdf_string(text: &str) -> String {
    format!("u:{text}")
}

pub fn apply_bookmarks(
    output: &Path,
    bookmarks: &[BookmarkConfig],
    remove_existing: bool,
) -> Result<()> {
    let active: Vec<&BookmarkConfig> = bookmarks
        .iter()
        .filter(|b| b.enabled && !b.label.is_empty())
        .collect();

    if active.is_empty() && !remove_existing {
        return Ok(());
    }

    let index = load_bookmarks_index(output)?;
    let mut next_obj = index.max_obj.saturating_add(1);
    let mut updates: Map<String, Value> = Map::new();

    if active.is_empty() {
        let mut catalog = index.catalog.clone();
        catalog.insert("/Outlines".to_string(), Value::Null);
        updates.insert(catalog_key(&index.catalog_ref), json!({ "value": catalog }));
        return apply_qpdf_json_update(output, &updates);
    }

    if index.page_refs.is_empty() {
        anyhow::bail!("PDF 无页面，无法写入书签");
    }

    let outlines_num = next_obj;
    next_obj += 1;
    let outlines_ref = format!("{outlines_num} 0 R");

    let mut item_nums = Vec::with_capacity(active.len());
    for config in &active {
        let page_index = config.page_index as usize;
        let page_ref = index
            .page_refs
            .get(page_index)
            .with_context(|| format!("书签页码超出文档范围（{}）", config.page_index))?;
        let item_num = next_obj;
        next_obj += 1;
        item_nums.push((item_num, config.label.clone(), page_ref.clone()));
    }

    let count = item_nums.len();
    for (i, (item_num, label, page_ref)) in item_nums.iter().enumerate() {
        let mut item = Map::new();
        item.insert("/Title".to_string(), Value::String(qpdf_string(label)));
        item.insert(
            "/Dest".to_string(),
            json!([
                format!("{page_ref} R"),
                "/XYZ",
                Value::Null,
                Value::Null,
                Value::Null
            ]),
        );
        item.insert("/Parent".to_string(), Value::String(outlines_ref.clone()));
        if i > 0 {
            item.insert(
                "/Prev".to_string(),
                Value::String(format!("{} 0 R", item_nums[i - 1].0)),
            );
        }
        if i + 1 < count {
            item.insert(
                "/Next".to_string(),
                Value::String(format!("{} 0 R", item_nums[i + 1].0)),
            );
        }
        updates.insert(obj_key(*item_num), json!({ "value": item }));
    }

    let mut outlines = Map::new();
    outlines.insert("/Type".to_string(), Value::String("/Outlines".into()));
    outlines.insert("/Count".to_string(), json!(count as i64));
    outlines.insert(
        "/First".to_string(),
        Value::String(format!("{} 0 R", item_nums[0].0)),
    );
    outlines.insert(
        "/Last".to_string(),
        Value::String(format!("{} 0 R", item_nums[count - 1].0)),
    );
    updates.insert(obj_key(outlines_num), json!({ "value": outlines }));

    let mut catalog = index.catalog.clone();
    catalog.insert("/Outlines".to_string(), Value::String(outlines_ref));
    updates.insert(catalog_key(&index.catalog_ref), json!({ "value": catalog }));

    apply_qpdf_json_update(output, &updates)
}

pub fn remove_pdf_bookmarks(path: &Path) -> Result<()> {
    let index = load_bookmarks_index(path)?;
    if !index.catalog.contains_key("/Outlines") {
        return Ok(());
    }
    let mut catalog = index.catalog.clone();
    catalog.insert("/Outlines".to_string(), Value::Null);
    let mut updates = Map::new();
    updates.insert(catalog_key(&index.catalog_ref), json!({ "value": catalog }));
    apply_qpdf_json_update(path, &updates)
}

pub fn has_pdf_bookmarks(path: &Path) -> Result<bool> {
    let index = load_bookmarks_index(path)?;
    Ok(index.catalog.contains_key("/Outlines")
        && !index.catalog.get("/Outlines").is_some_and(Value::is_null))
}

fn apply_qpdf_json_update(path: &Path, updates: &Map<String, Value>) -> Result<()> {
    let qpdf_bin = crate::external::QpdfTool.binary_path()?;
    // qpdf JSON v2 要求首元素含 jsonversion
    let update_doc = json!({
        "qpdf": [
            {
                "jsonversion": 2,
                "pdfversion": "1.7",
                "pushedinheritedpageresources": false,
                "calledgetallpages": false,
            },
            updates
        ]
    });

    let update_path = sibling_temp_path(path, "docsy-bookmarks-update");
    let update_guard = TempPathGuard::new(update_path.clone());
    std::fs::write(&update_path, serde_json::to_vec_pretty(&update_doc)?)
        .context("写入书签 JSON 更新失败")?;

    let temp_out = sibling_temp_path(path, "docsy-bookmarks");
    let out_guard = TempPathGuard::new(temp_out.clone());

    let result = crate::external::hidden_command(qpdf_bin)
        .arg(format!(
            "--update-from-json={}",
            update_path.to_string_lossy()
        ))
        .arg(path)
        .arg(&temp_out)
        .output()
        .context("执行 qpdf 书签更新失败")?;
    if !super::qpdf::status_is_success(&result.status) {
        anyhow::bail!(
            "qpdf 书签更新失败：{}",
            crate::external::command_failure_detail(&result)
        );
    }
    replace_file(&temp_out, path).context("替换书签 PDF 失败")?;
    drop(update_guard);
    let _ = out_guard;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fs::temp_named_path;

    fn create_simple_test_pdf(path: &Path) {
        use lopdf::{dictionary, Document, Object, Stream};

        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let content = lopdf::content::Content {
            operations: vec![
                lopdf::content::Operation::new("BT", vec![]),
                lopdf::content::Operation::new(
                    "Tf",
                    vec![Object::Name(b"F1".to_vec()), 12.into()],
                ),
                lopdf::content::Operation::new("Td", vec![80.into(), 500.into()]),
                lopdf::content::Operation::new("Tj", vec![Object::string_literal("body")]),
                lopdf::content::Operation::new("ET", vec![]),
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

    #[test]
    fn qpdf_string_uses_unicode_prefix() {
        assert_eq!(qpdf_string("测试"), "u:测试");
        assert_eq!(qpdf_string(""), "u:");
    }

    #[test]
    fn bookmark_roundtrip_via_qpdf() {
        if crate::external::QpdfTool.binary_path().is_err() {
            return;
        }
        let path = temp_named_path("docsy_bookmark_rt_qpdf", "pdf");
        create_simple_test_pdf(&path);

        apply_bookmarks(
            &path,
            &[BookmarkConfig {
                enabled: true,
                label: "测试书签".to_string(),
                page_index: 0,
            }],
            false,
        )
        .unwrap();
        assert!(has_pdf_bookmarks(&path).unwrap());

        remove_pdf_bookmarks(&path).unwrap();
        assert!(!has_pdf_bookmarks(&path).unwrap());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn out_of_range_bookmark_fails() {
        if crate::external::QpdfTool.binary_path().is_err() {
            return;
        }
        let path = temp_named_path("docsy_bookmark_range", "pdf");
        create_simple_test_pdf(&path);
        let before = std::fs::read(&path).unwrap();
        let result = apply_bookmarks(
            &path,
            &[BookmarkConfig {
                enabled: true,
                label: "越界".to_string(),
                page_index: 99,
            }],
            true,
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let _ = std::fs::remove_file(&path);
    }
}
