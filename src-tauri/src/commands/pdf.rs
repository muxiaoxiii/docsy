use crate::commands::run_blocking;
use crate::commands::run_managed;
use crate::external::ExternalTool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildEvidenceGroupPdfsArgs {
    root: String,
    #[serde(default)]
    groups: Vec<crate::pdf::evidence::GroupConfig>,
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
    pub items: Option<Vec<crate::pdf::header_footer::HeaderFooterJob>>,
    #[serde(default)]
    pub jobs: Option<Vec<crate::pdf::header_footer::HeaderFooterJob>>,
    #[serde(default)]
    pub merge: Option<serde_json::Value>,
    #[serde(default)]
    pub session: Option<serde_json::Value>,
    #[serde(default)]
    pub annotation_rule: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
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
    pub input_size: u64,
    pub output_size: u64,
}

#[derive(Debug, Serialize)]
pub struct OptimizeResult {
    pub output_path: String,
    pub input_size: u64,
    pub output_size: u64,
    pub changed: bool,
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
pub async fn merge_pdfs(
    inputs: Vec<String>,
    output: String,
    duplex_separate: Option<bool>,
) -> Result<String, String> {
    let duplex = duplex_separate.unwrap_or(false);
    run_blocking(move || crate::pdf::qpdf::merge(&inputs, &output, duplex)).await
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
        input_size: result.input_size,
        output_size: result.output_size,
    })
}

#[tauri::command]
pub async fn compress_pdf(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    input: String,
    output_dir: Option<String>,
    level: Option<u8>,
) -> Result<PdfOutputResult, String> {
    let progress_manager = manager.inner().clone();
    let result = run_managed(&manager, "compress_pdf", None, move |_token| {
        crate::pdf::qpdf::compress_with_progress(&input, output_dir.as_deref(), level, |progress| {
            progress_manager.update("compress_pdf:auto", progress.label());
        })
    })
    .await?;
    Ok(PdfOutputResult {
        output_path: result.output_path,
        input_size: result.input_size,
        output_size: result.output_size,
    })
}

#[tauri::command]
pub async fn optimize_pdf_lossless(
    input: String,
    output_dir: Option<String>,
) -> Result<OptimizeResult, String> {
    let result = run_blocking(move || {
        crate::pdf::qpdf::optimize_lossless_to_dir(
            &input,
            output_dir.as_deref().map(std::path::Path::new),
        )
    })
    .await?;
    Ok(OptimizeResult {
        output_path: result.output_path,
        input_size: result.input_size,
        output_size: result.output_size,
        changed: result.changed,
    })
}

#[tauri::command]
pub async fn split_merged_evidence_pdf(
    args: crate::pdf::split::SplitMergedArgs,
) -> Result<crate::pdf::split::SplitMergedResult, String> {
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
    let build_args = crate::pdf::evidence::BuildGroupPdfsArgs {
        root: args.root,
        groups: args.groups,
    };
    run_blocking(move || crate::pdf::evidence::build_group_pdfs(&build_args, &state)).await
}

#[tauri::command]
pub async fn merge_evidence_pdfs(args: MergeEvidencePdfsArgs) -> Result<String, String> {
    let merge_args = crate::pdf::evidence::MergeAllArgs {
        evidence_dir: args.evidence_dir,
        group_pdfs: args.group_pdfs,
        output_path: None,
        identity: None,
        overlay: None,
    };
    run_blocking(move || crate::pdf::evidence::merge_all(&merge_args)).await
}

#[tauri::command]
pub async fn overlay_pdf_text(
    args: crate::pdf::header_footer::HeaderFooterJob,
) -> Result<crate::pdf::header_footer::HeaderFooterResult, String> {
    run_blocking(move || crate::pdf::header_footer::overlay_text(&args)).await
}

#[tauri::command]
pub async fn batch_overlay_pdf_text(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    args: BatchOverlayArgs,
) -> Result<crate::pdf::header_footer::BatchHeaderFooterResult, String> {
    let progress_manager = manager.inner().clone();
    run_managed(&manager, "batch_overlay_pdf_text", None, move |token| {
        crate::pdf::header_footer::batch_overlay_cancellable(
            &args.items,
            &token,
            &|index, total, input| {
                let name = input.rsplit(['/', '\\']).next().unwrap_or(input);
                progress_manager.update(
                    "batch_overlay_pdf_text:auto",
                    format!("正在处理 {index}/{total}:{name}"),
                );
            },
        )
    })
    .await
}

#[tauri::command]
pub async fn apply_evidence_pdf_rules(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    args: ApplyEvidencePdfRulesArgs,
) -> Result<crate::pdf::evidence_session::ApplyRulesResult, String> {
    let progress_manager = manager.inner().clone();
    run_managed(&manager, "apply_evidence_pdf_rules", None, move |token| {
        crate::pdf::evidence_session::apply_rules_cancellable(&args, &token, &|label| {
            progress_manager.update("apply_evidence_pdf_rules:auto", label);
        })
    })
    .await
}

#[tauri::command]
pub async fn preview_pdf_header_footer(
    args: PreviewOverlayArgs,
) -> Result<crate::pdf::preview::PreviewResult, String> {
    let annotation_rule: Option<crate::pdf::header_footer::PreviewAnnotationRule> = args
        .annotation_rule
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?;
    run_blocking(move || {
        crate::pdf::header_footer::preview_overlay(
            &args.job,
            args.page,
            args.dpi,
            annotation_rule.as_ref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn detect_pdf_header_footer(
    args: crate::pdf::detection::DetectionArgs,
) -> Result<crate::pdf::detection::DetectionResult, String> {
    run_blocking(move || crate::pdf::detection::detect(&args)).await
}

#[tauri::command]
pub async fn inspect_merged_evidence_pdf(
    args: crate::pdf::detection::SplitSuggestionArgs,
) -> Result<crate::pdf::detection::SplitSuggestionResult, String> {
    run_blocking(move || crate::pdf::detection::suggest_split_ranges(&args)).await
}

#[tauri::command]
pub async fn delete_pdf_annotations(
    args: crate::pdf::annotations::DeleteAnnotationsArgs,
) -> Result<crate::pdf::annotations::DeleteAnnotationsResult, String> {
    run_blocking(move || crate::pdf::annotations::delete_annotations(&args)).await
}

#[tauri::command]
pub async fn delete_pdf_header_footer_artifacts(
    args: crate::pdf::artifacts::DeleteHeaderFooterArtifactsArgs,
) -> Result<crate::pdf::artifacts::DeleteHeaderFooterArtifactsResult, String> {
    run_blocking(move || crate::pdf::artifacts::delete_header_footer_artifacts(&args)).await
}

#[tauri::command]
pub async fn render_pdf_preview(
    args: crate::pdf::preview::PreviewArgs,
) -> Result<crate::pdf::preview::PreviewResult, String> {
    run_blocking(move || crate::pdf::preview::render_preview(&args)).await
}

#[tauri::command]
pub async fn get_pdf_page_count(input: String) -> Result<u32, String> {
    run_blocking(move || crate::pdf::qpdf::page_count(&input)).await
}

#[tauri::command]
pub async fn detect_anti_copy(
    input: String,
) -> Result<crate::pdf::anti_ocr::AntiCopyDetection, String> {
    run_blocking(move || crate::pdf::anti_ocr::detect_anti_copy(std::path::Path::new(&input))).await
}

#[tauri::command]
pub async fn apply_anti_copy(
    input: String,
    output: String,
    method: String,
) -> Result<usize, String> {
    let m = crate::pdf::anti_ocr::AntiCopyMethod::parse(&method);
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

#[tauri::command]
pub async fn has_pdf_bookmarks(input: String) -> Result<bool, String> {
    run_blocking(move || crate::pdf::header_footer::has_pdf_bookmarks(std::path::Path::new(&input)))
        .await
}

#[tauri::command]
pub async fn remove_pdf_bookmarks(input: String) -> Result<(), String> {
    run_blocking(move || {
        crate::pdf::header_footer::remove_pdf_bookmarks(std::path::Path::new(&input))
    })
    .await
}
