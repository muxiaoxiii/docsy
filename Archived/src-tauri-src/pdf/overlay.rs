//! PDF 文字 overlay 引擎。
//!
//! 策略：用 printpdf 生成一个透明的文字层 PDF，再用 qpdf --overlay
//! 把文字层叠到原始 PDF 上。这样不修改原始 PDF 的内容流，
//! 输出文件可重复生成，原始文件保持不变。

use std::fs;
use std::path::{Path, PathBuf};

use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Pt, TextItem,
};
use serde::{Deserialize, Serialize};

use super::qpdf;

// ── 数据类型 ──────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayArgs {
    /// 原始 PDF 路径
    pub input_path: String,
    /// 输出 PDF 路径
    pub output_path: String,
    /// 当前 PDF 在最终合并文件中的起始页码（1-indexed）
    #[serde(default = "default_page_start")]
    pub page_start: usize,
    /// 最终合并文件的总页数；为空时退回当前 PDF 页数
    #[serde(default)]
    pub total_pages: Option<usize>,
    /// 页眉配置（可选）
    pub header: Option<OverlayTextConfig>,
    /// 页脚配置（可选）
    pub footer: Option<OverlayTextConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTextConfig {
    /// 文本内容。支持占位符：
    /// - `{page}` 当前页码
    /// - `{total}` 总页数
    /// - `{range}` 当前子文档页数范围（如 "1-12"）
    pub text: String,
    /// 字体大小（pt），默认 10
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    /// 距页面顶部/底部的边距（mm），默认 10
    #[serde(default = "default_margin")]
    pub margin_mm: f64,
    /// 水平对齐：left / center / right，默认 center
    #[serde(default = "default_align")]
    pub align: String,
    /// 左右额外偏移（mm），默认 0。正值向右偏移。
    #[serde(default)]
    pub offset_x_mm: f64,
}

fn default_font_size() -> f64 {
    10.0
}
fn default_margin() -> f64 {
    10.0
}
fn default_align() -> String {
    "center".to_string()
}
fn default_page_start() -> usize {
    1
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayResult {
    pub input_path: String,
    pub output_path: String,
    pub pages: usize,
}

// ── 公共 API ──────────────────────────────────────────

pub fn overlay_text(args: &OverlayArgs) -> Result<OverlayResult, String> {
    let input = Path::new(&args.input_path);
    if !input.exists() {
        return Err(format!("原始 PDF 不存在：{}", input.display()));
    }
    let output = Path::new(&args.output_path);
    if same_path(input, output) {
        return Err("页眉页脚输出路径不能和原始 PDF 相同，请另存为副本".to_string());
    }

    // 读取原始 PDF 获取页数
    let page_count = count_pdf_pages(input)?;
    let page_check = check_pdf_pages(&args.input_path)?;
    if page_check.pages.len() != page_count {
        return Err(format!(
            "PDF 页面尺寸读取不完整：识别到 {} 页尺寸，但文件共有 {} 页",
            page_check.pages.len(),
            page_count
        ));
    }
    let page_start = args.page_start.max(1);
    let total_pages = args.total_pages.unwrap_or(page_count);
    let last_global_page = page_start + page_count.saturating_sub(1);
    if total_pages < last_global_page {
        return Err(format!(
            "全局总页数 {total_pages} 小于当前 PDF 的结束页码 {last_global_page}"
        ));
    }

    // 如果没有页眉也没有页脚，直接复制
    if args.header.is_none() && args.footer.is_none() {
        fs::copy(input, &args.output_path).map_err(|e| format!("复制 PDF 失败：{e}"))?;
        return Ok(OverlayResult {
            input_path: args.input_path.clone(),
            output_path: args.output_path.clone(),
            pages: page_count,
        });
    }

    // 确保输出目录存在
    if let Some(parent) = Path::new(&args.output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败：{e}"))?;
    }

    // 生成透明文字层 PDF
    let overlay_path = generate_overlay_pdf(
        &page_check.pages,
        page_start,
        total_pages,
        args.header.as_ref(),
        args.footer.as_ref(),
    )?;

    // 用 qpdf 把文字层叠到原始 PDF 上
    let result = qpdf::overlay(input, &overlay_path, Path::new(&args.output_path))
        .map_err(|e| e.to_string())?;

    // 清理临时文件
    let _ = fs::remove_file(&overlay_path);

    Ok(OverlayResult {
        input_path: args.input_path.clone(),
        output_path: result.output_path,
        pages: page_count,
    })
}

/// 获取 PDF 页数（轻量查询，不修改文件）
pub fn get_page_count(pdf_path: &str) -> Result<usize, String> {
    let path = Path::new(pdf_path);
    if !path.exists() {
        return Err(format!("PDF 不存在：{pdf_path}"));
    }
    count_pdf_pages(path)
}

// ── 页面尺寸检查 ──────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub page: usize,
    pub width_pt: f64,
    pub height_pt: f64,
    pub orientation: String, // "portrait" | "landscape"
    pub is_a4: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageCheckResult {
    pub path: String,
    pub pages: Vec<PageInfo>,
    pub has_landscape: bool,
    pub has_non_a4: bool,
    pub warnings: Vec<String>,
}

/// 检查 PDF 每页的尺寸和方向。
///
/// A4 尺寸：595.28pt × 841.89pt（210mm × 297mm），容差 ±5pt。
pub fn check_pdf_pages(pdf_path: &str) -> Result<PageCheckResult, String> {
    let path = Path::new(pdf_path);
    if !path.exists() {
        return Err(format!("PDF 不存在：{pdf_path}"));
    }

    let program = super::qpdf::resolve_qpdf_command();
    let mut cmd = std::process::Command::new(&program);
    cmd.arg("--json").arg(path);

    let out = cmd.output().map_err(|e| format!("调用 qpdf 失败：{e}"))?;
    if !out.status.success() {
        return Err(format!(
            "读取 PDF 页面信息失败：{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("解析 qpdf JSON 失败：{e}"))?;
    let pages = parse_pages_from_qpdf_json(&value)?;
    let has_landscape = pages.iter().any(|page| page.orientation == "landscape");
    let has_non_a4 = pages.iter().any(|page| !page.is_a4);
    let mut warnings = Vec::new();

    if has_landscape {
        warnings.push("包含横向页面，可能需要检查是否需要旋转".to_string());
    }
    if has_non_a4 {
        warnings.push("包含非 A4 尺寸页面".to_string());
    }

    Ok(PageCheckResult {
        path: pdf_path.to_string(),
        pages,
        has_landscape,
        has_non_a4,
        warnings,
    })
}

// ── 内部实现 ──────────────────────────────────────────

/// 读取 PDF 页数。用 qpdf --show-npages 获取。
fn count_pdf_pages(pdf_path: &Path) -> Result<usize, String> {
    let program = super::qpdf::resolve_qpdf_command();
    let mut cmd = std::process::Command::new(&program);
    cmd.arg("--show-npages").arg(pdf_path);

    let out = cmd.output().map_err(|e| format!("调用 qpdf 失败：{e}"))?;

    if !out.status.success() {
        return Err(format!(
            "获取 PDF 页数失败：{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    stdout
        .parse::<usize>()
        .map_err(|_| format!("无法解析页数：{stdout}"))
}

fn parse_pages_from_qpdf_json(value: &serde_json::Value) -> Result<Vec<PageInfo>, String> {
    let page_entries = value
        .get("pages")
        .and_then(|pages| pages.as_array())
        .ok_or_else(|| "qpdf JSON 中缺少 pages".to_string())?;
    let objects = value
        .get("qpdf")
        .and_then(|qpdf| qpdf.as_array())
        .and_then(|qpdf| qpdf.get(1))
        .and_then(|objects| objects.as_object())
        .ok_or_else(|| "qpdf JSON 中缺少页面对象".to_string())?;

    let mut pages = Vec::with_capacity(page_entries.len());
    for (index, entry) in page_entries.iter().enumerate() {
        let page_num = entry
            .get("pageposfrom1")
            .and_then(|page| page.as_u64())
            .map(|page| page as usize)
            .unwrap_or(index + 1);
        let object_ref = entry
            .get("object")
            .and_then(|object| object.as_str())
            .ok_or_else(|| format!("第 {page_num} 页缺少对象引用"))?;
        let object_key = format!("obj:{object_ref}");
        let object_value = objects
            .get(&object_key)
            .and_then(|object| object.get("value"))
            .ok_or_else(|| format!("第 {page_num} 页缺少页面对象"))?;
        let media_box = resolve_page_box(object_value, objects, page_num)?;
        if media_box.len() != 4 {
            return Err(format!("第 {page_num} 页 MediaBox 格式不正确"));
        }
        let x0 = json_number(&media_box[0], page_num, "MediaBox x0")?;
        let y0 = json_number(&media_box[1], page_num, "MediaBox y0")?;
        let x1 = json_number(&media_box[2], page_num, "MediaBox x1")?;
        let y1 = json_number(&media_box[3], page_num, "MediaBox y1")?;
        let width_pt = (x1 - x0).abs();
        let height_pt = (y1 - y0).abs();
        if width_pt <= 0.0 || height_pt <= 0.0 {
            return Err(format!("第 {page_num} 页尺寸无效"));
        }
        let orientation = if width_pt > height_pt {
            "landscape"
        } else {
            "portrait"
        };
        let is_a4 = is_a4_size(width_pt, height_pt);
        pages.push(PageInfo {
            page: page_num,
            width_pt,
            height_pt,
            orientation: orientation.to_string(),
            is_a4,
        });
    }

    Ok(pages)
}

fn resolve_page_box<'a>(
    object_value: &'a serde_json::Value,
    objects: &'a serde_json::Map<String, serde_json::Value>,
    page_num: usize,
) -> Result<&'a Vec<serde_json::Value>, String> {
    let mut current = object_value;
    for _ in 0..16 {
        if let Some(page_box) = current
            .get("/MediaBox")
            .or_else(|| current.get("/CropBox"))
            .and_then(|box_value| box_value.as_array())
        {
            return Ok(page_box);
        }
        let Some(parent_ref) = current.get("/Parent").and_then(|parent| parent.as_str()) else {
            break;
        };
        let parent_key = format!("obj:{parent_ref}");
        current = objects
            .get(&parent_key)
            .and_then(|object| object.get("value"))
            .ok_or_else(|| format!("第 {page_num} 页父对象缺失"))?;
    }
    Err(format!("第 {page_num} 页缺少 MediaBox/CropBox"))
}

fn json_number(value: &serde_json::Value, page: usize, name: &str) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("第 {page} 页 {name} 不是数字"))
}

fn is_a4_size(width_pt: f64, height_pt: f64) -> bool {
    let a4_w = 595.28;
    let a4_h = 841.89;
    let tol = 5.0;
    (width_pt - a4_w).abs() < tol && (height_pt - a4_h).abs() < tol
        || (width_pt - a4_h).abs() < tol && (height_pt - a4_w).abs() < tol
}

/// 用 printpdf 生成一个和原始 PDF 每页同尺寸的透明文字层 PDF。
fn generate_overlay_pdf(
    pages: &[PageInfo],
    page_start: usize,
    total_pages: usize,
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
) -> Result<PathBuf, String> {
    let overlay_path = std::env::temp_dir().join(format!(
        "docsy_overlay_{}.pdf",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    let mut doc = PdfDocument::new("overlay");

    // 尝试加载 CJK 字体；找不到则 fallback 到 Helvetica（不支持中文）
    let font_handle = match load_cjk_font(&mut doc) {
        Some(handle) => handle,
        None => PdfFontHandle::Builtin(BuiltinFont::Helvetica),
    };

    for (page_idx, page) in pages.iter().enumerate() {
        let page_w = Mm(pt_to_mm(page.width_pt) as f32);
        let page_h = Mm(pt_to_mm(page.height_pt) as f32);
        let page_width_pt = page.width_pt;
        let page_height_pt = page.height_pt;
        let mut ops: Vec<Op> = Vec::new();

        if let Some(hdr) = header {
            let text = resolve_placeholders(&hdr.text, page_idx, page_start, total_pages);
            let x_pt = compute_x_pt(
                page_width_pt,
                &hdr.align,
                hdr.offset_x_mm,
                hdr.font_size,
                &text,
            );
            // 距顶部：页面高度 - margin（PDF 坐标系从左下角开始）
            let y_pt = page_height_pt - mm_to_pt(hdr.margin_mm);
            ops.extend(text_ops(&font_handle, &text, hdr.font_size, x_pt, y_pt));
        }

        if let Some(ftr) = footer {
            let text = resolve_placeholders(&ftr.text, page_idx, page_start, total_pages);
            let x_pt = compute_x_pt(
                page_width_pt,
                &ftr.align,
                ftr.offset_x_mm,
                ftr.font_size,
                &text,
            );
            // 距底部：margin
            let y_pt = mm_to_pt(ftr.margin_mm);
            ops.extend(text_ops(&font_handle, &text, ftr.font_size, x_pt, y_pt));
        }

        doc.pages.push(PdfPage::new(page_w, page_h, ops));
    }

    let save_opts = PdfSaveOptions::default();
    let mut warnings = Vec::new();
    let bytes = doc.save(&save_opts, &mut warnings);

    fs::write(&overlay_path, &bytes).map_err(|e| format!("写入 overlay PDF 失败：{e}"))?;

    Ok(overlay_path)
}

/// 尝试加载系统 CJK 字体。优先使用中文黑体/宋体，fallback 到日文字体。
fn load_cjk_font(doc: &mut PdfDocument) -> Option<PdfFontHandle> {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Supplemental/Songti.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/Supplemental/STHeiti Medium.ttc",
            "/System/Library/Fonts/Supplemental/STHeiti Light.ttc",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\simsun.ttc",
            r"C:\Windows\Fonts\msyhbd.ttc",
        ]
    } else {
        vec![
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        ]
    };

    for path_str in candidates {
        let path = Path::new(path_str);
        if !path.exists() {
            continue;
        }
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        let mut font_warnings = Vec::new();
        if let Some(parsed) = printpdf::ParsedFont::from_bytes(&bytes, 0, &mut font_warnings) {
            let font_id = doc.add_font(&parsed);
            return Some(PdfFontHandle::External(font_id));
        }
    }

    None
}

/// 生成一组文本绘制 Op
fn text_ops(font: &PdfFontHandle, text: &str, font_size: f64, x_pt: f64, y_pt: f64) -> Vec<Op> {
    vec![
        Op::StartTextSection,
        Op::SetFont {
            font: font.clone(),
            size: Pt(font_size as f32),
        },
        Op::SetTextCursor {
            pos: printpdf::Point {
                x: Pt(x_pt as f32),
                y: Pt(y_pt as f32),
            },
        },
        Op::ShowText {
            items: vec![TextItem::Text(text.to_string())],
        },
        Op::EndTextSection,
    ]
}

/// 根据对齐方式计算文字 x 坐标（pt）
fn compute_x_pt(
    page_width_pt: f64,
    align: &str,
    offset_x_mm: f64,
    font_size: f64,
    text: &str,
) -> f64 {
    let text_width_pt = estimate_text_width_pt(text, font_size);
    let offset_pt = offset_x_mm * 72.0 / 25.4;

    match align {
        "left" => 36.0 + offset_pt,
        "right" => page_width_pt - text_width_pt - 36.0 - offset_pt,
        _ => (page_width_pt - text_width_pt) / 2.0 + offset_pt,
    }
}

/// 解析占位符
fn resolve_placeholders(
    template: &str,
    page_idx: usize,
    page_start: usize,
    total_pages: usize,
) -> String {
    let page_num = page_start + page_idx;
    template
        .replace("{page}", &page_num.to_string())
        .replace("{total}", &total_pages.to_string())
        .replace("{range}", &format!("{page_num}/{total_pages}"))
}

fn estimate_text_width_pt(text: &str, font_size: f64) -> f64 {
    text.chars()
        .map(|ch| {
            if ch.is_ascii() {
                if ch.is_ascii_whitespace() {
                    0.33
                } else {
                    0.55
                }
            } else {
                1.0
            }
        })
        .sum::<f64>()
        * font_size
}

fn mm_to_pt(mm: f64) -> f64 {
    mm * 72.0 / 25.4
}

fn pt_to_mm(pt: f64) -> f64 {
    pt * 25.4 / 72.0
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

// ── 批量 overlay ──────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOverlayArgs {
    pub items: Vec<OverlayArgs>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOverlayResult {
    pub results: Vec<OverlayResult>,
    pub failed: Vec<super::evidence::EvidenceStepMessage>,
}

pub fn batch_overlay(args: &BatchOverlayArgs) -> Result<BatchOverlayResult, String> {
    let mut results = Vec::new();
    let mut failed = Vec::new();

    for item in &args.items {
        match overlay_text(item) {
            Ok(r) => results.push(r),
            Err(e) => failed.push(super::evidence::EvidenceStepMessage {
                path: item.input_path.clone(),
                message: e,
            }),
        }
    }

    Ok(BatchOverlayResult { results, failed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_page_boxes_from_qpdf_json_including_inherited_media_box() {
        let value = json!({
            "pages": [
                { "object": "3 0 R", "pageposfrom1": 1 },
                { "object": "4 0 R", "pageposfrom1": 2 }
            ],
            "qpdf": [
                {},
                {
                    "obj:2 0 R": {
                        "value": {
                            "/Type": "/Pages",
                            "/MediaBox": [0, 0, 595.28, 841.89]
                        }
                    },
                    "obj:3 0 R": {
                        "value": {
                            "/Type": "/Page",
                            "/Parent": "2 0 R"
                        }
                    },
                    "obj:4 0 R": {
                        "value": {
                            "/Type": "/Page",
                            "/Parent": "2 0 R",
                            "/CropBox": [0, 0, 841.89, 595.28]
                        }
                    }
                }
            ]
        });

        let pages = parse_pages_from_qpdf_json(&value).expect("page boxes should parse");
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].orientation, "portrait");
        assert!(pages[0].is_a4);
        assert_eq!(pages[1].orientation, "landscape");
        assert!(pages[1].is_a4);
    }

    #[test]
    fn resolves_global_page_placeholders() {
        let text = resolve_placeholders("{page}/{total} {range}", 2, 11, 30);
        assert_eq!(text, "13/30 13/30");
    }

    #[test]
    fn estimates_cjk_width_by_chars_not_utf8_bytes() {
        let cjk = estimate_text_width_pt("证据一", 10.0);
        let ascii = estimate_text_width_pt("abc", 10.0);
        assert_eq!(cjk, 30.0);
        assert_eq!(ascii, 16.5);
        assert!(cjk < "证据一".len() as f64 * 10.0);
    }

    #[test]
    fn detects_same_paths_without_requiring_existing_output() {
        assert!(same_path(Path::new("/tmp/a.pdf"), Path::new("/tmp/a.pdf")));
        assert!(!same_path(
            Path::new("/tmp/a.pdf"),
            Path::new("/tmp/a_docsy_overlay.pdf")
        ));
    }
}
