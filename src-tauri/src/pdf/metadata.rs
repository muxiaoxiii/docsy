use anyhow::{Context, Result};
use pdfcraft_cos::{Dict, Document, ObjRef, Object, SaveOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

const MAX_METADATA_COPY_DEPTH: usize = 128;

/// 深度递归复制 PDF 对象树，正确保持间接引用关系并避免循环引用。
pub fn deep_copy_object(
    src: &Document,
    dst: &mut Document,
    obj: &Object,
    seen: &mut HashMap<ObjRef, ObjRef>,
    depth: usize,
) -> Result<Object> {
    if depth > MAX_METADATA_COPY_DEPTH {
        return Ok(Object::Null);
    }
    match obj {
        Object::Ref(r) => {
            if let Some(mapped) = seen.get(r) {
                return Ok(Object::Ref(*mapped));
            }
            let new_ref = dst.add(Object::Null);
            seen.insert(*r, new_ref);
            let resolved = src.get(*r);
            let copied = deep_copy_object(src, dst, &resolved, seen, depth + 1)?;
            dst.set(new_ref, copied);
            Ok(Object::Ref(new_ref))
        }
        Object::Array(arr) => {
            let mut new_arr = Vec::with_capacity(arr.len());
            for item in arr {
                new_arr.push(deep_copy_object(src, dst, item, seen, depth + 1)?);
            }
            Ok(Object::Array(new_arr))
        }
        Object::Dict(dict) => {
            let mut new_dict = Dict::new();
            for (k, v) in dict.iter() {
                new_dict.set(k.clone(), deep_copy_object(src, dst, v, seen, depth + 1)?);
            }
            Ok(Object::Dict(new_dict))
        }
        Object::Stream(stream) => {
            let copied_dict = match deep_copy_object(
                src,
                dst,
                &Object::Dict(stream.dict.clone()),
                seen,
                depth + 1,
            )? {
                Object::Dict(d) => d,
                _ => Dict::new(),
            };
            let mut s = stream.clone();
            s.dict = copied_dict;
            Ok(Object::Stream(s))
        }
        other => Ok(other.clone()),
    }
}

/// 将源文档的 Trailer `/Info` 字典与 Catalog `/Metadata` (XMP) 流递归迁移至目标文档。
pub fn copy_info_and_xmp(
    src_doc: &Document,
    dst_doc: &mut Document,
    preserve_info: bool,
    preserve_xmp: bool,
) -> Result<bool> {
    let mut modified = false;
    let mut seen = HashMap::new();

    if preserve_info {
        if let Some(src_info_obj) = src_doc.trailer().get(b"Info") {
            let resolved = src_doc.resolve(src_info_obj);
            if let Object::Dict(_) = resolved.as_ref() {
                let copied_dict =
                    deep_copy_object(src_doc, dst_doc, resolved.as_ref(), &mut seen, 0)?;
                let info_ref = dst_doc.add(copied_dict);
                dst_doc
                    .trailer_mut()
                    .set(b"Info".to_vec(), Object::Ref(info_ref));
                modified = true;
            }
        }
    }

    if preserve_xmp {
        if let Some(src_root) = src_doc.trailer().reference(b"Root") {
            if let Some(src_cat) = src_doc.get(src_root).as_dict() {
                if let Some(meta_obj) = src_cat.get(b"Metadata") {
                    let resolved = src_doc.resolve(meta_obj);
                    if let Object::Stream(_) = resolved.as_ref() {
                        let copied_meta =
                            deep_copy_object(src_doc, dst_doc, resolved.as_ref(), &mut seen, 0)?;
                        let meta_ref = dst_doc.add(copied_meta);
                        if let Some(dst_root) = dst_doc.trailer().reference(b"Root") {
                            let _ = dst_doc.update_dict(dst_root, |d| {
                                d.set(b"Metadata".to_vec(), Object::Ref(meta_ref));
                            });
                            modified = true;
                        }
                    }
                }
            }
        }
    }

    Ok(modified)
}

/// 跨文件将首个输入 PDF 的 `/Info`（含 `DocsyAntiCopyBackup`）与 `/Metadata` 迁移到合并产物。
pub fn migrate_first_input_metadata(
    source_path: &Path,
    target_path: &Path,
    preserve_info: bool,
    preserve_xmp: bool,
) -> Result<()> {
    if !preserve_info && !preserve_xmp {
        return Ok(());
    }
    if !source_path.exists() || !target_path.exists() {
        return Ok(());
    }

    let src_bytes = std::fs::read(source_path)
        .with_context(|| format!("读取源 PDF 失败: {}", source_path.display()))?;
    let src_doc = Document::open(Arc::new(src_bytes))
        .with_context(|| format!("解析源 PDF 失败: {}", source_path.display()))?;

    let has_info = src_doc.trailer().get(b"Info").is_some();
    let root_obj = src_doc
        .trailer()
        .reference(b"Root")
        .map(|r| src_doc.get(r));
    let has_meta = root_obj
        .as_ref()
        .and_then(|o| o.as_dict())
        .and_then(|c| c.get(b"Metadata"))
        .is_some();

    if (!preserve_info || !has_info) && (!preserve_xmp || !has_meta) {
        return Ok(());
    }

    let dst_bytes = std::fs::read(target_path)
        .with_context(|| format!("读取目标 PDF 失败: {}", target_path.display()))?;
    let mut dst_doc = Document::open(Arc::new(dst_bytes))
        .with_context(|| format!("解析目标 PDF 失败: {}", target_path.display()))?;

    if copy_info_and_xmp(&src_doc, &mut dst_doc, preserve_info, preserve_xmp)? {
        let out_bytes = pdfcraft_cos::write_full(&dst_doc, &SaveOptions::default())
            .with_context(|| format!("序列化迁移元数据 PDF 失败: {}", target_path.display()))?;
        let temp_out = crate::util::fs::sibling_temp_path(target_path, "docsy-meta-migrate");
        let temp_guard = crate::util::fs::TempPathGuard::new(temp_out.clone());
        std::fs::write(&temp_out, out_bytes)
            .with_context(|| format!("写入临时迁移 PDF 失败: {}", temp_out.display()))?;
        crate::util::fs::replace_file(&temp_out, target_path)
            .with_context(|| format!("原子替换目标 PDF 失败: {}", target_path.display()))?;
        let _ = temp_guard;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfcraft_cos::PdfString;

    #[test]
    fn test_deep_copy_preserves_nested_indirect_references_and_cycles() {
        let mut src = Document::new_empty();
        let mut dst = Document::new_empty();

        let child_dict_ref = src.add(Object::Dict(Dict::new()));
        let mut parent_dict = Dict::new();
        parent_dict.set(b"Child".to_vec(), Object::Ref(child_dict_ref));
        parent_dict.set(
            b"CustomTitle".to_vec(),
            Object::String(PdfString::text("Test Nested")),
        );
        let parent_dict_ref = src.add(Object::Dict(parent_dict));

        let mut seen = HashMap::new();
        let copied = deep_copy_object(
            &src,
            &mut dst,
            &Object::Ref(parent_dict_ref),
            &mut seen,
            0,
        )
        .unwrap();

        let Object::Ref(copied_parent_ref) = copied else {
            panic!("expected Object::Ref");
        };

        let parent_obj = dst.get(copied_parent_ref);
        let parent_d = parent_obj.as_dict().unwrap();
        assert_eq!(
            parent_d
                .get(b"CustomTitle")
                .and_then(|v| v.as_string())
                .unwrap()
                .to_text(),
            "Test Nested"
        );
        let child_ref = parent_d.reference(b"Child").unwrap();
        assert!(!matches!(dst.get(child_ref).as_ref(), Object::Null));
        assert_ne!(child_ref, child_dict_ref);
    }
}
