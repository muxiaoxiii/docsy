//! 长图与长文档智能切片分页 (Long Image & Long Document A4 Slicer)
//!
//! 支持将网页长截图、手机长截屏图片（PNG/JPG/WebP/BMP）以及单页或多页 PDF，
//! 逐页智能避让文字与段落缝隙，按 A4 纸张规范切片分页，并支持页面方向、内容宽度、
//! 页眉（标题/网址、打印日期）、页脚（页码）等丰富选项。

use anyhow::{bail, Context, Result};
use chrono::Local;
use image::{DynamicImage, GenericImageView, GrayImage};
use printpdf::{
    Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, RawImage, XObjectTransform,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::header_footer::OverlayTextConfig;
use super::overlay_pdf::build_overlay_pdf;
use super::page_info::PageSize;
use crate::external::ExternalTool;
use crate::util::fs::{safe_file_stem, temp_named_path, unique_output_path};

/// A4 纸张物理尺寸 (mm)
pub const A4_WIDTH_MM: f64 = 210.0;
pub const A4_HEIGHT_MM: f64 = 297.0;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LongSliceArgs {
    pub input_path: String,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub output_filename: Option<String>,
    /// 纸张方向: "portrait" (默认) 或 "landscape"
    #[serde(default = "default_orientation")]
    pub orientation: String,
    /// 纸张四周留白 (mm)，默认 5.0mm
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f64,
    /// 内容宽度模式: "fit" (充满可用宽度 100%), "phone" (手机适中 55%), "custom" (自定义比例)
    #[serde(default = "default_width_mode")]
    pub content_width_mode: String,
    /// 自定义内容宽度百分比 (40.0 ~ 100.0)
    #[serde(default = "default_custom_width_percent")]
    pub custom_width_percent: f64,
    /// 内容水平对齐方式: "center" (默认), "left", "right"
    #[serde(default = "default_alignment")]
    pub alignment: String,
    /// 智能避让文字行（防腰斩）
    #[serde(default = "default_true")]
    pub smart_cut: bool,
    /// 切片重叠量 (mm)，默认 0.0mm
    #[serde(default)]
    pub overlap_mm: f64,
    /// 页眉设置
    #[serde(default)]
    pub header_enabled: bool,
    #[serde(default)]
    pub header_title: Option<String>,
    #[serde(default = "default_true")]
    pub header_date: bool,
    /// 是否在日期前加上 "打印日期：" 字样
    #[serde(default = "default_true")]
    pub header_date_prefix: bool,
    #[serde(default)]
    pub custom_date: Option<String>,
    /// 页脚设置
    #[serde(default = "default_true")]
    pub footer_enabled: bool,
    /// 页码样式: "page_of_total" (第 1 页 / 共 5 页), "simple" (1 / 5), "dash" (- 1 -)
    #[serde(default = "default_page_number_style")]
    pub footer_page_number_style: String,
}

fn default_orientation() -> String {
    "portrait".to_string()
}
fn default_margin_mm() -> f64 {
    5.0
}
fn default_width_mode() -> String {
    "fit".to_string()
}
fn default_custom_width_percent() -> f64 {
    55.0
}
fn default_alignment() -> String {
    "center".to_string()
}
fn default_true() -> bool {
    true
}
fn default_page_number_style() -> String {
    "page_of_total".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceInfo {
    pub page_index: usize,
    pub source_page_index: usize,
    pub y_start_px: u32,
    pub y_end_px: u32,
    pub height_px: u32,
    pub height_mm: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LongSlicePreviewResult {
    pub source_width: u32,
    pub source_height: u32,
    pub source_page_index: usize,
    pub source_page_count: usize,
    pub is_pdf: bool,
    pub total_pages: usize,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub content_width_mm: f64,
    pub slices: Vec<SliceInfo>,
    pub preview_data_url: Option<String>,
    pub preview_page: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LongSliceResult {
    pub output_path: String,
    pub total_pages: usize,
    pub source_width: u32,
    pub source_height: u32,
    pub source_page_count: usize,
}

#[derive(Debug, Clone)]
struct SourceSlicePlan {
    source_page_index: usize,
    width: u32,
    height: u32,
    slices: Vec<SliceInfo>,
}

#[derive(Debug, Clone)]
struct LongSlicePlan {
    paper_w_mm: f64,
    paper_h_mm: f64,
    content_w_mm: f64,
    sources: Vec<SourceSlicePlan>,
    slices: Vec<SliceInfo>,
}

fn is_pdf_path(input_path: &Path) -> bool {
    input_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

fn load_source_page_as_image(input_path: &Path, source_page_index: usize) -> Result<DynamicImage> {
    if is_pdf_path(input_path) {
        let preview_png = super::preview::render_pdf_page_to_png(
            input_path,
            source_page_index as u32,
            150,
        )
        .with_context(|| format!("长图 PDF 第 {source_page_index} 页解析与渲染失败"))?;
        let image = image::open(&preview_png).context("读取渲染后的 PDF 图像失败");
        let _ = fs::remove_file(preview_png);
        image.context("读取渲染后的 PDF 图像失败")
    } else {
        crate::util::images::open_oriented(input_path).context("读取输入图片失败")
    }
}

fn build_slice_plan(input_path: &Path, args: &LongSliceArgs) -> Result<LongSlicePlan> {
    let source_page_count = if is_pdf_path(input_path) {
        super::page_info::get_page_infos(&input_path.to_string_lossy())?.len()
    } else {
        1
    };
    let mut sources = Vec::with_capacity(source_page_count);
    let mut all_slices = Vec::new();
    let mut next_page_index = 1;
    let mut paper = (0.0, 0.0, 0.0);

    for source_page_index in 1..=source_page_count {
        crate::operations::check_current_cancelled()?;
        let source = load_source_page_as_image(input_path, source_page_index)?;
        let (width, height) = source.dimensions();
        let gray = args.smart_cut.then(|| source.to_luma8());
        let (paper_w_mm, paper_h_mm, content_w_mm, slices) = calculate_slices(
            width,
            height,
            gray.as_ref(),
            args,
            next_page_index,
            source_page_index,
        );
        next_page_index += slices.len();
        all_slices.extend(slices.iter().cloned());
        sources.push(SourceSlicePlan {
            source_page_index,
            width,
            height,
            slices,
        });
        paper = (paper_w_mm, paper_h_mm, content_w_mm);
    }

    Ok(LongSlicePlan {
        paper_w_mm: paper.0,
        paper_h_mm: paper.1,
        content_w_mm: paper.2,
        sources,
        slices: all_slices,
    })
}

/// 获取纸张尺寸 (宽 mm, 高 mm)
fn get_paper_size_mm(orientation: &str) -> (f64, f64) {
    if orientation.eq_ignore_ascii_case("landscape") {
        (A4_HEIGHT_MM, A4_WIDTH_MM)
    } else {
        (A4_WIDTH_MM, A4_HEIGHT_MM)
    }
}

/// 寻找智能切分线（视觉水平空白检测）
fn find_visual_cut_y(
    gray: &GrayImage,
    target_y: u32,
    search_range: u32,
    min_y: u32,
) -> u32 {
    let img_w = gray.width();
    let img_h = gray.height();
    if target_y >= img_h {
        return img_h;
    }

    let start_y = target_y.min(img_h.saturating_sub(1));
    let limit_y = min_y.max(target_y.saturating_sub(search_range));

    if start_y <= limit_y {
        return target_y;
    }

    let mut best_y = target_y;
    let mut min_variance = f64::MAX;

    for y in (limit_y..=start_y).rev().step_by(2) {
        let mut sum: u64 = 0;
        let mut sum_sq: u64 = 0;
        let mut white_like_count: u32 = 0;

        for x in (0..img_w).step_by(3) {
            let p = gray.get_pixel(x, y)[0] as u64;
            sum += p;
            sum_sq += p * p;
            if p >= 245 {
                white_like_count += 1;
            }
        }

        let sampled_pixels = ((img_w + 2) / 3) as f64;
        let mean = sum as f64 / sampled_pixels;
        let variance = (sum_sq as f64 / sampled_pixels) - (mean * mean);

        let white_ratio = white_like_count as f64 / sampled_pixels;
        let mut score = variance;
        if white_ratio > 0.95 {
            score -= 1000.0;
        } else if white_ratio > 0.85 {
            score -= 300.0;
        }

        let dist = (target_y - y) as f64;
        let total_penalty = score + dist * 0.15;

        if total_penalty < min_variance {
            min_variance = total_penalty;
            best_y = y;
        }
    }

    best_y
}

/// 计算所有切片位置及几何参数
pub fn calculate_slices(
    img_w: u32,
    img_h: u32,
    gray: Option<&GrayImage>,
    args: &LongSliceArgs,
    first_page_index: usize,
    source_page_index: usize,
) -> (f64, f64, f64, Vec<SliceInfo>) {
    let (paper_w_mm, paper_h_mm) = get_paper_size_mm(&args.orientation);
    let margin = args.margin_mm.max(0.0);

    let header_reserve_mm = if args.header_enabled { 8.0 } else { 0.0 };
    let footer_reserve_mm = if args.footer_enabled { 8.0 } else { 0.0 };

    let avail_w_mm = (paper_w_mm - 2.0 * margin).max(10.0);
    let avail_h_mm = (paper_h_mm - 2.0 * margin - header_reserve_mm - footer_reserve_mm).max(10.0);

    let content_w_mm = match args.content_width_mode.as_str() {
        "phone" => (avail_w_mm * 0.55).clamp(30.0, avail_w_mm),
        "custom" => {
            let pct = (args.custom_width_percent / 100.0).clamp(0.4, 1.0);
            (avail_w_mm * pct).clamp(30.0, avail_w_mm)
        }
        _ => avail_w_mm, // "fit"
    };

    let px_per_mm = img_w as f64 / content_w_mm;
    let ideal_slice_h_px = (avail_h_mm * px_per_mm).max(50.0);

    let overlap_px = (args.overlap_mm.max(0.0) * px_per_mm) as u32;
    let search_range_px = ((ideal_slice_h_px * 0.20) as u32).clamp(40, 400);

    let mut slices = Vec::new();
    let mut curr_y: u32 = 0;
    let mut page_idx = first_page_index;

    while curr_y < img_h {
        let target_bottom = (curr_y as f64 + ideal_slice_h_px) as u32;

        let actual_bottom = if target_bottom >= img_h {
            img_h
        } else if args.smart_cut {
            if let Some(g) = gray {
                find_visual_cut_y(g, target_bottom, search_range_px, curr_y + (ideal_slice_h_px * 0.4) as u32)
            } else {
                target_bottom
            }
        } else {
            target_bottom
        };

        let slice_h_px = actual_bottom.saturating_sub(curr_y).max(1);
        let slice_h_mm = slice_h_px as f64 / px_per_mm;

        slices.push(SliceInfo {
            page_index: page_idx,
            source_page_index,
            y_start_px: curr_y,
            y_end_px: actual_bottom,
            height_px: slice_h_px,
            height_mm: slice_h_mm,
        });

        if actual_bottom >= img_h {
            break;
        }

        curr_y = (actual_bottom.saturating_sub(overlap_px)).max(curr_y + 10);
        page_idx += 1;
    }

    (paper_w_mm, paper_h_mm, content_w_mm, slices)
}

/// 格式化打印日期
fn format_print_date(args: &LongSliceArgs) -> String {
    let date_str = args
        .custom_date
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    if args.header_date_prefix {
        format!("打印日期：{}", date_str)
    } else {
        date_str
    }
}

/// 构建页眉页脚配置
fn build_overlay_configs(
    args: &LongSliceArgs,
    margin_mm: f64,
) -> (
    Option<OverlayTextConfig>,
    Option<OverlayTextConfig>,
    Vec<OverlayTextConfig>,
) {
    let margin_text_mm = (margin_mm * 0.6).clamp(3.0, 10.0) as f32;
    let margin_x_mm = margin_mm.max(6.0) as f32;

    let header_config = if args.header_enabled {
        args.header_title.as_ref().and_then(|t| {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(OverlayTextConfig {
                    text: trimmed.to_string(),
                    region: "header".into(),
                    font_family: "sans".into(),
                    font_size: 8.5,
                    margin_mm: margin_text_mm,
                    margin_x_mm: Some(margin_x_mm),
                    align: "left".into(),
                    color: "#555555".into(),
                    ..Default::default()
                })
            }
        })
    } else {
        None
    };

    let mut extra_overlays = Vec::new();
    if args.header_enabled && args.header_date {
        extra_overlays.push(OverlayTextConfig {
            text: format_print_date(args),
            region: "header".into(),
            font_family: "sans".into(),
            font_size: 8.5,
            margin_mm: margin_text_mm,
            margin_x_mm: Some(margin_x_mm),
            align: "right".into(),
            color: "#555555".into(),
            ..Default::default()
        });
    }

    let footer_config = if args.footer_enabled {
        let style = match args.footer_page_number_style.as_str() {
            "simple" => "{page} / {total}",
            "dash" => "- {page} -",
            _ => "第 {page} 页 / 共 {total} 页",
        };
        Some(OverlayTextConfig {
            text: style.into(),
            region: "footer".into(),
            font_family: "sans".into(),
            font_size: 8.5,
            margin_mm: margin_text_mm,
            align: "center".into(),
            color: "#555555".into(),
            ..Default::default()
        })
    } else {
        None
    };

    (header_config, footer_config, extra_overlays)
}

/// 为一组切片构建纯图片基础 PDF
fn render_base_slices_pdf(
    input_path: &Path,
    sources: &[SourceSlicePlan],
    paper_w_mm: f64,
    paper_h_mm: f64,
    content_w_mm: f64,
    args: &LongSliceArgs,
) -> Result<Vec<u8>> {
    let margin_mm = args.margin_mm.max(0.0);
    let header_reserve_mm = if args.header_enabled { 8.0 } else { 0.0 };
    let mut doc = PdfDocument::new("Docsy Long Document Slice");

    for planned in sources {
        crate::operations::check_current_cancelled()?;
        let source = load_source_page_as_image(input_path, planned.source_page_index)?;
        let img_w = source.width();
        for slice in &planned.slices {
            let cropped = source.crop_imm(0, slice.y_start_px, img_w, slice.height_px);
            let raw = RawImage::from_dynamic_image(cropped).map_err(|e| anyhow::anyhow!(e))?;
            let image_id = doc.add_image(&raw);

            let content_x_mm = match args.alignment.as_str() {
                "left" => margin_mm,
                "right" => paper_w_mm - margin_mm - content_w_mm,
                _ => (paper_w_mm - content_w_mm) / 2.0,
            };

            let content_y_bottom_mm =
                paper_h_mm - margin_mm - header_reserve_mm - slice.height_mm;
            let target_w_pt = content_w_mm * 72.0 / 25.4;
            let native_w_pt = img_w as f64 * 72.0 / 300.0;
            let scale = (target_w_pt / native_w_pt) as f32;

            let ops = vec![
                Op::SaveGraphicsState,
                Op::UseXobject {
                    id: image_id,
                    transform: XObjectTransform {
                        translate_x: Some(Pt((content_x_mm * 72.0 / 25.4) as f32)),
                        translate_y: Some(Pt((content_y_bottom_mm * 72.0 / 25.4) as f32)),
                        scale_x: Some(scale),
                        scale_y: Some(scale),
                        dpi: Some(300.0),
                        ..Default::default()
                    },
                },
                Op::RestoreGraphicsState,
            ];
            doc.pages.push(PdfPage::new(
                Mm(paper_w_mm as f32),
                Mm(paper_h_mm as f32),
                ops,
            ));
        }
    }

    Ok(doc.save(&PdfSaveOptions::default(), &mut Vec::new()))
}

fn deref_dict<'a>(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Option<&'a lopdf::Dictionary> {
    match obj {
        lopdf::Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok(),
        lopdf::Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

/// 纯 Rust 内存中将 overlay 页面叠加到 base 页面上（无需依赖外部 qpdf 进程）
fn apply_overlay_in_memory(base_pdf_bytes: &[u8], overlay_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut base_doc = lopdf::Document::load_mem(base_pdf_bytes)?;
    let mut overlay_doc = lopdf::Document::load_mem(overlay_bytes)?;

    let max_base_id = base_doc.max_id + 1;
    overlay_doc.renumber_objects_with(max_base_id);

    let overlay_pages: Vec<lopdf::ObjectId> = overlay_doc.get_pages().into_values().collect();

    // 复制非 Page/Catalog/Pages 的对象（字体、流、编码字典等）
    for (&id, obj) in &overlay_doc.objects {
        if !overlay_pages.contains(&id) {
            base_doc.objects.insert(id, obj.clone());
        }
    }

    let base_pages: Vec<lopdf::ObjectId> = base_doc.get_pages().into_values().collect();
    for (i, base_pid) in base_pages.iter().enumerate() {
        if let Some(&overlay_pid) = overlay_pages.get(i) {
            let overlay_page_obj = overlay_doc.get_object(overlay_pid)?;
            let overlay_dict = overlay_page_obj.as_dict()?;

            // 1. 获取 overlay 页面的字体
            let overlay_fonts = overlay_dict
                .get(b"Resources")
                .ok()
                .and_then(|r| deref_dict(&overlay_doc, r))
                .and_then(|rd| rd.get(b"Font").ok())
                .and_then(|f| deref_dict(&overlay_doc, f))
                .cloned();

            // 2. 获取 overlay 页面的 Contents
            let overlay_contents = overlay_dict.get(b"Contents").cloned().ok();

            // 3. 将字体合并到 base 页面的 /Resources /Font
            if let Some(fonts) = overlay_fonts {
                let base_page_dict = base_doc.get_object_mut(*base_pid)?.as_dict_mut()?;
                if !base_page_dict.has(b"Resources") {
                    base_page_dict.set("Resources", lopdf::Dictionary::new());
                }
                let resources_obj = base_page_dict.get_mut(b"Resources")?;
                let res_dict = match resources_obj {
                    lopdf::Object::Reference(r) => {
                        let r_id = *r;
                        base_doc.get_object_mut(r_id)?.as_dict_mut()?
                    }
                    lopdf::Object::Dictionary(ref mut d) => d,
                    _ => continue,
                };
                if !res_dict.has(b"Font") {
                    res_dict.set("Font", lopdf::Dictionary::new());
                }
                let font_obj = res_dict.get_mut(b"Font")?;
                let font_dict = match font_obj {
                    lopdf::Object::Reference(r) => {
                        let r_id = *r;
                        base_doc.get_object_mut(r_id)?.as_dict_mut()?
                    }
                    lopdf::Object::Dictionary(ref mut d) => d,
                    _ => continue,
                };
                for (k, v) in fonts.iter() {
                    font_dict.set(k.clone(), v.clone());
                }
            }

            // 4. 将 overlay Contents 流追加到 base 页面的 /Contents
            if let Some(overlay_c) = overlay_contents {
                let base_page_dict = base_doc.get_object_mut(*base_pid)?.as_dict_mut()?;
                let existing_c = base_page_dict.get(b"Contents").cloned().ok();
                let new_c = match existing_c {
                    Some(lopdf::Object::Array(mut arr)) => {
                        match overlay_c {
                            lopdf::Object::Array(o_arr) => arr.extend(o_arr),
                            other => arr.push(other),
                        }
                        lopdf::Object::Array(arr)
                    }
                    Some(single) => {
                        let mut arr = vec![single];
                        match overlay_c {
                            lopdf::Object::Array(o_arr) => arr.extend(o_arr),
                            other => arr.push(other),
                        }
                        lopdf::Object::Array(arr)
                    }
                    None => overlay_c,
                };
                base_page_dict.set("Contents", new_c);
            }
        }
    }

    let mut out = Vec::new();
    base_doc.save_to(&mut out)?;
    Ok(out)
}

/// 将页眉页脚覆盖层合并到基础 PDF 上（纯 Rust 内核优先，不依赖外部 qpdf 进程）
fn apply_overlay_if_needed(
    base_pdf_bytes: &[u8],
    page_count: usize,
    paper_w_mm: f64,
    paper_h_mm: f64,
    args: &LongSliceArgs,
    page_start: u32,
    total_pages: u32,
) -> Result<Vec<u8>> {
    let (header, footer, extra) = build_overlay_configs(args, args.margin_mm);
    if header.is_none() && footer.is_none() && extra.is_empty() {
        return Ok(base_pdf_bytes.to_vec());
    }

    let paper_w_pt = (paper_w_mm * 72.0 / 25.4) as f32;
    let paper_h_pt = (paper_h_mm * 72.0 / 25.4) as f32;

    let page_sizes = vec![
        PageSize {
            width_pt: paper_w_pt,
            height_pt: paper_h_pt,
            raw_width_pt: paper_w_pt,
            raw_height_pt: paper_h_pt,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        };
        page_count
    ];

    let (overlay_bytes, _warnings) = build_overlay_pdf(
        header.as_ref(),
        footer.as_ref(),
        &extra,
        &page_sizes,
        page_start,
        total_pages,
    )?;

    let native_error = match apply_overlay_in_memory(base_pdf_bytes, &overlay_bytes) {
        Ok(merged) => return Ok(merged),
        Err(error) => error,
    };

    let qpdf_tool = crate::external::QpdfTool;
    if let Ok(bin) = qpdf_tool.binary_path() {
        let base_temp = temp_named_path("docsy_base_slice", "pdf");
        let overlay_temp = temp_named_path("docsy_overlay_slice", "pdf");
        let output_temp = temp_named_path("docsy_merged_slice", "pdf");

        fs::write(&base_temp, base_pdf_bytes)?;
        fs::write(&overlay_temp, &overlay_bytes)?;

        let mut cmd = crate::external::hidden_command(&bin);
        cmd.arg(&base_temp)
            .arg("--overlay")
            .arg(&overlay_temp)
            .arg("--")
            .arg(&output_temp)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let status = cmd.status();
        let _ = fs::remove_file(&base_temp);
        let _ = fs::remove_file(&overlay_temp);

        match status {
            Ok(status) if status.success() && output_temp.exists() => {
                let res = fs::read(&output_temp);
                let _ = fs::remove_file(&output_temp);
                return Ok(res?);
            }
            Ok(_) => {
                let _ = fs::remove_file(&output_temp);
            }
            Err(error) => return Err(error).context("调用 qpdf 合并页眉页脚失败"),
        }
    }

    Err(native_error).context("页眉页脚覆盖层合并失败，qpdf 回退也不可用")
}

/// 采用纯 Rust 图像管线合成 A4 预览图（零外部 Poppler / pdftoppm 依赖）
fn render_preview_image(
    source: &DynamicImage,
    slice: &SliceInfo,
    paper_w_mm: f64,
    paper_h_mm: f64,
    content_w_mm: f64,
    args: &LongSliceArgs,
) -> Result<String> {
    let dpi = 100.0;
    let canvas_w = ((paper_w_mm * dpi) / 25.4).round() as u32;
    let canvas_h = ((paper_h_mm * dpi) / 25.4).round() as u32;

    let mut canvas = image::RgbaImage::from_pixel(
        canvas_w,
        canvas_h,
        image::Rgba([255, 255, 255, 255]),
    );

    let img_w = source.width();
    let cropped = source.crop_imm(0, slice.y_start_px, img_w, slice.height_px);

    let content_w_px = ((content_w_mm * dpi) / 25.4).round().max(1.0) as u32;
    let content_h_px = ((slice.height_mm * dpi) / 25.4).round().max(1.0) as u32;

    let resized = cropped.resize_exact(
        content_w_px,
        content_h_px,
        image::imageops::FilterType::Triangle,
    );

    let margin_px = ((args.margin_mm.max(0.0) * dpi) / 25.4).round() as i64;
    let header_reserve_px = if args.header_enabled {
        ((8.0 * dpi) / 25.4).round() as i64
    } else {
        0
    };

    let content_x = match args.alignment.as_str() {
        "left" => margin_px,
        "right" => canvas_w as i64 - margin_px - content_w_px as i64,
        _ => (canvas_w as i64 - content_w_px as i64) / 2,
    };

    let content_y = margin_px + header_reserve_px;

    image::imageops::overlay(&mut canvas, &resized, content_x, content_y);

    let mut buf = std::io::Cursor::new(Vec::new());
    canvas.write_to(&mut buf, image::ImageFormat::Png)?;
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        buf.into_inner(),
    );
    Ok(format!("data:image/png;base64,{}", b64))
}

fn render_full_preview_page(
    input_path: &Path,
    planned: &SourceSlicePlan,
    slice: &SliceInfo,
    paper_w_mm: f64,
    paper_h_mm: f64,
    content_w_mm: f64,
    args: &LongSliceArgs,
    total_pages: usize,
) -> Result<String> {
    let single_source = SourceSlicePlan {
        source_page_index: planned.source_page_index,
        width: planned.width,
        height: planned.height,
        slices: vec![slice.clone()],
    };
    let base_bytes = render_base_slices_pdf(
        input_path,
        &[single_source],
        paper_w_mm,
        paper_h_mm,
        content_w_mm,
        args,
    )?;
    let final_bytes = apply_overlay_if_needed(
        &base_bytes,
        1,
        paper_w_mm,
        paper_h_mm,
        args,
        slice.page_index as u32,
        total_pages as u32,
    )?;
    let temp_pdf = temp_named_path("docsy_preview_page", "pdf");
    fs::write(&temp_pdf, final_bytes)?;
    let png_path = super::preview::render_pdf_page_to_png(&temp_pdf, 1, 100);
    let _ = fs::remove_file(&temp_pdf);
    let png_path = png_path.context("渲染完整 A4 预览失败")?;
    let png_bytes = fs::read(&png_path)?;
    let _ = fs::remove_file(png_path);
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png_bytes);
    Ok(format!("data:image/png;base64,{}", b64))
}

/// 生成与最终导出一致的 A4 预览
pub fn preview_long_slice(
    args: &LongSliceArgs,
    preview_page: usize,
) -> Result<LongSlicePreviewResult> {
    let path = Path::new(&args.input_path);
    if !path.exists() {
        bail!("输入文件不存在: {}", args.input_path);
    }

    let plan = build_slice_plan(path, args)?;
    let total_pages = plan.slices.len();
    let target_page_idx = preview_page.clamp(1, total_pages.max(1));
    let slice = plan
        .slices
        .get(target_page_idx - 1)
        .context("未能生成当前页面的切片预览")?;
    let planned = plan
        .sources
        .iter()
        .find(|source| source.source_page_index == slice.source_page_index)
        .context("切片预览缺少来源页面")?;
    let source_page_index = planned.source_page_index;
    let source_width = planned.width;
    let source_height = planned.height;
    let (header, footer, extra) = build_overlay_configs(args, args.margin_mm);
    let overlay_requested = header.is_some() || footer.is_some() || !extra.is_empty();
    let mut preview_data_url = None;

    if !overlay_requested {
        let source = load_source_page_as_image(path, source_page_index)?;
        if let Ok(data_url) = render_preview_image(
            &source,
            slice,
            plan.paper_w_mm,
            plan.paper_h_mm,
            plan.content_w_mm,
            args,
        ) {
            preview_data_url = Some(data_url);
        }
    }
    if preview_data_url.is_none() {
        preview_data_url = Some(render_full_preview_page(
            path,
            planned,
            slice,
            plan.paper_w_mm,
            plan.paper_h_mm,
            plan.content_w_mm,
            args,
            total_pages,
        )?);
    }

    Ok(LongSlicePreviewResult {
        source_width,
        source_height,
        source_page_index,
        source_page_count: plan.sources.len(),
        is_pdf: is_pdf_path(path),
        total_pages,
        page_width_mm: plan.paper_w_mm,
        page_height_mm: plan.paper_h_mm,
        content_width_mm: plan.content_w_mm,
        slices: plan.slices,
        preview_data_url,
        preview_page: target_page_idx,
    })
}

/// 执行切片并输出标准 A4 多页 PDF
pub fn process_long_slice(args: &LongSliceArgs) -> Result<LongSliceResult> {
    crate::operations::check_current_cancelled()?;

    let input_path = Path::new(&args.input_path);
    if !input_path.exists() {
        bail!("输入文件不存在: {}", args.input_path);
    }

    let plan = build_slice_plan(input_path, args)?;
    if plan.slices.is_empty() {
        bail!("未能对输入文件划分出切片页面");
    }
    let total_pages = plan.slices.len();
    let first_source = plan.sources.first().context("输入文件没有可处理页面")?;
    let source_width = first_source.width;
    let source_height = first_source.height;
    let source_page_count = plan.sources.len();

    let parent = match &args.output_dir {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => input_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    let base_name = args.output_filename.as_deref().unwrap_or_else(|| {
        input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
    });

    let target_stem = format!("{}_A4", safe_file_stem(base_name));
    let base_bytes = render_base_slices_pdf(
        input_path,
        &plan.sources,
        plan.paper_w_mm,
        plan.paper_h_mm,
        plan.content_w_mm,
        args,
    )?;
    let final_bytes = apply_overlay_if_needed(
        &base_bytes,
        total_pages,
        plan.paper_w_mm,
        plan.paper_h_mm,
        args,
        1,
        total_pages as u32,
    )?;

    let temp_output = temp_named_path("docsy_long_slice", "pdf");
    fs::write(&temp_output, final_bytes).context("写入临时 PDF 失败")?;

    let final_output = unique_output_path(&parent, &target_stem, "pdf");
    fs::rename(&temp_output, &final_output).or_else(|_| {
        fs::copy(&temp_output, &final_output)?;
        let _ = fs::remove_file(&temp_output);
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(LongSliceResult {
        output_path: final_output.to_string_lossy().to_string(),
        total_pages,
        source_width,
        source_height,
        source_page_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_slices_on_tall_image() {
        let (paper_w, paper_h, content_w, slices) = calculate_slices(
            1080,
            7500,
            None,
            &LongSliceArgs {
                input_path: "mock.png".into(),
                output_dir: None,
                output_filename: None,
                orientation: "portrait".into(),
                margin_mm: 5.0,
                content_width_mode: "phone".into(),
                custom_width_percent: 55.0,
                alignment: "center".into(),
                smart_cut: false,
                overlap_mm: 0.0,
                header_enabled: false,
                header_title: None,
                header_date: false,
                header_date_prefix: false,
                custom_date: None,
                footer_enabled: false,
                footer_page_number_style: "page_of_total".into(),
            },
            1,
            1,
        );

        assert_eq!(paper_w, 210.0);
        assert_eq!(paper_h, 297.0);
        assert!(content_w > 50.0 && content_w < 200.0);
        assert!(slices.len() >= 3);
        assert_eq!(slices.first().unwrap().y_start_px, 0);
        assert_eq!(slices.last().unwrap().y_end_px, 7500);
    }

    #[test]
    fn test_process_long_slice_creates_valid_pdf() {
        let dir = crate::util::fs::temp_named_path("docsy_test_slice", "dir");
        fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();

        let img_path = dir.join("long_test.png");
        let mut img = image::RgbImage::from_pixel(400, 2400, image::Rgb([255, 255, 255]));
        for y in 200..250 {
            for x in 50..350 {
                img.put_pixel(x, y, image::Rgb([30, 30, 30]));
            }
        }
        for y in 1200..1250 {
            for x in 50..350 {
                img.put_pixel(x, y, image::Rgb([30, 30, 30]));
            }
        }
        img.save(&img_path).unwrap();

        let args = LongSliceArgs {
            input_path: img_path.to_string_lossy().to_string(),
            output_dir: Some(dir.to_string_lossy().to_string()),
            output_filename: Some("test_sliced".to_string()),
            orientation: "portrait".into(),
            margin_mm: 5.0,
            content_width_mode: "fit".into(),
            custom_width_percent: 100.0,
            alignment: "center".into(),
            smart_cut: true,
            overlap_mm: 2.0,
            header_enabled: true,
            header_title: Some("https://example.com/long-page".into()),
            header_date: true,
            header_date_prefix: true,
            custom_date: Some("2026-11-02".into()),
            footer_enabled: true,
            footer_page_number_style: "page_of_total".into(),
        };

        let result = process_long_slice(&args).expect("处理长图切片失败");
        assert!(result.total_pages >= 2);
        assert!(Path::new(&result.output_path).exists());
        let meta = fs::metadata(&result.output_path).unwrap();
        assert!(meta.len() > 1000);
        // 验证没有字体未子集化膨胀（应远小于 1MB，而不是 23MB）
        assert!(meta.len() < 1_000_000);

        // 验证纯 Rust 内存合并成功将字体与文字流注入到了输出 PDF
        let out_doc = lopdf::Document::load(&result.output_path).unwrap();
        let pages: Vec<lopdf::ObjectId> = out_doc.get_pages().into_values().collect();
        assert_eq!(pages.len(), result.total_pages);
        let p1 = out_doc.get_object(pages[0]).unwrap().as_dict().unwrap();
        assert!(p1.has(b"Resources"));
        let p1_res = deref_dict(&out_doc, p1.get(b"Resources").unwrap()).unwrap();
        assert!(p1_res.has(b"Font")); // 包含纯 Rust 注入的页眉页脚字体

        let preview = preview_long_slice(&args, 1).expect("预览生成失败");
        assert_eq!(preview.total_pages, result.total_pages);
        assert_eq!(preview.source_page_count, 1);
        let data_url = preview.preview_data_url.as_ref().unwrap();
        let encoded = data_url.strip_prefix("data:image/png;base64,").unwrap();
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .unwrap();
        let preview_image = image::load_from_memory(&bytes).unwrap().to_rgba8();
        let (width, height) = preview_image.dimensions();
        let ink_in_region = |start_y, end_y| {
            (start_y..end_y)
                .flat_map(|y| (0..width).map(move |x| (x, y)))
                .filter(|(x, y)| {
                    let pixel = preview_image.get_pixel(*x, *y).0;
                    pixel[0] < 200 || pixel[1] < 200 || pixel[2] < 200
                })
                .count()
        };
        assert!(ink_in_region(0, 35) > 0, "预览缺少页眉文字");
        assert!(
            ink_in_region(height.saturating_sub(35), height) > 0,
            "预览缺少页脚页码"
        );
    }

    #[test]
    fn test_multi_page_pdf_processes_every_source_page() {
        let dir = crate::util::fs::temp_named_path("docsy_test_multipage_slice", "dir");
        fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let input = dir.join("two_pages.pdf");
        let mut pdf = PdfDocument::new("Two long pages");
        for _ in 0..2 {
            pdf.pages.push(PdfPage::new(Mm(80.0), Mm(400.0), Vec::new()));
        }
        fs::write(&input, pdf.save(&PdfSaveOptions::default(), &mut Vec::new())).unwrap();
        let args = LongSliceArgs {
            input_path: input.to_string_lossy().to_string(),
            output_dir: Some(dir.to_string_lossy().to_string()),
            output_filename: Some("two_pages_sliced".to_string()),
            orientation: "portrait".into(),
            margin_mm: 5.0,
            content_width_mode: "fit".into(),
            custom_width_percent: 100.0,
            alignment: "center".into(),
            smart_cut: false,
            overlap_mm: 0.0,
            header_enabled: false,
            header_title: None,
            header_date: false,
            header_date_prefix: false,
            custom_date: None,
            footer_enabled: false,
            footer_page_number_style: "page_of_total".into(),
        };

        let result = process_long_slice(&args).expect("多页 PDF 切片失败");
        assert_eq!(result.source_page_count, 2);
        assert!(result.total_pages >= 2);
        let output = lopdf::Document::load(&result.output_path).unwrap();
        assert_eq!(output.get_pages().len(), result.total_pages);

        let preview = preview_long_slice(&args, result.total_pages).unwrap();
        assert_eq!(preview.source_page_count, 2);
        assert_eq!(preview.source_page_index, 2);
        let source_pages = preview
            .slices
            .iter()
            .map(|slice| slice.source_page_index)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(source_pages, [1, 2].into_iter().collect());
        assert!(preview
            .slices
            .windows(2)
            .all(|pair| pair[0].page_index + 1 == pair[1].page_index));
    }
}
