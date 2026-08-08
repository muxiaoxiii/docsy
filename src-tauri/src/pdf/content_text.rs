use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::temp_named_path;

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
    pub diagnostics: Vec<DeleteDiagnostic>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DeleteDiagnostic {
    pub reason: DeleteSkipReason,
}

impl Default for DeleteSkipReason {
    fn default() -> Self {
        DeleteSkipReason::FontUndecodable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeleteSkipReason {
    FontUndecodable,
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
        let (operations, page_result) = filter_page_operations(&content.operations, &page_plan, page_number);
        let direct_changed = page_result.removed() > 0;
        let mut combined_result = page_result;
        let xobjects = super::artifacts::page_xobjects(&doc, page_id);
        if !xobjects.is_empty() {
            let nested_result = filter_referenced_form_text(
                &mut doc,
                &content.operations,
                &xobjects,
                &page_plan,
                &mut HashSet::new(),
                page_number,
            )?;
            combined_result.removed_header += nested_result.removed_header;
            combined_result.removed_footer += nested_result.removed_footer;
            combined_result.diagnostics.extend(nested_result.diagnostics);
        }
        if combined_result.removed() == 0 {
            continue;
        }
        if direct_changed {
            let encoded = Content { operations }
                .encode()
                .context("编码删除普通文本页眉页脚后的内容流失败")?;
            doc.change_page_content(page_id, encoded)
                .context("写回删除普通文本页眉页脚后的内容流失败")?;
        }
        result.removed_header += combined_result.removed_header;
        result.removed_footer += combined_result.removed_footer;
    }

    doc.prune_objects();
    let temp = output_path.with_extension("pdf.tmp");
    doc.save(&temp)
        .context("保存删除普通文本页眉页脚后的 PDF 失败")?;
    std::fs::rename(&temp, output_path)
        .context("原子重命名 PDF 失败")?;
    Ok(result)
}

fn filter_referenced_form_text(
    doc: &mut Document,
    operations: &[Operation],
    xobjects: &Dictionary,
    plan: &PagePlainTextPlan,
    visited: &mut HashSet<ObjectId>,
    page_number: u32,
) -> Result<PlainTextCleanupResult> {
    let mut result = PlainTextCleanupResult::default();
    for operation in operations
        .iter()
        .filter(|operation| operation.operator == "Do")
    {
        let Some(name) = operation
            .operands
            .first()
            .and_then(|object| object.as_name().ok())
        else {
            continue;
        };
        let Some(object_id) = xobjects
            .get(name)
            .ok()
            .and_then(super::artifacts::object_reference)
        else {
            continue;
        };
        if !visited.insert(object_id) {
            continue;
        }
        let Some((stream_content, stream_dict)) = doc
            .get_object(object_id)
            .ok()
            .and_then(|object| object.as_stream().ok())
            .filter(|stream| {
                stream
                    .dict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|object| object.as_name().ok())
                    == Some(b"Form")
            })
            .and_then(|stream| {
                stream
                    .get_plain_content()
                    .ok()
                    .map(|content| (content, stream.dict.clone()))
            })
        else {
            continue;
        };
        let Ok(content) = Content::decode(&stream_content) else {
            continue;
        };
        // 读取 Form XObject 的 BBox 和 Matrix，构建表单本地坐标的 plan
        let form_plan = read_form_plan(&stream_dict, plan);
        let (filtered, form_result) = filter_page_operations(&content.operations, &form_plan, page_number);
        let direct_changed = form_result.removed() > 0;
        let resources =
            super::artifacts::resource_dictionary(doc, stream_dict.get(b"Resources").ok());
        let nested_xobjects = super::artifacts::xobjects_from_resources(doc, resources.as_ref());
        let mut combined_result = form_result;
        if !nested_xobjects.is_empty() {
            let nested_result = filter_referenced_form_text(
                doc,
                &content.operations,
                &nested_xobjects,
                plan,
                visited,
                page_number,
            )?;
            combined_result.removed_header += nested_result.removed_header;
            combined_result.removed_footer += nested_result.removed_footer;
            combined_result.diagnostics.extend(nested_result.diagnostics);
        }
        if direct_changed {
            let encoded = Content {
                operations: filtered.clone(),
            }
            .encode()
            .context("编码 Form XObject 中的普通文本页眉页脚失败")?;
            doc.get_object_mut(object_id)
                .and_then(Object::as_stream_mut)
                .context("写回 Form XObject 普通文本页眉页脚失败")?
                .set_plain_content(encoded);
            if !operations_use_font(&filtered, b"FCJKFallback") {
                remove_direct_form_font_resource(doc, object_id, b"FCJKFallback");
            }
        }
        result.removed_header += combined_result.removed_header;
        result.removed_footer += combined_result.removed_footer;
    }
    Ok(result)
}

fn operations_use_font(operations: &[Operation], font_name: &[u8]) -> bool {
    operations.iter().any(|operation| {
        operation.operator == "Tf"
            && operation
                .operands
                .first()
                .and_then(|object| object.as_name().ok())
                == Some(font_name)
    })
}

fn remove_direct_form_font_resource(doc: &mut Document, object_id: ObjectId, font_name: &[u8]) {
    let Ok(stream) = doc
        .get_object_mut(object_id)
        .and_then(Object::as_stream_mut)
    else {
        return;
    };
    let Ok(Object::Dictionary(resources)) = stream.dict.get_mut(b"Resources") else {
        return;
    };
    let Ok(Object::Dictionary(fonts)) = resources.get_mut(b"Font") else {
        return;
    };
    fonts.remove(font_name);
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
    _page_number: u32,
) -> (Vec<Operation>, PlainTextCleanupResult) {
    let mut output = Vec::with_capacity(operations.len());
    let mut result = PlainTextCleanupResult::default();
    let mut state = TextState::default();

    for operation in operations {
        update_text_state_before_show(&mut state, operation);
        let shown_text = shown_text(operation);
        let mut remove_region = None;
        if let Some(text) = shown_text.as_deref() {
            // 文本匹配：必须在 zone 内 + 文本内容匹配
            if is_in_header_zone(state.y, plan)
                && matches_any_target_by_text(text, &plan.header_targets)
            {
                remove_region = Some(TextRegion::Header);
            } else if is_in_footer_zone(state.y, plan)
                && matches_any_target_by_text(text, &plan.footer_targets)
            {
                remove_region = Some(TextRegion::Footer);
            }
            // bbox 匹配：独立于 zone check，处理 CID 字体无法解码的情况
            if remove_region.is_none() {
                let page_h = plan.page_box.max_y;
                let header_by_bbox = matches_any_target_by_bbox(&state, &plan.header_targets, page_h);
                let footer_by_bbox = !header_by_bbox
                    && matches_any_target_by_bbox(&state, &plan.footer_targets, page_h);
                if header_by_bbox {
                    remove_region = Some(TextRegion::Header);
                    result.diagnostics.push(DeleteDiagnostic {
                        reason: DeleteSkipReason::FontUndecodable,
                    });
                } else if footer_by_bbox {
                    remove_region = Some(TextRegion::Footer);
                    result.diagnostics.push(DeleteDiagnostic {
                        reason: DeleteSkipReason::FontUndecodable,
                    });
                }
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

fn matches_any_target_by_text(text: &str, targets: &[&PlainTextTarget]) -> bool {
    targets.iter().any(|target| target_matches(text, target))
}

fn matches_any_target_by_bbox(state: &TextState, targets: &[&PlainTextTarget], page_height: f32) -> bool {
    targets
        .iter()
        .any(|target| target_bbox_matches(state, target, page_height))
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

fn target_bbox_matches(state: &TextState, target: &PlainTextTarget, page_height: f32) -> bool {
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
    // 使用 lopdf 的页面高度（page_height）而非 pdftotext 的 bbox.height
    // 避免 CropBox/MediaBox 不一致导致的坐标偏移
    let height = if page_height > 0.0 { page_height } else { bbox.height };
    let pdf_y0 = height - bbox.y1;
    let pdf_y1 = height - bbox.y0;
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

/// 从 Form XObject 的 BBox 构建表单本地坐标的 plan
/// 表单内容使用 form-local 坐标，zone check 也需要在 form-local 坐标中进行
/// 所以 page_box 直接使用表单 BBox（不转换为页面坐标）
fn read_form_plan<'a>(
    stream_dict: &lopdf::Dictionary,
    page_plan: &'a PagePlainTextPlan<'a>,
) -> PagePlainTextPlan<'a> {
    let form_bbox = stream_dict
        .get(b"BBox")
        .ok()
        .and_then(|v| v.as_array().ok())
        .and_then(|arr| {
            if arr.len() >= 4 {
                Some((
                    object_number(&arr[0]).unwrap_or(0.0),
                    object_number(&arr[1]).unwrap_or(0.0),
                    object_number(&arr[2]).unwrap_or(0.0),
                    object_number(&arr[3]).unwrap_or(0.0),
                ))
            } else {
                None
            }
        });

    let Some((fx0, fy0, fx1, fy1)) = form_bbox else {
        // 没有 BBox，回退到页面级 plan
        return PagePlainTextPlan {
            header_targets: page_plan.header_targets.clone(),
            footer_targets: page_plan.footer_targets.clone(),
            header_zone_pt: page_plan.header_zone_pt,
            footer_zone_pt: page_plan.footer_zone_pt,
            page_box: page_plan.page_box,
        };
    };

    // 直接使用表单 BBox 作为 page_box
    // 表单内容的 state.y 是 form-local 坐标，zone check 也需要在同坐标系中
    PagePlainTextPlan {
        header_targets: page_plan.header_targets.clone(),
        footer_targets: page_plan.footer_targets.clone(),
        header_zone_pt: page_plan.header_zone_pt,
        footer_zone_pt: page_plan.footer_zone_pt,
        page_box: PageBox {
            width: fx1 - fx0,
            min_y: fy0,
            max_y: fy1,
        },
    }
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
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
        let (filtered, result) = filter_page_operations(&operations, &plan, 1);
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
        let (filtered, result) = filter_page_operations(&operations, &plan, 1);
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

    #[test]
    fn deletes_confirmed_plain_header_inside_form_xobject() {
        let input = temp_named_path("docsy_plain_form_input", "pdf");
        let output = temp_named_path("docsy_plain_form_output", "pdf");
        create_plain_text_form_test_pdf(&input);
        let plan = PlainTextCleanupPlan {
            header_targets: vec![PlainTextTarget {
                text: "Legacy Docsy Header".to_string(),
                normalized_text: "Legacy Docsy Header".to_string(),
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
        let document = Document::load(&output).unwrap();
        assert!(document.objects.values().all(|object| {
            !format!("{object:?}").contains("STSong-Light")
                && object
                    .as_stream()
                    .ok()
                    .and_then(|stream| stream.get_plain_content().ok())
                    .map(|content| {
                        !String::from_utf8_lossy(&content).contains("Legacy Docsy Header")
                    })
                    .unwrap_or(true)
        }));
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

    fn create_plain_text_form_test_pdf(path: &Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let fallback_font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type0",
            "BaseFont" => "STSong-Light",
            "Encoding" => "UniGB-UCS2-H",
        });
        let form_content = Content {
            operations: vec![
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
                Operation::new("Tj", vec![Object::string_literal("Legacy Docsy Header")]),
                Operation::new("ET", vec![]),
            ],
        };
        let form_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                "Resources" => dictionary! {
                    "Font" => dictionary! { "FCJKFallback" => fallback_font_id },
                },
            },
            form_content.encode().unwrap(),
        ));
        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Fx1" => form_id },
        });
        let page_content = Content {
            operations: vec![Operation::new("Do", vec![Object::Name(b"Fx1".to_vec())])],
        };
        let content_id = doc.add_object(Stream::new(
            Dictionary::new(),
            page_content.encode().unwrap(),
        ));
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
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
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
