use crate::commands::run_blocking;
use crate::external::ExternalTool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildEvidenceGroupPdfsArgs {
    root: String,
    #[serde(default)]
    groups: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeEvidencePdfsArgs {
    evidence_dir: String,
    group_pdfs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOverlayArgs {
    items: Vec<crate::pdf::header_footer::HeaderFooterJob>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewOverlayArgs {
    job: crate::pdf::header_footer::HeaderFooterJob,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    dpi: Option<u32>,
    #[serde(default)]
    annotation_rule: Option<serde_json::Value>,
    // Keep any other payload fields flowing through to the handler.
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyEvidencePdfRulesArgs {
    #[serde(default)]
    items: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    jobs: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    merge: Option<serde_json::Value>,
    #[serde(default)]
    session: Option<serde_json::Value>,
    #[serde(default)]
    annotation_rule: Option<serde_json::Value>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct QpdfStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[tauri::command]
pub async fn check_qpdf() -> QpdfStatus {
    let status = run_blocking(|| Ok(crate::external::QpdfTool.check()))
        .await
        .unwrap_or_else(|_| crate::external::ToolStatus {
            available: false,
            path: None,
            version: None,
            install_hint: "无法检测 qpdf".into(),
            managed: false,
            source: "unknown".into(),
        });
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
pub async fn inspect_pdf(input: String) -> Result<InspectResult, String> {
    let result = run_blocking(move || crate::pdf::qpdf::inspect(&input)).await?;
    Ok(InspectResult {
        encrypted: result.encrypted,
        pages: result.pages,
    })
}

#[derive(Debug, Serialize)]
pub struct UnlockResult {
    pub output_path: String,
    pub skipped: bool,
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
        skipped: result.skipped,
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
    args: crate::pdf::split::SplitMergedArgs,
) -> Result<crate::pdf::split::SplitMergedResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::split::split_merged(&args)).await
}

#[tauri::command]
pub async fn scan_evidence_folder(root: String) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::evidence::scan_folder(&root)).await
}

#[tauri::command]
pub async fn build_evidence_group_pdfs(
    args: BuildEvidenceGroupPdfsArgs,
    conversion_state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
) -> Result<serde_json::Value, String> {
    let state = (*conversion_state).clone();
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::evidence::build_group_pdfs(&args, &state)).await
}

#[tauri::command]
pub async fn merge_evidence_pdfs(args: MergeEvidencePdfsArgs) -> Result<String, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::evidence::merge_all(&args)).await
}

#[tauri::command]
pub async fn overlay_pdf_text(
    args: crate::pdf::header_footer::HeaderFooterJob,
) -> Result<serde_json::Value, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::overlay::overlay_text(&args)).await
}

#[tauri::command]
pub async fn batch_overlay_pdf_text(args: BatchOverlayArgs) -> Result<serde_json::Value, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::overlay::batch_overlay(&args)).await
}

#[tauri::command]
pub async fn apply_evidence_pdf_rules(
    args: ApplyEvidencePdfRulesArgs,
) -> Result<serde_json::Value, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::evidence_session::apply_rules(&args)).await
}

#[tauri::command]
pub async fn preview_pdf_header_footer(
    args: PreviewOverlayArgs,
) -> Result<crate::pdf::overlay::PreviewResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::overlay::preview_overlay(&args)).await
}

#[tauri::command]
pub async fn detect_pdf_header_footer(
    args: crate::pdf::detection::DetectionArgs,
) -> Result<crate::pdf::detection::DetectionResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::detection::detect(&args)).await
}

#[tauri::command]
pub async fn inspect_merged_evidence_pdf(
    args: crate::pdf::detection::SplitSuggestionArgs,
) -> Result<crate::pdf::detection::SplitSuggestionResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::detection::suggest_split_ranges(&args)).await
}

#[tauri::command]
pub async fn delete_pdf_annotations(
    args: crate::pdf::annotations::DeleteAnnotationsArgs,
) -> Result<crate::pdf::annotations::DeleteAnnotationsResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::annotations::delete_annotations(&args)).await
}

#[tauri::command]
pub async fn delete_pdf_header_footer_artifacts(
    args: crate::pdf::artifacts::DeleteHeaderFooterArtifactsArgs,
) -> Result<crate::pdf::artifacts::DeleteHeaderFooterArtifactsResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::artifacts::delete_header_footer_artifacts(&args)).await
}

#[tauri::command]
pub async fn render_pdf_preview(
    args: crate::pdf::preview::PreviewArgs,
) -> Result<crate::pdf::overlay::PreviewResult, String> {
    let args = serde_json::to_value(args).map_err(|e| e.to_string())?;
    run_blocking(move || crate::pdf::overlay::render_preview(&args)).await
}

#[tauri::command]
pub async fn get_pdf_page_count(input: String) -> Result<u32, String> {
    run_blocking(move || crate::pdf::qpdf::page_count(&input)).await
}

#[tauri::command]
pub async fn detect_anti_copy(input: String) -> Result<crate::pdf::anti_ocr::AntiCopyDetection, String> {
    run_blocking(move || crate::pdf::anti_ocr::detect_anti_copy(std::path::Path::new(&input))).await
}

#[tauri::command]
pub async fn apply_anti_copy(input: String, output: String, method: String) -> Result<usize, String> {
    let m = match method.as_str() {
        "cmap_remove" => crate::pdf::anti_ocr::AntiCopyMethod::CmapRemove,
        "text_overlay" => crate::pdf::anti_ocr::AntiCopyMethod::TextOverlay,
        _ => crate::pdf::anti_ocr::AntiCopyMethod::CmapScramble,
    };
    run_blocking(move || {
        crate::pdf::anti_ocr::apply_anti_copy(
            std::path::Path::new(&input),
            std::path::Path::new(&output),
            m,
        )
    })
    .await
}

#[tauri::command]
pub async fn remove_anti_copy(input: String, output: String) -> Result<usize, String> {
    run_blocking(move || {
        crate::pdf::anti_ocr::remove_anti_copy(
            std::path::Path::new(&input),
            std::path::Path::new(&output),
        )
    })
    .await
}
