//! 原生 PDF 注释、表单 Widget 与电子签章/水印外观压平（Print Flattening）。
//!
//! 对齐 ISO 32000-1 §12.5.5 (Appearance Streams) 与 §8.4.2 (Annotation Flags)：
//! - 将可见注释与表单控件（含 `/Widget`、`/Stamp`、`/Watermark` 等）的 `/AP /N` 外观流烘焙进页面 `/Contents`；
//! - 支持带状态外观字典（如复选框/单选框通过 `/AS` 选择子流）与带旋转/缩放的 `/Matrix` 仿射变换；
//! - 遵循打印压平语义：包含 `NoView` (`F` bit 6) 注释，剔除 `Invisible` (`F` bit 1) 与 `Hidden` (`F` bit 2) 注释而不绘制；
//! - 对退化 `/BBox`（宽或高为 0）不绘制任何内容并安全移除注释与字段引用；
//! - 对 `NeedAppearances=true`（过期外观）或缺少 `/AP /N` 的可见表单 Widget 主动返回 `BackendError::Unsupported`，
//!   由 `dispatch_merge` 受控回退至 `qpdf` 或显式报错，绝不静默产出残缺或仍可编辑的打印件。

use super::BackendError;
use pdfcraft_cos::{Dict, Document, ObjRef, Object, Stream};
use std::collections::HashSet;

const INVISIBLE: i64 = 1 << 0;
const HIDDEN: i64 = 1 << 1;
const NO_ROTATE: i64 = 1 << 4;

/// 不参与视觉外观压平的导航/弹窗类注释子类型。
const NON_FLATTENED_SUBTYPES: [&[u8]; 3] = [b"Link", b"Popup", b"Projection"];

/// 检查文档是否包含 `NeedAppearances=true` 且含有页面 Widget（过期表单外观）。
pub(crate) fn has_stale_form_appearances(doc: &Document) -> Result<bool, BackendError> {
    if !has_need_appearances(doc) {
        return Ok(false);
    }
    let pages = pdfcraft_organize::walk(doc)
        .map_err(|e| BackendError::ParseError(format!("遍历页面树失败: {e}")))?;
    Ok(document_has_widgets(doc, &pages))
}

/// 将文档所有页面上的注释、签章与表单 Widget 外观压平至页面内容流中。
///
/// 返回实际绘制到页面内容流中的外观数量。
pub fn flatten_document(doc: &mut Document) -> Result<usize, BackendError> {
    let pages = pdfcraft_organize::walk(doc)
        .map_err(|e| BackendError::ParseError(format!("遍历页面树失败: {e}")))?;

    // 检查是否存在 NeedAppearances=true 且包含页面 Widget 的情况（过期外观）
    if has_need_appearances(doc) && document_has_widgets(doc, &pages) {
        return Err(BackendError::Unsupported(
            "文档包含 NeedAppearances=true 的未更新表单外观，需回退至 Qpdf 重新生成外观后再压平"
                .to_string(),
        ));
    }

    let mut total_drawn = 0usize;
    let mut removed_widgets: HashSet<ObjRef> = HashSet::new();

    for (page_ref, _) in pages {
        total_drawn += flatten_page(doc, page_ref, &mut removed_widgets)?;
    }

    if !removed_widgets.is_empty() {
        cleanup_acroform_fields(doc, &removed_widgets)?;
    }

    Ok(total_drawn)
}

fn has_need_appearances(doc: &Document) -> bool {
    let Some(root) = doc.root() else {
        return false;
    };
    let root_obj = doc.get(root);
    let Some(catalog) = root_obj.as_dict() else {
        return false;
    };
    let Some(acro) = catalog.get(b"AcroForm").and_then(|o| doc.dict(o)) else {
        return false;
    };
    acro.get(b"NeedAppearances")
        .is_some_and(|v| matches!(&*doc.resolve(v), Object::Bool(true)))
}

fn document_has_widgets(doc: &Document, pages: &[(ObjRef, Dict)]) -> bool {
    for (page_ref, _) in pages {
        let page_obj = doc.get(*page_ref);
        let Some(page_dict) = page_obj.as_dict() else {
            continue;
        };
        let Some(annots) = page_dict
            .get(b"Annots")
            .map(|a| doc.resolve(a))
            .and_then(|a| a.as_array().cloned())
        else {
            continue;
        };
        for item in &annots {
            let resolved = doc.resolve(item);
            if let Some(d) = resolved.as_dict() {
                if is_widget_dict(doc, d) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_widget_dict(doc: &Document, d: &Dict) -> bool {
    d.name(b"Subtype") == Some(b"Widget")
        || (d.name(b"Subtype").is_none() && inherited_attr(doc, d, b"FT").is_some())
}

fn effective_page_rotation(doc: &Document, page: ObjRef) -> i32 {
    let mut current = Some(page);
    let mut depth = 0usize;
    while let Some(r) = current {
        if depth > 32 {
            break;
        }
        depth += 1;
        let obj = doc.get(r);
        let Some(dict) = obj.as_dict() else {
            break;
        };
        if let Some(rot) = dict.get(b"Rotate").and_then(|v| doc.resolve(v).as_int()) {
            let normalized = (rot as i32).rem_euclid(360);
            if matches!(normalized, 0 | 90 | 180 | 270) {
                return normalized;
            }
        }
        current = dict.reference(b"Parent");
    }
    0
}

fn flatten_page(
    doc: &mut Document,
    page: ObjRef,
    removed_widgets: &mut HashSet<ObjRef>,
) -> Result<usize, BackendError> {
    let page_obj = doc.get(page);
    let Some(annots) = page_obj
        .as_dict()
        .and_then(|d| d.get(b"Annots"))
        .map(|a| doc.resolve(a))
        .and_then(|a| a.as_array().cloned())
    else {
        return Ok(0);
    };
    drop(page_obj);
    if annots.is_empty() {
        return Ok(0);
    }

    let page_rotate = effective_page_rotation(doc, page);
    let mut used_names = existing_xobject_names(doc, page);
    let mut new_xobjects: Vec<(Vec<u8>, ObjRef)> = Vec::new();
    let mut overlay = Vec::new();
    let mut kept_annots: Vec<Object> = Vec::new();
    let mut drawn = 0usize;
    let mut any_removed = false;

    for (idx, item) in annots.iter().enumerate() {
        let annot_ref = item.as_ref();
        let resolved = doc.resolve(item);
        let Some(annot_dict) = resolved.as_dict() else {
            kept_annots.push(item.clone());
            continue;
        };

        let subtype = annot_dict.name(b"Subtype").unwrap_or(b"").to_vec();
        if NON_FLATTENED_SUBTYPES.contains(&subtype.as_slice()) {
            kept_annots.push(item.clone());
            continue;
        }

        let is_widget = is_widget_dict(doc, annot_dict);
        let flags = annot_dict
            .get(b"F")
            .and_then(|v| doc.resolve(v).as_int())
            .unwrap_or(0);

        // Invisible 或 Hidden 的注释/Widget：不绘制外观，但从页面 /Annots 和 /AcroForm /Fields 中移除
        if flags & (INVISIBLE | HIDDEN) != 0 {
            any_removed = true;
            if let Some(r) = annot_ref {
                removed_widgets.insert(r);
            }
            continue;
        }

        let rect = rect_of(doc, annot_dict, b"Rect");
        let has_valid_rect = rect
            .is_some_and(|[x1, y1, x2, y2]| (x2 - x1).abs() > 1e-6 && (y2 - y1).abs() > 1e-6);

        let Some(rect_box) = rect.filter(|_| has_valid_rect) else {
            any_removed = true;
            if let Some(r) = annot_ref {
                removed_widgets.insert(r);
            }
            continue;
        };

        let ap_ref = match normal_appearance(doc, annot_dict) {
            Some(ap) => {
                drop(resolved);
                ap
            }
            None => {
                if is_widget {
                    return Err(BackendError::Unsupported(
                        "可见表单 Widget 缺少 /AP /N 外观流，无法执行原生打印压平".to_string(),
                    ));
                }
                let annot_cloned = annot_dict.clone();
                drop(resolved);
                if let Some(synthesized) =
                    synthesize_missing_appearance(doc, &subtype, &annot_cloned, rect_box)
                {
                    synthesized
                } else {
                    return Err(BackendError::Unsupported(format!(
                        "可见注释 (/{}) 缺少 /AP /N 外观流且暂不支持原生合成，需回退至外置工具处理",
                        String::from_utf8_lossy(&subtype)
                    )));
                }
            }
        };

        // 计算将 Form XObject /BBox (经 /Matrix 变换后) 映射到注释 /Rect 的仿射矩阵（含 NoRotate 补偿）
        if let Some([a, b, c, d, e, f]) = placement(doc, ap_ref, rect_box, flags, page_rotate) {
            ensure_form_xobject_headers(doc, ap_ref)?;
            let name = unique_xobject_name(&mut used_names, idx);
            let name_str = String::from_utf8_lossy(&name);
            new_xobjects.push((name.clone(), ap_ref));

            let snippet = format!(
                "q\n{:.6} {:.6} {:.6} {:.6} {:.6} {:.6} cm\n/{} Do\nQ\n",
                a, b, c, d, e, f, name_str
            );
            overlay.extend_from_slice(snippet.as_bytes());
            drawn += 1;
        }

        // 无论是否因 zero_bbox 跳过绘制，该注释/Widget 都已被压平消费，需从 /Annots 与 /Fields 移除
        any_removed = true;
        if let Some(r) = annot_ref {
            removed_widgets.insert(r);
        }
    }

    if !any_removed && drawn == 0 {
        return Ok(0);
    }

    if !overlay.is_empty() {
        wrap_existing_page_contents(doc, page)?;
        let overlay_ref = add_compressed_stream(doc, &overlay);
        append_page_content_stream(doc, page, overlay_ref)?;
        register_page_xobjects(doc, page, &new_xobjects)?;
    }

    doc.update_dict(page, |d| {
        if kept_annots.is_empty() {
            d.remove(b"Annots");
        } else {
            d.set(b"Annots".to_vec(), Object::Array(kept_annots));
        }
    })
    .map_err(|e| BackendError::ParseError(format!("更新页面 /Annots 失败: {e}")))?;

    Ok(drawn)
}

/// 解析注释的正常外观流 `/AP /N`。
///
/// `/N` 可以直接是 Form XObject 流引用，也可以是按 `/AS`（或字段 `/V`）索引的状态外观字典。
fn normal_appearance(doc: &Document, annot: &Dict) -> Option<ObjRef> {
    let ap_obj = annot.get(b"AP")?;
    let ap = doc.dict(ap_obj)?;
    let n_obj = ap.get(b"N")?;

    if let Object::Ref(r) = n_obj {
        if matches!(&*doc.get(*r), Object::Stream(_)) {
            return Some(*r);
        }
    }

    let state_dict = doc.dict(n_obj)?;
    let state_name = annot
        .name(b"AS")
        .map(<[u8]>::to_vec)
        .or_else(|| {
            inherited_attr(doc, annot, b"V")
                .and_then(|v| doc.resolve(&v).as_name().map(<[u8]>::to_vec))
        });

    if let Some(ref state) = state_name {
        if let Some(r) = state_dict.reference(state) {
            if matches!(&*doc.get(r), Object::Stream(_)) {
                return Some(r);
            }
        }
    }

    // 若状态为 Off，不回退到勾选态流
    if state_name.as_deref() == Some(b"Off") {
        return None;
    }

    // 兜底：若状态字典中仅有一个非 Off 外观流，选取它
    for (k, v) in state_dict.iter() {
        if k.as_slice() != b"Off" {
            if let Some(r) = v.as_ref() {
                if matches!(&*doc.get(r), Object::Stream(_)) {
                    return Some(r);
                }
            }
        }
    }
    None
}

/// 沿 Widget 的 `/Parent` 链向上查找表单字段属性（如 `/FT`、`/V`）。
fn inherited_attr(doc: &Document, annot: &Dict, key: &[u8]) -> Option<Object> {
    if let Some(v) = annot.get(key) {
        return Some(v.clone());
    }
    let mut cur = annot.reference(b"Parent");
    for _ in 0..16 {
        let r = cur?;
        let obj = doc.get(r);
        let d = obj.as_dict()?;
        if let Some(v) = d.get(key) {
            return Some(v.clone());
        }
        cur = d.reference(b"Parent");
    }
    None
}

/// 为缺少 `/AP /N` 的常见可见标记/几何注释（如 `/Highlight`、`/Underline`、`/StrikeOut`、`/Square`）合成标准 Form XObject 外观流。
fn synthesize_missing_appearance(
    doc: &mut Document,
    subtype: &[u8],
    annot: &Dict,
    rect: [f64; 4],
) -> Option<ObjRef> {
    let rx1 = rect[0].min(rect[2]);
    let ry1 = rect[1].min(rect[3]);
    let rx2 = rect[0].max(rect[2]);
    let ry2 = rect[1].max(rect[3]);
    if (rx2 - rx1) <= 1e-6 || (ry2 - ry1) <= 1e-6 {
        return None;
    }

    let ca = annot
        .get(b"CA")
        .and_then(|v| doc.resolve(v).as_f64())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);

    let quads = extract_quad_boxes(doc, annot).unwrap_or_else(|| vec![[rx1, ry1, rx2, ry2]]);
    let mut ops = String::from("q\n/GS0 gs\n");
    let mut blend_multiply = false;

    match subtype {
        b"Highlight" => {
            blend_multiply = true;
            let color = extract_color(doc, annot, b"C").unwrap_or_else(|| vec![1.0, 1.0, 0.0]);
            ops.push_str(&color_operator(&color, true)?);
            for [qx1, qy1, qx2, qy2] in quads {
                let w = (qx2 - qx1).max(0.0);
                let h = (qy2 - qy1).max(0.0);
                if w > 1e-6 && h > 1e-6 {
                    ops.push_str(&format!("{qx1:.6} {qy1:.6} {w:.6} {h:.6} re f\n"));
                }
            }
        }
        b"Underline" | b"StrikeOut" => {
            let color = extract_color(doc, annot, b"C").unwrap_or_else(|| vec![1.0, 0.0, 0.0]);
            ops.push_str(&color_operator(&color, false)?);
            for [qx1, qy1, qx2, qy2] in quads {
                let h = (qy2 - qy1).max(0.0);
                if (qx2 - qx1) <= 1e-6 || h <= 1e-6 {
                    continue;
                }
                let y = if subtype == b"Underline" {
                    qy1 + h * 0.12
                } else {
                    qy1 + h * 0.5
                };
                let lw = (h * 0.06).max(0.75);
                ops.push_str(&format!(
                    "{lw:.4} w\n{qx1:.6} {y:.6} m\n{qx2:.6} {y:.6} l\nS\n"
                ));
            }
        }
        b"Square" => {
            let stroke = extract_color(doc, annot, b"C");
            let fill = extract_color(doc, annot, b"IC");
            if stroke.is_none() && fill.is_none() {
                return None;
            }
            if let Some(c) = &fill {
                ops.push_str(&color_operator(c, true)?);
            }
            if let Some(c) = &stroke {
                ops.push_str(&color_operator(c, false)?);
            }
            let w = rx2 - rx1;
            let h = ry2 - ry1;
            let paint = match (fill.is_some(), stroke.is_some()) {
                (true, true) => "B",
                (true, false) => "f",
                (false, true) => "S",
                (false, false) => return None,
            };
            ops.push_str(&format!("1 w\n{rx1:.6} {ry1:.6} {w:.6} {h:.6} re {paint}\n"));
        }
        _ => return None,
    }
    ops.push_str("Q\n");

    let mut gs = Dict::new();
    gs.set(b"Type".to_vec(), Object::name("ExtGState"));
    gs.set(b"CA".to_vec(), Object::Real(ca));
    gs.set(b"ca".to_vec(), Object::Real(ca));
    if blend_multiply {
        gs.set(b"BM".to_vec(), Object::name("Multiply"));
        gs.set(b"AIS".to_vec(), Object::Bool(true));
    }
    let mut ext_gstate = Dict::new();
    ext_gstate.set(b"GS0".to_vec(), Object::Dict(gs));
    let mut resources = Dict::new();
    resources.set(b"ExtGState".to_vec(), Object::Dict(ext_gstate));

    let mut form_dict = Dict::new();
    form_dict.set(b"Type".to_vec(), Object::name("XObject"));
    form_dict.set(b"Subtype".to_vec(), Object::name("Form"));
    form_dict.set(
        b"BBox".to_vec(),
        Object::Array(vec![
            Object::Real(rx1),
            Object::Real(ry1),
            Object::Real(rx2),
            Object::Real(ry2),
        ]),
    );
    form_dict.set(b"Resources".to_vec(), Object::Dict(resources));

    let stream = Stream::flate(form_dict, ops.as_bytes());
    Some(doc.add(Object::Stream(stream)))
}

fn extract_color(doc: &Document, dict: &Dict, key: &[u8]) -> Option<Vec<f64>> {
    let arr = dict
        .get(key)
        .map(|v| doc.resolve(v))
        .and_then(|v| v.as_array().cloned())?;
    if !matches!(arr.len(), 1 | 3 | 4) {
        return None;
    }
    let mut vals = Vec::with_capacity(arr.len());
    for item in &arr {
        vals.push(doc.resolve(item).as_f64()?.clamp(0.0, 1.0));
    }
    Some(vals)
}

fn color_operator(color: &[f64], fill: bool) -> Option<String> {
    match (color, fill) {
        ([g], true) => Some(format!("{g:.6} g\n")),
        ([g], false) => Some(format!("{g:.6} G\n")),
        ([r, g, b], true) => Some(format!("{r:.6} {g:.6} {b:.6} rg\n")),
        ([r, g, b], false) => Some(format!("{r:.6} {g:.6} {b:.6} RG\n")),
        ([c, m, y, k], true) => Some(format!("{c:.6} {m:.6} {y:.6} {k:.6} k\n")),
        ([c, m, y, k], false) => Some(format!("{c:.6} {m:.6} {y:.6} {k:.6} K\n")),
        _ => None,
    }
}

fn extract_quad_boxes(doc: &Document, dict: &Dict) -> Option<Vec<[f64; 4]>> {
    let arr = dict
        .get(b"QuadPoints")
        .map(|v| doc.resolve(v))
        .and_then(|v| v.as_array().cloned())?;
    if arr.len() < 8 || arr.len() % 8 != 0 {
        return None;
    }
    let mut boxes = Vec::with_capacity(arr.len() / 8);
    for chunk in arr.chunks_exact(8) {
        let mut xs = [0.0f64; 4];
        let mut ys = [0.0f64; 4];
        for i in 0..4 {
            xs[i] = doc.resolve(&chunk[i * 2]).as_f64()?;
            ys[i] = doc.resolve(&chunk[i * 2 + 1]).as_f64()?;
        }
        let x1 = xs.into_iter().fold(f64::INFINITY, f64::min);
        let y1 = ys.into_iter().fold(f64::INFINITY, f64::min);
        let x2 = xs.into_iter().fold(f64::NEG_INFINITY, f64::max);
        let y2 = ys.into_iter().fold(f64::NEG_INFINITY, f64::max);
        if (x2 - x1) > 1e-6 && (y2 - y1) > 1e-6 {
            boxes.push([x1, y1, x2, y2]);
        }
    }
    if boxes.is_empty() {
        None
    } else {
        Some(boxes)
    }
}

/// 计算 ISO 32000-1 §12.5.5 外观放置仿射变换矩阵 `[a, b, c, d, e, f]`（含 §12.5.3 `NoRotate` 补偿）。
///
/// Form XObject 绘制时会先应用其自身的 `/Matrix`，再应用此放置矩阵 `cm`。
/// 因此我们将 `/BBox` 四角先经 `/Matrix` 变换得到变换后包围盒 `T_box`，
/// 再求把 `T_box` 缩放平移至注释 `/Rect` 的矩阵；若设置了 `NoRotate` 标志，
/// 则以注释 `/Rect` 在默认用户空间中的左上角 `(rx1, ry2)` 为枢轴施加逆旋转补偿。
fn placement(
    doc: &Document,
    ap_ref: ObjRef,
    rect: [f64; 4],
    flags: i64,
    page_rotate: i32,
) -> Option<[f64; 6]> {
    let ap_obj = doc.get(ap_ref);
    let Object::Stream(stream) = &*ap_obj else {
        return None;
    };
    let bbox = rect_of(doc, &stream.dict, b"BBox")?;
    let m = matrix_of(doc, &stream.dict, b"Matrix").unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    let [bx1, by1, bx2, by2] = bbox;
    if (bx2 - bx1).abs() <= 1e-6 || (by2 - by1).abs() <= 1e-6 {
        return None;
    }

    let corners = [(bx1, by1), (bx2, by1), (bx2, by2), (bx1, by2)];
    let (mut tx1, mut ty1, mut tx2, mut ty2) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for (x, y) in corners {
        let tx = m[0] * x + m[2] * y + m[4];
        let ty = m[1] * x + m[3] * y + m[5];
        tx1 = tx1.min(tx);
        ty1 = ty1.min(ty);
        tx2 = tx2.max(tx);
        ty2 = ty2.max(ty);
    }

    let tw = tx2 - tx1;
    let th = ty2 - ty1;
    if tw <= 1e-6 || th <= 1e-6 || !tw.is_finite() || !th.is_finite() {
        return None;
    }

    let rx1 = rect[0].min(rect[2]);
    let ry1 = rect[1].min(rect[3]);
    let ry2 = rect[1].max(rect[3]);
    let rw = (rect[2] - rect[0]).abs();
    let rh = (rect[3] - rect[1]).abs();
    if rw <= 1e-6 || rh <= 1e-6 {
        return None;
    }

    let sx = rw / tw;
    let sy = rh / th;
    let base = [sx, 0.0, 0.0, sy, rx1 - tx1 * sx, ry1 - ty1 * sy];

    if (flags & NO_ROTATE) != 0 {
        let px = rx1;
        let py = ry2;
        let rot = match page_rotate.rem_euclid(360) {
            90 => [0.0, 1.0, -1.0, 0.0, px + py, py - px],
            180 => [-1.0, 0.0, 0.0, -1.0, 2.0 * px, 2.0 * py],
            270 => [0.0, -1.0, 1.0, 0.0, px - py, px + py],
            _ => return Some(base),
        };
        return Some([
            rot[0] * base[0] + rot[2] * base[1],
            rot[1] * base[0] + rot[3] * base[1],
            rot[0] * base[2] + rot[2] * base[3],
            rot[1] * base[2] + rot[3] * base[3],
            rot[0] * base[4] + rot[2] * base[5] + rot[4],
            rot[1] * base[4] + rot[3] * base[5] + rot[5],
        ]);
    }

    Some(base)
}

fn rect_of(doc: &Document, dict: &Dict, key: &[u8]) -> Option<[f64; 4]> {
    let arr = dict
        .get(key)
        .map(|v| doc.resolve(v))
        .and_then(|v| v.as_array().cloned())?;
    if arr.len() < 4 {
        return None;
    }
    Some([
        doc.resolve(&arr[0]).as_f64()?,
        doc.resolve(&arr[1]).as_f64()?,
        doc.resolve(&arr[2]).as_f64()?,
        doc.resolve(&arr[3]).as_f64()?,
    ])
}

fn matrix_of(doc: &Document, dict: &Dict, key: &[u8]) -> Option<[f64; 6]> {
    let arr = dict
        .get(key)
        .map(|v| doc.resolve(v))
        .and_then(|v| v.as_array().cloned())?;
    if arr.len() < 6 {
        return None;
    }
    Some([
        doc.resolve(&arr[0]).as_f64()?,
        doc.resolve(&arr[1]).as_f64()?,
        doc.resolve(&arr[2]).as_f64()?,
        doc.resolve(&arr[3]).as_f64()?,
        doc.resolve(&arr[4]).as_f64()?,
        doc.resolve(&arr[5]).as_f64()?,
    ])
}

/// 确保被引用的外观流具备 `/Type /XObject` 与 `/Subtype /Form` 头字段。
fn ensure_form_xobject_headers(doc: &mut Document, ap_ref: ObjRef) -> Result<(), BackendError> {
    let ap_obj = doc.get(ap_ref);
    let Object::Stream(stream) = &*ap_obj else {
        return Ok(());
    };
    if stream.dict.name(b"Subtype") == Some(b"Form")
        && stream.dict.name(b"Type") == Some(b"XObject")
    {
        return Ok(());
    }
    drop(ap_obj);
    doc.update_dict(ap_ref, |d| {
        d.set(b"Type".to_vec(), Object::name("XObject"));
        d.set(b"Subtype".to_vec(), Object::name("Form"));
    })
    .map_err(|e| BackendError::ParseError(format!("更新 Form XObject 字典失败: {e}")))
}

fn existing_xobject_names(doc: &Document, page: ObjRef) -> HashSet<Vec<u8>> {
    let mut names = HashSet::new();
    let page_obj = doc.get(page);
    let Some(page_dict) = page_obj.as_dict() else {
        return names;
    };
    let Some(res) = page_dict.get(b"Resources").and_then(|r| doc.dict(r)) else {
        return names;
    };
    let Some(xobj) = res.get(b"XObject").and_then(|x| doc.dict(x)) else {
        return names;
    };
    for (k, _) in xobj.iter() {
        names.insert(k.clone());
    }
    names
}

fn unique_xobject_name(used: &mut HashSet<Vec<u8>>, seed: usize) -> Vec<u8> {
    let mut idx = seed;
    loop {
        let candidate = format!("FmFlat{idx}").into_bytes();
        if used.insert(candidate.clone()) {
            return candidate;
        }
        idx += 1;
    }
}

/// 用 `q ... Q` 包裹页面原有内容流，防止正文遗留的图形状态（CTM 缩放/平移/裁剪）偏置后续叠加的签章或注释外观。
fn wrap_existing_page_contents(doc: &mut Document, page: ObjRef) -> Result<(), BackendError> {
    let existing = doc
        .get(page)
        .as_dict()
        .and_then(|d| d.get(b"Contents").cloned());

    let refs: Vec<ObjRef> = match existing {
        None => return Ok(()),
        Some(Object::Ref(r)) => match &*doc.get(r) {
            Object::Stream(_) => vec![r],
            Object::Array(arr) => arr.iter().filter_map(Object::as_ref).collect(),
            _ => return Ok(()),
        },
        Some(Object::Array(arr)) => arr.iter().filter_map(Object::as_ref).collect(),
        _ => return Ok(()),
    };

    if refs.is_empty() {
        return Ok(());
    }

    // 检查原内容流中 q / Q 嵌套深度，若存在未闭合的 q 则补齐 Q
    let mut depth = 0usize;
    for r in &refs {
        let stream_obj = doc.get(*r);
        if let Object::Stream(stream) = &*stream_obj {
            if let Ok(bytes) = stream.decoded() {
                if let Ok(content) = lopdf::content::Content::decode(&bytes) {
                    for op in content.operations {
                        match op.operator.as_str() {
                            "q" => depth = depth.saturating_add(1),
                            "Q" => depth = depth.saturating_sub(1),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let push_ref = add_compressed_stream(doc, b"q\n");
    let mut pop_bytes = vec![b'\n'];
    for _ in 0..=depth {
        pop_bytes.extend_from_slice(b"Q\n");
    }
    let pop_ref = add_compressed_stream(doc, &pop_bytes);

    let mut wrapped = Vec::with_capacity(refs.len() + 2);
    wrapped.push(Object::Ref(push_ref));
    wrapped.extend(refs.into_iter().map(Object::Ref));
    wrapped.push(Object::Ref(pop_ref));

    doc.update_dict(page, |d| {
        d.set(b"Contents".to_vec(), Object::Array(wrapped));
    })
    .map_err(|e| BackendError::ParseError(format!("包裹页面内容流失败: {e}")))
}

fn append_page_content_stream(
    doc: &mut Document,
    page: ObjRef,
    stream_ref: ObjRef,
) -> Result<(), BackendError> {
    let existing = doc
        .get(page)
        .as_dict()
        .and_then(|d| d.get(b"Contents").cloned());

    let new_contents = match existing {
        None => Object::Ref(stream_ref),
        Some(Object::Ref(r)) => match &*doc.get(r) {
            Object::Array(arr) => {
                let mut v = arr.clone();
                v.push(Object::Ref(stream_ref));
                Object::Array(v)
            }
            _ => Object::Array(vec![Object::Ref(r), Object::Ref(stream_ref)]),
        },
        Some(Object::Array(mut arr)) => {
            arr.push(Object::Ref(stream_ref));
            Object::Array(arr)
        }
        _ => Object::Ref(stream_ref),
    };

    doc.update_dict(page, |d| {
        d.set(b"Contents".to_vec(), new_contents);
    })
    .map_err(|e| BackendError::ParseError(format!("追加压平内容流失败: {e}")))
}

fn register_page_xobjects(
    doc: &mut Document,
    page: ObjRef,
    entries: &[(Vec<u8>, ObjRef)],
) -> Result<(), BackendError> {
    if entries.is_empty() {
        return Ok(());
    }
    let page_obj = doc.get(page);
    let mut res = page_obj
        .as_dict()
        .and_then(|d| d.get(b"Resources"))
        .and_then(|r| doc.dict(r))
        .unwrap_or_default();
    drop(page_obj);

    let mut xobj = res
        .get(b"XObject")
        .and_then(|x| doc.dict(x))
        .unwrap_or_default();

    for (name, r) in entries {
        xobj.set(name.clone(), Object::Ref(*r));
    }

    res.set(b"XObject".to_vec(), Object::Dict(xobj));
    doc.update_dict(page, |d| {
        d.set(b"Resources".to_vec(), Object::Dict(res));
    })
    .map_err(|e| BackendError::ParseError(format!("注册页面 XObject 资源失败: {e}")))
}

fn add_compressed_stream(doc: &mut Document, raw: &[u8]) -> ObjRef {
    doc.add(Object::Stream(Stream::flate(Dict::new(), raw)))
}

/// 从 `/Catalog -> /AcroForm -> /Fields` 中清理已压平或移除的 Widget 及其空的父字段。
fn cleanup_acroform_fields(
    doc: &mut Document,
    removed_widgets: &HashSet<ObjRef>,
) -> Result<(), BackendError> {
    let Some(root) = doc.root() else {
        return Ok(());
    };
    let Some(acro_obj) = doc
        .get(root)
        .as_dict()
        .and_then(|c| c.get(b"AcroForm").cloned())
    else {
        return Ok(());
    };

    let Some(acro_dict) = doc.dict(&acro_obj) else {
        return Ok(());
    };

    let Some(fields) = acro_dict
        .get(b"Fields")
        .map(|f| doc.resolve(f))
        .and_then(|f| f.as_array().cloned())
    else {
        return Ok(());
    };

    let mut kept_fields = Vec::new();
    for field_item in fields {
        let Some(field_ref) = field_item.as_ref() else {
            kept_fields.push(field_item);
            continue;
        };
        if removed_widgets.contains(&field_ref) {
            continue;
        }
        // 检查父级字段的 /Kids
        if let Some(field_dict) = doc.get(field_ref).as_dict().cloned() {
            if let Some(kids) = field_dict
                .get(b"Kids")
                .map(|k| doc.resolve(k))
                .and_then(|k| k.as_array().cloned())
            {
                let kept_kids: Vec<Object> = kids
                    .into_iter()
                    .filter(|k| {
                        k.as_ref()
                            .map(|kr| !removed_widgets.contains(&kr))
                            .unwrap_or(true)
                    })
                    .collect();
                if kept_kids.is_empty() {
                    continue;
                }
                let _ = doc.update_dict(field_ref, |d| {
                    d.set(b"Kids".to_vec(), Object::Array(kept_kids));
                });
            }
        }
        kept_fields.push(field_item);
    }

    match acro_obj {
        Object::Ref(acro_ref) => {
            doc.update_dict(acro_ref, |d| {
                d.set(b"Fields".to_vec(), Object::Array(kept_fields));
                d.remove(b"NeedAppearances");
            })
            .map_err(|e| BackendError::ParseError(format!("更新 AcroForm 失败: {e}")))?;
        }
        Object::Dict(mut d) => {
            d.set(b"Fields".to_vec(), Object::Array(kept_fields));
            d.remove(b"NeedAppearances");
            doc.update_dict(root, |c| {
                c.set(b"AcroForm".to_vec(), Object::Dict(d));
            })
            .map_err(|e| BackendError::ParseError(format!("更新内联 AcroForm 失败: {e}")))?;
        }
        _ => {}
    }

    Ok(())
}
