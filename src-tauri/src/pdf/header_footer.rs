use allsorts::binary::read::ReadScope;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::subset::{subset as subset_font_bytes, CmapTarget, SubsetProfile};
use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use printpdf::{
    generate_cmap_string, generate_gid_to_cid_map, get_normalized_widths_cff,
    get_normalized_widths_ttf, FontId, FontType, ParsedFont,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

use super::annotations;
use super::artifacts;
use super::content_text;
use super::normalize::normalize_pdf_to_a4;
use super::page_info::{get_page_infos, PageSize};
use super::preview::{render_preview, PreviewResult};
use super::qpdf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeaderFooterJob {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(alias = "output")]
    output_path: String,
    #[serde(default = "default_page_start")]
    page_start: u32,
    #[serde(default)]
    total_pages: Option<u32>,
    #[serde(default)]
    normalize_a4: bool,
    #[serde(default = "default_a4_orientation")]
    a4_orientation: String,
    #[serde(default = "default_raster_dpi")]
    raster_dpi: u32,
    #[serde(default)]
    cleanup: CleanupConfig,
    #[serde(default)]
    header: Option<OverlayTextConfig>,
    #[serde(default)]
    footer: Option<OverlayTextConfig>,
    #[serde(default)]
    extra_overlays: Vec<OverlayTextConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CleanupConfig {
    #[serde(default)]
    header_enabled: bool,
    #[serde(default)]
    footer_enabled: bool,
    #[serde(default)]
    force_delete_header: bool,
    #[serde(default)]
    force_delete_footer: bool,
    #[serde(default = "default_cleanup_zone_mm")]
    header_height_mm: f32,
    #[serde(default = "default_cleanup_zone_mm")]
    footer_height_mm: f32,
    #[serde(default)]
    plain_header_targets: Vec<PlainTextCleanupTargetConfig>,
    #[serde(default)]
    plain_footer_targets: Vec<PlainTextCleanupTargetConfig>,
    #[serde(default)]
    header_replacement: Option<OverlayTextConfig>,
    #[serde(default)]
    footer_replacement: Option<OverlayTextConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlainTextCleanupTargetConfig {
    text: String,
    #[serde(default)]
    normalized_text: String,
    #[serde(default = "default_page_start")]
    page_start: u32,
    #[serde(default)]
    page_end: u32,
    #[serde(default)]
    bbox: Option<PlainTextCleanupBBoxConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlainTextCleanupBBoxConfig {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    page: u32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayTextConfig {
    text: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    font_family: String,
    #[serde(default = "default_font_size")]
    font_size: f32,
    #[serde(default = "default_margin_mm")]
    margin_mm: f32,
    #[serde(default = "default_align")]
    align: String,
    #[serde(default)]
    offset_x_mm: f32,
    #[serde(default = "default_text_color")]
    color: String,
    #[serde(default)]
    page_start: Option<u32>,
    #[serde(default)]
    page_end: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewAnnotationRule {
    #[serde(default, alias = "remove")]
    remove_annotations: bool,
    #[serde(default)]
    kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeaderFooterResult {
    input_path: String,
    output_path: String,
    pages: u32,
    normalized: bool,
    cleaned: bool,
    semantic_removed: usize,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeaderFooterFailure {
    path: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchHeaderFooterResult {
    results: Vec<HeaderFooterResult>,
    failed: Vec<HeaderFooterFailure>,
}

fn default_page_start() -> u32 {
    1
}

fn default_raster_dpi() -> u32 {
    200
}

fn default_font_size() -> f32 {
    10.0
}

fn default_margin_mm() -> f32 {
    10.0
}

fn default_cleanup_zone_mm() -> f32 {
    18.0
}

fn default_align() -> String {
    "center".to_string()
}

fn default_text_color() -> String {
    "#000000".to_string()
}

fn default_a4_orientation() -> String {
    "auto".to_string()
}

pub fn overlay_text(args: &serde_json::Value) -> Result<serde_json::Value> {
    let args: HeaderFooterJob =
        serde_json::from_value(args.clone()).context("解析页眉页脚处理参数失败")?;
    let result = process_job(&args)?;
    Ok(serde_json::to_value(result)?)
}

pub fn batch_overlay(args: &serde_json::Value) -> Result<serde_json::Value> {
    let items = args
        .get("items")
        .or_else(|| args.get("inputs"))
        .and_then(|v| v.as_array())
        .context("缺少 items 数组")?;

    let mut results = Vec::new();
    let mut failed = Vec::new();

    for item in items {
        match serde_json::from_value::<HeaderFooterJob>(item.clone()) {
            Ok(job) => match process_job(&job) {
                Ok(result) => results.push(result),
                Err(err) => failed.push(HeaderFooterFailure {
                    path: job.input_path,
                    message: err.to_string(),
                }),
            },
            Err(err) => failed.push(HeaderFooterFailure {
                path: item
                    .get("inputPath")
                    .or_else(|| item.get("input"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                message: format!("解析页眉页脚处理参数失败: {err}"),
            }),
        }
    }

    Ok(serde_json::to_value(BatchHeaderFooterResult {
        results,
        failed,
    })?)
}

pub fn preview_overlay(args: &serde_json::Value) -> Result<PreviewResult> {
    let job_value = args.get("job").unwrap_or(args);
    let mut job: HeaderFooterJob =
        serde_json::from_value(job_value.clone()).context("解析页眉页脚预览参数失败")?;
    let annotation_rule = preview_annotation_rule(args);
    let page = args
        .get("page")
        .and_then(|value| value.as_u64())
        .unwrap_or(1) as u32;
    let dpi = args
        .get("dpi")
        .and_then(|value| value.as_u64())
        .unwrap_or(120) as u32;

    let preview_output = temp_named_path("docsy_hf_preview", "pdf");
    job.output_path = preview_output.to_string_lossy().to_string();
    let annotation_temp = if annotation_rule.remove_annotations {
        let temp = annotations::delete_annotations_to_temp(&job.input_path, &annotation_rule.kinds)
            .context("预览前删除批注失败")?;
        job.input_path = temp.to_string_lossy().to_string();
        Some(temp)
    } else {
        None
    };

    let result = process_job(&job).and_then(|_| {
        render_preview(&serde_json::json!({
            "inputPath": job.output_path,
            "page": page,
            "dpi": dpi,
        }))
    });
    let _ = fs::remove_file(&preview_output);
    cleanup_temp(annotation_temp);
    result
}

fn preview_annotation_rule(args: &serde_json::Value) -> PreviewAnnotationRule {
    args.get("annotationRule")
        .or_else(|| {
            args.get("session")
                .and_then(|session| session.get("annotationRule"))
        })
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

fn process_job(args: &HeaderFooterJob) -> Result<HeaderFooterResult> {
    let input = Path::new(&args.input_path);
    if !input.exists() {
        anyhow::bail!("原始 PDF 不存在: {}", input.display());
    }
    let output_requested = Path::new(&args.output_path);
    let output_path = unique_output_path(output_requested);
    let output = output_path.as_path();
    if same_path(input, output) {
        anyhow::bail!("输出路径不能和原始 PDF 相同，请另存为副本");
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).context("创建输出目录失败")?;
    }

    let semantic_deleted_path = edit_or_delete_standard_artifacts_if_requested(args)?;
    let _semantic_temp_guard = semantic_deleted_path
        .as_ref()
        .map(|result| TempPathGuard::new(result.path.clone()));
    let artifact_removed = semantic_deleted_path
        .as_ref()
        .map(|artifact_result| artifact_result.changed_count())
        .unwrap_or(0);
    let mut warnings = standard_artifact_processing_warnings(args, semantic_deleted_path.as_ref());
    let semantic_input = semantic_deleted_path
        .as_ref()
        .map(|artifact_result| artifact_result.path.as_path())
        .unwrap_or(input);
    let plain_deleted_path =
        delete_confirmed_plain_text_header_footer_if_requested(args, semantic_input)?;
    let _plain_text_temp_guard = plain_deleted_path
        .as_ref()
        .map(|(path, _)| TempPathGuard::new(path.clone()));
    let plain_removed = plain_deleted_path
        .as_ref()
        .map(|(_, result)| result.removed())
        .unwrap_or(0);
    warnings.extend(plain_text_processing_warnings(
        args,
        plain_deleted_path.as_ref().map(|(_, result)| result),
    ));
    let cleanup_input = plain_deleted_path
        .as_ref()
        .map(|(path, _)| path.as_path())
        .unwrap_or(semantic_input);
    let semantic_removed = artifact_removed + plain_removed;
    let semantic_rebuild_overlays = artifact_rebuild_overlays(args, semantic_deleted_path.as_ref());

    let normalized_path = if args.normalize_a4 {
        Some(normalize_pdf_to_a4(
            cleanup_input,
            args.raster_dpi,
            &args.a4_orientation,
        )?)
    } else {
        None
    };
    let _normalized_temp_guard = normalized_path
        .as_ref()
        .map(|path| TempPathGuard::new(path.clone()));
    let work_input = normalized_path.as_deref().unwrap_or(cleanup_input);
    let work_input_str = work_input.to_string_lossy().to_string();
    let page_infos = get_page_infos(&work_input_str)?;
    let pages = page_infos.len() as u32;
    let page_start = args.page_start.max(1);
    let total_pages = args.total_pages.unwrap_or(pages);
    let end_page = page_start + pages.saturating_sub(1);
    if total_pages < end_page {
        anyhow::bail!("全局总页数 {total_pages} 小于当前 PDF 的结束页码 {end_page}");
    }
    let skip_header_pages = semantic_deleted_path
        .as_ref()
        .map(|result| result.removed_header_pages.clone())
        .unwrap_or_default();
    let skip_footer_pages = semantic_deleted_path
        .as_ref()
        .map(|result| result.removed_footer_pages.clone())
        .unwrap_or_default();

    let cleaned = args.cleanup.header_enabled || args.cleanup.footer_enabled;
    if args.header.is_none()
        && args.footer.is_none()
        && args.extra_overlays.is_empty()
        && semantic_rebuild_overlays.is_empty()
    {
        fs::copy(work_input, output).context("复制 PDF 失败")?;
        cleanup_temp(normalized_path);
        cleanup_plain_text_temp(plain_deleted_path);
        cleanup_semantic_temp(semantic_deleted_path);
        return Ok(HeaderFooterResult {
            input_path: args.input_path.clone(),
            output_path: output.to_string_lossy().to_string(),
            pages,
            normalized: args.normalize_a4,
            cleaned,
            semantic_removed,
            warnings,
        });
    }

    let extra_overlays = combined_extra_overlays(args, &semantic_rebuild_overlays);
    let (overlay_pdf, mut overlay_warnings) = build_overlay_pdf(
        args.header.as_ref(),
        args.footer.as_ref(),
        &extra_overlays,
        &page_infos,
        page_start,
        total_pages,
        &skip_header_pages,
        &skip_footer_pages,
    )?;
    warnings.append(&mut overlay_warnings);
    let overlay_path = TempPathGuard::new(temp_named_path("docsy_overlay", "pdf"));
    fs::write(overlay_path.path(), &overlay_pdf).context("写入临时页眉页脚层失败")?;

    let qpdf_tool = crate::external::QpdfTool;
    let bin = qpdf_tool.binary_path()?;
    let overlay_output = TempPathGuard::new(temp_named_path("docsy_overlay_result", "pdf"));
    let command_output = std::process::Command::new(&bin)
        .arg(work_input)
        .arg("--overlay")
        .arg(overlay_path.path())
        .arg("--")
        .arg(overlay_output.path())
        .output()
        .context("执行 qpdf overlay 失败")?;

    cleanup_temp(normalized_path);
    cleanup_plain_text_temp(plain_deleted_path);
    cleanup_semantic_temp(semantic_deleted_path);

    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        anyhow::bail!("qpdf overlay 失败: {}", stderr.trim());
    }
    write_optimized_or_copy(overlay_output.path(), output).context("写入页眉页脚处理结果失败")?;

    Ok(HeaderFooterResult {
        input_path: args.input_path.clone(),
        output_path: output.to_string_lossy().to_string(),
        pages,
        normalized: args.normalize_a4,
        cleaned,
        semantic_removed,
        warnings: std::mem::take(&mut warnings),
    })
}

fn delete_confirmed_plain_text_header_footer_if_requested(
    args: &HeaderFooterJob,
    input: &Path,
) -> Result<Option<(PathBuf, content_text::PlainTextCleanupResult)>> {
    if args.cleanup.plain_header_targets.is_empty() && args.cleanup.plain_footer_targets.is_empty()
    {
        return Ok(None);
    }
    let plan = content_text::PlainTextCleanupPlan {
        header_targets: args
            .cleanup
            .plain_header_targets
            .iter()
            .map(plain_text_target_from_config)
            .collect(),
        footer_targets: args
            .cleanup
            .plain_footer_targets
            .iter()
            .map(plain_text_target_from_config)
            .collect(),
        header_zone_mm: args.cleanup.header_height_mm,
        footer_zone_mm: args.cleanup.footer_height_mm,
    };
    content_text::delete_plain_header_footer_to_temp(&input.to_string_lossy(), &plan)
}

fn plain_text_target_from_config(
    config: &PlainTextCleanupTargetConfig,
) -> content_text::PlainTextTarget {
    let page_start = config.page_start.max(1);
    content_text::PlainTextTarget {
        text: config.text.clone(),
        normalized_text: if config.normalized_text.is_empty() {
            config.text.clone()
        } else {
            config.normalized_text.clone()
        },
        page_start,
        page_end: config.page_end.max(page_start),
        bbox: config
            .bbox
            .as_ref()
            .map(|bbox| content_text::PlainTextTargetBBox {
                x0: bbox.x0,
                y0: bbox.y0,
                x1: bbox.x1,
                y1: bbox.y1,
                page: bbox.page,
                width: bbox.width,
                height: bbox.height,
            }),
    }
}

fn write_optimized_or_copy(input: &Path, output: &Path) -> Result<()> {
    match qpdf::optimize_to(input, output) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(input, output).context("复制 PDF 处理结果失败")?;
            Ok(())
        }
    }
}

fn unique_output_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("output");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for index in 1..10_000 {
        let name = if extension.is_empty() {
            format!("{stem}-{index}")
        } else {
            format!("{stem}-{index}.{extension}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

struct StandardArtifactProcessingResult {
    path: PathBuf,
    removed_header: usize,
    removed_footer: usize,
    edited_header: usize,
    edited_footer: usize,
    removed_header_pages: BTreeSet<usize>,
    removed_footer_pages: BTreeSet<usize>,
}

impl StandardArtifactProcessingResult {
    fn changed_count(&self) -> usize {
        self.removed_header + self.removed_footer + self.edited_header + self.edited_footer
    }
}

fn edit_or_delete_standard_artifacts_if_requested(
    args: &HeaderFooterJob,
) -> Result<Option<StandardArtifactProcessingResult>> {
    if !args.cleanup.header_enabled && !args.cleanup.footer_enabled {
        return Ok(None);
    }
    let page_infos = get_page_infos(&args.input_path)?;
    let page_count = page_infos.len();
    let header_texts = args
        .cleanup
        .header_replacement
        .as_ref()
        .filter(|_| !args.cleanup.force_delete_header)
        .map(|config| {
            artifact_replacement_texts(config, page_count, args.page_start, args.total_pages)
        })
        .unwrap_or_default();
    let footer_texts = args
        .cleanup
        .footer_replacement
        .as_ref()
        .filter(|_| !args.cleanup.force_delete_footer)
        .map(|config| {
            artifact_replacement_texts(config, page_count, args.page_start, args.total_pages)
        })
        .unwrap_or_default();
    let edit_result = artifacts::edit_header_footer_artifacts_to_temp(
        &args.input_path,
        &artifacts::HeaderFooterArtifactEditPlan {
            remove_header: args.cleanup.header_enabled,
            remove_footer: args.cleanup.footer_enabled,
            header_texts,
            footer_texts,
        },
    )?;
    Ok(
        edit_result.map(|(path, result)| StandardArtifactProcessingResult {
            path,
            removed_header: result.removed_header,
            removed_footer: result.removed_footer,
            edited_header: result.edited_header,
            edited_footer: result.edited_footer,
            removed_header_pages: result.removed_header_pages,
            removed_footer_pages: result.removed_footer_pages,
        }),
    )
}

fn combined_extra_overlays(
    args: &HeaderFooterJob,
    semantic_rebuild_overlays: &[OverlayTextConfig],
) -> Vec<OverlayTextConfig> {
    let mut overlays =
        Vec::with_capacity(args.extra_overlays.len() + semantic_rebuild_overlays.len());
    overlays.extend(args.extra_overlays.iter().cloned());
    overlays.extend(semantic_rebuild_overlays.iter().cloned());
    overlays
}

fn artifact_rebuild_overlays(
    args: &HeaderFooterJob,
    result: Option<&StandardArtifactProcessingResult>,
) -> Vec<OverlayTextConfig> {
    let Some(result) = result else {
        return Vec::new();
    };
    let mut overlays = Vec::new();
    if !args.cleanup.force_delete_header {
        if let Some(config) = args.cleanup.header_replacement.as_ref() {
            overlays.extend(artifact_rebuild_overlays_for_region(
                config,
                "header",
                &result.removed_header_pages,
            ));
        }
    }
    if !args.cleanup.force_delete_footer {
        if let Some(config) = args.cleanup.footer_replacement.as_ref() {
            overlays.extend(artifact_rebuild_overlays_for_region(
                config,
                "footer",
                &result.removed_footer_pages,
            ));
        }
    }
    overlays
}

fn artifact_rebuild_overlays_for_region(
    config: &OverlayTextConfig,
    region: &str,
    zero_based_pages: &BTreeSet<usize>,
) -> Vec<OverlayTextConfig> {
    contiguous_page_ranges(zero_based_pages)
        .into_iter()
        .map(|(start, end)| {
            let mut overlay = config.clone();
            overlay.region = region.to_string();
            overlay.page_start = Some(start);
            overlay.page_end = Some(end);
            overlay
        })
        .collect()
}

fn contiguous_page_ranges(zero_based_pages: &BTreeSet<usize>) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    let mut start: Option<u32> = None;
    let mut previous: Option<u32> = None;
    for page in zero_based_pages {
        let page = (*page as u32).saturating_add(1);
        match (start, previous) {
            (Some(range_start), Some(prev)) if page == prev + 1 => {
                start = Some(range_start);
                previous = Some(page);
            }
            (Some(range_start), Some(prev)) => {
                ranges.push((range_start, prev));
                start = Some(page);
                previous = Some(page);
            }
            _ => {
                start = Some(page);
                previous = Some(page);
            }
        }
    }
    if let (Some(range_start), Some(prev)) = (start, previous) {
        ranges.push((range_start, prev));
    }
    ranges
}

fn standard_artifact_processing_warnings(
    args: &HeaderFooterJob,
    result: Option<&StandardArtifactProcessingResult>,
) -> Vec<String> {
    let Some(result) = result else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    if args.cleanup.header_replacement.is_some()
        && !args.cleanup.force_delete_header
        && result.removed_header > 0
    {
        warnings.push("部分标准页眉无法原位编辑，已删除后按当前页眉设置重建".to_string());
    }
    if args.cleanup.footer_replacement.is_some()
        && !args.cleanup.force_delete_footer
        && result.removed_footer > 0
    {
        warnings.push("部分标准页脚无法原位编辑，已删除后按当前页脚设置重建".to_string());
    }
    warnings
}

fn plain_text_processing_warnings(
    args: &HeaderFooterJob,
    result: Option<&content_text::PlainTextCleanupResult>,
) -> Vec<String> {
    let Some(result) = result else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    if result.removed_header > 0
        && (args.header.is_some() || args.cleanup.header_replacement.is_some())
    {
        warnings.push("普通文本型页眉已转换为 Docsy 页眉，原字体格式无法无损保留".to_string());
    }
    if result.removed_footer > 0
        && (args.footer.is_some() || args.cleanup.footer_replacement.is_some())
    {
        warnings.push("普通文本型页脚已转换为 Docsy 页脚，原字体格式无法无损保留".to_string());
    }
    warnings
}

fn artifact_replacement_texts(
    config: &OverlayTextConfig,
    page_count: usize,
    page_start: u32,
    total_pages: Option<u32>,
) -> Vec<String> {
    let page_start = page_start.max(1);
    let total_pages = total_pages.unwrap_or(page_count as u32);
    (0..page_count)
        .map(|index| expand_placeholders(&config.text, page_start + index as u32, total_pages))
        .collect()
}

#[allow(clippy::too_many_arguments)] // overlay inputs are independent domain parameters
fn build_overlay_pdf(
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
    extra_overlays: &[OverlayTextConfig],
    pages: &[PageSize],
    page_start: u32,
    total_pages: u32,
    skip_header_pages: &BTreeSet<usize>,
    skip_footer_pages: &BTreeSet<usize>,
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
    let cjk_fallback_id = add_standard_cjk_font(&mut doc);
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
    font_resources.set("FCJKFallback", cjk_fallback_id);
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
        let mut operations = Vec::new();

        if let Some(config) = header.filter(|config| {
            !skip_header_pages.contains(&index) && overlay_applies_to_page(config, local_page)
        }) {
            append_overlay_text_ops(
                &mut operations,
                config,
                OverlayRegion::Header,
                size,
                current_page,
                total_pages,
                &embedded_fonts,
            );
        }

        if let Some(config) = footer.filter(|config| {
            !skip_footer_pages.contains(&index) && overlay_applies_to_page(config, local_page)
        }) {
            append_overlay_text_ops(
                &mut operations,
                config,
                OverlayRegion::Footer,
                size,
                current_page,
                total_pages,
                &embedded_fonts,
            );
        }

        for config in extra_overlays {
            if !overlay_applies_to_page(config, local_page) {
                continue;
            }
            let region = overlay_region(&config.region);
            append_overlay_text_ops(
                &mut operations,
                config,
                region,
                size,
                current_page,
                total_pages,
                &embedded_fonts,
            );
        }

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

fn overlay_applies_to_page(config: &OverlayTextConfig, local_page: u32) -> bool {
    let start = config.page_start.unwrap_or(1).max(1);
    let end = config.page_end.unwrap_or(u32::MAX).max(start);
    local_page >= start && local_page <= end
}

#[derive(Debug, Clone, Copy)]
enum OverlayRegion {
    Header,
    Footer,
}

#[derive(Debug, Clone)]
struct EmbeddedOverlayFont {
    resource_name: String,
    object_id: ObjectId,
    char_to_gid: BTreeMap<char, u16>,
}

#[derive(Debug, Clone)]
struct EmbeddedFontChoice {
    font: EmbeddedOverlayFont,
    family: String,
}

#[derive(Debug, Clone)]
struct FontCandidate {
    family: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
enum OverlayFontRef<'a> {
    Builtin(&'static str),
    Embedded(&'a EmbeddedOverlayFont),
    StandardCjk,
}

fn overlay_region(value: &str) -> OverlayRegion {
    if value.trim().eq_ignore_ascii_case("header") {
        OverlayRegion::Header
    } else {
        OverlayRegion::Footer
    }
}

fn add_standard_cjk_font(doc: &mut Document) -> ObjectId {
    let descendant_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType0",
        "BaseFont" => "STSong-Light",
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("GB1"),
            "Supplement" => 2,
        },
        "DW" => 1000,
    });
    doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "STSong-Light",
        "Encoding" => "UniGB-UCS2-H",
        "DescendantFonts" => vec![descendant_id.into()],
    })
}

#[allow(clippy::too_many_arguments)] // the font plan depends on each overlay source and page set
fn prepare_embedded_overlay_fonts(
    doc: &mut Document,
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
    extra_overlays: &[OverlayTextConfig],
    page_count: usize,
    page_start: u32,
    total_pages: u32,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, EmbeddedOverlayFont> {
    let mut texts_by_family: BTreeMap<String, String> = BTreeMap::new();
    for page_index in 0..page_count {
        let current_page = page_start + page_index as u32;
        for config in header
            .into_iter()
            .chain(footer)
            .chain(extra_overlays.iter())
        {
            let text = expand_placeholders(&config.text, current_page, total_pages);
            if requires_embedded_font(&text) {
                texts_by_family
                    .entry(font_family_key(&config.font_family))
                    .or_default()
                    .push_str(&text);
            }
        }
    }

    let mut fonts = BTreeMap::new();
    for (index, (family, text)) in texts_by_family.into_iter().enumerate() {
        let resource_name = format!("FEmbed{}", index + 1);
        match create_embedded_overlay_font(doc, &resource_name, &family, &text) {
            Ok(choice) => {
                if choice.family != family {
                    warnings.push(format!(
                        "字体「{}」无法嵌入，已改用相近字体「{}」",
                        display_font_family(&family),
                        display_font_family(&choice.family)
                    ));
                }
                fonts.insert(family, choice.font);
            }
            Err(err) => warnings.push(format!(
                "字体「{}」及相近字体均无法按子集嵌入，已降级为 PDF 标准中文字体：{}",
                display_font_family(&family),
                err
            )),
        }
    }
    fonts
}

fn create_embedded_overlay_font(
    doc: &mut Document,
    resource_name: &str,
    family: &str,
    text: &str,
) -> Result<EmbeddedFontChoice> {
    let mut last_error = None;
    for candidate in font_candidate_sequence(family) {
        if !candidate.path.exists() {
            continue;
        }
        match try_create_embedded_overlay_font(doc, resource_name, &candidate.path, text) {
            Ok(font) => {
                return Ok(EmbeddedFontChoice {
                    font,
                    family: candidate.family,
                })
            }
            Err(err) => last_error = Some(err),
        }
    }
    match last_error {
        Some(err) => Err(err),
        None => anyhow::bail!("未找到可用系统字体"),
    }
}

fn try_create_embedded_overlay_font(
    doc: &mut Document,
    resource_name: &str,
    path: &Path,
    text: &str,
) -> Result<EmbeddedOverlayFont> {
    let bytes = fs::read(path).with_context(|| format!("读取字体失败: {}", path.display()))?;
    let scope = ReadScope::new(&bytes);
    let font_data = scope
        .read::<FontData<'_>>()
        .with_context(|| format!("解析字体失败: {}", path.display()))?;
    let provider_for_lookup = font_data
        .table_provider(0)
        .with_context(|| format!("读取字体表失败: {}", path.display()))?;
    let mut font = Font::new(provider_for_lookup)
        .with_context(|| format!("初始化字体失败: {}", path.display()))?;

    let mut glyph_ids = vec![0_u16];
    let mut char_to_original_gid = BTreeMap::new();
    for ch in text.chars().filter(|ch| !ch.is_control()) {
        let (gid, _) = font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None);
        if gid == 0 && !ch.is_whitespace() {
            anyhow::bail!("字体缺少字符「{}」", ch);
        }
        if gid != 0 {
            char_to_original_gid.entry(ch).or_insert(gid);
            glyph_ids.push(gid);
        }
    }
    glyph_ids.sort_unstable();
    glyph_ids.dedup();
    if glyph_ids.first().copied() != Some(0) {
        glyph_ids.insert(0, 0);
    }

    let provider_for_subset = font_data
        .table_provider(0)
        .with_context(|| format!("读取字体子集表失败: {}", path.display()))?;
    let subset_bytes = subset_font_bytes(
        &provider_for_subset,
        &glyph_ids,
        &SubsetProfile::Pdf,
        CmapTarget::Unicode,
    )
    .with_context(|| format!("生成字体子集失败: {}", path.display()))?;

    let mut parsed_warnings = Vec::new();
    let parsed_subset = ParsedFont::from_bytes(&subset_bytes, 0, &mut parsed_warnings)
        .ok_or_else(|| anyhow::anyhow!("解析字体子集失败: {}", path.display()))?;
    let char_to_subset_gid = char_to_original_gid
        .into_iter()
        .filter_map(|(ch, original_gid)| {
            glyph_ids
                .iter()
                .position(|gid| *gid == original_gid)
                .map(|position| (ch, position as u16))
        })
        .collect::<BTreeMap<_, _>>();
    let new_glyph_ids = char_to_subset_gid
        .iter()
        .map(|(ch, gid)| (*gid, *ch))
        .collect::<Vec<_>>();
    let font_id = FontId(resource_name.to_string());
    let to_unicode = generate_cmap_string(&parsed_subset, &font_id, &new_glyph_ids);
    let widths = match parsed_subset.font_type {
        FontType::TrueType => get_normalized_widths_ttf(&parsed_subset, &new_glyph_ids),
        _ => {
            let gid_to_cid_map = generate_gid_to_cid_map(&parsed_subset, &new_glyph_ids);
            get_normalized_widths_cff(&parsed_subset, &gid_to_cid_map)
        }
    };
    let object_id = add_subset_font_to_doc(
        doc,
        resource_name,
        &parsed_subset,
        subset_bytes,
        to_unicode,
        widths,
    );
    Ok(EmbeddedOverlayFont {
        resource_name: resource_name.to_string(),
        object_id,
        char_to_gid: char_to_subset_gid,
    })
}

fn add_subset_font_to_doc(
    doc: &mut Document,
    resource_name: &str,
    font: &ParsedFont,
    font_bytes: Vec<u8>,
    to_unicode: String,
    widths: Vec<Object>,
) -> ObjectId {
    let font_name = font
        .font_name
        .clone()
        .unwrap_or_else(|| resource_name.to_string())
        .replace(' ', "");
    let face_name = format!("DOCSY+{font_name}");
    let (subtype, font_file_key, font_stream) = match &font.font_type {
        FontType::OpenTypeCFF(_) => (
            "CIDFontType0",
            "FontFile3",
            Stream::new(
                dictionary! {
                    "Subtype" => "CIDFontType0C",
                },
                font_bytes,
            )
            .with_compression(false),
        ),
        FontType::TrueType => (
            "CIDFontType2",
            "FontFile2",
            Stream::new(Dictionary::new(), font_bytes).with_compression(false),
        ),
    };
    let font_file_id = doc.add_object(font_stream);
    let to_unicode_id = doc.add_object(Stream::new(Dictionary::new(), to_unicode.into_bytes()));
    let descriptor_id = doc.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => Object::Name(face_name.as_bytes().to_vec()),
        "Ascent" => font.font_metrics.ascent as i64,
        "Descent" => font.font_metrics.descent as i64,
        "CapHeight" => font.font_metrics.ascent as i64,
        "ItalicAngle" => 0,
        "Flags" => 32,
        "StemV" => 80,
        font_file_key => font_file_id,
        "FontBBox" => vec![
            (font.pdf_font_metrics.x_min as i64).into(),
            (font.pdf_font_metrics.y_min as i64).into(),
            (font.pdf_font_metrics.x_max as i64).into(),
            (font.pdf_font_metrics.y_max as i64).into(),
        ],
    });
    let descendant = Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => subtype,
        "BaseFont" => Object::Name(face_name.as_bytes().to_vec()),
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "W" => Object::Array(widths),
        "DW" => 1000,
        "FontDescriptor" => descriptor_id,
    });
    doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => Object::Name(face_name.as_bytes().to_vec()),
        "Encoding" => "Identity-H",
        "ToUnicode" => to_unicode_id,
        "DescendantFonts" => vec![descendant],
    })
}

fn append_overlay_text_ops(
    ops: &mut Vec<Operation>,
    config: &OverlayTextConfig,
    region: OverlayRegion,
    size: &PageSize,
    current_page: u32,
    total_pages: u32,
    embedded_fonts: &BTreeMap<String, EmbeddedOverlayFont>,
) {
    let text = expand_placeholders(&config.text, current_page, total_pages);
    if text.is_empty() {
        return;
    }
    let y = match region {
        OverlayRegion::Header => size.height_pt - mm_to_pt(config.margin_mm),
        OverlayRegion::Footer => mm_to_pt(config.margin_mm),
    };
    let font_ref = overlay_font_ref(config, &text, embedded_fonts);
    let use_embedded = matches!(
        font_ref,
        OverlayFontRef::Embedded(_) | OverlayFontRef::StandardCjk
    );
    let x = compute_x(config, &text, use_embedded, size.width_pt);
    ops.extend(text_ops(
        &font_ref,
        config.font_size,
        x,
        y,
        &config.color,
        text,
    ));
}

fn overlay_font_ref<'a>(
    config: &OverlayTextConfig,
    text: &str,
    embedded_fonts: &'a BTreeMap<String, EmbeddedOverlayFont>,
) -> OverlayFontRef<'a> {
    if requires_embedded_font(text) {
        let key = font_family_key(&config.font_family);
        if let Some(font) = embedded_fonts.get(&key) {
            return OverlayFontRef::Embedded(font);
        }
        return OverlayFontRef::StandardCjk;
    }
    match config.font_family.trim().to_lowercase().as_str() {
        "times" | "times new roman" | "times-roman" => OverlayFontRef::Builtin("FTimes"),
        "courier" | "courier new" => OverlayFontRef::Builtin("FCourier"),
        _ => OverlayFontRef::Builtin("F1"),
    }
}

fn text_ops(
    font_ref: &OverlayFontRef<'_>,
    font_size: f32,
    x: f32,
    y: f32,
    color: &str,
    text: String,
) -> Vec<Operation> {
    let (r, g, b) = parse_hex_color(color).unwrap_or((0.0, 0.0, 0.0));
    let (font_name, text_object) = match font_ref {
        OverlayFontRef::Builtin(name) => (*name, Object::string_literal(text)),
        OverlayFontRef::Embedded(font) => (
            font.resource_name.as_str(),
            Object::String(
                encode_subset_glyph_text(&text, &font.char_to_gid),
                StringFormat::Hexadecimal,
            ),
        ),
        OverlayFontRef::StandardCjk => (
            "FCJKFallback",
            Object::String(encode_utf16be_text(&text), StringFormat::Hexadecimal),
        ),
    };
    vec![
        Operation::new("q", vec![]),
        Operation::new("BT", vec![]),
        Operation::new(
            "Tf",
            vec![
                Object::Name(font_name.as_bytes().to_vec()),
                font_size.into(),
            ],
        ),
        Operation::new("rg", vec![r.into(), g.into(), b.into()]),
        Operation::new(
            "Tm",
            vec![1.into(), 0.into(), 0.into(), 1.into(), x.into(), y.into()],
        ),
        Operation::new("Tj", vec![text_object]),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
    ]
}

fn encode_utf16be_text(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .flat_map(|unit| unit.to_be_bytes())
        .collect()
}

fn encode_subset_glyph_text(text: &str, char_to_gid: &BTreeMap<char, u16>) -> Vec<u8> {
    text.chars()
        .flat_map(|ch| {
            let gid = char_to_gid.get(&ch).copied().unwrap_or(0);
            gid.to_be_bytes()
        })
        .collect()
}

fn requires_embedded_font(text: &str) -> bool {
    text.chars().any(|ch| {
        let cp = ch as u32;
        !(0x20..=0x7E).contains(&cp)
    })
}

fn font_family_key(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "heiti" | "黑体" | "simhei" | "microsoft yahei" | "微软雅黑" | "pingfang" | "苹方" => {
            "heiti".to_string()
        }
        "kaiti" | "楷体" | "simkai" => "kaiti".to_string(),
        "fangsong" | "仿宋" | "simfang" => "fangsong".to_string(),
        "songti" | "宋体" | "simsun" | "serif" => "songti".to_string(),
        _ => "songti".to_string(),
    }
}

fn display_font_family(key: &str) -> &'static str {
    match key {
        "heiti" => "黑体",
        "kaiti" => "楷体",
        "fangsong" => "仿宋",
        _ => "宋体",
    }
}

fn font_candidate_sequence(family: &str) -> Vec<FontCandidate> {
    let mut candidates = Vec::new();
    for fallback_family in fallback_font_families(family) {
        for path in font_paths_for_family(fallback_family) {
            candidates.push(FontCandidate {
                family: fallback_family.to_string(),
                path,
            });
        }
    }
    candidates
}

fn fallback_font_families(family: &str) -> Vec<&'static str> {
    match family {
        "heiti" => vec!["heiti", "songti", "fangsong", "kaiti"],
        "kaiti" => vec!["kaiti", "songti", "fangsong", "heiti"],
        "fangsong" => vec!["fangsong", "songti", "kaiti", "heiti"],
        _ => vec!["songti", "fangsong", "kaiti", "heiti"],
    }
}

fn font_paths_for_family(family: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "macos")]
    {
        match family {
            "heiti" => {
                paths.push(PathBuf::from("/System/Library/Fonts/STHeiti Medium.ttc"));
                paths.push(PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"));
                paths.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
            }
            "kaiti" => {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Kaiti.ttc",
                ));
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Kaiti.ttf",
                ));
            }
            "fangsong" => {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/STFangsong.ttf",
                ));
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Fangsong.ttf",
                ));
            }
            _ => {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Songti.ttc",
                ));
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Songti.ttf",
                ));
                paths.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let fonts = PathBuf::from(win).join("Fonts");
        match family {
            "heiti" => {
                paths.push(fonts.join("simhei.ttf"));
                paths.push(fonts.join("msyh.ttc"));
                paths.push(fonts.join("msyh.ttf"));
            }
            "kaiti" => paths.push(fonts.join("simkai.ttf")),
            "fangsong" => paths.push(fonts.join("simfang.ttf")),
            _ => {
                paths.push(fonts.join("simsun.ttc"));
                paths.push(fonts.join("simsun.ttf"));
                paths.push(fonts.join("msyh.ttc"));
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        match family {
            "heiti" => {
                paths.push(PathBuf::from(
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                ));
                paths.push(PathBuf::from(
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                ));
            }
            _ => {
                paths.push(PathBuf::from(
                    "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
                ));
                paths.push(PathBuf::from(
                    "/usr/share/fonts/truetype/noto/NotoSerifCJK-Regular.ttc",
                ));
                paths.push(PathBuf::from(
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                ));
            }
        }
    }
    paths
}

fn parse_hex_color(value: &str) -> Option<(f32, f32, f32)> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&trimmed[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&trimmed[4..6], 16).ok()? as f32 / 255.0;
    Some((r, g, b))
}

fn expand_placeholders(template: &str, page: u32, total: u32) -> String {
    template
        .replace("{page}", &page.to_string())
        .replace("{total}", &total.to_string())
        .replace("{range}", &format!("{page}/{total}"))
}

fn compute_x(config: &OverlayTextConfig, text: &str, use_cjk: bool, page_width: f32) -> f32 {
    let text_width = estimate_text_width(text, use_cjk, config.font_size);
    let offset = mm_to_pt(config.offset_x_mm);
    let margin = mm_to_pt(config.margin_mm);

    match config.align.as_str() {
        "left" => (margin + offset).max(0.0),
        "right" => (page_width - margin - text_width + offset).max(0.0),
        _ => ((page_width - text_width) / 2.0 + offset).max(0.0),
    }
}

fn estimate_text_width(text: &str, use_cjk: bool, font_size: f32) -> f32 {
    text.chars()
        .map(|c| estimate_char_width(c, !use_cjk) * font_size)
        .sum()
}

fn estimate_char_width(c: char, is_builtin: bool) -> f32 {
    let cp = c as u32;
    let cjk = (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x20000..=0x2A6DF).contains(&cp);
    if cjk {
        return 1.0;
    }
    if !is_builtin {
        return 0.5;
    }
    match c {
        ' ' => 0.278,
        '0'..='9' => 0.556,
        'A'..='Z' => 0.667,
        'a'..='z' => 0.500,
        '.' | ',' | ':' | ';' | '/' | '\\' | '\'' | '"' => 0.278,
        '-' | '_' | '(' | ')' | '[' | ']' | '{' | '}' => 0.333,
        _ => 0.556,
    }
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

fn same_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normalize_path_components(&absolute)
}

fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn temp_named_path(prefix: &str, extension: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}.{extension}"))
}

struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn cleanup_temp(path: Option<PathBuf>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}

fn cleanup_semantic_temp(result: Option<StandardArtifactProcessingResult>) {
    if let Some(result) = result {
        let _ = fs::remove_file(result.path);
    }
}

fn cleanup_plain_text_temp(result: Option<(PathBuf, content_text::PlainTextCleanupResult)>) {
    if let Some((path, _)) = result {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_global_page_placeholders() {
        assert_eq!(
            expand_placeholders("{page}/{total} {range}", 13, 30),
            "13/30 13/30"
        );
    }

    #[test]
    fn rejects_same_paths() {
        assert!(same_path(Path::new("/tmp/a.pdf"), Path::new("/tmp/a.pdf")));
        assert!(!same_path(
            Path::new("/tmp/a.pdf"),
            Path::new("/tmp/a_overlay.pdf")
        ));
    }

    #[test]
    fn parses_hex_text_color() {
        let (r, g, b) = parse_hex_color("#336699").unwrap();
        assert!((r - 0.2).abs() < 0.01);
        assert!((g - 0.4).abs() < 0.01);
        assert!((b - 0.6).abs() < 0.01);
        assert!(parse_hex_color("not-a-color").is_none());
    }

    #[test]
    fn compute_x_uses_configured_margin_for_left_and_right_alignment() {
        let mut config = OverlayTextConfig {
            text: String::new(),
            region: "header".to_string(),
            font_family: "auto".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "left".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
        };

        assert!((compute_x(&config, "abc", false, 200.0) - mm_to_pt(10.0)).abs() < 0.01);
        config.align = "right".to_string();
        let expected = 200.0 - mm_to_pt(10.0) - estimate_text_width("abc", false, 10.0);
        assert!((compute_x(&config, "abc", false, 200.0) - expected).abs() < 0.01);
    }

    #[test]
    fn cjk_overlay_embeds_only_subset_font() {
        let pages = vec![PageSize {
            width_pt: 595.0,
            height_pt: 842.0,
            raw_width_pt: 595.0,
            raw_height_pt: 842.0,
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
        };
        let (bytes, _warnings) = build_overlay_pdf(
            Some(&header),
            None,
            &[],
            &pages,
            1,
            1,
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
        .unwrap();

        assert!(
            bytes.len() < 500_000,
            "overlay PDF too large: {}",
            bytes.len()
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
        };

        assert!(!overlay_applies_to_page(&config, 1));
        assert!(overlay_applies_to_page(&config, 2));
        assert!(overlay_applies_to_page(&config, 3));
        assert!(!overlay_applies_to_page(&config, 4));
    }

    #[test]
    fn artifact_rebuild_overlays_keep_only_removed_page_ranges() {
        let config = OverlayTextConfig {
            text: "替代页眉".to_string(),
            region: "header".to_string(),
            font_family: "songti".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "right".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
        };
        let pages = BTreeSet::from([0_usize, 2, 3]);

        let overlays = artifact_rebuild_overlays_for_region(&config, "header", &pages);

        assert_eq!(overlays.len(), 2);
        assert_eq!(overlays[0].page_start, Some(1));
        assert_eq!(overlays[0].page_end, Some(1));
        assert_eq!(overlays[1].page_start, Some(3));
        assert_eq!(overlays[1].page_end, Some(4));
    }

    #[test]
    fn font_fallback_sequence_tries_similar_families_before_standard_cjk() {
        let families = font_candidate_sequence("songti")
            .into_iter()
            .map(|candidate| candidate.family)
            .collect::<Vec<_>>();

        assert!(families.iter().any(|family| family == "songti"));
        assert!(families.iter().any(|family| family == "fangsong"));
        assert!(families.iter().any(|family| family == "kaiti"));
        assert!(families.iter().any(|family| family == "heiti"));
        assert_eq!(families.first().map(String::as_str), Some("songti"));
    }

    #[test]
    fn processing_cjk_header_does_not_embed_full_font() {
        if crate::external::QpdfTool.binary_path().is_err() {
            return;
        }
        let input = temp_named_path("docsy_hf_process_input", "pdf");
        let output = temp_named_path("docsy_hf_process_output", "pdf");
        create_simple_test_pdf(&input);

        let result = process_job(&HeaderFooterJob {
            input_path: input.to_string_lossy().to_string(),
            output_path: output.to_string_lossy().to_string(),
            page_start: 1,
            total_pages: Some(1),
            normalize_a4: false,
            a4_orientation: default_a4_orientation(),
            raster_dpi: default_raster_dpi(),
            cleanup: CleanupConfig::default(),
            header: Some(OverlayTextConfig {
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
            }),
            footer: None,
            extra_overlays: Vec::new(),
        })
        .unwrap();

        let output_size = fs::metadata(&result.output_path).unwrap().len();
        assert!(
            output_size < 1_000_000,
            "processed PDF too large: {}",
            output_size
        );
        if let Ok(text_output) = std::process::Command::new("pdftotext")
            .arg(&result.output_path)
            .arg("-")
            .output()
        {
            if text_output.status.success() {
                let extracted = String::from_utf8_lossy(&text_output.stdout);
                assert!(
                    extracted.contains("测试页眉3"),
                    "processed PDF text missing inserted header: {}",
                    extracted
                );
            }
        }
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(result.output_path);
        let _ = fs::remove_file(output);
    }

    fn create_simple_test_pdf(path: &Path) {
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
}
