use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{decode_text_string, Dictionary, Document, Object, ObjectId, StringFormat};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{cmap::ToUnicodeCMap, same_path, temp_named_path, text_utils::normalize_for_match};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteHeaderFooterArtifactsArgs {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(alias = "output")]
    output_path: String,
    #[serde(default)]
    remove_header: bool,
    #[serde(default)]
    remove_footer: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteHeaderFooterArtifactsResult {
    pub(crate) input_path: String,
    pub(crate) output_path: String,
    pub(crate) removed: usize,
    pub(crate) removed_header: usize,
    pub(crate) removed_footer: usize,
    pub(crate) pages_touched: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HeaderFooterArtifactTargets {
    pub header: bool,
    pub footer: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ArtifactRemovalStats {
    pub header: usize,
    pub footer: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HeaderFooterArtifactInspection {
    pub header_count: usize,
    pub footer_count: usize,
    pub occurrences: Vec<HeaderFooterArtifactOccurrence>,
}

#[derive(Debug, Clone)]
pub(crate) struct HeaderFooterArtifactOccurrence {
    pub id: String,
    pub page: u32,
    pub region: &'static str,
    pub text: Option<String>,
    pub docsy_kind: Option<String>,
    pub docsy_id: Option<String>,
    pub object_ref: Option<String>,
    pub operation_index: usize,
    pub visual_object_ref: Option<String>,
}

impl HeaderFooterArtifactInspection {
    fn add_region(&mut self, region: ArtifactRegion) {
        match region {
            ArtifactRegion::Header => self.header_count += 1,
            ArtifactRegion::Footer => self.footer_count += 1,
        }
    }

    fn merge(&mut self, other: Self) {
        self.header_count += other.header_count;
        self.footer_count += other.footer_count;
        self.occurrences.extend(other.occurrences);
    }
}

impl ArtifactRemovalStats {
    fn total(self) -> usize {
        self.header + self.footer
    }

    fn add_region(&mut self, region: ArtifactRegion) {
        match region {
            ArtifactRegion::Header => self.header += 1,
            ArtifactRegion::Footer => self.footer += 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ArtifactRegion {
    Header,
    Footer,
}

pub(crate) fn inspect_meaningful_header_footer_artifacts(
    input_path: &Path,
    max_pages: u32,
) -> Result<HeaderFooterArtifactInspection> {
    inspect_meaningful_header_footer_artifacts_qpdf(input_path, max_pages)
}

fn inspect_meaningful_header_footer_artifacts_qpdf(
    input_path: &Path,
    max_pages: u32,
) -> Result<HeaderFooterArtifactInspection> {
    let index = super::qpdf_stream::QpdfObjectIndex::load(input_path)?;
    let font_cmaps = super::cmap::load_font_cmaps(input_path, &index)?;
    let pages = index
        .pages()
        .iter()
        .take(if max_pages == 0 {
            usize::MAX
        } else {
            max_pages as usize
        })
        .cloned()
        .collect::<Vec<_>>();
    let direct_refs = pages
        .iter()
        .flat_map(|page| page.contents.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut streams =
        super::qpdf_stream::load_editable_streams(input_path, &direct_refs, "检测标准页眉页脚")?;
    let mut frontier = BTreeSet::new();
    for page in &pages {
        for content_ref in &page.contents {
            if let Some(stream) = streams.get(content_ref) {
                collect_invoked_forms(&stream.operations, &page.xobjects, &index, &mut frontier);
            }
        }
    }
    let mut loaded_forms = BTreeSet::new();
    while !frontier.is_empty() {
        let pending = frontier
            .iter()
            .filter(|reference| !streams.contains_key(*reference))
            .cloned()
            .collect::<Vec<_>>();
        if !pending.is_empty() {
            streams.extend(super::qpdf_stream::load_editable_streams(
                input_path,
                &pending,
                "检测标准页眉页脚 Form",
            )?);
        }
        let current = std::mem::take(&mut frontier);
        for form_ref in current {
            if !loaded_forms.insert(form_ref.clone()) {
                continue;
            }
            let Some(stream) = streams.get(&form_ref) else {
                continue;
            };
            let xobjects = index.form_xobjects(&form_ref);
            collect_invoked_forms(&stream.operations, &xobjects, &index, &mut frontier);
        }
        frontier.retain(|reference| !loaded_forms.contains(reference));
    }

    let mut result = HeaderFooterArtifactInspection::default();
    let started = std::time::Instant::now();
    for page in &pages {
        for content_ref in &page.contents {
            let stream = streams
                .get(content_ref)
                .with_context(|| format!("qpdf 未返回页面内容流 {content_ref}"))?;
            let path = format!("page:{}/content:{content_ref}", page.number);
            let mut direct = inspect_artifact_operations_detailed_qpdf(
                &stream.operations,
                &page.properties,
                &page.fonts,
                &font_cmaps,
                page.number,
                &path,
                content_ref,
            );
            attach_artifact_visual_forms(&mut direct, &stream.operations, &page.xobjects);
            result.merge(direct);
            inspect_qpdf_referenced_forms(
                &index,
                &streams,
                &stream.operations,
                &page.xobjects,
                page.number,
                &font_cmaps,
                &path,
                0,
                &mut BTreeSet::new(),
                &mut result,
            )?;
        }
        if page.number % 25 == 0 || page.number as usize == pages.len() {
            crate::app_log::info(
                "pdf.artifact.scan",
                "progress",
                serde_json::json!({
                    "file": input_path.to_string_lossy(),
                    "engine": "qpdf-index",
                    "pagesDone": page.number,
                    "pagesTotal": pages.len(),
                    "decodedStreams": streams.len(),
                    "headers": result.header_count,
                    "footers": result.footer_count,
                    "elapsedMs": started.elapsed().as_millis(),
                }),
            );
        }
    }
    Ok(result)
}

fn collect_invoked_forms(
    operations: &[Operation],
    xobjects: &BTreeMap<String, String>,
    index: &super::qpdf_stream::QpdfObjectIndex,
    output: &mut BTreeSet<String>,
) {
    for operation in operations
        .iter()
        .filter(|operation| operation.operator == "Do")
    {
        let Some(name) = operation.operands.first().and_then(name_bytes) else {
            continue;
        };
        let Ok(name) = std::str::from_utf8(name) else {
            continue;
        };
        if let Some(reference) = xobjects.get(name).filter(|value| index.is_form(value)) {
            output.insert(reference.clone());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect_qpdf_referenced_forms(
    index: &super::qpdf_stream::QpdfObjectIndex,
    streams: &BTreeMap<String, super::qpdf_stream::QpdfEditableStream>,
    operations: &[Operation],
    xobjects: &BTreeMap<String, String>,
    page: u32,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
    page_path: &str,
    depth: usize,
    visited: &mut BTreeSet<String>,
    result: &mut HeaderFooterArtifactInspection,
) -> Result<()> {
    if depth >= 128 {
        anyhow::bail!("标准页眉页脚 Form 嵌套超过安全上限");
    }
    for operation in operations
        .iter()
        .filter(|operation| operation.operator == "Do")
    {
        let Some(name) = operation.operands.first().and_then(name_bytes) else {
            continue;
        };
        let Ok(name) = std::str::from_utf8(name) else {
            continue;
        };
        let Some(object_ref) = xobjects.get(name).filter(|value| index.is_form(value)) else {
            continue;
        };
        if !visited.insert(object_ref.clone()) {
            continue;
        }
        let stream = streams
            .get(object_ref)
            .with_context(|| format!("qpdf 未返回标准页眉页脚 Form {object_ref}"))?;
        let properties = index.form_properties(object_ref);
        let nested_xobjects = index.form_xobjects(object_ref);
        let path = format!("{page_path}/form:{object_ref}");
        let mut inspection = inspect_artifact_operations_detailed_qpdf(
            &stream.operations,
            &properties,
            &index.form_fonts(object_ref),
            font_cmaps,
            page,
            &path,
            object_ref,
        );
        attach_artifact_visual_forms(&mut inspection, &stream.operations, &nested_xobjects);
        result.merge(inspection);
        inspect_qpdf_referenced_forms(
            index,
            streams,
            &stream.operations,
            &nested_xobjects,
            page,
            font_cmaps,
            &path,
            depth + 1,
            visited,
            result,
        )?;
    }
    Ok(())
}

#[cfg(test)]
fn inspect_artifact_operations(
    operations: &[Operation],
    properties: &Dictionary,
) -> HeaderFooterArtifactInspection {
    inspect_artifact_operations_detailed(operations, properties, 0, "content")
}

#[cfg(test)]
fn inspect_artifact_operations_detailed(
    operations: &[Operation],
    properties: &Dictionary,
    page: u32,
    path: &str,
) -> HeaderFooterArtifactInspection {
    inspect_artifact_operations_detailed_inner(
        operations,
        properties,
        &BTreeMap::new(),
        &BTreeMap::new(),
        page,
        path,
        None,
    )
}

fn inspect_artifact_operations_detailed_qpdf(
    operations: &[Operation],
    properties: &Dictionary,
    fonts: &BTreeMap<String, String>,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
    page: u32,
    path: &str,
    object_ref: &str,
) -> HeaderFooterArtifactInspection {
    inspect_artifact_operations_detailed_inner(
        operations,
        properties,
        fonts,
        font_cmaps,
        page,
        path,
        Some(object_ref),
    )
}

fn inspect_artifact_operations_detailed_inner(
    operations: &[Operation],
    properties: &Dictionary,
    fonts: &BTreeMap<String, String>,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
    page: u32,
    path: &str,
    object_ref: Option<&str>,
) -> HeaderFooterArtifactInspection {
    let mut result = HeaderFooterArtifactInspection::default();
    let targets = HeaderFooterArtifactTargets {
        header: true,
        footer: true,
    };
    let mut index = 0_usize;
    while index < operations.len() {
        if let Some(region) = target_artifact_region(&operations[index], targets, properties) {
            if let Some(end) = matching_marked_content_end(operations, index) {
                if artifact_range_has_meaningful_text(&operations[index + 1..end]) {
                    result.add_region(region);
                    let property = artifact_property_dictionary(&operations[index], properties);
                    let text = property
                        .and_then(|dict| dict.get(b"ActualText").ok())
                        .and_then(decode_pdf_string)
                        .or_else(|| {
                            artifact_range_text(&operations[index + 1..end], fonts, font_cmaps)
                        });
                    let docsy_kind = property
                        .and_then(|dict| dict.get(b"DocsyKind").ok())
                        .and_then(name_bytes)
                        .and_then(|value| String::from_utf8(value.to_vec()).ok());
                    let docsy_id = property
                        .and_then(|dict| dict.get(b"DocsyId").ok())
                        .and_then(decode_pdf_string);
                    result.occurrences.push(HeaderFooterArtifactOccurrence {
                        id: docsy_id
                            .clone()
                            .unwrap_or_else(|| format!("{path}:artifact:{index}")),
                        page,
                        region: match region {
                            ArtifactRegion::Header => "header",
                            ArtifactRegion::Footer => "footer",
                        },
                        text,
                        docsy_kind,
                        docsy_id,
                        object_ref: object_ref.map(str::to_string),
                        operation_index: index,
                        visual_object_ref: None,
                    });
                }
                index = end + 1;
                continue;
            }
        }
        index += 1;
    }
    result
}

fn attach_artifact_visual_forms(
    inspection: &mut HeaderFooterArtifactInspection,
    operations: &[Operation],
    xobjects: &BTreeMap<String, String>,
) {
    for occurrence in &mut inspection.occurrences {
        let Some(end) = matching_marked_content_end(operations, occurrence.operation_index) else {
            continue;
        };
        occurrence.visual_object_ref = operations[occurrence.operation_index..=end]
            .iter()
            .filter(|operation| operation.operator == "Do")
            .find_map(|operation| {
                let name = operation.operands.first().and_then(name_bytes)?;
                let name = std::str::from_utf8(name).ok()?;
                xobjects.get(name).cloned()
            });
    }
}

fn artifact_property_dictionary<'a>(
    operation: &'a Operation,
    properties: &'a Dictionary,
) -> Option<&'a Dictionary> {
    let property = operation.operands.get(1)?;
    match property {
        Object::Dictionary(dict) => Some(dict),
        Object::Name(name) => properties.get(name).ok()?.as_dict().ok(),
        _ => None,
    }
}

pub(crate) fn decode_pdf_string(object: &Object) -> Option<String> {
    let Object::String(bytes, _) = object else {
        return None;
    };
    if bytes.starts_with(&[0xfe, 0xff]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).ok();
    }
    String::from_utf8(bytes.clone())
        .ok()
        .or_else(|| decode_legacy_cjk_pdf_string(bytes).or_else(|| decode_text_string(object).ok()))
}

fn decode_legacy_cjk_pdf_string(bytes: &[u8]) -> Option<String> {
    if !bytes.iter().any(|byte| *byte >= 0x80) {
        return None;
    }
    for encoding in [
        encoding_rs::GB18030,
        encoding_rs::BIG5,
        encoding_rs::SHIFT_JIS,
        encoding_rs::EUC_KR,
    ] {
        let (decoded, _, had_errors) = encoding.decode(bytes);
        if had_errors {
            continue;
        }
        let value = decoded.trim();
        if !value.is_empty() && value.chars().any(is_cjk_character) {
            return Some(value.to_string());
        }
    }
    None
}

fn is_cjk_character(character: char) -> bool {
    matches!(character as u32,
        0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xF900..=0xFAFF
        | 0x3040..=0x30FF
        | 0xAC00..=0xD7AF)
}

fn artifact_range_text(
    operations: &[Operation],
    fonts: &BTreeMap<String, String>,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
) -> Option<String> {
    let mut text = String::new();
    let mut font_name = None;
    for operation in operations {
        if operation.operator == "Tf" {
            font_name = operation
                .operands
                .first()
                .and_then(name_bytes)
                .and_then(|name| std::str::from_utf8(name).ok());
            continue;
        }
        match operation.operator.as_str() {
            "Tj" | "'" => {
                let value = operation.operands.first().and_then(|object| {
                    artifact_object_text(object, font_name, fonts, font_cmaps)
                })?;
                text.push_str(&value);
            }
            "\"" => {
                let value = operation.operands.get(2).and_then(|object| {
                    artifact_object_text(object, font_name, fonts, font_cmaps)
                })?;
                text.push_str(&value);
            }
            "TJ" => {
                if let Some(items) = operation
                    .operands
                    .first()
                    .and_then(|value| value.as_array().ok())
                {
                    for item in items {
                        if matches!(item, Object::String(_, _)) {
                            let value = artifact_object_text(item, font_name, fonts, font_cmaps)?;
                            text.push_str(&value);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn artifact_object_text(
    object: &Object,
    font_name: Option<&str>,
    fonts: &BTreeMap<String, String>,
    font_cmaps: &BTreeMap<String, ToUnicodeCMap>,
) -> Option<String> {
    let Object::String(bytes, _) = object else {
        return None;
    };
    if let Some(font_ref) = font_name.and_then(|name| fonts.get(name)) {
        if let Some(cmap) = font_cmaps.get(font_ref) {
            return cmap.decode(bytes);
        }
    }
    decode_pdf_string(object)
}

fn artifact_range_has_meaningful_text(operations: &[Operation]) -> bool {
    operations
        .iter()
        .any(|operation| match operation.operator.as_str() {
            "Tj" | "'" => operation
                .operands
                .first()
                .is_some_and(text_object_has_meaningful_bytes),
            "\"" => operation
                .operands
                .get(2)
                .is_some_and(text_object_has_meaningful_bytes),
            "TJ" => operation
                .operands
                .first()
                .and_then(|object| object.as_array().ok())
                .is_some_and(|items| items.iter().any(text_object_has_meaningful_bytes)),
            _ => false,
        })
}

fn text_object_has_meaningful_bytes(object: &Object) -> bool {
    let Object::String(bytes, _) = object else {
        return false;
    };
    bytes
        .iter()
        .any(|byte| !byte.is_ascii_whitespace() && *byte != 0)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HeaderFooterArtifactEditPlan {
    pub remove_header: bool,
    pub remove_footer: bool,
    pub header_texts: Vec<String>,
    pub footer_texts: Vec<String>,
    pub header_targets: Vec<HeaderFooterArtifactEditTarget>,
    pub footer_targets: Vec<HeaderFooterArtifactEditTarget>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HeaderFooterArtifactEditTarget {
    pub artifact_id: Option<String>,
    pub normalized_text: String,
    pub page_start: u32,
    pub page_end: u32,
    pub docsy_kind: Option<String>,
    pub replacement_text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HeaderFooterArtifactEditResult {
    pub removed_header: usize,
    pub removed_footer: usize,
    pub edited_header: usize,
    pub edited_footer: usize,
    pub removed_header_pages: BTreeSet<usize>,
    pub removed_footer_pages: BTreeSet<usize>,
    pub edited_header_pages: BTreeSet<usize>,
    pub edited_footer_pages: BTreeSet<usize>,
}

impl HeaderFooterArtifactEditResult {
    pub(crate) fn changed_count(&self) -> usize {
        self.removed_header + self.removed_footer + self.edited_header + self.edited_footer
    }
}

pub fn delete_header_footer_artifacts(
    args: &serde_json::Value,
) -> Result<DeleteHeaderFooterArtifactsResult> {
    let args: DeleteHeaderFooterArtifactsArgs =
        serde_json::from_value(args.clone()).context("解析标准页眉页脚删除参数失败")?;
    delete_header_footer_artifacts_file(
        &args.input_path,
        &args.output_path,
        HeaderFooterArtifactTargets {
            header: args.remove_header,
            footer: args.remove_footer,
        },
    )
}

pub fn delete_header_footer_artifacts_file(
    input_path: &str,
    output_path: &str,
    targets: HeaderFooterArtifactTargets,
) -> Result<DeleteHeaderFooterArtifactsResult> {
    let input = Path::new(input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }
    let output = Path::new(output_path);
    if same_path(input, output) {
        anyhow::bail!("标准页眉页脚删除输出路径不能和原始 PDF 相同");
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("创建标准页眉页脚删除输出目录失败")?;
        }
    }
    if !targets.header && !targets.footer {
        std::fs::copy(input, output_path).context("复制 PDF 失败")?;
        return Ok(DeleteHeaderFooterArtifactsResult {
            input_path: input_path.to_string(),
            output_path: output_path.to_string(),
            removed: 0,
            removed_header: 0,
            removed_footer: 0,
            pages_touched: 0,
        });
    }
    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    let mut removed = ArtifactRemovalStats::default();
    let mut pages_touched = 0_usize;

    for page_id in page_ids {
        let content = match doc.get_and_decode_page_content(page_id) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let properties = page_properties(&doc, page_id);
        let (filtered, removed_on_page) =
            remove_target_artifact_ranges(&content.operations, targets, &properties);
        let mut nested_result = HeaderFooterArtifactEditResult::default();
        let xobjects = page_xobjects(&doc, page_id);
        if !xobjects.is_empty() {
            let plan = HeaderFooterArtifactEditPlan {
                remove_header: targets.header,
                remove_footer: targets.footer,
                ..Default::default()
            };
            nested_result = edit_referenced_form_artifacts(
                &mut doc,
                &content.operations,
                &xobjects,
                &plan,
                0,
                &mut BTreeSet::new(),
            )?;
        }
        if removed_on_page.total() > 0 {
            let encoded = Content {
                operations: filtered,
            }
            .encode()
            .context("编码删除标准页眉页脚后的内容流失败")?;
            doc.change_page_content(page_id, encoded)
                .context("写回删除标准页眉页脚后的内容流失败")?;
        }
        let nested_removed = nested_result.removed_header + nested_result.removed_footer;
        if removed_on_page.total() > 0 || nested_removed > 0 {
            removed.header += removed_on_page.header + nested_result.removed_header;
            removed.footer += removed_on_page.footer + nested_result.removed_footer;
            pages_touched += 1;
        }
    }

    doc.prune_objects();
    doc.save(output)
        .context("保存删除标准页眉页脚后的 PDF 失败")?;

    Ok(DeleteHeaderFooterArtifactsResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        removed: removed.total(),
        removed_header: removed.header,
        removed_footer: removed.footer,
        pages_touched,
    })
}

pub(crate) fn edit_header_footer_artifacts_to_temp(
    input_path: &str,
    plan: &HeaderFooterArtifactEditPlan,
) -> Result<Option<(PathBuf, HeaderFooterArtifactEditResult)>> {
    if !plan.remove_header && !plan.remove_footer {
        return Ok(None);
    }
    let output = temp_named_path("docsy_hf_artifacts_edited", "pdf");
    let result = edit_header_footer_artifacts_file(input_path, &output, plan)?;
    if result.changed_count() == 0 {
        let _ = std::fs::remove_file(&output);
        return Ok(None);
    }
    Ok(Some((output, result)))
}

fn edit_header_footer_artifacts_file(
    input_path: &str,
    output_path: &Path,
    plan: &HeaderFooterArtifactEditPlan,
) -> Result<HeaderFooterArtifactEditResult> {
    edit_header_footer_artifacts_qpdf(Path::new(input_path), output_path, plan)
}

#[derive(Debug, Clone)]
struct SelectedArtifactEdit {
    region: ArtifactRegion,
    operation_index: usize,
    pages: BTreeSet<u32>,
    replacement: Option<String>,
    visual_object_ref: Option<String>,
}

fn edit_header_footer_artifacts_qpdf(
    input: &Path,
    output: &Path,
    plan: &HeaderFooterArtifactEditPlan,
) -> Result<HeaderFooterArtifactEditResult> {
    let inspection = inspect_meaningful_header_footer_artifacts_qpdf(input, 0)?;
    type EditKey = (String, usize, String);
    let mut referenced_pages: BTreeMap<EditKey, BTreeSet<u32>> = BTreeMap::new();
    let mut selected: BTreeMap<EditKey, Vec<(&HeaderFooterArtifactOccurrence, Option<String>)>> =
        BTreeMap::new();
    for occurrence in &inspection.occurrences {
        let Some(object_ref) = occurrence.object_ref.as_ref() else {
            continue;
        };
        let key = (
            object_ref.clone(),
            occurrence.operation_index,
            occurrence.region.to_string(),
        );
        referenced_pages
            .entry(key.clone())
            .or_default()
            .insert(occurrence.page);
        let targets = if occurrence.region == "header" {
            &plan.header_targets
        } else {
            &plan.footer_targets
        };
        let selected_target = targets
            .iter()
            .find(|target| artifact_occurrence_matches_target(occurrence, target));
        let legacy_selected = targets.is_empty()
            && if occurrence.region == "header" {
                plan.remove_header
            } else {
                plan.remove_footer
            };
        if selected_target.is_none() && !legacy_selected {
            continue;
        }
        if let Some(target) = selected_target {
            if target.page_end > target.page_start
                && (target.normalized_text.contains("{page}")
                    || target.normalized_text.contains("{roman-page}"))
                && target
                    .replacement_text
                    .as_deref()
                    .is_some_and(|replacement| {
                        !replacement.contains("{page}") && !replacement.contains("{roman-page}")
                    })
            {
                anyhow::bail!(
                    "多页页码的编辑内容必须包含 {{page}} 或 {{roman-page}}；原文件已保留"
                );
            }
        }
        let replacement = selected_target
            .and_then(|target| target.replacement_text.as_deref())
            .map(|template| {
                expand_artifact_replacement(
                    template,
                    occurrence.page,
                    selected_target
                        .map(|target| target.page_end.max(target.page_start))
                        .unwrap_or(occurrence.page),
                )
            })
            .or_else(|| {
                let values = if occurrence.region == "header" {
                    &plan.header_texts
                } else {
                    &plan.footer_texts
                };
                values
                    .get(occurrence.page.saturating_sub(1) as usize)
                    .filter(|value| !value.is_empty())
                    .cloned()
            });
        selected
            .entry(key)
            .or_default()
            .push((occurrence, replacement));
    }
    if selected.is_empty() {
        return Ok(HeaderFooterArtifactEditResult::default());
    }

    let mut edits_by_object: BTreeMap<String, Vec<SelectedArtifactEdit>> = BTreeMap::new();
    for ((object_ref, operation_index, region), occurrences) in selected {
        let selected_pages = occurrences
            .iter()
            .map(|(occurrence, _)| occurrence.page)
            .collect::<BTreeSet<_>>();
        let all_pages = referenced_pages
            .get(&(object_ref.clone(), operation_index, region.clone()))
            .cloned()
            .unwrap_or_default();
        if selected_pages != all_pages {
            anyhow::bail!(
                "标准页眉页脚位于共享内容流中，当前只选择了部分引用页（已选 {:?}，全部 {:?}）；原文件已保留",
                selected_pages,
                all_pages
            );
        }
        let replacements = occurrences
            .iter()
            .filter_map(|(_, replacement)| replacement.clone())
            .collect::<BTreeSet<_>>();
        if replacements.len() > 1 {
            anyhow::bail!("共享页眉页脚设置了不同替换文字；原文件已保留，请先删除后重新插入");
        }
        edits_by_object
            .entry(object_ref)
            .or_default()
            .push(SelectedArtifactEdit {
                region: if region == "header" {
                    ArtifactRegion::Header
                } else {
                    ArtifactRegion::Footer
                },
                operation_index,
                pages: selected_pages,
                replacement: replacements.into_iter().next(),
                visual_object_ref: occurrences
                    .iter()
                    .find_map(|(occurrence, _)| occurrence.visual_object_ref.clone()),
            });
    }
    validate_visual_replacements(&edits_by_object)?;
    let references = edits_by_object
        .iter()
        .flat_map(|(reference, edits)| {
            std::iter::once(reference.clone()).chain(
                edits
                    .iter()
                    .filter(|edit| edit.replacement.is_some())
                    .filter_map(|edit| edit.visual_object_ref.clone()),
            )
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut streams =
        super::qpdf_stream::load_editable_streams(input, &references, "编辑标准页眉页脚")?;
    apply_visual_replacements(&mut streams, &edits_by_object)?;
    let mut result = HeaderFooterArtifactEditResult::default();
    for (object_ref, edits) in &mut edits_by_object {
        let stream = streams
            .get_mut(object_ref)
            .with_context(|| format!("qpdf 未返回标准页眉页脚内容流 {object_ref}"))?;
        edits.sort_by_key(|edit| std::cmp::Reverse(edit.operation_index));
        for edit in edits {
            let Some(end) = matching_marked_content_end(&stream.operations, edit.operation_index)
            else {
                anyhow::bail!("标准页眉页脚标记范围不完整：{object_ref}");
            };
            let edited = if let Some(replacement) = edit.replacement.as_deref() {
                let mut range = stream.operations[edit.operation_index..=end].to_vec();
                let visible_updated = edit.visual_object_ref.is_some()
                    || replace_first_text_show(&mut range, replacement);
                if !visible_updated {
                    anyhow::bail!("标准页眉页脚字体编码不支持安全原位编辑；原文件已保留");
                }
                if !replace_inline_artifact_semantic_text(&mut range, replacement) {
                    anyhow::bail!(
                        "标准页眉页脚使用命名属性保存语义文字，不能安全同步编辑；原文件已保留"
                    );
                }
                stream.operations.splice(edit.operation_index..=end, range);
                true
            } else {
                stream.operations.drain(edit.operation_index..=end);
                false
            };
            for page in &edit.pages {
                let page_index = page.saturating_sub(1) as usize;
                match (edit.region, edited) {
                    (ArtifactRegion::Header, true) => {
                        result.edited_header += 1;
                        result.edited_header_pages.insert(page_index);
                    }
                    (ArtifactRegion::Footer, true) => {
                        result.edited_footer += 1;
                        result.edited_footer_pages.insert(page_index);
                    }
                    (ArtifactRegion::Header, false) => {
                        result.removed_header += 1;
                        result.removed_header_pages.insert(page_index);
                    }
                    (ArtifactRegion::Footer, false) => {
                        result.removed_footer += 1;
                        result.removed_footer_pages.insert(page_index);
                    }
                }
            }
        }
    }
    super::qpdf_stream::update_streams(input, output, &streams, "标准页眉页脚")?;
    Ok(result)
}

fn merge_edit_result(
    target: &mut HeaderFooterArtifactEditResult,
    source: HeaderFooterArtifactEditResult,
) {
    target.removed_header += source.removed_header;
    target.removed_footer += source.removed_footer;
    target.edited_header += source.edited_header;
    target.edited_footer += source.edited_footer;
    target
        .removed_header_pages
        .extend(source.removed_header_pages);
    target
        .removed_footer_pages
        .extend(source.removed_footer_pages);
    target
        .edited_header_pages
        .extend(source.edited_header_pages);
    target
        .edited_footer_pages
        .extend(source.edited_footer_pages);
}

fn edit_target_artifact_ranges(
    operations: &[Operation],
    plan: &HeaderFooterArtifactEditPlan,
    properties: &Dictionary,
    page_index: usize,
) -> (Vec<Operation>, HeaderFooterArtifactEditResult) {
    let mut output = Vec::with_capacity(operations.len());
    let mut result = HeaderFooterArtifactEditResult::default();
    let targets = HeaderFooterArtifactTargets {
        header: plan.remove_header,
        footer: plan.remove_footer,
    };
    let mut index = 0_usize;

    while index < operations.len() {
        if let Some(region) = target_artifact_region(&operations[index], targets, properties) {
            if let Some(end) = matching_marked_content_end(operations, index) {
                let replacement = match region {
                    ArtifactRegion::Header => plan.header_texts.get(page_index),
                    ArtifactRegion::Footer => plan.footer_texts.get(page_index),
                };
                if let Some(replacement) = replacement.filter(|value| !value.is_empty()) {
                    let mut range = operations[index..=end].to_vec();
                    if replace_first_text_show(&mut range, replacement) {
                        match region {
                            ArtifactRegion::Header => {
                                result.edited_header += 1;
                                result.edited_header_pages.insert(page_index);
                            }
                            ArtifactRegion::Footer => {
                                result.edited_footer += 1;
                                result.edited_footer_pages.insert(page_index);
                            }
                        }
                        output.extend(range);
                        index = end + 1;
                        continue;
                    }
                }
                match region {
                    ArtifactRegion::Header => {
                        result.removed_header += 1;
                        result.removed_header_pages.insert(page_index);
                    }
                    ArtifactRegion::Footer => {
                        result.removed_footer += 1;
                        result.removed_footer_pages.insert(page_index);
                    }
                }
                index = end + 1;
                continue;
            }
        }
        output.push(operations[index].clone());
        index += 1;
    }

    (output, result)
}

fn replace_first_text_show(operations: &mut [Operation], replacement: &str) -> bool {
    let mut replaced = false;
    for operation in operations {
        match operation.operator.as_str() {
            "Tj" | "'" => {
                if let Some(object) = operation.operands.first_mut() {
                    if replaced {
                        empty_string_object(object);
                    } else if replace_string_object(object, replacement) {
                        replaced = true;
                    }
                }
            }
            "\"" => {
                if let Some(object) = operation.operands.get_mut(2) {
                    if replaced {
                        empty_string_object(object);
                    } else if replace_string_object(object, replacement) {
                        replaced = true;
                    }
                }
            }
            "TJ" => {
                if let Some(Object::Array(items)) = operation.operands.first_mut() {
                    if replaced {
                        empty_all_string_items(items);
                    } else {
                        let Some(first_string_index) = items
                            .iter()
                            .position(|item| matches!(item, Object::String(_, _)))
                        else {
                            continue;
                        };
                        let Some(replacement_object) =
                            replacement_object_like(&items[first_string_index], replacement)
                        else {
                            continue;
                        };
                        for (index, item) in items.iter_mut().enumerate() {
                            if matches!(item, Object::String(_, _)) {
                                *item = if index == first_string_index {
                                    replacement_object.clone()
                                } else {
                                    empty_string_like(item)
                                };
                            }
                        }
                        replaced = true;
                    }
                }
            }
            _ => {}
        }
    }
    replaced
}

fn artifact_occurrence_matches_target(
    occurrence: &HeaderFooterArtifactOccurrence,
    target: &HeaderFooterArtifactEditTarget,
) -> bool {
    let page_start = target.page_start.max(1);
    let page_end = target.page_end.max(page_start);
    if occurrence.page < page_start || occurrence.page > page_end {
        return false;
    }
    if let (Some(expected), Some(actual)) = (
        target.docsy_kind.as_deref(),
        occurrence.docsy_kind.as_deref(),
    ) {
        if expected != actual {
            return false;
        }
    }
    if let Some(expected) = target.artifact_id.as_deref() {
        if occurrence.id == expected || occurrence.docsy_id.as_deref() == Some(expected) {
            return true;
        }
    }
    occurrence
        .text
        .as_deref()
        .is_some_and(|text| artifact_selector_text_matches(text, &target.normalized_text))
}

fn artifact_selector_text_matches(actual: &str, expected: &str) -> bool {
    let actual = normalize_artifact_match_text(actual);
    let expected = normalize_artifact_match_text(expected);
    if actual == expected {
        return true;
    }
    if expected.contains("{page}") || expected.contains("{total}") {
        let pattern = expected
            .replace("{page}", "__DOCSY_PAGE__")
            .replace("{total}", "__DOCSY_TOTAL__");
        let escaped = regex::escape(&pattern)
            .replace("__DOCSY_PAGE__", r"\d+")
            .replace("__DOCSY_TOTAL__", r"\d+");
        return regex::Regex::new(&format!("^{escaped}$"))
            .is_ok_and(|regex| regex.is_match(&actual));
    }
    false
}

fn normalize_artifact_match_text(text: &str) -> String {
    normalize_for_match(text)
}

fn expand_artifact_replacement(template: &str, page: u32, total: u32) -> String {
    let roman = roman_page_number(page);
    template
        .replace("{range}", &format!("{page}/{total}"))
        .replace("{roman-page}", &roman)
        .replace("{page}", &page.to_string())
        .replace("{total}", &total.to_string())
}

fn roman_page_number(mut value: u32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut output = String::new();
    for (number, numeral) in [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while value >= number {
            output.push_str(numeral);
            value -= number;
        }
    }
    output
}

fn validate_visual_replacements(
    edits_by_object: &BTreeMap<String, Vec<SelectedArtifactEdit>>,
) -> Result<()> {
    let mut replacements: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edit in edits_by_object.values().flatten() {
        if let (Some(reference), Some(replacement)) =
            (edit.visual_object_ref.as_ref(), edit.replacement.as_ref())
        {
            replacements
                .entry(reference.clone())
                .or_default()
                .insert(replacement.clone());
        }
    }
    if let Some((reference, _)) = replacements.iter().find(|(_, values)| values.len() > 1) {
        anyhow::bail!("共享可见文字流 {reference} 设置了不同替换文字；原文件已保留");
    }
    Ok(())
}

fn apply_visual_replacements(
    streams: &mut BTreeMap<String, super::qpdf_stream::QpdfEditableStream>,
    edits_by_object: &BTreeMap<String, Vec<SelectedArtifactEdit>>,
) -> Result<()> {
    let replacements = edits_by_object
        .values()
        .flatten()
        .filter_map(|edit| Some((edit.visual_object_ref.clone()?, edit.replacement.clone()?)))
        .collect::<BTreeMap<_, _>>();
    for (reference, replacement) in replacements {
        let stream = streams
            .get_mut(&reference)
            .with_context(|| format!("qpdf 未返回页眉页脚可见文字流 {reference}"))?;
        if !replace_first_text_show(&mut stream.operations, &replacement) {
            anyhow::bail!("页眉页脚可见文字位于不支持原位编辑的嵌套结构中；原文件已保留");
        }
    }
    Ok(())
}

fn replace_inline_artifact_semantic_text(operations: &mut [Operation], replacement: &str) -> bool {
    let Some(start) = operations.first_mut() else {
        return false;
    };
    let Some(property) = start.operands.get_mut(1) else {
        return true;
    };
    let Object::Dictionary(dictionary) = property else {
        return !matches!(property, Object::Name(_));
    };
    let mut found = false;
    for key in [b"ActualText".as_slice(), b"Contents".as_slice()] {
        if let Ok(value) = dictionary.get_mut(key) {
            found |= replace_string_object(value, replacement);
        }
    }
    if !found {
        dictionary.set(
            b"ActualText".to_vec(),
            Object::String(
                encode_utf16be_pdf_string(replacement),
                StringFormat::Hexadecimal,
            ),
        );
    }
    true
}

fn replace_string_object(object: &mut Object, replacement: &str) -> bool {
    let Some(replacement_object) = replacement_object_like(object, replacement) else {
        return false;
    };
    *object = replacement_object;
    true
}

fn replacement_object_like(original: &Object, replacement: &str) -> Option<Object> {
    let Object::String(bytes, format) = original else {
        return None;
    };
    if is_utf16be_pdf_string(bytes) {
        return Some(Object::String(
            encode_utf16be_pdf_string(replacement),
            *format,
        ));
    }
    if replacement.is_ascii() {
        return Some(Object::String(replacement.as_bytes().to_vec(), *format));
    }
    None
}

fn empty_string_like(original: &Object) -> Object {
    match original {
        Object::String(_, format) => Object::String(Vec::new(), *format),
        _ => Object::String(Vec::new(), StringFormat::Literal),
    }
}

fn empty_string_object(object: &mut Object) {
    *object = empty_string_like(object);
}

fn empty_all_string_items(items: &mut [Object]) {
    for item in items {
        if matches!(item, Object::String(_, _)) {
            *item = empty_string_like(item);
        }
    }
}

fn is_utf16be_pdf_string(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFE, 0xFF])
}

fn encode_utf16be_pdf_string(value: &str) -> Vec<u8> {
    let mut encoded = vec![0xFE, 0xFF];
    for unit in value.encode_utf16() {
        encoded.push((unit >> 8) as u8);
        encoded.push((unit & 0xFF) as u8);
    }
    encoded
}

fn remove_target_artifact_ranges(
    operations: &[Operation],
    targets: HeaderFooterArtifactTargets,
    properties: &Dictionary,
) -> (Vec<Operation>, ArtifactRemovalStats) {
    let mut filtered = Vec::with_capacity(operations.len());
    let mut removed = ArtifactRemovalStats::default();
    let mut index = 0_usize;

    while index < operations.len() {
        if let Some(region) = target_artifact_region(&operations[index], targets, properties) {
            if let Some(end) = matching_marked_content_end(operations, index) {
                removed.add_region(region);
                index = end + 1;
                continue;
            }
        }
        filtered.push(operations[index].clone());
        index += 1;
    }

    (filtered, removed)
}

fn matching_marked_content_end(operations: &[Operation], start: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (index, operation) in operations.iter().enumerate().skip(start) {
        if is_marked_content_start(operation) {
            depth += 1;
        } else if operation.operator == "EMC" {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn is_marked_content_start(operation: &Operation) -> bool {
    operation.operator == "BMC" || operation.operator == "BDC"
}

fn target_artifact_region(
    operation: &Operation,
    targets: HeaderFooterArtifactTargets,
    properties: &Dictionary,
) -> Option<ArtifactRegion> {
    if operation.operator != "BDC" {
        return None;
    }
    let tag = operation.operands.first().and_then(name_bytes)?;
    if tag != b"Artifact" {
        return None;
    }
    let property = operation.operands.get(1)?;
    let subtype = artifact_subtype(property, properties)?;
    if targets.header && subtype == b"Header" {
        Some(ArtifactRegion::Header)
    } else if targets.footer && subtype == b"Footer" {
        Some(ArtifactRegion::Footer)
    } else {
        None
    }
}

fn artifact_subtype<'a>(property: &'a Object, properties: &'a Dictionary) -> Option<&'a [u8]> {
    match property {
        Object::Dictionary(dict) => dict.get(b"Subtype").ok().and_then(name_bytes),
        Object::Name(name) => properties
            .get(name)
            .ok()
            .and_then(|object| object.as_dict().ok())
            .and_then(|dict| dict.get(b"Subtype").ok())
            .and_then(name_bytes),
        _ => None,
    }
}

fn edit_referenced_form_artifacts(
    doc: &mut Document,
    operations: &[Operation],
    xobjects: &Dictionary,
    plan: &HeaderFooterArtifactEditPlan,
    page_index: usize,
    visited: &mut BTreeSet<ObjectId>,
) -> Result<HeaderFooterArtifactEditResult> {
    let mut result = HeaderFooterArtifactEditResult::default();
    for operation in operations
        .iter()
        .filter(|operation| operation.operator == "Do")
    {
        let Some(name) = operation.operands.first().and_then(name_bytes) else {
            continue;
        };
        let Some(object_id) = xobjects.get(name).ok().and_then(object_reference) else {
            continue;
        };
        if !visited.insert(object_id) {
            continue;
        }

        let Some((stream_content, stream_dict)) = doc
            .get_object(object_id)
            .ok()
            .and_then(|object| object.as_stream().ok())
            .filter(|stream| stream.dict.get(b"Subtype").ok().and_then(name_bytes) == Some(b"Form"))
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
        let resources = resource_dictionary(doc, stream_dict.get(b"Resources").ok());
        let properties = properties_from_resources(doc, resources.as_ref());
        let nested_xobjects = xobjects_from_resources(doc, resources.as_ref());
        let (edited, mut form_result) =
            edit_target_artifact_ranges(&content.operations, plan, &properties, page_index);
        let direct_changed = form_result.changed_count() > 0;
        if !nested_xobjects.is_empty() {
            let nested_result = edit_referenced_form_artifacts(
                doc,
                &content.operations,
                &nested_xobjects,
                plan,
                page_index,
                visited,
            )?;
            merge_edit_result(&mut form_result, nested_result);
        }
        if direct_changed {
            let encoded = Content { operations: edited }
                .encode()
                .context("编码 Form XObject 中的页眉页脚失败")?;
            doc.get_object_mut(object_id)
                .and_then(Object::as_stream_mut)
                .context("写回 Form XObject 页眉页脚失败")?
                .set_plain_content(encoded);
        }
        merge_edit_result(&mut result, form_result);
    }
    Ok(result)
}

pub(crate) fn object_reference(object: &Object) -> Option<ObjectId> {
    object.as_reference().ok()
}

pub(crate) fn resource_dictionary(doc: &Document, object: Option<&Object>) -> Option<Dictionary> {
    match object? {
        Object::Dictionary(dictionary) => Some(dictionary.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

fn properties_from_resources(doc: &Document, resources: Option<&Dictionary>) -> Dictionary {
    let mut properties = Dictionary::new();
    if let Some(resources) = resources {
        merge_properties(doc, resources, &mut properties);
    }
    properties
}

pub(crate) fn xobjects_from_resources(
    doc: &Document,
    resources: Option<&Dictionary>,
) -> Dictionary {
    let Some(resources) = resources else {
        return Dictionary::new();
    };
    resource_dictionary(doc, resources.get(b"XObject").ok()).unwrap_or_default()
}

pub(crate) fn page_xobjects(doc: &Document, page_id: ObjectId) -> Dictionary {
    let mut xobjects = Dictionary::new();
    let Ok((direct_resources, resource_ids)) = doc.get_page_resources(page_id) else {
        return xobjects;
    };
    for resource_id in resource_ids.into_iter().rev() {
        if let Ok(resources) = doc.get_dictionary(resource_id) {
            merge_xobjects(doc, resources, &mut xobjects);
        }
    }
    if let Some(resources) = direct_resources {
        merge_xobjects(doc, resources, &mut xobjects);
    }
    xobjects
}

fn merge_xobjects(doc: &Document, resources: &Dictionary, output: &mut Dictionary) {
    let Some(xobjects) = resource_dictionary(doc, resources.get(b"XObject").ok()) else {
        return;
    };
    for (name, value) in xobjects.iter() {
        output.set(name.clone(), value.clone());
    }
}

fn page_properties(doc: &Document, page_id: ObjectId) -> Dictionary {
    let mut properties = Dictionary::new();
    if let Ok((resource_dict, resource_ids)) = doc.get_page_resources(page_id) {
        if let Some(resources) = resource_dict {
            merge_properties(doc, resources, &mut properties);
        }
        for id in resource_ids {
            if let Ok(resources) = doc.get_dictionary(id) {
                merge_properties(doc, resources, &mut properties);
            }
        }
    }
    properties
}

fn merge_properties(doc: &Document, resources: &Dictionary, output: &mut Dictionary) {
    let Ok(properties_obj) = resources.get(b"Properties") else {
        return;
    };
    let properties_dict = match properties_obj {
        Object::Dictionary(dict) => Some(dict),
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        _ => None,
    };
    if let Some(properties_dict) = properties_dict {
        for (name, value) in properties_dict.iter() {
            output.set(name.clone(), value.clone());
        }
    }
}

fn name_bytes(object: &Object) -> Option<&[u8]> {
    object.as_name().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use lopdf::Stream;

    #[test]
    fn removes_standard_header_artifact_range() {
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Type" => "Pagination",
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("old header")]),
            Operation::new("EMC", vec![]),
            Operation::new("ET", vec![]),
        ];
        let (filtered, removed) = remove_target_artifact_ranges(
            &operations,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &Dictionary::new(),
        );
        assert_eq!(removed.total(), 1);
        assert_eq!(removed.header, 1);
        assert_eq!(
            filtered
                .iter()
                .map(|op| op.operator.as_str())
                .collect::<Vec<_>>(),
            vec!["BT", "ET"]
        );
    }

    #[test]
    fn keeps_footer_when_only_removing_header() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Subtype" => "Footer",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("1/2")]),
            Operation::new("EMC", vec![]),
        ];
        let (filtered, removed) = remove_target_artifact_ranges(
            &operations,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &Dictionary::new(),
        );
        assert_eq!(removed.total(), 0);
        assert_eq!(filtered.len(), operations.len());
    }

    #[test]
    fn resolves_named_artifact_properties() {
        let mut properties = Dictionary::new();
        properties.set(
            "HF1",
            Object::Dictionary(dictionary! {
                "Subtype" => "Header",
            }),
        );
        let operation = Operation::new(
            "BDC",
            vec![
                Object::Name(b"Artifact".to_vec()),
                Object::Name(b"HF1".to_vec()),
            ],
        );
        assert!(target_artifact_region(
            &operation,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &properties
        )
        .is_some());
    }

    #[test]
    fn refuses_unbalanced_marked_content_removal() {
        let operations = vec![Operation::new(
            "BDC",
            vec![
                Object::Name(b"Artifact".to_vec()),
                Object::Dictionary(dictionary! {
                    "Subtype" => "Header",
                }),
            ],
        )];
        let (filtered, removed) = remove_target_artifact_ranges(
            &operations,
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
            &Dictionary::new(),
        );
        assert_eq!(removed.total(), 0);
        assert_eq!(filtered.len(), operations.len());
    }

    #[test]
    fn inspects_cid_artifact_text_with_tounicode_map() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Type" => "Pagination",
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new("Tf", vec![Object::Name(b"FCID".to_vec()), 12.into()]),
            Operation::new(
                "Tj",
                vec![Object::String(
                    vec![0, 1, 0, 2],
                    lopdf::StringFormat::Hexadecimal,
                )],
            ),
            Operation::new("EMC", vec![]),
        ];
        let cmap = ToUnicodeCMap::parse(
            br#"
            1 begincodespacerange <0000> <00ff> endcodespacerange
            2 beginbfchar <0001> <4E2D> <0002> <6587> endbfchar
            "#,
        )
        .unwrap();
        let fonts = BTreeMap::from([(String::from("FCID"), String::from("7 0 R"))]);
        let cmaps = BTreeMap::from([(String::from("7 0 R"), cmap)]);
        let inspection = inspect_artifact_operations_detailed_qpdf(
            &operations,
            &Dictionary::new(),
            &fonts,
            &cmaps,
            1,
            "content",
            "7 0 R",
        );
        assert_eq!(inspection.occurrences.len(), 1);
        assert_eq!(inspection.occurrences[0].text.as_deref(), Some("中文"));
    }

    #[test]
    fn deletes_header_artifact_from_pdf_file() {
        let input = temp_named_path("docsy_artifact_test_input", "pdf");
        let output = temp_named_path("docsy_artifact_test_output", "pdf");
        create_artifact_test_pdf(&input);

        let result = delete_header_footer_artifacts_file(
            &input.to_string_lossy(),
            &output.to_string_lossy(),
            HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
        )
        .unwrap();
        assert_eq!(result.removed, 1);
        assert_eq!(result.pages_touched, 1);

        let doc = Document::load(&output).unwrap();
        let page_id = doc.get_pages().into_values().next().unwrap();
        let content = doc.get_and_decode_page_content(page_id).unwrap();
        let text = content
            .operations
            .iter()
            .flat_map(|op| op.operands.iter())
            .filter_map(|object| object.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .collect::<Vec<_>>();
        assert!(!text.iter().any(|value| value.contains("old header")));
        assert!(text.iter().any(|value| value.contains("body text")));

        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn qpdf_inspection_and_targeted_edit_roundtrip() {
        let input = temp_named_path("docsy_artifact_qpdf_input", "pdf");
        let output = temp_named_path("docsy_artifact_qpdf_output", "pdf");
        create_artifact_test_pdf(&input);

        let inspection = inspect_meaningful_header_footer_artifacts(&input, 0).unwrap();
        let occurrence = inspection
            .occurrences
            .iter()
            .find(|occurrence| occurrence.region == "header")
            .unwrap();
        assert_eq!(occurrence.text.as_deref(), Some("old header"));
        assert!(occurrence.object_ref.is_some());

        let plan = HeaderFooterArtifactEditPlan {
            remove_header: true,
            header_targets: vec![HeaderFooterArtifactEditTarget {
                artifact_id: Some(occurrence.id.clone()),
                normalized_text: "old header".to_string(),
                page_start: 1,
                page_end: 1,
                replacement_text: Some("new header".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result =
            edit_header_footer_artifacts_file(&input.to_string_lossy(), &output, &plan).unwrap();
        assert_eq!(result.edited_header, 1);

        let rescanned = inspect_meaningful_header_footer_artifacts(&output, 0).unwrap();
        assert!(rescanned
            .occurrences
            .iter()
            .any(|occurrence| occurrence.text.as_deref() == Some("new header")));
        let document = Document::load(&output).unwrap();
        let page_id = document.get_pages().into_values().next().unwrap();
        let content = document.get_and_decode_page_content(page_id).unwrap();
        assert!(content.operations.iter().any(|operation| {
            operation
                .operands
                .iter()
                .filter_map(|object| object.as_str().ok())
                .any(|bytes| bytes == b"new header")
        }));

        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn edits_ascii_header_artifact_in_place() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("old header")]),
            Operation::new("EMC", vec![]),
            Operation::new("Tj", vec![Object::string_literal("body text")]),
        ];
        let plan = HeaderFooterArtifactEditPlan {
            remove_header: true,
            header_texts: vec!["new header".to_string()],
            ..Default::default()
        };
        let (edited, result) =
            edit_target_artifact_ranges(&operations, &plan, &Dictionary::new(), 0);

        assert_eq!(result.edited_header, 1);
        assert_eq!(result.removed_header, 0);
        assert!(result.edited_header_pages.contains(&0));
        let text = edited
            .iter()
            .flat_map(|op| op.operands.iter())
            .filter_map(|object| object.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .collect::<Vec<_>>();
        assert!(text.iter().any(|value| value.contains("new header")));
        assert!(!text.iter().any(|value| value.contains("old header")));
        assert!(text.iter().any(|value| value.contains("body text")));
    }

    #[test]
    fn edits_artifact_range_and_clears_remaining_text_shows() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Subtype" => "Footer",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("1")]),
            Operation::new("Tj", vec![Object::string_literal(" / ")]),
            Operation::new("Tj", vec![Object::string_literal("old footer")]),
            Operation::new("EMC", vec![]),
            Operation::new("Tj", vec![Object::string_literal("body text")]),
        ];
        let plan = HeaderFooterArtifactEditPlan {
            remove_footer: true,
            footer_texts: vec!["1/20 new footer".to_string()],
            ..Default::default()
        };
        let (edited, result) =
            edit_target_artifact_ranges(&operations, &plan, &Dictionary::new(), 0);

        assert_eq!(result.edited_footer, 1);
        let text = edited
            .iter()
            .flat_map(|op| op.operands.iter())
            .filter_map(|object| object.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .collect::<Vec<_>>();
        assert!(text.iter().any(|value| value == "1/20 new footer"));
        assert!(!text.iter().any(|value| value == "old footer"));
        assert!(text.iter().any(|value| value == "body text"));
    }

    #[test]
    fn removes_standard_artifact_when_replacement_encoding_is_unsafe() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("old header")]),
            Operation::new("EMC", vec![]),
            Operation::new("Tj", vec![Object::string_literal("body text")]),
        ];
        let plan = HeaderFooterArtifactEditPlan {
            remove_header: true,
            header_texts: vec!["证据一".to_string()],
            ..Default::default()
        };
        let (edited, result) =
            edit_target_artifact_ranges(&operations, &plan, &Dictionary::new(), 0);

        assert_eq!(result.edited_header, 0);
        assert_eq!(result.removed_header, 1);
        assert_eq!(
            edited
                .iter()
                .map(|op| op.operator.as_str())
                .collect::<Vec<_>>(),
            vec!["Tj"]
        );
        let text = edited
            .iter()
            .flat_map(|op| op.operands.iter())
            .filter_map(|object| object.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .collect::<Vec<_>>();
        assert!(text.iter().any(|value| value.contains("body text")));
    }

    #[test]
    fn edits_utf16be_header_artifact_in_place() {
        let utf16_old = encode_utf16be_pdf_string("旧页眉");
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new(
                "Tj",
                vec![Object::String(utf16_old, StringFormat::Hexadecimal)],
            ),
            Operation::new("EMC", vec![]),
        ];
        let plan = HeaderFooterArtifactEditPlan {
            remove_header: true,
            header_texts: vec!["证据一".to_string()],
            ..Default::default()
        };
        let (edited, result) =
            edit_target_artifact_ranges(&operations, &plan, &Dictionary::new(), 0);

        assert_eq!(result.edited_header, 1);
        let Object::String(bytes, format) = &edited[1].operands[0] else {
            panic!("expected string");
        };
        assert_eq!(*format, StringFormat::Hexadecimal);
        assert_eq!(bytes, &encode_utf16be_pdf_string("证据一"));
    }

    fn create_artifact_test_pdf(path: &Path) {
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
                Operation::new(
                    "BDC",
                    vec![
                        Object::Name(b"Artifact".to_vec()),
                        Object::Dictionary(dictionary! {
                            "Type" => "Pagination",
                            "Subtype" => "Header",
                        }),
                    ],
                ),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
                Operation::new("Td", vec![450.into(), 800.into()]),
                Operation::new("Tj", vec![Object::string_literal("old header")]),
                Operation::new("ET", vec![]),
                Operation::new("EMC", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new("Td", vec![80.into(), 500.into()]),
                Operation::new("Tj", vec![Object::string_literal("body text")]),
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

    #[test]
    fn artifact_inspection_ignores_blank_header_and_counts_visible_footer() {
        let operations = vec![
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Type" => "Pagination",
                        "Subtype" => "Header",
                    }),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal(" ")]),
            Operation::new("EMC", vec![]),
            Operation::new(
                "BDC",
                vec![
                    Object::Name(b"Artifact".to_vec()),
                    Object::Dictionary(dictionary! {
                        "Type" => "Pagination",
                        "Subtype" => "Footer",
                    }),
                ],
            ),
            Operation::new(
                "TJ",
                vec![Object::Array(vec![Object::string_literal("1 / 14")])],
            ),
            Operation::new("EMC", vec![]),
        ];

        let result = inspect_artifact_operations(&operations, &Dictionary::new());
        assert_eq!(result.header_count, 0);
        assert_eq!(result.footer_count, 1);
    }
}
