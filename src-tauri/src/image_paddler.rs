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
const NOTE_MAX_LINES: usize = 3;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_width_mm: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RunArgs {
    pub folder: String,
    #[serde(default)]
    pub folders: Option<Vec<String>>,
    #[serde(default)]
    pub image_paths: Option<Vec<String>>,
    #[serde(default)]
    pub output_dir: Option<String>,
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
    #[serde(default)]
    pub use_table: Option<bool>,
    #[serde(default)]
    pub fixed_width_mm: Option<f64>,
    #[serde(default)]
    pub filename_color: Option<String>,
    #[serde(default)]
    pub page_scales: Option<Vec<f64>>,
    #[serde(default)]
    pub pair_mode: Option<String>,
    #[serde(default)]
    pub caption_position: Option<String>,
    #[serde(default)]
    pub print_safety_pad_mm: Option<f64>,
    #[serde(default)]
    pub caption_gap_mm: Option<f64>,
    /// keep = 末页保留原网格；reflow = 末页按实际张数重新铺满
    #[serde(default)]
    pub last_page_mode: Option<String>,
    #[serde(default)]
    pub reserve_note_placeholder: Option<bool>,
    #[serde(default)]
    pub note_placeholder_text: Option<String>,
    #[serde(default)]
    pub note_font_family: Option<String>,
    #[serde(default)]
    pub note_font_size_pt: Option<f64>,
    #[serde(default)]
    pub note_color: Option<String>,
    #[serde(default)]
    pub image_annotations: Option<std::collections::HashMap<String, ImageAnnotation>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ImageAnnotation {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
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
    use_table: bool,
    fixed_width_mm: Option<f64>,
    filename_color: String,
    caption_position: String,
    pair_mode: String,
    #[allow(dead_code)]
    print_safety_pad_mm: f64,
    caption_gap_mm: f64,
    last_page_mode: String,
    page_scales: Vec<f64>,
    reserve_note_placeholder: bool,
    note_placeholder_text: String,
    note_font_family: String,
    note_font_size_pt: f64,
    note_color: String,
    image_annotations: std::collections::HashMap<String, ImageAnnotation>,
}

impl LayoutConfig {
    /// Safe column width in mm taking into account margin and print safety padding.
    #[inline]
    #[allow(dead_code)]
    pub fn safe_column_width_mm(&self) -> f64 {
        let cols = self.grid.cols.max(1) as f64;
        let cell_w = (self.page_w_mm - self.margin_mm * 2.0).max(1.0) / cols;
        (cell_w - self.print_safety_pad_mm).max(20.0)
    }
}

fn resolve_image_title_and_note(
    img_path: &str,
    cfg: &LayoutConfig,
) -> (Vec<String>, Vec<String>) {
    let annotation = cfg.image_annotations.get(img_path);
    let title_str = annotation
        .and_then(|a| a.title.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            display_filename(
                img_path,
                cfg.filename_without_ext,
                &cfg.filename_remove_text,
                &cfg.filename_rules,
            )
        });

    let title_lines = if cfg.show_filename && !title_str.is_empty() {
        wrap_note_lines(
            &title_str,
            cfg.cell_w_mm,
            FILENAME_MAX_LINES,
            cfg.filename_font_size_pt,
        )
    } else {
        Vec::new()
    };

    let note_str = annotation
        .and_then(|a| a.description.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            if cfg.reserve_note_placeholder {
                Some(if cfg.note_placeholder_text.trim().is_empty() {
                    "[点击输入说明]".to_string()
                } else {
                    cfg.note_placeholder_text.clone()
                })
            } else {
                None
            }
        });

    let note_lines = if let Some(desc) = note_str {
        wrap_note_lines(&desc, cfg.cell_w_mm, NOTE_MAX_LINES, cfg.note_font_size_pt)
    } else {
        Vec::new()
    };

    (title_lines, note_lines)
}

fn wrap_note_lines(
    text: &str,
    cell_w_mm: f64,
    max_lines: usize,
    font_size_pt: f64,
) -> Vec<String> {
    let mut all_lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if !trimmed.is_empty() {
            let max_sub = max_lines.saturating_sub(all_lines.len()).max(1);
            let wrapped = wrap_filename_lines(trimmed, cell_w_mm, max_sub, font_size_pt);
            for w in wrapped {
                if !w.is_empty() {
                    all_lines.push(w);
                }
                if all_lines.len() >= max_lines {
                    return all_lines;
                }
            }
        }
    }
    if all_lines.is_empty() {
        all_lines.push(text.trim().to_string());
    }
    all_lines
}

fn caption_reserve_for_page(
    images: &[ImageInfo],
    config: &LayoutConfig,
) -> (usize, usize, f64) {
    let mut max_title_lines = 1;
    let mut max_note_lines = 0;
    for img in images {
        let (t_lines, n_lines) = resolve_image_title_and_note(&img.path, config);
        max_title_lines = max_title_lines.max(t_lines.len());
        max_note_lines = max_note_lines.max(n_lines.len());
    }
    let title_h = if config.show_filename {
        filename_line_height_mm(config.filename_font_size_pt) * max_title_lines as f64
    } else {
        0.0
    };
    let note_h = if max_note_lines > 0 {
        filename_line_height_mm(config.note_font_size_pt) * max_note_lines as f64
    } else {
        0.0
    };
    let safety = if (config.show_filename && max_title_lines > 0) || max_note_lines > 0 {
        config.filename_safety_mm
    } else {
        0.0
    };
    // Keep a fixed baseline clearance. The user's gap only moves text, never images.
    let baseline_gap = if safety > 0.0 { 2.0 } else { 0.0 };
    (max_title_lines, max_note_lines, title_h + note_h + safety + baseline_gap)
}

fn layout_for_page(config: &LayoutConfig, images: &[ImageInfo]) -> LayoutConfig {
    let image_count = images.len();
    let capacity = config.grid.rows * config.grid.cols;
    let mut page = config.clone();
    let compact_grid = if image_count == 0
        || image_count >= capacity
        || page.last_page_mode != "reflow"
    {
        page.grid.clone()
    } else {
        compact_grid_for_count(&page.grid, image_count)
    };
    let usable_width = page.cell_w_mm * page.grid.cols as f64;
    let usable_height = (page.image_cell_h_mm + page.filename_reserve_mm) * page.grid.rows as f64;
    page.cell_w_mm = usable_width / compact_grid.cols as f64;
    let (max_t_lines, _max_n_lines, caption_reserve) = caption_reserve_for_page(images, &page);
    page.filename_max_lines = max_t_lines;
    page.filename_reserve_mm = caption_reserve;
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

fn scan_images_recursive(dir_path: &Path, images: &mut Vec<ImageInfo>) -> Result<()> {
    let dir = match std::fs::read_dir(dir_path) {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };

    for entry in dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if entry.file_type()?.is_symlink() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if file_name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            let _ = scan_images_recursive(&path, images);
        } else if path.is_file() {
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
    }
    Ok(())
}

fn scan_images(folder: &str) -> Result<Vec<ImageInfo>> {
    let mut images = Vec::new();
    scan_images_recursive(Path::new(folder), &mut images)?;
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
    all.dedup_by(|first, second| first.path == second.path);
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
            recommended_width_mm: None,
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

    let recommended_width_mm = if orientation == "landscape" {
        match layout {
            "1" => Some(240.0),
            "1x2" => Some(125.0),
            "2x1" => Some(180.0),
            _ => Some(120.0),
        }
    } else {
        match layout {
            "1" => Some(170.0),
            "2x1" | "2" => {
                // 上下 2 张图，根据宽高比计算高度安全的最大宽度（上限 165mm）
                let max_h = 105.0;
                let w = (max_h * aspect).clamp(120.0, 165.0);
                Some((w / 5.0).round() * 5.0) // 规整到 5 的倍数，如 160.0
            }
            "1x2" => Some(85.0),
            _ => Some(160.0),
        }
    };

    RecommendedSettings {
        orientation: orientation.into(),
        layout: layout.into(),
        dpi: 300,
        scale_mode: scale_mode.into(),
        margin_mm: 12.0,
        show_filename: true,
        reason: reason.into(),
        recommended_width_mm,
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
    if let Some(paths) = &args.image_paths {
        if paths.is_empty() || paths.iter().any(|path| !Path::new(path).is_file()) {
            anyhow::bail!("所选图片已被移除或列表为空，请重新分析素材后生成");
        }
    }
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
    let grid = parse_layout(&args.layout, args.custom_rows, args.custom_cols);
    let capacity = (grid.rows * grid.cols).max(1);
    let mut page_offset = 0;
    for (folder, images) in groups {
        crate::operations::check_current_cancelled()?;
        let count = images.len().div_ceil(capacity);
        let mut file_args = args.clone();
        file_args.page_scales = args.page_scales.as_ref().map(|scales|
            scales.iter().skip(page_offset).take(count).copied().collect());
        page_offset += count;
        let result = run_images(&file_args, images, &folder.join("_docsy_image_out"))?;
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

fn run_images(args: &RunArgs, mut images: Vec<ImageInfo>, default_output_dir: &Path) -> Result<RunResult> {
    let output_dir = match args.output_dir.as_deref().filter(|value| !value.is_empty()) {
        Some(value) => {
            let path = Path::new(value);
            if !path.is_dir() { anyhow::bail!("输出目录不存在或不可用，请重新选择：{value}"); }
            path
        }
        None => default_output_dir,
    };
    if images.is_empty() {
        anyhow::bail!("未找到图片文件");
    }
    if !["pdf", "docx"].contains(&args.output_format.as_str()) {
        anyhow::bail!("不支持的输出格式");
    }
    if args.margin_mm.is_some_and(|value| !value.is_finite() || !(0.0..=30.0).contains(&value))
        || args.filename_font_size_pt.is_some_and(|value| !value.is_finite())
        || args.fixed_width_mm.is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        anyhow::bail!("排版参数无效，请检查页边距、字号和图片宽度");
    }

    let mut grid = parse_layout(&args.layout, args.custom_rows, args.custom_cols);
    if args.output_format == "docx" && args.use_table == Some(false) {
        grid = LayoutGrid { rows: grid.rows * grid.cols, cols: 1 };
    }
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
    let border_enabled = args.border_enabled.unwrap_or(false)
        && !(args.output_format == "docx" && args.use_table == Some(false));
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
    if image_cell_h <= 1.0 {
        anyhow::bail!("当前每页张数和字号没有为图片留下足够空间，请减少张数或缩小字号");
    }

    let total_pages = images.len().div_ceil(per_page);
    reorder_images(&mut images, &grid, order_mode);

    let mut config = LayoutConfig {
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
        dpi: if args.dpi == 0 { 300 } else { args.dpi.clamp(72, 1200) },
        use_table: args.use_table.unwrap_or(true),
        fixed_width_mm: args.fixed_width_mm,
        filename_color: args
            .filename_color
            .as_deref()
            .unwrap_or("dark_gray")
            .to_string(),
        caption_position: args
            .caption_position
            .as_deref()
            .unwrap_or("below")
            .to_string(),
        pair_mode: args
            .pair_mode
            .as_deref()
            .unwrap_or("cell-center")
            .to_string(),
        print_safety_pad_mm: args.print_safety_pad_mm.unwrap_or(6.0).max(0.0),
        caption_gap_mm: args.caption_gap_mm.unwrap_or(2.0).clamp(0.0, 20.0),
        last_page_mode: args
            .last_page_mode
            .clone()
            .unwrap_or_else(|| "keep".to_string()),
        page_scales: args.page_scales.clone().unwrap_or_default(),
        reserve_note_placeholder: args.reserve_note_placeholder.unwrap_or(false),
        note_placeholder_text: args
            .note_placeholder_text
            .clone()
            .unwrap_or_else(|| "[点击输入说明]".to_string()),
        note_font_family: normalize_filename_font_family(
            args.note_font_family.as_deref().or(Some("kaiti")),
        ),
        note_font_size_pt: args.note_font_size_pt.unwrap_or(8.0).clamp(6.0, 24.0),
        note_color: args
            .note_color
            .as_deref()
            .unwrap_or("gray")
            .to_string(),
        image_annotations: args.image_annotations.clone().unwrap_or_default(),
    };

    if config.scale_mode == "fixed_width" {
        config.fixed_width_mm = Some(config.fixed_width_mm.unwrap_or(160.0).clamp(0.1, 500.0));
    }

    // Validate actual per-page titles/notes, including reflow, before creating output.
    for (index, chunk) in images.chunks(per_page).enumerate() {
        let page = layout_for_page(&config, chunk);
        let cell_height = layout_usable_h / page.grid.rows as f64;
        if page.filename_reserve_mm + 0.1 >= cell_height {
            anyhow::bail!("第 {} 页标题或说明已占满单元格，请减少每页张数或缩小字号", index + 1);
        }
    }
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

    let staging = crate::util::fs::sibling_temp_path(&output_path, "paddler");
    let _guard = crate::util::fs::TempPathGuard::new(staging.clone());
    let warnings = match args.output_format.as_str() {
        "pdf" => generate_pdf(&images, &staging, &config)?,
        _ => {
            generate_docx(&images, &staging, &config)?;
            Vec::new()
        }
    };

    crate::operations::check_current_cancelled()?;
    std::fs::rename(&staging, &output_path)?;
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

fn pair_offsets(count: usize, rows: usize, cols: usize, mode: &str) -> Vec<&'static str> {
    let mut aligns = vec!["center"; count];
    let is_stack_2 = rows == 2 && cols == 1 && count == 2;
    let is_side_2 = rows == 1 && cols == 2 && count == 2;
    if !is_stack_2 && !is_side_2 {
        return aligns;
    }
    match mode {
        "page-gather" => {
            if is_stack_2 {
                aligns[0] = "bottom";
                aligns[1] = "top";
            } else {
                aligns[0] = "right";
                aligns[1] = "left";
            }
        }
        "page-spread" | "edge-align" | "gap-max" => {
            if is_stack_2 {
                aligns[0] = "top";
                aligns[1] = "bottom";
            } else {
                aligns[0] = "left";
                aligns[1] = "right";
            }
        }
        _ => {}
    }
    aligns
}

fn compute_placement(
    img_w: u32,
    img_h: u32,
    cell_w_pt: f64,
    cell_h_pt: f64,
    scale_mode: &str,
    dpi: u32,
    fixed_width_mm: Option<f64>,
    page_scale: f64,
) -> (f64, f64, f64, f64) {
    let safe_dpi = if dpi == 0 { 300.0 } else { (dpi as f64).clamp(72.0, 1200.0) };
    let native_w_pt = img_w as f64 * 72.0 / safe_dpi;
    let native_h_pt = img_h as f64 * 72.0 / safe_dpi;

    let (draw_w_pt, draw_h_pt) = match scale_mode {
        "fixed_width" => {
            let width_mm = (fixed_width_mm.unwrap_or(160.0) * page_scale).clamp(0.1, 500.0);
            let target_w_pt = width_mm * 72.0 / 25.4;
            let ratio = if img_w > 0 { img_h as f64 / img_w as f64 } else { 1.0 };
            (target_w_pt, target_w_pt * ratio)
        }
        "original" => {
            let fit_scale = (cell_w_pt / native_w_pt).min(cell_h_pt / native_h_pt);
            let scale = fit_scale.min(1.0) * page_scale;
            (native_w_pt * scale, native_h_pt * scale)
        }
        _ => {
            let scale_x = cell_w_pt / native_w_pt;
            let scale_y = cell_h_pt / native_h_pt;
            let scale = scale_x.min(scale_y) * page_scale;
            (native_w_pt * scale, native_h_pt * scale)
        }
    };

    (
        draw_w_pt,
        draw_h_pt,
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
    let has_non_ascii_filename = images.iter().any(|image| {
        let (t, n) = resolve_image_title_and_note(&image.path, config);
        t.iter().chain(n.iter()).any(|line| !line.is_ascii())
    });
    let prefers_external_filename_font = has_non_ascii_filename
        || config.filename_font_family != "sans";
    let filename_font = if prefers_external_filename_font {
        load_pdf_filename_font(&mut doc, &config.filename_font_family)
            .map(PdfFontHandle::External)
            .or_else(|| {
                (!has_non_ascii_filename)
                    .then(|| PdfFontHandle::Builtin(pdf_builtin_font(&config.filename_font_family)))
            })
    } else {
        Some(PdfFontHandle::Builtin(BuiltinFont::Helvetica))
    };

    let prefers_external_note_font = has_non_ascii_filename
        || config.note_font_family != "sans";
    let note_font = if prefers_external_note_font {
        load_pdf_filename_font(&mut doc, &config.note_font_family)
            .map(PdfFontHandle::External)
            .or_else(|| {
                (!has_non_ascii_filename)
                    .then(|| PdfFontHandle::Builtin(pdf_builtin_font(&config.note_font_family)))
            })
            .or_else(|| filename_font.clone())
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
        let page_scale = config.page_scales.get(page_idx).copied().unwrap_or(1.0);
        let aligns = pair_offsets(chunk.len(), config.grid.rows, config.grid.cols, &config.pair_mode);
        let mut ops: Vec<Op> = Vec::new();

        if config.border_enabled {
            for slot in 0..config.grid.rows * config.grid.cols {
                let cell_x_mm = config.margin_mm + (slot % config.grid.cols) as f64 * config.cell_w_mm;
                let cell_y_mm = config.page_h_mm - config.margin_mm
                    - ((slot / config.grid.cols) as f64 + 1.0) * (config.image_cell_h_mm + config.filename_reserve_mm);

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
        }

        for (i, img_info) in chunk.iter().enumerate() {
            crate::operations::check_current_cancelled()?;
            let row = i / config.grid.cols;
            let col = i % config.grid.cols;
            let align = aligns.get(i).copied().unwrap_or("center");

            let cell_x_mm = config.margin_mm + col as f64 * config.cell_w_mm;
            let cell_y_mm = config.page_h_mm
                - config.margin_mm
                - (row as f64 + 1.0) * (config.image_cell_h_mm + config.filename_reserve_mm);
            let cell_w_pt = config.cell_w_mm * 72.0 / 25.4;
            let cell_h_pt = config.image_cell_h_mm * 72.0 / 25.4;

            let (draw_w_pt, draw_h_pt, _nw, _nh) = compute_placement(
                img_info.width,
                img_info.height,
                cell_w_pt,
                cell_h_pt,
                &config.scale_mode,
                config.dpi,
                config.fixed_width_mm,
                page_scale,
            );
            let (target_width_px, target_height_px) =
                target_pixel_size(draw_w_pt, draw_h_pt, config.dpi);
            let img = image_for_output(&img_info.path, target_width_px, target_height_px)?;
            let encoded_width = img.width();
            let raw_image =
                RawImage::from_dynamic_image(img).map_err(|e| anyhow::anyhow!("{}", e))?;
            let xobj_id = doc.add_image(&raw_image);

            let offset_x_pt = match align {
                "left" => 0.0,
                "right" => (cell_w_pt - draw_w_pt).max(0.0),
                _ => (cell_w_pt - draw_w_pt) / 2.0,
            };
            let (image_top_mm, text_top_mm) = caption_geometry(config, draw_h_pt * 25.4 / 72.0,
                caption_height(&img_info.path, config), align);
            let cell_top_mm = cell_y_mm + config.image_cell_h_mm + config.filename_reserve_mm;
            let base_x_pt = cell_x_mm * 72.0 / 25.4 + offset_x_pt;
            let base_y_pt = (cell_top_mm - image_top_mm) * 72.0 / 25.4 - draw_h_pt;
            let text_area_y_mm = cell_top_mm - text_top_mm - caption_height(&img_info.path, config);

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

            let (title_lines, note_lines) = resolve_image_title_and_note(&img_info.path, config);
            let has_title = config.show_filename && !omit_filenames && filename_font.is_some() && !title_lines.is_empty();
            let has_note = !omit_filenames && note_font.is_some() && !note_lines.is_empty();

            if has_title || has_note {
                ops.push(Op::StartTextSection);

                if has_title {
                    let font = filename_font.as_ref().unwrap();
                    ops.push(Op::SetFont {
                        font: font.clone(),
                        size: Pt(config.filename_font_size_pt as f32),
                    });
                    let (r, g, b) = text_rgb(&config.filename_color);
                    ops.push(Op::SetFillColor {
                        col: Color::Rgb(Rgb::new(r, g, b, None)),
                    });
                    let note_height_offset = if has_note {
                        note_lines.len() as f64 * filename_line_height_mm(config.note_font_size_pt)
                    } else {
                        0.0
                    };
                    for (line_idx, line) in title_lines.iter().enumerate() {
                        let line_w_pt =
                            name_units(line) as f64 * config.filename_font_size_pt * 0.56;
                        let text_x_pt = cell_x_mm * 72.0 / 25.4
                            + ((cell_w_pt - line_w_pt) / 2.0).max(0.0);
                        let line_from_bottom = title_lines.len() - line_idx - 1;
                        let text_y_pt = (text_area_y_mm
                            + 0.8
                            + note_height_offset
                            + line_from_bottom as f64
                                * filename_line_height_mm(config.filename_font_size_pt))
                            * 72.0
                            / 25.4;
                        ops.push(Op::SetTextMatrix {
                            matrix: printpdf::TextMatrix::Translate(Pt(text_x_pt as f32), Pt(text_y_pt as f32)),
                        });
                        ops.push(Op::ShowText {
                            items: vec![TextItem::Text(line.clone())],
                        });
                    }
                }

                if has_note {
                    let font = note_font.as_ref().unwrap();
                    ops.push(Op::SetFont {
                        font: font.clone(),
                        size: Pt(config.note_font_size_pt as f32),
                    });
                    let (r, g, b) = text_rgb(&config.note_color);
                    ops.push(Op::SetFillColor {
                        col: Color::Rgb(Rgb::new(r, g, b, None)),
                    });
                    for (line_idx, line) in note_lines.iter().enumerate() {
                        let line_w_pt =
                            name_units(line) as f64 * config.note_font_size_pt * 0.56;
                        let text_x_pt = cell_x_mm * 72.0 / 25.4
                            + ((cell_w_pt - line_w_pt) / 2.0).max(0.0);
                        let line_from_bottom = note_lines.len() - line_idx - 1;
                        let text_y_pt = (text_area_y_mm
                            + 0.8
                            + line_from_bottom as f64
                                * filename_line_height_mm(config.note_font_size_pt))
                            * 72.0
                            / 25.4;
                        ops.push(Op::SetTextMatrix {
                            matrix: printpdf::TextMatrix::Translate(Pt(text_x_pt as f32), Pt(text_y_pt as f32)),
                        });
                        ops.push(Op::ShowText {
                            items: vec![TextItem::Text(line.clone())],
                        });
                    }
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
            "kaiti" => paths.extend([
                "/System/Library/Fonts/Supplemental/Kaiti.ttc",
                "/System/Library/Fonts/Supplemental/Songti.ttc",
            ]),
            "fangsong" => paths.extend([
                "/System/Library/Fonts/Supplemental/STFangsong.ttf",
                "/System/Library/Fonts/Supplemental/Songti.ttc",
            ]),
            "serif" => paths.extend([
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

fn create_caption_paragraphs(
    path: &str, cfg: &LayoutConfig, align: docx_rs::AlignmentType,
    page_break: bool, keep_next: bool, before: u32, after: u32,
) -> Vec<docx_rs::Paragraph> {
    let (title, note) = resolve_image_title_and_note(path, cfg);
    let has_title = cfg.show_filename && !title.is_empty();
    let has_note = !note.is_empty();
    let mut parts = Vec::new();
    if has_title {
        let title_cfg = cfg.clone();
        if let Some(p) = create_caption_part(title, vec![], &title_cfg, align, page_break,
            has_note || keep_next, before, if has_note { 0 } else { after }) { parts.push(p); }
    }
    if has_note {
        let note_cfg = cfg.clone();
        if let Some(p) = create_caption_part(vec![], note, &note_cfg, align, page_break && !has_title,
            keep_next, if has_title { 0 } else { before }, after) { parts.push(p); }
    }
    parts
}

fn create_caption_part(
    title_lines: Vec<String>,
    note_lines: Vec<String>,
    cfg: &LayoutConfig,
    h_align: docx_rs::AlignmentType,
    page_break_before: bool,
    keep_next: bool,
    safety_before_twips: u32,
    safety_after_twips: u32,
) -> Option<docx_rs::Paragraph> {
    use docx_rs::{BreakType, LineSpacing, LineSpacingType, Paragraph, Run, RunFonts};

    let has_title = cfg.show_filename && !title_lines.is_empty();
    let has_note = !note_lines.is_empty();
    if !has_title && !has_note {
        return None;
    }

    let mut para = Paragraph::new()
        .align(h_align)
        .page_break_before(page_break_before)
        .keep_next(keep_next)
        .keep_lines(true);

    let caption_line_height_mm = if has_title {
        filename_line_height_mm(cfg.filename_font_size_pt)
    } else if has_note {
        filename_line_height_mm(cfg.note_font_size_pt)
    } else {
        filename_line_height_mm(cfg.filename_font_size_pt)
    };

    let (before_twips, after_twips) = (safety_before_twips, safety_after_twips);

    para = para.line_spacing(
        LineSpacing::new()
            .before(before_twips)
            .after(after_twips)
            .line_rule(LineSpacingType::Exact)
            .line(mm_to_twips(caption_line_height_mm)),
    );

    if has_title {
        let font_name = docx_font_name(&cfg.filename_font_family);
        let mut title_run = Run::new()
            .fonts(
                RunFonts::new()
                    .ascii(font_name)
                    .hi_ansi(font_name)
                    .east_asia(font_name)
                    .cs(font_name),
            )
            .size((cfg.filename_font_size_pt * 2.0).round() as usize)
            .color(docx_text_color(&cfg.filename_color));
        for (idx, line) in title_lines.into_iter().enumerate() {
            if idx > 0 {
                title_run = title_run.add_break(BreakType::TextWrapping);
            }
            title_run = title_run.add_text(line);
        }
        para = para.add_run(title_run);
    }

    if has_note {
        let note_font = docx_font_name(&cfg.note_font_family);
        let mut note_run = Run::new()
            .fonts(
                RunFonts::new()
                    .ascii(note_font)
                    .hi_ansi(note_font)
                    .east_asia(note_font)
                    .cs(note_font),
            )
            .size((cfg.note_font_size_pt * 2.0).round() as usize)
            .color(docx_text_color(&cfg.note_color));
        if has_title {
            note_run = note_run.add_break(BreakType::TextWrapping);
        }
        for (idx, line) in note_lines.into_iter().enumerate() {
            if idx > 0 {
                note_run = note_run.add_break(BreakType::TextWrapping);
            }
            note_run = note_run.add_text(line);
        }
        para = para.add_run(note_run);
    }

    Some(para)
}

/// Top-down coordinates inside a cell. Text spacing must not affect image placement.
fn caption_geometry(
    config: &LayoutConfig,
    draw_h_mm: f64,
    text_h_mm: f64,
    align: &str,
) -> (f64, f64) {
    let image_area_top = if config.caption_position == "above" {
        config.filename_reserve_mm
    } else {
        0.0
    };
    let spare = config.image_cell_h_mm - draw_h_mm;
    let offset = match align {
        "top" => 0.0,
        "bottom" => spare,
        _ => spare / 2.0,
    };
    let image_top = image_area_top + offset;
    let text_top = if config.caption_position == "above" {
        image_top - text_h_mm - config.caption_gap_mm
    } else {
        image_top + draw_h_mm + config.caption_gap_mm
    };
    (image_top, text_top)
}

fn caption_height(path: &str, config: &LayoutConfig) -> f64 {
    let (title, note) = resolve_image_title_and_note(path, config);
    (if config.show_filename {
        title.len() as f64 * filename_line_height_mm(config.filename_font_size_pt)
    } else {
        0.0
    }) + note.len() as f64 * filename_line_height_mm(config.note_font_size_pt)
}

/// Fixed cell height and explicit spacers keep the drawing stable when captions move.
fn docx_cell_paragraphs(
    img: &ImageInfo,
    cfg: &LayoutConfig,
    align: &str,
    page_scale: f64,
    page_break: bool,
) -> Result<Vec<docx_rs::Paragraph>> {
    use docx_rs::{AlignmentType, LineSpacing, LineSpacingType, Paragraph, Pic, Run};
    let (draw_w, draw_h, _, _) = compute_placement(
        img.width,
        img.height,
        cfg.cell_w_mm * 72.0 / 25.4,
        cfg.image_cell_h_mm * 72.0 / 25.4,
        &cfg.scale_mode,
        cfg.dpi,
        cfg.fixed_width_mm,
        page_scale,
    );
    let draw_h_mm = draw_h * 25.4 / 72.0;
    let text_h = caption_height(&img.path, cfg);
    let (image_top, text_top) = caption_geometry(cfg, draw_h_mm, text_h, align);
    let cell_h = cfg.image_cell_h_mm + cfg.filename_reserve_mm;
    let has_caption = text_h > 0.0;
    // Paragraph spacing cannot be negative. Reject a newly displaced caption rather
    // than shifting the image or silently clipping its text in Word's exact-height row.
    if has_caption && cfg.caption_gap_mm > 0.0 {
        let base_top = if cfg.caption_position == "above" {
            image_top - text_h
        } else {
            image_top + draw_h_mm
        };
        if base_top >= -0.01
            && base_top + text_h <= cell_h + 0.01
            && (text_top < -0.01 || text_top + text_h > cell_h + 0.01)
        {
            anyhow::bail!(
                "图文间距使标题或说明超出单元格，请减小间距或图片比例后生成 Word：{}",
                img.path
            );
        }
    }
    let twips = |mm: f64| mm_to_twips(mm.max(0.0)).max(0) as u32;
    let h_align = match align {
        "left" => AlignmentType::Left,
        "right" => AlignmentType::Right,
        _ => AlignmentType::Center,
    };
    let (target_w, target_h) = target_pixel_size(draw_w, draw_h, cfg.dpi);
    let (png, width, height) = image_as_png(&img.path, target_w, target_h)?;
    let pic =
        Pic::new_with_dimensions(png, width, height).size(pt_to_emu(draw_w), pt_to_emu(draw_h));
    let above = has_caption && cfg.caption_position == "above";
    let image = Paragraph::new()
        .align(h_align)
        .keep_next(has_caption && !above)
        .keep_lines(true)
        .line_spacing(
            LineSpacing::new()
                .before(0)
                .after(0)
                .line_rule(LineSpacingType::Exact)
                .line(mm_to_twips(draw_h_mm)),
        )
        .add_run(Run::new().add_image(pic));
    let captions = create_caption_paragraphs(&img.path, cfg, h_align, false, above, 0, 0);
    let mut parts = Vec::new();
    // Word collapses adjacent paragraph before/after spacing to their maximum.
    // Exact-height spacer paragraphs therefore carry all vertical coordinates.
    let spacer = |parts: &mut Vec<Paragraph>, mm: f64, keep_next: bool| {
        let height = twips(mm);
        if height > 0 {
            parts.push(
                Paragraph::new().keep_next(keep_next).line_spacing(
                    LineSpacing::new()
                        .before(0)
                        .after(0)
                        .line_rule(LineSpacingType::Exact)
                        .line(height as i32),
                ),
            );
        }
    };
    if above {
        spacer(&mut parts, text_top, true);
        parts.extend(captions);
        spacer(&mut parts, cfg.caption_gap_mm, true);
        parts.push(image);
        spacer(&mut parts, cell_h - image_top - draw_h_mm, false);
    } else {
        spacer(&mut parts, image_top, true);
        parts.push(image);
        if has_caption {
            spacer(&mut parts, cfg.caption_gap_mm, true);
            parts.extend(captions);
            spacer(&mut parts, cell_h - text_top - text_h, false);
        } else {
            spacer(&mut parts, cell_h - image_top - draw_h_mm, false);
        }
    }
    if page_break {
        parts[0] = parts[0].clone().page_break_before(true);
    }
    Ok(parts)
}

fn generate_docx(images: &[ImageInfo], output_path: &Path, config: &LayoutConfig) -> Result<()> {
    use docx_rs::{
        Docx, HeightRule, PageMargin,
        PageOrientationType, Paragraph, Table, TableAlignmentType, TableBorder,
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

    if !config.use_table {
        for (chunk_idx, chunk) in images.chunks(per_page).enumerate() {
            let page_config = layout_for_page(config, chunk);
            let cfg = &page_config;
            let page_scale = cfg.page_scales.get(chunk_idx).copied().unwrap_or(1.0);
            let aligns = pair_offsets(chunk.len(), cfg.grid.rows, cfg.grid.cols, &cfg.pair_mode);
            for (image_idx, img_info) in chunk.iter().enumerate() {
                crate::operations::check_current_cancelled()?;
                let align = aligns.get(image_idx).copied().unwrap_or("center");
                for paragraph in docx_cell_paragraphs(img_info, cfg, align, page_scale, chunk_idx > 0 && image_idx == 0)? {
                    doc = doc.add_paragraph(paragraph);
                }
            }
        }

        let mut buf = std::io::Cursor::new(Vec::new());
        doc.build().pack(&mut buf)?;
        post_process_docx(&buf.into_inner(), output_path)?;
        return Ok(());
    }

    for (chunk_idx, chunk) in images.chunks(per_page).enumerate() {
        let page_config = layout_for_page(config, chunk);
        let config = &page_config;
        let page_scale = config.page_scales.get(chunk_idx).copied().unwrap_or(1.0);
        let aligns = pair_offsets(chunk.len(), config.grid.rows, config.grid.cols, &config.pair_mode);
        let cell_w_twips = (usable_w_twips / config.grid.cols).max(1);
        let cell_h_twips = (docx_usable_h_twips / config.grid.rows).max(1);
        let mut rows = Vec::with_capacity(config.grid.rows);
        for row_idx in 0..config.grid.rows {
            let mut cells = Vec::with_capacity(config.grid.cols);
            for col_idx in 0..config.grid.cols {
                crate::operations::check_current_cancelled()?;
                let idx = row_idx * config.grid.cols + col_idx;
                let align = aligns.get(idx).copied().unwrap_or("center");
                let mut cell = TableCell::new()
                    .width(cell_w_twips, WidthType::Dxa)
                    .vertical_align(VAlignType::Top);
                cell = if config.border_enabled {
                    cell.set_borders(cell_borders())
                } else {
                    cell.clear_all_border()
                };

                if let Some(img_info) = chunk.get(idx) {
                    for paragraph in docx_cell_paragraphs(img_info, config, align, page_scale, false)? {
                        cell = cell.add_paragraph(paragraph);
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

    let mut buf = std::io::Cursor::new(Vec::new());
    doc.build().pack(&mut buf)?;
    post_process_docx(&buf.into_inner(), output_path)?;
    Ok(())
}

fn post_process_docx(raw_bytes: &[u8], output_path: &Path) -> Result<()> {
    use std::io::{Cursor, Read, Write};
    let reader = Cursor::new(raw_bytes);
    let mut archive = zip::ZipArchive::new(reader)?;
    let outfile = std::fs::File::create(output_path)?;
    let mut writer = zip::ZipWriter::new(outfile);

    let docpr_re = Regex::new(r#"<wp:docPr\b[^>]*/>"#).unwrap();
    let override_re =
        Regex::new(r#"<Override\b[^>]*PartName="/word/numbering\.xml"[^>]*/>"#).unwrap();
    let rel_re = Regex::new(r#"<Relationship\b[^>]*Target="numbering\.xml"[^>]*/>"#).unwrap();

    let mut docpr_counter = 1_000_000usize;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if name == "word/numbering.xml" {
            // Drop unreferenced numbering.xml component
            continue;
        }

        let options = zip::write::FileOptions::default()
            .compression_method(file.compression())
            .unix_permissions(file.unix_mode().unwrap_or(0o644));

        writer.start_file(&name, options)?;

        if name == "word/document.xml" {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let replaced = docpr_re.replace_all(&content, |_caps: &regex::Captures| {
                docpr_counter += 1;
                format!(
                    r#"<wp:docPr id="{}" name="Picture_{}" />"#,
                    docpr_counter,
                    docpr_counter - 1_000_000
                )
            });
            writer.write_all(replaced.as_bytes())?;
        } else if name == "[Content_Types].xml" {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let replaced = override_re.replace_all(&content, "");
            writer.write_all(replaced.as_bytes())?;
        } else if name == "word/_rels/document.xml.rels" {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let replaced = rel_re.replace_all(&content, "");
            writer.write_all(replaced.as_bytes())?;
        } else {
            std::io::copy(&mut file, &mut writer)?;
        }
    }

    writer.finish()?;
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

fn docx_text_color(color: &str) -> &'static str {
    match color {
        "black" => "000000",
        "gray" => "6B7280",
        "blue" => "2563EB",
        _ => "4B5563", // 默认深灰
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

fn text_rgb(color: &str) -> (f32, f32, f32) {
    match color {
        "black" => (0.0, 0.0, 0.0),
        "gray" => (0.42, 0.447, 0.502),
        "blue" => (0.145, 0.388, 0.922),
        _ => (0.294, 0.333, 0.388), // 默认深灰 (4B5563)
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
    fn export_matrix_preserves_page_counts_and_rejects_missing_selection() {
        let root = std::env::temp_dir().join(format!("docsy-matrix-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut paths = Vec::new();
        for index in 0..7 {
            let path = root.join(format!("image_{index}.png"));
            image::RgbaImage::from_pixel(120, 60 + index * 20, image::Rgba([100, 80, 60, 180])).save(&path).unwrap();
            paths.push(path.display().to_string());
        }
        for format in ["pdf", "docx"] {
            for layout in ["1x3", "2x1", "custom"] {
                for table in [true, false] {
                    for scale in ["fit", "fixed_width", "original"] {
                        let args: RunArgs = serde_json::from_value(serde_json::json!({
                            "folder": root, "image_paths": paths, "output_format": format,
                            "layout": layout, "orientation": "portrait", "dpi": 72,
                            "scale_mode": scale, "use_table": table, "fixed_width_mm": 40,
                            "custom_rows": 2, "custom_cols": 2, "filename_color": "dark_gray"
                        })).unwrap();
                        let result = run(&args).unwrap();
                        let expected_pages = match layout { "1x3" => 3, "2x1" => 4, _ => 2 };
                        assert_eq!(result.pages, expected_pages);
                        assert_eq!(result.images, 7);
                        if format == "pdf" {
                            let pdf = lopdf::Document::load(&result.output_path).unwrap();
                            assert_eq!(pdf.get_pages().len(), expected_pages as usize);
                        } else {
                            let mut archive = zip::ZipArchive::new(std::fs::File::open(&result.output_path).unwrap()).unwrap();
                            let mut xml = String::new();
                            archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
                            assert_eq!(xml.matches("<wp:docPr ").count(), 7);
                            assert_eq!(xml.contains("<w:tbl>"), table);
                            assert!(xml.contains("w:color w:val=\"4B5563\""));
                        }
                    }
                }
            }
        }
        let mut args: RunArgs = serde_json::from_value(serde_json::json!({
            "folder": root, "image_paths": [root.join("missing.png")], "output_format": "pdf",
            "layout": "1", "orientation": "portrait", "dpi": 72, "scale_mode": "fit"
        })).unwrap();
        assert!(run(&args).is_err());
        args.image_paths = Some(paths);
        args.margin_mm = Some(f64::NAN);
        assert!(run(&args).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

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
            output_dir: None,
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
            use_table: None,
            fixed_width_mm: None,
            filename_color: None,
            page_scales: None,
            pair_mode: None,
            caption_position: None,
            print_safety_pad_mm: None,
            caption_gap_mm: None,
            last_page_mode: None,
            reserve_note_placeholder: None,
            note_placeholder_text: None,
            note_font_family: None,
            note_font_size_pt: None,
            note_color: None,
            image_annotations: None,
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
            output_dir: None,
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
            use_table: None,
            fixed_width_mm: None,
            filename_color: None,
            page_scales: None,
            pair_mode: None,
            caption_position: None,
            print_safety_pad_mm: None,
            caption_gap_mm: None,
            last_page_mode: None,
            reserve_note_placeholder: None,
            note_placeholder_text: None,
            note_font_family: None,
            note_font_size_pt: None,
            note_color: None,
            image_annotations: None,
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
        // 验证未引用的 numbering.xml 已被移除
        assert!(archive.by_name("word/numbering.xml").is_err());
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
        // 验证 docPr 不包含硬编码的 Figure 且具备唯一 id
        assert!(!document_xml.contains(r#"name="Figure""#));
        assert!(document_xml.contains(r#"<wp:docPr id="1000001" name="Picture_1""#));
        assert!(document_xml.contains(r#"<wp:docPr id="1000002" name="Picture_2""#));
        assert!(document_xml.contains("<w:tblBorders>"));
        assert!(document_xml.contains("<w:insideH"));
        assert!(document_xml.contains("<w:insideV"));
        assert!(document_xml.contains("<w:tcBorders>"));
        assert!(document_xml.contains("4B5563"));
        assert!(document_xml.contains("w:color w:val=\"4B5563\""));
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
        let (fit_w, fit_h, _, _) = compute_placement(1920, 1080, cell_w_pt, cell_h_pt, "fit", 300, None, 1.0);
        let (original_w, original_h, _, _) =
            compute_placement(1920, 1080, cell_w_pt, cell_h_pt, "original", 300, None, 1.0);
        let (fixed_w, fixed_h, _, _) =
            compute_placement(1920, 1080, cell_w_pt, cell_h_pt, "fixed_width", 300, Some(160.0), 1.0);
        assert!(fit_w > original_w);
        assert!(fit_h > original_h);
        assert!((fixed_w - (160.0 * 72.0 / 25.4)).abs() < 0.01);
        assert!((fixed_h - (fixed_w * 1080.0 / 1920.0)).abs() < 0.01);
    }

    #[test]
    fn recursive_sources_are_deduplicated() {
        let root = std::env::temp_dir().join(format!("docsy-scan-{}", std::process::id()));
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("evidence.png");
        image::RgbImage::new(4, 4).save(&path).unwrap();
        let sources = Some(vec![root.display().to_string(), nested.display().to_string(), path.display().to_string()]);
        let images = scan_image_folders("", &sources).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].path, path.display().to_string());
        std::fs::remove_dir_all(root).unwrap();
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
            use_table: true,
            fixed_width_mm: None,
            filename_color: "dark_gray".into(),
            caption_position: "below".into(),
            pair_mode: "cell-center".into(),
            print_safety_pad_mm: 6.0,
            caption_gap_mm: 2.0,
            last_page_mode: "reflow".into(),
            page_scales: Vec::new(),
            reserve_note_placeholder: false,
            note_placeholder_text: "[点击输入说明]".into(),
            note_font_family: "kaiti".into(),
            note_font_size_pt: 8.0,
            note_color: "gray".into(),
            image_annotations: std::collections::HashMap::new(),
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

    #[test]
    fn flow_mode_docx_generates_paragraphs_without_tables() {
        let root = std::env::temp_dir().join(format!(
            "docsy_flow_test_{}_{}",
            std::process::id(),
            chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();

        let img_path1 = root.join("flow_img_1.png");
        let img_path2 = root.join("flow_img_2.png");
        let img_path3 = root.join("flow_img_3.png");
        let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(100, 100, image::Rgba([200, 200, 200, 255]));
        img.save(&img_path1).unwrap();
        img.save(&img_path2).unwrap();
        img.save(&img_path3).unwrap();

        let args = RunArgs {
            output_dir: None,
            folder: root.display().to_string(),
            folders: None,
            image_paths: None,
            output_format: "docx".into(),
            layout: "2x1".into(),
            orientation: "portrait".into(),
            dpi: 300,
            scale_mode: "fixed_width".into(),
            custom_rows: None,
            custom_cols: None,
            margin_mm: Some(15.0),
            show_filename: Some(true),
            filename_without_ext: Some(true),
            filename_font_family: Some("sans".into()),
            filename_font_size_pt: Some(9.0),
            filename_remove_text: None,
            filename_rules: None,
            order_mode: None,
            border_enabled: None,
            border_color: None,
            output_mode: None,
            output_stem: Some("flow_evidence".into()),
            use_table: Some(false),
            fixed_width_mm: Some(160.0),
            filename_color: Some("blue".into()),
            page_scales: None,
            pair_mode: None,
            caption_position: None,
            print_safety_pad_mm: None,
            caption_gap_mm: None,
            last_page_mode: None,
            reserve_note_placeholder: None,
            note_placeholder_text: None,
            note_font_family: None,
            note_font_size_pt: None,
            note_color: None,
            image_annotations: None,
        };

        let result = run(&args).unwrap();
        let file = std::fs::File::open(&result.output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("word/numbering.xml").is_err());
        let mut doc_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc_xml)
            .unwrap();

        // 核心验证：段落流式排版中严禁出现表格标签 <w:tbl>
        assert!(!doc_xml.contains("<w:tbl>"));
        assert!(!doc_xml.contains("<w:tbl "));
        // 包含图片绘图元素与唯一 docPr id，不含 Figure
        assert!(doc_xml.contains("<wp:docPr"));
        assert!(!doc_xml.contains(r#"name="Figure""#));
        assert!(doc_xml.contains(r#"<wp:docPr id="1000001" name="Picture_1""#));
        // 包含硬分页符
        assert!(doc_xml.contains("<w:pageBreakBefore"));
        assert!(doc_xml.contains("<w:keepNext"));
        // 包含文件名
        assert!(doc_xml.contains(">flow_img_1<"));
        assert!(doc_xml.contains("w:color w:val=\"2563EB\""));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_pair_offsets_and_compute_placement_with_scale() {
        let offsets_gather_2x1 = pair_offsets(2, 2, 1, "page-gather");
        assert_eq!(offsets_gather_2x1, vec!["bottom", "top"]);

        let offsets_spread_2x1 = pair_offsets(2, 2, 1, "page-spread");
        assert_eq!(offsets_spread_2x1, vec!["top", "bottom"]);

        let offsets_gather_1x2 = pair_offsets(2, 1, 2, "page-gather");
        assert_eq!(offsets_gather_1x2, vec!["right", "left"]);

        let offsets_spread_1x2 = pair_offsets(2, 1, 2, "page-spread");
        assert_eq!(offsets_spread_1x2, vec!["left", "right"]);

        let offsets_center = pair_offsets(2, 2, 1, "cell-center");
        assert_eq!(offsets_center, vec!["center", "center"]);

        // 测试 page_scale 对 fixed_width 尺度的等比缩放
        let cell_w_pt = 200.0 * 72.0 / 25.4;
        let cell_h_pt = 200.0 * 72.0 / 25.4;
        let (w_1x, _, _, _) = compute_placement(1000, 1000, cell_w_pt, cell_h_pt, "fixed_width", 300, Some(100.0), 1.0);
        let (w_1_2x, _, _, _) = compute_placement(1000, 1000, cell_w_pt, cell_h_pt, "fixed_width", 300, Some(100.0), 1.2);
        assert!((w_1_2x - w_1x * 1.2).abs() < 0.001);
    }

    #[test]
    fn test_image_annotations_and_placeholder_resolution() {
        let mut annotations = std::collections::HashMap::new();
        annotations.insert(
            "img1.png".to_string(),
            ImageAnnotation {
                title: Some("自定义标题 1".into()),
                description: Some("拍摄时间：2026-09-18\n见证人：张三".into()),
            },
        );

        let mut config = LayoutConfig {
            page_w_mm: 210.0,
            page_h_mm: 297.0,
            margin_mm: 12.0,
            grid: LayoutGrid { rows: 2, cols: 1 },
            cell_w_mm: 186.0,
            image_cell_h_mm: 120.0,
            filename_reserve_mm: 8.0,
            show_filename: true,
            filename_without_ext: false,
            filename_font_family: "sans".into(),
            filename_font_size_pt: 8.0,
            filename_max_lines: 2,
            filename_safety_mm: 2.0,
            filename_remove_text: String::new(),
            filename_rules: Vec::new(),
            border_enabled: false,
            border_color: "black".into(),
            scale_mode: "fit".into(),
            dpi: 300,
            use_table: true,
            fixed_width_mm: None,
            filename_color: "dark_gray".into(),
            caption_position: "below".into(),
            pair_mode: "cell-center".into(),
            print_safety_pad_mm: 6.0,
            caption_gap_mm: 2.0,
            last_page_mode: "keep".into(),
            page_scales: Vec::new(),
            reserve_note_placeholder: true,
            note_placeholder_text: "[点击输入说明]".into(),
            note_font_family: "kaiti".into(),
            note_font_size_pt: 8.0,
            note_color: "gray".into(),
            image_annotations: annotations,
        };

        // img1 has custom title and multiline description
        let (t1, n1) = resolve_image_title_and_note("img1.png", &config);
        assert_eq!(t1, vec!["自定义标题 1"]);
        assert_eq!(n1, vec!["拍摄时间：2026-09-18", "见证人：张三"]);

        // img2 has no annotation, but reserve_note_placeholder is true -> uses filename and placeholder
        let (t2, n2) = resolve_image_title_and_note("img2.png", &config);
        assert_eq!(t2, vec!["img2.png"]);
        assert_eq!(n2, vec!["[点击输入说明]"]);

        // When reserve_note_placeholder is false and no annotation, note is empty
        config.reserve_note_placeholder = false;
        let (_t3, n3) = resolve_image_title_and_note("img2.png", &config);
        assert!(n3.is_empty());
    }

    #[test]
    fn test_golden_fixture_wysiwyg_rectangles() {
        // Golden scenario: A4 (210 x 297 mm), margin: 12 mm, grid: 2x1
        // cell_w = 186.0 mm, cell_h = 136.5 mm
        // filename_reserve = 5.2 mm, image_cell_h = 131.3 mm
        // 2 images: 1600 x 1200, scale_mode: fixed_width (160.0 mm), page_gather
        let margin_mm = 12.0;
        let cell_w_mm = 186.0;
        let filename_reserve_mm = 5.2;
        let image_cell_h_mm = 131.3;
        let cell_w_pt = cell_w_mm * 72.0 / 25.4;
        let cell_h_pt = image_cell_h_mm * 72.0 / 25.4;

        let aligns = pair_offsets(2, 2, 1, "page-gather");
        assert_eq!(aligns, vec!["bottom", "top"]);

        let (draw_w_pt, draw_h_pt, _, _) = compute_placement(
            1600,
            1200,
            cell_w_pt,
            cell_h_pt,
            "fixed_width",
            300,
            Some(160.0),
            1.0,
        );

        let draw_w_mm = draw_w_pt * 25.4 / 72.0;
        let draw_h_mm = draw_h_pt * 25.4 / 72.0;
        assert!((draw_w_mm - 160.0).abs() < 0.01);
        assert!((draw_h_mm - 120.0).abs() < 0.01);

        // Image 0 (row 0, col 0, bottom-aligned in cell image area)
        let row0_draw_x_mm = margin_mm + (cell_w_mm - draw_w_mm) / 2.0;
        let row0_draw_y_mm = margin_mm + (image_cell_h_mm - draw_h_mm);
        assert!((row0_draw_x_mm - 25.0).abs() < 0.01);
        assert!((row0_draw_y_mm - 23.3).abs() < 0.01);

        // Image 1 (row 1, col 0, top-aligned in cell image area)
        let row1_draw_x_mm = margin_mm + (cell_w_mm - draw_w_mm) / 2.0;
        let row1_draw_y_mm = margin_mm + (image_cell_h_mm + filename_reserve_mm);
        assert!((row1_draw_x_mm - 25.0).abs() < 0.01);
        assert!((row1_draw_y_mm - 148.5).abs() < 0.01);

        // Golden Scenario 2: 2x1 page-spread with page_scale 1.25 and fixed_width 120mm
        let aligns2 = pair_offsets(2, 2, 1, "page-spread");
        assert_eq!(aligns2, vec!["top", "bottom"]);
        let (draw_w_pt2, draw_h_pt2, _, _) = compute_placement(
            1500,
            1000,
            cell_w_pt,
            cell_h_pt,
            "fixed_width",
            300,
            Some(120.0),
            1.25,
        );
        let draw_w_mm2 = draw_w_pt2 * 25.4 / 72.0;
        let draw_h_mm2 = draw_h_pt2 * 25.4 / 72.0;
        assert!((draw_w_mm2 - 150.0).abs() < 0.01);
        assert!((draw_h_mm2 - 100.0).abs() < 0.01);
        // Image 0 (row 0, top-aligned):
        let s2_img0_x = margin_mm + (cell_w_mm - draw_w_mm2) / 2.0;
        let s2_img0_y = margin_mm; // top of cell 0
        assert!((s2_img0_x - 30.0).abs() < 0.01);
        assert!((s2_img0_y - 12.0).abs() < 0.01);
        // Image 1 (row 1, bottom-aligned):
        let s2_img1_x = margin_mm + (cell_w_mm - draw_w_mm2) / 2.0;
        let s2_img1_y = margin_mm + (image_cell_h_mm + filename_reserve_mm) + (image_cell_h_mm - draw_h_mm2);
        assert!((s2_img1_x - 30.0).abs() < 0.01);
        assert!((s2_img1_y - 179.8).abs() < 0.01);

        // Golden Scenario 3: 1x2 side-by-side with caption above (cell_w = 93mm, img_h = 265mm, reserve = 8mm)
        let cell_w_s3 = 93.0;
        let img_h_s3 = 265.0;
        let reserve_s3 = 8.0;
        let aligns3 = pair_offsets(2, 1, 2, "page-gather");
        assert_eq!(aligns3, vec!["right", "left"]);
        let (draw_w_pt3, draw_h_pt3, _, _) = compute_placement(
            800,
            1200,
            cell_w_s3 * 72.0 / 25.4,
            img_h_s3 * 72.0 / 25.4,
            "fixed_width",
            300,
            Some(80.0),
            1.0,
        );
        let draw_w_mm3 = draw_w_pt3 * 25.4 / 72.0;
        let draw_h_mm3 = draw_h_pt3 * 25.4 / 72.0;
        assert!((draw_w_mm3 - 80.0).abs() < 0.01);
        assert!((draw_h_mm3 - 120.0).abs() < 0.01);
        // Image 0 (col 0, right-aligned, caption above):
        let s3_img0_x = margin_mm + (cell_w_s3 - draw_w_mm3);
        let s3_img0_y = margin_mm + reserve_s3 + (img_h_s3 - draw_h_mm3) / 2.0;
        assert!((s3_img0_x - 25.0).abs() < 0.01);
        assert!((s3_img0_y - 92.5).abs() < 0.01);
        // Image 1 (col 1, left-aligned, caption above):
        let s3_img1_x = margin_mm + cell_w_s3;
        let s3_img1_y = margin_mm + reserve_s3 + (img_h_s3 - draw_h_mm3) / 2.0;
        assert!((s3_img1_x - 105.0).abs() < 0.01);
        assert!((s3_img1_y - 92.5).abs() < 0.01);
    }

    #[test]
    fn test_layout_for_page_preserves_fixed_width_without_clamping_to_safe_column_width() {
        let config = LayoutConfig {
            page_w_mm: 210.0,
            page_h_mm: 297.0,
            margin_mm: 12.0,
            grid: LayoutGrid { rows: 2, cols: 1 },
            cell_w_mm: 186.0,
            image_cell_h_mm: 120.0,
            filename_reserve_mm: 8.0,
            show_filename: false,
            filename_without_ext: false,
            filename_font_family: "sans".into(),
            filename_font_size_pt: 8.0,
            filename_max_lines: 2,
            filename_safety_mm: 2.0,
            filename_remove_text: String::new(),
            filename_rules: Vec::new(),
            border_enabled: false,
            border_color: "black".into(),
            scale_mode: "fixed_width".into(),
            dpi: 300,
            use_table: true,
            fixed_width_mm: Some(200.0), // Requested 200mm > safe width 180mm
            filename_color: "dark_gray".into(),
            caption_position: "below".into(),
            pair_mode: "cell-center".into(),
            print_safety_pad_mm: 6.0,
            caption_gap_mm: 2.0,
            last_page_mode: "keep".into(),
            page_scales: Vec::new(),
            reserve_note_placeholder: false,
            note_placeholder_text: String::new(),
            note_font_family: "sans".into(),
            note_font_size_pt: 8.0,
            note_color: "gray".into(),
            image_annotations: std::collections::HashMap::new(),
        };

        let images = vec![
            ImageInfo {
                path: "img1.png".into(),
                width: 1000,
                height: 1000,
                file_size: 1,
            },
        ];

        let page = layout_for_page(&config, &images);
        // cell_w = 186mm, pad = 6mm -> safe_column_width = 180mm
        assert_eq!(page.safe_column_width_mm(), 180.0);
        // User requested fixed_width_mm (200mm) is preserved for WYSIWYG parity with preview
        assert_eq!(page.fixed_width_mm, Some(200.0));

        // Test 2: In a 2x2 grid (cols=2), cell_w = 93mm, pad = 6mm -> safe = 87mm.
        let mut config_2x2 = config.clone();
        config_2x2.grid = LayoutGrid { rows: 2, cols: 2 };
        config_2x2.cell_w_mm = 93.0;
        config_2x2.fixed_width_mm = Some(120.0);
        config_2x2.last_page_mode = "keep".into();
        let page_keep = layout_for_page(&config_2x2, &images);
        assert_eq!(page_keep.grid.cols, 2); // 默认保留原网格
        assert_eq!(page_keep.fixed_width_mm, Some(120.0));
        config_2x2.last_page_mode = "reflow".into();
        let page_compact = layout_for_page(&config_2x2, &images);
        assert_eq!(page_compact.grid.cols, 1); // reflow 时才压缩到 1 列
        // Safe column width of base grid is 87mm
        assert_eq!(config_2x2.safe_column_width_mm(), 87.0);
        // User width is preserved
        assert_eq!(page_compact.fixed_width_mm, Some(120.0));
    }
}

#[cfg(test)]
mod freeze_layout_tests {
    use super::*;
    use std::io::Read;

    fn read_xml(path: &str) -> String {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut xml = String::new();
        zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        xml
    }

    #[test]
    fn folder_scales_and_separate_caption_line_heights_reach_docx() {
        let dir = crate::util::fs::temp_named_path("docsy-folder-scale-test", "dir");
        std::fs::create_dir(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let mut paths = Vec::new();
        for folder in ["A", "B"] {
            std::fs::create_dir(dir.join(folder)).unwrap();
            for name in ["1.png", "2.png"] {
                let path = dir.join(folder).join(name);
                image::RgbImage::new(100,100).save(&path).unwrap();
                paths.push(path.to_string_lossy().into_owned());
            }
        }
        let mut args: RunArgs = serde_json::from_value(serde_json::json!({
            "folder": dir, "image_paths": paths, "output_format": "docx",
            "layout": "2x1", "orientation": "portrait", "dpi": 300,
            "scale_mode": "fixed_width", "fixed_width_mm": 20,
            "output_mode": "per_folder", "order_mode": "custom", "page_scales": [0.8,1.2],
            "show_filename": true, "filename_font_size_pt": 8, "note_font_size_pt": 24,
            "image_annotations": { paths[0].clone(): { "title": "TITLE\nSECOND\nTHIRD", "description": "NOTE" } }
        })).unwrap();
        for table in [true,false] {
            args.use_table=Some(table);
            args.border_enabled=Some(true);
            let result=run(&args).unwrap();
            assert_eq!(result.pages,2);
            assert_eq!(result.output_paths.len(),2);
            let a=read_xml(&result.output_paths[0]);
            let b=read_xml(&result.output_paths[1]);
            let extent=regex::Regex::new(r#"<wp:extent cx="(\d+)""#).unwrap();
            let width_a: f64=extent.captures(&a).unwrap()[1].parse().unwrap();
            let width_b: f64=extent.captures(&b).unwrap()[1].parse().unwrap();
            assert!((width_b / width_a - 1.5).abs()<0.001);
            let paragraphs=regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
            let title=paragraphs.find_iter(&a).find(|p| p.as_str().contains(">TITLE<")).unwrap();
            let note=paragraphs.find_iter(&a).find(|p| p.as_str().contains(">NOTE<")).unwrap();
            assert!(!title.as_str().contains(">NOTE<"));
            assert!(title.as_str().contains(">SECOND<"));
            assert!(title.as_str().contains(">THIRD<"));
            assert!(note.as_str().contains(&format!("w:line=\"{}\"",mm_to_twips(filename_line_height_mm(24.0)))));
        }
    }
}

#[cfg(test)]
mod caption_and_destination_tests {
    use super::*;
    use std::io::Read;

    fn docx_positions(path: &str) -> (f64, f64) {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut xml = String::new();
        zip.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        let cell = regex::Regex::new(r"(?s)<w:tc>.*?</w:tc>").unwrap();
        let xml = cell.find(&xml).map(|m| m.as_str()).unwrap_or(&xml);
        let paragraphs = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
        let line = regex::Regex::new(r#"w:line="(\d+)""#).unwrap();
        let mut y = 0.0;
        let mut image = None;
        let mut text = None;
        for p in paragraphs.find_iter(xml) {
            let p = p.as_str();
            if p.contains("<w:drawing>") {
                image.get_or_insert(y);
            }
            if p.contains(">CAPTION<") {
                text.get_or_insert(y);
            }
            if let Some(line) = line.captures(p) {
                y += line[1].parse::<f64>().unwrap() * (1 + p.matches("<w:br").count()) as f64;
            }
        }
        (image.unwrap(), text.unwrap())
    }

    fn pdf_positions(path: &str) -> (Vec<Vec<lopdf::Object>>, Vec<f64>) {
        let doc = lopdf::Document::load(path).unwrap();
        let page = *doc.get_pages().values().next().unwrap();
        let content =
            lopdf::content::Content::decode(&doc.get_page_content(page).unwrap()).unwrap();
        let mut images = Vec::new();
        let mut text = Vec::new();
        for op in content.operations {
            if op.operator == "cm" {
                images.push(op.operands);
            } else if op.operator == "Tm" {
                text.push(op.operands[5].as_float().unwrap() as f64);
            } else {
                assert_ne!(
                    op.operator, "Td",
                    "caption lines must use absolute text coordinates"
                );
            }
        }
        (images, text)
    }

    #[test]
    fn changing_caption_gap_moves_text_without_moving_images_in_both_exports() {
        let dir = crate::util::fs::temp_named_path("docsy-caption-gap", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let image = dir.join("fixture.png");
        image::RgbImage::new(200, 100).save(&image).unwrap();
        for position in ["above", "below"] {
            for (format, table) in [("pdf", true), ("docx", true), ("docx", false)] {
                let mut args: RunArgs = serde_json::from_value(serde_json::json!({
                    "folder": dir, "image_paths": [image], "output_format": format,
                    "output_dir": dir, "use_table": table, "layout": "2x1", "orientation": "portrait", "dpi": 300,
                    "scale_mode": "fixed_width", "fixed_width_mm": 80, "caption_position": position,
                    "caption_gap_mm": 2, "image_annotations": {image.to_string_lossy().to_string(): {"title":"CAPTION", "description":"NOTE"}}
                })).unwrap();
                let a = run(&args).unwrap();
                args.caption_gap_mm = Some(8.0);
                let b = run(&args).unwrap();
                if format == "pdf" {
                    let (ia, ta) = pdf_positions(&a.output_path);
                    let (ib, tb) = pdf_positions(&b.output_path);
                    assert!(!ia.is_empty());
                    assert_eq!(ia, ib);
                    assert!(!ta.is_empty());
                    assert_eq!(ta.len(), tb.len());
                    let delta = if position == "above" { 6.0 } else { -6.0 } * 72.0 / 25.4;
                    for (a, b) in ta.iter().zip(tb.iter()) {
                        assert!((b - a - delta).abs() < 0.02, "{position}: {a} → {b}");
                    }
                } else {
                    let (ia, ta) = docx_positions(&a.output_path);
                    let (ib, tb) = docx_positions(&b.output_path);
                    assert!((ia - ib).abs() <= 1.0, "drawing moved: {ia} → {ib}");
                    let delta =
                        mm_to_twips(6.0) as f64 * if position == "above" { -1.0 } else { 1.0 };
                    assert!((tb - ta - delta).abs() <= 2.0, "{position}: {ta} → {tb}");
                }
            }
        }
    }

    #[test]
    fn actual_title_and_note_reserve_cannot_silently_squeeze_images_to_one_mm() {
        let dir = crate::util::fs::temp_named_path("docsy-caption-reserve", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let path = dir.join("image.png");
        image::RgbImage::new(20, 20).save(&path).unwrap();
        let mut args: RunArgs = serde_json::from_value(serde_json::json!({
            "folder": dir, "image_paths": [path], "output_dir": dir,
            "output_format": "pdf", "layout": "custom", "custom_rows": 8, "custom_cols": 1,
            "orientation": "portrait", "dpi": 300, "scale_mode": "fit",
            "filename_font_size_pt": 24, "note_font_size_pt": 24,
            "image_annotations": {path.to_string_lossy().to_string(): {
                "title": "CUSTOM TITLE", "description": "line1\nline2\nline3\nline4"
            }}
        })).unwrap();
        for format in ["pdf", "docx"] {
            args.output_format = format.into();
            let error = run(&args).unwrap_err().to_string();
            assert!(error.contains("标题或说明已占满"), "{error}");
        }
    }

    #[test]
    fn chosen_destination_covers_merged_grouped_and_scanned_sources_without_overwriting() {
        let dir = crate::util::fs::temp_named_path("docsy-output-dir", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let output = dir.join("自选输出");
        std::fs::create_dir(&output).unwrap();
        let mut paths = Vec::new();
        let mut folders = Vec::new();
        for name in ["A", "B"] {
            let folder = dir.join(name);
            std::fs::create_dir(&folder).unwrap();
            let path = folder.join("image.png");
            image::RgbImage::new(20, 20).save(&path).unwrap();
            paths.push(path);
            folders.push(folder);
        }
        let mut args: RunArgs = serde_json::from_value(serde_json::json!({
            "folder": folders[0], "folders": folders, "image_paths": paths, "output_dir": output,
            "output_format":"docx", "layout":"2x1", "orientation":"portrait", "dpi":300,
            "scale_mode":"fixed_width", "fixed_width_mm":20, "show_filename":false
        }))
        .unwrap();
        let a = run(&args).unwrap();
        let b = run(&args).unwrap();
        assert_ne!(a.output_path, b.output_path);
        assert_eq!(Path::new(&a.output_path).parent(), Some(output.as_path()));
        args.output_mode = Some("per_folder".into());
        for explicit in [true, false] {
            if !explicit {
                args.image_paths = None;
            }
            let result = run(&args).unwrap();
            assert_eq!(result.output_paths.len(), 2);
            for path in result.output_paths {
                assert_eq!(Path::new(&path).parent(), Some(output.as_path()));
            }
        }
        args.output_dir = Some(dir.join("missing").display().to_string());
        assert!(run(&args).unwrap_err().to_string().contains("输出目录"));
        args.output_dir = None;
        let result = run(&args).unwrap();
        for (path, folder) in result.output_paths.iter().zip(folders.iter()) {
            assert_eq!(
                Path::new(path).parent(),
                Some(folder.join("_docsy_image_out").as_path())
            );
        }
    }

    #[test]
    #[ignore = "manual visual fixtures for PDF and Word"]
    fn caption_gap_visual_samples() {
        let dir = Path::new("/tmp/docsy-paddler-gap-samples");
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("sample.png");
        image::RgbImage::from_fn(600, 400, |x, y| {
            if x < 10 || x > 590 || y < 10 || y > 390 {
                image::Rgb([45, 86, 75])
            } else {
                image::Rgb([211, 229, 221])
            }
        })
        .save(&path)
        .unwrap();
        for format in ["pdf", "docx"] {
            for above in ["above", "below"] {
                for gap in [2, 8] {
                    let args: RunArgs=serde_json::from_value(serde_json::json!({
                "folder":dir, "image_paths":[path,path], "output_dir":dir,"output_stem":format!("{format}-{above}-gap{gap}"),
                "output_format":format,"layout":"2x1","orientation":"portrait","dpi":300,
                "scale_mode":"fixed_width","fixed_width_mm":100,"caption_position":above,"caption_gap_mm":gap,
                "image_annotations":{path.to_string_lossy().to_string():{"title":"标题位置检查","description":"说明随标题移动，图片保持不动"}}
            })).unwrap();
                    println!("{}", run(&args).unwrap().output_path);
                }
            }
        }
    }
}
