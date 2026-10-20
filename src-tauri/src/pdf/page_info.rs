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

impl PageSize {
    /// Effective PDF `/UserUnit` factor (physical pt per default user space unit).
    pub fn user_unit(&self) -> f32 {
        let unscaled_visual_w = if matches!(self.rotate.rem_euclid(360), 90 | 270) {
            self.raw_height_pt
        } else {
            self.raw_width_pt
        };
        if unscaled_visual_w > 0.0 && self.width_pt > 0.0 {
            let u = self.width_pt / unscaled_visual_w;
            if u.is_finite() && u > 0.0 && (u - 1.0).abs() > 1e-4 {
                return u;
            }
        }
        1.0
    }
}

pub fn get_page_infos(input: &str) -> Result<Vec<PageSize>> {
    let input_path = Path::new(input);
    if let Ok(pages) = get_page_infos_with_pdfcraft(input_path) {
        return Ok(pages);
    }
    if let Ok(pages) = get_page_infos_with_lopdf(input_path) {
        return Ok(pages);
    }
    get_page_infos_with_qpdf(input).with_context(|| {
        format!(
            "无法读取 PDF 页面尺寸（{}），已停止处理以避免按错误 A4 尺寸定位内容",
            input
        )
    })
}

fn get_page_infos_with_pdfcraft(input: &Path) -> Result<Vec<PageSize>> {
    let bytes = std::fs::read(input).context("无法读取 PDF 文件字节")?;
    let doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes))
        .context("pdfcraft 无法解析 PDF")?;
    page_infos_from_pdfcraft_document(&doc)
}

pub(crate) fn page_infos_from_pdfcraft_document(
    doc: &pdfcraft_cos::Document,
) -> Result<Vec<PageSize>> {
    let pages = pdfcraft_organize::walk(doc).context("pdfcraft 无法遍历页面树")?;
    if pages.is_empty() {
        anyhow::bail!("PDF 无页面");
    }

    pages
        .into_iter()
        .enumerate()
        .map(|(idx, (page_ref, attrs))| {
            let page_number = idx + 1;
            let box_obj = attrs
                .get(b"CropBox")
                .or_else(|| attrs.get(b"MediaBox"))
                .map(|v| doc.resolve(v))
                .with_context(|| format!("第 {page_number} 页及其父级均无 CropBox/MediaBox"))?;
            let size = page_size_from_pdfcraft_box(doc, &box_obj)
                .with_context(|| format!("第 {page_number} 页的页面框格式无效"))?;
            // ISO 32000-1 Table 30：/UserUnit 仅属于叶节点 /Page 属性，不可从父节点 /Pages 继承
            let user_unit = doc
                .get(page_ref)
                .as_dict()
                .and_then(|d| d.get(b"UserUnit"))
                .and_then(|v| doc.resolve(v).as_f64())
                .map(|v| v as f32)
                .unwrap_or(1.0);
            let rotate = attrs
                .get(b"Rotate")
                .and_then(|v| doc.resolve(v).as_int())
                .unwrap_or(0)
                .rem_euclid(360) as i32;
            Ok(apply_rotation(scale_by_user_unit(size, user_unit), rotate))
        })
        .collect()
}

fn page_size_from_pdfcraft_box(
    doc: &pdfcraft_cos::Document,
    value: &pdfcraft_cos::Object,
) -> Option<PageSize> {
    let values = value.as_array()?;
    if values.len() < 4 {
        return None;
    }
    let x0 = doc.resolve(&values[0]).as_f64()? as f32;
    let y0 = doc.resolve(&values[1]).as_f64()? as f32;
    let x1 = doc.resolve(&values[2]).as_f64()? as f32;
    let y1 = doc.resolve(&values[3]).as_f64()? as f32;
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

fn get_page_infos_with_qpdf(input: &str) -> Result<Vec<PageSize>> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    // qpdf 的 pages 条目本身不含 CropBox/MediaBox，只有 object 引用；
    // 因此需要同时拉 objects 字典，但禁止内联图像流，避免扫描件整档进内存。
    let output = crate::external::hidden_command(&bin)
        .arg("--json=1")
        .arg("--json-key=pages")
        .arg("--json-key=objects")
        .arg("--json-stream-data=none")
        .arg(input)
        .output()
        .context("执行 qpdf 页面尺寸 JSON 失败")?;

    if !super::qpdf::status_is_success(&output.status) {
        anyhow::bail!(
            "qpdf 读取页面尺寸失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&output)
        );
    }

    let json: Value = serde_json::from_slice(&output.stdout).context("解析 qpdf JSON 失败")?;
    parse_page_sizes(&json).context("qpdf 页面元数据缺少页面框")
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
            // ISO 32000-1 Table 30：/UserUnit 仅属于叶节点 /Page 属性，不可从父节点 /Pages 继承
            let user_unit = doc
                .get_dictionary(page_id)
                .ok()
                .and_then(|d| d.get(b"UserUnit").ok())
                .and_then(|v| resolve_lopdf_object(doc, v))
                .and_then(|value| value.as_float().ok())
                .unwrap_or(1.0);
            let rotate = inherited_page_value(doc, page_id, b"Rotate")
                .and_then(|value| value.as_i64().ok())
                .unwrap_or(0)
                .rem_euclid(360) as i32;
            Ok(apply_rotation(scale_by_user_unit(size, user_unit), rotate))
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
    };
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
    // 兼容两种 qpdf JSON 形态：
    // - 旧：`qpdf: [{...metadata...}, { "obj:N 0 R": {...} }]`
    // - 新（--json=1）：`objects: { "N 0 R": {...} }` 或 `obj:N 0 R`
    let objects_map = json
        .get("objects")
        .and_then(Value::as_object)
        .map(|map| map as &dyn ObjectLookup)
        .or_else(|| {
            json.get("qpdf")
                .and_then(Value::as_array)
                .map(|arr| arr as &dyn ObjectLookup)
        });

    let mut sizes = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let obj_ref = page.get("object").and_then(|v| v.as_str());
        let size = page_size_from_page_entry(page)
            .or_else(|| {
                objects_map
                    .and_then(|lookup| lookup.resolve_page_size(obj_ref?))
            })
            .with_context(|| {
                format!(
                    "第 {} 页缺少 CropBox/MediaBox，无法安全计算 A4 定位",
                    index + 1
                )
            })?;
        sizes.push(size);
    }

    if sizes.is_empty() {
        anyhow::bail!("PDF 无页面");
    }

    Ok(sizes)
}

/// 在 qpdf objects 表中按页面引用解析尺寸（含 /Parent 继承）。
trait ObjectLookup {
    fn object_dict(&self, obj_ref: &str) -> Option<serde_json::Map<String, Value>>;
    fn resolve_page_size(&self, obj_ref: &str) -> Option<PageSize> {
        let mut current_ref = obj_ref.to_string();
        let mut visited = std::collections::HashSet::new();
        let mut crop_size: Option<PageSize> = None;
        let mut media_size: Option<PageSize> = None;
        let mut rotate: Option<i32> = None;
        let mut user_unit: Option<f32> = None;

        for depth in 0..32 {
            if !visited.insert(current_ref.clone()) {
                break;
            }
            let Some(dict) = self.object_dict(&current_ref) else {
                break;
            };
            if crop_size.is_none() {
                crop_size = dict
                    .get("/CropBox")
                    .or_else(|| dict.get("CropBox"))
                    .and_then(page_size_from_box);
            }
            if media_size.is_none() {
                media_size = dict
                    .get("/MediaBox")
                    .or_else(|| dict.get("MediaBox"))
                    .and_then(page_size_from_box);
            }
            if rotate.is_none() {
                rotate = dict
                    .get("/Rotate")
                    .or_else(|| dict.get("Rotate"))
                    .and_then(|v| v.as_i64())
                    .map(|r| r.rem_euclid(360) as i32);
            }
            // ISO 32000-1 Table 30：/UserUnit 仅属于叶节点 /Page 属性（depth == 0），不可从父节点继承
            if depth == 0 && user_unit.is_none() {
                user_unit = dict
                    .get("/UserUnit")
                    .or_else(|| dict.get("UserUnit"))
                    .and_then(|v| v.as_f64())
                    .map(|u| u as f32);
            }
            let Some(parent) = dict
                .get("/Parent")
                .or_else(|| dict.get("Parent"))
                .and_then(reference_string)
            else {
                break;
            };
            current_ref = parent;
        }

        let size = crop_size.or(media_size)?;
        let scaled = scale_by_user_unit(size, user_unit.unwrap_or(1.0));
        Some(apply_rotation(scaled, rotate.unwrap_or(0)))
    }
}

fn reference_string(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => Some(raw.trim_end_matches(" R").trim().to_string()),
        Value::Array(items) if items.len() >= 2 => {
            let num = items[0].as_i64()?;
            let gen = items[1].as_i64().unwrap_or(0);
            Some(format!("{num} {gen}"))
        }
        _ => None,
    }
}

impl ObjectLookup for serde_json::Map<String, Value> {
    fn object_dict(&self, obj_ref: &str) -> Option<serde_json::Map<String, Value>> {
        let trimmed = obj_ref.trim_end_matches(" R").trim();
        for key in [
            trimmed.to_string(),
            format!("{trimmed} R"),
            format!("obj:{trimmed}"),
            format!("obj:{trimmed} R"),
        ] {
            if let Some(entry) = self.get(&key) {
                if let Some(dict) = dict_value_of(entry) {
                    return Some(dict);
                }
            }
        }
        None
    }
}

impl ObjectLookup for Vec<Value> {
    fn object_dict(&self, obj_ref: &str) -> Option<serde_json::Map<String, Value>> {
        for obj in self {
            // qpdf emits a metadata object before the object map.
            if let Some(dict) = obj.as_object().and_then(|map| {
                if map.contains_key("jsonversion") || map.contains_key("pdfversion") {
                    return None;
                }
                map.get(obj_ref)
                    .or_else(|| map.get(format!("obj:{obj_ref}").as_str()))
                    .or_else(|| {
                        let trimmed = obj_ref.trim_end_matches(" R").trim();
                        map.get(format!("{trimmed} R").as_str())
                    })
                    .and_then(dict_value_of)
            }) {
                return Some(dict);
            }
        }
        None
    }
}

fn dict_value_of(entry: &Value) -> Option<serde_json::Map<String, Value>> {
    let map = entry.as_object()?;
    if map.contains_key("/Type") || map.contains_key("/MediaBox") || map.contains_key("/CropBox") {
        return Some(map.clone());
    }
    map.get("value").and_then(Value::as_object).cloned()
}

fn page_size_from_page_entry(page: &Value) -> Option<PageSize> {
    let size = page
        .get("cropBox")
        .or_else(|| page.get("CropBox"))
        .or_else(|| page.get("mediaBox"))
        .or_else(|| page.get("MediaBox"))
        .and_then(page_size_from_box)?;
    let user_unit = page
        .get("userUnit")
        .or_else(|| page.get("UserUnit"))
        .or_else(|| page.get("/UserUnit"))
        .and_then(|v| v.as_f64())
        .map(|u| u as f32)
        .unwrap_or(1.0);
    let rotate = page
        .get("rotate")
        .or_else(|| page.get("Rotate"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .rem_euclid(360);
    Some(apply_rotation(
        scale_by_user_unit(size, user_unit),
        rotate as i32,
    ))
}

fn scale_by_user_unit(size: PageSize, user_unit: f32) -> PageSize {
    let u = if user_unit.is_finite() && user_unit > 0.0 {
        user_unit
    } else {
        1.0
    };
    if (u - 1.0).abs() <= f32::EPSILON {
        return size;
    }
    // 物理显示尺寸 (width_pt / height_pt) 乘上 UserUnit；
    // raw_width_pt / raw_height_pt / box_x0 / box_y0 保持默认用户空间坐标（未乘 UserUnit），
    // 以免破坏 normalize 内容流 CTM 与裁剪起点平移。
    PageSize {
        width_pt: size.width_pt * u,
        height_pt: size.height_pt * u,
        raw_width_pt: size.raw_width_pt,
        raw_height_pt: size.raw_height_pt,
        box_x0: size.box_x0,
        box_y0: size.box_y0,
        rotate: size.rotate,
    }
}

fn apply_rotation(size: PageSize, rotate: i32) -> PageSize {
    if rotate == 90 || rotate == 270 {
        PageSize {
            width_pt: size.height_pt,
            height_pt: size.width_pt,
            raw_width_pt: size.raw_width_pt,
            raw_height_pt: size.raw_height_pt,
            box_x0: size.box_x0,
            box_y0: size.box_y0,
            rotate,
        }
    } else {
        PageSize {
            width_pt: size.width_pt,
            height_pt: size.height_pt,
            raw_width_pt: size.raw_width_pt,
            raw_height_pt: size.raw_height_pt,
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
    fn parses_modern_objects_map_layout() {
        // qpdf --json=1 实际输出：objects 顶层映射，键为 "N 0 R"，无 obj: 前缀。
        let value = json!({
            "pages": [{ "object": "4 0 R" }],
            "objects": {
                "4 0 R": {
                    "value": { "/Type": "/Page", "/MediaBox": [0.0, 0.0, 612, 792], "/Parent": "2 0 R" }
                }
            }
        });
        let pages = parse_page_sizes(&value).expect("modern objects map should parse");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].width_pt, 612.0);
        assert_eq!(pages[0].height_pt, 792.0);
    }

    #[test]
    fn resolves_inherited_media_box_from_parent() {
        let value = json!({
            "pages": [{ "object": "3 0 R" }],
            "objects": {
                "3 0 R": { "value": { "/Type": "/Page", "/Parent": "2 0 R" } },
                "2 0 R": { "value": { "/Type": "/Pages", "/MediaBox": [0, 0, 595.28, 841.89] } }
            }
        });
        let pages = parse_page_sizes(&value).expect("inherited box should resolve");
        assert_eq!(pages[0].width_pt, 595.28);
        assert_eq!(pages[0].height_pt, 841.89);
    }

    #[test]
    fn real_qpdf_pages_do_not_embed_boxes_so_objects_are_required() {
        // 契约测试：真实 qpdf pages 条目不含 mediaBox；必须能从 objects 解析。
        let value = json!({
            "version": 1,
            "pages": [{
                "contents": [],
                "images": [],
                "label": null,
                "object": "4 0 R",
                "outlines": [],
                "pageposfrom1": 1
            }],
            "objects": {
                "4 0 R": {
                    "value": { "/MediaBox": [0.0, 0.0, 612, 792], "/Type": "/Page" }
                }
            }
        });
        let pages = parse_page_sizes(&value).expect("must resolve via objects");
        assert_eq!(pages[0].width_pt, 612.0);
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

    #[test]
    fn user_unit_scales_display_dimensions_only_on_leaf_page_and_preserves_raw_user_space() {
        let leaf_json = json!({
            "pages": [{ "object": "3 0 R" }],
            "objects": {
                "3 0 R": {
                    "value": {
                        "/Type": "/Page",
                        "/Parent": "2 0 R",
                        "/UserUnit": 2.0,
                        "/CropBox": [20.0, 30.0, 180.0, 190.0]
                    }
                },
                "2 0 R": {
                    "value": {
                        "/Type": "/Pages",
                        "/MediaBox": [0.0, 0.0, 200.0, 220.0]
                    }
                }
            }
        });
        let leaf_pages = parse_page_sizes(&leaf_json).expect("leaf UserUnit should resolve");
        assert_eq!(leaf_pages[0].width_pt, 320.0);
        assert_eq!(leaf_pages[0].height_pt, 320.0);
        assert_eq!(leaf_pages[0].raw_width_pt, 160.0);
        assert_eq!(leaf_pages[0].raw_height_pt, 160.0);
        assert_eq!(leaf_pages[0].box_x0, 20.0);
        assert_eq!(leaf_pages[0].box_y0, 30.0);
        assert!((leaf_pages[0].user_unit() - 2.0).abs() < 1e-4);

        let parent_json = json!({
            "pages": [{ "object": "3 0 R" }],
            "objects": {
                "3 0 R": {
                    "value": {
                        "/Type": "/Page",
                        "/Parent": "2 0 R",
                        "/CropBox": [20.0, 30.0, 180.0, 190.0]
                    }
                },
                "2 0 R": {
                    "value": {
                        "/Type": "/Pages",
                        "/UserUnit": 2.0,
                        "/MediaBox": [0.0, 0.0, 200.0, 220.0]
                    }
                }
            }
        });
        let parent_pages =
            parse_page_sizes(&parent_json).expect("parent UserUnit must not inherit");
        assert_eq!(parent_pages[0].width_pt, 160.0);
        assert_eq!(parent_pages[0].height_pt, 160.0);
        assert_eq!(parent_pages[0].raw_width_pt, 160.0);
        assert_eq!(parent_pages[0].raw_height_pt, 160.0);
        assert_eq!(parent_pages[0].box_x0, 20.0);
        assert_eq!(parent_pages[0].box_y0, 30.0);
        assert!((parent_pages[0].user_unit() - 1.0).abs() < 1e-4);
    }
}
