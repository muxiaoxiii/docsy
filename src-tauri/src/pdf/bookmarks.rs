//! PDF 书签（Outlines）增删查。

use anyhow::{Context, Result};
use lopdf::{dictionary, Document, Object};
use std::path::Path;

#[cfg(test)]
use crate::util::fs::temp_named_path;
#[cfg(test)]
use std::fs;

use super::header_footer::BookmarkConfig;
use super::overlay_font::utf16be_pdf_text;
use crate::util::fs::{replace_file, sibling_temp_path, TempPathGuard};

/// 写入多个书签，创建 /First /Last /Next /Prev 链
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

    let temp = TempPathGuard::new(sibling_temp_path(output, "docsy-bookmarks"));
    let mut doc = Document::load(output).context("加载 PDF 以写入书签失败")?;
    if remove_existing {
        remove_outlines(&mut doc)?;
    }
    if active.is_empty() {
        save_replacing(doc, output, temp.path(), "保存删除书签后的 PDF 失败")?;
        return Ok(());
    }
    let pages = doc.get_pages();
    let page_ids: Vec<lopdf::ObjectId> = pages.into_values().collect();

    // Create outline items
    let mut item_ids = Vec::new();
    for config in &active {
        let page_id = page_ids
            .get(config.page_index as usize)
            .copied()
            .context("书签页码超出文档范围")?;

        let item_id = doc.add_object(dictionary! {
            "Title" => Object::String(utf16be_pdf_text(&config.label), lopdf::StringFormat::Hexadecimal),
            "Dest" => vec![
                Object::Reference(page_id),
                Object::Name(b"XYZ".to_vec()),
                Object::Null,
                Object::Null,
                Object::Null,
            ],
        });
        item_ids.push(item_id);
    }

    // Link items with Next/Prev
    for i in 0..item_ids.len() {
        if let Some(Object::Dictionary(item)) = doc.objects.get_mut(&item_ids[i]) {
            if i > 0 {
                item.set("Prev", item_ids[i - 1]);
            }
            if i + 1 < item_ids.len() {
                item.set("Next", item_ids[i + 1]);
            }
        }
    }

    // Create Outlines dictionary
    let outlines_id = doc.add_object(dictionary! {
        "Type" => "Outlines",
        "Count" => item_ids.len() as i64,
        "First" => item_ids[0],
        "Last" => item_ids[item_ids.len() - 1],
    });

    // Set Parent on all items
    for item_id in &item_ids {
        if let Some(Object::Dictionary(item)) = doc.objects.get_mut(item_id) {
            item.set("Parent", outlines_id);
        }
    }

    // Set Outlines in Catalog
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|obj| obj.as_reference())
        .context("找不到 PDF Catalog")?;
    if let Some(Object::Dictionary(catalog)) = doc.objects.get_mut(&catalog_id) {
        catalog.set("Outlines", outlines_id);
    }

    save_replacing(doc, output, temp.path(), "保存书签 PDF 失败")
}

/// 检查 PDF 的 Catalog 是否包含 /Outlines（即已有书签）
pub fn has_pdf_bookmarks(path: &Path) -> Result<bool> {
    let doc = Document::load(path).with_context(|| format!("加载 PDF 失败: {}", path.display()))?;
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|obj| obj.as_reference())
        .context("找不到 PDF Catalog")?;
    let has = doc
        .objects
        .get(&catalog_id)
        .and_then(|obj| obj.as_dict().ok())
        .map(|dict| dict.has(b"Outlines"))
        .unwrap_or(false);
    Ok(has)
}

/// 删除 PDF 的 /Outlines 对象并从 Catalog 移除引用
pub fn remove_pdf_bookmarks(path: &Path) -> Result<()> {
    let temp = TempPathGuard::new(sibling_temp_path(path, "docsy-rm-bookmarks"));
    let mut doc =
        Document::load(path).with_context(|| format!("加载 PDF 失败: {}", path.display()))?;
    remove_outlines(&mut doc)?;
    save_replacing(doc, path, temp.path(), "保存 PDF 失败")
}

fn remove_outlines(doc: &mut Document) -> Result<()> {
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|obj| obj.as_reference())
        .context("找不到 PDF Catalog")?;
    if let Some(Object::Dictionary(catalog)) = doc.objects.get_mut(&catalog_id) {
        catalog.remove(b"Outlines");
    }
    Ok(())
}

fn save_replacing(mut doc: Document, output: &Path, temp: &Path, context: &str) -> Result<()> {
    doc.prune_objects();
    doc.save(temp).with_context(|| context.to_string())?;
    replace_file(temp, output).context("替换 PDF 文件失败")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_test_pdf(path: &Path) {
        use lopdf::content::{Content, Operation};
        use lopdf::Stream;

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

    #[test]
    fn bookmark_roundtrip() {
        let path = temp_named_path("docsy_bookmark_rt", "pdf");
        create_simple_test_pdf(&path);

        let config = BookmarkConfig {
            enabled: true,
            label: "测试书签".to_string(),
            page_index: 0,
        };
        apply_bookmarks(&path, &[config], false).unwrap();

        // Reload and verify
        let doc = Document::load(&path).unwrap();
        let catalog_id = doc
            .trailer
            .get(b"Root")
            .and_then(|obj| obj.as_reference())
            .unwrap();
        let catalog = doc.objects.get(&catalog_id).unwrap().as_dict().unwrap();

        // Catalog must reference Outlines
        let outlines_ref = catalog.get(b"Outlines").unwrap().as_reference().unwrap();
        let outlines = doc.objects.get(&outlines_ref).unwrap().as_dict().unwrap();
        assert_eq!(
            outlines.get(b"Type").unwrap().as_name().unwrap(),
            b"Outlines"
        );
        assert_eq!(outlines.get(b"Count").unwrap().as_i64().unwrap(), 1);

        // First outline item
        let first_ref = outlines.get(b"First").unwrap().as_reference().unwrap();
        let last_ref = outlines.get(b"Last").unwrap().as_reference().unwrap();
        assert_eq!(first_ref, last_ref);

        let item = doc.objects.get(&first_ref).unwrap().as_dict().unwrap();
        let title =
            super::super::artifacts::decode_pdf_string(item.get(b"Title").unwrap()).unwrap();
        assert_eq!(title, "测试书签");
        assert!(item.get(b"Dest").is_ok());
        assert_eq!(
            item.get(b"Parent").unwrap().as_reference().unwrap(),
            outlines_ref
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn failed_replacement_keeps_existing_bookmarks() {
        let path = temp_named_path("docsy_bookmark_atomic", "pdf");
        create_simple_test_pdf(&path);
        apply_bookmarks(
            &path,
            &[BookmarkConfig {
                enabled: true,
                label: "原有书签".to_string(),
                page_index: 0,
            }],
            false,
        )
        .unwrap();

        let result = apply_bookmarks(
            &path,
            &[BookmarkConfig {
                enabled: true,
                label: "越界书签".to_string(),
                page_index: 99,
            }],
            true,
        );
        assert!(result.is_err());
        assert!(has_pdf_bookmarks(&path).unwrap());

        let _ = fs::remove_file(&path);
    }
}
