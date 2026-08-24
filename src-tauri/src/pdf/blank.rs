//! 空白页 PDF 生成：用于"双面打印分隔模式"。
//!
//! 双面打印时，若某份 PDF 页数为奇数，它的最后一页会与下一份文件的第一页
//! 落在同一张纸的正反面。该模式在奇数页文件的末尾补一页空白，使每份文件
//! 都占满整张纸，文件之间自然分隔。
//!
//! 空白页的页面尺寸取自源文件最后一页（含旋转），保证双面打印时纸张方向一致。

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

use super::page_info::PageSize;
use crate::util::fs::TempPathGuard;

/// 生成一张与 `size` 同尺寸的空白页 PDF，返回临时文件守卫。
///
/// 守卫持有临时文件路径（`guard.path()`），drop 时自动删除文件。
/// 调用方在 qpdf 合并命令运行期间持有守卫即可。
pub fn write_blank_page(size: &PageSize) -> Result<TempPathGuard> {
    let path = crate::util::fs::temp_named_path("docsy-blank", "pdf");
    write_blank_pdf(&path, size)?;
    Ok(TempPathGuard::new(path))
}

/// 写入一份最小化的单页空白 PDF。
///
/// 只包含 Catalog / Pages / Page 三个对象，页面无内容流，MediaBox 与
/// Rotate 复制自源页。qpdf 可直接作为 `--pages` 输入读取。
fn write_blank_pdf(path: &Path, size: &PageSize) -> Result<()> {
    let x0 = size.box_x0;
    let y0 = size.box_y0;
    let x1 = x0 + size.raw_width_pt;
    let y1 = y0 + size.raw_height_pt;
    let rotate = if size.rotate != 0 {
        format!(" /Rotate {}", size.rotate)
    } else {
        String::new()
    };

    let page_dict = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [{x0} {y0} {x1} {y1}] /Resources << >>{rotate} >>"
    );
    let objects: [&[u8]; 3] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page_dict.as_bytes(),
    ];

    let mut buf: Vec<u8> = Vec::with_capacity(512);
    buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    let mut offsets = vec![0u64];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(buf.len() as u64);
        let _ = writeln!(buf, "{} 0 obj", index + 1);
        buf.extend_from_slice(object);
        buf.extend_from_slice(b"\nendobj\n");
    }

    let xref_position = buf.len() as u64;
    let object_count = objects.len() + 1;
    let _ = writeln!(buf, "xref\n0 {object_count}");
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets[1..] {
        let _ = writeln!(buf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        buf,
        "trailer\n<< /Size {object_count} /Root 1 0 R >>\nstartxref\n{xref_position}\n%%EOF\n"
    );

    std::fs::write(path, buf)
        .with_context(|| format!("写入空白页临时文件失败：{}", path.display()))?;
    crate::util::fs::set_private_permissions(path)
        .with_context(|| format!("设置空白页临时文件权限失败：{}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Document;

    fn size(width: f32, height: f32) -> PageSize {
        PageSize {
            width_pt: width,
            height_pt: height,
            raw_width_pt: width,
            raw_height_pt: height,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 0,
        }
    }

    #[test]
    fn writes_blank_pdf_with_matching_media_box() {
        let guard = write_blank_page(&size(595.28, 841.89)).expect("blank page should be written");

        let doc = Document::load(guard.path()).expect("generated PDF should parse");
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 1, "blank PDF must contain exactly one page");
        let (_, page_id) = pages.into_iter().next().unwrap();

        let page = doc.get_dictionary(page_id).expect("page dict should exist");
        let media_box = page
            .get(b"MediaBox")
            .expect("page should have MediaBox")
            .as_array()
            .expect("MediaBox should be an array");
        let coords: Vec<f32> = media_box
            .iter()
            .map(|value| value.as_float().ok().unwrap_or(0.0))
            .collect();
        assert_eq!(&coords[..], &[0.0, 0.0, 595.28, 841.89]);
    }

    #[test]
    fn blank_page_preserves_rotation() {
        let guard = write_blank_page(&PageSize {
            width_pt: 841.89,
            height_pt: 595.28,
            raw_width_pt: 595.28,
            raw_height_pt: 841.89,
            box_x0: 0.0,
            box_y0: 0.0,
            rotate: 90,
        })
        .expect("blank page should be written");

        let doc = Document::load(guard.path()).expect("generated PDF should parse");
        let (_, page_id) = doc.get_pages().into_iter().next().unwrap();
        let page = doc.get_dictionary(page_id).expect("page dict should exist");
        assert_eq!(
            page.get(b"Rotate")
                .ok()
                .and_then(|value| value.as_i64().ok())
                .expect("rotated blank page should carry Rotate"),
            90
        );
        let media_box = page
            .get(b"MediaBox")
            .expect("page should have MediaBox")
            .as_array()
            .expect("MediaBox should be an array");
        let coords: Vec<f32> = media_box
            .iter()
            .map(|value| value.as_float().ok().unwrap_or(0.0))
            .collect();
        // 旋转前原始框尺寸保留在 MediaBox，视觉尺寸由 Rotate 表达。
        assert_eq!(&coords[..], &[0.0, 0.0, 595.28, 841.89]);
    }

    #[test]
    fn blank_page_preserves_non_zero_box_origin() {
        let guard = write_blank_page(&PageSize {
            width_pt: 595.28,
            height_pt: 841.89,
            raw_width_pt: 595.28,
            raw_height_pt: 841.89,
            box_x0: 18.0,
            box_y0: 24.0,
            rotate: 0,
        })
        .expect("blank page should be written");

        let doc = Document::load(guard.path()).expect("generated PDF should parse");
        let (_, page_id) = doc.get_pages().into_iter().next().unwrap();
        let page = doc.get_dictionary(page_id).expect("page dict should exist");
        let media_box = page
            .get(b"MediaBox")
            .expect("page should have MediaBox")
            .as_array()
            .expect("MediaBox should be an array");
        let coords: Vec<f32> = media_box
            .iter()
            .map(|value| value.as_float().ok().unwrap_or(0.0))
            .collect();
        assert_eq!(&coords[..], &[18.0, 24.0, 613.28, 865.89]);
    }
}
