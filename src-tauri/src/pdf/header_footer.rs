use anyhow::{Context, Result};
use printpdf::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
    let artifact_removed = semantic_deleted_path
        .as_ref()
        .map(|artifact_result| artifact_result.changed_count())
        .unwrap_or(0);
    let semantic_input = semantic_deleted_path
        .as_ref()
        .map(|artifact_result| artifact_result.path.as_path())
        .unwrap_or(input);
    let plain_deleted_path =
        delete_confirmed_plain_text_header_footer_if_requested(args, semantic_input)?;
    let plain_removed = plain_deleted_path
        .as_ref()
        .map(|(_, result)| result.removed())
        .unwrap_or(0);
    let cleanup_input = plain_deleted_path
        .as_ref()
        .map(|(path, _)| path.as_path())
        .unwrap_or(semantic_input);
    let semantic_removed = artifact_removed + plain_removed;

    let normalized_path = if args.normalize_a4 {
        Some(normalize_pdf_to_a4(
            cleanup_input,
            args.raster_dpi,
            &args.a4_orientation,
        )?)
    } else {
        None
    };
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

    let cleaned = args.cleanup.header_enabled || args.cleanup.footer_enabled;
    if args.header.is_none() && args.footer.is_none() && args.extra_overlays.is_empty() {
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
        });
    }

    let overlay_pdf = build_overlay_pdf(
        args.header.as_ref(),
        args.footer.as_ref(),
        &args.extra_overlays,
        &page_infos,
        page_start,
        total_pages,
        &BTreeSet::new(),
        &BTreeSet::new(),
    )?;
    let overlay_path = temp_named_path("docsy_overlay", "pdf");
    fs::write(&overlay_path, &overlay_pdf).context("写入临时页眉页脚层失败")?;

    let qpdf_tool = crate::external::QpdfTool;
    let bin = qpdf_tool.binary_path()?;
    let overlay_output = temp_named_path("docsy_overlay_result", "pdf");
    let command_output = std::process::Command::new(&bin)
        .arg(work_input)
        .arg("--overlay")
        .arg(&overlay_path)
        .arg("--")
        .arg(&overlay_output)
        .output()
        .context("执行 qpdf overlay 失败")?;

    let _ = fs::remove_file(&overlay_path);
    cleanup_temp(normalized_path);
    cleanup_plain_text_temp(plain_deleted_path);
    cleanup_semantic_temp(semantic_deleted_path);

    if !command_output.status.success() {
        let _ = fs::remove_file(&overlay_output);
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        anyhow::bail!("qpdf overlay 失败: {}", stderr.trim());
    }
    write_optimized_or_copy(&overlay_output, output).context("写入页眉页脚处理结果失败")?;
    let _ = fs::remove_file(&overlay_output);

    Ok(HeaderFooterResult {
        input_path: args.input_path.clone(),
        output_path: output.to_string_lossy().to_string(),
        pages,
        normalized: args.normalize_a4,
        cleaned,
        semantic_removed,
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
    removed: usize,
    edited: usize,
}

impl StandardArtifactProcessingResult {
    fn changed_count(&self) -> usize {
        self.removed + self.edited
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
            removed: result.removed_header + result.removed_footer,
            edited: result.edited_header + result.edited_footer,
        }),
    )
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

fn build_overlay_pdf(
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
    extra_overlays: &[OverlayTextConfig],
    pages: &[PageSize],
    page_start: u32,
    total_pages: u32,
    skip_header_pages: &BTreeSet<usize>,
    skip_footer_pages: &BTreeSet<usize>,
) -> Result<Vec<u8>> {
    let mut doc = PdfDocument::new("Docsy Header Footer Processor");
    let mut font_cache = OverlayFontCache::default();
    let mut pdf_pages = Vec::new();

    for (index, size) in pages.iter().enumerate() {
        let current_page = page_start + index as u32;
        let mut ops = Vec::new();

        if let Some(config) = header.filter(|_| !skip_header_pages.contains(&index)) {
            append_overlay_text_ops(
                &mut ops,
                config,
                OverlayRegion::Header,
                size,
                current_page,
                total_pages,
                &mut font_cache,
                &mut doc,
            );
        }

        if let Some(config) = footer.filter(|_| !skip_footer_pages.contains(&index)) {
            append_overlay_text_ops(
                &mut ops,
                config,
                OverlayRegion::Footer,
                size,
                current_page,
                total_pages,
                &mut font_cache,
                &mut doc,
            );
        }

        for config in extra_overlays {
            let region = overlay_region(&config.region);
            append_overlay_text_ops(
                &mut ops,
                config,
                region,
                size,
                current_page,
                total_pages,
                &mut font_cache,
                &mut doc,
            );
        }

        pdf_pages.push(PdfPage::new(
            Mm(pt_to_mm(size.width_pt)),
            Mm(pt_to_mm(size.height_pt)),
            ops,
        ));
    }

    Ok(doc
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new()))
}

#[derive(Debug, Clone, Copy)]
enum OverlayRegion {
    Header,
    Footer,
}

fn overlay_region(value: &str) -> OverlayRegion {
    if value.trim().eq_ignore_ascii_case("header") {
        OverlayRegion::Header
    } else {
        OverlayRegion::Footer
    }
}

fn append_overlay_text_ops(
    ops: &mut Vec<Op>,
    config: &OverlayTextConfig,
    region: OverlayRegion,
    size: &PageSize,
    current_page: u32,
    total_pages: u32,
    font_cache: &mut OverlayFontCache,
    doc: &mut PdfDocument,
) {
    let text = expand_placeholders(&config.text, current_page, total_pages);
    if text.is_empty() {
        return;
    }
    let font = font_cache.select_font(&text, &config.font_family, doc);
    let y = match region {
        OverlayRegion::Header => size.height_pt - mm_to_pt(config.margin_mm),
        OverlayRegion::Footer => mm_to_pt(config.margin_mm),
    };
    let x = compute_x(config, &text, font.clone(), size.width_pt);
    ops.extend(text_ops(font, config.font_size, x, y, &config.color, text));
}

#[derive(Default)]
struct OverlayFontCache {
    cjk: Option<PdfFontHandle>,
    songti: Option<PdfFontHandle>,
    heiti: Option<PdfFontHandle>,
    kaiti: Option<PdfFontHandle>,
    fangsong: Option<PdfFontHandle>,
}

impl OverlayFontCache {
    fn select_font(
        &mut self,
        text: &str,
        font_family: &str,
        doc: &mut PdfDocument,
    ) -> PdfFontHandle {
        match normalized_font_family(font_family).as_str() {
            "helvetica" => return PdfFontHandle::Builtin(BuiltinFont::Helvetica),
            "times" => return PdfFontHandle::Builtin(BuiltinFont::TimesRoman),
            "courier" => return PdfFontHandle::Builtin(BuiltinFont::Courier),
            "songti" => {
                if let Some(font) = load_named_cjk_font(&mut self.songti, "songti", doc) {
                    return font;
                }
            }
            "heiti" => {
                if let Some(font) = load_named_cjk_font(&mut self.heiti, "heiti", doc) {
                    return font;
                }
            }
            "kaiti" => {
                if let Some(font) = load_named_cjk_font(&mut self.kaiti, "kaiti", doc) {
                    return font;
                }
            }
            "fangsong" => {
                if let Some(font) = load_named_cjk_font(&mut self.fangsong, "fangsong", doc) {
                    return font;
                }
            }
            _ => {}
        }
        if has_cjk(text) {
            if let Some(font) = &self.cjk {
                return font.clone();
            }
            if let Some(font_path) = find_cjk_font_path() {
                if let Ok(bytes) = fs::read(&font_path) {
                    if let Some(parsed) = ParsedFont::from_bytes(&bytes, 0, &mut Vec::new()) {
                        let font = PdfFontHandle::External(doc.add_font(&parsed));
                        self.cjk = Some(font.clone());
                        return font;
                    }
                }
            }
        }

        PdfFontHandle::Builtin(BuiltinFont::Helvetica)
    }
}

fn normalized_font_family(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "宋体" | "simsun" | "songti" | "song" => "songti".to_string(),
        "黑体" | "simhei" | "heiti" | "hei" => "heiti".to_string(),
        "楷体" | "kaiti" | "kai" => "kaiti".to_string(),
        "仿宋" | "fangsong" | "fang song" => "fangsong".to_string(),
        "times" | "times new roman" | "times-roman" => "times".to_string(),
        "courier" | "courier new" => "courier".to_string(),
        "helvetica" | "arial" => "helvetica".to_string(),
        _ => "auto".to_string(),
    }
}

fn load_named_cjk_font(
    cache: &mut Option<PdfFontHandle>,
    family: &str,
    doc: &mut PdfDocument,
) -> Option<PdfFontHandle> {
    if let Some(font) = cache {
        return Some(font.clone());
    }
    let path = find_named_cjk_font_path(family)?;
    let bytes = fs::read(path).ok()?;
    let parsed = ParsedFont::from_bytes(&bytes, 0, &mut Vec::new())?;
    let font = PdfFontHandle::External(doc.add_font(&parsed));
    *cache = Some(font.clone());
    Some(font)
}

fn text_ops(
    font: PdfFontHandle,
    font_size: f32,
    x: f32,
    y: f32,
    color: &str,
    text: String,
) -> Vec<Op> {
    let (r, g, b) = parse_hex_color(color).unwrap_or((0.0, 0.0, 0.0));
    vec![
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point { x: Pt(x), y: Pt(y) },
        },
        Op::SetFont {
            font,
            size: Pt(font_size),
        },
        Op::SetLineHeight { lh: Pt(font_size) },
        Op::SetFillColor {
            col: Color::Rgb(Rgb {
                r,
                g,
                b,
                icc_profile: None,
            }),
        },
        Op::ShowText {
            items: vec![TextItem::Text(text)],
        },
        Op::EndTextSection,
    ]
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

fn has_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        let cp = c as u32;
        (0x4E00..=0x9FFF).contains(&cp)
            || (0x3400..=0x4DBF).contains(&cp)
            || (0x20000..=0x2A6DF).contains(&cp)
            || (0xF900..=0xFAFF).contains(&cp)
            || (0x2F800..=0x2FA1F).contains(&cp)
            || (0x3000..=0x303F).contains(&cp)
            || (0xFF00..=0xFFEF).contains(&cp)
            || (0x3040..=0x309F).contains(&cp)
            || (0x30A0..=0x30FF).contains(&cp)
            || (0xAC00..=0xD7AF).contains(&cp)
    })
}

fn find_cjk_font_path() -> Option<PathBuf> {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Supplemental/Songti.ttc",
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\msyhbd.ttc",
        ]
    } else {
        vec![
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        ]
    };

    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
}

fn find_named_cjk_font_path(family: &str) -> Option<PathBuf> {
    let candidates = if cfg!(target_os = "macos") {
        match family {
            "songti" => vec![
                "/System/Library/Fonts/Supplemental/Songti.ttc",
                "/System/Library/Fonts/Supplemental/Songti.ttf",
            ],
            "heiti" => vec![
                "/System/Library/Fonts/STHeiti Medium.ttc",
                "/System/Library/Fonts/STHeiti Light.ttc",
                "/System/Library/Fonts/PingFang.ttc",
            ],
            "kaiti" => vec![
                "/System/Library/Fonts/Supplemental/Kaiti.ttc",
                "/System/Library/Fonts/Supplemental/Kaiti.ttf",
            ],
            "fangsong" => vec![
                "/System/Library/Fonts/Supplemental/STFangsong.ttf",
                "/System/Library/Fonts/Supplemental/Fangsong.ttf",
            ],
            _ => vec![],
        }
    } else if cfg!(target_os = "windows") {
        match family {
            "songti" => vec!["C:\\Windows\\Fonts\\simsun.ttc"],
            "heiti" => vec![
                "C:\\Windows\\Fonts\\simhei.ttf",
                "C:\\Windows\\Fonts\\msyh.ttc",
            ],
            "kaiti" => vec!["C:\\Windows\\Fonts\\simkai.ttf"],
            "fangsong" => vec!["C:\\Windows\\Fonts\\simfang.ttf"],
            _ => vec![],
        }
    } else {
        match family {
            "songti" => vec![
                "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
                "/usr/share/fonts/truetype/arphic/uming.ttc",
            ],
            "heiti" => vec![
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            ],
            "kaiti" => vec!["/usr/share/fonts/truetype/arphic/ukai.ttc"],
            "fangsong" => vec!["/usr/share/fonts/truetype/arphic/uming.ttc"],
            _ => vec![],
        }
    };

    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
}

fn compute_x(config: &OverlayTextConfig, text: &str, font: PdfFontHandle, page_width: f32) -> f32 {
    let text_width = estimate_text_width(text, font, config.font_size);
    let offset = mm_to_pt(config.offset_x_mm);

    match config.align.as_str() {
        "left" => 36.0 + offset,
        "right" => (page_width - 36.0 - text_width + offset).max(36.0),
        _ => ((page_width - text_width) / 2.0 + offset).max(0.0),
    }
}

fn estimate_text_width(text: &str, font: PdfFontHandle, font_size: f32) -> f32 {
    let is_builtin = matches!(font, PdfFontHandle::Builtin(_));
    text.chars()
        .map(|c| estimate_char_width(c, is_builtin) * font_size)
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

fn pt_to_mm(pt: f32) -> f32 {
    pt * 25.4 / 72.0
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
}
