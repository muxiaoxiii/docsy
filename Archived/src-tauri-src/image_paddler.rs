//! 图片排版成 docx/pdf。
//!
//! 迁移 PicPaddler 的核心经验：自然排序、A4 横/竖、1/2/3/4 张每页、
//! fit/fill、DPI 建议、文件名显示和基础文件校验。

use std::cmp::Ordering;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};
use zip::write::{FileOptions, ZipWriter};
use zip::ZipArchive;

use crate::docx::utils::xml_escape;

const IMG_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff"];
const A4_W_PT: f64 = 595.275590551;
const A4_H_PT: f64 = 841.88976378;
const EMU_PER_PT: f64 = 12_700.0;
const DOCX_PAGE_SAFETY_PT: f64 = 24.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    pub path: String,
    pub name: String,
    pub stem: String,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageStats {
    pub count: usize,
    pub avg_w: f64,
    pub avg_h: f64,
    pub med_w: f64,
    pub med_h: f64,
    pub alpha_ratio: f64,
    pub jpeg_ratio: f64,
    pub photo_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddlerSettings {
    pub layout: u8,
    /// 布局模式："auto" / "count" / "grid"
    #[serde(default = "default_layout_mode")]
    pub layout_mode: String,
    /// 行数（layout_mode="grid" 时使用）
    #[serde(default = "default_rows")]
    pub rows: u8,
    /// 列数（layout_mode="grid" 时使用）
    #[serde(default = "default_cols")]
    pub cols: u8,
    pub dpi: String,
    pub scale: String,
    pub raster: String,
    pub orientation: String,
    pub order: String,
    pub show_filename: bool,
    pub output_format: String,
    pub jpeg_quality: u8,
    pub margin_mm: f64,
    pub gap_mm: f64,
    pub depth: usize,
}

fn default_layout_mode() -> String {
    "auto".to_string()
}

fn default_rows() -> u8 {
    1
}

fn default_cols() -> u8 {
    2
}

impl Default for PaddlerSettings {
    fn default() -> Self {
        Self {
            layout: 2,
            layout_mode: "auto".to_string(),
            rows: 1,
            cols: 2,
            dpi: "300".to_string(),
            scale: "fit".to_string(),
            raster: "auto".to_string(),
            orientation: "landscape".to_string(),
            order: "z".to_string(),
            show_filename: true,
            output_format: "docx".to_string(),
            jpeg_quality: 85,
            margin_mm: 10.0,
            gap_mm: 6.0,
            depth: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilenameGroup {
    /// 文件名前缀（去掉编号和扩展名）
    pub prefix: String,
    /// 该前缀的图片数量
    pub count: usize,
    /// 示例文件名
    pub sample: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddlerAnalysis {
    pub folder: String,
    pub image_count: usize,
    pub folder_count: usize,
    pub stats: ImageStats,
    pub recommended: PaddlerSettings,
    /// 实际扫描到的图片清单。前端滚动展示完整列表，不再只展示前 20 张。
    pub sample_images: Vec<ImageInfo>,
    /// 文件名分组（当同一目录中有多种前缀时返回）
    pub groups: Vec<FilenameGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddlerOutput {
    pub path: String,
    pub format: String,
    pub image_count: usize,
    pub page_count: usize,
    pub valid: bool,
    pub validation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddlerRunResult {
    pub output_dir: String,
    pub outputs: Vec<PaddlerOutput>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddlerRunArgs {
    pub folder: String,
    pub output_dir: Option<String>,
    pub settings: PaddlerSettings,
    /// 分组模式："auto"（按前缀分组）、"merge"（全部合并）、"selected"（只处理选中的前缀）
    #[serde(default = "default_group_mode")]
    pub group_mode: String,
    /// 当 group_mode="selected" 时，指定要处理的前缀列表
    #[serde(default)]
    pub selected_prefixes: Vec<String>,
}

fn default_group_mode() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Copy)]
struct BoxPt {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Debug, Clone, Copy)]
struct LayoutCell {
    row: usize,
    col: usize,
    rect: BoxPt,
}

pub fn analyze_folder(folder: &Path, depth: usize) -> Result<PaddlerAnalysis, String> {
    let dirs = list_image_dirs(folder, depth);
    let mut all = Vec::new();
    for dir in &dirs {
        all.extend(list_images(dir));
    }
    let infos = read_image_infos(&all);
    let stats = build_stats(&infos);
    let recommended = recommend_settings(&stats);
    let image_count = dirs.iter().map(|d| list_images(d).len()).sum();
    let folder_count = dirs.iter().filter(|d| list_images(d).len() >= 2).count();

    // 分析根目录的文件名分组
    let groups = analyze_groups(folder);

    Ok(PaddlerAnalysis {
        folder: folder.display().to_string(),
        image_count,
        folder_count,
        stats,
        recommended,
        sample_images: infos,
        groups,
    })
}

pub fn run(args: PaddlerRunArgs) -> Result<PaddlerRunResult, String> {
    let root = PathBuf::from(&args.folder);
    if !root.is_dir() {
        return Err(format!("输入目录不存在：{}", root.display()));
    }
    let output_dir = args
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("_docsy_image_out"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("创建输出目录失败：{e}"))?;

    // 解析 auto 值：layout=0 或 orientation="auto" 时，先分析再取推荐值
    let mut settings = args.settings;
    if settings.layout == 0 || settings.orientation == "auto" {
        if let Ok(analysis) = analyze_folder(&root, settings.depth) {
            if settings.layout == 0 {
                settings.layout = analysis.recommended.layout;
            }
            if settings.orientation == "auto" {
                settings.orientation = analysis.recommended.orientation;
            }
        } else {
            // 分析失败时回退到默认值
            if settings.layout == 0 {
                settings.layout = 2;
            }
            if settings.orientation == "auto" {
                settings.orientation = "landscape".to_string();
            }
        }
    }

    // 收集所有目标目录的图片
    let target_dirs: Vec<PathBuf> = list_image_dirs(&root, settings.depth)
        .into_iter()
        .filter(|d| list_images(d).len() >= 2)
        .collect();
    if target_dirs.is_empty() {
        return Ok(PaddlerRunResult {
            output_dir: output_dir.display().to_string(),
            outputs: vec![],
            skipped: vec!["没有找到至少包含 2 张图片的目录".to_string()],
        });
    }

    // 构建生成任务列表：(base_name, images)
    let mut tasks: Vec<(String, Vec<PathBuf>)> = Vec::new();

    // 检查根目录是否有图片（不在子目录中的）
    let root_images = list_images(&root);
    if root_images.len() >= 2 {
        let groups = analyze_groups(&root);
        if !groups.is_empty() && args.group_mode != "merge" {
            // 有多个分组，按分组模式处理
            let selected = match args.group_mode.as_str() {
                "selected" => {
                    // 只处理选中的前缀
                    let set: std::collections::HashSet<&str> =
                        args.selected_prefixes.iter().map(|s| s.as_str()).collect();
                    groups
                        .iter()
                        .filter(|g| set.contains(g.prefix.as_str()))
                        .collect::<Vec<_>>()
                }
                _ => {
                    // "auto" 模式：按前缀分组
                    groups.iter().collect::<Vec<_>>()
                }
            };
            for group in selected {
                let group_images: Vec<PathBuf> = root_images
                    .iter()
                    .filter(|p| extract_prefix(p) == group.prefix)
                    .cloned()
                    .collect();
                if group_images.len() >= 2 {
                    tasks.push((group.prefix.clone(), group_images));
                }
            }
        } else {
            // merge 模式或只有单一前缀：全部合并
            // 使用第一个图片的文件名前缀作为输出文件名
            let first_prefix = root_images
                .first()
                .map(|p| extract_prefix(p))
                .unwrap_or_else(|| "合并".to_string());
            tasks.push((first_prefix, root_images));
        }
    }

    // 子目录的图片（按目录分组，不按前缀分组）
    for dir in &target_dirs {
        if dir == &root {
            continue; // 根目录已在上面处理
        }
        let images = list_images(dir);
        let base_name = output_base_name(&root, dir);
        tasks.push((base_name, images));
    }

    if tasks.is_empty() {
        return Ok(PaddlerRunResult {
            output_dir: output_dir.display().to_string(),
            outputs: vec![],
            skipped: vec!["没有找到至少包含 2 张图片的目录".to_string()],
        });
    }

    let mut outputs = Vec::new();
    let mut skipped = Vec::new();
    for (base_name, images) in tasks {
        let formats = output_formats(&settings.output_format);
        for fmt in formats {
            let out = unique_path(&output_dir, &base_name, fmt);
            let result = if fmt == "docx" {
                make_docx_from_images(&images, &out, &settings)
            } else {
                make_pdf_from_images(&images, &out, &settings)
            };
            match result {
                Ok((image_count, page_count)) => {
                    let (valid, validation) = if fmt == "docx" {
                        validate_docx(&out)
                    } else {
                        validate_pdf(&out)
                    };
                    outputs.push(PaddlerOutput {
                        path: out.display().to_string(),
                        format: fmt.to_string(),
                        image_count,
                        page_count,
                        valid,
                        validation,
                    });
                }
                Err(err) => skipped.push(format!("{}: {}", base_name, err)),
            }
        }
    }

    Ok(PaddlerRunResult {
        output_dir: output_dir.display().to_string(),
        outputs,
        skipped,
    })
}

fn list_image_dirs(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if dir.file_name().and_then(|s| s.to_str()) == Some("_docsy_image_out") {
            continue;
        }
        out.push(dir.clone());
        if depth >= max_depth {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push((path, depth + 1));
                }
            }
        }
    }
    out
}

fn list_images(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            if IMG_EXTS.contains(&ext.to_ascii_lowercase().as_str()) {
                files.push(path);
            }
        }
    }
    files.sort_by(|a, b| natural_cmp(&filename(a), &filename(b)));
    files
}

/// 提取文件名前缀：去掉末尾的编号（-0001、_001、 01 等）和扩展名。
///
/// 例：
/// - "20251231-取证-0001.png" → "20251231-取证"
/// - "截图_001.jpg" → "截图"
/// - "photo 01.png" → "photo"
fn extract_prefix(path: &Path) -> String {
    let stem = file_stem(path);
    // 匹配末尾编号模式：-数字、_数字、空格+数字（至少2位）
    let re = regex::Regex::new(r"[-_ ]\d{2,}$").unwrap();
    re.replace(&stem, "").to_string()
}

/// 分析目录中的文件名分组
fn analyze_groups(folder: &Path) -> Vec<FilenameGroup> {
    let images = list_images(folder);
    if images.is_empty() {
        return vec![];
    }

    let mut groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for path in &images {
        let prefix = extract_prefix(path);
        groups.entry(prefix).or_default().push(filename(path));
    }

    // 只有多个分组时才返回
    if groups.len() <= 1 {
        return vec![];
    }

    let mut result: Vec<FilenameGroup> = groups
        .into_iter()
        .map(|(prefix, files)| FilenameGroup {
            count: files.len(),
            sample: files.first().cloned().unwrap_or_default(),
            prefix,
        })
        .collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.count)); // 按数量降序
    result
}

fn natural_cmp(a: &str, b: &str) -> Ordering {
    let ta = natural_tokens(a);
    let tb = natural_tokens(b);
    ta.cmp(&tb)
}

fn natural_tokens(s: &str) -> Vec<NaturalToken> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut number = None;
    for ch in s.chars() {
        let is_num = ch.is_ascii_digit();
        if number == Some(is_num) || number.is_none() {
            buf.push(ch);
            number = Some(is_num);
        } else {
            out.push(token_from_buf(&buf, number.unwrap_or(false)));
            buf.clear();
            buf.push(ch);
            number = Some(is_num);
        }
    }
    if !buf.is_empty() {
        out.push(token_from_buf(&buf, number.unwrap_or(false)));
    }
    out
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
enum NaturalToken {
    Text(String),
    Num(u64),
}

fn token_from_buf(buf: &str, number: bool) -> NaturalToken {
    if number {
        NaturalToken::Num(buf.parse().unwrap_or(0))
    } else {
        NaturalToken::Text(buf.to_ascii_lowercase())
    }
}

fn read_image_infos(paths: &[PathBuf]) -> Vec<ImageInfo> {
    paths
        .iter()
        .filter_map(|p| {
            let reader = ImageReader::open(p).ok()?.with_guessed_format().ok()?;
            let fmt = reader.format();
            let im = reader.decode().ok()?;
            let has_alpha = matches!(
                im.color(),
                image::ColorType::Rgba8
                    | image::ColorType::Rgba16
                    | image::ColorType::La8
                    | image::ColorType::La16
            );
            Some(ImageInfo {
                path: p.display().to_string(),
                name: filename(p),
                stem: file_stem(p),
                width: im.width(),
                height: im.height(),
                has_alpha,
                format: fmt.map(|f| format!("{f:?}")).unwrap_or_default(),
            })
        })
        .collect()
}

fn build_stats(infos: &[ImageInfo]) -> ImageStats {
    if infos.is_empty() {
        return ImageStats {
            count: 0,
            avg_w: 0.0,
            avg_h: 0.0,
            med_w: 0.0,
            med_h: 0.0,
            alpha_ratio: 0.0,
            jpeg_ratio: 0.0,
            photo_ratio: 0.0,
        };
    }
    let count = infos.len();
    let mut widths: Vec<u32> = infos.iter().map(|i| i.width).collect();
    let mut heights: Vec<u32> = infos.iter().map(|i| i.height).collect();
    widths.sort_unstable();
    heights.sort_unstable();
    let avg_w = widths.iter().map(|v| *v as f64).sum::<f64>() / count as f64;
    let avg_h = heights.iter().map(|v| *v as f64).sum::<f64>() / count as f64;
    let alpha_ratio = infos.iter().filter(|i| i.has_alpha).count() as f64 / count as f64;
    let jpeg_ratio = infos
        .iter()
        .filter(|i| i.format.eq_ignore_ascii_case("jpeg"))
        .count() as f64
        / count as f64;
    ImageStats {
        count,
        avg_w,
        avg_h,
        med_w: median_u32(&widths),
        med_h: median_u32(&heights),
        alpha_ratio,
        jpeg_ratio,
        photo_ratio: jpeg_ratio,
    }
}

fn median_u32(values: &[u32]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid] as f64
    } else {
        (values[mid - 1] as f64 + values[mid] as f64) / 2.0
    }
}

fn recommend_settings(stats: &ImageStats) -> PaddlerSettings {
    let mut settings = PaddlerSettings::default();
    if stats.count == 0 {
        return settings;
    }
    let base_w = if stats.med_w > 0.0 {
        stats.med_w
    } else {
        stats.avg_w
    };
    let base_h = if stats.med_h > 0.0 {
        stats.med_h
    } else {
        stats.avg_h
    };
    settings.orientation = if base_h >= base_w {
        "portrait"
    } else {
        "landscape"
    }
    .to_string();
    let (page_w, page_h) = page_size_pt(&settings.orientation);
    let mut best = (1u8, 0.0);
    let mut above = Vec::new();
    for layout in [1u8, 2, 3, 4] {
        let mut test_settings = settings.clone();
        test_settings.layout = layout;
        test_settings.layout_mode = "count".to_string();
        let boxes = compute_layout(&test_settings, page_w, page_h);
        let Some(cell) = boxes.first() else { continue };
        let scale = (cell.w / base_w).min(cell.h / base_h);
        let dpi_eff = if scale > 0.0 { 72.0 / scale } else { 0.0 };
        if dpi_eff >= 300.0 {
            above.push((layout, dpi_eff));
        }
        if dpi_eff > best.1 {
            best = (layout, dpi_eff);
        }
    }
    settings.layout = above.last().map(|x| x.0).unwrap_or(best.0);
    if stats.count == 3 && base_h > base_w {
        settings.layout = 3;
        settings.orientation = "landscape".to_string();
    }
    settings.raster = if stats.alpha_ratio > 0.1 {
        "png".to_string()
    } else if stats.jpeg_ratio > 0.6 || stats.photo_ratio > 0.6 {
        "jpeg".to_string()
    } else {
        "png".to_string()
    };
    if best.1 < 300.0 {
        settings.dpi = "orig".to_string();
    }
    settings
}

fn make_docx_from_images(
    images: &[PathBuf],
    out: &Path,
    settings: &PaddlerSettings,
) -> Result<(usize, usize), String> {
    let (page_w, page_h) = page_size_pt(&settings.orientation);
    let ordered_cells = compute_layout_cells(settings, page_w, page_h - DOCX_PAGE_SAFETY_PT);
    if ordered_cells.is_empty() {
        return Err("布局为空".to_string());
    }
    let per_page = ordered_cells.len();
    let pages = images.chunks(per_page).count();
    let mut media = Vec::new();
    let mut rels = String::new();
    let mut body = String::new();
    let mut rel_id = 1usize;
    let gap_pt = mm_to_pt(settings.gap_mm);

    for (page_idx, group) in images.chunks(per_page).enumerate() {
        let (cols, rows) = get_grid(settings);
        let mut slots: Vec<Option<&PathBuf>> = vec![None; cols * rows];
        for (image_idx, path) in group.iter().enumerate() {
            if let Some(cell) = ordered_cells.get(image_idx) {
                let slot_idx = cell.row * cols + cell.col;
                if slot_idx < slots.len() {
                    slots[slot_idx] = Some(path);
                }
            }
        }
        let empty_gap_cell = |width_pt: f64| {
            format!(
                r#"<w:tc><w:tcPr><w:tcW w:w="{}" w:type="dxa"/></w:tcPr><w:p/></w:tc>"#,
                pt_to_dxa(width_pt)
            )
        };
        let col_widths = docx_table_col_widths(&ordered_cells, cols, gap_pt);
        let table_w: i64 = col_widths.iter().map(|w| pt_to_dxa(*w)).sum();
        body.push_str(&format!(
            r#"<w:tbl><w:tblPr><w:tblW w:w="{table_w}" w:type="dxa"/><w:tblLayout w:type="fixed"/><w:tblCellMar><w:top w:w="0" w:type="dxa"/><w:left w:w="0" w:type="dxa"/><w:bottom w:w="0" w:type="dxa"/><w:right w:w="0" w:type="dxa"/></w:tblCellMar><w:tblBorders><w:top w:val="nil"/><w:left w:val="nil"/><w:bottom w:val="nil"/><w:right w:val="nil"/><w:insideH w:val="nil"/><w:insideV w:val="nil"/></w:tblBorders></w:tblPr>{}"#,
            docx_table_grid_xml(&col_widths)
        ));
        for row in 0..rows {
            let row_h = ordered_cells
                .iter()
                .find(|cell| cell.row == row)
                .map(|cell| cell.rect.h)
                .unwrap_or_else(|| (page_h - 2.0 * mm_to_pt(settings.margin_mm)) / rows as f64);
            body.push_str(&format!(
                r#"<w:tr><w:trPr><w:trHeight w:val="{}" w:hRule="exact"/></w:trPr>"#,
                pt_to_twip(row_h)
            ));
            for col in 0..cols {
                let idx = row * cols + col;
                let cell = ordered_cells
                    .iter()
                    .find(|cell| cell.row == row && cell.col == col)
                    .map(|cell| cell.rect)
                    .unwrap_or(BoxPt {
                        x: 0.0,
                        y: 0.0,
                        w: 1.0,
                        h: row_h,
                    });
                body.push_str(&format!(
                    r#"<w:tc><w:tcPr><w:tcW w:w="{}" w:type="dxa"/><w:vAlign w:val="top"/><w:noWrap/></w:tcPr>"#,
                    pt_to_dxa(cell.w)
                ));
                if let Some(path) = slots.get(idx).and_then(|p| *p) {
                    let source = image::open(path)
                        .map_err(|e| format!("读取图片失败 {}：{e}", path.display()))?;
                    let (source_w, source_h) = source.dimensions();
                    let avail_h = image_area_h(cell.h, settings);

                    let (draw_w, draw_h, draw_img) = match settings.scale.as_str() {
                        "fill" => (cell.w, avail_h, crop_to_aspect(source, cell.w / avail_h)),
                        "original" => {
                            let w = source_w as f64;
                            let h = source_h as f64;
                            if w > cell.w || h > avail_h {
                                let (fw, fh) = fit_size(w, h, cell.w, avail_h);
                                (fw, fh, source)
                            } else {
                                (w, h, source)
                            }
                        }
                        _ => {
                            let (w, h) =
                                fit_size(source_w as f64, source_h as f64, cell.w, avail_h);
                            (w, h, source)
                        }
                    };
                    let image = encode_docx_image(draw_img, settings)?;
                    let image_name = format!("image{rel_id}.{}", image.ext);
                    let target = format!("media/{image_name}");
                    rels.push_str(&format!(
                        r#"<Relationship Id="rId{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{target}"/>"#
                    ));
                    media.push((format!("word/{target}"), image.bytes));

                    body.push_str(&image_paragraph(rel_id, &image_name, draw_w, draw_h));
                    if settings.show_filename {
                        body.push_str(&text_paragraph(&file_stem(path)));
                    }
                    rel_id += 1;
                } else {
                    body.push_str("<w:p/>");
                }
                body.push_str("</w:tc>");
                if col + 1 < cols {
                    body.push_str(&empty_gap_cell(gap_pt));
                }
            }
            body.push_str("</w:tr>");
            if row + 1 < rows {
                body.push_str(&format!(
                    r#"<w:tr><w:trPr><w:trHeight w:val="{}" w:hRule="exact"/></w:trPr>"#,
                    pt_to_twip(gap_pt)
                ));
                for col in 0..cols {
                    let cell_w = ordered_cells
                        .iter()
                        .find(|cell| cell.row == row && cell.col == col)
                        .map(|cell| cell.rect.w)
                        .unwrap_or(1.0);
                    body.push_str(&empty_gap_cell(cell_w));
                    if col + 1 < cols {
                        body.push_str(&empty_gap_cell(gap_pt));
                    }
                }
                body.push_str("</w:tr>");
            }
        }
        body.push_str("</w:tbl>");
        if page_idx + 1 < pages {
            body.push_str(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#);
        }
    }

    body.push_str(&section_xml(
        page_w,
        page_h,
        &settings.orientation,
        settings.margin_mm,
    ));
    let document = document_xml(&body);
    let rels_xml = document_rels_xml(&rels);

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip_file(
            &mut zip,
            opts,
            "[Content_Types].xml",
            content_types_xml(&media),
        )?;
        zip_file(&mut zip, opts, "_rels/.rels", package_rels_xml())?;
        zip_file(&mut zip, opts, "word/document.xml", document)?;
        zip_file(&mut zip, opts, "word/_rels/document.xml.rels", rels_xml)?;
        zip_file(&mut zip, opts, "word/styles.xml", styles_xml())?;
        for (name, bytes) in media {
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    fs::write(out, buf).map_err(|e| format!("写入 docx 失败：{e}"))?;
    Ok((images.len(), pages))
}

fn make_pdf_from_images(
    images: &[PathBuf],
    out: &Path,
    settings: &PaddlerSettings,
) -> Result<(usize, usize), String> {
    let (page_w_pt, page_h_pt) = page_size_pt(&settings.orientation);
    let boxes = compute_layout(settings, page_w_pt, page_h_pt);
    if boxes.is_empty() {
        return Err("布局为空".to_string());
    }
    let per_page = boxes.len();
    let pages = images.chunks(per_page).count();
    let mut pdf_pages = Vec::new();
    let mut next_image_index = 1usize;
    for group in images.chunks(per_page) {
        let mut content = String::new();
        let mut page_images = Vec::new();
        for (idx, path) in group.iter().enumerate() {
            let cell = boxes[idx];
            let im =
                image::open(path).map_err(|e| format!("读取图片失败 {}：{e}", path.display()))?;
            let (img_w, img_h) = im.dimensions();
            let avail_h = image_area_h(cell.h, settings);
            let (draw_w, draw_h, draw_img) = match settings.scale.as_str() {
                "fill" => {
                    // 填满裁切：裁切图片以填满整个区域
                    (cell.w, avail_h, crop_to_aspect(im, cell.w / avail_h))
                }
                "original" => {
                    // 原始大小：使用图片实际像素大小（1pt = 1px）
                    let w = img_w as f64;
                    let h = img_h as f64;
                    // 如果超过 cell 大小，则缩小
                    if w > cell.w || h > avail_h {
                        let (fw, fh) = fit_size(w, h, cell.w, avail_h);
                        (fw, fh, im)
                    } else {
                        (w, h, im)
                    }
                }
                _ => {
                    // 适应页面：缩放图片以完全显示在区域内
                    let (w, h) = fit_size(img_w as f64, img_h as f64, cell.w, avail_h);
                    (w, h, im)
                }
            };
            // 图片在 cell 内居中（考虑文件名区域）
            let text_h = if settings.show_filename { 14.0 } else { 0.0 };
            let x = cell.x + (cell.w - draw_w) / 2.0;
            let y = cell.y + text_h + (avail_h - draw_h) / 2.0;
            let prepared = prepare_pdf_image(draw_img, settings)?;
            let jpeg = encode_jpeg(&prepared, settings.jpeg_quality)?;
            let image_name = format!("Im{next_image_index}");
            content.push_str(&format!(
                "q {:.3} 0 0 {:.3} {:.3} {:.3} cm /{} Do Q\n",
                draw_w, draw_h, x, y, image_name
            ));
            page_images.push(PdfImageObject {
                name: image_name,
                width: prepared.width(),
                height: prepared.height(),
                bytes: jpeg,
            });
            next_image_index += 1;
            if settings.show_filename {
                let label = ascii_pdf_label(&file_stem(path));
                let font_size = 9.0;
                let label_w = estimate_pdf_text_width(&label, font_size);
                let tx = cell.x + ((cell.w - label_w) / 2.0).max(0.0);
                let ty = cell.y + 3.0;
                content.push_str(&format!(
                    "BT /F1 {:.1} Tf {:.3} {:.3} Td ({}) Tj ET\n",
                    font_size,
                    tx,
                    ty,
                    pdf_escape(&label)
                ));
            }
        }
        pdf_pages.push(PdfPageObject {
            content,
            images: page_images,
        });
    }
    let bytes = build_pdf(page_w_pt, page_h_pt, pdf_pages)?;
    fs::write(out, bytes).map_err(|e| format!("写入 PDF 失败：{e}"))?;
    Ok((images.len(), pages))
}

struct PdfImageObject {
    name: String,
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

struct PdfPageObject {
    content: String,
    images: Vec<PdfImageObject>,
}

struct DocxImage {
    bytes: Vec<u8>,
    ext: String,
}

fn encode_docx_image(im: DynamicImage, settings: &PaddlerSettings) -> Result<DocxImage, String> {
    let mut buf = Vec::new();
    let use_jpeg =
        settings.raster == "jpeg" || (settings.raster == "auto" && !im.color().has_alpha());
    if use_jpeg {
        let rgb = flatten_to_rgb(&im);
        let mut enc = JpegEncoder::new_with_quality(&mut buf, settings.jpeg_quality);
        enc.encode_image(&rgb)
            .map_err(|e| format!("转换图片失败：{e}"))?;
    } else {
        im.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| format!("转换图片失败：{e}"))?;
    }
    Ok(DocxImage {
        bytes: buf,
        ext: if use_jpeg { "jpg" } else { "png" }.to_string(),
    })
}

fn prepare_pdf_image(im: DynamicImage, settings: &PaddlerSettings) -> Result<DynamicImage, String> {
    let dpi = settings.dpi.parse::<u32>().ok();
    let mut out = flatten_to_rgb(&im);
    if let Some(dpi) = dpi {
        let max_side = (dpi * 12).max(1);
        if out.width() > max_side || out.height() > max_side {
            out = out.resize(max_side, max_side, image::imageops::FilterType::Lanczos3);
        }
    }
    Ok(out)
}

fn flatten_to_rgb(im: &DynamicImage) -> DynamicImage {
    if im.color().has_alpha() {
        let rgba = im.to_rgba8();
        let mut bg =
            image::RgbImage::from_pixel(im.width(), im.height(), image::Rgb([255, 255, 255]));
        for (x, y, px) in rgba.enumerate_pixels() {
            let alpha = px[3] as f32 / 255.0;
            let inv = 1.0 - alpha;
            let r = (px[0] as f32 * alpha + 255.0 * inv).round() as u8;
            let g = (px[1] as f32 * alpha + 255.0 * inv).round() as u8;
            let b = (px[2] as f32 * alpha + 255.0 * inv).round() as u8;
            bg.put_pixel(x, y, image::Rgb([r, g, b]));
        }
        DynamicImage::ImageRgb8(bg)
    } else {
        DynamicImage::ImageRgb8(im.to_rgb8())
    }
}

fn encode_jpeg(im: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let rgb = im.to_rgb8();
    let mut out = Vec::new();
    let mut enc = JpegEncoder::new_with_quality(&mut out, quality);
    enc.encode_image(&DynamicImage::ImageRgb8(rgb))
        .map_err(|e| format!("编码 JPEG 失败：{e}"))?;
    Ok(out)
}

fn crop_to_aspect(im: DynamicImage, target_ratio: f64) -> DynamicImage {
    let (w, h) = im.dimensions();
    let current = if h == 0 { 1.0 } else { w as f64 / h as f64 };
    if (current - target_ratio).abs() < 0.0001 {
        return im;
    }
    if current > target_ratio {
        let new_w = (h as f64 * target_ratio).round().max(1.0) as u32;
        let left = (w - new_w) / 2;
        im.crop_imm(left, 0, new_w, h)
    } else {
        let new_h = (w as f64 / target_ratio).round().max(1.0) as u32;
        let top = (h - new_h) / 2;
        im.crop_imm(0, top, w, new_h)
    }
}

fn compute_layout(settings: &PaddlerSettings, page_w: f64, page_h: f64) -> Vec<BoxPt> {
    compute_layout_cells(settings, page_w, page_h)
        .into_iter()
        .map(|cell| cell.rect)
        .collect()
}

fn compute_layout_cells(settings: &PaddlerSettings, page_w: f64, page_h: f64) -> Vec<LayoutCell> {
    let margin = mm_to_pt(settings.margin_mm);
    let gap = mm_to_pt(settings.gap_mm);
    let content_w = page_w - 2.0 * margin;
    let content_h = page_h - 2.0 * margin;
    let (cols, rows) = get_grid(settings);
    let cell_w = (content_w - gap * (cols as f64 - 1.0)) / cols as f64;
    let cell_h = (content_h - gap * (rows as f64 - 1.0)) / rows as f64;
    let mut items = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            items.push(LayoutCell {
                row,
                col,
                rect: BoxPt {
                    x: margin + col as f64 * (cell_w + gap),
                    y: margin + (rows - 1 - row) as f64 * (cell_h + gap),
                    w: cell_w,
                    h: cell_h,
                },
            });
        }
    }
    match settings.order.as_str() {
        "n" => items.sort_by_key(|cell| (usize::MAX - cell.row, cell.col)),
        "z_rev" => items.sort_by_key(|cell| (cell.row, usize::MAX - cell.col)),
        "n_rev" => items.sort_by_key(|cell| (usize::MAX - cell.row, usize::MAX - cell.col)),
        _ => items.sort_by_key(|cell| (cell.row, cell.col)),
    }
    items
}

fn docx_table_col_widths(cells: &[LayoutCell], cols: usize, gap_pt: f64) -> Vec<f64> {
    let mut widths = Vec::new();
    for col in 0..cols {
        let w = cells
            .iter()
            .find(|cell| cell.col == col)
            .map(|cell| cell.rect.w)
            .unwrap_or(1.0);
        widths.push(w);
        if col + 1 < cols {
            widths.push(gap_pt);
        }
    }
    widths
}

fn docx_table_grid_xml(widths: &[f64]) -> String {
    let cols = widths
        .iter()
        .map(|w| format!(r#"<w:gridCol w:w="{}"/>"#, pt_to_dxa(*w)))
        .collect::<Vec<_>>()
        .join("");
    format!("<w:tblGrid>{cols}</w:tblGrid>")
}

fn grid(layout: u8) -> (usize, usize) {
    match layout {
        1 => (1, 1),
        2 => (2, 1),
        3 => (3, 1),
        _ => (2, 2),
    }
}

/// 根据设置获取行列数
fn get_grid(settings: &PaddlerSettings) -> (usize, usize) {
    match settings.layout_mode.as_str() {
        "grid" => {
            // 自定义行列
            (settings.cols.max(1) as usize, settings.rows.max(1) as usize)
        }
        "count" => {
            // 按张数，自动计算行列
            grid(settings.layout)
        }
        _ => {
            // auto，使用推荐值或默认值
            grid(settings.layout)
        }
    }
}

fn page_size_pt(orientation: &str) -> (f64, f64) {
    if orientation == "portrait" {
        (A4_W_PT, A4_H_PT)
    } else {
        (A4_H_PT, A4_W_PT)
    }
}

fn fit_size(img_w: f64, img_h: f64, box_w: f64, box_h: f64) -> (f64, f64) {
    if img_w <= 0.0 || img_h <= 0.0 || box_w <= 0.0 || box_h <= 0.0 {
        return (0.0, 0.0);
    }
    let scale = (box_w / img_w).min(box_h / img_h);
    (img_w * scale, img_h * scale)
}

/// 计算图片可用高度（减去文件名高度）
///
/// PDF 和 Word 都使用这个函数，确保布局一致
fn image_area_h(cell_h: f64, settings: &PaddlerSettings) -> f64 {
    if settings.show_filename {
        // 文件名区域高度：字号(9pt) + 间距(5pt) = 14pt
        (cell_h - 14.0).max(1.0)
    } else {
        cell_h
    }
}

fn zip_file(
    zip: &mut ZipWriter<Cursor<&mut Vec<u8>>>,
    opts: FileOptions,
    name: &str,
    data: String,
) -> Result<(), String> {
    zip.start_file(name, opts).map_err(|e| e.to_string())?;
    zip.write_all(data.as_bytes()).map_err(|e| e.to_string())
}

fn content_types_xml(media: &[(String, Vec<u8>)]) -> String {
    let mut png = false;
    let mut jpg = false;
    for (name, _) in media {
        png |= name.ends_with(".png");
        jpg |= name.ends_with(".jpg") || name.ends_with(".jpeg");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{png}{jpg}<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#,
        png = if png {
            r#"<Default Extension="png" ContentType="image/png"/>"#
        } else {
            ""
        },
        jpg = if jpg {
            r#"<Default Extension="jpg" ContentType="image/jpeg"/>"#
        } else {
            ""
        },
    )
}

fn package_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_string()
}

fn document_rels_xml(image_rels: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>{image_rels}</Relationships>"#
    )
}

fn document_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}</w:body></w:document>"#
    )
}

fn styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style></w:styles>"#.to_string()
}

fn section_xml(page_w: f64, page_h: f64, orientation: &str, margin_mm: f64) -> String {
    let orient = if orientation == "portrait" {
        ""
    } else {
        r#" w:orient="landscape""#
    };
    let mar = pt_to_twip(mm_to_pt(margin_mm));
    format!(
        r#"<w:sectPr><w:pgSz w:w="{}" w:h="{}"{orient}/><w:pgMar w:top="{mar}" w:right="{mar}" w:bottom="{mar}" w:left="{mar}" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#,
        pt_to_twip(page_w),
        pt_to_twip(page_h),
    )
}

fn image_paragraph(rel_id: usize, name: &str, w_pt: f64, h_pt: f64) -> String {
    let cx = (w_pt * EMU_PER_PT).round() as i64;
    let cy = (h_pt * EMU_PER_PT).round() as i64;
    format!(
        r#"<w:p><w:pPr><w:jc w:val="center"/><w:spacing w:before="0" w:after="0"/></w:pPr><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{rel_id}" name="{name}"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{rel_id}" name="{name}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId{rel_id}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"#
    )
}

fn text_paragraph(text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:jc w:val="center"/><w:spacing w:before="0" w:after="0"/></w:pPr><w:r><w:rPr><w:b/><w:sz w:val="18"/><w:szCs w:val="18"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
        xml_escape(text)
    )
}

fn output_formats(s: &str) -> Vec<&'static str> {
    match s {
        "pdf" => vec!["pdf"],
        "both" => vec!["docx", "pdf"],
        _ => vec!["docx"],
    }
}

fn output_base_name(root: &Path, dir: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => filename(root),
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("-"),
        Err(_) => filename(dir),
    }
}

fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut path = dir.join(format!("{base}.{ext}"));
    if !path.exists() {
        return path;
    }
    for i in 2..10000 {
        path = dir.join(format!("{base}_{i}.{ext}"));
        if !path.exists() {
            return path;
        }
    }
    dir.join(format!("{base}_overflow.{ext}"))
}

fn validate_docx(path: &Path) -> (bool, String) {
    let Ok(file) = fs::File::open(path) else {
        return (false, "文件不存在".to_string());
    };
    let Ok(mut z) = ZipArchive::new(file) else {
        return (false, "不是有效 zip/docx".to_string());
    };
    for name in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/document.xml",
        "word/_rels/document.xml.rels",
    ] {
        if z.by_name(name).is_err() {
            return (false, format!("缺少 {name}"));
        }
    }
    let mut rels = String::new();
    {
        if let Ok(mut f) = z.by_name("word/_rels/document.xml.rels") {
            let _ = f.read_to_string(&mut rels);
        }
    }
    for target in rels
        .split("Target=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
    {
        if target.starts_with("media/") {
            let media_name = format!("word/{target}");
            if z.by_name(&media_name).is_err() {
                return (false, format!("图片关系丢失：{media_name}"));
            }
        }
    }
    (true, "docx 容器校验通过".to_string())
}

fn validate_pdf(path: &Path) -> (bool, String) {
    let Ok(bytes) = fs::read(path) else {
        return (false, "文件不存在".to_string());
    };
    if bytes.len() < 64 {
        return (false, "PDF 文件过小".to_string());
    }
    if !bytes.starts_with(b"%PDF-") {
        return (false, "缺少 PDF 文件头".to_string());
    }
    let tail = &bytes[bytes.len().saturating_sub(1024)..];
    if !tail.windows(5).any(|w| w == b"%%EOF") {
        return (false, "缺少 PDF EOF 标记".to_string());
    }
    (true, "PDF 基础校验通过".to_string())
}

fn build_pdf(page_w: f64, page_h: f64, pages: Vec<PdfPageObject>) -> Result<Vec<u8>, String> {
    let mut objects: Vec<Vec<u8>> = vec![Vec::new(), Vec::new(), Vec::new()];
    let mut page_ids = Vec::new();

    for page in pages {
        let page_id = objects.len() + 1;
        let content_id = page_id + 1;
        page_ids.push(page_id);
        let mut image_refs = Vec::new();
        let mut image_ids = Vec::new();
        let image_count = page.images.len();
        for i in 0..image_count {
            image_ids.push(content_id + 1 + i);
        }
        for (img, obj_id) in page.images.into_iter().zip(image_ids.iter()) {
            image_refs.push(format!("/{} {} 0 R", img.name, obj_id));
            objects.push(pdf_image_object(img));
        }
        let resources = if image_refs.is_empty() {
            r#"<< /Font << /F1 3 0 R >> >>"#.to_string()
        } else {
            format!(
                r#"<< /Font << /F1 3 0 R >> /XObject << {} >> >>"#,
                image_refs.join(" ")
            )
        };
        let page_obj = format!(
            r#"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.3} {:.3}] /Resources {} /Contents {} 0 R >>"#,
            page_w, page_h, resources, content_id
        );
        let content = stream_object(page.content.as_bytes());
        objects.insert(page_id - 1, page_obj.into_bytes());
        objects.insert(content_id - 1, content);
    }

    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects[1] = format!(
        r#"<< /Type /Pages /Kids [ {} ] /Count {} >>"#,
        kids,
        page_ids.len()
    )
    .into_bytes();
    objects[2] =
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec();

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0usize];
    for (idx, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref
        )
        .as_bytes(),
    );
    Ok(out)
}

fn stream_object(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("<< /Length {} >>\nstream\n", data.len()).as_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn pdf_image_object(img: PdfImageObject) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(
        format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            img.width,
            img.height,
            img.bytes.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&img.bytes);
    out.extend_from_slice(b"\nendstream");
    out
}

fn pdf_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn estimate_pdf_text_width(s: &str, font_size: f64) -> f64 {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                0.62
            } else if matches!(ch, '_' | '-' | '.' | '/' | ' ') {
                0.33
            } else {
                0.55
            }
        })
        .sum::<f64>()
        * font_size
}

fn filename(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image")
        .to_string()
}

fn ascii_pdf_label(s: &str) -> String {
    if s.is_ascii() {
        return s.to_string();
    }
    // 保留 ASCII 可打印字符，移除非 ASCII 字符，清理多余空格
    let mut out = String::new();
    let mut last_space = false;
    for ch in s.chars() {
        if ch.is_ascii() && !ch.is_control() {
            out.push(ch);
            last_space = ch == ' ';
        } else if !last_space && !out.is_empty() {
            out.push(' ');
            last_space = true;
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        "image".to_string()
    } else {
        trimmed.to_string()
    }
}

fn mm_to_pt(v: f64) -> f64 {
    v * 72.0 / 25.4
}

fn pt_to_dxa(v: f64) -> i64 {
    (v * 20.0).round() as i64
}

fn pt_to_twip(v: f64) -> i64 {
    (v * 20.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_docx_and_pdf_from_sample_images() {
        let root = std::env::temp_dir().join(format!(
            "docsy-image-paddler-test-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        fs::create_dir_all(&root).expect("创建临时目录失败");
        for i in 1..=3 {
            let mut img = image::RgbImage::new(320, 180);
            for (x, y, px) in img.enumerate_pixels_mut() {
                *px = image::Rgb([(x % 255) as u8, (y % 255) as u8, (i * 60) as u8]);
            }
            img.save(root.join(format!("{i}.png")))
                .expect("写入测试图片失败");
        }

        let settings = PaddlerSettings {
            layout: 3,
            output_format: "both".to_string(),
            orientation: "landscape".to_string(),
            show_filename: true,
            ..PaddlerSettings::default()
        };
        let result = run(PaddlerRunArgs {
            folder: root.display().to_string(),
            output_dir: None,
            settings,
            group_mode: "merge".to_string(),
            selected_prefixes: vec![],
        })
        .expect("生成失败");

        assert_eq!(result.outputs.len(), 2);
        assert!(result.outputs.iter().all(|o| o.valid));
        assert!(result.outputs.iter().any(|o| o.format == "docx"));
        assert!(result.outputs.iter().any(|o| o.format == "pdf"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn docx_keeps_tall_images_and_filenames_inside_fixed_cells() {
        let root = std::env::temp_dir().join(format!(
            "docsy-image-paddler-tall-test-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        fs::create_dir_all(&root).expect("创建临时目录失败");
        let mut images = Vec::new();
        for i in 1..=4 {
            let mut img = image::RgbImage::new(644, 1398);
            for (x, y, px) in img.enumerate_pixels_mut() {
                *px = image::Rgb([(x % 255) as u8, (y % 255) as u8, (i * 50) as u8]);
            }
            let path = root.join(format!("TSA_SCREEN_20260526102633_{i:05}.jpg"));
            img.save(&path).expect("写入测试图片失败");
            images.push(path);
        }

        let settings = PaddlerSettings {
            layout: 2,
            layout_mode: "count".to_string(),
            output_format: "docx".to_string(),
            orientation: "landscape".to_string(),
            show_filename: true,
            scale: "fit".to_string(),
            margin_mm: 10.0,
            gap_mm: 6.0,
            ..PaddlerSettings::default()
        };
        let out = root.join("tall.docx");
        let (image_count, page_count) =
            make_docx_from_images(&images, &out, &settings).expect("生成 docx 失败");
        assert_eq!(image_count, 4);
        assert_eq!(page_count, 2);

        let (valid, validation) = validate_docx(&out);
        assert!(valid, "{validation}");

        let file = fs::File::open(&out).expect("读取 docx 失败");
        let mut zip = ZipArchive::new(file).expect("打开 docx zip 失败");
        let mut document = String::new();
        zip.by_name("word/document.xml")
            .expect("缺少 document.xml")
            .read_to_string(&mut document)
            .expect("读取 document.xml 失败");

        assert!(document.contains(r#"w:trHeight"#));
        assert!(document.contains(r#"w:hRule="exact""#));
        assert!(document.contains("<w:tblGrid>"));
        assert_eq!(document.matches("<wp:inline").count(), 4);

        let (page_w, page_h) = page_size_pt(&settings.orientation);
        let first_cell = compute_layout(&settings, page_w, page_h)[0];
        let max_image_cy = (image_area_h(first_cell.h, &settings) * EMU_PER_PT).round() as i64;
        for cy in document
            .split(r#"cy=""#)
            .skip(1)
            .filter_map(|part| part.split('"').next())
            .filter_map(|raw| raw.parse::<i64>().ok())
        {
            assert!(
                cy <= max_image_cy,
                "图片高度 {cy} 超过可用高度 {max_image_cy}"
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn custom_grid_is_used_by_docx_generation() {
        let settings = PaddlerSettings {
            layout: 0,
            layout_mode: "grid".to_string(),
            rows: 2,
            cols: 3,
            orientation: "landscape".to_string(),
            ..PaddlerSettings::default()
        };
        let (page_w, page_h) = page_size_pt(&settings.orientation);
        let boxes = compute_layout(&settings, page_w, page_h);
        assert_eq!(boxes.len(), 6);
    }
}
