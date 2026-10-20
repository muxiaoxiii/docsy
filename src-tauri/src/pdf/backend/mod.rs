pub mod extract_poc;
pub mod flatten;
pub mod marks;
pub mod pdfcraft_backend;
pub mod policy;
pub mod qpdf_backend;

#[allow(unused_imports)]
pub use marks::{DocsyMarkFilter, DocsyMarkKind, DocsyMarkSpec, DocsyMarkSummary};
pub use pdfcraft_backend::PdfCraftBackend;
pub use policy::{BackendWarning, FallbackReason, MergePolicy, OutputPolicy, PreflightSummary};
pub use qpdf_backend::QpdfBackend;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// 支持的 PDF 处理后端类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    /// 成熟稳定的 qpdf 生产默认后端。
    Qpdf,
    /// 纯 Rust pdfcraft 原生页面组织候选后端。
    PdfCraft,
}

impl Default for BackendType {
    fn default() -> Self {
        Self::PdfCraft
    }
}

/// 后端统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("操作已取消")]
    Cancelled,
    #[error("不支持的 PDF 特性: {0}")]
    Unsupported(String),
    #[error("PDF 解析失败: {0}")]
    ParseError(String),
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("后端执行错误 ({backend:?}): {message}")]
    ExecutionError {
        backend: BackendType,
        message: String,
    },
    #[error("{0}")]
    Other(String),
}

/// 合并请求结构体。
#[derive(Clone)]
pub struct MergeRequest {
    pub inputs: Vec<String>,
    pub output_path: PathBuf,
    pub policy: OutputPolicy,
    pub preferred_backend: Option<BackendType>,
    pub conversion_state: Option<Arc<crate::ConversionState>>,
}

/// 合并执行结果结构体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub output_path: PathBuf,
    pub backend: BackendType,
    pub total_pages: u32,
    pub warnings: Vec<BackendWarning>,
    pub renamed_fields: Vec<(String, String)>,
    pub fallback_reason: Option<FallbackReason>,
}

/// 提取请求结构体（页码对外统一为 1-based）。
#[derive(Clone)]
pub struct ExtractRequest {
    pub input_path: PathBuf,
    pub pages: Vec<u32>,
    pub output_dir: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub preferred_backend: Option<BackendType>,
}

/// 提取执行结果结构体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub output_path: PathBuf,
    pub backend: BackendType,
    pub total_pages: u32,
    pub input_size: u64,
    pub output_size: u64,
    pub warnings: Vec<BackendWarning>,
    pub fallback_reason: Option<FallbackReason>,
}

/// PDF 后端统一接口 trait。
pub trait PdfBackend: Send + Sync {
    #[allow(dead_code)]
    fn backend_type(&self) -> BackendType;
    fn preflight_merge(&self, req: &MergeRequest) -> Result<PreflightSummary, BackendError>;
    fn preflight_extract(&self, req: &ExtractRequest) -> Result<PreflightSummary, BackendError>;
    fn merge(&self, req: &MergeRequest) -> Result<MergeResult, BackendError>;
    fn extract(&self, req: &ExtractRequest) -> Result<ExtractResult, BackendError>;
}

/// 从环境变量中获取后端偏好。
///
/// 默认启用纯 Rust `PdfCraft` 原生优先后端（失败或命中复杂特性时自动回退 `Qpdf`）；
/// 设置 `DOCSY_PDF_BACKEND=qpdf` 可强制走外置 `Qpdf` 后端。
pub fn backend_type_from_env() -> Option<BackendType> {
    match std::env::var("DOCSY_PDF_BACKEND").as_deref() {
        Ok("pdfcraft") | Ok("pdfcraft_structural") => Some(BackendType::PdfCraft),
        Ok("qpdf") => Some(BackendType::Qpdf),
        _ => None,
    }
}

/// 确定最终选用的后端（默认为 PdfCraft 原生优先，支持自动回退 Qpdf）。
pub fn select_backend(preferred: Option<BackendType>) -> BackendType {
    preferred
        .or_else(backend_type_from_env)
        .unwrap_or(BackendType::PdfCraft)
}

/// 获取指定后端实例。
#[allow(dead_code)]
pub fn get_backend(backend_type: BackendType) -> Box<dyn PdfBackend> {
    match backend_type {
        BackendType::Qpdf => Box::new(QpdfBackend::new()),
        BackendType::PdfCraft => Box::new(PdfCraftBackend::new()),
    }
}

/// 分派执行合并，具备候选后端预检与严格受控回退机制。
pub fn dispatch_merge(req: &MergeRequest) -> Result<MergeResult, BackendError> {
    let chosen_type = select_backend(req.preferred_backend);

    if chosen_type == BackendType::PdfCraft {
        let candidate = PdfCraftBackend::new();
        let qpdf = QpdfBackend::new();

        // 步骤 1：候选后端预检
        let preflight = candidate.preflight_merge(req);
        let preflight_warnings = match preflight {
            Ok(summary) => {
                let reason_opt = summary.should_fallback_for_candidate(&req.policy);
                let warnings = summary.warnings;
                if let Some(reason) = reason_opt {
                    let mut fallback_req = req.clone();
                    fallback_req.preferred_backend = Some(BackendType::Qpdf);
                    let mut res = qpdf.merge(&fallback_req)?;
                    res.fallback_reason = Some(reason.clone());
                    res.warnings.splice(0..0, warnings);
                    res.warnings.push(BackendWarning::FallbackTriggered {
                        from: "PdfCraft".to_string(),
                        to: "Qpdf".to_string(),
                        reason: reason.to_string(),
                    });
                    return Ok(res);
                }
                warnings
            }
            Err(BackendError::Cancelled) => return Err(BackendError::Cancelled),
            Err(err) => {
                if req.policy.merge_policy == MergePolicy::Structural {
                    return Err(err);
                }
                let reason = FallbackReason::PreflightRefused(err.to_string());
                let mut fallback_req = req.clone();
                fallback_req.preferred_backend = Some(BackendType::Qpdf);
                let mut res = qpdf.merge(&fallback_req)?;
                res.fallback_reason = Some(reason.clone());
                res.warnings.push(BackendWarning::FallbackTriggered {
                    from: "PdfCraft".to_string(),
                    to: "Qpdf".to_string(),
                    reason: reason.to_string(),
                });
                return Ok(res);
            }
        };

        // 步骤 2：执行候选合并
        match candidate.merge(req) {
            Ok(mut res) => {
                res.warnings.splice(0..0, preflight_warnings);
                Ok(res)
            }
            Err(BackendError::Cancelled) => Err(BackendError::Cancelled),
            Err(BackendError::IoError(e)) => Err(BackendError::IoError(e)),
            Err(err) => {
                if req.policy.merge_policy == MergePolicy::Structural {
                    return Err(err);
                }
                let reason = FallbackReason::CandidateExecutionFailed(err.to_string());
                let mut fallback_req = req.clone();
                fallback_req.preferred_backend = Some(BackendType::Qpdf);
                let mut res = qpdf.merge(&fallback_req)?;
                res.fallback_reason = Some(reason.clone());
                res.warnings.splice(0..0, preflight_warnings);
                res.warnings.push(BackendWarning::FallbackTriggered {
                    from: "PdfCraft".to_string(),
                    to: "Qpdf".to_string(),
                    reason: reason.to_string(),
                });
                Ok(res)
            }
        }
    } else {
        let backend = QpdfBackend::new();
        backend.merge(req)
    }
}

/// 分派执行页面提取，具备严格受控回退机制。
pub fn dispatch_extract(req: &ExtractRequest) -> Result<ExtractResult, BackendError> {
    let chosen_type = select_backend(req.preferred_backend);

    if chosen_type == BackendType::PdfCraft {
        let candidate = PdfCraftBackend::new();
        let qpdf = QpdfBackend::new();

        let preflight = candidate.preflight_extract(req);
        let preflight_warnings = match preflight {
            Ok(summary) => {
                let any_enc = summary.any_encrypted;
                let warnings = summary.warnings;
                if any_enc {
                    let reason = FallbackReason::UnsupportedEncryption;
                    let mut fallback_req = req.clone();
                    fallback_req.preferred_backend = Some(BackendType::Qpdf);
                    let mut res = qpdf.extract(&fallback_req)?;
                    res.fallback_reason = Some(reason.clone());
                    res.warnings.splice(0..0, warnings);
                    res.warnings.push(BackendWarning::FallbackTriggered {
                        from: "PdfCraft".to_string(),
                        to: "Qpdf".to_string(),
                        reason: reason.to_string(),
                    });
                    return Ok(res);
                }
                warnings
            }
            Err(BackendError::Cancelled) => return Err(BackendError::Cancelled),
            Err(err) => {
                let reason = FallbackReason::PreflightRefused(err.to_string());
                let mut fallback_req = req.clone();
                fallback_req.preferred_backend = Some(BackendType::Qpdf);
                let mut res = qpdf.extract(&fallback_req)?;
                res.fallback_reason = Some(reason.clone());
                res.warnings.push(BackendWarning::FallbackTriggered {
                    from: "PdfCraft".to_string(),
                    to: "Qpdf".to_string(),
                    reason: reason.to_string(),
                });
                return Ok(res);
            }
        };

        match candidate.extract(req) {
            Ok(mut res) => {
                res.warnings.splice(0..0, preflight_warnings);
                Ok(res)
            }
            Err(BackendError::Cancelled) => Err(BackendError::Cancelled),
            Err(BackendError::IoError(e)) => Err(BackendError::IoError(e)),
            Err(err) => {
                let reason = FallbackReason::CandidateExecutionFailed(err.to_string());
                let mut fallback_req = req.clone();
                fallback_req.preferred_backend = Some(BackendType::Qpdf);
                let mut res = qpdf.extract(&fallback_req)?;
                res.fallback_reason = Some(reason.clone());
                res.warnings.splice(0..0, preflight_warnings);
                res.warnings.push(BackendWarning::FallbackTriggered {
                    from: "PdfCraft".to_string(),
                    to: "Qpdf".to_string(),
                    reason: reason.to_string(),
                });
                Ok(res)
            }
        }
    } else {
        let backend = QpdfBackend::new();
        backend.extract(req)
    }
}

#[cfg(test)]
mod tests;

