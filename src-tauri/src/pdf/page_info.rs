use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

use crate::external::ExternalTool;

pub const A4_WIDTH_PT: f32 = 595.28;
pub const A4_HEIGHT_PT: f32 = 841.89;

#[derive(Debug, Clone)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
    pub raw_width_pt: f32,
    pub raw_height_pt: f32,
    /// Lower-left corner of the source CropBox/MediaBox in raw page space.
    pub box_x0: f32,
    pub box_y0: f32,
    pub rotate: i32,
}

pub fn get_page_infos(input: &str) -> Result<Vec<PageSize>> {
    match get_page_infos_with_qpdf(input) {
        Ok(pages) => Ok(pages),
        Err(qpdf_error) => get_page_infos_with_lopdf(Path::new(input)).with_context(|| {
            format!(
                "无法读取 PDF 页面尺寸，已停止处理以避免按错误 A4 尺寸定位内容；qpdf 解析失败：{qpdf_error:#}"
            )
        }),
    }
}

fn get_page_infos_with_qpdf(input: &str) -> Result<Vec<PageSize>> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let output = crate::external::hidden_command(&bin)
        .arg("--json")
        .arg(input)
        .output()
        .context("执行 qpdf --json 失败")?;

    if !super::qpdf::status_is_success(&output.status) {
        anyhow::bail!(
            "qpdf --json 失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&output)
        );
    }

    let json: Value = serde_json::from_slice(&output.stdout).context("解析 qpdf JSON 失败")?;
    parse_page_sizes(&json)
}

fn get_page_infos_with_lopdf(input: &Path) -> Result<Vec<PageSize>> {
    let doc = Document::load(input).context("lopdf 无法读取 PDF")?;
    page_infos_from_lopdf_document(&doc)
}

fn page_infos_from_lopdf_document(doc: &Document) -> Result<Vec<PageSize>> {
    let pages = doc.get_pages();
    if pages.is_empty() {
        anyhow::bail!("PDF 无页面");
    }

    pages
        .into_iter()
        .map(|(page_number, page_id)| {
            let box_value = inherited_page_value(doc, page_id, b"CropBox")
                .or_else(|| inherited_page_value(doc, page_id, b"MediaBox"))
                .with_context(|| format!("第 {page_number} 页及其父级均无 CropBox/MediaBox"))?;
            let size = page_size_from_lopdf_box(&box_value)
                .with_context(|| format!("第 {page_number} 页的页面框格式无效"))?;
            let rotate = inherited_page_value(doc, page_id, b"Rotate")
                .and_then(|value| value.as_i64().ok())
                .unwrap_or(0)
                .rem_euclid(360) as i32;
            Ok(apply_rotation(size, rotate))
        })
        .collect()
}

fn inherited_page_value(doc: &Document, start: ObjectId, key: &[u8]) -> Option<Object> {
    let mut current = Some(start);
    let mut visited = HashSet::new();
    while let Some(object_id) = current {
        if !visited.insert(object_id) {
            return None;
        }
        let dictionary = doc.get_dictionary(object_id).ok()?;
        if let Ok(value) = dictionary.get(key) {
            return resolve_lopdf_object(doc, value);
        }
        current = dictionary
            .get(b"Parent")
            .ok()
            .and_then(|value| value.as_reference().ok());
    }
    None
}

fn resolve_lopdf_object(doc: &Document, value: &Object) -> Option<Object> {
    let mut current = value.clone();
    let mut visited = HashSet::new();
    for _ in 0..32 {
        let Object::Reference(object_id) = current else {
            return Some(current);
        };
        if !visited.insert(object_id) {
            return None;
        }
        current = doc.get_object(object_id).ok()?.clone();
    }
    None
}

fn page_size_from_lopdf_box(value: &Object) -> Option<PageSize> {
    let Object::Array(values) = value else {
        return None;
    };
    if values.len() < 4 {
        return None;
    }
    let x0 = values[0].as_float().ok()?;
    let y0 = values[1].as_float().ok()?;
    let x1 = values[2].as_float().ok()?;
    let y1 = values[3].as_float().ok()?;
    let width = (x1 - x0).abs();
    let height = (y1 - y0).abs();
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(PageSize {
        width_pt: width,
        height_pt: height,
        raw_width_pt: width,
        raw_height_pt: height,
        box_x0: x0.min(x1),
        box_y0: y0.min(y1),
        rotate: 0,
    })
}

pub(crate) fn parse_page_sizes(json: &Value) -> Result<Vec<PageSize>> {
    let pages = json
        .get("pages")
        .and_then(|v| v.as_array())
        .context("qpdf JSON 中无 pages 数组")?;
    let objects = json.get("qpdf").and_then(|v| v.as_array());

    let mut sizes = Vec::new();
    for page in pages {
        let obj_ref = page.get("object").and_then(|v| v.as_str());
        let size = page_size_from_page_entry(page)
            .or_else(|| obj_ref.and_then(|r| resolve_page_size(objects, r)))
            .with_context(|| "无法读取 PDF 页面尺寸，已停止处理以避免按错误 A4 尺寸定位内容")?;
        sizes.push(size);
    }

    if sizes.is_empty() {
        anyhow::bail!("PDF 无页面");
    }

    Ok(sizes)
}

fn page_size_from_page_entry(page: &Value) -> Option<PageSize> {
    let size = page
        .get("cropBox")
        .or_else(|| page.get("CropBox"))
        .or_else(|| page.get("mediaBox"))
        .or_else(|| page.get("MediaBox"))
        .and_then(page_size_from_box)?;
    let rotate = page
        .get("rotate")
        .or_else(|| page.get("Rotate"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .rem_euclid(360);
    Some(apply_rotation(size, rotate as i32))
}

fn resolve_page_size(objects: Option<&Vec<Value>>, obj_ref: &str) -> Option<PageSize> {
    let objects = objects?;

    for obj in objects {
        // qpdf emits a metadata object before the object map. A missing page
        // key in that first entry is normal, not a reason to abandon the
        // remaining object maps.
        let Some(page_obj) = obj
            .get(obj_ref)
            .or_else(|| obj.get(format!("obj:{obj_ref}").as_str()))
        else {
            continue;
        };
        let Some(value) = page_obj.get("value") else {
            continue;
        };
        let Some(box_value) = value
            .get("/CropBox")
            .or_else(|| value.get("CropBox"))
            .or_else(|| value.get("/MediaBox"))
            .or_else(|| value.get("MediaBox"))
        else {
            continue;
        };
        if let Some(size) = page_size_from_box(box_value) {
            let rotate = value
                .get("/Rotate")
                .or_else(|| value.get("Rotate"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .rem_euclid(360) as i32;
            return Some(apply_rotation(size, rotate));
        }
    }

    None
}

fn apply_rotation(size: PageSize, rotate: i32) -> PageSize {
    if rotate == 90 || rotate == 270 {
        PageSize {
            width_pt: size.height_pt,
            height_pt: size.width_pt,
            raw_width_pt: size.width_pt,
            raw_height_pt: size.height_pt,
            box_x0: size.box_x0,
            box_y0: size.box_y0,
            rotate,
        }
    } else {
        PageSize {
            width_pt: size.width_pt,
            height_pt: size.height_pt,
            raw_width_pt: size.width_pt,
            raw_height_pt: size.height_pt,
            box_x0: size.box_x0,
            box_y0: size.box_y0,
            rotate,
        }
    }
}

fn page_size_from_box(value: &Value) -> Option<PageSize> {
    let arr = value.as_array()?;
    if arr.len() < 4 {
        return None;
    }
    let x0 = arr[0].as_f64()? as f32;
    let y0 = arr[1].as_f64()? as f32;
    let x1 = arr[2].as_f64()? as f32;
    let y1 = arr[3].as_f64()? as f32;
    Some(PageSize {
        width_pt: (x1 - x0).abs(),
        height_pt: (y1 - y0).abs(),
        raw_width_pt: (x1 - x0).abs(),
        raw_height_pt: (y1 - y0).abs(),
        box_x0: x0.min(x1),
        box_y0: y0.min(y1),
        rotate: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use serde_json::json;

    fn document_with_inherited_page_box() -> Document {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.add_object(dictionary! {});
        let inherited_crop_box = doc.add_object(Object::Array(vec![
            10.into(),
            20.into(),
            510.into(),
            720.into(),
        ]));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            // The inherited CropBox must take precedence over this local
            // MediaBox, matching the PDF page attribute rules.
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![Object::Reference(page_id)],
                "Count" => 1,
                "CropBox" => inherited_crop_box,
                "Rotate" => 90,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc
    }

    #[test]
    fn parses_qpdf_page_sizes() {
        let value = json!({
            "pages": [{ "object": "3 0 R" }],
            "qpdf": [{
                "obj:3 0 R": {
                    "value": { "/MediaBox": [0, 0, 595.28, 841.89] }
                }
            }]
        });
        let pages = parse_page_sizes(&value).expect("page sizes should parse");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].width_pt, 595.28);
        assert_eq!(pages[0].box_x0, 0.0);
    }

    #[test]
    fn parses_rotated_page_size_as_visual_size() {
        let value = json!({
            "pages": [{
                "object": "3 0 R",
                "mediaBox": [0, 0, 595.28, 841.89],
                "rotate": 90
            }]
        });
        let pages = parse_page_sizes(&value).expect("page sizes should parse");
        assert_eq!(pages[0].width_pt, 841.89);
        assert_eq!(pages[0].height_pt, 595.28);
        assert_eq!(pages[0].raw_width_pt, 595.28);
        assert_eq!(pages[0].raw_height_pt, 841.89);
        assert_eq!(pages[0].rotate, 90);
    }

    #[test]
    fn preserves_non_zero_page_box_origin() {
        let value = json!({
            "pages": [{
                "object": "3 0 R",
                "cropBox": [18, 24, 613.28, 865.89]
            }]
        });
        let pages = parse_page_sizes(&value).expect("page sizes should parse");
        assert_eq!(pages[0].width_pt, 595.28);
        assert_eq!(pages[0].height_pt, 841.89);
        assert_eq!(pages[0].box_x0, 18.0);
        assert_eq!(pages[0].box_y0, 24.0);
    }

    #[test]
    fn resolves_rotated_page_size_from_page_object() {
        let value = json!({
            "pages": [{ "object": "3 0 R" }],
            "qpdf": [{
                "obj:3 0 R": {
                    "value": { "/MediaBox": [0, 0, 595.28, 841.89], "/Rotate": 270 }
                }
            }]
        });
        let pages = parse_page_sizes(&value).expect("page sizes should parse");
        assert_eq!(pages[0].width_pt, 841.89);
        assert_eq!(pages[0].height_pt, 595.28);
        assert_eq!(pages[0].rotate, 270);
    }

    #[test]
    fn skips_qpdf_metadata_before_object_map() {
        let value = json!({
            "pages": [{ "object": "5 0 R" }],
            "qpdf": [
                { "jsonversion": 2 },
                {
                    "obj:5 0 R": {
                        "value": { "/MediaBox": [0, 0, 612, 792] }
                    }
                }
            ]
        });
        let pages = parse_page_sizes(&value).expect("page size should be read after metadata");
        assert_eq!(pages[0].width_pt, 612.0);
        assert_eq!(pages[0].height_pt, 792.0);
    }

    #[test]
    fn lopdf_fallback_resolves_inherited_and_indirect_page_attributes() {
        let doc = document_with_inherited_page_box();

        let pages = page_infos_from_lopdf_document(&doc).expect("inherited box should resolve");

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].raw_width_pt, 500.0);
        assert_eq!(pages[0].raw_height_pt, 700.0);
        assert_eq!(pages[0].width_pt, 700.0);
        assert_eq!(pages[0].height_pt, 500.0);
        assert_eq!(pages[0].box_x0, 10.0);
        assert_eq!(pages[0].box_y0, 20.0);
        assert_eq!(pages[0].rotate, 90);
    }
}
