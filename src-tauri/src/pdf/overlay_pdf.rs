//! Overlay PDF 构建：将页眉/页脚/自定义 overlay 渲染为独立的 PDF 层。

use anyhow::{Context, Result};
use lopdf::content::Content;
use lopdf::{dictionary, Dictionary, Document, Object, Stream};

use super::header_footer::OverlayTextConfig;
use super::overlay_font::{
    append_overlay_text_ops, overlay_region, prepare_embedded_overlay_fonts, OverlayRegion,
};
use super::page_info::PageSize;

#[cfg(test)]
use super::artifacts;
#[cfg(test)]
use super::overlay_font::{estimate_text_width, mm_to_pt};

/// 构建一个仅包含 overlay 文本算子的 PDF，供 qpdf --overlay 叠加到原始 PDF 上。
pub(crate) fn build_overlay_pdf(
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
    extra_overlays: &[OverlayTextConfig],
    pages: &[PageSize],
    page_start: u32,
    total_pages: u32,
) -> Result<(Vec<u8>, Vec<String>)> {
    let mut doc = Document::with_version("1.6");
    let mut warnings = Vec::new();
    let pages_id = doc.new_object_id();
    let helvetica_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let times_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Times-Roman",
    });
    let courier_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });
    let embedded_fonts = prepare_embedded_overlay_fonts(
        &mut doc,
        header,
        footer,
        extra_overlays,
        pages.len(),
        page_start,
        total_pages,
        &mut warnings,
    );
    let mut font_resources = Dictionary::new();
    font_resources.set("F1", helvetica_id);
    font_resources.set("FTimes", times_id);
    font_resources.set("FCourier", courier_id);
    for font in embedded_fonts.values() {
        font_resources.set(font.resource_name.as_str(), font.object_id);
    }
    let resources_id = doc.add_object(dictionary! {
        "Font" => font_resources,
    });
    let mut page_ids = Vec::new();

    for (index, size) in pages.iter().enumerate() {
        let current_page = page_start + index as u32;
        let local_page = index as u32 + 1;
        let page_h_mm = size.height_pt * 25.4 / 72.0;
        let mut operations = Vec::new();
        let mut placed: Vec<(OverlayRegion, f32, f32, f32, f32)> = Vec::new();
        let mut page_warnings: Vec<String> = Vec::new();

        // Draw order: main header, main footer, then extra overlays. Overlapping
        // overlays are still rendered as-is (真实反映重叠), with a warning only.
        let mut candidates: Vec<(OverlayRegion, &OverlayTextConfig)> = Vec::new();
        if let Some(config) = header.filter(|config| overlay_applies_to_page(config, local_page)) {
            candidates.push((OverlayRegion::Header, config));
        }
        if let Some(config) = footer.filter(|config| overlay_applies_to_page(config, local_page)) {
            candidates.push((OverlayRegion::Footer, config));
        }
        for config in extra_overlays {
            if overlay_applies_to_page(config, local_page) {
                candidates.push((overlay_region(&config.region), config));
            }
        }

        for (region, config) in candidates {
            let (y0, y1) = overlay_y_range_mm(config, region, page_h_mm);
            let text =
                super::overlay_font::expand_config_placeholders(config, current_page, total_pages);
            let use_embedded = super::overlay_font::requires_embedded_font(&text);
            let width_pt =
                super::overlay_font::estimate_text_width(&text, use_embedded, config.font_size);
            let x0_pt = super::overlay_font::compute_x(config, &text, use_embedded, size.width_pt);
            let x0_mm = x0_pt * 25.4 / 72.0;
            let width_mm = width_pt * 25.4 / 72.0;
            let x1_mm = x0_mm + width_mm;

            let overlaps = placed.iter().any(|(pr, py0, py1, px0, px1)| {
                *pr == region
                    && y0 < *py1 - 0.5
                    && *py0 < y1 - 0.5
                    && x0_mm < *px1 - 0.5
                    && *px0 < x1_mm - 0.5
            });
            if overlaps {
                page_warnings.push(format!(
                    "第 {local_page} 页的\u{201c}{}\u{201d}与其他页眉页脚位置重叠，将按实际位置叠加渲染",
                    config.text.trim()
                ));
            }
            placed.push((region, y0, y1, x0_mm, x1_mm));
            if append_overlay_text_ops(
                &mut operations,
                config,
                region,
                size,
                current_page,
                total_pages,
                &embedded_fonts,
            )? {
                page_warnings.push(format!(
                    "第 {local_page} 页的\"{}\"超出页面右缘，已收拢到页面内；如仍被裁切请缩短文本或调小字号",
                    config.text.trim()
                ));
            }
        }
        if !page_warnings.is_empty() {
            warnings.push(page_warnings.join("；"));
        }

        // 注意：overlay 页按「视觉尺寸」（旋转页宽高已互换）建页即可。
        // qpdf --overlay 会自动按 base 页的 /Rotate 对叠加内容施加逆旋转
        // cm（实证：qpdf 12.3.2 对 Rotate=90 的 base 页自动包
        // `q 0 1 -1 0 <raw_w> 0 cm ... Q`），因此文字算子直接按视觉坐标
        // 书写即可落在正确位置；切勿再自行加补偿变换，否则会双重旋转。
        let content = Content { operations };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), size.width_pt.into(), size.height_pt.into()],
        });
        page_ids.push(page_id);
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
            "Count" => page_ids.len() as i64,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .context("生成轻量页眉页脚 PDF 失败")?;
    Ok((output, warnings))
}

pub(crate) fn overlay_applies_to_page(config: &OverlayTextConfig, local_page: u32) -> bool {
    let start = config.page_start.unwrap_or(1).max(1);
    let end = config.page_end.unwrap_or(u32::MAX).max(start);
    local_page >= start && local_page <= end
}

pub(crate) fn overlay_uses_page_placeholders(config: &OverlayTextConfig) -> bool {
    config.text.contains("{page}")
        || config.text.contains("{total}")
        || config.text.contains("{range}")
}

/// Approximate vertical extent (mm from page top) of an overlay for collision checks.
fn overlay_y_range_mm(
    config: &OverlayTextConfig,
    region: OverlayRegion,
    page_h_mm: f32,
) -> (f32, f32) {
    let height = config.font_size * 0.4;
    match region {
        OverlayRegion::Header => (config.margin_mm, config.margin_mm + height),
        OverlayRegion::Footer => (
            page_h_mm - config.margin_mm - height,
            page_h_mm - config.margin_mm,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf_number(object: &Object) -> f32 {
        match object {
            Object::Real(value) => *value,
            Object::Integer(value) => *value as f32,
            other => panic!("expected PDF number, got {other:?}"),
        }
    }

    /// 构造一个走内建 Helvetica 字体的英文页眉配置，避免测试依赖系统字体文件。
    fn plain_header_config() -> OverlayTextConfig {
        OverlayTextConfig {
            text: "Header".to_string(),
            region: "header".to_string(),
            font_family: "auto".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "center".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: String::new(),
        }
    }

    /// 未旋转的 A4 纵向页。
    fn unrotated_page_size() -> PageSize {
        PageSize {
            width_pt: 595.0,
            height_pt: 842.0,
            raw_width_pt: 595.0,
            raw_height_pt: 842.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        }
    }

    /// Rotate=90 的页：未旋转 595×842，视觉 842×595。
    fn rotated_page_size(rotate: i32) -> PageSize {
        let mut size = unrotated_page_size();
        size.rotate = rotate;
        if rotate == 90 || rotate == 270 {
            size.width_pt = 842.0;
            size.height_pt = 595.0;
        }
        size
    }

    /// 回归测试（旋转页 overlay 坐标）：qpdf --overlay 会自动按 base 页的
    /// /Rotate 对叠加内容施加逆旋转 cm（实证见 deferred-issues.md 旋转页条目），
    /// 因此旋转页的 overlay 页必须按「视觉尺寸」建 MediaBox，且内容流不得再
    /// 自行叠加任何补偿 cm——否则会被 qpdf 的自动补偿双重旋转。
    #[test]
    fn rotated_page_overlay_uses_visual_mediabox_without_extra_cm() {
        let header = plain_header_config();
        let pages = vec![rotated_page_size(90)];
        let (bytes, warnings) = build_overlay_pdf(Some(&header), None, &[], &pages, 1, 1).unwrap();
        assert!(warnings.is_empty());

        let document = Document::load_mem(&bytes).unwrap();
        let page_id = document.get_pages().into_values().next().unwrap();
        let page_dict = document.get_dictionary(page_id).unwrap();
        let media_box: Vec<f32> = page_dict
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(pdf_number)
            .collect();
        assert_eq!(media_box, vec![0.0, 0.0, 842.0, 595.0]);

        let content = document.get_and_decode_page_content(page_id).unwrap();
        assert!(content.operations.iter().all(|op| op.operator != "cm"));

        let tm = content
            .operations
            .iter()
            .find(|op| op.operator == "Tm")
            .unwrap();
        let y = pdf_number(&tm.operands[5]);
        let expected_y = 595.0 - mm_to_pt(10.0);
        assert!(
            (y - expected_y).abs() < 0.01,
            "y={y}, expected={expected_y}"
        );
    }

    #[test]
    fn unrotated_page_overlay_has_no_compensation_cm() {
        let header = plain_header_config();
        let pages = vec![unrotated_page_size()];
        let (bytes, _) = build_overlay_pdf(Some(&header), None, &[], &pages, 1, 1).unwrap();

        let document = Document::load_mem(&bytes).unwrap();
        let page_id = document.get_pages().into_values().next().unwrap();
        let page_dict = document.get_dictionary(page_id).unwrap();
        let media_box: Vec<f32> = page_dict
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(pdf_number)
            .collect();
        assert_eq!(media_box, vec![0.0, 0.0, 595.0, 842.0]);

        let content = document.get_and_decode_page_content(page_id).unwrap();
        assert!(content.operations.iter().all(|op| op.operator != "cm"));
    }

    #[test]
    fn cjk_overlay_embeds_only_subset_font() {
        let pages = vec![PageSize {
            width_pt: 595.0,
            height_pt: 842.0,
            raw_width_pt: 595.0,
            raw_height_pt: 842.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        }];
        let header = OverlayTextConfig {
            text: "测试页眉3".to_string(),
            region: "header".to_string(),
            font_family: "songti".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "right".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: "HeaderText".to_string(),
        };
        let (bytes, _warnings) = build_overlay_pdf(Some(&header), None, &[], &pages, 1, 1).unwrap();

        assert!(
            bytes.len() < 500_000,
            "overlay PDF too large: {}",
            bytes.len()
        );
        let document = Document::load_mem(&bytes).unwrap();
        assert!(document
            .objects
            .values()
            .all(|object| !format!("{object:?}").contains("STSong-Light")));
        let page_id = document.get_pages().into_values().next().unwrap();
        let content = document.get_and_decode_page_content(page_id).unwrap();
        let artifact = content
            .operations
            .iter()
            .find(|operation| operation.operator == "BDC")
            .and_then(|operation| operation.operands.get(1))
            .and_then(|object| object.as_dict().ok())
            .unwrap();
        assert_eq!(
            artifact.get(b"Subtype").unwrap().as_name().unwrap(),
            b"Header"
        );
        assert_eq!(
            artifact.get(b"DocsyKind").unwrap().as_name().unwrap(),
            b"HeaderText"
        );
        assert!(artifact.get(b"DocsyId").is_ok());
        assert_eq!(
            artifacts::decode_pdf_string(artifact.get(b"ActualText").unwrap()).as_deref(),
            Some("测试页眉3")
        );
        let descriptor = document
            .objects
            .values()
            .filter_map(|object| object.as_dict().ok())
            .find(|dict| dict.get_type().ok() == Some(b"FontDescriptor"))
            .unwrap();
        let bbox = descriptor.get(b"FontBBox").unwrap().as_array().unwrap();
        let bbox_values = bbox
            .iter()
            .map(|value| value.as_i64().unwrap())
            .collect::<Vec<_>>();
        assert!(bbox_values[2] - bbox_values[0] >= 500);
        assert!(bbox_values
            .iter()
            .all(|value| (-2_000..=2_000).contains(value)));
        let font_name = descriptor.get(b"FontName").unwrap().as_name().unwrap();
        assert_eq!(font_name.iter().position(|byte| *byte == b'+'), Some(6));
        let type0_font = document
            .objects
            .values()
            .filter_map(|object| object.as_dict().ok())
            .find(|dict| {
                dict.get(b"Subtype")
                    .ok()
                    .and_then(|value| value.as_name().ok())
                    == Some(b"Type0")
                    && dict
                        .get(b"BaseFont")
                        .ok()
                        .and_then(|value| value.as_name().ok())
                        .map(|name| name.starts_with(b"DCSYA"))
                        .unwrap_or(false)
            })
            .unwrap();
        assert!(matches!(
            type0_font
                .get(b"DescendantFonts")
                .unwrap()
                .as_array()
                .unwrap()
                .first(),
            Some(Object::Reference(_))
        ));
    }

    #[test]
    fn overlapping_overlays_warn_but_are_both_written() {
        let pages = vec![PageSize {
            width_pt: 595.0,
            height_pt: 842.0,
            raw_width_pt: 595.0,
            raw_height_pt: 842.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        }];
        let config = |text: &str| OverlayTextConfig {
            text: text.to_string(),
            region: "header".to_string(),
            font_family: "auto".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "right".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: "HeaderText".to_string(),
        };
        let header = config("主页眉");
        let extra = config("页码");
        let (bytes, warnings) = build_overlay_pdf(Some(&header), None, &[extra], &pages, 1, 1)
            .expect("overlay PDF should be generated");

        assert_eq!(warnings.len(), 1);
        let document = Document::load_mem(&bytes).expect("overlay PDF should be readable");
        let page_id = document.get_pages().into_values().next().unwrap();
        let content = document.get_and_decode_page_content(page_id).unwrap();
        let texts = content
            .operations
            .iter()
            .filter(|operation| operation.operator == "BDC")
            .filter_map(|operation| operation.operands.get(1))
            .filter_map(|object| object.as_dict().ok())
            .filter_map(|dict| dict.get(b"ActualText").ok())
            .filter_map(artifacts::decode_pdf_string)
            .collect::<Vec<_>>();
        assert!(texts.iter().any(|text| text == "主页眉"));
        assert!(texts.iter().any(|text| text == "页码"));
    }

    #[test]
    fn overflowing_overlay_is_pulled_back_into_page_with_warning() {
        let pages = vec![PageSize {
            width_pt: 400.0,
            height_pt: 300.0,
            raw_width_pt: 400.0,
            raw_height_pt: 300.0,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        }];
        let footer = OverlayTextConfig {
            text: "证据十三. 中航试金石检测科技（大厂）有限公司检测报告".to_string(),
            region: "footer".to_string(),
            font_family: "auto".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "right".to_string(),
            offset_x_mm: 50.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: "FooterText".to_string(),
        };
        let (bytes, warnings) = build_overlay_pdf(None, Some(&footer), &[], &pages, 1, 1)
            .expect("overlay PDF should be generated");
        assert!(
            warnings.iter().any(|w| w.contains("超出页面右缘")),
            "overflow should produce a warning, got {warnings:?}"
        );
        let document = Document::load_mem(&bytes).expect("overlay PDF should be readable");
        let page_id = document.get_pages().into_values().next().unwrap();
        let content = document.get_and_decode_page_content(page_id).unwrap();
        let tm_x = content
            .operations
            .iter()
            .filter(|operation| operation.operator == "Tm")
            .filter_map(|operation| operation.operands.get(4))
            .filter_map(|object| match object {
                Object::Real(value) => Some(*value),
                Object::Integer(value) => Some(*value as f32),
                _ => None,
            })
            .next()
            .expect("Tm should exist");
        let width = estimate_text_width(
            "证据十三. 中航试金石检测科技（大厂）有限公司检测报告",
            true,
            10.0,
        );
        assert!(
            tm_x + width <= 400.0 + 0.5,
            "text should be pulled back into the page: x={tm_x}, width={width}"
        );
    }

    #[test]
    fn overlay_page_range_limits_rebuilt_text_to_detected_pages() {
        let config = OverlayTextConfig {
            text: "新页脚".to_string(),
            region: "footer".to_string(),
            font_family: "songti".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "right".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: Some(2),
            page_end: Some(3),
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: "FooterText".to_string(),
        };

        assert!(!overlay_applies_to_page(&config, 1));
        assert!(overlay_applies_to_page(&config, 2));
        assert!(overlay_applies_to_page(&config, 3));
        assert!(!overlay_applies_to_page(&config, 4));
    }
}
