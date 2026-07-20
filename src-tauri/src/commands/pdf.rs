use crate::commands::run_blocking;
use crate::external::ExternalTool;
use serde::Serialize;

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

#[derive(Debug, Serialize)]
pub struct PdfOutputResult {
    pub output_path: String,
}

#[tauri::command]
pub async fn unlock_pdf(input: String) -> Result<UnlockResult, String> {
    let result =
        run_blocking(move || crate::pdf::qpdf::unlock(&std::path::PathBuf::from(&input))).await?;
    Ok(UnlockResult {
        output_path: result.output_path,
    })
}

#[tauri::command]
pub async fn merge_pdfs(inputs: Vec<String>, output: String) -> Result<String, String> {
    run_blocking(move || crate::pdf::qpdf::merge(&inputs, &output)).await
}

#[tauri::command]
pub async fn split_pdf(input: String, output_dir: String) -> Result<Vec<String>, String> {
    run_blocking(move || crate::pdf::qpdf::split(&input, &output_dir)).await
}

#[tauri::command]
pub async fn extract_pdf_pages(
    input: String,
    pages: Vec<u32>,
    output_dir: Option<String>,
) -> Result<PdfOutputResult, String> {
    let result = run_blocking(move || {
        crate::pdf::qpdf::extract_pages(&input, &pages, output_dir.as_deref())
    })
    .await?;
    Ok(PdfOutputResult {
        output_path: result.output_path,
    })
}

#[tauri::command]
pub async fn compress_pdf(
    input: String,
    output_dir: Option<String>,
) -> Result<PdfOutputResult, String> {
    let result =
        run_blocking(move || crate::pdf::qpdf::compress(&input, output_dir.as_deref())).await?;
    Ok(PdfOutputResult {
        output_path: result.output_path,
    })
}

#[tauri::command]
pub async fn split_merged_evidence_pdf(
    args: serde_json::Value,
) -> Result<crate::pdf::split::SplitMergedResult, String> {
    run_blocking(move || crate::pdf::split::split_merged(&args)).await
}

#[tauri::command]
pub async fn scan_evidence_folder(root: String) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::evidence::scan_folder(&root)).await
}

#[tauri::command]
pub async fn build_evidence_group_pdfs(
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::evidence::build_group_pdfs(&args)).await
}

#[tauri::command]
pub async fn merge_evidence_pdfs(args: serde_json::Value) -> Result<String, String> {
    run_blocking(move || crate::pdf::evidence::merge_all(&args)).await
}

#[tauri::command]
pub async fn overlay_pdf_text(args: serde_json::Value) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::overlay::overlay_text(&args)).await
}

#[tauri::command]
pub async fn batch_overlay_pdf_text(args: serde_json::Value) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::overlay::batch_overlay(&args)).await
}

#[tauri::command]
pub async fn apply_evidence_pdf_rules(
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::evidence_session::apply_rules(&args)).await
}

#[tauri::command]
pub async fn preview_pdf_header_footer(
    args: serde_json::Value,
) -> Result<crate::pdf::overlay::PreviewResult, String> {
    run_blocking(move || crate::pdf::overlay::preview_overlay(&args)).await
}

#[tauri::command]
pub async fn detect_pdf_header_footer(
    args: serde_json::Value,
) -> Result<crate::pdf::detection::DetectionResult, String> {
    run_blocking(move || crate::pdf::detection::detect(&args)).await
}

#[tauri::command]
pub async fn inspect_merged_evidence_pdf(
    args: serde_json::Value,
) -> Result<crate::pdf::detection::SplitSuggestionResult, String> {
    run_blocking(move || crate::pdf::detection::suggest_split_ranges(&args)).await
}

#[tauri::command]
pub fn delete_pdf_annotations(
    args: serde_json::Value,
) -> Result<crate::pdf::annotations::DeleteAnnotationsResult, String> {
    crate::pdf::annotations::delete_annotations(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_pdf_header_footer_artifacts(
    args: serde_json::Value,
) -> Result<crate::pdf::artifacts::DeleteHeaderFooterArtifactsResult, String> {
    crate::pdf::artifacts::delete_header_footer_artifacts(&args).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn render_pdf_preview(
    args: serde_json::Value,
) -> Result<crate::pdf::overlay::PreviewResult, String> {
    run_blocking(move || crate::pdf::overlay::render_preview(&args)).await
}

#[tauri::command]
pub fn get_pdf_page_count(input: String) -> Result<u32, String> {
    crate::pdf::qpdf::page_count(&input).map_err(|e| e.to_string())
}
