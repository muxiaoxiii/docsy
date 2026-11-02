use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

use super::page_info::{get_page_infos, PageSize};
use super::temp_named_path;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewArgs {
    pub input_path: String,
    #[serde(default = "default_preview_page")]
    pub page: u32,
    #[serde(default = "default_preview_dpi")]
    pub dpi: u32,
    #[serde(default)]
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    image_data_url: String,
    page: u32,
    pages: u32,
    width_pt: f32,
    height_pt: f32,
    width_px: u32,
    height_px: u32,
}

fn default_preview_page() -> u32 {
    1
}

fn default_preview_dpi() -> u32 {
    120
}

#[derive(Clone, PartialEq, Eq)]
struct PreviewFileVersion {
    path: PathBuf,
    length: u64,
    modified: SystemTime,
}

impl PreviewFileVersion {
    fn read(path: &Path) -> Option<Self> {
        let info = fs::metadata(path).ok()?;
        Some(Self { path: path.to_path_buf(), length: info.len(), modified: info.modified().ok()? })
    }
}

type GeometryEntry = (PreviewFileVersion, Arc<Vec<PageSize>>);
static PREVIEW_GEOMETRY: LazyLock<Mutex<VecDeque<GeometryEntry>>> = LazyLock::new(|| Mutex::new(VecDeque::new()));

// Parse the page tree once for neighbouring pages of a large scan. Never retain
// file bytes or trust cached geometry after an external save or explicit reload.
fn cached_page_infos(input: &Path, version: &Option<PreviewFileVersion>, force: bool) -> Result<Arc<Vec<PageSize>>> {
    cached_page_infos_with(&PREVIEW_GEOMETRY, input, version, force)
}

fn cached_page_infos_with(geometry_cache: &Mutex<VecDeque<GeometryEntry>>, input: &Path, version: &Option<PreviewFileVersion>, force: bool) -> Result<Arc<Vec<PageSize>>> {
    if let Some(version) = version {
        let mut cache = geometry_cache.lock().unwrap_or_else(|error| error.into_inner());
        if force { cache.retain(|(key, _)| key.path != version.path); }
        if let Some(index) = cache.iter().position(|(key, _)| key == version) {
            let entry = cache.remove(index).unwrap();
            let pages = Arc::clone(&entry.1);
            cache.push_back(entry);
            return Ok(pages);
        }
    }
    let pages = Arc::new(get_page_infos(&input.to_string_lossy())?);
    if let Some(version) = version {
        if PreviewFileVersion::read(input).as_ref() == Some(version) {
            let mut cache = geometry_cache.lock().unwrap_or_else(|error| error.into_inner());
            cache.retain(|(key, _)| key.path != version.path);
            cache.push_back((version.clone(), Arc::clone(&pages)));
            while cache.len() > 4 { cache.pop_front(); }
        }
    }
    Ok(pages)
}

pub fn render_preview(args: &PreviewArgs) -> Result<PreviewResult> {
    let input = Path::new(&args.input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }

    let version = PreviewFileVersion::read(input);
    let page_infos = cached_page_infos(input, &version, args.force_refresh)?;
    let pages = page_infos.len() as u32;
    let page = args.page.clamp(1, pages);
    let rendered = render_pdf_page_to_png(input, page, args.dpi)?;
    let img_result = ::image::open(&rendered).context("读取预览图片失败");
    let bytes_result = fs::read(&rendered).context("读取预览图片失败");
    let _ = fs::remove_file(&rendered);
    let img = img_result?;
    let bytes = bytes_result?;
    if version.is_some() && PreviewFileVersion::read(input) != version {
        anyhow::bail!("文件在预览期间发生变化，请重新预览");
    }

    Ok(PreviewResult {
        image_data_url: format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD.encode(bytes)
        ),
        page,
        pages,
        width_pt: page_infos[(page - 1) as usize].width_pt,
        height_pt: page_infos[(page - 1) as usize].height_pt,
        width_px: img.width(),
        height_px: img.height(),
    })
}

pub(crate) fn render_pdf_page_to_png(input: &Path, page: u32, dpi: u32) -> Result<PathBuf> {
    let pdftoppm = find_pdftoppm().context("未找到 pdftoppm，无法渲染 PDF 预览")?;
    match run_pdftoppm(&pdftoppm, input, page, dpi) {
        Ok(output) => Ok(output),
        Err(first_error) => {
            let repaired = temp_named_path("docsy_pdf_preview_repaired", "pdf");
            let repair_result = repair_pdf_for_preview(input, &repaired);
            if repair_result.is_err() {
                let _ = fs::remove_file(&repaired);
                return Err(first_error);
            }
            let retry = run_pdftoppm(&pdftoppm, &repaired, page, dpi);
            let _ = fs::remove_file(&repaired);
            retry.with_context(|| format!("原文件预览失败；qpdf 修复后重试仍失败：{first_error:#}"))
        }
    }
}

fn run_pdftoppm(pdftoppm: &Path, input: &Path, page: u32, dpi: u32) -> Result<PathBuf> {
    let prefix = temp_named_path("docsy_pdf_preview", "");
    let output = PathBuf::from(format!("{}.png", prefix.display()));

    let mut command = crate::external::hidden_command(pdftoppm);
    command
        .arg("-png")
        // Match PDF readers and page_info: show the visible CropBox, not a
        // larger original MediaBox left behind by a crop/split operation.
        .arg("-cropbox")
        .arg("-singlefile")
        .arg("-r")
        .arg(dpi.to_string())
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(input)
        .arg(&prefix);
    // 设置字体配置路径，让 poppler 能找到系统字体（日语、韩语等 CJK 字体）
    configure_fontconfig_env(&mut command);
    let command_output = command.output().context("执行 pdftoppm 失败")?;

    if !command_output.status.success() {
        anyhow::bail!(
            "pdftoppm 渲染失败（{}）：{}",
            pdftoppm.display(),
            crate::external::command_failure_detail(&command_output)
        );
    }
    if !output.exists() {
        anyhow::bail!("pdftoppm 未生成预览图片");
    }

    Ok(output)
}

fn repair_pdf_for_preview(input: &Path, output: &Path) -> Result<()> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let command_output = crate::external::hidden_command(&bin)
        .arg(input)
        .arg(output)
        .output()
        .context("执行 qpdf 预览修复失败")?;
    if !super::qpdf::status_is_success(&command_output.status) || !output.exists() {
        anyhow::bail!(
            "qpdf 预览修复失败（{}）：{}",
            bin.display(),
            crate::external::command_failure_detail(&command_output)
        );
    }
    Ok(())
}

fn find_pdftoppm() -> Option<PathBuf> {
    crate::external::PopplerTool::binary_path_for("pdftoppm").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Dictionary, Document, Stream};

    #[test]
    fn preview_geometry_cache_reuses_pages_and_invalidates_after_save_or_reload() {
        use lopdf::{dictionary, Document, Object};
        let input = temp_named_path("docsy_preview_geometry", "pdf");
        let write_pdf = |width: i64, height: i64, producer: &str| {
            let mut doc = Document::with_version("1.7");
            let pages = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages,
                "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()] });
            doc.objects.insert(pages, dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into());
            let root = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
            let info = doc.add_object(dictionary! { "Producer" => Object::string_literal(producer) });
            doc.trailer.set("Root", root); doc.trailer.set("Info", info);
            doc.save(&input).unwrap();
        };
        let cache = Mutex::new(VecDeque::new());
        write_pdf(300, 400, "first");
        let version = PreviewFileVersion::read(&input);
        let a = cached_page_infos_with(&cache, &input, &version, false).unwrap();
        let b = cached_page_infos_with(&cache, &input, &version, false).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let refreshed = cached_page_infos_with(&cache, &input, &version, true).unwrap();
        assert!(!Arc::ptr_eq(&a, &refreshed));
        write_pdf(400, 600, "changed externally, with a different byte length");
        let current = PreviewFileVersion::read(&input);
        assert!(current != version);
        let source = fs::read(&input).unwrap();
        let changed = cached_page_infos_with(&cache, &input, &current, false).unwrap();
        assert_eq!((changed[0].width_pt, changed[0].height_pt), (400., 600.));
        assert_eq!(fs::read(&input).unwrap(), source);
        assert_eq!(cache.lock().unwrap().len(), 1);
        let _ = fs::remove_file(input);
    }

    #[test]
    fn preview_renders_the_visible_crop_box_instead_of_the_uncropped_spread() {
        if find_pdftoppm().is_none() {
            return;
        }
        let input = temp_named_path("docsy_preview_cropbox", "pdf");
        let _guard = crate::util::fs::TempPathGuard::new(input.clone());
        let mut doc = Document::with_version("1.7");
        let pages = doc.new_object_id();
        let content = doc.add_object(Stream::new(
            Dictionary::new(),
            b"1 0 0 rg 0 0 300 400 re f 0 0 1 rg 300 0 300 400 re f".to_vec(),
        ));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => content,
            "Resources" => Dictionary::new(),
            "MediaBox" => vec![0.into(), 0.into(), 600.into(), 400.into()],
            "CropBox" => vec![300.into(), 0.into(), 600.into(), 400.into()],
        });
        doc.objects.insert(
            pages,
            dictionary! {
                "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
            }
            .into(),
        );
        let root = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", root);
        doc.save(&input).unwrap();
        let original = fs::read(&input).unwrap();

        let preview = render_preview(&PreviewArgs {
            input_path: input.to_string_lossy().into_owned(),
            page: 1,
            dpi: 72,
            force_refresh: false,
        })
        .unwrap();
        assert_eq!((preview.width_pt, preview.height_pt), (300.0, 400.0));
        assert_eq!((preview.width_px, preview.height_px), (300, 400));
        let bytes = general_purpose::STANDARD
            .decode(preview.image_data_url.split_once(',').unwrap().1)
            .unwrap();
        let image = ::image::load_from_memory(&bytes).unwrap().to_rgb8();
        assert_eq!(image.get_pixel(150, 200).0, [0, 0, 255]);
        assert_eq!(
            fs::read(&input).unwrap(),
            original,
            "preview must not rewrite the source PDF"
        );
    }
}

/// 为 poppler 工具设置字体配置环境变量，让自管理的 poppler 能找到系统字体
/// （日语、韩语等 CJK 字体在预览渲染时显示为方块的问题）
fn configure_fontconfig_env(command: &mut std::process::Command) {
    // 如果已经有 FONTCONFIG_FILE，说明用户自行配置了，不覆盖
    if std::env::var("FONTCONFIG_FILE").is_ok() {
        return;
    }

    // 如果已经有 FONTCONFIG_PATH 且目录存在，不覆盖
    if let Ok(existing) = std::env::var("FONTCONFIG_PATH") {
        if std::path::Path::new(&existing).exists() {
            return;
        }
    }

    // 收集系统字体目录
    let font_dirs = system_font_dirs();
    if font_dirs.is_empty() {
        return;
    }

    // 生成 fontconfig XML 配置
    let config_xml = generate_fontconfig_xml(&font_dirs);
    let config_path = std::env::temp_dir().join("docsy-fontconfig.xml");
    if std::fs::write(&config_path, config_xml).is_ok() {
        command.env("FONTCONFIG_FILE", config_path.to_string_lossy().to_string());
    }
}

/// 获取系统字体目录列表
fn system_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(format!("{home}/Library/Fonts")));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windows) = std::env::var("SystemRoot") {
            dirs.push(PathBuf::from(format!("{windows}\\Fonts")));
        }
        // Windows 10+ 用户字体目录
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let user_fonts = PathBuf::from(format!("{local}\\Microsoft\\Windows\\Fonts"));
            if user_fonts.exists() {
                dirs.push(user_fonts);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/usr/share/fonts/truetype",
            "/usr/share/fonts/opentype",
        ];
        for path in &candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                dirs.push(p);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let user_fonts = PathBuf::from(format!("{home}/.local/share/fonts"));
            if user_fonts.exists() {
                dirs.push(user_fonts);
            }
        }
    }

    dirs
}

/// 生成 fontconfig XML 配置
fn generate_fontconfig_xml(font_dirs: &[PathBuf]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
"#,
    );
    for dir in font_dirs {
        xml.push_str(&format!("  <dir>{}</dir>\n", dir.display()));
    }
    xml.push_str("</fontconfig>\n");
    xml
}
