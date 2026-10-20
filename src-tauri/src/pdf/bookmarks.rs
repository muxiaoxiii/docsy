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

fn apply_bookmarks_native(
    output: &Path,
    active: &[&BookmarkConfig],
) -> Result<()> {
    crate::pdf::qpdf::validate_recovery_backup(output)?;
    let bytes = std::fs::read(output)
        .with_context(|| format!("读取 PDF 文件失败: {}", output.display()))?;
    let mut doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes))
        .with_context(|| format!("解析 PDF 文件失败: {}", output.display()))?;
    if doc.security().is_some() {
        anyhow::bail!("加密 PDF 书签更新需保留原加密设置，交由外置工具处理");
    }

    let root = doc.root().context("找不到 PDF Catalog")?;
    let page_count = pdfcraft_organize::page_count(&doc).context("读取 PDF 页数失败")?;

    if !active.is_empty() && page_count == 0 {
        anyhow::bail!("PDF 无页面，无法写入书签");
    }

    for config in active {
        if (config.page_index as usize) >= page_count {
            anyhow::bail!("书签页码超出文档范围（{}）", config.page_index);
        }
    }

    doc.update_dict(root, |c| {
        c.remove(b"Outlines");
    })
    .context("清理原有 PDF Outlines 失败")?;

    for (idx, config) in active.iter().enumerate() {
        pdfcraft_organize::add_bookmark(
            &mut doc,
            &[],
            idx,
            &config.label,
            config.page_index as usize,
        )
        .with_context(|| format!("添加 PDF 书签失败: {}", config.label))?;
    }

    let out_bytes = pdfcraft_cos::write_full(&doc, &pdfcraft_cos::SaveOptions::default())
        .context("序列化书签 PDF 失败")?;
    let temp_out = sibling_temp_path(output, "docsy-bookmarks-native");
    let out_guard = TempPathGuard::new(temp_out.clone());
    std::fs::write(&temp_out, &out_bytes).context("写入书签临时文件失败")?;
    replace_file(&temp_out, output).context("替换书签 PDF 失败")?;
    let _ = out_guard;
    Ok(())
}

pub fn apply_bookmarks(
    output: &Path,
    bookmarks: &[BookmarkConfig],
    remove_existing: bool,
) -> Result<()> {
    let active: Vec<&BookmarkConfig> = bookmarks
        .iter()
        .filter(|b| b.enabled && !b.label.trim().is_empty())
        .collect();

    if active.is_empty() && !remove_existing {
        return Ok(());
    }

    match apply_bookmarks_native(output, &active) {
        Ok(()) => return Ok(()),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("书签页码超出文档范围")
                || msg.contains("PDF 无页面")
                || msg.contains("旧版防复制恢复备份")
            {
                return Err(err);
            }
        }
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

fn remove_pdf_bookmarks_native(path: &Path) -> Result<()> {
    crate::pdf::qpdf::validate_recovery_backup(path)?;
    let bytes = std::fs::read(path)
        .with_context(|| format!("读取 PDF 文件失败: {}", path.display()))?;
    let mut doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes))
        .with_context(|| format!("解析 PDF 文件失败: {}", path.display()))?;
    if doc.security().is_some() {
        anyhow::bail!("加密 PDF 书签移除需保留原加密设置，交由外置工具处理");
    }
    let root = doc.root().context("找不到 PDF Catalog")?;
    let has_outlines = doc
        .get(root)
        .as_dict()
        .is_some_and(|c| c.contains(b"Outlines"));
    if !has_outlines {
        return Ok(());
    }
    doc.update_dict(root, |c| {
        c.remove(b"Outlines");
    })
    .context("移除 PDF Outlines 失败")?;

    let out_bytes = pdfcraft_cos::write_full(&doc, &pdfcraft_cos::SaveOptions::default())
        .context("序列化无书签 PDF 失败")?;
    let temp_out = sibling_temp_path(path, "docsy-bookmarks-remove");
    let out_guard = TempPathGuard::new(temp_out.clone());
    std::fs::write(&temp_out, &out_bytes).context("写入临时文件失败")?;
    replace_file(&temp_out, path).context("替换 PDF 失败")?;
    let _ = out_guard;
    Ok(())
}

pub fn remove_pdf_bookmarks(path: &Path) -> Result<()> {
    crate::pdf::qpdf::validate_recovery_backup(path)?;
    if remove_pdf_bookmarks_native(path).is_ok() {
        return Ok(());
    }
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

fn has_pdf_bookmarks_native(path: &Path) -> Result<bool> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("读取 PDF 文件失败: {}", path.display()))?;
    let doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes))
        .with_context(|| format!("解析 PDF 文件失败: {}", path.display()))?;
    let root = doc.root().context("找不到 PDF Catalog")?;
    let catalog_obj = doc.get(root);
    let catalog = catalog_obj.as_dict().context("Catalog 不是字典")?;
    Ok(catalog
        .get(b"Outlines")
        .is_some_and(|v| !matches!(doc.resolve(v).as_ref(), pdfcraft_cos::Object::Null)))
}

pub fn has_pdf_bookmarks(path: &Path) -> Result<bool> {
    if let Ok(has) = has_pdf_bookmarks_native(path) {
        return Ok(has);
    }
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

        let bytes = std::fs::read(&path).unwrap();
        let doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();
        let items = pdfcraft_organize::bookmarks(&doc);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "测试书签");
        assert_eq!(items[0].page, Some(0));

        remove_pdf_bookmarks(&path).unwrap();
        assert!(!has_pdf_bookmarks(&path).unwrap());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn out_of_range_bookmark_fails() {
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

    #[test]
    fn bookmark_operations_do_not_strip_encryption() {
        let plain_path = temp_named_path("docsy_bookmark_enc_plain", "pdf");
        let enc_path = temp_named_path("docsy_bookmark_enc_target", "pdf");
        create_simple_test_pdf(&plain_path);

        let Ok(qpdf_bin) = crate::external::QpdfTool.binary_path() else {
            let _ = std::fs::remove_file(&plain_path);
            return;
        };
        let status = crate::external::hidden_command(qpdf_bin)
            .args([
                "--encrypt",
                "",
                "owner-secret",
                "256",
                "--",
                &plain_path.to_string_lossy(),
                &enc_path.to_string_lossy(),
            ])
            .status()
            .unwrap();
        assert!(status.success());

        // Native path must refuse to strip encryption, falling back to qpdf JSON update
        assert!(apply_bookmarks_native(
            &enc_path,
            &[&BookmarkConfig {
                enabled: true,
                label: "加密书签".to_string(),
                page_index: 0,
            }],
        )
        .is_err());

        apply_bookmarks(
            &enc_path,
            &[BookmarkConfig {
                enabled: true,
                label: "加密书签".to_string(),
                page_index: 0,
            }],
            false,
        )
        .unwrap();

        let after_add =
            pdfcraft_cos::Document::open(std::sync::Arc::new(std::fs::read(&enc_path).unwrap()))
                .unwrap();
        assert!(
            after_add.security().is_some(),
            "adding bookmarks must preserve PDF encryption"
        );

        remove_pdf_bookmarks(&enc_path).unwrap();
        let after_remove =
            pdfcraft_cos::Document::open(std::sync::Arc::new(std::fs::read(&enc_path).unwrap()))
                .unwrap();
        assert!(
            after_remove.security().is_some(),
            "removing bookmarks must preserve PDF encryption"
        );

        let _ = std::fs::remove_file(&plain_path);
        let _ = std::fs::remove_file(&enc_path);
    }
}
