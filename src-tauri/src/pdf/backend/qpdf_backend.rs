use super::policy::{BackendWarning, MergePolicy, PreflightDocument, PreflightSummary};
use super::{BackendError, BackendType, ExtractRequest, ExtractResult, MergeRequest, MergeResult, PdfBackend};
use std::path::Path;

/// 基于成熟外部工具 qpdf 的生产默认后端。
pub struct QpdfBackend;

impl QpdfBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QpdfBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfBackend for QpdfBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Qpdf
    }

    fn preflight_merge(&self, req: &MergeRequest) -> Result<PreflightSummary, BackendError> {
        let mut summary = PreflightSummary::new();
        for input in &req.inputs {
            let path = Path::new(input);
            if !path.exists() {
                return Err(BackendError::Other(format!("输入文件不存在: {input}")));
            }
            if crate::pdf::word_convert::is_word_file(input) {
                summary.add_document(PreflightDocument {
                    path: path.to_path_buf(),
                    page_count: 0,
                    is_encrypted: false,
                    has_signatures: false,
                    has_bookmarks: false,
                    has_attachments: false,
                    has_form_fields: false,
                    has_anticopy_backup: false,
                    anticopy_version: None,
                });
                continue;
            }
            let is_enc = crate::pdf::qpdf::inspect(input)
                .map(|i| i.encrypted)
                .unwrap_or(false);
            let count = crate::pdf::qpdf::page_count(input).unwrap_or(0);
            let has_sigs = crate::pdf::signatures::inspect_signatures(path)
                .map(|s| s.has_signatures)
                .unwrap_or(false);
            let has_bm = crate::pdf::bookmarks::has_pdf_bookmarks(path).unwrap_or(false);
            let mut has_anticopy = false;
            let mut anticopy_ver = None;
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(doc) = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)) {
                    if let Some(text) = pdfcraft_organize::info(&doc, "DocsyAntiCopyBackup") {
                        has_anticopy = true;
                        let stripped = text.strip_prefix("u:").unwrap_or(&text);
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stripped) {
                            anticopy_ver = json.get("version").and_then(|v| v.as_u64());
                        }
                    }
                }
            }
            summary.add_document(PreflightDocument {
                path: path.to_path_buf(),
                page_count: count,
                is_encrypted: is_enc,
                has_signatures: has_sigs,
                has_bookmarks: has_bm,
                has_attachments: false,
                has_form_fields: false,
                has_anticopy_backup: has_anticopy,
                anticopy_version: anticopy_ver,
            });
        }
        Ok(summary)
    }

    fn preflight_extract(&self, req: &ExtractRequest) -> Result<PreflightSummary, BackendError> {
        let mut summary = PreflightSummary::new();
        let path = &req.input_path;
        if !path.exists() {
            return Err(BackendError::Other(format!(
                "输入文件不存在: {}",
                path.display()
            )));
        }
        let input_str = path.to_string_lossy();
        let is_enc = crate::pdf::qpdf::inspect(&input_str)
            .map(|i| i.encrypted)
            .unwrap_or(false);
        let count = crate::pdf::qpdf::page_count(&input_str).unwrap_or(0);
        let has_sigs = crate::pdf::signatures::inspect_signatures(path)
            .map(|s| s.has_signatures)
            .unwrap_or(false);
        let has_bm = crate::pdf::bookmarks::has_pdf_bookmarks(path).unwrap_or(false);
        let mut has_anticopy = false;
        let mut anticopy_ver = None;
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(doc) = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)) {
                if let Some(text) = pdfcraft_organize::info(&doc, "DocsyAntiCopyBackup") {
                    has_anticopy = true;
                    let stripped = text.strip_prefix("u:").unwrap_or(&text);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stripped) {
                        anticopy_ver = json.get("version").and_then(|v| v.as_u64());
                    }
                }
            }
        }
        if has_bm {
            summary.warnings.push(BackendWarning::StructureOmittedOnExtract {
                path: path.display().to_string(),
                item: "Bookmarks".to_string(),
            });
        }
        summary.add_document(PreflightDocument {
            path: path.clone(),
            page_count: count,
            is_encrypted: is_enc,
            has_signatures: has_sigs,
            has_bookmarks: has_bm,
            has_attachments: false,
            has_form_fields: false,
            has_anticopy_backup: has_anticopy,
            anticopy_version: anticopy_ver,
        });
        Ok(summary)
    }

    fn merge(&self, req: &MergeRequest) -> Result<MergeResult, BackendError> {
        if req.policy.merge_policy == MergePolicy::Structural {
            return Err(BackendError::Unsupported(
                "Qpdf 生产引擎仅提供打印外观压平合并，不支持保留交互表单等结构语义".to_string(),
            ));
        }

        let state = req
            .conversion_state
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(crate::ConversionState::new()));
        let preflight = self.preflight_merge(req).unwrap_or_default();

        let out_str = req.output_path.to_string_lossy();
        let final_path_str = crate::pdf::qpdf::merge_with_state(
            &req.inputs,
            &out_str,
            req.policy.duplex_separate,
            &state,
        )
        .map_err(|e| {
            if crate::operations::check_current_cancelled().is_err() {
                BackendError::Cancelled
            } else {
                BackendError::ExecutionError {
                    backend: BackendType::Qpdf,
                    message: e.to_string(),
                }
            }
        })?;

        let total_pages = crate::pdf::qpdf::page_count(&final_path_str).unwrap_or(0);

        Ok(MergeResult {
            output_path: std::path::PathBuf::from(final_path_str),
            backend: BackendType::Qpdf,
            total_pages,
            warnings: preflight.warnings,
            renamed_fields: Vec::new(),
            fallback_reason: None,
        })
    }

    fn extract(&self, req: &ExtractRequest) -> Result<ExtractResult, BackendError> {
        let input_str = req.input_path.to_string_lossy();
        let preflight = self.preflight_extract(req).unwrap_or_default();
        let out_dir_str = req
            .output_dir
            .as_ref()
            .map(|d| d.to_string_lossy().to_string());

        let res = crate::pdf::qpdf::extract_pages_with_target(
            &input_str,
            &req.pages,
            out_dir_str.as_deref(),
            req.output_path.as_deref(),
        )
        .map_err(|e| {
            if crate::operations::check_current_cancelled().is_err() {
                BackendError::Cancelled
            } else {
                BackendError::ExecutionError {
                    backend: BackendType::Qpdf,
                    message: e.to_string(),
                }
            }
        })?;

        let total_pages = req.pages.len() as u32;

        Ok(ExtractResult {
            output_path: std::path::PathBuf::from(res.output_path),
            backend: BackendType::Qpdf,
            total_pages,
            input_size: res.input_size,
            output_size: res.output_size,
            warnings: preflight.warnings,
            fallback_reason: None,
        })
    }
}
