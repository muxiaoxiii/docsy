use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId};
use std::path::{Path, PathBuf};

use super::page_info::{get_page_infos, PageSize, A4_HEIGHT_PT, A4_WIDTH_PT};

pub fn normalize_pdf_to_a4(input: &Path, _dpi: u32, orientation: &str) -> Result<PathBuf> {
    let input_str = input.to_string_lossy().to_string();
    let pages = get_page_infos(&input_str)?;
    let mut doc = Document::load(input).context("读取待规范化 PDF 失败")?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();

    for (idx, page_id) in page_ids.iter().enumerate() {
        let Some(page) = pages.get(idx) else {
            continue;
        };
        let transform = a4_transform(page, orientation);
        normalize_page_content(&mut doc, *page_id, &transform)?;
        set_page_box(&mut doc, *page_id, transform.page_w, transform.page_h)?;
    }

    let output = temp_named_path("docsy_a4_normalized", "pdf");
    doc.save(&output).context("写入 A4 规范化 PDF 失败")?;
    Ok(output)
}

#[derive(Debug, Clone, Copy)]
struct A4Transform {
    page_w: f32,
    page_h: f32,
    scale: f32,
    translate_x: f32,
    translate_y: f32,
}

fn a4_page_size(source: &PageSize, orientation: &str) -> (f32, f32) {
    match orientation {
        "portrait" => (A4_WIDTH_PT, A4_HEIGHT_PT),
        "landscape" => (A4_HEIGHT_PT, A4_WIDTH_PT),
        _ if source.width_pt > source.height_pt => (A4_HEIGHT_PT, A4_WIDTH_PT),
        _ => (A4_WIDTH_PT, A4_HEIGHT_PT),
    }
}

fn fit_scale_to_a4(source: &PageSize, orientation: &str) -> f32 {
    if source.width_pt <= 0.0 || source.height_pt <= 0.0 {
        return 1.0;
    }
    let (page_w, page_h) = a4_page_size(source, orientation);
    (page_w / source.width_pt)
        .min(page_h / source.height_pt)
        .min(1.0)
}

fn a4_transform(source: &PageSize, orientation: &str) -> A4Transform {
    let (page_w, page_h) = a4_page_size(source, orientation);
    let scale = fit_scale_to_a4(source, orientation);
    let draw_w = source.width_pt * scale;
    let draw_h = source.height_pt * scale;
    A4Transform {
        page_w,
        page_h,
        scale,
        translate_x: (page_w - draw_w) / 2.0,
        translate_y: (page_h - draw_h) / 2.0,
    }
}

fn normalize_page_content(
    doc: &mut Document,
    page_id: ObjectId,
    transform: &A4Transform,
) -> Result<()> {
    let content = match doc.get_and_decode_page_content(page_id) {
        Ok(content) => content,
        Err(_) => return Ok(()),
    };
    if content.operations.is_empty() {
        return Ok(());
    }

    let mut operations = vec![
        Operation::new("q", vec![]),
        Operation::new(
            "cm",
            vec![
                pdf_number(transform.scale),
                pdf_number(0.0),
                pdf_number(0.0),
                pdf_number(transform.scale),
                pdf_number(transform.translate_x),
                pdf_number(transform.translate_y),
            ],
        ),
    ];
    operations.extend(content.operations);
    operations.push(Operation::new("Q", vec![]));

    let encoded = Content { operations }
        .encode()
        .context("编码 A4 规范化内容流失败")?;
    doc.change_page_content(page_id, encoded)
        .context("写回 A4 规范化内容流失败")?;
    Ok(())
}

fn set_page_box(doc: &mut Document, page_id: ObjectId, width: f32, height: f32) -> Result<()> {
    let page = doc
        .get_object_mut(page_id)
        .context("读取 PDF 页面对象失败")?
        .as_dict_mut()
        .context("PDF 页面对象不是字典")?;
    let box_object = Object::Array(vec![
        pdf_number(0.0),
        pdf_number(0.0),
        pdf_number(width),
        pdf_number(height),
    ]);
    page.set("MediaBox", box_object.clone());
    page.set("CropBox", box_object);
    page.remove(b"Rotate");
    Ok(())
}

fn pdf_number(value: f32) -> Object {
    Object::Real(value)
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

    #[test]
    fn a4_normalize_does_not_upscale_small_pages() {
        let source = PageSize {
            width_pt: 300.0,
            height_pt: 400.0,
        };
        assert_eq!(fit_scale_to_a4(&source, "portrait"), 1.0);
    }

    #[test]
    fn a4_normalize_scales_down_large_pages() {
        let source = PageSize {
            width_pt: A4_WIDTH_PT * 2.0,
            height_pt: A4_HEIGHT_PT * 2.0,
        };
        assert!((fit_scale_to_a4(&source, "portrait") - 0.5).abs() < 0.001);
    }
}
