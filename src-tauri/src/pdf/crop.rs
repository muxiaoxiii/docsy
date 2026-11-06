//! Crop visible page bounds without rasterizing or modifying the original.
use anyhow::{bail, Context, Result};
use lopdf::{Document, Object};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropArgs {
    input_path: String,
    output_dir: Option<String>,
    pages: Vec<u32>,
    // Normalized bounds in the displayed page, origin at top left.
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

fn raw_bounds(size: &super::page_info::PageSize, a: &CropArgs) -> Result<[f32; 4]> {
    let vals = [a.left, a.top, a.right, a.bottom];
    if vals.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
        || a.right - a.left < 0.001
        || a.bottom - a.top < 0.001
    {
        bail!("裁剪区域无效");
    }
    let map = |x: f32, y: f32| match size.rotate.rem_euclid(360) {
        0 => (x, 1.0 - y),
        90 => (y, x),
        180 => (1.0 - x, y),
        270 => (1.0 - y, 1.0 - x),
        _ => (x, 1.0 - y),
    };
    if size.rotate.rem_euclid(90) != 0 {
        bail!("不支持此页面的旋转角度");
    }
    let points = [
        map(a.left, a.top),
        map(a.right, a.top),
        map(a.left, a.bottom),
        map(a.right, a.bottom),
    ];
    let xmin = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let xmax = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let ymin = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let ymax = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    Ok([
        size.box_x0 + xmin * size.raw_width_pt,
        size.box_y0 + ymin * size.raw_height_pt,
        size.box_x0 + xmax * size.raw_width_pt,
        size.box_y0 + ymax * size.raw_height_pt,
    ])
}

pub fn crop_pdf_pages(args: &CropArgs) -> Result<String> {
    let input = Path::new(&args.input_path);
    let sizes = super::page_info::get_page_infos(&args.input_path)?;
    if args.pages.is_empty()
        || args
            .pages
            .iter()
            .any(|p| *p == 0 || *p as usize > sizes.len())
    {
        bail!("请选择有效的页面范围");
    }
    let mut doc = Document::load(input).context("无法打开 PDF；加密文件请先解锁")?;
    if doc.is_encrypted() {
        bail!("加密文件请先解锁");
    }
    let pages = doc.get_pages();
    for page in &args.pages {
        crate::operations::check_current_cancelled()?;
        let bounds = raw_bounds(&sizes[*page as usize - 1], args)?;
        let page_id = *pages.get(page).context("页面不存在")?;
        let dict = doc.get_object_mut(page_id)?.as_dict_mut()?;
        dict.set(
            "CropBox",
            Object::Array(bounds.iter().map(|v| Object::Real(*v)).collect()),
        );
    }
    let parent = args
        .output_dir
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(Path::new)
        .or_else(|| input.parent())
        .context("无法确定输出目录")?;
    let stem = format!(
        "{}_cropped",
        super::safe_file_stem(
            input
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("document")
        )
    );
    let temp = super::temp_named_path("docsy_crop", "pdf");
    let result = (|| -> Result<String> {
        doc.save(&temp).context("保存裁剪 PDF 失败")?;
        let path = crate::util::fs::copy_unique_with_stem(&temp, parent, &stem, "pdf")?;
        Ok(path.to_string_lossy().to_string())
    })();
    let _ = std::fs::remove_file(temp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Document};

    #[test]
    fn crop_preserves_source_and_maps_rotated_visible_page() {
        let dir = crate::util::fs::temp_named_path("docsy_crop_test", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let input = dir.join("source.pdf");
        let mut doc = Document::with_version("1.5");
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![10.into(), 20.into(), 410.into(), 220.into()], "Rotate" => 90 });
        doc.objects.insert(
            pages,
            dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into(),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        doc.save(&input).unwrap();
        let original = std::fs::read(&input).unwrap();
        let args = CropArgs {
            input_path: input.to_string_lossy().into(),
            output_dir: None,
            pages: vec![1],
            left: 0.25,
            top: 0.1,
            right: 0.75,
            bottom: 0.6,
        };
        let output = crop_pdf_pages(&args).unwrap();
        assert_eq!(std::fs::read(&input).unwrap(), original);
        let result = Document::load(output).unwrap();
        let id = *result.get_pages().get(&1).unwrap();
        let bounds = result
            .get_object(id)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"CropBox")
            .unwrap()
            .as_array()
            .unwrap();
        let values: Vec<f32> = bounds.iter().map(|o| o.as_float().unwrap()).collect();
        for (actual, expected) in values.iter().zip([50.0, 70.0, 250.0, 170.0]) {
            assert!((actual - expected).abs() < 0.001);
        }
    }

    #[test]
    #[ignore = "manual crop comparison using a real PDF"]
    fn manual_crop_real_pdf() {
        let args = CropArgs {
            input_path: std::env::var("DOCSY_TEST_CROP_INPUT").unwrap(),
            output_dir: Some(std::env::var("DOCSY_TEST_CROP_OUTPUT").unwrap()),
            pages: vec![1],
            left: 0.1,
            top: 0.1,
            right: 0.9,
            bottom: 0.9,
        };
        println!("{}", crop_pdf_pages(&args).unwrap());
    }
}
