use anyhow::Result;
use std::path::PathBuf;
use crate::external::ExternalTool;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GenerateArgs {
    pub template_id: String,
    pub values: serde_json::Value,
    pub output_path: Option<String>,
    pub export_pdf: bool,
}

#[derive(Debug, Serialize)]
pub struct GenerateResult {
    pub docx_path: String,
    pub pdf_path: Option<String>,
    pub warnings: Vec<String>,
}

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
        export_pdf(&output_path).ok().map(|p| p.display().to_string())
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
    let doc = crate::docx::model::parse(&docx_bytes)?;
    // Convert to simple HTML for preview
    let html: String = doc.paragraphs.iter().map(|p| {
        let text: String = p.runs.iter().map(|r| r.text.as_str()).collect();
        format!("<p>{}</p>", text)
    }).collect();
    Ok(html)
}

fn default_output_path(template_id: &str) -> PathBuf {
    let _settings = crate::services::history::get_settings().unwrap_or_default();
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

pub fn placeholder_analyze(folder: &str) -> Result<serde_json::Value> {
    let result = crate::image_paddler::analyze(&crate::image_paddler::AnalyzeArgs {
        folder: folder.to_string(),
    })?;
    Ok(serde_json::to_value(result)?)
}

pub fn placeholder_run(args: &serde_json::Value) -> Result<serde_json::Value> {
    let run_args: crate::image_paddler::RunArgs = serde_json::from_value(args.clone())?;
    let result = crate::image_paddler::run(&run_args)?;
    Ok(serde_json::to_value(result)?)
}
