use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId};
use std::path::{Path, PathBuf};

use super::page_info::{get_page_infos, PageSize, A4_HEIGHT_PT, A4_WIDTH_PT};
use super::qpdf;

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
    let optimized = temp_named_path("docsy_a4_normalized_opt", "pdf");
    match qpdf::optimize_to(&output, &optimized) {
        Ok(()) => {
            let _ = std::fs::remove_file(&output);
            Ok(optimized)
        }
        Err(_) => {
            let _ = std::fs::remove_file(&optimized);
            Ok(output)
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct A4Transform {
    page_w: f32,
    page_h: f32,
    matrix: [f32; 6],
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
    let (visual_w, visual_h) = oriented_visual_size(source, orientation);
    if visual_w <= 0.0 || visual_h <= 0.0 {
        return 1.0;
    }
    let (page_w, page_h) = a4_page_size(source, orientation);
    (page_w / visual_w).min(page_h / visual_h).min(1.0)
}

fn a4_transform(source: &PageSize, orientation: &str) -> A4Transform {
    let (page_w, page_h) = a4_page_size(source, orientation);
    let scale = fit_scale_to_a4(source, orientation);
    let (visual_w, visual_h) = oriented_visual_size(source, orientation);
    let draw_w = visual_w * scale;
    let draw_h = visual_h * scale;
    let fit_matrix = [
        scale,
        0.0,
        0.0,
        scale,
        (page_w - draw_w) / 2.0,
        (page_h - draw_h) / 2.0,
    ];
    let rotate_matrix = unrotate_matrix(source);
    let forced_matrix = forced_orientation_matrix(source, orientation);
    A4Transform {
        page_w,
        page_h,
        matrix: multiply_matrix(multiply_matrix(fit_matrix, forced_matrix), rotate_matrix),
    }
}

fn oriented_visual_size(source: &PageSize, orientation: &str) -> (f32, f32) {
    if should_force_quarter_turn(source, orientation) {
        (source.height_pt, source.width_pt)
    } else {
        (source.width_pt, source.height_pt)
    }
}

fn should_force_quarter_turn(source: &PageSize, orientation: &str) -> bool {
    let landscape = source.width_pt > source.height_pt;
    match orientation {
        "portrait" => landscape,
        "landscape" => !landscape,
        _ => false,
    }
}

fn forced_orientation_matrix(source: &PageSize, orientation: &str) -> [f32; 6] {
    let landscape = source.width_pt > source.height_pt;
    match orientation {
        "portrait" if landscape => [0.0, 1.0, -1.0, 0.0, source.height_pt, 0.0],
        "landscape" if !landscape => [0.0, -1.0, 1.0, 0.0, 0.0, source.width_pt],
        _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    }
}

fn unrotate_matrix(source: &PageSize) -> [f32; 6] {
    match source.rotate.rem_euclid(360) {
        90 => [0.0, 1.0, -1.0, 0.0, source.raw_height_pt, 0.0],
        180 => [
            -1.0,
            0.0,
            0.0,
            -1.0,
            source.raw_width_pt,
            source.raw_height_pt,
        ],
        270 => [0.0, -1.0, 1.0, 0.0, 0.0, source.raw_width_pt],
        _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    }
}

fn multiply_matrix(left: [f32; 6], right: [f32; 6]) -> [f32; 6] {
    [
        left[0] * right[0] + left[2] * right[1],
        left[1] * right[0] + left[3] * right[1],
        left[0] * right[2] + left[2] * right[3],
        left[1] * right[2] + left[3] * right[3],
        left[0] * right[4] + left[2] * right[5] + left[4],
        left[1] * right[4] + left[3] * right[5] + left[5],
    ]
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
                pdf_number(transform.matrix[0]),
                pdf_number(transform.matrix[1]),
                pdf_number(transform.matrix[2]),
                pdf_number(transform.matrix[3]),
                pdf_number(transform.matrix[4]),
                pdf_number(transform.matrix[5]),
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
            raw_width_pt: 300.0,
            raw_height_pt: 400.0,
            rotate: 0,
        };
        assert_eq!(fit_scale_to_a4(&source, "portrait"), 1.0);
    }

    #[test]
    fn a4_normalize_scales_down_large_pages() {
        let source = PageSize {
            width_pt: A4_WIDTH_PT * 2.0,
            height_pt: A4_HEIGHT_PT * 2.0,
            raw_width_pt: A4_WIDTH_PT * 2.0,
            raw_height_pt: A4_HEIGHT_PT * 2.0,
            rotate: 0,
        };
        assert!((fit_scale_to_a4(&source, "portrait") - 0.5).abs() < 0.001);
    }

    #[test]
    fn a4_normalize_keeps_rotated_landscape_visual_orientation() {
        let source = PageSize {
            width_pt: 842.0,
            height_pt: 595.0,
            raw_width_pt: 595.0,
            raw_height_pt: 842.0,
            rotate: 90,
        };
        let transform = a4_transform(&source, "auto");

        assert_eq!(
            (transform.page_w, transform.page_h),
            (A4_HEIGHT_PT, A4_WIDTH_PT)
        );
        assert!(transform.matrix[1] > 0.0);
        assert!(transform.matrix[2] < 0.0);
    }

    #[test]
    fn forced_portrait_rotates_landscape_page_before_fit() {
        let source = PageSize {
            width_pt: A4_HEIGHT_PT,
            height_pt: A4_WIDTH_PT,
            raw_width_pt: A4_HEIGHT_PT,
            raw_height_pt: A4_WIDTH_PT,
            rotate: 0,
        };
        let transform = a4_transform(&source, "portrait");

        assert_eq!(
            (transform.page_w, transform.page_h),
            (A4_WIDTH_PT, A4_HEIGHT_PT)
        );
        assert!(transform.matrix[1] > 0.0);
        assert!(transform.matrix[2] < 0.0);
    }

    #[test]
    fn forced_landscape_rotates_portrait_page_before_fit() {
        let source = PageSize {
            width_pt: A4_WIDTH_PT,
            height_pt: A4_HEIGHT_PT,
            raw_width_pt: A4_WIDTH_PT,
            raw_height_pt: A4_HEIGHT_PT,
            rotate: 0,
        };
        let transform = a4_transform(&source, "landscape");

        assert_eq!(
            (transform.page_w, transform.page_h),
            (A4_HEIGHT_PT, A4_WIDTH_PT)
        );
        assert!(transform.matrix[1] < 0.0);
        assert!(transform.matrix[2] > 0.0);
    }
}
