use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// PDF 合并语义策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergePolicy {
    /// 打印合并语义：注释与签章外观压平、页面几何标准化、支持双面补白、输出适合打印与归档。
    Print,
    /// 结构保留合并语义：尽量保留交互表单、书签层次、附件与元数据；适用于电子卷宗与可交互文档组合。
    Structural,
}

impl Default for MergePolicy {
    fn default() -> Self {
        Self::Print
    }
}

/// 细粒度输出与结构保留政策配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicy {
    /// 合并语义模式。
    pub merge_policy: MergePolicy,
    /// 是否保留 Info 元数据。
    pub preserve_info: bool,
    /// 是否保留 XMP 元数据流。
    pub preserve_xmp: bool,
    /// 是否保留文档书签（大纲）。
    pub preserve_bookmarks: bool,
    /// 是否保留嵌入式附件。
    pub preserve_attachments: bool,
    /// 是否保留交互表单字段。
    pub preserve_form_fields: bool,
    /// 遇到防复制恢复备份时是否拒绝候选引擎重写（避免对象重编号破坏恢复链）。
    pub refuse_on_anticopy_backup: bool,
    /// 遇到加密文档时是否直接拒绝候选引擎重写。
    pub refuse_on_encrypted: bool,
    /// 双面打印时是否在奇数页末尾插入空白页。
    pub duplex_separate: bool,
}

impl Default for OutputPolicy {
    fn default() -> Self {
        Self {
            merge_policy: MergePolicy::Print,
            preserve_info: true,
            preserve_xmp: true,
            preserve_bookmarks: true,
            preserve_attachments: true,
            preserve_form_fields: true,
            refuse_on_anticopy_backup: true,
            refuse_on_encrypted: true,
            duplex_separate: false,
        }
    }
}

impl OutputPolicy {
    pub fn for_policy(policy: MergePolicy, duplex_separate: bool) -> Self {
        match policy {
            MergePolicy::Print => Self {
                merge_policy: MergePolicy::Print,
                preserve_info: true,
                preserve_xmp: true,
                preserve_bookmarks: false,
                preserve_attachments: false,
                preserve_form_fields: false,
                refuse_on_anticopy_backup: true,
                refuse_on_encrypted: true,
                duplex_separate,
            },
            MergePolicy::Structural => Self {
                merge_policy: MergePolicy::Structural,
                preserve_info: true,
                preserve_xmp: true,
                preserve_bookmarks: true,
                preserve_attachments: true,
                preserve_form_fields: true,
                refuse_on_anticopy_backup: true,
                refuse_on_encrypted: true,
                duplex_separate,
            },
        }
    }
}

/// 预检期间或执行期间产生的警告项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "detail")]
pub enum BackendWarning {
    /// 签名证书有效性无法在重组后维持；签章视觉外观已保留。
    SignatureValidityInvalidated { path: String },
    /// 自定义或不受支持的 Info 属性未被保留。
    CustomInfoDropped { path: String, key: String },
    /// 检测到防复制备份；为保护恢复链，候选引擎已拒绝或已回退。
    AntiCopyBackupDetected { path: String, version: Option<u64> },
    /// 同名表单字段冲突，已自动重命名。
    FormFieldRenamed { original: String, renamed: String },
    /// 提取页面时，原文档的书签或附件未迁移至单页产物。
    StructureOmittedOnExtract { path: String, item: String },
    /// 触发了后端回退。
    FallbackTriggered { from: String, to: String, reason: String },
}

/// 回退原因枚举。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackReason {
    UnsupportedEncryption,
    DocsyAntiCopyBackupDetected,
    PrintFlatteningNotSupported,
    UnsupportedFeature(String),
    PreflightRefused(String),
    CandidateExecutionFailed(String),
}

impl std::fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedEncryption => write!(f, "文档包含候选后端不支持的加密或密码保护"),
            Self::DocsyAntiCopyBackupDetected => write!(f, "文档包含 Docsy 防复制恢复备份，候选后端重编号会破坏恢复链"),
            Self::PrintFlatteningNotSupported => write!(f, "候选后端尚未实现打印外观压平，回退到 Qpdf 生产引擎"),
            Self::UnsupportedFeature(msg) => write!(f, "不支持的 PDF 特性: {msg}"),
            Self::PreflightRefused(msg) => write!(f, "预检拒绝: {msg}"),
            Self::CandidateExecutionFailed(msg) => write!(f, "候选后端执行失败: {msg}"),
        }
    }
}

/// 单个输入文件的预检信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightDocument {
    pub path: PathBuf,
    pub page_count: u32,
    pub is_encrypted: bool,
    pub has_signatures: bool,
    pub has_bookmarks: bool,
    pub has_attachments: bool,
    pub has_form_fields: bool,
    pub has_anticopy_backup: bool,
    pub anticopy_version: Option<u64>,
}

/// 整体预检结果摘要。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightSummary {
    pub documents: Vec<PreflightDocument>,
    pub total_input_pages: u32,
    pub any_encrypted: bool,
    pub any_signatures: bool,
    pub any_anticopy_backup: bool,
    pub warnings: Vec<BackendWarning>,
}

impl PreflightSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_document(&mut self, doc: PreflightDocument) {
        self.total_input_pages += doc.page_count;
        if doc.is_encrypted {
            self.any_encrypted = true;
        }
        if doc.has_signatures {
            self.any_signatures = true;
            self.warnings.push(BackendWarning::SignatureValidityInvalidated {
                path: doc.path.display().to_string(),
            });
        }
        if doc.has_anticopy_backup {
            self.any_anticopy_backup = true;
            self.warnings.push(BackendWarning::AntiCopyBackupDetected {
                path: doc.path.display().to_string(),
                version: doc.anticopy_version,
            });
        }
        self.documents.push(doc);
    }

    /// 评估是否应由候选后端回退至生产默认后端。
    pub fn should_fallback_for_candidate(&self, policy: &OutputPolicy) -> Option<FallbackReason> {
        if self.any_encrypted && policy.refuse_on_encrypted {
            return Some(FallbackReason::UnsupportedEncryption);
        }
        if self.any_anticopy_backup && policy.merge_policy == MergePolicy::Print {
            return Some(FallbackReason::DocsyAntiCopyBackupDetected);
        }
        None
    }
}
