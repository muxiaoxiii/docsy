use crate::commands::run_blocking;
use crate::commands::run_managed;
use crate::external::ExternalTool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Emitter;

#[tauri::command]
pub async fn crop_pdf_pages(args: crate::pdf::crop::CropArgs) -> Result<String, String> {
    run_blocking(move || crate::pdf::crop::crop_pdf_pages(&args)).await
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildEvidenceGroupPdfsArgs {
    root: String,
    #[serde(default)]
    groups: Vec<crate::pdf::evidence::GroupConfig>,
    #[serde(default)] output_mode: String,
    #[serde(default)] duplex_files: bool,
    #[serde(default)] duplex_groups: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<crate::pdf::backend::BackendWarning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<crate::pdf::backend::FallbackReason>,
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
    app: tauri::AppHandle,
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    inputs: Vec<String>,
    output: String,
    duplex_separate: Option<bool>,
    conversion_state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
) -> Result<String, String> {
    let duplex = duplex_separate.unwrap_or(false);
    let state = (*conversion_state).clone();
    let app_handle = app.clone();
    run_managed(&manager, "merge_pdfs", None, move |_token| {
        let req = crate::pdf::backend::MergeRequest {
            inputs,
            output_path: std::path::PathBuf::from(output),
            policy: crate::pdf::backend::OutputPolicy::for_policy(
                crate::pdf::backend::MergePolicy::Print,
                duplex,
            ),
            preferred_backend: None,
            conversion_state: Some(state),
        };
        let res = crate::pdf::backend::dispatch_merge(&req)
            .map_err(anyhow::Error::from)?;
        if !res.warnings.is_empty() || res.fallback_reason.is_some() {
            let _ = app_handle.emit("pdf-backend-notice", serde_json::json!({
                "operation": "merge",
                "backend": res.backend,
                "fallbackReason": res.fallback_reason,
                "warnings": res.warnings,
            }));
        }
        Ok(res.output_path.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn convert_image_to_pdf(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    input: String,
) -> Result<String, String> {
    run_managed(&manager, "convert_image_to_pdf", None, move |_token| {
        crate::pdf::image_convert::convert_image_to_pdf(&input)
    }).await
}

#[tauri::command]
pub async fn convert_word_to_pdf(
    input: String,
    output: Option<String>,
    conversion_state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
) -> Result<String, String> {
    let state = (*conversion_state).clone();
    run_blocking(move || {
        if let Some(out) = output {
            let target_path = std::path::PathBuf::from(out);
            crate::pdf::word_convert::convert_word_to_pdf_file(&input, &target_path, &state)
        } else {
            let path = std::path::Path::new(&input);
            let parent = path.parent().unwrap_or(std::path::Path::new("."));
            crate::pdf::word_convert::convert_word_to_pdf(&input, parent, &state)
        }
    })
    .await
}

#[tauri::command]
pub async fn extract_pdf_pages(
    app: tauri::AppHandle,
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    input: String,
    pages: Vec<u32>,
    output_dir: Option<String>,
) -> Result<PdfOutputResult, String> {
    let app_handle = app.clone();
    let result = run_managed(&manager, "extract_pdf_pages", None, move |_token| {
        let req = crate::pdf::backend::ExtractRequest {
            input_path: std::path::PathBuf::from(input),
            pages,
            output_dir: output_dir.map(std::path::PathBuf::from),
            output_path: None,
            preferred_backend: None,
        };
        let res = crate::pdf::backend::dispatch_extract(&req)
            .map_err(anyhow::Error::from)?;
        if !res.warnings.is_empty() || res.fallback_reason.is_some() {
            let _ = app_handle.emit("pdf-backend-notice", serde_json::json!({
                "operation": "extract",
                "backend": res.backend,
                "fallbackReason": res.fallback_reason,
                "warnings": res.warnings,
            }));
        }
        Ok(PdfOutputResult {
            output_path: res.output_path.to_string_lossy().to_string(),
            input_size: res.input_size,
            output_size: res.output_size,
            warnings: res.warnings,
            fallback_reason: res.fallback_reason,
        })
    })
    .await?;
    Ok(result)
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
        warnings: Vec::new(),
        fallback_reason: None,
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
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    args: crate::pdf::split::SplitMergedArgs,
) -> Result<crate::pdf::split::SplitMergedResult, String> {
    run_managed(&manager, "split_merged_evidence_pdf", None, move |_token| {
        crate::pdf::split::split_merged(&args)
    })
    .await
}

#[tauri::command]
pub async fn scan_evidence_folder(root: String) -> Result<serde_json::Value, String> {
    run_blocking(move || crate::pdf::evidence::scan_folder(&root)).await
}

#[tauri::command]
pub async fn build_evidence_group_pdfs(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    args: BuildEvidenceGroupPdfsArgs,
    conversion_state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
) -> Result<serde_json::Value, String> {
    let state = (*conversion_state).clone();
    let build_args = crate::pdf::evidence::BuildGroupPdfsArgs {
        root: args.root,
        groups: args.groups,
        output_mode: args.output_mode,
        duplex_files: args.duplex_files,
        duplex_groups: args.duplex_groups,
    };
    run_managed(&manager, "build_evidence_group_pdfs", None, move |_token| {
        crate::pdf::evidence::build_group_pdfs(&build_args, &state)
    })
    .await
}

#[tauri::command]
pub async fn merge_evidence_pdfs(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    args: MergeEvidencePdfsArgs,
) -> Result<String, String> {
    let merge_args = crate::pdf::evidence::MergeAllArgs {
        evidence_dir: args.evidence_dir,
        group_pdfs: args.group_pdfs,
        output_path: None,
        identity: None,
        overlay: None,
    };
    run_managed(&manager, "merge_evidence_pdfs", None, move |_token| {
        crate::pdf::evidence::merge_all(&merge_args)
    })
    .await
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
) -> Result<crate::pdf::anti_ocr::AntiCopyOutput, String> {
    let m = crate::pdf::anti_ocr::AntiCopyMethod::parse(&method);
    run_blocking(move || {
        crate::pdf::anti_ocr::process_copy(
            std::path::Path::new(&input),
            std::path::Path::new(&output),
            Some(m),
        )
    })
    .await
}

#[tauri::command]
pub async fn remove_anti_copy(input: String, output: String) -> Result<crate::pdf::anti_ocr::AntiCopyOutput, String> {
    run_blocking(move || {
        crate::pdf::anti_ocr::process_copy(
            std::path::Path::new(&input),
            std::path::Path::new(&output),
            None,
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
pub async fn detect_pdf_signatures(
    input: String,
) -> Result<crate::pdf::signatures::SignatureInspection, String> {
    run_blocking(move || crate::pdf::signatures::inspect_signatures(std::path::Path::new(&input)))
        .await
}

#[tauri::command]
pub async fn remove_pdf_bookmarks(input: String) -> Result<(), String> {
    run_blocking(move || {
        crate::pdf::header_footer::remove_pdf_bookmarks(std::path::Path::new(&input))
    })
    .await
}

#[tauri::command]
pub async fn plan_image_pdf(args: crate::pdf::image_convert::ImagePdfArgs) -> Result<crate::pdf::image_convert::ImagePdfPlan, String> {
    run_blocking(move || crate::pdf::image_convert::plan(&args)).await
}
#[tauri::command]
pub async fn export_image_pdf(args: crate::pdf::image_convert::ImagePdfArgs, manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>) -> Result<String, String> {
    run_managed(&manager, "export_image_pdf", None, move |_token| crate::pdf::image_convert::export(&args)).await
}

#[tauri::command]
pub async fn preview_long_slice(
    args: crate::pdf::long_slice::LongSliceArgs,
    preview_page: Option<usize>,
) -> Result<crate::pdf::long_slice::LongSlicePreviewResult, String> {
    run_blocking(move || {
        crate::pdf::long_slice::preview_long_slice(&args, preview_page.unwrap_or(1))
    })
    .await
}

#[tauri::command]
pub async fn process_long_slice(
    args: crate::pdf::long_slice::LongSliceArgs,
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
) -> Result<crate::pdf::long_slice::LongSliceResult, String> {
    run_managed(&manager, "process_long_slice", None, move |_token| {
        crate::pdf::long_slice::process_long_slice(&args)
    })
    .await
}

#[tauri::command]
pub async fn get_evidence_catalog_fingerprint(path: String) -> Result<String, String> {
    run_blocking(move || {
        let meta = std::fs::metadata(path)?;
        let modified = meta.modified()?.duration_since(std::time::UNIX_EPOCH)?;
        Ok(format!("{}:{}:{}", meta.len(), modified.as_secs(), modified.subsec_nanos()))
    }).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceCatalogRow {
    number: String,
    name: String,
    page_range: String,
}

#[tauri::command]
pub async fn export_evidence_catalog(output_path: String, rows: Vec<EvidenceCatalogRow>, source_paths: Vec<String>) -> Result<(), String> {
    run_blocking(move || {
        let output = std::path::Path::new(&output_path);
        if output.exists() {
            let canonical_output = std::fs::canonicalize(output)?;
            for source in source_paths {
                if std::fs::canonicalize(source).is_ok_and(|p| p == canonical_output) {
                    anyhow::bail!("不能覆盖已导入的源文件，请选择其他保存位置");
                }
            }
        }
        write_evidence_catalog(&output_path, &rows)
    }).await
}

fn write_evidence_catalog(output_path: &str, rows: &[EvidenceCatalogRow]) -> anyhow::Result<()> {
    use docx_rs::{AlignmentType, Docx, HeightRule, PageMargin, PageOrientationType, Paragraph, Run, RunFonts, Shading, Table, TableCell, TableRow, VAlignType, WidthType};
    use std::path::Path;
    if rows.is_empty() { anyhow::bail!("证据列表为空"); }
    let path = Path::new(output_path);
    if !path.extension().is_some_and(|s| s.eq_ignore_ascii_case("docx")) { anyhow::bail!("请选择 .docx 文件"); }
    let widths = [702, 1833, 4033, 4880, 1415, 1040];
    let text_run = |text: &str| Run::new().fonts(RunFonts::new().ascii("Times New Roman").hi_ansi("Times New Roman").east_asia("Songti SC")).add_text(text);
    let make_row = |values: &[&str], header: bool| {
        let cells = values.iter().enumerate().map(|(i, text)| {
            let run = text_run(text).size(22);
            let run = if header { run.bold() } else { run };
            let para = Paragraph::new().align(if header || i == 0 || i == 5 { AlignmentType::Center } else { AlignmentType::Left }).add_run(run);
            let cell = TableCell::new().width(widths[i], WidthType::Dxa).vertical_align(VAlignType::Center).add_paragraph(para);
            if header { cell.shading(Shading::new().fill("D9E0E9")) } else { cell }
        }).collect();
        TableRow::new(cells).row_height(if header { 400.0 } else { 700.0 }).height_rule(HeightRule::AtLeast)
    };
    let mut table_rows = vec![make_row(&["编号", "证据名称", "证据内容", "证明目的", "证据来源", "页码"], true)];
    for row in rows { table_rows.push(make_row(&[&row.number, &row.name, "", "", "", &row.page_range], false)); }
    let doc = Docx::new()
        .page_size(16838, 11906).page_orient(PageOrientationType::Landscape)
        .page_margin(PageMargin { top: 1800, right: 1440, bottom: 1800, left: 1440, header: 0, footer: 0, gutter: 0 })
        .add_paragraph(Paragraph::new().align(AlignmentType::Center).add_run(text_run("证  据  目  录").size(36).bold()))
        .add_paragraph(Paragraph::new().add_run(text_run("提交人：").size(22)))
        .add_table(Table::new(table_rows).set_grid(widths.to_vec()))
        .add_paragraph(Paragraph::new().add_run(text_run("提交人：").size(22)))
        .add_paragraph(Paragraph::new().add_run(text_run("代理人：").size(22)))
        .add_paragraph(Paragraph::new().add_run(text_run("日期：").size(22)));
    let mut output = std::fs::File::create(path)?;
    doc.build().pack(&mut output)?;
    Ok(())
}

#[cfg(test)]
mod evidence_catalog_tests {
    use super::*;
    #[test]
    fn writes_editable_catalog() {
        let path = std::env::temp_dir().join("docsy-evidence-catalog-test.docx");
        let rows = vec![EvidenceCatalogRow { number: "1".into(), name: "证据文件名称示例".into(), page_range: "1–3".into() }, EvidenceCatalogRow { number: "2".into(), name: "合同和附件".into(), page_range: "5–6".into() }];
        write_evidence_catalog(path.to_str().unwrap(), &rows).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
        for text in ["证据文件名称示例", "5–6", "证明目的"] { assert!(xml.contains(text)); }
        if std::env::var_os("DOCSY_KEEP_CATALOG_FIXTURE").is_none() { std::fs::remove_file(path).unwrap(); }
    }
}
