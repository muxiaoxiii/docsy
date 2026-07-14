use anyhow::{Context, Result};
use printpdf::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

use super::annotations;
use super::artifacts::{self, HeaderFooterArtifactTargets};
use super::normalize::normalize_pdf_to_a4;
use super::page_info::{get_page_infos, PageSize};
use super::preview::{render_preview, PreviewResult};

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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CleanupConfig {
    #[serde(default)]
    header_enabled: bool,
    #[serde(default)]
    footer_enabled: bool,
    #[serde(default = "default_cleanup_height_mm")]
    header_height_mm: f32,
    #[serde(default = "default_cleanup_height_mm")]
    footer_height_mm: f32,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            header_enabled: false,
            footer_enabled: false,
            header_height_mm: default_cleanup_height_mm(),
            footer_height_mm: default_cleanup_height_mm(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayTextConfig {
    text: String,
    #[serde(default = "default_font_size")]
    font_size: f32,
    #[serde(default = "default_margin_mm")]
    margin_mm: f32,
    #[serde(default = "default_align")]
    align: String,
    #[serde(default)]
    offset_x_mm: f32,
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

fn default_cleanup_height_mm() -> f32 {
    18.0
}

fn default_align() -> String {
    "center".to_string()
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
    let output = Path::new(&args.output_path);
    if same_path(input, output) {
        anyhow::bail!("输出路径不能和原始 PDF 相同，请另存为副本");
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).context("创建输出目录失败")?;
    }

    let semantic_deleted_path = delete_standard_artifacts_if_requested(args)?;
    let semantic_removed = semantic_deleted_path
        .as_ref()
        .map(|(_, removed)| *removed)
        .unwrap_or(0);
    let semantic_input = semantic_deleted_path
        .as_ref()
        .map(|(path, _)| path.as_path())
        .unwrap_or(input);

    let normalized_path = if args.normalize_a4 {
        Some(normalize_pdf_to_a4(
            semantic_input,
            args.raster_dpi,
            &args.a4_orientation,
        )?)
    } else {
        None
    };
    let work_input = normalized_path.as_deref().unwrap_or(semantic_input);
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
    if args.header.is_none() && args.footer.is_none() && !cleaned {
        fs::copy(work_input, output).context("复制 PDF 失败")?;
        cleanup_temp(normalized_path);
        cleanup_semantic_temp(semantic_deleted_path);
        return Ok(HeaderFooterResult {
            input_path: args.input_path.clone(),
            output_path: args.output_path.clone(),
            pages,
            normalized: args.normalize_a4,
            cleaned,
            semantic_removed,
        });
    }

    let overlay_pdf = build_overlay_pdf(
        &args.cleanup,
        args.header.as_ref(),
        args.footer.as_ref(),
        &page_infos,
        page_start,
        total_pages,
    )?;
    let overlay_path = temp_named_path("docsy_overlay", "pdf");
    fs::write(&overlay_path, &overlay_pdf).context("写入临时页眉页脚层失败")?;

    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let command_output = std::process::Command::new(&bin)
        .arg(work_input)
        .arg("--overlay")
        .arg(&overlay_path)
        .arg("--")
        .arg(output)
        .output()
        .context("执行 qpdf overlay 失败")?;

    let _ = fs::remove_file(&overlay_path);
    cleanup_temp(normalized_path);
    cleanup_semantic_temp(semantic_deleted_path);

    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        anyhow::bail!("qpdf overlay 失败: {}", stderr.trim());
    }

    Ok(HeaderFooterResult {
        input_path: args.input_path.clone(),
        output_path: args.output_path.clone(),
        pages,
        normalized: args.normalize_a4,
        cleaned,
        semantic_removed,
    })
}

fn delete_standard_artifacts_if_requested(
    args: &HeaderFooterJob,
) -> Result<Option<(PathBuf, usize)>> {
    if !args.cleanup.header_enabled && !args.cleanup.footer_enabled {
        return Ok(None);
    }
    let result = artifacts::delete_header_footer_artifacts_to_temp(
        &args.input_path,
        HeaderFooterArtifactTargets {
            header: args.cleanup.header_enabled,
            footer: args.cleanup.footer_enabled,
        },
    )?;
    Ok(result.map(|(path, result)| (path, result.removed_count())))
}

fn build_overlay_pdf(
    cleanup: &CleanupConfig,
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
    pages: &[PageSize],
    page_start: u32,
    total_pages: u32,
) -> Result<Vec<u8>> {
    let mut doc = PdfDocument::new("Docsy Header Footer Processor");
    let mut pdf_pages = Vec::new();

    for (index, size) in pages.iter().enumerate() {
        let current_page = page_start + index as u32;
        let mut ops = cleanup_ops(cleanup, size);

        if let Some(config) = header {
            let text = expand_placeholders(&config.text, current_page, total_pages);
            let font = select_font(&text, &mut doc);
            let y = size.height_pt - mm_to_pt(config.margin_mm);
            let x = compute_x(config, &text, font.clone(), size.width_pt);
            ops.extend(text_ops(font, config.font_size, x, y, text));
        }

        if let Some(config) = footer {
            let text = expand_placeholders(&config.text, current_page, total_pages);
            let font = select_font(&text, &mut doc);
            let y = mm_to_pt(config.margin_mm);
            let x = compute_x(config, &text, font.clone(), size.width_pt);
            ops.extend(text_ops(font, config.font_size, x, y, text));
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

fn cleanup_ops(cleanup: &CleanupConfig, size: &PageSize) -> Vec<Op> {
    let mut ops = Vec::new();
    if cleanup.header_enabled {
        let height = mm_to_pt(cleanup.header_height_mm).min(size.height_pt);
        ops.extend(white_rect_ops(
            0.0,
            size.height_pt - height,
            size.width_pt,
            height,
        ));
    }
    if cleanup.footer_enabled {
        let height = mm_to_pt(cleanup.footer_height_mm).min(size.height_pt);
        ops.extend(white_rect_ops(0.0, 0.0, size.width_pt, height));
    }
    ops
}

fn white_rect_ops(x: f32, y: f32, width: f32, height: f32) -> Vec<Op> {
    let mut rect = Rect::from_xywh(Pt(x), Pt(y), Pt(width), Pt(height));
    rect.mode = Some(PaintMode::Fill);
    vec![
        Op::SetFillColor {
            col: Color::Rgb(Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                icc_profile: None,
            }),
        },
        Op::DrawPolygon {
            polygon: rect.to_polygon(),
        },
    ]
}

fn text_ops(font: PdfFontHandle, font_size: f32, x: f32, y: f32, text: String) -> Vec<Op> {
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
                r: 0.0,
                g: 0.0,
                b: 0.0,
                icc_profile: None,
            }),
        },
        Op::ShowText {
            items: vec![TextItem::Text(text)],
        },
        Op::EndTextSection,
    ]
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

fn select_font(text: &str, doc: &mut PdfDocument) -> PdfFontHandle {
    if has_cjk(text) {
        if let Some(font_path) = find_cjk_font_path() {
            if let Ok(bytes) = fs::read(&font_path) {
                if let Some(parsed) = ParsedFont::from_bytes(&bytes, 0, &mut Vec::new()) {
                    return PdfFontHandle::External(doc.add_font(&parsed));
                }
            }
        }
    }

    PdfFontHandle::Builtin(BuiltinFont::Helvetica)
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

fn cleanup_semantic_temp(path: Option<(PathBuf, usize)>) {
    if let Some((path, _)) = path {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::super::page_info::{A4_HEIGHT_PT, A4_WIDTH_PT};
    use super::*;

    #[test]
    fn expands_global_page_placeholders() {
        assert_eq!(
            expand_placeholders("{page}/{total} {range}", 13, 30),
            "13/30 13/30"
        );
    }

    #[test]
    fn creates_cleanup_rectangles_for_header_and_footer() {
        let ops = cleanup_ops(
            &CleanupConfig {
                header_enabled: true,
                footer_enabled: true,
                header_height_mm: 20.0,
                footer_height_mm: 12.0,
            },
            &PageSize {
                width_pt: A4_WIDTH_PT,
                height_pt: A4_HEIGHT_PT,
            },
        );
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn rejects_same_paths() {
        assert!(same_path(Path::new("/tmp/a.pdf"), Path::new("/tmp/a.pdf")));
        assert!(!same_path(
            Path::new("/tmp/a.pdf"),
            Path::new("/tmp/a_overlay.pdf")
        ));
    }
}
