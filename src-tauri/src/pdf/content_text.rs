use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub(crate) struct PlainTextCleanupPlan {
    pub header_targets: Vec<PlainTextTarget>,
    pub footer_targets: Vec<PlainTextTarget>,
    pub header_zone_mm: f32,
    pub footer_zone_mm: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct PlainTextTarget {
    pub text: String,
    pub normalized_text: String,
    pub page_start: u32,
    pub page_end: u32,
    pub bbox: Option<PlainTextTargetBBox>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlainTextTargetBBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub page: u32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PlainTextCleanupResult {
    pub removed_header: usize,
    pub removed_footer: usize,
}

impl PlainTextCleanupResult {
    pub(crate) fn removed(&self) -> usize {
        self.removed_header + self.removed_footer
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextRegion {
    Header,
    Footer,
}

#[derive(Debug, Clone, Copy, Default)]
struct TextState {
    in_text: bool,
    x: f32,
    y: f32,
    leading: f32,
}

pub(crate) fn delete_plain_header_footer_to_temp(
    input_path: &str,
    plan: &PlainTextCleanupPlan,
) -> Result<Option<(PathBuf, PlainTextCleanupResult)>> {
    if plan.header_targets.is_empty() && plan.footer_targets.is_empty() {
        return Ok(None);
    }
    let output = temp_named_path("docsy_plain_hf_deleted", "pdf");
    let result = delete_plain_header_footer_file(input_path, &output, plan)?;
    if result.removed() == 0 {
        let _ = std::fs::remove_file(&output);
        return Ok(None);
    }
    Ok(Some((output, result)))
}

fn delete_plain_header_footer_file(
    input_path: &str,
    output_path: &Path,
    plan: &PlainTextCleanupPlan,
) -> Result<PlainTextCleanupResult> {
    let input = Path::new(input_path);
    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    let mut result = PlainTextCleanupResult::default();

    for (page_index, page_id) in page_ids.into_iter().enumerate() {
        let page_number = page_index as u32 + 1;
        let content = match doc.get_and_decode_page_content(page_id) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let Some(page_box) = page_box(&doc, page_id) else {
            continue;
        };
        let page_plan = PagePlainTextPlan {
            header_targets: active_targets(&plan.header_targets, page_number),
            footer_targets: active_targets(&plan.footer_targets, page_number),
            header_zone_pt: mm_to_pt(plan.header_zone_mm.max(1.0)),
            footer_zone_pt: mm_to_pt(plan.footer_zone_mm.max(1.0)),
            page_box,
        };
        if page_plan.header_targets.is_empty() && page_plan.footer_targets.is_empty() {
            continue;
        }
        let (operations, page_result) = filter_page_operations(&content.operations, &page_plan);
        if page_result.removed() == 0 {
            continue;
        }
        let encoded = Content { operations }
            .encode()
            .context("编码删除普通文本页眉页脚后的内容流失败")?;
        doc.change_page_content(page_id, encoded)
            .context("写回删除普通文本页眉页脚后的内容流失败")?;
        result.removed_header += page_result.removed_header;
        result.removed_footer += page_result.removed_footer;
    }

    doc.prune_objects();
    doc.save(output_path)
        .context("保存删除普通文本页眉页脚后的 PDF 失败")?;
    Ok(result)
}

struct PagePlainTextPlan<'a> {
    header_targets: Vec<&'a PlainTextTarget>,
    footer_targets: Vec<&'a PlainTextTarget>,
    header_zone_pt: f32,
    footer_zone_pt: f32,
    page_box: PageBox,
}

#[derive(Debug, Clone, Copy)]
struct PageBox {
    width: f32,
    min_y: f32,
    max_y: f32,
}

fn active_targets(targets: &[PlainTextTarget], page_number: u32) -> Vec<&PlainTextTarget> {
    targets
        .iter()
        .filter(|target| {
            let start = target.page_start.max(1);
            let end = target.page_end.max(start);
            page_number >= start && page_number <= end
        })
        .collect()
}

fn filter_page_operations(
    operations: &[Operation],
    plan: &PagePlainTextPlan,
) -> (Vec<Operation>, PlainTextCleanupResult) {
    let mut output = Vec::with_capacity(operations.len());
    let mut result = PlainTextCleanupResult::default();
    let mut state = TextState::default();

    for operation in operations {
        update_text_state_before_show(&mut state, operation);
        let shown_text = shown_text(operation);
        let mut remove_region = None;
        if let Some(text) = shown_text.as_deref() {
            if is_in_header_zone(state.y, plan)
                && matches_any_target(text, &plan.header_targets, &state)
            {
                remove_region = Some(TextRegion::Header);
            } else if is_in_footer_zone(state.y, plan)
                && matches_any_target(text, &plan.footer_targets, &state)
            {
                remove_region = Some(TextRegion::Footer);
            }
        }
        match remove_region {
            Some(TextRegion::Header) => result.removed_header += 1,
            Some(TextRegion::Footer) => result.removed_footer += 1,
            None => output.push(operation.clone()),
        }
        update_text_state_after_show(&mut state, operation);
    }

    (output, result)
}

fn update_text_state_before_show(state: &mut TextState, operation: &Operation) {
    match operation.operator.as_str() {
        "BT" => {
            state.in_text = true;
            state.x = 0.0;
            state.y = 0.0;
        }
        "ET" => {
            state.in_text = false;
        }
        "Td" => {
            if let (Some(tx), Some(ty)) =
                (number_operand(operation, 0), number_operand(operation, 1))
            {
                state.x += tx;
                state.y += ty;
            }
        }
        "TD" => {
            if let (Some(tx), Some(ty)) =
                (number_operand(operation, 0), number_operand(operation, 1))
            {
                state.leading = -ty;
                state.x += tx;
                state.y += ty;
            }
        }
        "Tm" => {
            if let (Some(x), Some(y)) = (number_operand(operation, 4), number_operand(operation, 5))
            {
                state.x = x;
                state.y = y;
            }
        }
        "TL" => {
            if let Some(leading) = number_operand(operation, 0) {
                state.leading = leading;
            }
        }
        "T*" | "'" | "\"" => {
            state.y -= state.leading;
        }
        _ => {}
    }
}

fn update_text_state_after_show(_state: &mut TextState, _operation: &Operation) {}

fn number_operand(operation: &Operation, index: usize) -> Option<f32> {
    match operation.operands.get(index)? {
        Object::Integer(value) => Some(*value as f32),
        Object::Real(value) => Some(*value),
        _ => None,
    }
}

fn shown_text(operation: &Operation) -> Option<String> {
    match operation.operator.as_str() {
        "Tj" | "'" => operation.operands.first().and_then(object_text),
        "\"" => operation.operands.get(2).and_then(object_text),
        "TJ" => {
            let Object::Array(items) = operation.operands.first()? else {
                return None;
            };
            let mut text = String::new();
            for item in items {
                if let Some(part) = object_text(item) {
                    text.push_str(&part);
                }
            }
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

fn object_text(object: &Object) -> Option<String> {
    let Object::String(bytes, _) = object else {
        return None;
    };
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).ok();
    }
    Some(String::from_utf8_lossy(bytes).to_string())
}

fn is_in_header_zone(y: f32, plan: &PagePlainTextPlan) -> bool {
    y >= plan.page_box.max_y - plan.header_zone_pt
        && y <= plan.page_box.max_y + 24.0
        && plan.page_box.width > 0.0
}

fn is_in_footer_zone(y: f32, plan: &PagePlainTextPlan) -> bool {
    y >= plan.page_box.min_y - 24.0
        && y <= plan.page_box.min_y + plan.footer_zone_pt
        && plan.page_box.width > 0.0
}

fn matches_any_target(text: &str, targets: &[&PlainTextTarget], state: &TextState) -> bool {
    targets
        .iter()
        .any(|target| target_matches(text, target) || target_bbox_matches(state, target))
}

fn target_matches(text: &str, target: &PlainTextTarget) -> bool {
    let normalized_text = normalize_for_match(text);
    let target_text = normalize_for_match(&target.text);
    let target_normalized = normalize_for_match(&target.normalized_text);
    if !target_text.is_empty() && normalized_text == target_text {
        return true;
    }
    if !target_normalized.is_empty() && normalized_text == target_normalized {
        return true;
    }
    if target_normalized.contains("{page}") || target_normalized.contains("{total}") {
        return placeholder_pattern_matches(&normalized_text, &target_normalized);
    }
    false
}

fn target_bbox_matches(state: &TextState, target: &PlainTextTarget) -> bool {
    let Some(bbox) = target.bbox else {
        return false;
    };
    if bbox.width <= 0.0 || bbox.height <= 0.0 {
        return false;
    }
    if bbox.page > 0 && (bbox.page < target.page_start || bbox.page > target.page_end) {
        return false;
    }
    let x_padding = 18.0;
    let y_padding = 18.0;
    let pdf_y0 = bbox.height - bbox.y1;
    let pdf_y1 = bbox.height - bbox.y0;
    state.x >= bbox.x0 - x_padding
        && state.x <= bbox.x1 + x_padding
        && state.y >= pdf_y0 - y_padding
        && state.y <= pdf_y1 + y_padding
}

fn placeholder_pattern_matches(text: &str, pattern: &str) -> bool {
    let mut regex = String::from("^");
    let mut rest = pattern;
    while let Some(index) = rest.find('{') {
        regex.push_str(&regex::escape(&rest[..index]));
        if rest[index..].starts_with("{page}") {
            regex.push_str(r"\d+");
            rest = &rest[index + 6..];
        } else if rest[index..].starts_with("{total}") {
            regex.push_str(r"\d+");
            rest = &rest[index + 7..];
        } else {
            regex.push_str("\\{");
            rest = &rest[index + 1..];
        }
    }
    regex.push_str(&regex::escape(rest));
    regex.push('$');
    Regex::new(&regex)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

fn normalize_for_match(text: &str) -> String {
    text.chars()
        .filter_map(|ch| {
            let normalized = match ch {
                '０'..='９' => char::from_u32(ch as u32 - '０' as u32 + '0' as u32).unwrap_or(ch),
                _ => ch,
            };
            if normalized.is_whitespace() {
                None
            } else {
                Some(normalized)
            }
        })
        .collect()
}

fn page_box(doc: &Document, page_id: ObjectId) -> Option<PageBox> {
    let mut current_id = page_id;
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(current_id) {
            return None;
        }
        let node = doc.get_object(current_id).ok()?.as_dict().ok()?;
        if let Some(page_box) =
            node_box(doc, node, b"CropBox").or_else(|| node_box(doc, node, b"MediaBox"))
        {
            return Some(page_box);
        }
        current_id = node.get(b"Parent").ok()?.as_reference().ok()?;
    }
}

fn node_box(doc: &Document, node: &lopdf::Dictionary, key: &[u8]) -> Option<PageBox> {
    let value = node.get(key).ok()?;
    let page_box = match value {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        other => other,
    };
    let page_box = page_box.as_array().ok()?;
    if page_box.len() != 4 {
        return None;
    }
    let x0 = object_number(&page_box[0])?;
    let y0 = object_number(&page_box[1])?;
    let x1 = object_number(&page_box[2])?;
    let y1 = object_number(&page_box[3])?;
    Some(PageBox {
        width: (x1 - x0).abs(),
        min_y: y0.min(y1),
        max_y: y0.max(y1),
    })
}

fn object_number(object: &Object) -> Option<f32> {
    match object {
        Object::Integer(value) => Some(*value as f32),
        Object::Real(value) => Some(*value),
        _ => None,
    }
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
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
    use lopdf::content::Operation;
    use lopdf::dictionary;
    use lopdf::Stream;

    #[test]
    fn removes_matching_header_text_in_header_zone() {
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    460.into(),
                    812.into(),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("Existing Header")]),
            Operation::new("ET", vec![]),
            Operation::new("BT", vec![]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    72.into(),
                    500.into(),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("Existing Header")]),
            Operation::new("ET", vec![]),
        ];
        let target = PlainTextTarget {
            text: "Existing Header".to_string(),
            normalized_text: "Existing Header".to_string(),
            page_start: 1,
            page_end: 1,
            bbox: None,
        };
        let plan = PagePlainTextPlan {
            header_targets: vec![&target],
            footer_targets: vec![],
            header_zone_pt: 60.0,
            footer_zone_pt: 60.0,
            page_box: PageBox {
                width: 595.0,
                min_y: 0.0,
                max_y: 842.0,
            },
        };
        let (filtered, result) = filter_page_operations(&operations, &plan);
        assert_eq!(result.removed_header, 1);
        assert_eq!(
            filtered.iter().filter_map(shown_text).collect::<Vec<_>>(),
            vec!["Existing Header".to_string()]
        );
    }

    #[test]
    fn matches_page_number_placeholder_targets() {
        let target = PlainTextTarget {
            text: "1/20".to_string(),
            normalized_text: "{page}/{total}".to_string(),
            page_start: 1,
            page_end: 20,
            bbox: None,
        };
        assert!(target_matches(" 2 / 20 ", &target));
        assert!(!target_matches("body 2 / 20", &target));
    }

    #[test]
    fn deletes_header_by_detected_bbox_when_text_is_not_decodable() {
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    506.into(),
                    808.into(),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("encoded-glyphs")]),
            Operation::new("ET", vec![]),
            Operation::new("BT", vec![]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    90.into(),
                    700.into(),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("body text")]),
            Operation::new("ET", vec![]),
        ];
        let target = PlainTextTarget {
            text: "测试页眉3".to_string(),
            normalized_text: "测试页眉3".to_string(),
            page_start: 1,
            page_end: 1,
            bbox: Some(PlainTextTargetBBox {
                x0: 505.0,
                y0: 15.0,
                x1: 578.0,
                y1: 35.0,
                page: 1,
                width: 595.0,
                height: 842.0,
            }),
        };
        let plan = PagePlainTextPlan {
            header_targets: vec![&target],
            footer_targets: vec![],
            header_zone_pt: 60.0,
            footer_zone_pt: 60.0,
            page_box: PageBox {
                width: 595.0,
                min_y: 0.0,
                max_y: 842.0,
            },
        };
        let (filtered, result) = filter_page_operations(&operations, &plan);
        assert_eq!(result.removed_header, 1);
        let text = filtered.iter().filter_map(shown_text).collect::<Vec<_>>();
        assert!(!text.iter().any(|value| value == "encoded-glyphs"));
        assert!(text.iter().any(|value| value == "body text"));
    }

    #[test]
    fn deletes_plain_text_from_pdf_file() {
        let input = temp_named_path("docsy_plain_text_input", "pdf");
        let output = temp_named_path("docsy_plain_text_output", "pdf");
        create_plain_text_test_pdf(&input);
        let plan = PlainTextCleanupPlan {
            header_targets: vec![PlainTextTarget {
                text: "Existing Header".to_string(),
                normalized_text: "Existing Header".to_string(),
                page_start: 1,
                page_end: 1,
                bbox: None,
            }],
            header_zone_mm: 25.0,
            footer_zone_mm: 25.0,
            ..Default::default()
        };
        let result =
            delete_plain_header_footer_file(&input.to_string_lossy(), &output, &plan).unwrap();
        assert_eq!(result.removed_header, 1);
        let doc = Document::load(&output).unwrap();
        let page_id = doc.get_pages().into_values().next().unwrap();
        let content = doc.get_and_decode_page_content(page_id).unwrap();
        let text = content
            .operations
            .iter()
            .filter_map(shown_text)
            .collect::<Vec<_>>();
        assert!(!text.iter().any(|value| value == "Existing Header"));
        assert!(text.iter().any(|value| value == "Body Existing Header"));
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn deletes_plain_text_when_page_box_is_inherited() {
        let input = temp_named_path("docsy_plain_text_inherited_input", "pdf");
        let output = temp_named_path("docsy_plain_text_inherited_output", "pdf");
        create_plain_text_test_pdf_with_inherited_media_box(&input);
        let plan = PlainTextCleanupPlan {
            header_targets: vec![PlainTextTarget {
                text: "Existing Header".to_string(),
                normalized_text: "Existing Header".to_string(),
                page_start: 1,
                page_end: 1,
                bbox: None,
            }],
            header_zone_mm: 25.0,
            footer_zone_mm: 25.0,
            ..Default::default()
        };
        let result =
            delete_plain_header_footer_file(&input.to_string_lossy(), &output, &plan).unwrap();
        assert_eq!(result.removed_header, 1);
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    fn create_plain_text_test_pdf(path: &Path) {
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
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
                Operation::new(
                    "Tm",
                    vec![
                        1.into(),
                        0.into(),
                        0.into(),
                        1.into(),
                        460.into(),
                        812.into(),
                    ],
                ),
                Operation::new("Tj", vec![Object::string_literal("Existing Header")]),
                Operation::new("ET", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new(
                    "Tm",
                    vec![
                        1.into(),
                        0.into(),
                        0.into(),
                        1.into(),
                        72.into(),
                        500.into(),
                    ],
                ),
                Operation::new("Tj", vec![Object::string_literal("Body Existing Header")]),
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

    fn create_plain_text_test_pdf_with_inherited_media_box(path: &Path) {
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
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
                Operation::new(
                    "Tm",
                    vec![
                        1.into(),
                        0.into(),
                        0.into(),
                        1.into(),
                        460.into(),
                        812.into(),
                    ],
                ),
                Operation::new("Tj", vec![Object::string_literal("Existing Header")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }
}
