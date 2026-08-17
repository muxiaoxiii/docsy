use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::path::{Path, PathBuf};

use super::page_info::{get_page_infos, PageSize, A4_HEIGHT_PT, A4_WIDTH_PT};
use super::temp_named_path;

pub fn normalize_pdf_to_a4(
    input: &Path,
    _dpi: u32,
    orientation: &str,
    content_rotation: &str,
    content_margin_mm: f32,
) -> Result<PathBuf> {
    let input_str = input.to_string_lossy().to_string();
    let pages = get_page_infos(&input_str)?;
    let mut doc = Document::load(input).context("读取待规范化 PDF 失败")?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();

    for (idx, page_id) in page_ids.iter().enumerate() {
        let Some(page) = pages.get(idx) else {
            continue;
        };
        let transform = a4_transform(
            page,
            orientation,
            content_rotation,
            mm_to_pt(content_margin_mm),
        );
        normalize_page_content(&mut doc, *page_id, &transform)?;
        set_page_box(
            &mut doc,
            *page_id,
            transform.page_w,
            transform.page_h,
            transform.preserve_rotation,
        )?;
    }

    let output = temp_named_path("docsy_a4_normalized", "pdf");
    doc.save(&output).context("写入 A4 规范化 PDF 失败")?;
    // The caller immediately feeds this file into qpdf overlay. Recompressing
    // large scan streams here would decode the same images twice and does not
    // improve coordinate correctness or PDF validity.
    Ok(output)
}

#[derive(Debug, Clone, Copy)]
struct A4Transform {
    page_w: f32,
    page_h: f32,
    matrix: [f32; 6],
    preserve_rotation: bool,
}

fn a4_visual_page_size(source: &PageSize, orientation: &str) -> (f32, f32) {
    match orientation {
        "portrait" => (A4_WIDTH_PT, A4_HEIGHT_PT),
        "landscape" => (A4_HEIGHT_PT, A4_WIDTH_PT),
        "preserve" if source.width_pt > source.height_pt => (A4_HEIGHT_PT, A4_WIDTH_PT),
        "preserve" => (A4_WIDTH_PT, A4_HEIGHT_PT),
        _ if source.width_pt > source.height_pt => (A4_HEIGHT_PT, A4_WIDTH_PT),
        _ => (A4_WIDTH_PT, A4_HEIGHT_PT),
    }
}

fn fit_scale(source_w: f32, source_h: f32, page_w: f32, page_h: f32) -> f32 {
    if source_w <= 0.0 || source_h <= 0.0 {
        return 1.0;
    }
    (page_w / source_w).min(page_h / source_h).min(1.0)
}

fn a4_transform(
    source: &PageSize,
    orientation: &str,
    content_rotation: &str,
    content_margin_pt: f32,
) -> A4Transform {
    match content_rotation {
        "auto" | "clockwise" | "counterclockwise" | "180" => {
            rotated_content_transform(source, orientation, content_rotation, content_margin_pt)
        }
        _ => unrotated_content_transform(source, orientation, content_margin_pt),
    }
}

/// Change only the A4 paper size while retaining the page's current visual
/// content orientation. `/Rotate` remains intact; for quarter-turned pages the
/// raw MediaBox is therefore the inverse orientation of the requested visual
/// paper.
fn unrotated_content_transform(
    source: &PageSize,
    orientation: &str,
    content_margin_pt: f32,
) -> A4Transform {
    let (visual_page_w, visual_page_h) = a4_visual_page_size(source, orientation);
    let quarter_turned = matches!(source.rotate.rem_euclid(360), 90 | 270);
    let (page_w, page_h) = if quarter_turned {
        (visual_page_h, visual_page_w)
    } else {
        (visual_page_w, visual_page_h)
    };
    let raw_w = source.raw_width_pt.max(0.0);
    let raw_h = source.raw_height_pt.max(0.0);
    let (safe_w, safe_h) = safe_content_size(page_w, page_h, content_margin_pt);
    let scale = fit_scale(raw_w, raw_h, safe_w, safe_h);
    let fit_matrix = [
        scale,
        0.0,
        0.0,
        scale,
        (page_w - raw_w * scale) / 2.0,
        (page_h - raw_h * scale) / 2.0,
    ];
    let origin_matrix = [1.0, 0.0, 0.0, 1.0, -source.box_x0, -source.box_y0];
    A4Transform {
        page_w,
        page_h,
        matrix: multiply_matrix(fit_matrix, origin_matrix),
        preserve_rotation: true,
    }
}

fn rotated_content_transform(
    source: &PageSize,
    orientation: &str,
    requested_rotation: &str,
    content_margin_pt: f32,
) -> A4Transform {
    let (page_w, page_h) = a4_visual_page_size(source, orientation);
    let (safe_w, safe_h) = safe_content_size(page_w, page_h, content_margin_pt);
    let rotation = if requested_rotation == "auto" {
        let normal_scale = fit_scale(source.width_pt, source.height_pt, safe_w, safe_h);
        let quarter_scale = fit_scale(source.height_pt, source.width_pt, safe_w, safe_h);
        if quarter_scale > normal_scale + 0.0001 {
            "clockwise"
        } else {
            "none"
        }
    } else {
        requested_rotation
    };
    let quarter_turn = matches!(rotation, "clockwise" | "counterclockwise");
    let (content_w, content_h) = if quarter_turn {
        (source.height_pt, source.width_pt)
    } else {
        (source.width_pt, source.height_pt)
    };
    let scale = fit_scale(content_w, content_h, safe_w, safe_h);
    let fit_matrix = [
        scale,
        0.0,
        0.0,
        scale,
        (page_w - content_w * scale) / 2.0,
        (page_h - content_h * scale) / 2.0,
    ];
    let user_rotation = content_rotation_matrix(source.width_pt, source.height_pt, rotation);
    let source_rotation = unrotate_matrix(source);
    let origin_matrix = [1.0, 0.0, 0.0, 1.0, -source.box_x0, -source.box_y0];
    A4Transform {
        page_w,
        page_h,
        matrix: multiply_matrix(
            multiply_matrix(multiply_matrix(fit_matrix, user_rotation), source_rotation),
            origin_matrix,
        ),
        preserve_rotation: false,
    }
}

fn safe_content_size(page_w: f32, page_h: f32, requested_margin_pt: f32) -> (f32, f32) {
    let margin = if requested_margin_pt.is_finite() {
        requested_margin_pt.max(0.0)
    } else {
        0.0
    };
    (
        (page_w - margin * 2.0).max(1.0),
        (page_h - margin * 2.0).max(1.0),
    )
}

fn mm_to_pt(value: f32) -> f32 {
    value * 72.0 / 25.4
}

fn content_rotation_matrix(width: f32, height: f32, rotation: &str) -> [f32; 6] {
    match rotation {
        "clockwise" => [0.0, -1.0, 1.0, 0.0, 0.0, width],
        "counterclockwise" => [0.0, 1.0, -1.0, 0.0, height, 0.0],
        "180" => [-1.0, 0.0, 0.0, -1.0, width, height],
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
    let contents = doc
        .get_object(page_id)
        .context("读取 PDF 页面对象失败")?
        .as_dict()
        .context("PDF 页面对象不是字典")?
        .get(b"Contents")
        .ok()
        .cloned();
    let Some(contents) = contents else {
        return Ok(());
    };

    let prefix = Content {
        operations: vec![
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
        ],
    }
    .encode()
    .context("编码 A4 规范化前置内容流失败")?;
    let suffix = Content {
        operations: vec![Operation::new("Q", vec![])],
    }
    .encode()
    .context("编码 A4 规范化后置内容流失败")?;
    let prefix_id = doc.add_object(Stream::new(Dictionary::new(), prefix));
    let suffix_id = doc.add_object(Stream::new(Dictionary::new(), suffix));

    let mut wrapped = vec![Object::Reference(prefix_id)];
    match contents {
        Object::Array(items) => wrapped.extend(
            items
                .into_iter()
                .filter(|item| !matches!(item, Object::Null)),
        ),
        Object::Null => {}
        Object::Stream(stream) => wrapped.push(Object::Reference(doc.add_object(stream))),
        other => wrapped.push(other),
    }
    wrapped.push(Object::Reference(suffix_id));

    doc.get_object_mut(page_id)
        .context("读取 PDF 页面对象失败")?
        .as_dict_mut()
        .context("PDF 页面对象不是字典")?
        .set("Contents", Object::Array(wrapped));
    Ok(())
}

fn set_page_box(
    doc: &mut Document,
    page_id: ObjectId,
    width: f32,
    height: f32,
    preserve_rotation: bool,
) -> Result<()> {
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
    // qpdf sizes overlay/underlay forms against the destination page boxes.
    // Leaving a source-sized TrimBox/ArtBox/BleedBox while only enlarging the
    // MediaBox and CropBox makes an A4 overlay get scaled back into the old
    // content area. A normalized page must expose one consistent A4 box.
    for name in ["MediaBox", "CropBox", "TrimBox", "BleedBox", "ArtBox"] {
        page.set(name, box_object.clone());
    }
    if !preserve_rotation {
        // `/Rotate` is inheritable. Removing a page-local value could expose
        // a rotation from the parent page tree, so forced directions must
        // explicitly reset it.
        page.set("Rotate", 0);
    }
    Ok(())
}

fn pdf_number(value: f32) -> Object {
    Object::Real(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::ExternalTool;
    use crate::pdf::header_footer::OverlayTextConfig;
    use crate::pdf::overlay_font::mm_to_pt;
    use crate::pdf::overlay_pdf::build_overlay_pdf;
    use lopdf::dictionary;

    fn page_size(width: f32, height: f32) -> PageSize {
        PageSize {
            width_pt: width,
            height_pt: height,
            raw_width_pt: width,
            raw_height_pt: height,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        }
    }

    #[test]
    fn a4_normalize_does_not_upscale_small_pages() {
        let source = PageSize {
            width_pt: 300.0,
            height_pt: 400.0,
            raw_width_pt: 300.0,
            raw_height_pt: 400.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        };
        assert_eq!(
            fit_scale(source.width_pt, source.height_pt, A4_WIDTH_PT, A4_HEIGHT_PT),
            1.0
        );
    }

    #[test]
    fn a4_normalize_scales_down_large_pages() {
        let source = PageSize {
            width_pt: A4_WIDTH_PT * 2.0,
            height_pt: A4_HEIGHT_PT * 2.0,
            raw_width_pt: A4_WIDTH_PT * 2.0,
            raw_height_pt: A4_HEIGHT_PT * 2.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        };
        assert!(
            (fit_scale(source.width_pt, source.height_pt, A4_WIDTH_PT, A4_HEIGHT_PT) - 0.5).abs()
                < 0.001
        );
    }

    #[test]
    fn safety_margin_keeps_wide_content_away_from_a4_edges() {
        let margin = mm_to_pt(10.0);
        let source = page_size(A4_WIDTH_PT, 120.0);

        let transform = a4_transform(&source, "portrait", "none", margin);

        let expected_scale = (A4_WIDTH_PT - margin * 2.0) / A4_WIDTH_PT;
        assert!((transform.matrix[0] - expected_scale).abs() < 0.001);
        assert!((transform.matrix[4] - margin).abs() < 0.01);
        assert!(
            (A4_WIDTH_PT - (transform.matrix[4] + source.width_pt * expected_scale) - margin).abs()
                < 0.01
        );
    }

    #[test]
    fn a4_normalize_keeps_rotated_landscape_visual_orientation() {
        let source = PageSize {
            width_pt: 842.0,
            height_pt: 595.0,
            raw_width_pt: 595.0,
            raw_height_pt: 842.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 90,
        };
        let transform = a4_transform(&source, "preserve", "none", 0.0);

        assert_eq!(
            (transform.page_w, transform.page_h),
            (A4_WIDTH_PT, A4_HEIGHT_PT)
        );
        assert!(transform.preserve_rotation);
        assert!(transform.matrix[1].abs() < f32::EPSILON);
        assert!(transform.matrix[2].abs() < f32::EPSILON);
    }

    #[test]
    fn preserve_mode_keeps_page_rotation_entry() {
        let mut doc = Document::with_version("1.7");
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
            "Rotate" => 90,
        });

        set_page_box(&mut doc, page_id, A4_WIDTH_PT, A4_HEIGHT_PT, true).unwrap();

        let page = doc.get_dictionary(page_id).unwrap();
        assert_eq!(page.get(b"Rotate").unwrap().as_i64().unwrap(), 90);
    }

    #[test]
    fn explicit_content_rotation_resets_page_rotation_entry() {
        let mut doc = Document::with_version("1.7");
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
            "Rotate" => 90,
        });

        set_page_box(&mut doc, page_id, A4_WIDTH_PT, A4_HEIGHT_PT, false).unwrap();

        let page = doc.get_dictionary(page_id).unwrap();
        assert_eq!(page.get(b"Rotate").unwrap().as_i64().unwrap(), 0);
    }

    #[test]
    fn portrait_paper_does_not_rotate_landscape_content_by_default() {
        let source = PageSize {
            width_pt: A4_HEIGHT_PT,
            height_pt: A4_WIDTH_PT,
            raw_width_pt: A4_HEIGHT_PT,
            raw_height_pt: A4_WIDTH_PT,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        };
        let transform = a4_transform(&source, "portrait", "none", 0.0);

        assert_eq!(
            (transform.page_w, transform.page_h),
            (A4_WIDTH_PT, A4_HEIGHT_PT)
        );
        assert!(transform.preserve_rotation);
        assert!(transform.matrix[1].abs() < f32::EPSILON);
        assert!(transform.matrix[2].abs() < f32::EPSILON);
    }

    #[test]
    fn landscape_paper_does_not_rotate_portrait_content_by_default() {
        let source = PageSize {
            width_pt: A4_WIDTH_PT,
            height_pt: A4_HEIGHT_PT,
            raw_width_pt: A4_WIDTH_PT,
            raw_height_pt: A4_HEIGHT_PT,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        };
        let transform = a4_transform(&source, "landscape", "none", 0.0);

        assert_eq!(
            (transform.page_w, transform.page_h),
            (A4_HEIGHT_PT, A4_WIDTH_PT)
        );
        assert!(transform.preserve_rotation);
        assert!(transform.matrix[1].abs() < f32::EPSILON);
        assert!(transform.matrix[2].abs() < f32::EPSILON);
    }

    #[test]
    fn automatic_rotation_is_opt_in_and_uses_the_better_fit() {
        let source = page_size(1000.0, 200.0);

        let transform = a4_transform(&source, "portrait", "auto", 0.0);

        assert!(!transform.preserve_rotation);
        assert!(transform.matrix[1] < 0.0);
        assert!(transform.matrix[2] > 0.0);
    }

    #[test]
    fn explicit_counterclockwise_rotation_is_respected() {
        let source = page_size(1000.0, 200.0);

        let transform = a4_transform(&source, "portrait", "counterclockwise", 0.0);

        assert!(!transform.preserve_rotation);
        assert!(transform.matrix[1] > 0.0);
        assert!(transform.matrix[2] < 0.0);
    }

    #[test]
    fn non_zero_source_origin_is_translated_before_centering() {
        let mut source = page_size(300.0, 400.0);
        source.box_x0 = 18.0;
        source.box_y0 = 24.0;
        let transform = a4_transform(&source, "portrait", "none", 0.0);
        let x = transform.matrix[0] * source.box_x0
            + transform.matrix[2] * source.box_y0
            + transform.matrix[4];
        let y = transform.matrix[1] * source.box_x0
            + transform.matrix[3] * source.box_y0
            + transform.matrix[5];
        assert!((x - (A4_WIDTH_PT - 300.0) / 2.0).abs() < 0.01);
        assert!((y - (A4_HEIGHT_PT - 400.0) / 2.0).abs() < 0.01);
    }

    #[test]
    fn normalization_wraps_original_stream_without_reencoding_it() {
        let mut doc = Document::with_version("1.7");
        let original_bytes = b"q 1 0 0 1 20 30 cm /Original Do Q".to_vec();
        let content_id = doc.add_object(Stream::new(Dictionary::new(), original_bytes.clone()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let transform = a4_transform(&page_size(300.0, 400.0), "portrait", "none", 0.0);

        normalize_page_content(&mut doc, page_id, &transform).unwrap();

        let page = doc.get_dictionary(page_id).unwrap();
        let contents = page.get(b"Contents").unwrap().as_array().unwrap();
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[1].as_reference().unwrap(), content_id);
        assert_eq!(
            doc.get_object(content_id)
                .unwrap()
                .as_stream()
                .unwrap()
                .content,
            original_bytes
        );

        let prefix_id = contents[0].as_reference().unwrap();
        let prefix = doc.get_object(prefix_id).unwrap().as_stream().unwrap();
        let operations = Content::decode(&prefix.content).unwrap().operations;
        assert_eq!(operations[0].operator, "q");
        assert_eq!(operations[1].operator, "cm");

        let suffix_id = contents[2].as_reference().unwrap();
        let suffix = doc.get_object(suffix_id).unwrap().as_stream().unwrap();
        let operations = Content::decode(&suffix.content).unwrap().operations;
        assert_eq!(operations[0].operator, "Q");
    }

    #[test]
    fn normalized_page_dimensions_drive_the_following_overlay_coordinates() {
        if crate::external::QpdfTool.binary_path().is_err() {
            return;
        }
        let input = temp_named_path("docsy_normalize_overlay_input", "pdf");
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tj", vec![Object::string_literal("body text")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Dictionary::new(),
            "MediaBox" => vec![18.into(), 24.into(), 318.into(), 424.into()],
            "CropBox" => vec![18.into(), 24.into(), 318.into(), 424.into()],
            "TrimBox" => vec![18.into(), 24.into(), 318.into(), 424.into()],
            "BleedBox" => vec![18.into(), 24.into(), 318.into(), 424.into()],
            "ArtBox" => vec![18.into(), 24.into(), 318.into(), 424.into()],
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
        doc.save(&input).unwrap();

        let normalized = normalize_pdf_to_a4(&input, 200, "portrait", "none", 10.0).unwrap();
        let page_infos = get_page_infos(&normalized.to_string_lossy()).unwrap();
        assert_eq!(page_infos.len(), 1);
        assert!((page_infos[0].width_pt - A4_WIDTH_PT).abs() < 0.1);
        assert!((page_infos[0].height_pt - A4_HEIGHT_PT).abs() < 0.1);
        assert_eq!(page_infos[0].box_x0, 0.0);
        assert_eq!(page_infos[0].box_y0, 0.0);

        let normalized_doc = Document::load(&normalized).unwrap();
        let normalized_page = normalized_doc.get_pages().into_values().next().unwrap();
        let normalized_page_dict = normalized_doc.get_dictionary(normalized_page).unwrap();
        for name in [
            &b"MediaBox"[..],
            &b"CropBox"[..],
            &b"TrimBox"[..],
            &b"BleedBox"[..],
            &b"ArtBox"[..],
        ] {
            let values = normalized_page_dict.get(name).unwrap().as_array().unwrap();
            let number = |object: &Object| match object {
                Object::Real(value) => *value,
                Object::Integer(value) => *value as f32,
                _ => panic!("page box coordinate must be numeric"),
            };
            assert!((number(&values[0]) - 0.0).abs() < 0.1);
            assert!((number(&values[1]) - 0.0).abs() < 0.1);
            assert!((number(&values[2]) - A4_WIDTH_PT).abs() < 0.1);
            assert!((number(&values[3]) - A4_HEIGHT_PT).abs() < 0.1);
        }

        let header = OverlayTextConfig {
            text: "Header".to_string(),
            region: "header".to_string(),
            font_family: "auto".to_string(),
            font_size: 10.0,
            margin_mm: 2.0,
            align: "center".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: "HeaderText".to_string(),
        };
        let (overlay, _) = build_overlay_pdf(Some(&header), None, &[], &page_infos, 1, 1).unwrap();
        let overlay_path = temp_named_path("docsy_normalize_overlay_layer", "pdf");
        let output_path = temp_named_path("docsy_normalize_overlay_result", "pdf");
        std::fs::write(&overlay_path, &overlay).unwrap();
        let qpdf = crate::external::QpdfTool.binary_path().unwrap();
        let status = crate::external::hidden_command(qpdf)
            .arg(&normalized)
            .arg("--overlay")
            .arg(&overlay_path)
            .arg("--")
            .arg(&output_path)
            .status()
            .unwrap();
        assert!(crate::pdf::qpdf::status_is_success(&status));

        let output_doc = Document::load(&output_path).unwrap();
        let output_page = output_doc.get_pages().into_values().next().unwrap();
        let output_ops = output_doc
            .get_and_decode_page_content(output_page)
            .unwrap()
            .operations;
        let overlay_matrix = output_ops
            .iter()
            .rfind(|operation| operation.operator == "cm")
            .expect("qpdf overlay transform missing");
        let matrix_number = |index: usize| match &overlay_matrix.operands[index] {
            Object::Real(value) => *value,
            Object::Integer(value) => *value as f32,
            _ => panic!("overlay matrix operand must be numeric"),
        };
        assert!((matrix_number(0) - 1.0).abs() < 0.001);
        assert!((matrix_number(3) - 1.0).abs() < 0.001);
        assert!(matrix_number(4).abs() < 0.001);
        assert!(matrix_number(5).abs() < 0.001);

        let overlay_doc = Document::load_mem(&overlay).unwrap();
        let overlay_page = overlay_doc.get_pages().into_values().next().unwrap();
        let operations = overlay_doc
            .get_and_decode_page_content(overlay_page)
            .unwrap()
            .operations;
        let tm = operations.iter().find(|op| op.operator == "Tm").unwrap();
        let y = match &tm.operands[5] {
            Object::Real(value) => *value,
            Object::Integer(value) => *value as f32,
            _ => panic!("expected numeric text position"),
        };
        assert!((y - (A4_HEIGHT_PT - mm_to_pt(2.0))).abs() < 0.1);

        let normalized_ops = normalized_doc
            .get_and_decode_page_content(normalized_page)
            .unwrap()
            .operations;
        assert!(normalized_ops
            .iter()
            .any(|operation| operation.operator == "Tj"));

        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(normalized);
        let _ = std::fs::remove_file(overlay_path);
        let _ = std::fs::remove_file(output_path);
    }
}
