use super::policy::{BackendWarning, MergePolicy, PreflightDocument, PreflightSummary};
use super::{BackendError, BackendType, ExtractRequest, ExtractResult, MergeRequest, MergeResult, PdfBackend};
use pdfcraft_cos::{CosError, Document, SaveOptions};
use pdfcraft_organize::Combiner;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 映射提取前后页面上 Font 对象的 ID，便于重映射 v1 备份数据。
fn map_fonts(
    src: &Document,
    dst: &Document,
    pages_0based: &[usize],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(src_pages) = pdfcraft_organize::walk(src) else { return map; };
    let Ok(dst_pages) = pdfcraft_organize::walk(dst) else { return map; };

    for (dst_idx, &src_idx) in pages_0based.iter().enumerate() {
        if src_idx >= src_pages.len() || dst_idx >= dst_pages.len() {
            continue;
        }
        let (_, src_attrs) = &src_pages[src_idx];
        let (_, dst_attrs) = &dst_pages[dst_idx];

        let get_fonts = |d: &Document, attrs: &pdfcraft_cos::Dict| -> HashMap<Vec<u8>, pdfcraft_cos::ObjRef> {
            let mut res_map = HashMap::new();
            if let Some(res_obj) = attrs.get(b"Resources").map(|o| d.resolve(o)) {
                if let Some(res_dict) = res_obj.as_dict() {
                    if let Some(font_obj) = res_dict.get(b"Font").map(|o| d.resolve(o)) {
                        if let Some(font_dict) = font_obj.as_dict() {
                            for (k, v) in font_dict.iter() {
                                if let pdfcraft_cos::Object::Ref(r) = v {
                                    res_map.insert(k.clone(), *r);
                                }
                            }
                        }
                    }
                }
            }
            res_map
        };

        let src_fonts = get_fonts(src, src_attrs);
        let dst_fonts = get_fonts(dst, dst_attrs);

        for (name, old_r) in src_fonts {
            if let Some(new_r) = dst_fonts.get(&name) {
                let old_key = format!("({}, {})", old_r.num, old_r.generation);
                let new_key = format!("({}, {})", new_r.num, new_r.generation);
                map.insert(old_key, new_key);
            }
        }
    }
    map
}

/// 基于纯 Rust `pdfcraft` (cos + organize) 的原生页面组织候选后端。
pub struct PdfCraftBackend;

impl PdfCraftBackend {
    pub fn new() -> Self {
        Self
    }

    /// 检查当前进程是否收到了取消信号。
    fn check_cancelled() -> Result<(), BackendError> {
        if crate::operations::check_current_cancelled().is_err() {
            Err(BackendError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// 读取并解析 PDF 文档（若存在空用户密码的权限加密，自动剥离加密字典）。
    fn open_pdf_document(path: &Path) -> Result<Document, BackendError> {
        let bytes = std::fs::read(path).map_err(BackendError::IoError)?;
        let mut doc = Document::open(Arc::new(bytes)).map_err(|err| match err {
            CosError::NeedsPassword | CosError::WrongPassword | CosError::Security(_) => {
                BackendError::Unsupported(format!("受保护或加密的 PDF 文件: {err}"))
            }
            other => BackendError::ParseError(other.to_string()),
        })?;
        if doc.security().is_some() {
            doc.remove_encryption();
        }
        Ok(doc)
    }

    /// 预检单个文件。
    fn inspect_file(path: &Path) -> Result<PreflightDocument, BackendError> {
        if crate::pdf::word_convert::is_word_file(path.to_str().unwrap_or("")) {
            return Ok(PreflightDocument {
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
        }

        let bytes = std::fs::read(path).map_err(BackendError::IoError)?;
        let doc = match Document::open(Arc::new(bytes)) {
            Ok(d) => d,
            Err(CosError::NeedsPassword | CosError::WrongPassword | CosError::Security(_)) => {
                return Ok(PreflightDocument {
                    path: path.to_path_buf(),
                    page_count: 0,
                    is_encrypted: true,
                    has_signatures: false,
                    has_bookmarks: false,
                    has_attachments: false,
                    has_form_fields: false,
                    has_anticopy_backup: false,
                    anticopy_version: None,
                });
            }
            Err(e) => return Err(BackendError::ParseError(e.to_string())),
        };

        let page_count = pdfcraft_organize::page_count(&doc).unwrap_or(0) as u32;
        // Document::open 已能原生解开空用户密码的权限锁（并在 open_pdf_document 中剥离 /Encrypt）；
        // 仅当上方 Document::open 返回 NeedsPassword / WrongPassword / Security 错误时才视为需回退的加密文档。
        let is_encrypted = false;

        let mut has_signatures = false;
        let mut has_bookmarks = false;
        let mut has_attachments = false;
        let mut has_form_fields = false;

        if let Some(root_ref) = doc.root() {
            let root_obj = doc.get(root_ref);
            if let Some(catalog) = root_obj.as_dict() {
                if catalog.contains(b"Outlines") {
                    has_bookmarks = true;
                }
                if let Some(names) = catalog.get(b"Names").and_then(|n| doc.dict(n)) {
                    if names.contains(b"EmbeddedFiles") {
                        has_attachments = true;
                    }
                }
                if let Some(acro) = catalog.get(b"AcroForm").and_then(|a| doc.dict(a)) {
                    if let Some(fields) = acro.get(b"Fields").and_then(|f| doc.resolve(f).as_array().cloned()) {
                        if !fields.is_empty() {
                            has_form_fields = true;
                        }
                    }
                    if let Some(sig_flags) = acro.get(b"SigFlags").and_then(|s| doc.resolve(s).as_int()) {
                        if sig_flags > 0 {
                            has_signatures = true;
                        }
                    }
                }
            }
        }

        if !has_signatures && crate::pdf::signatures::count_signature_objects_pdfcraft(&doc) > 0 {
            has_signatures = true;
        }

        let mut has_anticopy_backup = false;
        let mut anticopy_version = None;
        if let Some(text) = pdfcraft_organize::info(&doc, "DocsyAntiCopyBackup") {
            has_anticopy_backup = true;
            let stripped = text.strip_prefix("u:").unwrap_or(&text);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(stripped) {
                anticopy_version = json.get("version").and_then(|v| v.as_u64());
            }
        }

        Ok(PreflightDocument {
            path: path.to_path_buf(),
            page_count,
            is_encrypted,
            has_signatures,
            has_bookmarks,
            has_attachments,
            has_form_fields,
            has_anticopy_backup,
            anticopy_version,
        })
    }
}

impl Default for PdfCraftBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfBackend for PdfCraftBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::PdfCraft
    }

    fn preflight_merge(&self, req: &MergeRequest) -> Result<PreflightSummary, BackendError> {
        let mut summary = PreflightSummary::new();
        for input in &req.inputs {
            let path = Path::new(input);
            if !path.exists() {
                return Err(BackendError::Other(format!("输入文件不存在: {input}")));
            }
            let doc_info = Self::inspect_file(path)?;
            summary.add_document(doc_info);
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
        let doc_info = Self::inspect_file(path)?;
        if doc_info.has_bookmarks {
            summary.warnings.push(BackendWarning::StructureOmittedOnExtract {
                path: path.display().to_string(),
                item: "Bookmarks".to_string(),
            });
        }
        if doc_info.has_attachments {
            summary.warnings.push(BackendWarning::StructureOmittedOnExtract {
                path: path.display().to_string(),
                item: "Attachments".to_string(),
            });
        }
        summary.add_document(doc_info);
        Ok(summary)
    }

    fn merge(&self, req: &MergeRequest) -> Result<MergeResult, BackendError> {
        Self::check_cancelled()?;

        if req.inputs.is_empty() {
            return Err(BackendError::Other("没有可合并的文件".to_string()));
        }

        let state = req
            .conversion_state
            .clone()
            .unwrap_or_else(|| Arc::new(crate::ConversionState::new()));

        // 步骤 1：处理 Word 格式转换，临时 PDF 严格由 TempPathGuard 守卫生命周期
        let mut temp_converted_guards: Vec<crate::util::fs::TempPathGuard> = Vec::new();
        let mut resolved_inputs: Vec<PathBuf> = Vec::with_capacity(req.inputs.len());

        for input in &req.inputs {
            Self::check_cancelled()?;
            let path = Path::new(input);
            if crate::pdf::word_convert::is_word_file(path.to_str().unwrap_or("")) {
                let temp_pdf = crate::util::fs::temp_named_path("docsy_pdfcraft_word", "pdf");
                let converted_str = crate::pdf::word_convert::convert_word_to_pdf_file(
                    input,
                    &temp_pdf,
                    &state,
                )
                .map_err(|e| BackendError::ExecutionError {
                    backend: BackendType::PdfCraft,
                    message: format!("Word 转 PDF 失败: {e}"),
                })?;
                temp_converted_guards.push(crate::util::fs::TempPathGuard::new(temp_pdf));
                resolved_inputs.push(PathBuf::from(converted_str));
            } else {
                resolved_inputs.push(path.to_path_buf());
            }
        }

        // 步骤 2：执行预检与约束检查
        let mut warnings = Vec::new();
        for path in &resolved_inputs {
            let doc_info = Self::inspect_file(path)?;
            if doc_info.is_encrypted && req.policy.refuse_on_encrypted {
                return Err(BackendError::Unsupported(format!(
                    "文件已加密且设置了拒绝策略: {}",
                    path.display()
                )));
            }
            if doc_info.has_anticopy_backup && doc_info.anticopy_version.unwrap_or(0) < 2 && req.policy.refuse_on_anticopy_backup {
                return Err(BackendError::Unsupported(format!(
                    "文件包含旧版 Docsy 防复制恢复备份 (v{})，候选后端重编号可能破坏恢复链: {}",
                    doc_info.anticopy_version.unwrap_or(0),
                    path.display()
                )));
            }
            if doc_info.has_signatures {
                warnings.push(BackendWarning::SignatureValidityInvalidated {
                    path: path.display().to_string(),
                });
            }
        }

        // 步骤 3：使用 Combiner 逐个加入输入
        let mut combiner = Combiner::new();
        let mut blank_guards: Vec<crate::util::fs::TempPathGuard> = Vec::new();

        for input_path in &resolved_inputs {
            Self::check_cancelled()?;
            let doc = Self::open_pdf_document(input_path)?;
            if req.policy.merge_policy == MergePolicy::Print
                && super::flatten::has_stale_form_appearances(&doc)?
            {
                return Err(BackendError::Unsupported(format!(
                    "文档包含 NeedAppearances=true 的未更新表单外观，需回退至 Qpdf 处理: {}",
                    input_path.display()
                )));
            }
            let page_count = pdfcraft_organize::page_count(&doc)
                .map_err(|e| BackendError::ParseError(e.to_string()))?;

            let title = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Document");

            combiner
                .add(title, &doc, None)
                .map_err(|e| BackendError::ExecutionError {
                    backend: BackendType::PdfCraft,
                    message: format!("添加文档失败: {e}"),
                })?;

            // 双面打印分隔模式：奇数页末尾补一页同尺寸空白页
            if req.policy.duplex_separate && page_count % 2 == 1 {
                let page_infos = crate::pdf::page_info::get_page_infos(&input_path.to_string_lossy())
                    .map_err(|e| BackendError::ExecutionError {
                        backend: BackendType::PdfCraft,
                        message: format!("读取页面尺寸失败: {e}"),
                    })?;
                if let Some(last_size) = page_infos.last() {
                    let blank_guard = crate::pdf::blank::write_blank_page(last_size)
                        .map_err(|e| BackendError::ExecutionError {
                            backend: BackendType::PdfCraft,
                            message: format!("生成空白页失败: {e}"),
                        })?;
                    let blank_doc = Self::open_pdf_document(blank_guard.path())?;
                    combiner
                        .add("", &blank_doc, None)
                        .map_err(|e| BackendError::ExecutionError {
                            backend: BackendType::PdfCraft,
                            message: format!("追加空白页失败: {e}"),
                        })?;
                    blank_guards.push(blank_guard);
                }
            }
        }

        Self::check_cancelled()?;

        // 步骤 4：完成合并与字段重命名收集
        let outcome = combiner
            .finish_with_renames()
            .map_err(|e| BackendError::ExecutionError {
                backend: BackendType::PdfCraft,
                message: format!("合并完成处理失败: {e}"),
            })?;

        if req.policy.merge_policy != MergePolicy::Print {
            for (old_name, new_name) in &outcome.renamed_fields {
                warnings.push(BackendWarning::FormFieldRenamed {
                    original: old_name.clone(),
                    renamed: new_name.clone(),
                });
            }
        }

        let total_pages = pdfcraft_organize::page_count(&outcome.doc).unwrap_or(0) as u32;

        let mut final_doc = outcome.doc;

        // Print 模式：执行原生注释、签章与表单 Widget 外观压平
        if req.policy.merge_policy == MergePolicy::Print {
            super::flatten::flatten_document(&mut final_doc)?;
            if !req.policy.preserve_bookmarks {
                if let Some(root_ref) = final_doc.root() {
                    let _ = final_doc.update_dict(root_ref, |c| {
                        c.remove(b"Outlines");
                    });
                }
            }
        }

        // 落实附件保留策略：Print 模式或明确不保留附件时，彻底移除内嵌附件
        if !req.policy.preserve_attachments {
            if let Some(root_ref) = final_doc.root() {
                let names_ref = final_doc
                    .get(root_ref)
                    .as_dict()
                    .and_then(|c| c.reference(b"Names"));
                if let Some(nr) = names_ref {
                    let mut is_empty = false;
                    let _ = final_doc.update_dict(nr, |n| {
                        n.remove(b"EmbeddedFiles");
                        is_empty = n.is_empty();
                    });
                    if is_empty {
                        let _ = final_doc.update_dict(root_ref, |c| {
                            c.remove(b"Names");
                        });
                    }
                } else {
                    let _ = final_doc.update_dict(root_ref, |c| {
                        if let Some(pdfcraft_cos::Object::Dict(mut d)) = c.get(b"Names").cloned() {
                            d.remove(b"EmbeddedFiles");
                            if d.is_empty() {
                                c.remove(b"Names");
                            } else {
                                c.set(b"Names".to_vec(), pdfcraft_cos::Object::Dict(d));
                            }
                        }
                    });
                }
            }
        }

        // 落实结构保留策略：递归保留首个输入文档的 Info 字典（含嵌套间接引用与防复制备份）与 Catalog /Metadata (XMP)
        if (req.policy.preserve_info || req.policy.preserve_xmp) && !resolved_inputs.is_empty() {
            if let Ok(first_bytes) = std::fs::read(&resolved_inputs[0]) {
                if let Ok(first_doc) = pdfcraft_cos::Document::open(std::sync::Arc::new(first_bytes)) {
                    let _ = crate::pdf::metadata::copy_info_and_xmp(
                        &first_doc,
                        &mut final_doc,
                        req.policy.preserve_info,
                        req.policy.preserve_xmp,
                    );
                }
            }
        }

        Self::check_cancelled()?;

        // 步骤 5：无损重写输出字节流
        let bytes = pdfcraft_cos::write_full(&final_doc, &SaveOptions::default())
            .map_err(|e| BackendError::ExecutionError {
                backend: BackendType::PdfCraft,
                message: format!("序列化 PDF 失败: {e}"),
            })?;

        // 步骤 6：排他发布并返回实际分配的新路径，防止覆盖已有文件、输入源及并发产物
        let parent = req
            .output_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent).map_err(BackendError::IoError)?;

        let sibling_temp = crate::util::fs::sibling_temp_path(&req.output_path, "pdfcraft-merge");
        let sibling_guard = crate::util::fs::TempPathGuard::new(sibling_temp.clone());

        std::fs::write(&sibling_temp, &bytes).map_err(BackendError::IoError)?;

        Self::check_cancelled()?;

        let published_path = crate::util::fs::copy_unique_with_stem(
            sibling_guard.path(),
            parent,
            req.output_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output"),
            req.output_path.extension().and_then(|s| s.to_str()).unwrap_or("pdf"),
        ).map_err(|e| BackendError::ExecutionError {
            backend: BackendType::PdfCraft,
            message: format!("安全发布 PDF 失败: {e}"),
        })?;

        Ok(MergeResult {
            output_path: published_path,
            backend: BackendType::PdfCraft,
            total_pages,
            warnings,
            renamed_fields: outcome.renamed_fields,
            fallback_reason: None,
        })
    }

    fn extract(&self, req: &ExtractRequest) -> Result<ExtractResult, BackendError> {
        Self::check_cancelled()?;

        if req.pages.is_empty() {
            return Err(BackendError::Other("缺少要提取的页码".to_string()));
        }
        if req.pages.contains(&0) {
            return Err(BackendError::Other("页码必须从 1 开始".to_string()));
        }

        let doc = Self::open_pdf_document(&req.input_path)?;
        let total_pages = pdfcraft_organize::page_count(&doc)
            .map_err(|e| BackendError::ParseError(e.to_string()))? as u32;

        if let Some(page) = req.pages.iter().find(|p| **p > total_pages) {
            return Err(BackendError::Other(format!(
                "页码 {page} 超过 PDF 总页数 {total_pages}"
            )));
        }

        let mut warnings = Vec::new();
        if let Some(root_ref) = doc.root() {
            let root_obj = doc.get(root_ref);
            if let Some(catalog) = root_obj.as_dict() {
                if catalog.contains(b"Outlines") {
                    warnings.push(BackendWarning::StructureOmittedOnExtract {
                        path: req.input_path.display().to_string(),
                        item: "Bookmarks".to_string(),
                    });
                }
                if let Some(names) = catalog.get(b"Names").and_then(|n| doc.dict(n)) {
                    if names.contains(b"EmbeddedFiles") {
                        warnings.push(BackendWarning::StructureOmittedOnExtract {
                            path: req.input_path.display().to_string(),
                            item: "Attachments".to_string(),
                        });
                    }
                }
            }
        }

        // 统一在边界处完成 1-based 到 0-based 映射
        let zero_based: Vec<usize> = req.pages.iter().map(|p| (*p - 1) as usize).collect();

        Self::check_cancelled()?;

        let mut extracted_doc = pdfcraft_organize::extract_pages(&doc, &zero_based)
            .map_err(|e| BackendError::ExecutionError {
                backend: BackendType::PdfCraft,
                message: format!("提取页面失败: {e}"),
            })?;

        // 迁移源文档 Info 字典与 DocsyAntiCopyBackup（含 v1 字体编号重映射）
        if let Some(src_info_obj) = doc.trailer().get(b"Info") {
            let resolved_info = doc.resolve(src_info_obj);
            if let Some(src_dict) = resolved_info.as_dict() {
                let mut new_info = pdfcraft_cos::Dict::new();
                let font_map = map_fonts(&doc, &extracted_doc, &zero_based);

                for (k, v) in src_dict.iter() {
                    let val_resolved = doc.resolve(v);
                    if k == b"DocsyAntiCopyBackup" {
                        if let Some(s) = val_resolved.as_string() {
                            let text = s.to_text();
                            let json_str = text.strip_prefix("u:").unwrap_or(&text);
                            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                let version = val.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
                                if version == 1 {
                                    if let Some(cmaps) = val.get_mut("cmaps").and_then(|c| c.as_object_mut()) {
                                        let mut remapped = serde_json::Map::new();
                                        for (font_key, cmap_val) in cmaps.iter() {
                                            if let Some(new_key) = font_map.get(font_key) {
                                                remapped.insert(new_key.clone(), cmap_val.clone());
                                            } else {
                                                remapped.insert(font_key.clone(), cmap_val.clone());
                                            }
                                        }
                                        *cmaps = remapped;
                                    }
                                }
                                let new_json = serde_json::to_string(&val).unwrap_or_else(|_| json_str.to_string());
                                let final_str = if text.starts_with("u:") {
                                    format!("u:{new_json}")
                                } else {
                                    new_json
                                };
                                new_info.set(k.clone(), pdfcraft_cos::Object::String(pdfcraft_cos::PdfString::text(&final_str)));
                                continue;
                            }
                        }
                    }
                    new_info.set(k.clone(), val_resolved.as_ref().clone());
                }
                let info_ref = extracted_doc.add(new_info);
                extracted_doc.trailer_mut().set(b"Info".to_vec(), pdfcraft_cos::Object::Ref(info_ref));
            }
        }

        Self::check_cancelled()?;

        let bytes = pdfcraft_cos::write_full(&extracted_doc, &SaveOptions::default())
            .map_err(|e| BackendError::ExecutionError {
                backend: BackendType::PdfCraft,
                message: format!("序列化提取 PDF 失败: {e}"),
            })?;

        let output_path = if let Some(ref path) = req.output_path {
            path.clone()
        } else {
            let parent = req
                .output_dir
                .as_ref()
                .map(|p| p.as_path())
                .unwrap_or_else(|| req.input_path.parent().unwrap_or(Path::new(".")));
            let stem = req
                .input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let ext = req
                .input_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("pdf");
            crate::util::fs::unique_output_path(parent, &format!("{stem}_pages"), ext)
        };

        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent).map_err(BackendError::IoError)?;

        let sibling_temp = crate::util::fs::sibling_temp_path(&output_path, "pdfcraft-extract");
        let sibling_guard = crate::util::fs::TempPathGuard::new(sibling_temp.clone());

        std::fs::write(&sibling_temp, &bytes).map_err(BackendError::IoError)?;

        Self::check_cancelled()?;

        let published_path = crate::util::fs::copy_unique_with_stem(
            sibling_guard.path(),
            parent,
            output_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output"),
            output_path.extension().and_then(|s| s.to_str()).unwrap_or("pdf"),
        ).map_err(|e| BackendError::ExecutionError {
            backend: BackendType::PdfCraft,
            message: format!("安全发布提取 PDF 失败: {e}"),
        })?;

        let input_size = std::fs::metadata(&req.input_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let output_size = bytes.len() as u64;

        Ok(ExtractResult {
            output_path: published_path,
            backend: BackendType::PdfCraft,
            total_pages: req.pages.len() as u32,
            input_size,
            output_size,
            warnings,
            fallback_reason: None,
        })
    }
}
