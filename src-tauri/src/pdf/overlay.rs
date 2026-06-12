use anyhow::{Context, Result};
use printpdf::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

#[derive(Debug, Deserialize)]
struct OverlaySpec {
    text: String,
    #[serde(default = "default_position")]
    position: String,
}

fn default_position() -> String {
    "center".to_string()
}

#[derive(Debug, Deserialize)]
struct OverlayArgs {
    input: String,
    output: String,
    #[serde(default)]
    header: Option<OverlaySpec>,
    #[serde(default)]
    footer: Option<OverlaySpec>,
}

#[derive(Debug, Clone)]
struct PageSize {
    width_pt: f32,
    height_pt: f32,
}

pub fn overlay_text(args: &serde_json::Value) -> Result<String> {
    let args: OverlayArgs =
        serde_json::from_value(args.clone()).context("解析叠加参数失败")?;

    let page_infos = get_page_infos(&args.input)?;
    let total = page_infos.len() as u32;

    let header = args.header.as_ref();
    let footer = args.footer.as_ref();

    if header.is_none() && footer.is_none() {
        anyhow::bail!("未指定页眉或页脚");
    }

    let overlay_pdf = build_overlay_pdf(header, footer, &page_infos, total)?;
    let overlay_path = temp_overlay_path(&args.input);
    std::fs::write(&overlay_path, &overlay_pdf)?;

    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let status = std::process::Command::new(&bin)
        .arg(&args.input)
        .arg("--overlay")
        .arg(&overlay_path)
        .arg("--")
        .arg(&args.output)
        .status()
        .context("执行 qpdf overlay 失败")?;

    let _ = std::fs::remove_file(&overlay_path);

    if !status.success() {
        anyhow::bail!("qpdf overlay 失败");
    }

    Ok(args.output)
}

pub fn batch_overlay(args: &serde_json::Value) -> Result<Vec<String>> {
    let inputs = args
        .get("inputs")
        .and_then(|v| v.as_array())
        .context("缺少 inputs 数组")?;

    let mut results = Vec::new();
    for item in inputs {
        let result = overlay_text(item)?;
        results.push(result);
    }
    Ok(results)
}

// ── Page info extraction via qpdf --json ──────────────────────────────

fn get_page_infos(input: &str) -> Result<Vec<PageSize>> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let output = std::process::Command::new(&bin)
        .arg("--json")
        .arg(input)
        .output()
        .context("执行 qpdf --json 失败")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("qpdf --json 失败: {}", stderr);
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("解析 qpdf JSON 失败")?;

    parse_page_sizes(&json)
}

fn parse_page_sizes(json: &serde_json::Value) -> Result<Vec<PageSize>> {
    let pages = json
        .get("pages")
        .and_then(|v| v.as_array())
        .context("qpdf JSON 中无 pages 数组")?;

    let objects = json.get("qpdf").and_then(|v| v.as_array());

    let mut sizes = Vec::new();

    for page in pages {
        let obj_ref = page.get("object").and_then(|v| v.as_str());
        let size = obj_ref
            .and_then(|r| resolve_page_size(objects, r))
            .unwrap_or(PageSize {
                width_pt: 595.28,
                height_pt: 841.89,
            });
        sizes.push(size);
    }

    if sizes.is_empty() {
        anyhow::bail!("PDF 无页面");
    }

    Ok(sizes)
}

fn resolve_page_size(objects: Option<&Vec<serde_json::Value>>, obj_ref: &str) -> Option<PageSize> {
    let objects = objects?;

    for obj in objects {
        let page_obj = obj.get(obj_ref)?;
        let mediabox = page_obj
            .get("value")
            .and_then(|v| v.get("MediaBox"))
            .or_else(|| page_obj.get("value").and_then(|v| v.get("/MediaBox")))?;

        let arr = mediabox.as_array()?;
        if arr.len() >= 4 {
            let x0 = arr[0].as_f64()? as f32;
            let y0 = arr[1].as_f64()? as f32;
            let x1 = arr[2].as_f64()? as f32;
            let y1 = arr[3].as_f64()? as f32;
            return Some(PageSize {
                width_pt: (x1 - x0).abs(),
                height_pt: (y1 - y0).abs(),
            });
        }
    }
    None
}

// ── Overlay PDF generation ────────────────────────────────────────────

fn build_overlay_pdf(
    header: Option<&OverlaySpec>,
    footer: Option<&OverlaySpec>,
    pages: &[PageSize],
    total: u32,
) -> Result<Vec<u8>> {
    let mut doc = PdfDocument::new("Docsy Overlay");
    let mut pdf_pages = Vec::new();

    for (i, size) in pages.iter().enumerate() {
        let page_num = (i + 1) as u32;
        let mut ops = Vec::new();

        if let Some(h) = header {
            let text = expand_placeholders(&h.text, page_num, total);
            let font = select_font(&text, &mut doc);
            let y = size.height_pt - 28.0;
            let x = compute_x(&h.position, &text, font.clone(), 9.0, size.width_pt);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetTextCursor { pos: Point { x: Pt(x), y: Pt(y) } });
            ops.push(Op::SetFont { font, size: Pt(9.0) });
            ops.push(Op::SetLineHeight { lh: Pt(9.0) });
            ops.push(Op::SetFillColor { col: Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }) });
            ops.push(Op::ShowText { items: vec![TextItem::Text(text)] });
            ops.push(Op::EndTextSection);
        }

        if let Some(f) = footer {
            let text = expand_placeholders(&f.text, page_num, total);
            let font = select_font(&text, &mut doc);
            let y = 20.0;
            let x = compute_x(&f.position, &text, font.clone(), 9.0, size.width_pt);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetTextCursor { pos: Point { x: Pt(x), y: Pt(y) } });
            ops.push(Op::SetFont { font, size: Pt(9.0) });
            ops.push(Op::SetLineHeight { lh: Pt(9.0) });
            ops.push(Op::SetFillColor { col: Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }) });
            ops.push(Op::ShowText { items: vec![TextItem::Text(text)] });
            ops.push(Op::EndTextSection);
        }

        let w = Mm(size.width_pt * 0.352_778);
        let h = Mm(size.height_pt * 0.352_778);
        pdf_pages.push(PdfPage::new(w, h, ops));
    }

    let bytes = doc
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new());

    Ok(bytes)
}

// ── Placeholder expansion ─────────────────────────────────────────────

fn expand_placeholders(template: &str, page: u32, total: u32) -> String {
    template
        .replace("{page}", &page.to_string())
        .replace("{total}", &total.to_string())
        .replace("{range}", &format!("{}/{}", page, total))
}

// ── CJK font selection ───────────────────────────────────────────────

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
    let candidates: Vec<&str> = if cfg!(target_os = "macos") {
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

    for path in &candidates {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

fn select_font(text: &str, doc: &mut PdfDocument) -> PdfFontHandle {
    if has_cjk(text) {
        if let Some(font_path) = find_cjk_font_path() {
            if let Ok(bytes) = std::fs::read(&font_path) {
                if let Some(parsed) = ParsedFont::from_bytes(&bytes, 0, &mut Vec::new()) {
                    let id = doc.add_font(&parsed);
                    return PdfFontHandle::External(id);
                }
            }
        }
    }
    PdfFontHandle::Builtin(BuiltinFont::Helvetica)
}

// ── X position calculation ────────────────────────────────────────────

fn compute_x(position: &str, text: &str, font: PdfFontHandle, font_size: f32, page_width: f32) -> f32 {
    let char_width = estimate_char_width(&font, font_size);
    let text_width = text.chars().map(|c| char_width(c)).sum::<f32>();

    match position {
        "left" => 36.0,
        "right" => (page_width - 36.0 - text_width).max(36.0),
        _ => ((page_width - text_width) / 2.0).max(0.0),
    }
}

fn estimate_char_width(font: &PdfFontHandle, font_size: f32) -> Box<dyn Fn(char) -> f32> {
    let is_builtin = matches!(font, PdfFontHandle::Builtin(_));

    Box::new(move |c: char| {
        let base = if is_builtin {
            match c {
                ' ' => 0.278,
                '0'..='9' => 0.556,
                'A'..='Z' => 0.667,
                'a'..='z' => 0.500,
                '.' | ',' | ':' | ';' => 0.278,
                '-' | '_' => 0.333,
                '(' | ')' | '[' | ']' | '{' | '}' => 0.333,
                '/' | '\\' => 0.278,
                '\'' | '"' => 0.278,
                '!' | '?' => 0.556,
                '@' | '#' | '$' | '%' | '&' | '*' => 0.556,
                '+' | '=' | '<' | '>' | '~' | '^' => 0.556,
                _ => {
                    let cp = c as u32;
                    if (0x4E00..=0x9FFF).contains(&cp)
                        || (0x3400..=0x4DBF).contains(&cp)
                        || (0x20000..=0x2A6DF).contains(&cp)
                    {
                        1.0
                    } else if (0x3040..=0x30FF).contains(&cp)
                        || (0xAC00..=0xD7AF).contains(&cp)
                    {
                        0.85
                    } else {
                        0.556
                    }
                }
            }
        } else {
            let cp = c as u32;
            if (0x4E00..=0x9FFF).contains(&cp)
                || (0x3400..=0x4DBF).contains(&cp)
                || (0x20000..=0x2A6DF).contains(&cp)
            {
                1.0
            } else if (0x3040..=0x30FF).contains(&cp)
                || (0xAC00..=0xD7AF).contains(&cp)
            {
                0.85
            } else {
                0.5
            }
        };
        base * font_size
    })
}

// ── Temp file path ────────────────────────────────────────────────────

fn temp_overlay_path(input: &str) -> PathBuf {
    let p = Path::new(input);
    let parent = p.parent().unwrap_or(Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("overlay");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    parent.join(format!("{}_overlay_{}.pdf", stem, ts))
}
