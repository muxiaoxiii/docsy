use serde::Serialize;
use crate::external::ExternalTool;

#[derive(Debug, Serialize)]
pub struct QpdfStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[tauri::command]
pub fn check_qpdf() -> QpdfStatus {
    let status = crate::external::QpdfTool.check();
    QpdfStatus {
        available: status.available,
        path: status.path,
        version: status.version,
    }
}

#[derive(Debug, Serialize)]
pub struct InspectResult {
    pub encrypted: bool,
    pub pages: Option<u32>,
}

#[tauri::command]
pub fn inspect_pdf(input: String) -> Result<InspectResult, String> {
    let result = crate::pdf::qpdf::inspect(&input).map_err(|e| e.to_string())?;
    Ok(InspectResult {
        encrypted: result.encrypted,
        pages: result.pages,
    })
}

#[derive(Debug, Serialize)]
pub struct UnlockResult {
    pub output_path: String,
}

#[tauri::command]
pub fn unlock_pdf(input: String) -> Result<UnlockResult, String> {
    let result = crate::pdf::qpdf::unlock(&std::path::PathBuf::from(&input)).map_err(|e| e.to_string())?;
    Ok(UnlockResult { output_path: result.output_path })
}

#[tauri::command]
pub fn merge_pdfs(inputs: Vec<String>, output: String) -> Result<String, String> {
    crate::pdf::qpdf::merge(&inputs, &output).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn split_pdf(input: String, output_dir: String) -> Result<Vec<String>, String> {
    crate::pdf::qpdf::split(&input, &output_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_evidence_folder(root: String) -> Result<serde_json::Value, String> {
    crate::pdf::evidence::scan_folder(&root).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn build_evidence_group_pdfs(args: serde_json::Value) -> Result<serde_json::Value, String> {
    crate::pdf::evidence::build_group_pdfs(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn merge_evidence_pdfs(args: serde_json::Value) -> Result<String, String> {
    crate::pdf::evidence::merge_all(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn overlay_pdf_text(args: serde_json::Value) -> Result<String, String> {
    crate::pdf::overlay::overlay_text(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_overlay_pdf_text(args: serde_json::Value) -> Result<Vec<String>, String> {
    crate::pdf::overlay::batch_overlay(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pdf_page_count(input: String) -> Result<u32, String> {
    crate::pdf::qpdf::page_count(&input).map_err(|e| e.to_string())
}
