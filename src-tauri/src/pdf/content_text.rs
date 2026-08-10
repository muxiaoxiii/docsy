use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::Object;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{cmap::ToUnicodeCMap, temp_named_path, text_utils::normalize_for_match};

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum DeleteSkipReason {
    #[default]
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

#[derive(Debug, Clone, Default)]
struct TextState {
    in_text: bool,
    x: f32,
    y: f32,
    leading: f32,
    font_name: Option<String>,
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
    let index = super::qpdf_stream::QpdfObjectIndex::load(input)?;
    let font_cmaps = super::cmap::load_font_cmaps(input, &index)?;

    // 诊断：记录 target 信息
    for (i, t) in plan.header_targets.iter().enumerate() {
        log::info!(
            "plain_delete.target header[{}]: text={:?} normalized={:?} pages={}-{} bbox={}",
            i,
            t.text,
            t.normalized_text,
            t.page_start,
            t.page_end,
            t.bbox
                .as_ref()
                .map(|b| format!(
                    "({},{} - {},{}) w={} h={}",
                    b.x0, b.y0, b.x1, b.y1, b.width, b.height
                ))
                .unwrap_or_else(|| "null".to_string())
        );
    }
    for (i, t) in plan.footer_targets.iter().enumerate() {
        log::info!(
            "plain_delete.target footer[{}]: text={:?} normalized={:?} pages={}-{} bbox={}",
            i,
            t.text,
            t.normalized_text,
            t.page_start,
            t.page_end,
            t.bbox
                .as_ref()
                .map(|b| format!(
                    "({},{} - {},{}) w={} h={}",
                    b.x0, b.y0, b.x1, b.y1, b.width, b.height
                ))
                .unwrap_or_else(|| "null".to_string())
        );
    }

    let direct_references = index
        .pages()
        .iter()
        .flat_map(|page| page.contents.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut streams =
        super::qpdf_stream::load_editable_streams(input, &direct_references, "删除现有页眉页脚")?;
    let mut seeds = Vec::new();
    for page in index.pages() {
        let has_targets = !active_targets(&plan.header_targets, page.number).is_empty()
            || !active_targets(&plan.footer_targets, page.number).is_empty();
        let page_box = page.page_box.map(qpdf_box_to_page_box);
        if has_targets && page_box.is_none() {
            anyhow::bail!("第 {} 页缺少可解析的页面尺寸，未修改原文件", page.number);
        }
        for content_ref in &page.contents {
            seeds.push(QpdfStreamUsageSeed {
                object_ref: content_ref.clone(),
                page: page.number,
                page_box,
                fonts: page.fonts.clone(),
                xobjects: page.xobjects.clone(),
                depth: 0,
            });
        }
    }

    let mut usages: BTreeMap<String, Vec<QpdfStreamUsage>> = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let mut cursor = 0;
    while cursor < seeds.len() {
        let missing = seeds[cursor..]
            .iter()
            .map(|seed| seed.object_ref.clone())
            .filter(|reference| !streams.contains_key(reference))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            streams.extend(super::qpdf_stream::load_editable_streams(
                input,
                &missing,
                "删除现有页眉页脚 Form",
            )?);
        }
        let seed = seeds[cursor].clone();
        cursor += 1;
        if seed.depth >= 128 {
            anyhow::bail!("PDF Form 嵌套超过安全上限，未修改原文件");
        }
        if !visited.insert((seed.object_ref.clone(), seed.page)) {
            continue;
        }
        let stream = streams
            .get(&seed.object_ref)
            .with_context(|| format!("qpdf 未返回内容流 {}", seed.object_ref))?;
        usages
            .entry(seed.object_ref.clone())
            .or_default()
            .push(QpdfStreamUsage {
                page: seed.page,
                page_box: seed.page_box,
                fonts: seed.fonts.clone(),
            });
        for operation in stream
            .operations
            .iter()
            .filter(|operation| operation.operator == "Do")
        {
            let Some(name) = operation
                .operands
                .first()
                .and_then(|object| object.as_name().ok())
                .and_then(|name| std::str::from_utf8(name).ok())
            else {
                continue;
            };
            let Some(form_ref) = seed
                .xobjects
                .get(name)
                .filter(|reference| index.is_form(reference))
            else {
                continue;
            };
            seeds.push(QpdfStreamUsageSeed {
                object_ref: form_ref.clone(),
                page: seed.page,
                page_box: index
                    .form_box(form_ref)
                    .map(qpdf_box_to_page_box)
                    .or(seed.page_box),
                fonts: index.form_fonts(form_ref),
                xobjects: index.form_xobjects(form_ref),
                depth: seed.depth + 1,
            });
        }
    }

    let mut result = PlainTextCleanupResult::default();
    let mut changed_streams = BTreeMap::new();
    for (object_ref, stream_usages) in usages {
        let stream = streams
            .get(&object_ref)
            .with_context(|| format!("qpdf 未返回内容流 {object_ref}"))?;
        let original = encoded_operations(&stream.operations)?;
        let mut desired_operations = None;
        let mut desired_bytes = None;
        let mut changed = false;
        for usage in &stream_usages {
            let header_targets = active_targets(&plan.header_targets, usage.page);
            let footer_targets = active_targets(&plan.footer_targets, usage.page);
            let Some(page_box) = usage.page_box else {
                if header_targets.is_empty() && footer_targets.is_empty() {
                    continue;
                }
                anyhow::bail!("第 {} 页引用的内容流缺少坐标范围，未修改原文件", usage.page);
            };
            let page_plan = PagePlainTextPlan {
                header_targets,
                footer_targets,
                header_zone_pt: mm_to_pt(plan.header_zone_mm.max(1.0)),
                footer_zone_pt: mm_to_pt(plan.footer_zone_mm.max(1.0)),
                page_box,
            };
            let local_cmaps = local_font_cmaps(&usage.fonts, &font_cmaps);
            let (filtered, usage_result) = filter_page_operations_with_cmaps(
                &stream.operations,
                &page_plan,
                usage.page,
                &local_cmaps,
            );
            let bytes = encoded_operations(&filtered)?;
            if let Some(expected) = desired_bytes.as_ref() {
                if expected != &bytes {
                    anyhow::bail!(
                        "现有页眉页脚位于多页共享内容流中，但只在部分引用页命中；为避免修改未选页面，原文件已保留"
                    );
                }
            } else {
                desired_bytes = Some(bytes.clone());
                desired_operations = Some(filtered);
            }
            if bytes != original {
                changed = true;
                result.removed_header += usage_result.removed_header;
                result.removed_footer += usage_result.removed_footer;
                result.diagnostics.extend(usage_result.diagnostics);
            }
        }
        if changed {
            let mut edited = stream.clone();
            edited.operations = desired_operations.unwrap_or_else(|| stream.operations.clone());
            remove_unused_fallback_font_resource(&mut edited);
            changed_streams.insert(object_ref, edited);
        }
    }
    if result.removed() == 0 {
        return Ok(result);
    }
    super::qpdf_stream::update_streams(input, output_path, &changed_streams, "现有页眉页脚")?;
    Ok(result)
}

#[derive(Debug, Clone)]
struct QpdfStreamUsageSeed {
    object_ref: String,
    page: u32,
    page_box: Option<PageBox>,
    fonts: BTreeMap<String, String>,
    xobjects: BTreeMap<String, String>,
    depth: usize,
}

#[derive(Debug, Clone)]
struct QpdfStreamUsage {
    page: u32,
    page_box: Option<PageBox>,
    fonts: BTreeMap<String, String>,
}

fn local_font_cmaps(
    fonts: &BTreeMap<String, String>,
    all_cmaps: &BTreeMap<String, ToUnicodeCMap>,
) -> BTreeMap<String, ToUnicodeCMap> {
    fonts
        .iter()
        .filter_map(|(name, reference)| {
            all_cmaps
                .get(reference)
                .cloned()
                .map(|cmap| (name.clone(), cmap))
        })
        .collect()
}

fn qpdf_box_to_page_box(value: super::qpdf_stream::QpdfBox) -> PageBox {
    PageBox {
        width: (value.x1 - value.x0).abs(),
        min_y: value.y0.min(value.y1),
        max_y: value.y0.max(value.y1),
    }
}

fn encoded_operations(operations: &[Operation]) -> Result<Vec<u8>> {
    Content {
        operations: operations.to_vec(),
    }
    .encode()
    .context("编码 PDF 内容流失败")
}

fn remove_unused_fallback_font_resource(stream: &mut super::qpdf_stream::QpdfEditableStream) {
    if operations_use_font(&stream.operations, b"FCJKFallback") {
        return;
    }
    let Some(dictionary) = stream.dictionary.as_object_mut() else {
        return;
    };
    let Some(resources) = dictionary
        .get_mut("/Resources")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    let Some(fonts) = resources
        .get_mut("/Font")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    fonts.remove("/FCJKFallback");
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

#[cfg(test)]
fn filter_page_operations(
    operations: &[Operation],
    plan: &PagePlainTextPlan,
    page_number: u32,
) -> (Vec<Operation>, PlainTextCleanupResult) {
    filter_page_operations_with_cmaps(operations, plan, page_number, &BTreeMap::new())
}

fn filter_page_operations_with_cmaps(
    operations: &[Operation],
    plan: &PagePlainTextPlan,
    _page_number: u32,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
) -> (Vec<Operation>, PlainTextCleanupResult) {
    let mut output = Vec::with_capacity(operations.len());
    let mut result = PlainTextCleanupResult::default();
    let mut state = TextState::default();

    for operation in operations {
        update_text_state_before_show(&mut state, operation);
        let shown_text = shown_text_with_cmaps(operation, &state, font_cmaps);
        let mut remove_region = None;
        if let Some(text) = shown_text.as_deref() {
            // 文本匹配：必须在 zone 内 + 文本内容匹配
            let in_header_zone = is_in_header_zone(state.y, plan);
            let in_footer_zone = is_in_footer_zone(state.y, plan);
            if in_header_zone && matches_any_target_by_text(text, &plan.header_targets) {
                remove_region = Some(TextRegion::Header);
            } else if in_footer_zone && matches_any_target_by_text(text, &plan.footer_targets) {
                remove_region = Some(TextRegion::Footer);
            }
            // 诊断：记录前2页文本操作的匹配情况
            if _page_number <= 2 && !plan.header_targets.is_empty() {
                log::info!(
                    "plain_delete.match page={} y={:.1} in_header={} in_footer={} text={:?} matched={:?}",
                    _page_number, state.y, in_header_zone, in_footer_zone,
                    text.chars().take(20).collect::<String>(),
                    remove_region
                );
            }
        }
        // bbox 匹配必须独立于文本解码结果。CID 字体没有可用 CMap，或一条
        // TJ 中混入了不可解码字符时，仍可用检测阶段确认的显示范围做保守删除。
        if remove_region.is_none()
            && matches!(operation.operator.as_str(), "Tj" | "TJ" | "'" | "\"")
        {
            let page_h = plan.page_box.max_y;
            let header_by_bbox = matches_any_target_by_bbox(&state, &plan.header_targets, page_h);
            let footer_by_bbox =
                !header_by_bbox && matches_any_target_by_bbox(&state, &plan.footer_targets, page_h);
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
        "Tf" => {
            state.font_name = operation
                .operands
                .first()
                .and_then(|object| object.as_name().ok())
                .and_then(|name| std::str::from_utf8(name).ok())
                .map(str::to_string);
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

#[cfg(test)]
fn shown_text(operation: &Operation) -> Option<String> {
    shown_text_with_cmaps(operation, &TextState::default(), &BTreeMap::new())
}

fn shown_text_with_cmaps(
    operation: &Operation,
    state: &TextState,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
) -> Option<String> {
    match operation.operator.as_str() {
        "Tj" | "'" => operation
            .operands
            .first()
            .and_then(|object| object_text(object, state, font_cmaps)),
        "\"" => operation
            .operands
            .get(2)
            .and_then(|object| object_text(object, state, font_cmaps)),
        "TJ" => {
            let Object::Array(items) = operation.operands.first()? else {
                return None;
            };
            let mut text = String::new();
            for item in items {
                if matches!(item, Object::String(_, _)) {
                    let part = object_text(item, state, font_cmaps)?;
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

fn object_text(
    object: &Object,
    state: &TextState,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
) -> Option<String> {
    let Object::String(bytes, _) = object else {
        return None;
    };
    if let Some(font_name) = state.font_name.as_deref() {
        if let Some(cmap) = font_cmaps.get(font_name) {
            return cmap.decode(bytes);
        }
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).ok();
    }
    // Ordinary content streams often use PDFDocEncoding or a legacy CJK
    // single-byte encoding even when no ToUnicode CMap is attached. Reuse the
    // artifact decoder here instead of silently treating those strings as
    // undecodable; bbox fallback remains available when it still returns None.
    super::artifacts::decode_pdf_string(object)
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

fn matches_any_target_by_bbox(
    state: &TextState,
    targets: &[&PlainTextTarget],
    page_height: f32,
) -> bool {
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
    let height = if page_height > 0.0 {
        page_height
    } else {
        bbox.height
    };
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

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Operation;
    use lopdf::dictionary;
    use lopdf::{Dictionary, Document, Stream};

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
    fn decodes_cid_text_with_current_font_tounicode_map() {
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"FCID".to_vec()), 12.into()]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    120.into(),
                    812.into(),
                ],
            ),
            Operation::new(
                "Tj",
                vec![Object::String(
                    vec![0, 1, 0, 2],
                    lopdf::StringFormat::Hexadecimal,
                )],
            ),
            Operation::new("ET", vec![]),
        ];
        let target = PlainTextTarget {
            text: "中文".to_string(),
            normalized_text: "中文".to_string(),
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
        let cmap = ToUnicodeCMap::parse(
            br#"
            1 begincodespacerange <0000> <00ff> endcodespacerange
            2 beginbfchar <0001> <4E2D> <0002> <6587> endbfchar
            "#,
        )
        .unwrap();
        let font_cmaps = BTreeMap::from([(String::from("FCID"), cmap)]);
        let (filtered, result) =
            filter_page_operations_with_cmaps(&operations, &plan, 1, &font_cmaps);
        assert_eq!(result.removed_header, 1);
        assert!(filtered.iter().all(|operation| operation.operator != "Tj"));
    }

    #[test]
    fn decodes_pdf_doc_encoding_in_plain_content_streams() {
        let operation = Operation::new(
            "Tj",
            vec![Object::String(
                b"Header \x8dQuoted\x8e".to_vec(),
                lopdf::StringFormat::Literal,
            )],
        );

        assert_eq!(shown_text(&operation), Some("Header “Quoted”".to_string()));
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

