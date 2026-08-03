use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

use super::page_info::get_page_infos;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewArgs {
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
            let repaired = temp_named_path("docsy_pdf_preview_repaired").with_extension("pdf");
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
    let prefix = temp_named_path("docsy_pdf_preview");
    let output = PathBuf::from(format!("{}.png", prefix.display()));

    let command_output = crate::external::hidden_command(pdftoppm)
        .arg("-png")
        .arg("-singlefile")
        .arg("-r")
        .arg(dpi.to_string())
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(input)
        .arg(&prefix)
        .output()
        .context("执行 pdftoppm 失败")?;

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

fn temp_named_path(prefix: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}"))
}
