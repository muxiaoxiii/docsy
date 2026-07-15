use anyhow::{Context, Result};
use serde_json::Value;

use crate::external::ExternalTool;

pub const A4_WIDTH_PT: f32 = 595.28;
pub const A4_HEIGHT_PT: f32 = 841.89;

#[derive(Debug, Clone)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
    pub raw_width_pt: f32,
    pub raw_height_pt: f32,
    pub rotate: i32,
}

pub fn get_page_infos(input: &str) -> Result<Vec<PageSize>> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let output = std::process::Command::new(&bin)
        .arg("--json")
        .arg(input)
        .output()
        .context("执行 qpdf --json 失败")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("qpdf --json 失败: {}", stderr.trim());
    }

    let json: Value = serde_json::from_slice(&output.stdout).context("解析 qpdf JSON 失败")?;
    parse_page_sizes(&json)
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
            .unwrap_or(PageSize {
                width_pt: A4_WIDTH_PT,
                height_pt: A4_HEIGHT_PT,
                raw_width_pt: A4_WIDTH_PT,
                raw_height_pt: A4_HEIGHT_PT,
                rotate: 0,
            });
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
        let page_obj = obj
            .get(obj_ref)
            .or_else(|| obj.get(format!("obj:{obj_ref}").as_str()))?;
        let value = page_obj.get("value")?;
        let box_value = value
            .get("/CropBox")
            .or_else(|| value.get("CropBox"))
            .or_else(|| value.get("/MediaBox"))
            .or_else(|| value.get("MediaBox"))?;
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
            rotate,
        }
    } else {
        PageSize {
            width_pt: size.width_pt,
            height_pt: size.height_pt,
            raw_width_pt: size.width_pt,
            raw_height_pt: size.height_pt,
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
        rotate: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
