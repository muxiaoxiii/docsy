use anyhow::{Context, Result};
use image::ImageEncoder;
use lopdf::{Document, Object, ObjectId};
use std::path::Path;

/// 压缩选项
pub struct CompressOptions {
    /// JPEG 质量 1-100
    pub image_quality: u8,
    /// 最大 DPI，超过则缩放（0 = 不缩放）
    pub max_dpi: u32,
}

impl CompressOptions {
    pub fn from_level(level: u8) -> Self {
        match level {
            1 => Self {
                image_quality: 85,
                max_dpi: 300,
            },
            3 => Self {
                image_quality: 40,
                max_dpi: 96,
            },
            _ => Self {
                image_quality: 65,
                max_dpi: 150,
            },
        }
    }
}

/// 对 PDF 中的图片进行重编码压缩。
///
/// 流程：lopdf 读取 → 遍历 Image XObject → 解码 → 降采样 → JPEG 重编码 → 写回 → 保存。
pub fn compress_pdf(input: &Path, output: &Path, options: &CompressOptions) -> Result<()> {
    let mut doc = Document::load(input).context("读取 PDF 失败")?;

    let image_ids = collect_image_xobjects(&doc);
    let mut compressed_count = 0u32;
    let mut saved_bytes: u64 = 0;

    for obj_id in image_ids {
        let original_size = stream_data_size(&doc, obj_id);
        if let Err(e) = recompress_image(&mut doc, obj_id, options) {
            // 跳过无法处理的图片，不中断整个压缩流程
            log::warn!("跳过图片 {:?}: {} ({}KB)", obj_id, e, original_size / 1024);
            continue;
        }
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

    log::info!(
        "图片压缩完成：{} 张图片被重编码，节省 {:.1} MB",
        compressed_count,
        saved_bytes as f64 / 1024.0 / 1024.0
    );

    doc.compress();
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

/// 尝试重编码单张图片。
fn recompress_image(doc: &mut Document, obj_id: ObjectId, options: &CompressOptions) -> Result<()> {
    // 先提取需要的信息，避免借用冲突
    let (filter, width, height, bpc, content, colorspace, original_compressed_size);
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
        width = dict
            .get(b"Width")
            .ok()
            .and_then(|w| match w {
                Object::Integer(i) => Some(*i as u32),
                Object::Real(r) => Some(*r as u32),
                _ => None,
            })
            .context("缺少 Width")?;
        height = dict
            .get(b"Height")
            .ok()
            .and_then(|h| match h {
                Object::Integer(i) => Some(*i as u32),
                Object::Real(r) => Some(*r as u32),
                _ => None,
            })
            .context("缺少 Height")?;
        bpc = dict
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|b| match b {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .unwrap_or(8);
        colorspace = dict.get(b"ColorSpace").ok().cloned();

        content = stream.get_plain_content().context("解压 stream 失败")?;
    }

    let filter_str = filter
        .as_deref()
        .map(|f| std::str::from_utf8(f).unwrap_or(""));

    log::info!(
        "处理图片 {:?}: filter={:?} {}x{} bpc={} content={}KB compressed={}KB",
        obj_id,
        filter_str,
        width,
        height,
        bpc,
        content.len() / 1024,
        original_compressed_size / 1024
    );
    eprintln!(
        "处理图片 {:?}: filter={:?} {}x{} bpc={} content={}KB compressed={}KB",
        obj_id,
        filter_str,
        width,
        height,
        bpc,
        content.len() / 1024,
        original_compressed_size / 1024
    );

    // 1-bit 图片（CCITTFaxDecode）保持不变，转 JPEG 反而更大
    if bpc == 1 {
        return Ok(());
    }

    // 已经很小的图片（< 50KB）不处理
    if content.len() < 50 * 1024 {
        return Ok(());
    }

    // 尝试解码图片
    let decoded_pixels = match filter_str {
        Some("DCTDecode") => decode_jpeg(&content)?,
        Some("FlateDecode") | None => decode_raw_pixels(&content, width, height, bpc, &colorspace)?,
        _ => {
            anyhow::bail!("不支持的过滤器: {:?}", filter_str);
        }
    };

    let Some(pixels) = decoded_pixels else {
        return Ok(());
    };

    // 计算缩放
    let (new_width, new_height, new_pixels) =
        if options.max_dpi > 0 && (width > options.max_dpi * 2 || height > options.max_dpi * 2) {
            let scale = (options.max_dpi as f64 * 2.0) / (width.max(height) as f64);
            if scale < 1.0 {
                let nw = (width as f64 * scale).round().max(1.0) as u32;
                let nh = (height as f64 * scale).round().max(1.0) as u32;
                let resized = resize_image(&pixels, width, height, nw, nh)?;
                (nw, nh, resized)
            } else {
                (width, height, pixels)
            }
        } else {
            (width, height, pixels)
        };

    // 重新编码为 JPEG
    let jpeg_data = encode_jpeg(&new_pixels, new_width, new_height, options.image_quality)?;

    // 如果 JPEG 更大，跳过（与原始压缩大小比较）
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
        stream
            .dict
            .set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
        // 移除不适用的字段
        let _ = stream.dict.remove(b"DecodeParms");
        let _ = stream.dict.remove(b"SMask");
    }

    Ok(())
}

/// 解码 JPEG 数据为 RGB 像素。
fn decode_jpeg(data: &[u8]) -> Result<Option<Vec<u8>>> {
    use image::ImageDecoder;
    let decoder = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(data))
        .context("JPEG 解码失败")?;
    let color = decoder.color_type();

    let mut buf = vec![0u8; decoder.total_bytes() as usize];
    decoder.read_image(&mut buf).context("JPEG 读取像素失败")?;

    // 转换为 RGB
    let rgb = match color {
        image::ColorType::Rgb8 => buf,
        image::ColorType::Rgba8 => rgba_to_rgb(&buf),
        image::ColorType::L8 => gray_to_rgb(&buf),
        image::ColorType::La8 => la_to_rgb(&buf),
        _ => anyhow::bail!("不支持的 JPEG 颜色格式: {:?}", color),
    };

    Ok(Some(rgb))
}

/// 解码原始像素数据（FlateDecode 或无过滤器）。
fn decode_raw_pixels(
    data: &[u8],
    width: u32,
    height: u32,
    bpc: u32,
    colorspace: &Option<Object>,
) -> Result<Option<Vec<u8>>> {
    if bpc != 8 {
        anyhow::bail!("不支持的 BitsPerComponent: {}", bpc);
    }

    let channels = detect_channels(colorspace);
    let expected_size = (width as usize) * (height as usize) * (channels as usize);

    if data.len() < expected_size {
        anyhow::bail!(
            "像素数据不足：需要 {} 字节，实际 {} 字节",
            expected_size,
            data.len()
        );
    }

    let pixels = &data[..expected_size];

    let rgb = match channels {
        3 => pixels.to_vec(),
        4 => rgba_to_rgb(pixels),
        1 => gray_to_rgb(pixels),
        _ => anyhow::bail!("不支持的通道数: {}", channels),
    };

    Ok(Some(rgb))
}

/// 检测颜色空间的通道数。
fn detect_channels(colorspace: &Option<Object>) -> u32 {
    match colorspace {
        Some(Object::Name(name)) => match name.as_slice() {
            b"DeviceRGB" => 3,
            b"DeviceCMYK" => 4,
            b"DeviceGray" => 1,
            _ => 3,
        },
        Some(Object::Array(arr)) => {
            // [ICCBased N] 等
            if let Some(Object::Name(name)) = arr.first() {
                match name.as_slice() {
                    b"ICCBased" => {
                        // 尝试从 ICC profile 获取 N
                        if let Some(Object::Integer(n)) = arr.get(1) {
                            *n as u32
                        } else {
                            3
                        }
                    }
                    b"DeviceN" | b"Separation" => 4,
                    _ => 3,
                }
            } else {
                3
            }
        }
        _ => 3,
    }
}

/// RGBA → RGB
fn rgba_to_rgb(data: &[u8]) -> Vec<u8> {
    data.chunks(4).flat_map(|c| [c[0], c[1], c[2]]).collect()
}

/// Gray → RGB
fn gray_to_rgb(data: &[u8]) -> Vec<u8> {
    data.iter().flat_map(|&g| [g, g, g]).collect()
}

/// LA → RGB
fn la_to_rgb(data: &[u8]) -> Vec<u8> {
    data.chunks(2).flat_map(|c| [c[0], c[0], c[0]]).collect()
}

/// 缩放图片（简单最近邻，速度快）。
fn resize_image(pixels: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Result<Vec<u8>> {
    let mut out = vec![0u8; (dst_w * dst_h * 3) as usize];
    for y in 0..dst_h {
        let src_y = (y as f64 * src_h as f64 / dst_h as f64).min((src_h - 1) as f64) as u32;
        for x in 0..dst_w {
            let src_x = (x as f64 * src_w as f64 / dst_w as f64).min((src_w - 1) as f64) as u32;
            let src_idx = ((src_y * src_w + src_x) * 3) as usize;
            let dst_idx = ((y * dst_w + x) * 3) as usize;
            if src_idx + 2 < pixels.len() && dst_idx + 2 < out.len() {
                out[dst_idx] = pixels[src_idx];
                out[dst_idx + 1] = pixels[src_idx + 1];
                out[dst_idx + 2] = pixels[src_idx + 2];
            }
        }
    }
    Ok(out)
}

/// 编码 RGB 像素为 JPEG。
fn encode_jpeg(pixels: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .write_image(pixels, width, height, image::ExtendedColorType::Rgb8)
        .context("JPEG 编码失败")?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_options_from_level() {
        let low = CompressOptions::from_level(1);
        assert_eq!(low.image_quality, 85);
        assert_eq!(low.max_dpi, 300);

        let mid = CompressOptions::from_level(2);
        assert_eq!(mid.image_quality, 65);
        assert_eq!(mid.max_dpi, 150);

        let high = CompressOptions::from_level(3);
        assert_eq!(high.image_quality, 40);
        assert_eq!(high.max_dpi, 96);
    }

    #[test]
    #[ignore = "requires a large local evidence PDF; run explicitly for manual compression QA"]
    fn test_compress_real_pdf() {
        let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test-pdf/4W122724 I D1-D14/00_意见陈述正文.pdf");
        if !input.exists() {
            eprintln!("测试文件不存在，跳过");
            return;
        }

        let input_size = std::fs::metadata(&input).unwrap().len();
        let output = std::env::temp_dir().join("docsy_compress_test.pdf");
        let options = CompressOptions::from_level(2);

        let result = compress_pdf(&input, &output, &options);
        if let Err(e) = &result {
            eprintln!("压缩出错: {:#}", e);
        } else {
            let output_size = std::fs::metadata(&output).unwrap().len();
            eprintln!(
                "压缩前: {:.1} MB, 压缩后: {:.1} MB, 压缩率: {:.1}%",
                input_size as f64 / 1024.0 / 1024.0,
                output_size as f64 / 1024.0 / 1024.0,
                (1.0 - output_size as f64 / input_size as f64) * 100.0
            );
        }
        // 清理
        let _ = std::fs::remove_file(&output);
    }
}
