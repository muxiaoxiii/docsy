#![allow(dead_code)]

use super::BackendError;
use pdfcraft_cos::{Dict, Document, ObjRef, Object, PdfString, Stream};
use serde::{Deserialize, Serialize};

/// Docsy 页面标记类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsyMarkKind {
    Header,
    Footer,
    Watermark,
    Bates,
}

impl DocsyMarkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "Header",
            Self::Footer => "Footer",
            Self::Watermark => "Watermark",
            Self::Bates => "Bates",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Header" => Some(Self::Header),
            "Footer" => Some(Self::Footer),
            "Watermark" => Some(Self::Watermark),
            "Bates" => Some(Self::Bates),
            _ => None,
        }
    }
}

/// 单个标记的规格描述。
#[derive(Debug, Clone)]
pub struct DocsyMarkSpec {
    pub id: String,
    pub kind: DocsyMarkKind,
    pub batch_id: Option<String>,
    pub page_index: usize,
    /// 在显示空间（以页面左下角为原点、向上为正、未经旋转的视觉坐标）内的绘图算子字节。
    pub content_ops: Vec<u8>,
}

/// 标记过滤条件。
#[derive(Debug, Clone, Default)]
pub struct DocsyMarkFilter {
    pub kind: Option<DocsyMarkKind>,
    pub id: Option<String>,
    pub batch_id: Option<String>,
}

/// 标记概要信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsyMarkSummary {
    pub id: String,
    pub kind: DocsyMarkKind,
    pub batch_id: Option<String>,
    pub page_index: usize,
    pub stream_obj_num: u32,
}

/// 计算从显示空间（视觉原点在左下角，视觉宽度 w，视觉高度 h）转换到 PDF 用户空间的 2D 仿射矩阵。
///
/// 严格处理：
/// - 四方向旋转（0°, 90°, 180°, 270°）
/// - 非零原点 CropBox / MediaBox
/// - UserUnit 缩放因子
pub fn view_matrix_for(rotation: i64, crop: [f64; 4], user_unit: f64) -> [f64; 6] {
    let [x0, y0, x1, y1] = crop;
    let norm_rot = ((rotation % 360) + 360) % 360;
    let m = match norm_rot {
        90 => [0.0, 1.0, -1.0, 0.0, x1, y0],
        180 => [-1.0, 0.0, 0.0, -1.0, x1, y1],
        270 => [0.0, -1.0, 1.0, 0.0, x0, y1],
        _ => [1.0, 0.0, 0.0, 1.0, x0, y0],
    };
    if (user_unit - 1.0).abs() < 1e-6 {
        m
    } else {
        [
            m[0] / user_unit,
            m[1] / user_unit,
            m[2] / user_unit,
            m[3] / user_unit,
            m[4],
            m[5],
        ]
    }
}

/// 沿 Parent 链向上解析可继承属性（PDF 32000-1 §12.5.5）。
pub fn get_inherited_object(doc: &Document, start_dict: &Dict, key: &[u8]) -> Option<Object> {
    if let Some(obj) = start_dict.get(key) {
        return Some(doc.resolve(obj).as_ref().clone());
    }
    let mut cur_parent = start_dict.reference(b"Parent");
    let mut visited = std::collections::HashSet::new();
    while let Some(parent_ref) = cur_parent {
        if !visited.insert(parent_ref) || visited.len() > 100 {
            break;
        }
        let parent_obj = doc.get(parent_ref);
        if let Some(dict) = parent_obj.as_dict() {
            if let Some(obj) = dict.get(key) {
                return Some(doc.resolve(obj).as_ref().clone());
            }
            cur_parent = dict.reference(b"Parent");
        } else {
            break;
        }
    }
    None
}

/// 提取页面的有效几何属性：旋转角度、有效边界框（优先 CropBox，无则 MediaBox）及 UserUnit。
/// 支持沿 Parent 树链继承。
pub fn page_geometry(doc: &Document, page_dict: &Dict) -> (i64, [f64; 4], f64) {
    let rotation = get_inherited_object(doc, page_dict, b"Rotate")
        .and_then(|r| r.as_int())
        .unwrap_or(0);

    let box_obj = get_inherited_object(doc, page_dict, b"CropBox")
        .or_else(|| get_inherited_object(doc, page_dict, b"MediaBox"));

    let crop = box_obj
        .and_then(|b| b.as_array().cloned())
        .and_then(|arr| {
            if arr.len() == 4 {
                let x0 = doc.resolve(&arr[0]).as_f64()?;
                let y0 = doc.resolve(&arr[1]).as_f64()?;
                let x1 = doc.resolve(&arr[2]).as_f64()?;
                let y1 = doc.resolve(&arr[3]).as_f64()?;
                Some([x0, y0, x1, y1])
            } else {
                None
            }
        })
        .unwrap_or([0.0, 0.0, 595.0, 842.0]);

    let user_unit = get_inherited_object(doc, page_dict, b"UserUnit")
        .and_then(|u| u.as_f64())
        .unwrap_or(1.0);

    (rotation, crop, user_unit)
}

/// 收集所有叶子页面，带 visited 集合防止页面树环形死循环。
pub fn collect_leaf_pages(doc: &Document) -> Result<Vec<ObjRef>, BackendError> {
    let root_ref = doc
        .root()
        .ok_or_else(|| BackendError::ParseError("缺少 Catalog".to_string()))?;
    let catalog = doc.get(root_ref);
    let pages_root = catalog
        .as_dict()
        .and_then(|d| d.reference(b"Pages"))
        .ok_or_else(|| BackendError::ParseError("缺少 Pages 树".to_string()))?;

    let mut pages_stack = vec![pages_root];
    let mut visited = std::collections::HashSet::new();
    let mut leaf_pages = Vec::new();

    while let Some(node_ref) = pages_stack.pop() {
        if !visited.insert(node_ref) {
            continue;
        }
        let node_obj = doc.get(node_ref);
        let Some(node_dict) = node_obj.as_dict() else {
            continue;
        };
        let is_page = node_dict.name(b"Type") == Some(b"Page");
        if is_page {
            leaf_pages.push(node_ref);
        } else {
            let kids = node_dict
                .get(b"Kids")
                .and_then(|k| doc.resolve(k).as_array().cloned())
                .unwrap_or_default();
            for kid in kids.iter().rev() {
                if let Some(r) = kid.as_ref() {
                    pages_stack.push(r);
                }
            }
        }
    }

    Ok(leaf_pages)
}

/// 获取页面所有内容流引用的列表。若原本为单流则保留或转化为引用列表。
fn page_contents_list(doc: &mut Document, page_ref: ObjRef) -> Result<Vec<ObjRef>, BackendError> {
    let page_obj = doc.get(page_ref);
    let page_dict = page_obj
        .as_dict()
        .ok_or_else(|| BackendError::ParseError("页面对象不是 Dictionary".to_string()))?;

    let contents_val = page_dict.get(b"Contents").cloned();
    match contents_val {
        None => Ok(Vec::new()),
        Some(Object::Ref(r)) => {
            let target = doc.get(r);
            if let Some(arr) = target.as_array() {
                let mut refs = Vec::new();
                for item in arr {
                    if let Some(item_ref) = item.as_ref() {
                        refs.push(item_ref);
                    }
                }
                Ok(refs)
            } else {
                Ok(vec![r])
            }
        }
        Some(Object::Array(arr)) => {
            let mut refs = Vec::new();
            for item in arr {
                if let Some(item_ref) = item.as_ref() {
                    refs.push(item_ref);
                }
            }
            Ok(refs)
        }
        Some(other) => {
            let new_ref = doc.add(other);
            Ok(vec![new_ref])
        }
    }
}

/// 判断某内容流是否为 Docsy 标记流（严格判断非零真值，避免将普通正文 /Docsy false 误删）。
pub fn is_docsy_stream(doc: &Document, stream_ref: ObjRef) -> bool {
    let obj = doc.get(stream_ref);
    let Some(stream) = obj.as_dict() else {
        return false;
    };
    let resolved = stream.get(b"Docsy").map(|v| doc.resolve(v));
    match resolved.as_deref() {
        Some(Object::Bool(b)) => *b,
        Some(Object::Int(n)) => *n != 0,
        Some(_) => false,
        None => stream.contains(b"DocsyId") && stream.contains(b"DocsyVersion"),
    }
}

/// 判断是否为 Docsy 状态包装流（严格校验所有权真值、角色名称与流字节，防止误删带 /DocsyWrapper false 的普通正文流）。
pub fn is_wrapper_stream(doc: &Document, stream_ref: ObjRef) -> bool {
    if !is_docsy_stream(doc, stream_ref) {
        return false;
    }
    let obj = doc.get(stream_ref);
    let Object::Stream(stream) = &*obj else {
        return false;
    };
    let Ok(bytes) = stream.decoded_within(64) else {
        return false;
    };
    match stream.dict.name(b"DocsyWrapper") {
        Some(b"Push") => bytes == b"q\n",
        Some(b"Pop") => bytes == b"\nQ\n",
        _ => false,
    }
}

/// PDF 字符串字面量转义（转义 `\`, `(`, `)`, `\r`, `\n`）。
pub fn escape_pdf_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\r' => out.push_str("\\r"),
            b'\n' => out.push_str("\\n"),
            _ => out.push(b as char),
        }
    }
    out
}

/// 判断页面或其父级资源中是否存在指定字体资源。
fn page_has_font_resource(doc: &Document, page_dict: &Dict, font_name: &[u8]) -> bool {
    let Some(res_obj) = get_inherited_object(doc, page_dict, b"Resources") else {
        return false;
    };
    let Some(res_dict) = res_obj.as_dict() else {
        return false;
    };
    let Some(font_obj) = res_dict.get(b"Font") else {
        return false;
    };
    let resolved_font = doc.resolve(font_obj);
    let Some(font_dict) = resolved_font.as_dict() else {
        return false;
    };
    font_dict.get(font_name).is_some()
}

/// 判断某内容流是否为 Docsy 标记流，并返回标记元数据。
fn inspect_stream_mark(
    doc: &Document,
    stream_ref: ObjRef,
    page_index: usize,
) -> Option<DocsyMarkSummary> {
    let obj = doc.get(stream_ref);
    let stream = obj.as_dict()?;
    if stream.contains(b"DocsyWrapper") {
        return None;
    }
    if !is_docsy_stream(doc, stream_ref) {
        return None;
    }

    let id = stream
        .get(b"DocsyId")
        .and_then(|v| doc.resolve(v).as_string().map(|s| s.to_text()))
        .unwrap_or_else(|| format!("docsy-stream-{}", stream_ref.num));

    let kind_str = stream
        .get(b"DocsyKind")
        .and_then(|v| doc.resolve(v).as_name().map(|n| n.to_vec()))
        .and_then(|n| String::from_utf8(n).ok());

    let kind = kind_str
        .as_deref()
        .and_then(DocsyMarkKind::from_str)
        .unwrap_or(DocsyMarkKind::Header);

    let batch_id = stream
        .get(b"DocsyBatch")
        .and_then(|v| doc.resolve(v).as_string().map(|s| s.to_text()));

    Some(DocsyMarkSummary {
        id,
        kind,
        batch_id,
        page_index,
        stream_obj_num: stream_ref.num,
    })
}

/// 检查给定标记是否匹配过滤规则。
fn matches_filter(summary: &DocsyMarkSummary, filter: &DocsyMarkFilter) -> bool {
    if let Some(kind) = filter.kind {
        if summary.kind != kind {
            return false;
        }
    }
    if let Some(ref id) = filter.id {
        if &summary.id != id {
            return false;
        }
    }
    if let Some(ref batch_id) = filter.batch_id {
        if summary.batch_id.as_ref() != Some(batch_id) {
            return false;
        }
    }
    true
}

/// 扫描文档中所有由 Docsy 写入的独立标记流。
pub fn inspect_marks(doc: &Document) -> Result<Vec<DocsyMarkSummary>, BackendError> {
    let leaf_pages = collect_leaf_pages(doc)?;
    let mut results = Vec::new();

    for (page_idx, page_ref) in leaf_pages.into_iter().enumerate() {
        let page_obj = doc.get(page_ref);
        let Some(page_dict) = page_obj.as_dict() else {
            continue;
        };
        if let Some(contents_val) = page_dict.get(b"Contents") {
            let resolved = doc.resolve(contents_val);
            if let Some(arr) = resolved.as_array() {
                for item in arr {
                    if let Some(r) = item.as_ref() {
                        if let Some(summary) = inspect_stream_mark(doc, r, page_idx) {
                            results.push(summary);
                        }
                    }
                }
            } else if let Some(r) = contents_val.as_ref() {
                if let Some(summary) = inspect_stream_mark(doc, r, page_idx) {
                    results.push(summary);
                }
            }
        }
    }

    Ok(results)
}

/// 移除符合过滤条件的 Docsy 标记流（物理剥离独立流引用，原正文和其它内容 100% 保持完全不变）。
pub fn remove_marks(
    doc: &mut Document,
    filter: &DocsyMarkFilter,
) -> Result<usize, BackendError> {
    let existing_marks = inspect_marks(doc)?;
    let to_remove: Vec<_> = existing_marks
        .into_iter()
        .filter(|m| matches_filter(m, filter))
        .collect();

    if to_remove.is_empty() {
        return Ok(0);
    }

    let remove_nums: std::collections::HashSet<u32> =
        to_remove.iter().map(|m| m.stream_obj_num).collect();

    let leaf_pages = collect_leaf_pages(doc)?;
    let mut removed_count = 0;

    for page_ref in leaf_pages {
        let streams = page_contents_list(doc, page_ref)?;
        let mut page_removed = 0;
        for r in &streams {
            if remove_nums.contains(&r.num) {
                page_removed += 1;
            }
        }
        if page_removed > 0 {
            removed_count += page_removed;
            let remaining_user_marks = streams.iter().any(|r| {
                !remove_nums.contains(&r.num)
                    && is_docsy_stream(doc, *r)
                    && !is_wrapper_stream(doc, *r)
            });

            let mut final_streams = Vec::new();
            for r in streams {
                if remove_nums.contains(&r.num) {
                    continue;
                }
                if is_wrapper_stream(doc, r) && !remaining_user_marks {
                    // 当页面上所有标记均被清除时，同步清理状态隔离包装流
                    continue;
                }
                final_streams.push(r);
            }

            let new_contents = Object::Array(final_streams.into_iter().map(Object::Ref).collect());
            doc.update_dict(page_ref, |d| {
                d.set(b"Contents".to_vec(), new_contents);
            })
            .map_err(|e| BackendError::ExecutionError {
                backend: super::BackendType::PdfCraft,
                message: e.to_string(),
            })?;
        }
    }

    Ok(removed_count)
}

/// 为页面追加一个独立受管的 Docsy 标记内容流。
pub fn apply_mark_to_page(
    doc: &mut Document,
    page_ref: ObjRef,
    mark: &DocsyMarkSpec,
) -> Result<ObjRef, BackendError> {
    let page_obj = doc.get(page_ref);
    let page_dict = page_obj
        .as_dict()
        .cloned()
        .ok_or_else(|| BackendError::ParseError("页面对象不是 Dictionary".to_string()))?;

    // 验证标记算子引用的字体资源均已在页面资源字典中定义
    if let Ok(content) = lopdf::content::Content::decode(&mark.content_ops) {
        for op in &content.operations {
            if op.operator == "Tf" {
                if let Some(font_name_obj) = op.operands.first() {
                    let font_name = match font_name_obj {
                        lopdf::Object::Name(name) => name.clone(),
                        _ => Vec::new(),
                    };
                    if !font_name.is_empty() && !page_has_font_resource(doc, &page_dict, &font_name) {
                        return Err(BackendError::Other(format!(
                            "未定义的字体资源: /{}",
                            String::from_utf8_lossy(&font_name)
                        )));
                    }
                }
            }
        }
    }

    let (rotation, crop, user_unit) = page_geometry(doc, &page_dict);
    let matrix = view_matrix_for(rotation, crop, user_unit);

    let mut stream_bytes = Vec::new();
    let escaped_id = escape_pdf_string_literal(&mark.id);
    let header = format!(
        "q\n/Artifact <</Type /Pagination /Subtype /{} /Docsy true /DocsyVersion 1 /DocsyKind /{} /DocsyId ({})>> BDC\n{:.4} {:.4} {:.4} {:.4} {:.4} {:.4} cm\n",
        mark.kind.as_str(),
        mark.kind.as_str(),
        escaped_id,
        matrix[0],
        matrix[1],
        matrix[2],
        matrix[3],
        matrix[4],
        matrix[5],
    );
    stream_bytes.extend_from_slice(header.as_bytes());
    stream_bytes.extend_from_slice(&mark.content_ops);
    stream_bytes.extend_from_slice(b"\nEMC\nQ\n");

    let mut stream_dict = Dict::new();
    stream_dict.set(b"Docsy".to_vec(), Object::Bool(true));
    stream_dict.set(b"DocsyVersion".to_vec(), Object::Int(1));
    stream_dict.set(
        b"DocsyKind".to_vec(),
        Object::name(mark.kind.as_str()),
    );
    stream_dict.set(
        b"DocsyId".to_vec(),
        Object::String(PdfString::literal(mark.id.as_bytes().to_vec())),
    );
    if let Some(ref batch) = mark.batch_id {
        stream_dict.set(
            b"DocsyBatch".to_vec(),
            Object::String(PdfString::literal(batch.as_bytes().to_vec())),
        );
    }

    let stream = Stream::flate(stream_dict, &stream_bytes);
    let stream_ref = doc.add(Object::Stream(stream));

    // 图形状态隔离：若页面已有基础正文流，插入 q/Q 包装流，确保基础正文内的未闭合矩阵不污染标记
    let existing_streams = page_contents_list(doc, page_ref)?;
    let mut base_streams = Vec::new();
    let mut push_wrapper = None;
    let mut pop_wrapper = None;
    let mut mark_streams = Vec::new();

    for r in existing_streams {
        if is_wrapper_stream(doc, r) {
            let obj = doc.get(r);
            let dict = obj.as_dict().unwrap();
            if dict.name(b"DocsyWrapper") == Some(b"Push") {
                push_wrapper = Some(r);
            } else if dict.name(b"DocsyWrapper") == Some(b"Pop") {
                pop_wrapper = Some(r);
            }
        } else if is_docsy_stream(doc, r) {
            mark_streams.push(r);
        } else {
            base_streams.push(r);
        }
    }

    if !base_streams.is_empty() {
        if push_wrapper.is_none() {
            let mut push_dict = Dict::new();
            push_dict.set(b"Docsy".to_vec(), Object::Bool(true));
            push_dict.set(b"DocsyWrapper".to_vec(), Object::name("Push"));
            let push_stream = Stream::flate(push_dict, b"q\n");
            push_wrapper = Some(doc.add(Object::Stream(push_stream)));
        }
        if pop_wrapper.is_none() {
            let mut pop_dict = Dict::new();
            pop_dict.set(b"Docsy".to_vec(), Object::Bool(true));
            pop_dict.set(b"DocsyWrapper".to_vec(), Object::name("Pop"));
            let pop_stream = Stream::flate(pop_dict, b"\nQ\n");
            pop_wrapper = Some(doc.add(Object::Stream(pop_stream)));
        }
    }

    let mut new_streams = Vec::new();
    if let Some(push) = push_wrapper {
        new_streams.push(push);
    }
    new_streams.extend(base_streams);
    if let Some(pop) = pop_wrapper {
        new_streams.push(pop);
    }
    new_streams.extend(mark_streams);
    new_streams.push(stream_ref);

    let new_contents = Object::Array(new_streams.into_iter().map(Object::Ref).collect());
    doc.update_dict(page_ref, |d| {
        d.set(b"Contents".to_vec(), new_contents);
    })
    .map_err(|e| BackendError::ExecutionError {
        backend: super::BackendType::PdfCraft,
        message: e.to_string(),
    })?;

    Ok(stream_ref)
}

/// 批量在指定页面添加或更新标记。
pub fn apply_or_update_marks(
    doc: &mut Document,
    marks: &[DocsyMarkSpec],
) -> Result<(), BackendError> {
    let leaf_pages = collect_leaf_pages(doc)?;

    for mark in marks {
        if mark.page_index >= leaf_pages.len() {
            return Err(BackendError::Other(format!(
                "页面索引 {} 超出总页数 {}",
                mark.page_index + 1,
                leaf_pages.len()
            )));
        }
        let page_ref = leaf_pages[mark.page_index];

        // 幂等更新：若同 ID 的标记已存在，则先移除该页上的旧标记流
        let filter = DocsyMarkFilter {
            id: Some(mark.id.clone()),
            kind: None,
            batch_id: None,
        };
        let _ = remove_marks(doc, &filter)?;

        apply_mark_to_page(doc, page_ref, mark)?;
    }

    Ok(())
}
