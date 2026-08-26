use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::sort_utils::natural_cmp;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff"];

const A4_WIDTH_MM: f64 = 210.0;
const A4_HEIGHT_MM: f64 = 297.0;
const DEFAULT_FILENAME_FONT_PT: f64 = 8.0;
const FILENAME_MAX_LINES: usize = 3;
const DOCX_TRAILING_GAP_MM: f64 = 2.0;
const DOCX_FILENAME_SAFETY_MM: f64 = 2.0;
const PDF_FILENAME_SAFETY_MM: f64 = 0.6;

static TIME_PART_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\d{1,2}[:：_-]\d{2}(?:[:：_-]\d{2})?|\d+(?:\.\d+)?s|\d+m\d+s").unwrap()
});
static NUMBER_PART_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());
static OUTPUT_STEM_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r"(?i)(?:[_-]\d{2}[_-]\d{2}[_-]\d{2}[_-]\d{3})?[_-](?:frame|img|image)[_-]?\d+(?:[_-]\d+)?$",
        )
        .unwrap()
});

#[derive(Debug, Deserialize)]
pub struct AnalyzeArgs {
    pub folder: String,
    #[serde(default)]
    pub folders: Option<Vec<String>>,
    #[serde(default)]
    pub image_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResult {
    pub images: Vec<ImageInfo>,
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
    #[serde(default)]
    pub image_paths: Option<Vec<String>>,
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
    pub filename_font_family: Option<String>,
    #[serde(default)]
    pub filename_font_size_pt: Option<f64>,
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
    #[serde(default)]
    pub output_stem: Option<String>,
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

#[derive(Clone)]
struct LayoutGrid {
    rows: usize,
    cols: usize,
}

/// 布局配置，聚合 generate_pdf / generate_docx 的所有排版参数。
#[derive(Clone)]
struct LayoutConfig {
    page_w_mm: f64,
    page_h_mm: f64,
    margin_mm: f64,
    grid: LayoutGrid,
    cell_w_mm: f64,
    image_cell_h_mm: f64,
    filename_reserve_mm: f64,
    show_filename: bool,
    filename_without_ext: bool,
    filename_font_family: String,
    filename_font_size_pt: f64,
    filename_max_lines: usize,
    filename_safety_mm: f64,
    filename_remove_text: String,
    filename_rules: Vec<FilenameRule>,
    border_enabled: bool,
    border_color: String,
    scale_mode: String,
    dpi: u32,
}

fn layout_for_page(config: &LayoutConfig, images: &[ImageInfo]) -> LayoutConfig {
    let image_count = images.len();
    let capacity = config.grid.rows * config.grid.cols;
    let mut page = config.clone();
    let compact_grid = if image_count == 0 || image_count >= capacity {
        page.grid.clone()
    } else {
        compact_grid_for_count(&page.grid, image_count)
    };
    let usable_width = page.cell_w_mm * page.grid.cols as f64;
    let usable_height = (page.image_cell_h_mm + page.filename_reserve_mm) * page.grid.rows as f64;
    page.cell_w_mm = usable_width / compact_grid.cols as f64;
    page.filename_max_lines = filename_lines_for_images(
        images,
        page.filename_without_ext,
        &page.filename_remove_text,
        &page.filename_rules,
        page.cell_w_mm,
        page.filename_font_size_pt,
    );
    page.filename_reserve_mm = if page.show_filename {
        filename_line_height_mm(page.filename_font_size_pt) * page.filename_max_lines as f64
            + page.filename_safety_mm
    } else {
        0.0
    };
    page.image_cell_h_mm =
        (usable_height / compact_grid.rows as f64 - page.filename_reserve_mm).max(1.0);
    page.grid = compact_grid;
    page
}

fn compact_grid_for_count(base: &LayoutGrid, count: usize) -> LayoutGrid {
    if count <= 1 {
        return LayoutGrid { rows: 1, cols: 1 };
    }
    let target_ratio = base.cols as f64 / base.rows as f64;
    let mut best = LayoutGrid {
        rows: 1,
        cols: count,
    };
    let mut best_score = ((best.cols as f64 / best.rows as f64).ln() - target_ratio.ln()).abs();
    for rows in 1..=count {
        if !count.is_multiple_of(rows) {
            continue;
        }
        let cols = count / rows;
        let score = ((cols as f64 / rows as f64).ln() - target_ratio.ln()).abs();
        if score < best_score {
            best = LayoutGrid { rows, cols };
            best_score = score;
        }
    }
    best
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
    let images = if let Some(paths) = args.image_paths.as_ref().filter(|paths| !paths.is_empty()) {
        paths
            .iter()
            .filter_map(|path| image_info_from_path(Path::new(path)).ok().flatten())
            .collect()
    } else {
        scan_image_folders(&args.folder, &args.folders)?
    };
    let recommended = recommend_settings(&images);

    Ok(AnalyzeResult {
        images,
        recommended,
    })
}

pub fn run(args: &RunArgs) -> Result<RunResult> {
    if let Some(paths) = explicit_image_paths(args) {
        if args.output_mode.as_deref() == Some("per_folder") {
            return run_explicit_images_per_folder(args, paths);
        }
        let first = paths
            .first()
            .cloned()
            .unwrap_or_else(|| args.folder.clone());
        let images = paths
            .iter()
            .filter_map(|path| image_info_from_path(Path::new(path)).ok().flatten())
            .collect::<Vec<_>>();
        return run_images(args, images, &image_output_dir(&first));
    }
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
    let first_source = sources
        .first()
        .cloned()
        .unwrap_or_else(|| args.folder.clone());
    run_images(args, images, &image_output_dir(&first_source))
}

fn explicit_image_paths(args: &RunArgs) -> Option<Vec<String>> {
    let paths = args
        .image_paths
        .as_ref()?
        .iter()
        .filter(|path| !path.trim().is_empty() && Path::new(path).is_file())
        .cloned()
        .collect::<Vec<_>>();
    (!paths.is_empty()).then_some(paths)
}

fn run_explicit_images_per_folder(args: &RunArgs, paths: Vec<String>) -> Result<RunResult> {
    let mut groups: Vec<(PathBuf, Vec<ImageInfo>)> = Vec::new();
    for path in paths {
        let source = Path::new(&path);
        let parent = source
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let Some(info) = image_info_from_path(source)? else {
            continue;
        };
        if let Some((_, images)) = groups.iter_mut().find(|(folder, _)| *folder == parent) {
            images.push(info);
        } else {
            groups.push((parent, vec![info]));
        }
    }
    let mut outputs = Vec::new();
    let mut pages = 0_u32;
    let mut image_count = 0_u32;
    let mut warnings = Vec::new();
    for (folder, images) in groups {
        let result = run_images(args, images, &folder.join("_docsy_image_out"))?;
        pages = pages.saturating_add(result.pages);
        image_count = image_count.saturating_add(result.images);
        warnings.extend(result.warnings);
        outputs.push(result.output_path);
    }
    let Some(output_path) = outputs.first().cloned() else {
        anyhow::bail!("显式图片列表中没有可排版的图片");
    };
    Ok(RunResult {
        output_path,
        output_paths: outputs,
        pages,
        images: image_count,
        warnings,
    })
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
    let margin_mm = args.margin_mm.unwrap_or(12.0);
    let show_filename = args.show_filename.unwrap_or(true);
    let filename_without_ext = args.filename_without_ext.unwrap_or(false);
    let filename_font_family = normalize_filename_font_family(args.filename_font_family.as_deref());
    let filename_font_size_pt = args
        .filename_font_size_pt
        .unwrap_or(DEFAULT_FILENAME_FONT_PT)
        .clamp(6.0, 24.0);
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
    let filename_safety_mm = if args.output_format == "docx" {
        DOCX_FILENAME_SAFETY_MM
    } else {
        PDF_FILENAME_SAFETY_MM
    };
    let filename_max_lines = filename_lines_for_images(
        &images,
        filename_without_ext,
        &filename_remove_text,
        &filename_rules,
        cell_w,
        filename_font_size_pt,
    );
    let filename_reserve = if show_filename {
        filename_line_height_mm(filename_font_size_pt) * filename_max_lines as f64
            + filename_safety_mm
    } else {
        0.0
    };
    let image_cell_h = cell_h - filename_reserve;

    let total_pages = images.len().div_ceil(per_page);
    reorder_images(&mut images, &grid, order_mode);

    let config = LayoutConfig {
        page_w_mm: page_w,
        page_h_mm: page_h,
        margin_mm,
        grid,
        cell_w_mm: cell_w,
        image_cell_h_mm: image_cell_h,
        filename_reserve_mm: filename_reserve,
        show_filename,
        filename_without_ext,
        filename_font_family,
        filename_font_size_pt,
        filename_max_lines,
        filename_safety_mm,
        filename_remove_text,
        filename_rules,
        border_enabled,
        border_color: border_color.to_string(),
        scale_mode: args.scale_mode.clone(),
        dpi: args.dpi,
    };

    std::fs::create_dir_all(output_dir)?;
    let ext = if args.output_format == "pdf" {
        "pdf"
    } else {
        "docx"
    };
    let output_stem = args
        .output_stem
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(sanitize_output_name)
        .unwrap_or_else(|| output_file_stem(&images));
    let output_path = unique_output_path(output_dir, &format!("{output_stem}_docsy_paddler"), ext);

    let warnings = match args.output_format.as_str() {
        "pdf" => generate_pdf(&images, &output_path, &config)?,
        _ => {
            generate_docx(&images, &output_path, &config)?;
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
    font_size_pt: f64,
    max_lines: usize,
) -> Vec<String> {
    wrap_filename_lines(
        &display_filename(path, without_ext, remove_text, rules),
        cell_w_mm,
        max_lines,
        font_size_pt,
    )
}

fn filename_lines_for_images(
    images: &[ImageInfo],
    without_ext: bool,
    remove_text: &str,
    rules: &[FilenameRule],
    cell_w_mm: f64,
    font_size_pt: f64,
) -> usize {
    images
        .iter()
        .map(|image| {
            let name = display_filename(&image.path, without_ext, remove_text, rules);
            required_filename_lines(&name, cell_w_mm, font_size_pt)
        })
        .max()
        .unwrap_or(1)
}

fn required_filename_lines(name: &str, cell_w_mm: f64, font_size_pt: f64) -> usize {
    let max_units = filename_max_units(cell_w_mm, font_size_pt);
    name_units(name)
        .div_ceil(max_units)
        .clamp(1, FILENAME_MAX_LINES)
}

fn filename_max_units(cell_w_mm: f64, font_size_pt: f64) -> usize {
    ((cell_w_mm * 72.0 / 25.4) / (font_size_pt.clamp(6.0, 24.0) * 0.56))
        .floor()
        .max(6.0) as usize
}

fn wrap_filename_lines(
    name: &str,
    cell_w_mm: f64,
    max_lines: usize,
    font_size_pt: f64,
) -> Vec<String> {
    let max_units = filename_max_units(cell_w_mm, font_size_pt);
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

fn filename_line_height_mm(font_size_pt: f64) -> f64 {
    font_size_pt.clamp(6.0, 24.0) * 25.4 / 72.0 * 1.32 + 0.45
}

fn normalize_filename_font_family(value: Option<&str>) -> String {
    match value {
        Some(value @ ("serif" | "kaiti" | "fangsong")) => value.to_string(),
        _ => "sans".to_string(),
    }
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
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let value = value.trim().trim_end_matches(['.', ' ']);
    if value.is_empty() {
        "images".into()
    } else {
        value.to_string()
    }
}

// 注意：本函数有意保留本地版本，未收敛到 crate::util::fs::unique_output_path ——
// 撞名序号格式为 `{stem}_{N}`（下划线），有测试依赖该命名（见本文件 tests：
// `evidence_clip_docsy_paddler_2.docx`）；通用版本使用 `{stem}-{N}`。
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

fn generate_pdf(
    images: &[ImageInfo],
    output_path: &Path,
    config: &LayoutConfig,
) -> Result<Vec<String>> {
    use printpdf::*;

    let mut doc = PdfDocument::new("image_paddler");
    let mut result_warnings = Vec::new();
    let per_page = config.grid.rows * config.grid.cols;
    let has_non_ascii_filename = config.show_filename
        && images.iter().any(|image| {
            !display_filename(
                &image.path,
                config.filename_without_ext,
                &config.filename_remove_text,
                &config.filename_rules,
            )
            .is_ascii()
        });
    let prefers_external_font = has_non_ascii_filename || config.filename_font_family != "sans";
    let filename_font = if prefers_external_font {
        load_pdf_filename_font(&mut doc, &config.filename_font_family)
            .map(PdfFontHandle::External)
            .or_else(|| {
                (!has_non_ascii_filename)
                    .then(|| PdfFontHandle::Builtin(pdf_builtin_font(&config.filename_font_family)))
            })
    } else {
        Some(PdfFontHandle::Builtin(BuiltinFont::Helvetica))
    };
    let omit_filenames = config.show_filename && filename_font.is_none();
    if omit_filenames {
        result_warnings.push(
            "PDF 中的图片文件名包含中文或其他非 ASCII 字符，但未找到可嵌入的 CJK 字体，已省略 PDF 中的文件名。请安装 PingFang、微软雅黑或思源黑体后重新生成。"
                .into(),
        );
    }

    for (page_idx, chunk) in images.chunks(per_page).enumerate() {
        let page_config = layout_for_page(config, chunk);
        let config = &page_config;
        let mut ops: Vec<Op> = Vec::new();

        for (i, img_info) in chunk.iter().enumerate() {
            let row = i / config.grid.cols;
            let col = i % config.grid.cols;

            let cell_x_mm = config.margin_mm + col as f64 * config.cell_w_mm;
            let cell_y_mm = config.page_h_mm
                - config.margin_mm
                - (row as f64 + 1.0) * (config.image_cell_h_mm + config.filename_reserve_mm);
            let image_area_y_mm = cell_y_mm + config.filename_reserve_mm;
            if config.border_enabled {
                let rect_x_pt = cell_x_mm * 72.0 / 25.4;
                let rect_y_pt = cell_y_mm * 72.0 / 25.4;
                let rect_w_pt = config.cell_w_mm * 72.0 / 25.4;
                let rect_h_pt = (config.image_cell_h_mm + config.filename_reserve_mm) * 72.0 / 25.4;
                ops.push(Op::SetOutlineColor {
                    col: pdf_border_color(&config.border_color),
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

            let cell_w_pt = config.cell_w_mm * 72.0 / 25.4;
            let cell_h_pt = config.image_cell_h_mm * 72.0 / 25.4;

            let (draw_w_pt, draw_h_pt, _nw, _nh) = compute_placement(
                img_info.width,
                img_info.height,
                cell_w_pt,
                cell_h_pt,
                &config.scale_mode,
                config.dpi,
            );
            let (target_width_px, target_height_px) =
                target_pixel_size(draw_w_pt, draw_h_pt, config.dpi);
            let img = image_for_output(&img_info.path, target_width_px, target_height_px)?;
            let encoded_width = img.width();
            let raw_image =
                RawImage::from_dynamic_image(img).map_err(|e| anyhow::anyhow!("{}", e))?;
            let xobj_id = doc.add_image(&raw_image);

            let offset_x_pt = (cell_w_pt - draw_w_pt) / 2.0;
            let offset_y_pt = (cell_h_pt - draw_h_pt) / 2.0;

            let base_x_pt = cell_x_mm * 72.0 / 25.4 + offset_x_pt;
            let base_y_pt = image_area_y_mm * 72.0 / 25.4 + offset_y_pt;

            let scale_factor = draw_w_pt / (encoded_width as f64 * 72.0 / config.dpi as f64);

            ops.push(Op::SaveGraphicsState);
            ops.push(Op::UseXobject {
                id: xobj_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(base_x_pt as f32)),
                    translate_y: Some(Pt(base_y_pt as f32)),
                    scale_x: Some(scale_factor as f32),
                    scale_y: Some(scale_factor as f32),
                    dpi: Some(config.dpi as f32),
                    ..Default::default()
                },
            });
            ops.push(Op::RestoreGraphicsState);

            if config.show_filename && !omit_filenames {
                let lines = display_filename_lines(
                    &img_info.path,
                    config.filename_without_ext,
                    &config.filename_remove_text,
                    &config.filename_rules,
                    config.cell_w_mm,
                    config.filename_font_size_pt,
                    config.filename_max_lines,
                );
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: filename_font
                        .as_ref()
                        .expect("未省略文件名时必须存在 PDF 字体")
                        .clone(),
                    size: Pt(config.filename_font_size_pt as f32),
                });
                ops.push(Op::SetFillColor {
                    col: Color::Rgb(Rgb::new(0.2, 0.2, 0.2, None)),
                });
                for (line_idx, line) in lines.iter().enumerate() {
                    let line_w_pt = name_units(line) as f64 * config.filename_font_size_pt * 0.56;
                    let text_x_pt =
                        cell_x_mm * 72.0 / 25.4 + ((cell_w_pt - line_w_pt) / 2.0).max(0.0);
                    let text_y_pt = (cell_y_mm
                        + 0.8
                        + (lines.len() - line_idx - 1) as f64
                            * filename_line_height_mm(config.filename_font_size_pt))
                        * 72.0
                        / 25.4;
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

        let page = PdfPage::new(
            Mm(config.page_w_mm as f32),
            Mm(config.page_h_mm as f32),
            ops,
        );

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

fn load_pdf_filename_font(
    doc: &mut printpdf::PdfDocument,
    family: &str,
) -> Option<printpdf::FontId> {
    for path in cjk_font_candidates(family) {
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

fn cjk_font_candidates(family: &str) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "macos") {
        match family {
            "serif" | "kaiti" | "fangsong" => paths.extend([
                "/System/Library/Fonts/Supplemental/Songti.ttc",
                "/System/Library/Fonts/STHeiti Light.ttc",
                "/System/Library/Fonts/Hiragino Sans GB.ttc",
            ]),
            _ => paths.extend([
                "/System/Library/Fonts/PingFang.ttc",
                "/System/Library/Fonts/Hiragino Sans GB.ttc",
                "/System/Library/Fonts/STHeiti Light.ttc",
            ]),
        }
        paths.push("/Library/Fonts/Arial Unicode.ttf");
    } else if cfg!(windows) {
        match family {
            "serif" => paths.extend([
                "C:\\Windows\\Fonts\\simsun.ttc",
                "C:\\Windows\\Fonts\\simfang.ttf",
            ]),
            "kaiti" => paths.extend([
                "C:\\Windows\\Fonts\\simkai.ttf",
                "C:\\Windows\\Fonts\\simsun.ttc",
            ]),
            "fangsong" => paths.extend([
                "C:\\Windows\\Fonts\\simfang.ttf",
                "C:\\Windows\\Fonts\\simsun.ttc",
            ]),
            _ => paths.extend([
                "C:\\Windows\\Fonts\\msyh.ttc",
                "C:\\Windows\\Fonts\\simhei.ttf",
            ]),
        }
        paths.push("C:\\Windows\\Fonts\\simsun.ttc");
    } else {
        if family == "sans" {
            paths.extend([
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            ]);
        }
        paths.extend([
            "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ]);
    }
    paths.into_iter().map(std::path::PathBuf::from).collect()
}

fn pdf_builtin_font(family: &str) -> printpdf::BuiltinFont {
    if family == "sans" {
        printpdf::BuiltinFont::Helvetica
    } else {
        printpdf::BuiltinFont::TimesRoman
    }
}

fn docx_font_name(family: &str) -> &'static str {
    if cfg!(windows) {
        match family {
            "serif" => "SimSun",
            "kaiti" => "KaiTi",
            "fangsong" => "FangSong",
            _ => "Microsoft YaHei",
        }
    } else if cfg!(target_os = "macos") {
        match family {
            "serif" => "Songti SC",
            "kaiti" => "Kaiti SC",
            "fangsong" => "STFangsong",
            _ => "PingFang SC",
        }
    } else if family == "serif" || family == "kaiti" || family == "fangsong" {
        "Noto Serif CJK SC"
    } else {
        "Noto Sans CJK SC"
    }
}

fn pdf_border_color(color: &str) -> printpdf::Color {
    let (r, g, b) = border_rgb(color);
    printpdf::Color::Rgb(printpdf::Rgb::new(r, g, b, None))
}

fn generate_docx(images: &[ImageInfo], output_path: &Path, config: &LayoutConfig) -> Result<()> {
    use docx_rs::{
        AlignmentType, BreakType, Docx, HeightRule, LineSpacing, LineSpacingType, PageMargin,
        PageOrientationType, Paragraph, Pic, Run, RunFonts, Table, TableAlignmentType, TableBorder,
        TableBorderPosition, TableBorders, TableCell, TableCellBorder, TableCellBorderPosition,
        TableCellBorders, TableCellMargins, TableLayoutType, TableRow, VAlignType, WidthType,
    };

    let per_page = config.grid.rows * config.grid.cols;
    let page_w_twips = mm_to_twips(config.page_w_mm) as u32;
    let page_h_twips = mm_to_twips(config.page_h_mm) as u32;
    let margin_l_twips = mm_to_twips(config.margin_mm);
    let margin_r_twips = margin_l_twips;
    let margin_t_twips = margin_l_twips;
    let margin_b_twips = margin_l_twips;
    let usable_w_twips =
        (page_w_twips as i32).saturating_sub(margin_l_twips + margin_r_twips) as usize;
    let usable_h_twips =
        (page_h_twips as i32).saturating_sub(margin_t_twips + margin_b_twips) as usize;
    let docx_usable_h_twips =
        usable_h_twips.saturating_sub(mm_to_twips(DOCX_TRAILING_GAP_MM).max(0) as usize);
    let page_orientation = if config.page_w_mm > config.page_h_mm {
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
    let border_hex = docx_border_color(&config.border_color);
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
    let table_borders = || {
        TableBorders::with_empty()
            .set(
                TableBorder::new(TableBorderPosition::Top)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableBorder::new(TableBorderPosition::Left)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableBorder::new(TableBorderPosition::Bottom)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableBorder::new(TableBorderPosition::Right)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableBorder::new(TableBorderPosition::InsideH)
                    .size(8)
                    .color(border_hex),
            )
            .set(
                TableBorder::new(TableBorderPosition::InsideV)
                    .size(8)
                    .color(border_hex),
            )
    };

    let mut doc = Docx::new()
        .page_size(page_w_twips, page_h_twips)
        .page_orient(page_orientation)
        .page_margin(page_margin);

    for chunk in images.chunks(per_page) {
        let page_config = layout_for_page(config, chunk);
        let config = &page_config;
        let cell_w_twips = (usable_w_twips / config.grid.cols).max(1);
        let cell_h_twips = (docx_usable_h_twips / config.grid.rows).max(1);
        let cell_w_pt = config.cell_w_mm * 72.0 / 25.4;
        let cell_h_pt = config.image_cell_h_mm * 72.0 / 25.4;
        let mut rows = Vec::with_capacity(config.grid.rows);
        for row_idx in 0..config.grid.rows {
            let mut cells = Vec::with_capacity(config.grid.cols);
            for col_idx in 0..config.grid.cols {
                let idx = row_idx * config.grid.cols + col_idx;
                let mut cell = TableCell::new()
                    .width(cell_w_twips, WidthType::Dxa)
                    .vertical_align(VAlignType::Center);
                cell = if config.border_enabled {
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
                        &config.scale_mode,
                        config.dpi,
                    );
                    let (target_width_px, target_height_px) =
                        target_pixel_size(draw_w_pt, draw_h_pt, config.dpi);
                    let (png_data, width_px, height_px) =
                        image_as_png(&img_info.path, target_width_px, target_height_px)?;
                    let pic = Pic::new_with_dimensions(png_data, width_px, height_px)
                        .size(pt_to_emu(draw_w_pt), pt_to_emu(draw_h_pt));
                    cell = cell.add_paragraph(
                        Paragraph::new()
                            .align(AlignmentType::Center)
                            .line_spacing(LineSpacing::new().before(0).after(0))
                            .add_run(Run::new().add_image(pic)),
                    );
                    let filename_lines = display_filename_lines(
                        &img_info.path,
                        config.filename_without_ext,
                        &config.filename_remove_text,
                        &config.filename_rules,
                        config.cell_w_mm,
                        config.filename_font_size_pt,
                        config.filename_max_lines,
                    );
                    if config.show_filename {
                        let font_name = docx_font_name(&config.filename_font_family);
                        let mut filename_run = Run::new()
                            .fonts(
                                RunFonts::new()
                                    .ascii(font_name)
                                    .hi_ansi(font_name)
                                    .east_asia(font_name)
                                    .cs(font_name),
                            )
                            .size((config.filename_font_size_pt * 2.0).round() as usize);
                        for (line_idx, line) in filename_lines.into_iter().enumerate() {
                            if line_idx > 0 {
                                filename_run = filename_run.add_break(BreakType::TextWrapping);
                            }
                            filename_run = filename_run.add_text(line);
                        }
                        let filename_line_twips =
                            mm_to_twips(filename_line_height_mm(config.filename_font_size_pt));
                        cell = cell.add_paragraph(
                            Paragraph::new()
                                .align(AlignmentType::Center)
                                .line_spacing(
                                    LineSpacing::new()
                                        .before(0)
                                        .after(0)
                                        .line_rule(LineSpacingType::Exact)
                                        .line(filename_line_twips),
                                )
                                .add_run(filename_run),
                        );
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

        // 同时写入表格级和单元格级边框。部分 Word/WPS 版本不会稳定显示
        // 只有 tcBorders 的固定布局表格，tblBorders 作为兼容性兜底。
        let table = if config.border_enabled {
            Table::new(rows).set_borders(table_borders())
        } else {
            Table::without_borders(rows)
        }
        .set_grid(vec![cell_w_twips; config.grid.cols])
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

fn target_pixel_size(draw_w_pt: f64, draw_h_pt: f64, dpi: u32) -> (u32, u32) {
    let dpi = dpi.clamp(72, 600) as f64;
    (
        (draw_w_pt / 72.0 * dpi).ceil().max(1.0) as u32,
        (draw_h_pt / 72.0 * dpi).ceil().max(1.0) as u32,
    )
}

fn image_for_output(
    path: &str,
    target_width: u32,
    target_height: u32,
) -> Result<::image::DynamicImage> {
    let img = ::image::open(path).map_err(|e| anyhow::anyhow!("{}", e))?;
    if img.width() <= target_width && img.height() <= target_height {
        return Ok(img);
    }
    Ok(img.resize(
        target_width.max(1),
        target_height.max(1),
        ::image::imageops::FilterType::Lanczos3,
    ))
}

fn image_as_png(path: &str, target_width: u32, target_height: u32) -> Result<(Vec<u8>, u32, u32)> {
    let source_dimensions =
        ::image::image_dimensions(path).map_err(|e| anyhow::anyhow!("{}", e))?;
    let is_png = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"));
    if is_png && source_dimensions.0 <= target_width && source_dimensions.1 <= target_height {
        return Ok((
            std::fs::read(path)?,
            source_dimensions.0,
            source_dimensions.1,
        ));
    }
    let img = image_for_output(path, target_width, target_height)?;
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
    fn test_natural_sort() {
        let mut v = vec!["img_10.jpg", "img_2.jpg", "img_1.jpg"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["img_1.jpg", "img_2.jpg", "img_10.jpg"]);
    }

    #[test]
    fn explicit_image_paths_preserve_user_order() {
        let root = std::env::temp_dir().join(format!(
            "docsy_image_order_test_{}_{}",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("img_10.png");
        let second = root.join("img_2.png");
        std::fs::write(&first, b"x").unwrap();
        std::fs::write(&second, b"x").unwrap();
        let args = RunArgs {
            folder: root.display().to_string(),
            folders: None,
            image_paths: Some(vec![
                first.display().to_string(),
                second.display().to_string(),
            ]),
            output_format: "pdf".into(),
            layout: "1".into(),
            orientation: "portrait".into(),
            dpi: 300,
            scale_mode: "fit".into(),
            custom_rows: None,
            custom_cols: None,
            margin_mm: None,
            show_filename: None,
            filename_without_ext: None,
            filename_font_family: None,
            filename_font_size_pt: None,
            filename_remove_text: None,
            filename_rules: None,
            order_mode: None,
            border_enabled: None,
            border_color: None,
            output_mode: None,
            output_stem: None,
        };
        assert_eq!(
            explicit_image_paths(&args).unwrap(),
            vec![first.display().to_string(), second.display().to_string()]
        );
        let _ = std::fs::remove_dir_all(root);
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
        let long_name_path = root.join(
            "evidence_frame_0002_with_a_filename_long_enough_to_wrap_onto_two_full_lines.png",
        );
        img.save(&long_name_path).unwrap();

        let args = RunArgs {
            folder: root.display().to_string(),
            folders: None,
            image_paths: None,
            output_format: "docx".into(),
            layout: "1x2".into(),
            orientation: "portrait".into(),
            dpi: 300,
            scale_mode: "fit".into(),
            custom_rows: None,
            custom_cols: None,
            margin_mm: Some(6.0),
            show_filename: Some(true),
            filename_without_ext: Some(true),
            filename_font_family: Some("serif".into()),
            filename_font_size_pt: Some(11.0),
            filename_remove_text: Some("_clip".into()),
            filename_rules: None,
            order_mode: Some("z".into()),
            border_enabled: Some(true),
            border_color: Some("dark_gray".into()),
            output_mode: None,
            output_stem: None,
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
        // docx-rs 的图片 rid 来自进程级全局计数器，并行测试下序号不固定，
        // 只断言存在一张 rIdImage*.png
        assert!(archive
            .file_names()
            .any(|n| n.starts_with("word/media/rIdImage") && n.ends_with(".png")));

        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();
        assert!(document_xml.contains("<wp:docPr"));
        assert!(document_xml.contains("<w:tblBorders>"));
        assert!(document_xml.contains("<w:insideH"));
        assert!(document_xml.contains("<w:insideV"));
        assert!(document_xml.contains("<w:tcBorders>"));
        assert!(document_xml.contains("4B5563"));
        assert!(document_xml.contains(docx_font_name("serif")));
        assert!(document_xml.contains("w:lineRule=\"exact\""));
        assert!(document_xml.contains("<w:br"));
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
            8.0,
        );
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'));
    }

    #[test]
    fn frame_document_name_removes_timeline_and_keeps_video_stem() {
        let images = vec![ImageInfo {
            path: "/tmp/当事人 微信记录_00_00_03_500_frame_0004.jpg".into(),
            width: 1200,
            height: 800,
            file_size: 1,
        }];

        assert_eq!(output_file_stem(&images), "当事人 微信记录");
        assert_eq!(
            sanitize_output_name("当事人 微信（原始）"),
            "当事人 微信（原始）"
        );
    }

    #[test]
    fn six_point_long_name_can_expand_to_three_lines() {
        let name = "中华人民共和国人民法院当事人微信聊天记录视频文件名较长补充说明材料_00_00_03_500_frame_0004.jpg";
        let lines = wrap_filename_lines(name, 52.0, FILENAME_MAX_LINES, 6.0);

        assert_eq!(lines.len(), 3);
        assert_eq!(lines.concat(), name);
    }

    #[test]
    fn docx_reuses_png_bytes_when_layout_does_not_need_downsampling() {
        let root = std::env::temp_dir().join(format!(
            "docsy_image_passthrough_test_{}_{}",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("evidence.png");
        let image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(160, 90, image::Rgba([28, 76, 57, 255]));
        image.save(&path).unwrap();
        let original = std::fs::read(&path).unwrap();

        let (embedded, width, height) = image_as_png(path.to_str().unwrap(), 320, 180).unwrap();

        assert_eq!((width, height), (160, 90));
        assert_eq!(embedded, original);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn output_image_is_downsampled_to_the_selected_print_dpi_bounds() {
        let root = std::env::temp_dir().join(format!(
            "docsy_image_downsample_test_{}_{}",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("evidence.png");
        let image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(1200, 800, image::Rgba([240, 240, 236, 255]));
        image.save(&path).unwrap();

        let resized = image_for_output(path.to_str().unwrap(), 600, 400).unwrap();

        assert_eq!((resized.width(), resized.height()), (600, 400));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn final_single_image_uses_the_full_page_cell() {
        let config = LayoutConfig {
            page_w_mm: 210.0,
            page_h_mm: 297.0,
            margin_mm: 12.0,
            grid: LayoutGrid { rows: 2, cols: 1 },
            cell_w_mm: 186.0,
            image_cell_h_mm: 128.1,
            filename_reserve_mm: 8.4,
            show_filename: true,
            filename_without_ext: false,
            filename_font_family: "sans".into(),
            filename_font_size_pt: 8.0,
            filename_max_lines: 2,
            filename_safety_mm: DOCX_FILENAME_SAFETY_MM,
            filename_remove_text: String::new(),
            filename_rules: Vec::new(),
            border_enabled: false,
            border_color: "black".into(),
            scale_mode: "fit".into(),
            dpi: 300,
        };

        let images = vec![ImageInfo {
            path: "evidence_frame_0001.png".into(),
            width: 1200,
            height: 800,
            file_size: 1,
        }];
        let page = layout_for_page(&config, &images);

        assert_eq!(page.grid.rows, 1);
        assert_eq!(page.grid.cols, 1);
        assert!(page.image_cell_h_mm > config.image_cell_h_mm * 1.9);
    }

    #[test]
    fn incomplete_final_page_uses_a_compact_grid_without_empty_cells() {
        let base = LayoutGrid { rows: 2, cols: 2 };

        let compact = compact_grid_for_count(&base, 3);

        assert_eq!(compact.rows * compact.cols, 3);
    }
}
