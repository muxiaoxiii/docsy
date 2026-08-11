use anyhow::{Context, Result};
use image::ImageEncoder;
use lopdf::{content::Content, Document, Object, ObjectId};
use std::collections::HashMap;
use std::path::Path;

/// 图片压缩阶段进度。total 为 0 表示阶段没有可计数的子项。
#[derive(Debug, Clone)]
pub struct CompressProgress {
    pub phase: &'static str,
    pub current: usize,
    pub total: usize,
    pub detail: Option<String>,
}

impl CompressProgress {
    pub fn phase(label: &'static str) -> Self {
        Self {
            phase: label,
            current: 0,
            total: 0,
            detail: None,
        }
    }

    pub fn item(label: &'static str, current: usize, total: usize) -> Self {
        Self {
            phase: label,
            current,
            total,
            detail: None,
        }
    }

    pub fn done() -> Self {
        Self {
            phase: "压缩完成",
            current: 1,
            total: 1,
            detail: None,
        }
    }

    pub fn label(&self) -> String {
        match (self.total, self.current, self.detail.as_deref()) {
            (total, current, _) if total > 0 => format!("{}（{}/{}）", self.phase, current, total),
            (_, _, Some(detail)) if !detail.is_empty() => format!("{}：{}", self.phase, detail),
            _ => format!("{}…", self.phase),
        }
    }
}

/// 压缩选项
pub struct CompressOptions {
    /// JPEG 质量 1-100
    pub jpeg_quality: u8,
    /// 目标 DPI：仅当图片在页面上的有效 DPI 超过该值才降采样
    pub target_dpi: u32,
}

impl CompressOptions {
    pub fn from_level(level: u8) -> Self {
        match level {
            // 清晰优先（默认）
            1 => Self {
                jpeg_quality: 92,
                target_dpi: 300,
            },
            // 体积最小
            3 => Self {
                jpeg_quality: 75,
                target_dpi: 150,
            },
            // 均衡
            _ => Self {
                jpeg_quality: 85,
                target_dpi: 200,
            },
        }
    }
}

/// 对 PDF 中的图片进行重编码压缩。
///
/// 流程：lopdf 读取 → 解析内容流计算每张图片的有效 DPI → 遍历 Image XObject
/// → 按策略决定跳过/降采样/转码 → 写回 → 保存。
pub fn compress_pdf_with_progress(
    input: &Path,
    output: &Path,
    options: &CompressOptions,
    progress: &mut dyn FnMut(CompressProgress),
) -> Result<()> {
    let mut doc = Document::load(input).context("读取 PDF 失败")?;

    let image_ids = collect_image_xobjects(&doc);
    progress(CompressProgress::item("分析页面图像", 0, image_ids.len()));
    let effective_dpi = collect_effective_dpi(&doc, &image_ids, progress);
    let mut compressed_count = 0u32;
    let mut saved_bytes: u64 = 0;

    let image_total = image_ids.len();
    for (index, obj_id) in image_ids.into_iter().enumerate() {
        let original_size = stream_data_size(&doc, obj_id);
        let max_dpi = effective_dpi.get(&obj_id).copied();
        if let Err(e) = recompress_image(&mut doc, obj_id, options, max_dpi) {
            // 跳过无法处理的图片，不中断整个压缩流程
            log::warn!("跳过图片 {:?}: {} ({}KB)", obj_id, e, original_size / 1024);
        } else {
            let new_size = stream_data_size(&doc, obj_id);
            if new_size < original_size {
                compressed_count += 1;
                saved_bytes += (original_size - new_size) as u64;
                log::info!(
                    "压缩图片 {:?}: {}KB → {}KB (节省 {}KB)",
                    obj_id,
                    original_size / 1024,
                    new_size / 1024,
                    (original_size - new_size) / 1024
                );
            }
        }
        progress(CompressProgress::item("处理图片", index + 1, image_total));
    }

    log::info!(
        "图片压缩完成：{} 张图片被重编码，节省 {:.1} MB",
        compressed_count,
        saved_bytes as f64 / 1024.0 / 1024.0
    );

    doc.compress();
    progress(CompressProgress::phase("保存压缩 PDF"));
    doc.save(output).context("保存压缩 PDF 失败")?;
    Ok(())
}

/// 收集所有 Image XObject 的 ObjectId。
fn collect_image_xobjects(doc: &Document) -> Vec<ObjectId> {
    let mut image_ids = Vec::new();
    for (&obj_id, obj) in doc.objects.iter() {
        if let Object::Stream(stream) = obj {
            let dict = &stream.dict;
            if let Ok(Object::Name(subtype)) = dict.get(b"Subtype") {
                if subtype == b"Image" {
                    image_ids.push(obj_id);
                }
            }
        }
    }
    image_ids
}

/// 获取 stream 数据大小。
fn stream_data_size(doc: &Document, obj_id: ObjectId) -> usize {
    if let Ok(Object::Stream(stream)) = doc.get_object(obj_id) {
        stream.content.len()
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 有效 DPI 计算
// ---------------------------------------------------------------------------

/// 2D 变换矩阵 [a b c d e f]，对应 PDF 的六位矩阵。
type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// 矩阵复合：先应用 m2，再应用 m1（PDF 的 cm / Form Matrix 均为左乘）。
fn mat_mul(m1: Matrix, m2: Matrix) -> Matrix {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

/// 图片绘制在单位正方形上，CTM 的 x/y 基向量长度即放置尺寸（点）。
/// 返回两轴有效 DPI 的较大值；基向量退化时返回 None。
fn placement_dpi(pixel_w: u32, pixel_h: u32, ctm: &Matrix) -> Option<f64> {
    let place_w = ctm[0].hypot(ctm[1]);
    let place_h = ctm[2].hypot(ctm[3]);
    if place_w <= 0.0 || place_h <= 0.0 {
        return None;
    }
    let dpi_x = pixel_w as f64 / (place_w / 72.0);
    let dpi_y = pixel_h as f64 / (place_h / 72.0);
    Some(dpi_x.max(dpi_y))
}

/// 解析每页内容流，跟踪 q/Q/cm 维护 CTM，统计每张图片被放置时的最大有效 DPI。
/// 解析失败的页/表单直接跳过（保守起见不降采样）。
fn collect_effective_dpi(
    doc: &Document,
    image_ids: &[ObjectId],
    progress: &mut dyn FnMut(CompressProgress),
) -> HashMap<ObjectId, f64> {
    let mut dpi_map: HashMap<ObjectId, f64> = HashMap::new();
    if image_ids.is_empty() {
        return dpi_map;
    }
    let pages = doc.get_pages();
    let page_total = pages.len();
    for (page_index, (_, page_id)) in pages.into_iter().enumerate() {
        progress(CompressProgress::item(
            "分析页面图像",
            page_index + 1,
            page_total,
        ));
        let Ok(content_bytes) = doc.get_page_content(page_id) else {
            continue;
        };
        let Ok(content) = Content::decode(&content_bytes) else {
            continue;
        };
        let resources = doc
            .get_page_resources(page_id)
            .ok()
            .and_then(|(dict, _)| dict);
        scan_content_stream(
            doc,
            &content.operations,
            resources,
            IDENTITY,
            &mut dpi_map,
            0,
        );
    }
    dpi_map
}

/// 扫描一段内容流（页面或 Form XObject），depth 控制 Form 递归层数（仅一层）。
fn scan_content_stream(
    doc: &Document,
    operations: &[lopdf::content::Operation],
    resources: Option<&lopdf::Dictionary>,
    base_ctm: Matrix,
    dpi_map: &mut HashMap<ObjectId, f64>,
    depth: u32,
) {
    let mut ctm = base_ctm;
    let mut stack: Vec<Matrix> = Vec::new();

    for op in operations {
        match op.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or(base_ctm),
            "cm" => {
                if let Some(m) = operands_to_matrix(&op.operands) {
                    ctm = mat_mul(m, ctm);
                }
            }
            "Do" => {
                let Some(name) = op.operands.first().and_then(|o| o.as_name().ok()) else {
                    continue;
                };
                let Some(xobject) = resolve_xobject(doc, resources, name) else {
                    continue;
                };
                let Object::Stream(stream) = xobject else {
                    continue;
                };
                match stream.dict.get(b"Subtype") {
                    Ok(Object::Name(subtype)) if subtype == b"Image" => {
                        let w = dict_u32(&stream.dict, b"Width");
                        let h = dict_u32(&stream.dict, b"Height");
                        if let (Some(w), Some(h)) = (w, h) {
                            if let Some(dpi) = placement_dpi(w, h, &ctm) {
                                if let Ok(obj_id) = xobject_id(doc, resources, name) {
                                    let entry = dpi_map.entry(obj_id).or_insert(dpi);
                                    if dpi > *entry {
                                        *entry = dpi;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Object::Name(subtype)) if subtype == b"Form" && depth == 0 => {
                        recurse_form_xobject(doc, stream, ctm, dpi_map, depth);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// 递归进入 Form XObject 一层：CTM 左乘其 /Matrix，用其自身 /Resources 解析引用。
fn recurse_form_xobject(
    doc: &Document,
    stream: &lopdf::Stream,
    ctm: Matrix,
    dpi_map: &mut HashMap<ObjectId, f64>,
    depth: u32,
) {
    let form_matrix = stream
        .dict
        .get(b"Matrix")
        .ok()
        .and_then(|m| m.as_array().ok())
        .and_then(|arr| operands_to_matrix(arr))
        .unwrap_or(IDENTITY);
    let form_ctm = mat_mul(form_matrix, ctm);
    let form_resources = stream
        .dict
        .get_deref(b"Resources", doc)
        .ok()
        .and_then(|r| r.as_dict().ok());
    let content_bytes = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());
    let Ok(content) = Content::decode(&content_bytes) else {
        return;
    };
    scan_content_stream(
        doc,
        &content.operations,
        form_resources,
        form_ctm,
        dpi_map,
        depth + 1,
    );
}

/// 从资源字典解析 Do 算子引用的 XObject 对象。
fn resolve_xobject<'a>(
    doc: &'a Document,
    resources: Option<&'a lopdf::Dictionary>,
    name: &[u8],
) -> Option<&'a Object> {
    let xobjects = resources?.get_deref(b"XObject", doc).ok()?.as_dict().ok()?;
    xobjects.get_deref(name, doc).ok()
}

/// 取 XObject 的间接引用 ObjectId（内联对象拿不到，返回 Err）。
fn xobject_id(
    doc: &Document,
    resources: Option<&lopdf::Dictionary>,
    name: &[u8],
) -> Result<ObjectId> {
    let xobjects = resources
        .context("缺少资源字典")?
        .get_deref(b"XObject", doc)?
        .as_dict()?;
    xobjects.get(name)?.as_reference().map_err(Into::into)
}

fn operands_to_matrix(operands: &[Object]) -> Option<Matrix> {
    if operands.len() != 6 {
        return None;
    }
    let mut m = [0.0f64; 6];
    for (i, o) in operands.iter().enumerate() {
        m[i] = object_f64(o)?;
    }
    Some(m)
}

fn object_f64(obj: &Object) -> Option<f64> {
    match obj {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}

fn dict_u32(dict: &lopdf::Dictionary, key: &[u8]) -> Option<u32> {
    match dict.get(key).ok()? {
        Object::Integer(i) if *i > 0 => Some(*i as u32),
        Object::Real(r) if *r > 0.0 => Some(*r as u32),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 图片处理策略
// ---------------------------------------------------------------------------

/// 图片处理计划（纯函数判定，便于单测）。
#[derive(Debug, Clone, PartialEq)]
enum ImagePlan {
    /// 保持原样，不触碰
    Skip,
    /// 重编码为 JPEG；downsample_to 为 Some 时先降采样到目标像素尺寸
    ReencodeJpeg {
        quality: u8,
        downsample_to: Option<(u32, u32)>,
    },
}

/// 判定单张图片的处理策略。法律证据文件：宁可不动也不能毁画质。
#[allow(clippy::too_many_arguments)]
fn plan_image_action(
    filter: Option<&[u8]>,
    bpc: u32,
    image_mask: bool,
    has_smask: bool,
    color_ok: bool,
    width: u32,
    height: u32,
    decoded_len: usize,
    max_effective_dpi: Option<f64>,
    options: &CompressOptions,
) -> ImagePlan {
    // 1-bit / 蒙版 / 带透明蒙版 / 非 DeviceRGB/DeviceGray：一律跳过
    if bpc == 1 || image_mask || has_smask || !color_ok {
        return ImagePlan::Skip;
    }
    let is_dct = filter == Some(b"DCTDecode".as_slice());
    let is_flate_or_raw = filter.is_none() || filter == Some(b"FlateDecode".as_slice());
    // CCITTFaxDecode / JPXDecode / LZW 等：保持原样
    if !is_dct && !is_flate_or_raw {
        return ImagePlan::Skip;
    }

    // 仅当有效 DPI 超标才降采样：新像素尺寸 = target_dpi × 放置英寸 = 原像素 × target/实际
    let downsample_to = max_effective_dpi.and_then(|dpi| {
        if options.target_dpi > 0 && dpi > options.target_dpi as f64 {
            let scale = options.target_dpi as f64 / dpi;
            let nw = ((width as f64 * scale).round() as u32).max(1);
            let nh = ((height as f64 * scale).round() as u32).max(1);
            Some((nw, nh))
        } else {
            None
        }
    });

    if is_dct {
        // DCTDecode 且 DPI 不超标：原样保留，绝不转码（避免代际损失）
        return match downsample_to {
            Some(size) => ImagePlan::ReencodeJpeg {
                quality: options.jpeg_quality,
                downsample_to: Some(size),
            },
            None => ImagePlan::Skip,
        };
    }

    // FlateDecode / 无 Filter 的原始大图（> 64KB）才值得转 JPEG
    if decoded_len <= 64 * 1024 {
        return ImagePlan::Skip;
    }
    let quality = if downsample_to.is_some() {
        options.jpeg_quality
    } else {
        options.jpeg_quality.saturating_add(3).min(95)
    };
    ImagePlan::ReencodeJpeg {
        quality,
        downsample_to,
    }
}

/// 按策略处理单张图片。
fn recompress_image(
    doc: &mut Document,
    obj_id: ObjectId,
    options: &CompressOptions,
    max_effective_dpi: Option<f64>,
) -> Result<()> {
    // 先提取需要的信息，避免借用冲突
    let (filter, width, height, is_gray, plain, original_compressed_size);
    let plan;
    {
        let Object::Stream(stream) = doc.get_object(obj_id)? else {
            anyhow::bail!("不是 stream 对象");
        };
        let dict = &stream.dict;

        original_compressed_size = stream.content.len();

        filter = dict.get(b"Filter").ok().and_then(|f| match f {
            Object::Name(n) => Some(n.clone()),
            _ => None,
        });
        width = dict_u32(dict, b"Width").context("缺少 Width")?;
        height = dict_u32(dict, b"Height").context("缺少 Height")?;
        let bpc = match dict.get(b"BitsPerComponent").ok() {
            Some(Object::Integer(i)) => *i as u32,
            _ => 8,
        };
        let image_mask = matches!(dict.get(b"ImageMask"), Ok(Object::Boolean(true)));
        let has_smask = dict.get(b"SMask").is_ok();
        // 仅接受 DeviceRGB / DeviceGray；ICCBased、DeviceCMYK、Indexed 等一律跳过
        let (color_ok, gray) = match dict.get(b"ColorSpace").ok() {
            Some(Object::Name(n)) if n == b"DeviceRGB" => (true, false),
            Some(Object::Name(n)) if n == b"DeviceGray" => (true, true),
            _ => (false, false),
        };
        is_gray = gray;

        // These images can never be safely handled by the JPEG path. Do this
        // metadata-only check before inflating Flate streams; scanned PDFs
        // commonly contain many 1-bit masks and transparent overlays.
        let supported_filter = filter.is_none()
            || filter.as_deref() == Some(b"FlateDecode".as_slice())
            || filter.as_deref() == Some(b"DCTDecode".as_slice());
        if bpc == 1 || image_mask || has_smask || !color_ok || !supported_filter {
            return Ok(());
        }

        // FlateDecode / 无 Filter 需要先解压拿到原始像素数据长度
        let needs_plain = filter.is_none() || filter.as_deref() == Some(b"FlateDecode".as_slice());
        plain = if needs_plain {
            Some(stream.get_plain_content().context("解压 stream 失败")?)
        } else {
            None
        };
        let decoded_len = plain
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(stream.content.len());

        plan = plan_image_action(
            filter.as_deref(),
            bpc,
            image_mask,
            has_smask,
            color_ok,
            width,
            height,
            decoded_len,
            max_effective_dpi,
            options,
        );
    }

    let ImagePlan::ReencodeJpeg {
        quality,
        downsample_to,
    } = plan
    else {
        return Ok(());
    };

    log::info!(
        "处理图片 {:?}: filter={:?} {}x{} gray={} dpi={:?} quality={} downsample={:?}",
        obj_id,
        filter.as_deref().map(|f| String::from_utf8_lossy(f)),
        width,
        height,
        is_gray,
        max_effective_dpi,
        quality,
        downsample_to
    );

    // 解码像素（灰度保持灰度，RGB 保持 RGB）
    let is_raw_pixels = plain.is_some();
    let raw = match plain {
        Some(raw) => raw,
        None => {
            // DCTDecode：取原始 JPEG 码流解码
            let Object::Stream(stream) = doc.get_object(obj_id)? else {
                anyhow::bail!("不是 stream 对象");
            };
            stream.get_plain_content().context("读取 JPEG 码流失败")?
        }
    };
    let pixels = if is_raw_pixels {
        decode_raw_pixels(&raw, width, height, is_gray)?
    } else {
        decode_jpeg_pixels(&raw, is_gray)?
    };

    // 降采样（Lanczos3）
    let (new_width, new_height, new_pixels) = match downsample_to {
        Some((nw, nh)) if nw < width || nh < height => {
            let resized = resize_pixels(&pixels, width, height, nw, nh, is_gray)?;
            (nw, nh, resized)
        }
        _ => (width, height, pixels),
    };

    // 重新编码为 JPEG（灰度写灰度 JPEG）
    let jpeg_data = encode_jpeg(&new_pixels, new_width, new_height, is_gray, quality)?;

    // 只有新数据更小才写回（与原始压缩大小比较）
    if jpeg_data.len() >= original_compressed_size {
        return Ok(());
    }

    // 写回 PDF stream
    {
        let Object::Stream(stream) = doc.get_object_mut(obj_id)? else {
            anyhow::bail!("不是 stream 对象");
        };
        stream.set_plain_content(jpeg_data);
        stream
            .dict
            .set("Filter", Object::Name(b"DCTDecode".to_vec()));
        stream.dict.set("Width", Object::Integer(new_width as i64));
        stream
            .dict
            .set("Height", Object::Integer(new_height as i64));
        stream.dict.set("BitsPerComponent", Object::Integer(8));
        let colorspace: &[u8] = if is_gray { b"DeviceGray" } else { b"DeviceRGB" };
        stream
            .dict
            .set("ColorSpace", Object::Name(colorspace.to_vec()));
        // 移除不适用的字段（不动 SMask——带 SMask 的图片已在策略层跳过）
        let _ = stream.dict.remove(b"DecodeParms");
    }

    Ok(())
}

/// 解码 JPEG 数据，按目标色彩空间输出灰度或 RGB 像素。
fn decode_jpeg_pixels(data: &[u8], gray: bool) -> Result<Vec<u8>> {
    let img = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
        .context("JPEG 解码失败")?;
    if gray {
        Ok(img.to_luma8().into_raw())
    } else {
        Ok(img.to_rgb8().into_raw())
    }
}

/// 解码原始像素数据（FlateDecode 或无过滤器），gray 决定通道数。
fn decode_raw_pixels(data: &[u8], width: u32, height: u32, gray: bool) -> Result<Vec<u8>> {
    let channels = if gray { 1usize } else { 3usize };
    let expected_size = (width as usize) * (height as usize) * channels;

    if data.len() < expected_size {
        anyhow::bail!(
            "像素数据不足：需要 {} 字节，实际 {} 字节",
            expected_size,
            data.len()
        );
    }

    Ok(data[..expected_size].to_vec())
}

/// Lanczos3 高质量缩放。
fn resize_pixels(
    pixels: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    gray: bool,
) -> Result<Vec<u8>> {
    let filter = image::imageops::FilterType::Lanczos3;
    if gray {
        let img = image::GrayImage::from_raw(src_w, src_h, pixels.to_vec())
            .context("灰度像素缓冲区尺寸不符")?;
        Ok(image::imageops::resize(&img, dst_w, dst_h, filter).into_raw())
    } else {
        let img = image::RgbImage::from_raw(src_w, src_h, pixels.to_vec())
            .context("RGB 像素缓冲区尺寸不符")?;
        Ok(image::imageops::resize(&img, dst_w, dst_h, filter).into_raw())
    }
}

/// 编码像素为 JPEG（灰度或 RGB）。
fn encode_jpeg(pixels: &[u8], width: u32, height: u32, gray: bool, quality: u8) -> Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    let color = if gray {
        image::ExtendedColorType::L8
    } else {
        image::ExtendedColorType::Rgb8
    };
    encoder
        .write_image(pixels, width, height, color)
        .context("JPEG 编码失败")?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(level: u8) -> CompressOptions {
        CompressOptions::from_level(level)
    }

    #[test]
    fn test_compress_options_from_level() {
        let l1 = opts(1);
        assert_eq!(l1.jpeg_quality, 92);
        assert_eq!(l1.target_dpi, 300);

        let l2 = opts(2);
        assert_eq!(l2.jpeg_quality, 85);
        assert_eq!(l2.target_dpi, 200);

        let l3 = opts(3);
        assert_eq!(l3.jpeg_quality, 75);
        assert_eq!(l3.target_dpi, 150);

        // 未知档位回退到均衡
        let unknown = opts(99);
        assert_eq!(unknown.jpeg_quality, 85);
        assert_eq!(unknown.target_dpi, 200);
    }

    #[test]
    fn test_mat_mul_composition() {
        // 先缩放 2 倍再平移 (10, 20)：mat_mul(translate, scale)
        let translate: Matrix = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let scale: Matrix = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let m = mat_mul(translate, scale);
        assert_eq!(m, [2.0, 0.0, 0.0, 2.0, 10.0, 20.0]);

        // 单位矩阵不改变结果
        assert_eq!(mat_mul(IDENTITY, scale), scale);
        assert_eq!(mat_mul(scale, IDENTITY), scale);
    }

    #[test]
    fn test_placement_dpi_full_page_scan() {
        // 200dpi A4 扫描整页放置：cm = [612 0 0 792 0 0]，1651×2335 像素
        let ctm: Matrix = [612.0, 0.0, 0.0, 792.0, 0.0, 0.0];
        let dpi = placement_dpi(1651, 2335, &ctm).unwrap();
        // x 轴 ≈ 194dpi，y 轴 ≈ 212dpi，取较大值
        assert!((dpi - 212.3).abs() < 1.0, "dpi = {dpi}");
        // 关键性质：低于 300 目标，不应触发降采样
        assert!(dpi < 300.0);
    }

    #[test]
    fn test_placement_dpi_scaled_with_cm() {
        // q 1 0 0 1 0 0 cm / 2 0 0 2 0 0 cm：放置尺寸翻倍，DPI 减半
        let base: Matrix = [306.0, 0.0, 0.0, 396.0, 0.0, 0.0];
        let double: Matrix = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let ctm = mat_mul(double, base);
        let dpi = placement_dpi(1651, 2335, &ctm).unwrap();
        let dpi_base = placement_dpi(1651, 2335, &base).unwrap();
        assert!((dpi * 2.0 - dpi_base).abs() < 0.01);
    }

    #[test]
    fn test_plan_skips_untouchable_images() {
        let o = opts(1);
        // bpc == 1
        assert_eq!(
            plan_image_action(
                Some(b"DCTDecode"),
                1,
                false,
                false,
                true,
                100,
                100,
                99999,
                None,
                &o
            ),
            ImagePlan::Skip
        );
        // ImageMask
        assert_eq!(
            plan_image_action(None, 8, true, false, true, 100, 100, 99999, None, &o),
            ImagePlan::Skip
        );
        // 带 SMask
        assert_eq!(
            plan_image_action(
                Some(b"DCTDecode"),
                8,
                false,
                true,
                true,
                100,
                100,
                99999,
                None,
                &o
            ),
            ImagePlan::Skip
        );
        // 非 DeviceRGB/DeviceGray（ICCBased 等）
        assert_eq!(
            plan_image_action(
                Some(b"DCTDecode"),
                8,
                false,
                false,
                false,
                100,
                100,
                99999,
                None,
                &o
            ),
            ImagePlan::Skip
        );
        // CCITTFaxDecode / JPXDecode
        assert_eq!(
            plan_image_action(
                Some(b"CCITTFaxDecode"),
                8,
                false,
                false,
                true,
                100,
                100,
                99999,
                None,
                &o
            ),
            ImagePlan::Skip
        );
        assert_eq!(
            plan_image_action(
                Some(b"JPXDecode"),
                8,
                false,
                false,
                true,
                100,
                100,
                99999,
                None,
                &o
            ),
            ImagePlan::Skip
        );
    }

    #[test]
    fn test_plan_dct_never_transcoded_when_dpi_ok() {
        let o = opts(1);
        // DCT + DPI 不超标（或未找到放置）：原样保留
        assert_eq!(
            plan_image_action(
                Some(b"DCTDecode"),
                8,
                false,
                false,
                true,
                1651,
                2335,
                99999,
                Some(200.0),
                &o
            ),
            ImagePlan::Skip
        );
        assert_eq!(
            plan_image_action(
                Some(b"DCTDecode"),
                8,
                false,
                false,
                true,
                1651,
                2335,
                99999,
                None,
                &o
            ),
            ImagePlan::Skip
        );
    }

    #[test]
    fn test_plan_dct_downsample_when_dpi_exceeded() {
        let o = opts(1); // target 300
        let plan = plan_image_action(
            Some(b"DCTDecode"),
            8,
            false,
            false,
            true,
            4000,
            2000,
            99999,
            Some(400.0),
            &o,
        );
        // scale = 300/400 = 0.75 → 3000×1500，质量 92
        assert_eq!(
            plan,
            ImagePlan::ReencodeJpeg {
                quality: 92,
                downsample_to: Some((3000, 1500))
            }
        );
    }

    #[test]
    fn test_plan_flate_to_jpeg() {
        let l1 = opts(1);
        let l2 = opts(2);
        let l3 = opts(3);

        // 小图（<= 64KB）不处理
        assert_eq!(
            plan_image_action(
                Some(b"FlateDecode"),
                8,
                false,
                false,
                true,
                100,
                100,
                64 * 1024,
                None,
                &l1
            ),
            ImagePlan::Skip
        );
        // 大图转 JPEG，不降尺寸时质量 = min(95, q+3)
        assert_eq!(
            plan_image_action(
                Some(b"FlateDecode"),
                8,
                false,
                false,
                true,
                1000,
                1000,
                100 * 1024,
                None,
                &l1
            ),
            ImagePlan::ReencodeJpeg {
                quality: 95,
                downsample_to: None
            }
        );
        assert_eq!(
            plan_image_action(
                None,
                8,
                false,
                false,
                true,
                1000,
                1000,
                100 * 1024,
                None,
                &l2
            ),
            ImagePlan::ReencodeJpeg {
                quality: 88,
                downsample_to: None
            }
        );
        assert_eq!(
            plan_image_action(
                Some(b"FlateDecode"),
                8,
                false,
                false,
                true,
                1000,
                1000,
                100 * 1024,
                None,
                &l3
            ),
            ImagePlan::ReencodeJpeg {
                quality: 78,
                downsample_to: None
            }
        );
        // 大图且 DPI 超标：降采样 + jpeg_quality
        assert_eq!(
            plan_image_action(
                Some(b"FlateDecode"),
                8,
                false,
                false,
                true,
                1000,
                1000,
                100 * 1024,
                Some(600.0),
                &l2
            ),
            ImagePlan::ReencodeJpeg {
                quality: 85,
                downsample_to: Some((333, 333))
            }
        );
    }

    #[test]
    #[ignore = "requires local evidence PDFs; run explicitly for manual compression QA"]
    fn test_evidence_fixtures() {
        let base =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-pdf/4W122724 I D1-D14");
        let evidence3 = base.join("03 证据 3. 中国发明专利申请公开文本 CN105829563A.pdf");
        let notice =
            base.join("Notification of Acceptance of Request for Invalidation-4W122724.pdf");
        if !evidence3.exists() || !notice.exists() {
            eprintln!("测试文件不存在，跳过");
            return;
        }

        // 1) 无损优化：38MB 中 90% 是不可达垃圾对象，应被剥离到 10MB 以内，页数不变
        let evidence3_str = evidence3.to_string_lossy().to_string();
        let input_pages = crate::pdf::qpdf::page_count(&evidence3_str).unwrap();
        let result = crate::pdf::qpdf::optimize_lossless_to_dir(&evidence3_str, None).unwrap();
        assert!(result.changed, "优化应产生更小输出");
        assert!(
            result.output_size < 10 * 1024 * 1024,
            "优化后仍超过 10MB: {} bytes",
            result.output_size
        );
        let output_pages = crate::pdf::qpdf::page_count(&result.output_path).unwrap();
        assert_eq!(input_pages, output_pages, "页数不应变化");
        let _ = std::fs::remove_file(&result.output_path);

        // 2) 受理通知书前 18 页拆分件跑 level 1：有效 DPI ≈ 200 < 300，不得降采样
        let tmp = std::env::temp_dir().join(format!("docsy-fixture-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let pages: Vec<u32> = (1..=18).collect();
        let split = crate::pdf::qpdf::extract_pages(
            &notice.to_string_lossy(),
            &pages,
            Some(&tmp.to_string_lossy()),
        )
        .unwrap();
        let split_path = std::path::PathBuf::from(&split.output_path);
        let output = tmp.join("compressed-l1.pdf");
        compress_pdf_with_progress(&split_path, &output, &opts(1), &mut |_| {}).unwrap();

        let doc = Document::load(&output).unwrap();
        let mut max_width = 0u32;
        for (_, obj) in doc.objects.iter() {
            if let Object::Stream(stream) = obj {
                if let Ok(Object::Name(subtype)) = stream.dict.get(b"Subtype") {
                    if subtype == b"Image" {
                        if let Some(w) = dict_u32(&stream.dict, b"Width") {
                            max_width = max_width.max(w);
                        }
                    }
                }
            }
        }
        assert!(
            max_width >= 1651,
            "整页扫描图不应被降采样：输出最大图宽 = {max_width}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
