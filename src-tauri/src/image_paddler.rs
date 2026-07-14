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
    #[serde(default)]
    pub folders: Option<Vec<String>>,
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
    pub scale_mode: String,
    pub margin_mm: f64,
    pub show_filename: bool,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RunArgs {
    pub folder: String,
    #[serde(default)]
    pub folders: Option<Vec<String>>,
    pub output_format: String,
    pub layout: String,
    pub orientation: String,
    pub dpi: u32,
    pub scale_mode: String,
    #[serde(default)]
    pub custom_rows: Option<usize>,
    #[serde(default)]
    pub custom_cols: Option<usize>,
    #[serde(default)]
    pub margin_mm: Option<f64>,
    #[serde(default)]
    pub show_filename: Option<bool>,
    #[serde(default)]
    pub filename_without_ext: Option<bool>,
    #[serde(default)]
    pub filename_remove_text: Option<String>,
    #[serde(default)]
    pub order_mode: Option<String>,
    #[serde(default)]
    pub border_enabled: Option<bool>,
    #[serde(default)]
    pub border_color: Option<String>,
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

fn parse_layout(
    layout: &str,
    custom_rows: Option<usize>,
    custom_cols: Option<usize>,
) -> LayoutGrid {
    if layout == "custom" {
        return LayoutGrid {
            rows: custom_rows.unwrap_or(2).clamp(1, 8),
            cols: custom_cols.unwrap_or(2).clamp(1, 8),
        };
    }
    if layout.contains('x') {
        let parts: Vec<&str> = layout.split('x').collect();
        if parts.len() == 2 {
            let rows = parts[0].parse::<usize>().unwrap_or(1).clamp(1, 8);
            let cols = parts[1].parse::<usize>().unwrap_or(1).clamp(1, 8);
            return LayoutGrid { rows, cols };
        }
    }
    match layout {
        "1" => LayoutGrid { rows: 1, cols: 1 },
        "2" => LayoutGrid { rows: 1, cols: 2 },
        "3" => LayoutGrid { rows: 1, cols: 3 },
        "4" => LayoutGrid { rows: 2, cols: 2 },
        _ => {
            let n = layout.parse::<usize>().unwrap_or(4).clamp(1, 64);
            let cols = (n as f64).sqrt().ceil() as usize;
            let rows = n.div_ceil(cols);
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

fn folders_from_args(folder: &str, folders: &Option<Vec<String>>) -> Vec<String> {
    let mut result: Vec<String> = folders
        .as_ref()
        .map(|items| {
            items
                .iter()
                .filter(|item| !item.trim().is_empty())
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if result.is_empty() && !folder.trim().is_empty() {
        result.push(folder.to_string());
    }
    result
}

fn scan_image_folders(folder: &str, folders: &Option<Vec<String>>) -> Result<Vec<ImageInfo>> {
    let mut all = Vec::new();
    for item in folders_from_args(folder, folders) {
        let path = Path::new(&item);
        if path.is_dir() {
            all.extend(scan_images(&item)?);
        } else if path.is_file() {
            if let Some(img) = image_info_from_path(path)? {
                all.push(img);
            }
        }
    }
    all.sort_by(|a, b| {
        let ka = natural_sort_key(&a.path);
        let kb = natural_sort_key(&b.path);
        ka.cmp(&kb)
    });
    Ok(all)
}

fn image_info_from_path(path: &Path) -> Result<Option<ImageInfo>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return Ok(None);
    }
    let metadata = std::fs::metadata(path)?;
    let img = match image::open(path) {
        Ok(img) => img,
        Err(_) => return Ok(None),
    };
    Ok(Some(ImageInfo {
        path: path.display().to_string(),
        width: img.width(),
        height: img.height(),
        file_size: metadata.len(),
    }))
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
            layout: "2x1".into(),
            dpi: 300,
            scale_mode: "fit".into(),
            margin_mm: 12.0,
            show_filename: true,
            reason: "未找到图片时使用保守的 A4 证据排版参数".into(),
        };
    }

    let mut widths: Vec<u32> = images.iter().map(|i| i.width).collect();
    let mut heights: Vec<u32> = images.iter().map(|i| i.height).collect();
    widths.sort_unstable();
    heights.sort_unstable();

    let median_w = widths[widths.len() / 2] as f64;
    let median_h = heights[heights.len() / 2] as f64;
    let aspect = if median_h > 0.0 {
        median_w / median_h
    } else {
        1.0
    };
    let native_w_mm = median_w * 25.4 / 300.0;
    let native_h_mm = median_h * 25.4 / 300.0;
    let original_size_is_printable = native_w_mm >= 120.0 && native_h_mm >= 65.0;
    let original_fits_two_up =
        native_w_mm <= 186.0 && native_h_mm <= 126.0 && original_size_is_printable;
    let scale_mode = if original_fits_two_up {
        "original"
    } else {
        "fit"
    };

    let (orientation, layout, reason) = if images.len() == 1 {
        let orientation = if aspect >= 1.15 {
            "landscape"
        } else {
            "portrait"
        };
        (
            orientation,
            "1",
            "只有 1 张图片，推荐一页一张，便于作为单独证据页打印",
        )
    } else if aspect >= 1.25 {
        if median_w >= 1200.0 && median_h >= 650.0 {
            (
                "portrait",
                "2x1",
                "检测到横向视频截图，推荐 A4 竖页上下排 2 张，能保持接近整页宽度并节省页数",
            )
        } else {
            (
                "landscape",
                "1",
                "横向截图分辨率偏低，推荐横向 A4 一页一张以优先保证可读性",
            )
        }
    } else if aspect <= 0.8 {
        if median_w >= 650.0 && median_h >= 1200.0 {
            (
                "landscape",
                "1x2",
                "检测到竖向截图，推荐 A4 横页左右排 2 张，能保持较大的显示高度",
            )
        } else {
            (
                "portrait",
                "1",
                "竖向截图分辨率偏低，推荐竖向 A4 一页一张以优先保证可读性",
            )
        }
    } else {
        (
            "portrait",
            "2x1",
            "图片比例接近方形或普通截图，推荐 A4 竖页上下排 2 张，兼顾清晰度和页数",
        )
    };

    RecommendedSettings {
        orientation: orientation.into(),
        layout: layout.into(),
        dpi: 300,
        scale_mode: scale_mode.into(),
        margin_mm: 12.0,
        show_filename: true,
        reason: reason.into(),
    }
}

pub fn analyze(args: &AnalyzeArgs) -> Result<AnalyzeResult> {
    let images = scan_image_folders(&args.folder, &args.folders)?;
    let groups = build_groups(&images);
    let recommended = recommend_settings(&images);

    Ok(AnalyzeResult {
        images,
        groups,
        recommended,
    })
}

pub fn run(args: &RunArgs) -> Result<RunResult> {
    let mut images = scan_image_folders(&args.folder, &args.folders)?;
    if images.is_empty() {
        anyhow::bail!("未找到图片文件");
    }

    let grid = parse_layout(&args.layout, args.custom_rows, args.custom_cols);
    let per_page = grid.rows * grid.cols;
    let margin_mm = args.margin_mm.unwrap_or(15.0);
    let show_filename = args.show_filename.unwrap_or(true);
    let filename_without_ext = args.filename_without_ext.unwrap_or(false);
    let filename_remove_text = args.filename_remove_text.clone().unwrap_or_default();
    let order_mode = args.order_mode.as_deref().unwrap_or("z");
    let border_enabled = args.border_enabled.unwrap_or(false);
    let border_color = args.border_color.as_deref().unwrap_or("black");
    let resolved_orientation = if args.orientation == "auto" {
        recommend_settings(&images).orientation
    } else {
        args.orientation.clone()
    };

    let (page_w, page_h) = if resolved_orientation == "landscape" {
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

    let total_pages = images.len().div_ceil(per_page);
    reorder_images(&mut images, &grid, order_mode);

    let first_folder = folders_from_args(&args.folder, &args.folders)
        .into_iter()
        .next()
        .unwrap_or_else(|| args.folder.clone());
    let output_dir = Path::new(&first_folder).join("_docsy_image_out");
    std::fs::create_dir_all(&output_dir)?;
    let ext = if args.output_format == "pdf" {
        "pdf"
    } else {
        "docx"
    };
    let output_stem = output_file_stem(&images);
    let output_path = unique_output_path(&output_dir, &format!("{output_stem}_docsy_paddler"), ext);

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
            filename_without_ext,
            &filename_remove_text,
            border_enabled,
            border_color,
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
            filename_without_ext,
            &filename_remove_text,
            border_enabled,
            border_color,
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

fn reorder_images(images: &mut Vec<ImageInfo>, grid: &LayoutGrid, order_mode: &str) {
    let per_page = grid.rows * grid.cols;
    let mut reordered = Vec::with_capacity(images.len());
    for chunk in images.chunks(per_page) {
        let order = cell_order(grid, order_mode);
        for idx in order {
            if idx < chunk.len() {
                reordered.push(chunk[idx].clone());
            }
        }
    }
    *images = reordered;
}

fn cell_order(grid: &LayoutGrid, order_mode: &str) -> Vec<usize> {
    let mut order = Vec::with_capacity(grid.rows * grid.cols);
    match order_mode {
        "n" => {
            for col in 0..grid.cols {
                for row in 0..grid.rows {
                    order.push(row * grid.cols + col);
                }
            }
        }
        "reverse_n" => {
            for col in (0..grid.cols).rev() {
                for row in 0..grid.rows {
                    order.push(row * grid.cols + col);
                }
            }
        }
        _ => {
            for row in 0..grid.rows {
                for col in 0..grid.cols {
                    order.push(row * grid.cols + col);
                }
            }
        }
    }
    order
}

fn display_filename(path: &str, without_ext: bool, remove_text: &str) -> String {
    let path = Path::new(path);
    let mut name = if without_ext {
        path.file_stem()
    } else {
        path.file_name()
    }
    .and_then(|s| s.to_str())
    .unwrap_or("")
    .to_string();
    if !remove_text.is_empty() {
        name = name.replace(remove_text, "");
    }
    name
}

fn output_file_stem(images: &[ImageInfo]) -> String {
    let first_name = images
        .first()
        .and_then(|img| Path::new(&img.path).file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("images");
    let re = Regex::new(r"(?i)(?:[_-]?(?:frame|img|image)?[_-]?\d+)$").unwrap();
    let cleaned = re.replace(first_name, "");
    let stem = cleaned.trim_matches(['_', '-', ' ']);
    let fallback = if stem.is_empty() { first_name } else { stem };
    sanitize_output_name(fallback)
}

fn sanitize_output_name(name: &str) -> String {
    let value: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric()
                || matches!(ch, '-' | '_')
                || ('\u{4e00}'..='\u{9fff}').contains(&ch)
            {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if value.is_empty() {
        "images".into()
    } else {
        value
    }
}

fn unique_output_path(dir: &Path, stem: &str, ext: &str) -> std::path::PathBuf {
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut index = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}_{index}.{ext}"));
        index += 1;
    }
    candidate
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

    let scale = match scale_mode {
        "original" => {
            let fit_scale = (cell_w_pt / native_w_pt).min(cell_h_pt / native_h_pt);
            fit_scale.min(1.0)
        }
        _ => {
            let scale_x = cell_w_pt / native_w_pt;
            let scale_y = cell_h_pt / native_h_pt;
            scale_x.min(scale_y)
        }
    };

    (
        native_w_pt * scale,
        native_h_pt * scale,
        native_w_pt,
        native_h_pt,
    )
}

#[allow(clippy::too_many_arguments)]
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
    filename_without_ext: bool,
    filename_remove_text: &str,
    border_enabled: bool,
    border_color: &str,
    scale_mode: &str,
    dpi: u32,
) -> Result<()> {
    use printpdf::*;

    let mut doc = PdfDocument::new("image_paddler");
    let mut warnings = Vec::new();
    let per_page = grid.rows * grid.cols;
    let font = BuiltinFont::Helvetica;

    for (page_idx, chunk) in images.chunks(per_page).enumerate() {
        let mut ops: Vec<Op> = Vec::new();

        for (i, img_info) in chunk.iter().enumerate() {
            let row = i / grid.cols;
            let col = i % grid.cols;

            let cell_x_mm = margin_mm + col as f64 * cell_w_mm;
            let cell_y_mm = page_h_mm
                - margin_mm
                - (row as f64 + 1.0) * (image_cell_h_mm + filename_reserve_mm);
            let image_area_y_mm = cell_y_mm + filename_reserve_mm;
            if border_enabled {
                let rect_x_pt = cell_x_mm * 72.0 / 25.4;
                let rect_y_pt = cell_y_mm * 72.0 / 25.4;
                let rect_w_pt = cell_w_mm * 72.0 / 25.4;
                let rect_h_pt = (image_cell_h_mm + filename_reserve_mm) * 72.0 / 25.4;
                ops.push(Op::SetOutlineColor {
                    col: pdf_border_color(border_color),
                });
                ops.push(Op::SetOutlineThickness { pt: Pt(0.75) });
                ops.push(Op::DrawRectangle {
                    rectangle: Rect::from_xywh(
                        Pt(rect_x_pt as f32),
                        Pt(rect_y_pt as f32),
                        Pt(rect_w_pt as f32),
                        Pt(rect_h_pt as f32),
                    ),
                });
            }

            let img = ::image::open(&img_info.path).map_err(|e| anyhow::anyhow!("{}", e))?;
            let raw_image =
                RawImage::from_dynamic_image(img).map_err(|e| anyhow::anyhow!("{}", e))?;
            let xobj_id = doc.add_image(&raw_image);

            let cell_w_pt = cell_w_mm * 72.0 / 25.4;
            let cell_h_pt = image_cell_h_mm * 72.0 / 25.4;

            let (draw_w_pt, draw_h_pt, _nw, _nh) = compute_placement(
                img_info.width,
                img_info.height,
                cell_w_pt,
                cell_h_pt,
                scale_mode,
                dpi,
            );

            let offset_x_pt = (cell_w_pt - draw_w_pt) / 2.0;
            let offset_y_pt = (cell_h_pt - draw_h_pt) / 2.0;

            let base_x_pt = cell_x_mm * 72.0 / 25.4 + offset_x_pt;
            let base_y_pt = image_area_y_mm * 72.0 / 25.4 + offset_y_pt;

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
                let name =
                    display_filename(&img_info.path, filename_without_ext, filename_remove_text);
                let text_x_pt = cell_x_mm * 72.0 / 25.4;
                let text_y_pt = (cell_y_mm + 1.5) * 72.0 / 25.4;
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
                    items: vec![TextItem::Text(name)],
                });
                ops.push(Op::EndTextSection);
            }
        }

        let page = PdfPage::new(Mm(page_w_mm as f32), Mm(page_h_mm as f32), ops);

        if page_idx == 0 {
            doc.with_pages(vec![page]);
        } else {
            doc.pages.push(page);
        }
    }

    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    std::fs::write(output_path, &bytes)?;
    Ok(())
}

fn pdf_border_color(color: &str) -> printpdf::Color {
    match color {
        "white" => printpdf::Color::Rgb(printpdf::Rgb::new(1.0, 1.0, 1.0, None)),
        _ => printpdf::Color::Rgb(printpdf::Rgb::new(0.0, 0.0, 0.0, None)),
    }
}

#[allow(clippy::too_many_arguments)]
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
    filename_without_ext: bool,
    filename_remove_text: &str,
    border_enabled: bool,
    border_color: &str,
    scale_mode: &str,
    dpi: u32,
) -> Result<()> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let per_page = grid.rows * grid.cols;
    let page_w_twips = (page_w_mm * 1440.0 / 25.4) as u64;
    let page_h_twips = (page_h_mm * 1440.0 / 25.4) as u64;
    let margin_l_twips = (margin_mm * 1440.0 / 25.4) as u64;
    let margin_r_twips = margin_l_twips;
    let margin_t_twips = margin_l_twips;
    let margin_b_twips = margin_l_twips;
    let usable_w_twips = page_w_twips.saturating_sub(margin_l_twips + margin_r_twips);
    let usable_h_twips = page_h_twips.saturating_sub(margin_t_twips + margin_b_twips);
    let cell_w_twips = (usable_w_twips / grid.cols as u64).max(1);
    let cell_h_twips = (usable_h_twips / grid.rows as u64).max(1);

    let mut media_entries: Vec<(String, Vec<u8>, String, String)> = Vec::new();
    let mut image_idx = 0usize;

    let mut body_xml = String::new();

    let total_pages = images.len().div_ceil(per_page);

    for (chunk_idx, chunk) in images.chunks(per_page).enumerate() {
        let mut table_xml = String::new();
        table_xml.push_str(
            "<w:tbl><w:tblPr>\
<w:tblW w:w=\"0\" w:type=\"auto\"/>\
<w:tblLayout w:type=\"fixed\"/>\
<w:tblBorders><w:top w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:left w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:bottom w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:right w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:insideH w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
<w:insideV w:val=\"none\" w:sz=\"0\" w:space=\"0\" w:color=\"auto\"/>\
</w:tblBorders></w:tblPr><w:tblGrid>",
        );
        for _ in 0..grid.cols {
            table_xml.push_str(&format!("<w:gridCol w:w=\"{}\"/>", cell_w_twips));
        }
        table_xml.push_str("</w:tblGrid>");

        for row_idx in 0..grid.rows {
            table_xml.push_str(&format!(
                "<w:tr><w:trPr><w:trHeight w:val=\"{}\" w:hRule=\"atLeast\"/></w:trPr>",
                cell_h_twips
            ));
            for col_idx in 0..grid.cols {
                let idx = row_idx * grid.cols + col_idx;
                if idx < chunk.len() {
                    let img_info = &chunk[idx];
                    let img_data = std::fs::read(&img_info.path)?;
                    let ext = Path::new(&img_info.path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png")
                        .to_ascii_lowercase();
                    let content_type = match ext.as_str() {
                        "jpg" | "jpeg" => "image/jpeg",
                        "png" => "image/png",
                        "webp" => "image/webp",
                        "bmp" => "image/bmp",
                        "tif" | "tiff" => "image/tiff",
                        _ => "image/png",
                    };

                    image_idx += 1;
                    let media_name = format!("image{}.{}", image_idx, ext);
                    media_entries.push((
                        media_name.clone(),
                        img_data,
                        content_type.to_string(),
                        ext.clone(),
                    ));

                    let cell_w_pt = cell_w_mm * 72.0 / 25.4;
                    let cell_h_pt = image_cell_h_mm * 72.0 / 25.4;
                    let (draw_w_pt, draw_h_pt, _, _) = compute_placement(
                        img_info.width,
                        img_info.height,
                        cell_w_pt,
                        cell_h_pt,
                        scale_mode,
                        dpi,
                    );
                    let draw_w_emu = (draw_w_pt * 914400.0 / 72.0) as u64;
                    let draw_h_emu = (draw_h_pt * 914400.0 / 72.0) as u64;

                    let name = display_filename(
                        &img_info.path,
                        filename_without_ext,
                        filename_remove_text,
                    );
                    let escaped_name = xml_escape(&name);

                    table_xml.push_str(&format!(
                        "<w:tc><w:tcPr><w:tcW w:w=\"{}\" w:type=\"dxa\"/><w:vAlign w:val=\"center\"/>",
                        cell_w_twips
                    ));
                    if border_enabled {
                        table_xml.push_str(&format!(
                            "<w:tcBorders>\
<w:top w:val=\"single\" w:sz=\"8\" w:space=\"0\" w:color=\"{}\"/>\
<w:left w:val=\"single\" w:sz=\"8\" w:space=\"0\" w:color=\"{}\"/>\
<w:bottom w:val=\"single\" w:sz=\"8\" w:space=\"0\" w:color=\"{}\"/>\
<w:right w:val=\"single\" w:sz=\"8\" w:space=\"0\" w:color=\"{}\"/>\
</w:tcBorders>",
                            docx_border_color(border_color),
                            docx_border_color(border_color),
                            docx_border_color(border_color),
                            docx_border_color(border_color)
                        ));
                    }
                    table_xml.push_str("</w:tcPr>");
                    table_xml.push_str(&format!(
                        "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:drawing>\
<wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{}\" cy=\"{}\"/>\
<wp:docPr id=\"{}\" name=\"{}\"/>\
<wp:cNvGraphicFramePr/>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\
<a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:nvPicPr><pic:cNvPr id=\"{}\" name=\"{}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"rId{}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr>\
</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>",
                        draw_w_emu,
                        draw_h_emu,
                        image_idx,
                        escaped_name,
                        image_idx,
                        escaped_name,
                        image_idx,
                        draw_w_emu,
                        draw_h_emu
                    ));

                    if show_filename {
                        table_xml.push_str(&format!(
                            "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:rPr><w:sz w:val=\"16\"/></w:rPr><w:t>{}</w:t></w:r></w:p>",
                            escaped_name
                        ));
                    }

                    table_xml.push_str("</w:tc>");
                } else {
                    table_xml.push_str(&format!(
                        "<w:tc><w:tcPr><w:tcW w:w=\"{}\" w:type=\"dxa\"/></w:tcPr><w:p/></w:tc>",
                        cell_w_twips
                    ));
                }
            }
            table_xml.push_str("</w:tr>");
        }

        table_xml.push_str("</w:tbl>");
        body_xml.push_str(&table_xml);
        if chunk_idx + 1 < total_pages {
            body_xml.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
    }

    let mut rels_xml = String::new();
    rels_xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#);

    let mut media_rels = String::new();
    for (i, (media_name, _, _ct, _ext)) in media_entries.iter().enumerate() {
        media_rels.push_str(&format!(
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{}\"/>",
            i + 1, media_name
        ));
    }

    let mut content_types_xml = String::new();
    content_types_xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>"#,
    );
    let mut seen_ext: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_, _, ct, ext) in &media_entries {
        if seen_ext.insert(ext.clone()) {
            content_types_xml.push_str(&format!(
                "<Default Extension=\"{}\" ContentType=\"{}\"/>",
                xml_escape(ext),
                ct
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
{}
<w:sectPr>
<w:pgSz w:w="{}" w:h="{}"/>
<w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="0" w:footer="0" w:gutter="0"/>
</w:sectPr>
</w:body>
</w:document>"#,
        body_xml,
        page_w_twips,
        page_h_twips,
        margin_t_twips,
        margin_r_twips,
        margin_b_twips,
        margin_l_twips
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

    for (media_name, data, _, _) in &media_entries {
        zip.start_file(format!("word/media/{}", media_name), options)?;
        zip.write_all(data)?;
    }

    let cursor = zip.finish()?;
    let bytes = cursor.into_inner();
    std::fs::write(output_path, &bytes)?;

    Ok(())
}

fn docx_border_color(color: &str) -> &'static str {
    match color {
        "white" => "FFFFFF",
        _ => "000000",
    }
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_parse_layout_grid() {
        let g = parse_layout("2x3", None, None);
        assert_eq!(g.rows, 2);
        assert_eq!(g.cols, 3);
    }

    #[test]
    fn test_parse_layout_count() {
        let g = parse_layout("4", None, None);
        assert_eq!(g.rows, 2);
        assert_eq!(g.cols, 2);
    }

    #[test]
    fn test_parse_custom_layout() {
        let g = parse_layout("custom", Some(3), Some(2));
        assert_eq!(g.rows, 3);
        assert_eq!(g.cols, 2);
    }

    #[test]
    fn recommends_stacked_pages_for_landscape_video_frames() {
        let images = vec![
            ImageInfo {
                path: "frame_001.jpg".into(),
                width: 1920,
                height: 1080,
                file_size: 1,
            },
            ImageInfo {
                path: "frame_002.jpg".into(),
                width: 1920,
                height: 1080,
                file_size: 1,
            },
        ];
        let rec = recommend_settings(&images);
        assert_eq!(rec.orientation, "portrait");
        assert_eq!(rec.layout, "2x1");
        assert_eq!(rec.scale_mode, "original");
    }

    #[test]
    fn recommends_side_by_side_pages_for_portrait_frames() {
        let images = vec![
            ImageInfo {
                path: "phone_001.jpg".into(),
                width: 1080,
                height: 1920,
                file_size: 1,
            },
            ImageInfo {
                path: "phone_002.jpg".into(),
                width: 1080,
                height: 1920,
                file_size: 1,
            },
        ];
        let rec = recommend_settings(&images);
        assert_eq!(rec.orientation, "landscape");
        assert_eq!(rec.layout, "1x2");
    }

    #[test]
    fn recommends_single_page_for_single_image() {
        let images = vec![ImageInfo {
            path: "single.jpg".into(),
            width: 1280,
            height: 720,
            file_size: 1,
        }];
        let rec = recommend_settings(&images);
        assert_eq!(rec.orientation, "landscape");
        assert_eq!(rec.layout, "1");
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

    #[test]
    fn generated_docx_has_valid_package_parts_and_unique_name() {
        let root = std::env::temp_dir().join(format!(
            "docsy_image_paddler_test_{}_{}",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();

        let img_path = root.join("evidence_clip_frame_0001.png");
        let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(240, 120, image::Rgba([240, 240, 240, 255]));
        img.save(&img_path).unwrap();

        let args = RunArgs {
            folder: root.display().to_string(),
            folders: None,
            output_format: "docx".into(),
            layout: "1".into(),
            orientation: "portrait".into(),
            dpi: 300,
            scale_mode: "fit".into(),
            custom_rows: None,
            custom_cols: None,
            margin_mm: Some(6.0),
            show_filename: Some(true),
            filename_without_ext: Some(true),
            filename_remove_text: Some("_clip".into()),
            order_mode: Some("z".into()),
            border_enabled: Some(true),
            border_color: Some("black".into()),
        };

        let first = run(&args).unwrap();
        let second = run(&args).unwrap();
        assert_ne!(first.output_path, second.output_path);
        assert!(first
            .output_path
            .ends_with("evidence_clip_docsy_paddler.docx"));
        assert!(second
            .output_path
            .ends_with("evidence_clip_docsy_paddler_2.docx"));

        let file = std::fs::File::open(&first.output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        archive.by_name("[Content_Types].xml").unwrap();
        archive.by_name("_rels/.rels").unwrap();
        archive.by_name("word/_rels/document.xml.rels").unwrap();
        archive.by_name("word/media/image1.png").unwrap();

        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();
        assert!(document_xml.contains("<wp:docPr"));
        assert!(document_xml.contains("<w:tcBorders>"));
        assert!(document_xml.contains(">evidence_frame_0001<"));

        let _ = std::fs::remove_dir_all(root);
    }
}
