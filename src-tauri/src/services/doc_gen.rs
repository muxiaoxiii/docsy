use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::commands::doc_gen::{GenerateArgs, GenerateResult};
use crate::commands::settings::AppSettings;

pub fn generate(args: GenerateArgs) -> Result<GenerateResult> {
    let tpl = crate::services::template_store::resolve(&args.template_id)?;

    let docx_bytes = std::fs::read(&tpl.docx_path)?;
    let rendered = crate::docx::render::render_document(&docx_bytes, &args.values)?;

    let output_path = args
        .output_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&args.template_id));

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, &rendered)?;

    let pdf_path = if args.export_pdf {
        Some(export_pdf(&output_path)?)
    } else {
        None
    };

    Ok(GenerateResult {
        docx_path: output_path.display().to_string(),
        pdf_path,
        warnings: vec![],
    })
}

pub fn preview(template_id: &str, _values: &serde_json::Value) -> Result<String> {
    let tpl = crate::services::template_store::resolve(template_id)?;
    let docx_bytes = std::fs::read(&tpl.docx_path)?;
    let html = mammoth::convert_to_html(&docx_bytes).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(html)
}

fn default_output_path(template_id: &str) -> PathBuf {
    let settings = crate::services::history::get_settings().unwrap_or_default();
    let base = dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Docsy");
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    base.join(format!("{}-{}.docx", template_id, timestamp))
}

fn export_pdf(docx_path: &PathBuf) -> Result<PathBuf> {
    let pdf_path = docx_path.with_extension("pdf");
    let soffice = crate::external::LibreOfficeTool.binary_path()?;
    let status = std::process::Command::new(&soffice)
        .args(["--headless", "--convert-to", "pdf"])
        .arg("--outdir")
        .arg(docx_path.parent().unwrap_or(&PathBuf::from(".")))
        .arg(docx_path)
        .status()?;
    if !status.success() {
        anyhow::bail!("LibreOffice PDF 转换失败");
    }
    Ok(pdf_path)
}

pub fn placeholder_analyze(_folder: &str) -> Result<serde_json::Value> {
    // TODO: implement image paddler analysis
    anyhow::bail!("图片排版分析功能待实现")
}

pub fn placeholder_run(_args: &serde_json::Value) -> Result<serde_json::Value> {
    // TODO: implement image paddler run
    anyhow::bail!("图片排版生成功能待实现")
}
