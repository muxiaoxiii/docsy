use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use crate::sort_utils::natural_cmp;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff"];

const A4_WIDTH_MM: f64 = 210.0;
const A4_HEIGHT_MM: f64 = 297.0;
const FILENAME_FONT_PT: f64 = 8.0;
const FILENAME_MAX_LINES: usize = 2;
const FILENAME_LINE_HEIGHT_MM: f64 = 4.2;
const DOCX_TRAILING_GAP_MM: f64 = 2.0;

static TRAILING_NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[-_]\d+$").unwrap());
static TIME_PART_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\d{1,2}[:：_-]\d{2}(?:[:：_-]\d{2})?|\d+(?:\.\d+)?s|\d+m\d+s").unwrap()
});
static NUMBER_PART_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());
static OUTPUT_STEM_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:[_-]?(?:frame|img|image)?[_-]?\d+)$").unwrap());

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
    pub filename_rules: Option<Vec<FilenameRule>>,
    #[serde(default)]
    pub order_mode: Option<String>,
    #[serde(default)]
    pub border_enabled: Option<bool>,
    #[serde(default)]
    pub border_color: Option<String>,
    /// `merged` keeps the selected folders as one evidence set. `per_folder`
    /// writes one document for each selected folder so unrelated batches never
    /// silently end up in the same filing.
    #[serde(default)]
    pub output_mode: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FilenameRule {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub replacement: String,
    #[serde(default)]
    pub keep_number: Option<bool>,
    #[serde(default)]
    pub keep_time: Option<bool>,
    #[serde(default)]
    pub keep_text: Option<bool>,
    #[serde(default)]
    pub separator: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunResult {
    pub output_path: String,
    #[serde(default)]
    pub output_paths: Vec<String>,
    pub pages: u32,
    pub images: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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
        "2" => LayoutGrid { rows: 2, cols: 1 },
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

fn extract_prefix(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    TRAILING_NUMBER_RE.replace(stem, "").to_string()
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
                let (width, height) = match image::image_dimensions(&path) {
                    Ok(dimensions) => dimensions,
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

    images.sort_by(|a, b| natural_cmp(&a.path, &b.path));

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
    all.sort_by(|a, b| natural_cmp(&a.path, &b.path));
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
    let (width, height) = match image::image_dimensions(path) {
        Ok(dimensions) => dimensions,
        Err(_) => return Ok(None),
    };
    Ok(Some(ImageInfo {
        path: path.display().to_string(),
        width,
        height,
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
    groups.sort_by(|a, b| natural_cmp(&a.prefix, &b.prefix));
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
    let sources = folders_from_args(&args.folder, &args.folders);
    if args.output_mode.as_deref() == Some("per_folder") && sources.len() > 1 {
        let mut outputs = Vec::new();
        let mut total_pages = 0_u32;
        let mut total_images = 0_u32;
        let mut warnings = Vec::new();
        for source in sources {
            let images = scan_image_source(&source)?;
            if images.is_empty() {
                warnings.push(format!("未在 {} 找到可排版的图片", source));
                continue;
            }
            let result = run_images(args, images, &image_output_dir(&source))?;
            total_pages = total_pages.saturating_add(result.pages);
            total_images = total_images.saturating_add(result.images);
            warnings.extend(result.warnings);
            outputs.push(result.output_path);
        }
        let Some(output_path) = outputs.first().cloned() else {
            anyhow::bail!("未在所选文件夹中找到图片文件");
        };
        return Ok(RunResult {
            output_path,
            output_paths: outputs,
            pages: total_pages,
            images: total_images,
            warnings,
        });
    }

    let images = scan_image_folders(&args.folder, &args.folders)?;
    let first_source = sources.first().cloned().unwrap_or_else(|| args.folder.clone());
    run_images(args, images, &image_output_dir(&first_source))
}

fn scan_image_source(source: &str) -> Result<Vec<ImageInfo>> {
    let path = Path::new(source);
    if path.is_dir() {
        scan_images(source)
    } else {
        Ok(image_info_from_path(path)?.into_iter().collect())
    }
}

fn image_output_dir(source: &str) -> std::path::PathBuf {
    let path = Path::new(source);
    if path.is_dir() {
        path.join("_docsy_image_out")
    } else {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .join("_docsy_image_out")
    }
}

fn run_images(args: &RunArgs, mut images: Vec<ImageInfo>, output_dir: &Path) -> Result<RunResult> {
    if images.is_empty() {
        anyhow::bail!("未找到图片文件");
    }

    let grid = parse_layout(&args.layout, args.custom_rows, args.custom_cols);
    let per_page = grid.rows * grid.cols;
    let margin_mm = args.margin_mm.unwrap_or(15.0);
    let show_filename = args.show_filename.unwrap_or(true);
    let filename_without_ext = args.filename_without_ext.unwrap_or(false);
    let filename_remove_text = args.filename_remove_text.clone().unwrap_or_default();
    let filename_rules = args.filename_rules.clone().unwrap_or_default();
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
    let layout_usable_h = if args.output_format == "docx" {
        (usable_h - DOCX_TRAILING_GAP_MM).max(1.0)
    } else {
        usable_h
    };
    let cell_w = usable_w / grid.cols as f64;
    let cell_h = layout_usable_h / grid.rows as f64;
    let filename_reserve = if show_filename {
        FILENAME_LINE_HEIGHT_MM * FILENAME_MAX_LINES as f64
    } else {
        0.0
    };
    let image_cell_h = cell_h - filename_reserve;

    let total_pages = images.len().div_ceil(per_page);
    reorder_images(&mut images, &grid, order_mode);

    std::fs::create_dir_all(output_dir)?;
    let ext = if args.output_format == "pdf" {
        "pdf"
    } else {
        "docx"
    };
    let output_stem = output_file_stem(&images);
    let output_path = unique_output_path(output_dir, &format!("{output_stem}_docsy_paddler"), ext);

    let warnings = match args.output_format.as_str() {
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
            &filename_rules,
            border_enabled,
            border_color,
            &args.scale_mode,
            args.dpi,
        )?,
        _ => {
            generate_docx(
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
                &filename_rules,
                border_enabled,
                border_color,
                &args.scale_mode,
                args.dpi,
            )?;
            Vec::new()
        }
    };

    Ok(RunResult {
        output_path: output_path.display().to_string(),
        output_paths: Vec::new(),
        pages: total_pages as u32,
        images: images.len() as u32,
        warnings,
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

fn display_filename(
    path: &str,
    without_ext: bool,
    remove_text: &str,
    rules: &[FilenameRule],
) -> String {
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
    apply_filename_rules(&name, rules)
}

fn apply_filename_rules(input: &str, rules: &[FilenameRule]) -> String {
    let mut value = input.to_string();
    for rule in rules {
        match rule.kind.as_str() {
            "remove" => {
                if !rule.value.is_empty() {
                    value = value.replace(&rule.value, "");
                }
            }
            "replace" => {
                if !rule.value.is_empty() {
                    value = value.replace(&rule.value, &rule.replacement);
                }
            }
            "prefix" => {
                if !rule.value.is_empty() {
                    value = format!("{}{}", rule.value, value);
                }
            }
            "suffix" => {
                if !rule.value.is_empty() {
                    value = format!("{}{}", value, rule.value);
                }
            }
            "keep" => {
                value = keep_filename_parts(&value, rule);
            }
            _ => {}
        }
    }
    value.trim().to_string()
}

fn keep_filename_parts(input: &str, rule: &FilenameRule) -> String {
    let separator = rule
        .separator
        .as_deref()
        .filter(|v| !v.is_empty())
        .unwrap_or("_");
    let mut parts = Vec::new();
    if !rule.replacement.trim().is_empty() {
        parts.push(rule.replacement.trim().to_string());
    }
    if rule.keep_time.unwrap_or(false) {
        parts.extend(extract_time_parts(input));
    }
    if rule.keep_number.unwrap_or(false) {
        parts.extend(extract_number_parts_without_times(input));
    }
    if rule.keep_text.unwrap_or(false) {
        parts.extend(extract_text_parts(input));
    }
    dedupe_parts(parts).join(separator)
}

fn extract_time_parts(input: &str) -> Vec<String> {
    TIME_PART_RE
        .find_iter(input)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn extract_number_parts(input: &str) -> Vec<String> {
    NUMBER_PART_RE
        .find_iter(input)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn extract_number_parts_without_times(input: &str) -> Vec<String> {
    let cleaned = TIME_PART_RE.replace_all(input, " ");
    extract_number_parts(&cleaned)
}

fn extract_text_parts(input: &str) -> Vec<String> {
    input
        .split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .filter(|part| !part.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
        .collect()
}

fn dedupe_parts(parts: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for part in parts {
        if !part.is_empty() && !result.contains(&part) {
            result.push(part);
        }
    }
    result
}

fn display_filename_lines(
    path: &str,
    without_ext: bool,
    remove_text: &str,
    rules: &[FilenameRule],
    cell_w_mm: f64,
) -> Vec<String> {
    wrap_filename_lines(
        &display_filename(path, without_ext, remove_text, rules),
        cell_w_mm,
        FILENAME_MAX_LINES,
    )
}

fn wrap_filename_lines(name: &str, cell_w_mm: f64, max_lines: usize) -> Vec<String> {
    let max_units = ((cell_w_mm * 72.0 / 25.4) / (FILENAME_FONT_PT * 0.56))
        .floor()
        .max(6.0) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_units = 0usize;

    for ch in name.chars() {
        let units = if ch.is_ascii() { 1 } else { 2 };
        if current_units + units > max_units && !current.is_empty() {
            lines.push(current);
            current = String::new();
            current_units = 0;
            if lines.len() >= max_lines {
                break;
            }
        }
        current.push(ch);
        current_units += units;
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    if name_units(name) > lines.iter().map(|line| name_units(line)).sum::<usize>() {
        if let Some(last) = lines.last_mut() {
            while name_units(last) + 1 > max_units && !last.is_empty() {
                last.pop();
            }
            last.push('…');
        }
    }
    lines
}

fn name_units(value: &str) -> usize {
    value
        .chars()
        .map(|ch| if ch.is_ascii() { 1 } else { 2 })
        .sum()
}

fn output_file_stem(images: &[ImageInfo]) -> String {
    let first_name = images
        .first()
        .and_then(|img| Path::new(&img.path).file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("images");
    let cleaned = OUTPUT_STEM_SUFFIX_RE.replace(first_name, "");
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
    filename_rules: &[FilenameRule],
    border_enabled: bool,
    border_color: &str,
    scale_mode: &str,
    dpi: u32,
) -> Result<Vec<String>> {
    use printpdf::*;

    let mut doc = PdfDocument::new("image_paddler");
    let mut result_warnings = Vec::new();
    let per_page = grid.rows * grid.cols;
    let needs_external_filename_font = show_filename
        && images.iter().any(|image| {
            !display_filename(
                &image.path,
                filename_without_ext,
                filename_remove_text,
                filename_rules,
            )
            .is_ascii()
        });
    let filename_font = if needs_external_filename_font {
        load_pdf_filename_font(&mut doc).map(PdfFontHandle::External)
    } else {
        Some(PdfFontHandle::Builtin(BuiltinFont::Helvetica))
    };
    let omit_filenames = show_filename && filename_font.is_none();
    if omit_filenames {
        result_warnings.push(
            "PDF 中的图片文件名包含中文或其他非 ASCII 字符，但未找到可嵌入的 CJK 字体，已省略 PDF 中的文件名。请安装 PingFang、微软雅黑或思源黑体后重新生成。"
                .into(),
        );
    }

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

            if show_filename && !omit_filenames {
                let lines = display_filename_lines(
                    &img_info.path,
                    filename_without_ext,
                    filename_remove_text,
                    filename_rules,
                    cell_w_mm,
                );
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: filename_font
                        .as_ref()
                        .expect("未省略文件名时必须存在 PDF 字体")
                        .clone(),
                    size: Pt(FILENAME_FONT_PT as f32),
                });
                ops.push(Op::SetFillColor {
                    col: Color::Rgb(Rgb::new(0.2, 0.2, 0.2, None)),
                });
                for (line_idx, line) in lines.iter().enumerate() {
                    let line_w_pt = name_units(line) as f64 * FILENAME_FONT_PT * 0.56;
                    let text_x_pt =
                        cell_x_mm * 72.0 / 25.4 + ((cell_w_pt - line_w_pt) / 2.0).max(0.0);
                    let text_y_pt =
                        (cell_y_mm + 1.4 + (lines.len() - line_idx - 1) as f64 * 3.6) * 72.0 / 25.4;
                    ops.push(Op::SetTextCursor {
                        pos: Point {
                            x: Pt(text_x_pt as f32),
                            y: Pt(text_y_pt as f32),
                        },
                    });
                    ops.push(Op::ShowText {
                        items: vec![TextItem::Text(line.clone())],
                    });
                }
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

    let bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
    std::fs::write(output_path, &bytes)?;
    Ok(result_warnings)
}

fn load_pdf_filename_font(doc: &mut printpdf::PdfDocument) -> Option<printpdf::FontId> {
    for path in cjk_font_candidates() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let mut warnings = Vec::new();
        if let Some(font) = printpdf::ParsedFont::from_bytes(&bytes, 0, &mut warnings) {
            return Some(doc.add_font(&font));
        }
    }
    None
}

fn cjk_font_candidates() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "macos") {
        paths.extend([
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]);
    } else if cfg!(windows) {
        paths.extend([
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
        ]);
    } else {
        paths.extend([
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
        ]);
    }
    paths.into_iter().map(std::path::PathBuf::from).collect()
}

fn pdf_border_color(color: &str) -> printpdf::Color {
    let (r, g, b) = border_rgb(color);
    printpdf::Color::Rgb(printpdf::Rgb::new(r, g, b, None))
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
    filename_rules: &[FilenameRule],
    border_enabled: bool,
    border_color: &str,
    scale_mode: &str,
    dpi: u32,
) -> Result<()> {
    use docx_rs::{
        AlignmentType, Docx, HeightRule, PageMargin, PageOrientationType, Paragraph, Pic, Run,
        Table, TableAlignmentType, TableCell, TableCellBorder, TableCellBorderPosition,
        TableCellBorders, TableCellMargins, TableLayoutType, TableRow, VAlignType, WidthType,
    };

    let per_page = grid.rows * grid.cols;
    let page_w_twips = mm_to_twips(page_w_mm) as u32;
    let page_h_twips = mm_to_twips(page_h_mm) as u32;
    let margin_l_twips = mm_to_twips(margin_mm);
    let margin_r_twips = margin_l_twips;
    let margin_t_twips = margin_l_twips;
    let margin_b_twips = margin_l_twips;
    let usable_w_twips =
        (page_w_twips as i32).saturating_sub(margin_l_twips + margin_r_twips) as usize;
    let usable_h_twips =
        (page_h_twips as i32).saturating_sub(margin_t_twips + margin_b_twips) as usize;
    let docx_usable_h_twips =
        usable_h_twips.saturating_sub(mm_to_twips(DOCX_TRAILING_GAP_MM).max(0) as usize);
    let cell_w_twips = (usable_w_twips / grid.cols).max(1);
    let cell_h_twips = (docx_usable_h_twips / grid.rows).max(1);
    let page_orientation = if page_w_mm > page_h_mm {
        PageOrientationType::Landscape
    } else {
        PageOrientationType::Portrait
    };
    let page_margin = PageMargin {
        top: margin_t_twips,
        right: margin_r_twips,
        bottom: margin_b_twips,
        left: margin_l_twips,
        header: 0,
        footer: 0,
        gutter: 0,
    };
    let table_margins = TableCellMargins::new().margin(0, 0, 0, 0);
    let border_hex = docx_border_color(border_color);
    let cell_borders = || {
        TableCellBorders::with_empty()
            .set(
                TableCellBorder::new(TableCellBorderPosition::Top)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableCellBorder::new(TableCellBorderPosition::Left)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableCellBorder::new(TableCellBorderPosition::Bottom)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableCellBorder::new(TableCellBorderPosition::Right)
                    .size(8)
                    .color(border_hex),
            )
    };

    let mut doc = Docx::new()
        .page_size(page_w_twips, page_h_twips)
        .page_orient(page_orientation)
        .page_margin(page_margin);

    let cell_w_pt = cell_w_mm * 72.0 / 25.4;
    let cell_h_pt = image_cell_h_mm * 72.0 / 25.4;
    for chunk in images.chunks(per_page) {
        let mut rows = Vec::with_capacity(grid.rows);
        for row_idx in 0..grid.rows {
            let mut cells = Vec::with_capacity(grid.cols);
            for col_idx in 0..grid.cols {
                let idx = row_idx * grid.cols + col_idx;
                let mut cell = TableCell::new()
                    .width(cell_w_twips, WidthType::Dxa)
                    .vertical_align(VAlignType::Center);
                cell = if border_enabled {
                    cell.set_borders(cell_borders())
                } else {
                    cell.clear_all_border()
                };

                if let Some(img_info) = chunk.get(idx) {
                    let (draw_w_pt, draw_h_pt, _, _) = compute_placement(
                        img_info.width,
                        img_info.height,
                        cell_w_pt,
                        cell_h_pt,
                        scale_mode,
                        dpi,
                    );
                    let (png_data, width_px, height_px) = image_as_png(&img_info.path)?;
                    let pic = Pic::new_with_dimensions(png_data, width_px, height_px)
                        .size(pt_to_emu(draw_w_pt), pt_to_emu(draw_h_pt));
                    cell = cell.add_paragraph(
                        Paragraph::new()
                            .align(AlignmentType::Center)
                            .add_run(Run::new().add_image(pic)),
                    );
                    let filename_lines = display_filename_lines(
                        &img_info.path,
                        filename_without_ext,
                        filename_remove_text,
                        filename_rules,
                        cell_w_mm,
                    );
                    if show_filename {
                        for line in filename_lines {
                            cell = cell.add_paragraph(
                                Paragraph::new()
                                    .align(AlignmentType::Center)
                                    .add_run(Run::new().size(16).add_text(line)),
                            );
                        }
                    }
                } else {
                    cell = cell.add_paragraph(Paragraph::new());
                }
                cells.push(cell);
            }
            rows.push(
                TableRow::new(cells)
                    .row_height(cell_h_twips as f32)
                    .height_rule(HeightRule::Exact)
                    .cant_split(),
            );
        }

        let table = Table::without_borders(rows)
            .set_grid(vec![cell_w_twips; grid.cols])
            .width(usable_w_twips, WidthType::Dxa)
            .layout(TableLayoutType::Fixed)
            .align(TableAlignmentType::Center)
            .margins(table_margins.clone());
        doc = doc.add_table(table);
    }

    let file = std::fs::File::create(output_path)?;
    doc.build().pack(file)?;
    Ok(())
}

fn docx_border_color(color: &str) -> &'static str {
    match color {
        "white" => "FFFFFF",
        "dark_gray" => "4B5563",
        "light_gray" => "D1D5DB",
        "red" => "DC2626",
        "yellow" => "D97706",
        "blue" => "2563EB",
        _ => "000000",
    }
}

fn border_rgb(color: &str) -> (f32, f32, f32) {
    match color {
        "white" => (1.0, 1.0, 1.0),
        "dark_gray" => (0.294, 0.333, 0.388),
        "light_gray" => (0.82, 0.835, 0.859),
        "red" => (0.863, 0.149, 0.149),
        "yellow" => (0.851, 0.467, 0.024),
        "blue" => (0.145, 0.388, 0.922),
        _ => (0.0, 0.0, 0.0),
    }
}

fn mm_to_twips(mm: f64) -> i32 {
    (mm * 1440.0 / 25.4).round().max(0.0) as i32
}

fn pt_to_emu(pt: f64) -> u32 {
    (pt * 914400.0 / 72.0).round().max(1.0) as u32
}

fn image_as_png(path: &str) -> Result<(Vec<u8>, u32, u32)> {
    let img = ::image::open(path).map_err(|e| anyhow::anyhow!("{}", e))?;
    let width = img.width();
    let height = img.height();
    let mut cursor = std::io::Cursor::new(Vec::new());
    img.write_to(&mut cursor, ::image::ImageFormat::Png)?;
    Ok((cursor.into_inner(), width, height))
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
    fn legacy_two_image_layout_matches_preview_stacked_order() {
        let g = parse_layout("2", None, None);
        assert_eq!(g.rows, 2);
        assert_eq!(g.cols, 1);
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
        v.sort_by(|a, b| natural_cmp(a, b));
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
            filename_rules: None,
            order_mode: Some("z".into()),
            border_enabled: Some(true),
            border_color: Some("dark_gray".into()),
            output_mode: None,
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
        archive.by_name("docProps/app.xml").unwrap();
        archive.by_name("docProps/core.xml").unwrap();
        archive.by_name("word/_rels/document.xml.rels").unwrap();
        archive.by_name("word/styles.xml").unwrap();
        archive.by_name("word/settings.xml").unwrap();
        archive.by_name("word/fontTable.xml").unwrap();
        archive.by_name("word/media/rIdImage1.png").unwrap();

        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();
        assert!(document_xml.contains("<wp:docPr"));
        assert!(document_xml.contains("<w:tcBorders>"));
        assert!(document_xml.contains("4B5563"));
        assert!(!document_xml.contains("w:type=\"page\""));
        assert!(document_xml.contains(">evidence_frame_0001<"));
        assert!(document_xml.contains("<w:pgSz"));
        assert!(document_xml.contains("<w:pgMar"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn placement_fit_and_original_have_distinct_print_sizes() {
        let cell_w_pt = 180.0 * 72.0 / 25.4;
        let cell_h_pt = 120.0 * 72.0 / 25.4;
        let (fit_w, fit_h, _, _) = compute_placement(1920, 1080, cell_w_pt, cell_h_pt, "fit", 300);
        let (original_w, original_h, _, _) =
            compute_placement(1920, 1080, cell_w_pt, cell_h_pt, "original", 300);
        assert!(fit_w > original_w);
        assert!(fit_h > original_h);
    }

    #[test]
    fn filename_rules_keep_time_and_number_with_custom_name() {
        let rules = vec![FilenameRule {
            kind: "keep".into(),
            value: String::new(),
            replacement: "证据截图".into(),
            keep_number: Some(true),
            keep_time: Some(true),
            keep_text: Some(false),
            separator: Some("_".into()),
        }];
        let name = display_filename("/tmp/video_clip_00_01_23_frame_0042.png", true, "", &rules);
        assert_eq!(name, "证据截图_00_01_23_0042");
    }

    #[test]
    fn wraps_long_filename_to_two_lines_with_ellipsis() {
        let lines = wrap_filename_lines(
            "这是一个非常非常长的证据截图文件名_00_01_23_frame_0042",
            35.0,
            2,
        );
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'));
    }
}
