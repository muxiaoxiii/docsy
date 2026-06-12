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

#[tauri::command]
pub fn generate_document(args: GenerateArgs) -> Result<GenerateResult, String> {
    crate::services::doc_gen::generate(args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preview_document(template_id: String, values: serde_json::Value) -> Result<String, String> {
    crate::services::doc_gen::preview(&template_id, &values).map_err(|e| e.to_string())
}
