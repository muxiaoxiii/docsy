use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

use super::page_info::get_page_infos;
use super::temp_named_path;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewArgs {
    input_path: String,
    #[serde(default = "default_preview_page")]
    page: u32,
    #[serde(default = "default_preview_dpi")]
    dpi: u32,
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

pub fn render_preview(args: &serde_json::Value) -> Result<PreviewResult> {
    let args: PreviewArgs = serde_json::from_value(args.clone()).context("解析预览参数失败")?;
    let input = Path::new(&args.input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }

    let page_infos = get_page_infos(&args.input_path)?;
    let pages = page_infos.len() as u32;
    let page = args.page.clamp(1, pages);
    let rendered = render_pdf_page_to_png(input, page, args.dpi)?;
    let img_result = ::image::open(&rendered).context("读取预览图片失败");
    let bytes_result = fs::read(&rendered).context("读取预览图片失败");
    let _ = fs::remove_file(&rendered);
    let img = img_result?;
    let bytes = bytes_result?;

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
