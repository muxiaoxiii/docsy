use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff"];

const A4_WIDTH_MM: f64 = 210.0;
const A4_HEIGHT_MM: f64 = 297.0;

#[derive(Debug, Deserialize)]
pub struct AnalyzeArgs {
    pub folder: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResult {
    pub images: Vec<ImageInfo>,
    pub groups: Vec<ImageGroup>,
    pub recommended: RecommendedSettings,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImageInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
}

#[derive(Debug, Serialize)]
pub struct ImageGroup {
    pub prefix: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct RecommendedSettings {
    pub orientation: String,
    pub layout: String,
    pub dpi: u32,
}

#[derive(Debug, Deserialize)]
pub struct RunArgs {
    pub folder: String,
    pub output_format: String,
    pub layout: String,
    pub orientation: String,
    pub dpi: u32,
    pub scale_mode: String,
    #[serde(default)]
    pub margin_mm: Option<f64>,
    #[serde(default)]
    pub show_filename: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RunResult {
    pub output_path: String,
    pub pages: u32,
    pub images: u32,
}

struct LayoutGrid {
    rows: usize,
    cols: usize,
}

fn parse_layout(layout: &str, _image_count: usize) -> LayoutGrid {
    if layout.contains('x') {
        let parts: Vec<&str> = layout.split('x').collect();
        if parts.len() == 2 {
            let rows = parts[0].parse::<usize>().unwrap_or(1);
            let cols = parts[1].parse::<usize>().unwrap_or(1);
            return LayoutGrid { rows, cols };
        }
    }
    match layout {
        "1" => LayoutGrid { rows: 1, cols: 1 },
        "2" => LayoutGrid { rows: 1, cols: 2 },
        "3" => LayoutGrid { rows: 1, cols: 3 },
        "4" => LayoutGrid { rows: 2, cols: 2 },
        _ => {
            let n = layout.parse::<usize>().unwrap_or(4);
            let cols = (n as f64).sqrt().ceil() as usize;
            let rows = (n + cols - 1) / cols;
            LayoutGrid { rows, cols }
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum NatPart<'a> {
    Text(&'a str),
    Num(u64),
}

fn natural_sort_key(s: &str) -> Vec<NatPart<'_>> {
    let re = Regex::new(r"(\d+)").unwrap();
    let mut result = Vec::new();
    let mut last = 0;
    for m in re.find_iter(s) {
        if m.start() > last {
            result.push(NatPart::Text(&s[last..m.start()]));
        }
        if let Ok(n) = m.as_str().parse::<u64>() {
            result.push(NatPart::Num(n));
        }
        last = m.end();
    }
    if last < s.len() {
        result.push(NatPart::Text(&s[last..]));
    }
    result
}

fn extract_prefix(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let re = Regex::new(r"[-_]\d+$").unwrap();
    re.replace(stem, "").to_string()
}

fn scan_images(folder: &str) -> Result<Vec<ImageInfo>> {
    let mut images = Vec::new();
    let dir = std::fs::read_dir(folder)?;

    for entry in dir {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
                let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let (width, height) = match image::open(&path) {
                    Ok(img) => (img.width(), img.height()),
                    Err(_) => continue,
                };
                images.push(ImageInfo {
                    path: path.display().to_string(),
                    width,
                    height,
                    file_size,
                });
            }
        }
    }

    images.sort_by(|a, b| {
        let ka = natural_sort_key(&a.path);
        let kb = natural_sort_key(&b.path);
        ka.cmp(&kb)
    });

    Ok(images)
}

fn build_groups(images: &[ImageInfo]) -> Vec<ImageGroup> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for img in images {
        let name = Path::new(&img.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let prefix = extract_prefix(name);
        *map.entry(prefix).or_insert(0) += 1;
    }
    let mut groups: Vec<ImageGroup> = map
        .into_iter()
        .map(|(prefix, count)| ImageGroup { prefix, count })
        .collect();
    groups.sort_by(|a, b| a.prefix.cmp(&b.prefix));
    groups
}

fn recommend_settings(images: &[ImageInfo]) -> RecommendedSettings {
    if images.is_empty() {
        return RecommendedSettings {
            orientation: "portrait".into(),
            layout: "2x2".into(),
            dpi: 300,
        };
    }

    let mut widths: Vec<u32> = images.iter().map(|i| i.width).collect();
    let mut heights: Vec<u32> = images.iter().map(|i| i.height).collect();
    widths.sort_unstable();
    heights.sort_unstable();

    let median_w = widths[widths.len() / 2] as f64;
    let median_h = heights[heights.len() / 2] as f64;

    let orientation = if median_w > median_h * 1.2 {
        "landscape"
    } else {
        "portrait"
    };

    let avg_pixels = median_w * median_h;
    let layout = if avg_pixels > 2_000_000.0 {
        "1"
    } else if avg_pixels > 500_000.0 {
        "2x2"
    } else {
        "2x3"
    };

    RecommendedSettings {
        orientation: orientation.into(),
        layout: layout.into(),
        dpi: 300,
    }
}

pub fn analyze(args: &AnalyzeArgs) -> Result<AnalyzeResult> {
    let images = scan_images(&args.folder)?;
    let groups = build_groups(&images);
    let recommended = recommend_settings(&images);

    Ok(AnalyzeResult {
        images,
        groups,
        recommended,
    })
}

pub fn run(args: &RunArgs) -> Result<RunResult> {
    let images = scan_images(&args.folder)?;
    if images.is_empty() {
        anyhow::bail!("未找到图片文件");
    }

    let grid = parse_layout(&args.layout, images.len());
    let per_page = grid.rows * grid.cols;
    let margin_mm = args.margin_mm.unwrap_or(15.0);
    let show_filename = args.show_filename.unwrap_or(true);

    let (page_w, page_h) = if args.orientation == "landscape" {
        (A4_HEIGHT_MM, A4_WIDTH_MM)
    } else {
        (A4_WIDTH_MM, A4_HEIGHT_MM)
    };

    let usable_w = page_w - margin_mm * 2.0;
    let usable_h = page_h - margin_mm * 2.0;
    let cell_w = usable_w / grid.cols as f64;
    let cell_h = usable_h / grid.rows as f64;
    let filename_reserve = if show_filename { 6.0 } else { 0.0 };
    let image_cell_h = cell_h - filename_reserve;

    let total_pages = (images.len() + per_page - 1) / per_page;

    let output_dir = Path::new(&args.folder).join("_docsy_image_out");
    std::fs::create_dir_all(&output_dir)?;
    let ext = if args.output_format == "pdf" { "pdf" } else { "docx" };
    let output_path = output_dir.join(format!("image_paddler_output.{}", ext));

    match args.output_format.as_str() {
        "pdf" => generate_pdf(
            &images,
            &output_path,
            page_w,
            page_h,
            margin_mm,
            &grid,
            cell_w,
            image_cell_h,
            filename_reserve,
            show_filename,
            &args.scale_mode,
            args.dpi,
        )?,
        _ => generate_docx(
            &images,
            &output_path,
            page_w,
            page_h,
            margin_mm,
            &grid,
            cell_w,
            image_cell_h,
            filename_reserve,
            show_filename,
            &args.scale_mode,
            args.dpi,
        )?,
    }

    Ok(RunResult {
        output_path: output_path.display().to_string(),
        pages: total_pages as u32,
        images: images.len() as u32,
    })
}

fn compute_placement(
    img_w: u32,
    img_h: u32,
    cell_w_pt: f64,
    cell_h_pt: f64,
    scale_mode: &str,
    dpi: u32,
) -> (f64, f64, f64, f64) {
    let native_w_pt = img_w as f64 * 72.0 / dpi as f64;
    let native_h_pt = img_h as f64 * 72.0 / dpi as f64;

    let (draw_w, draw_h) = match scale_mode {
        "original" => (native_w_pt, native_h_pt),
        "fill" => {
            let scale_x = cell_w_pt / native_w_pt;
            let scale_y = cell_h_pt / native_h_pt;
            let scale = scale_x.max(scale_y);
            (native_w_pt * scale, native_h_pt * scale)
        }
        _ => {
            let scale_x = cell_w_pt / native_w_pt;
            let scale_y = cell_h_pt / native_h_pt;
            let scale = scale_x.min(scale_y);
            (native_w_pt * scale, native_h_pt * scale)
        }
    };

    let final_w = draw_w.min(cell_w_pt);
    let final_h = draw_h.min(cell_h_pt);

    (final_w, final_h, native_w_pt, native_h_pt)
}

fn generate_pdf(
    images: &[ImageInfo],
    output_path: &Path,
    page_w_mm: f64,
    page_h_mm: f64,
    margin_mm: f64,
    grid: &LayoutGrid,
    cell_w_mm: f64,
    image_cell_h_mm: f64,
    filename_reserve_mm: f64,
    show_filename: bool,
    scale_mode: &str,
    dpi: u32,
) -> Result<()> {
    use printpdf::*;

    let mut doc = PdfDocument::new("image_paddler");
    let mut warnings = Vec::new();
    let per_page = grid.rows * grid.cols;
    let font = BuiltinFont::Helvetica;

    let mut page_idx = 0;
    for chunk in images.chunks(per_page) {
        let mut ops: Vec<Op> = Vec::new();

        for (i, img_info) in chunk.iter().enumerate() {
            let row = i / grid.cols;
            let col = i % grid.cols;

            let cell_x_mm = margin_mm + col as f64 * cell_w_mm;
            let cell_y_mm = page_h_mm - margin_mm - (row as f64 + 1.0) * (image_cell_h_mm + filename_reserve_mm);

            let img = ::image::open(&img_info.path).map_err(|e| anyhow::anyhow!("{}", e))?;
            let raw_image = RawImage::from_dynamic_image(img)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let xobj_id = doc.add_image(&raw_image);

            let cell_w_pt = cell_w_mm * 72.0 / 25.4;
            let cell_h_pt = image_cell_h_mm * 72.0 / 25.4;

            let (draw_w_pt, draw_h_pt, _nw, _nh) =
                compute_placement(img_info.width, img_info.height, cell_w_pt, cell_h_pt, scale_mode, dpi);

            let offset_x_pt = (cell_w_pt - draw_w_pt) / 2.0;
            let offset_y_pt = (cell_h_pt - draw_h_pt) / 2.0;

            let base_x_pt = cell_x_mm * 72.0 / 25.4 + offset_x_pt;
            let base_y_pt = cell_y_mm * 72.0 / 25.4 + offset_y_pt;

            let scale_factor = draw_w_pt / (img_info.width as f64 * 72.0 / dpi as f64);

            ops.push(Op::SaveGraphicsState);
            ops.push(Op::UseXobject {
                id: xobj_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(base_x_pt as f32)),
                    translate_y: Some(Pt(base_y_pt as f32)),
                    scale_x: Some(scale_factor as f32),
                    scale_y: Some(scale_factor as f32),
                    dpi: Some(dpi as f32),
                    ..Default::default()
                },
            });
            ops.push(Op::RestoreGraphicsState);

            if show_filename {
                let name = Path::new(&img_info.path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let text_x_pt = cell_x_mm * 72.0 / 25.4;
                let text_y_pt = (cell_y_mm - 2.0) * 72.0 / 25.4;
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: PdfFontHandle::Builtin(font),
                    size: Pt(8.0),
                });
                ops.push(Op::SetTextCursor {
                    pos: Point {
                        x: Pt(text_x_pt as f32),
                        y: Pt(text_y_pt as f32),
                    },
                });
                ops.push(Op::SetFillColor {
                    col: Color::Rgb(Rgb::new(0.2, 0.2, 0.2, None)),
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(name.to_string())],
                });
                ops.push(Op::EndTextSection);
            }
        }

        let page = PdfPage::new(
            Mm(page_w_mm as f32),
            Mm(page_h_mm as f32),
            ops,
        );

        if page_idx == 0 {
            doc.with_pages(vec![page]);
        } else {
            doc.pages.push(page);
        }
        page_idx += 1;
    }

    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    std::fs::write(output_path, &bytes)?;
    Ok(())
}

fn generate_docx(
    images: &[ImageInfo],
    output_path: &Path,
    page_w_mm: f64,
    page_h_mm: f64,
    margin_mm: f64,
    grid: &LayoutGrid,
    cell_w_mm: f64,
    image_cell_h_mm: f64,
    _filename_reserve_mm: f64,
    show_filename: bool,
    scale_mode: &str,
    dpi: u32,
) -> Result<()> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    let per_page = grid.rows * grid.cols;
    let page_w_emu = (page_w_mm * 914400.0 / 25.4) as u64;
    let page_h_emu = (page_h_mm * 914400.0 / 25.4) as u64;
    let margin_l_emu = (margin_mm * 914400.0 / 25.4) as u64;
    let margin_r_emu = margin_l_emu;
    let margin_t_emu = margin_l_emu;
    let margin_b_emu = margin_l_emu;

    let mut media_entries: Vec<(String, Vec<u8>, String)> = Vec::new();
    let mut image_idx = 0usize;

    let mut body_xml = String::new();

    for chunk in images.chunks(per_page) {
        let mut table_xml = String::new();
        table_xml.push_str(&format!(
            "<w:tbl><w:tblPr>\
<w:tblW w:w=\"0\" w:type=\"auto\"/>\
<w:tblBorders><w:top w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:left w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:bottom w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:right w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:insideH w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:insideV w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
</w:tblBorders></w:tblPr>"
        ));

        for row_idx in 0..grid.rows {
            table_xml.push_str("<w:tr>");
            for col_idx in 0..grid.cols {
                let idx = row_idx * grid.cols + col_idx;
                if idx < chunk.len() {
                    let img_info = &chunk[idx];
                    let img_data = std::fs::read(&img_info.path)?;
                    let ext = Path::new(&img_info.path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png");
                    let content_type = match ext.to_lowercase().as_str() {
                        "jpg" | "jpeg" => "image/jpeg",
                        "png" => "image/png",
                        "webp" => "image/webp",
                        "bmp" => "image/bmp",
                        "tif" | "tiff" => "image/tiff",
                        _ => "image/png",
                    };

                    image_idx += 1;
                    let media_name = format!("image{}.{}", image_idx, ext);
                    media_entries.push((media_name.clone(), img_data, content_type.to_string()));

                    let cell_w_pt = cell_w_mm * 72.0 / 25.4;
                    let cell_h_pt = image_cell_h_mm * 72.0 / 25.4;
                    let (draw_w_pt, draw_h_pt, _, _) =
                        compute_placement(img_info.width, img_info.height, cell_w_pt, cell_h_pt, scale_mode, dpi);
                    let draw_w_emu = (draw_w_pt * 914400.0 / 72.0) as u64;
                    let draw_h_emu = (draw_h_pt * 914400.0 / 72.0) as u64;

                    let name = Path::new(&img_info.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let escaped_name = crate::docx::utils::xml_escape(name);

                    table_xml.push_str(&format!(
                        "<w:tc><w:tcPr><w:tcW w:w=\"0\" w:type=\"auto\"/></w:tcPr>"
                    ));
                    table_xml.push_str(&format!(
                        "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:drawing>\
<wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{}\" cy=\"{}\"/>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\
<a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:nvPicPr><pic:cNvPr id=\"{}\" name=\"{}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"rId{}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr>\
</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>",
                        draw_w_emu, draw_h_emu,
                        image_idx, escaped_name,
                        image_idx,
                        draw_w_emu, draw_h_emu
                    ));

                    if show_filename {
                        table_xml.push_str(&format!(
                            "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:rPr><w:sz w:val=\"16\"/></w:rPr><w:t>{}</w:t></w:r></w:p>",
                            escaped_name
                        ));
                    }

                    table_xml.push_str("</w:tc>");
                } else {
                    table_xml.push_str("<w:tc><w:tcPr><w:tcW w:w=\"0\" w:type=\"auto\"/></w:tcPr><w:p/></w:tc>");
                }
            }
            table_xml.push_str("</w:tr>");
        }

        table_xml.push_str("</w:tbl>");
        body_xml.push_str(&table_xml);
    }

    let mut rels_xml = String::new();
    rels_xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#);

    let mut media_rels = String::new();
    for (i, (media_name, _, _ct)) in media_entries.iter().enumerate() {
        media_rels.push_str(&format!(
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{}\"/>",
            i + 1, media_name
        ));
    }

    let mut content_types_xml = String::new();
    content_types_xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>"#);
    let mut seen_ext: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_, _, ct) in &media_entries {
        let ext_part = ct.split('/').last().unwrap_or("png");
        if seen_ext.insert(ext_part.to_string()) {
            content_types_xml.push_str(&format!(
                "<Default Extension=\"{}\" ContentType=\"{}\"/>",
                ext_part, ct
            ));
        }
    }
    content_types_xml.push_str("</Types>");

    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<w:body>
<w:sectPr>
<w:pgSz w:w=\"{}\" w:h=\"{}\"/>
<w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"0\" w:footer=\"0\" w:gutter=\"0\"/>
</w:sectPr>
{}
</w:body>
</w:document>"#,
        page_w_emu, page_h_emu,
        margin_t_emu, margin_r_emu, margin_b_emu, margin_l_emu,
        body_xml
    );

    let doc_rels_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
{}
</Relationships>"#,
        media_rels
    );

    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(content_types_xml.as_bytes())?;

    zip.start_file("_rels/.rels", options)?;
    zip.write_all(rels_xml.as_bytes())?;

    zip.start_file("word/document.xml", options)?;
    zip.write_all(document_xml.as_bytes())?;

    zip.start_file("word/_rels/document.xml.rels", options)?;
    zip.write_all(doc_rels_xml.as_bytes())?;

    for (media_name, data, _) in &media_entries {
        zip.start_file(&format!("word/media/{}", media_name), options)?;
        zip.write_all(data)?;
    }

    let cursor = zip.finish()?;
    let bytes = cursor.into_inner();
    std::fs::write(output_path, &bytes)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layout_grid() {
        let g = parse_layout("2x3", 6);
        assert_eq!(g.rows, 2);
        assert_eq!(g.cols, 3);
    }

    #[test]
    fn test_parse_layout_count() {
        let g = parse_layout("4", 8);
        assert_eq!(g.rows, 2);
        assert_eq!(g.cols, 2);
    }

    #[test]
    fn test_extract_prefix() {
        assert_eq!(extract_prefix("IMG_001.jpg"), "IMG");
        assert_eq!(extract_prefix("photo-001.jpg"), "photo");
        assert_eq!(extract_prefix("test123.jpg"), "test123");
    }

    #[test]
    fn test_natural_sort() {
        let mut v = vec!["img_10.jpg", "img_2.jpg", "img_1.jpg"];
        v.sort_by(|a, b| natural_sort_key(a).cmp(&natural_sort_key(b)));
        assert_eq!(v, vec!["img_1.jpg", "img_2.jpg", "img_10.jpg"]);
    }
}
