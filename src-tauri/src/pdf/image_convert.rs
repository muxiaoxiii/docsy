//! A4 image import: the preview and PDF use the same immutable placement plan.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageEdit {
    pub path: String,
    #[serde(default)]
    pub reference_width: Option<f64>,
    #[serde(default)]
    pub rotation: u16,
    #[serde(default = "full_crop")]
    pub crop: [f64; 4], // left, top, right, bottom fractions
    #[serde(default = "full_scale")]
    pub scale: f64,
}
fn full_crop() -> [f64; 4] {
    [0.0, 0.0, 1.0, 1.0]
}
fn full_scale() -> f64 {
    1.0
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImagePageLayout {
    pub count: usize,
    pub rows: usize,
    pub cols: usize,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImagePdfArgs {
    pub images: Vec<ImageEdit>,
    #[serde(default)]
    pub page_layouts: Vec<ImagePageLayout>,
    pub landscape: bool,
    pub rows: usize,
    pub cols: usize,
    pub margin: f64,
    pub gap: f64,
    #[serde(default)]
    pub output_dir: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct ImagePlacement {
    pub index: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub crop_px: [u32; 4],
    pub source_width: u32,
    pub source_height: u32,
    pub rotation: u16,
    pub effective_dpi: f64,
    pub reference_width: f64,
    pub max_scale: f64,
    pub fits: bool,
}
#[derive(Debug, Serialize)]
pub struct ImagePdfPlan {
    pub width: f64,
    pub height: f64,
    pub pages: Vec<Vec<ImagePlacement>>,
}
fn source_dimensions(path: &Path) -> Result<(u32, u32)> {
    use std::io::BufReader;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["jpg", "jpeg", "png", "webp", "bmp"].contains(&ext.as_str()) {
        bail!("请选择 JPG、PNG、WebP 或 BMP 静态图片");
    }
    if ext == "webp"
        && image::codecs::webp::WebPDecoder::new(BufReader::new(std::fs::File::open(path)?))?
            .has_animation()
    {
        bail!("动态 WebP 请先导出为静态图片，避免遗漏动画帧");
    }
    if ext == "png"
        && image::codecs::png::PngDecoder::new(BufReader::new(std::fs::File::open(path)?))?
            .is_apng()?
    {
        bail!("动态 PNG 请先导出为静态图片，避免遗漏动画帧");
    }
    let dims = crate::util::images::oriented_dimensions(path)
        .with_context(|| format!("无法读取图片：{}", path.display()))?;
    if dims.0 == 0 || dims.1 == 0 || u64::from(dims.0) * u64::from(dims.1) > 64_000_000 {
        bail!("图片像素过大或无效，单张最多支持 6400 万像素");
    }
    Ok(dims)
}
pub fn plan(args: &ImagePdfArgs) -> Result<ImagePdfPlan> {
    if args.images.is_empty() || args.images.len() > 500 {
        bail!("请选择 1～500 张图片");
    }
    if !(1..=4).contains(&args.rows)
        || !(1..=4).contains(&args.cols)
        || !args.margin.is_finite()
        || !(0.0..=40.0).contains(&args.margin)
        || !args.gap.is_finite()
        || !(0.0..=20.0).contains(&args.gap)
    {
        bail!("纸张边距或排列参数无效");
    }
    let (width, height) = if args.landscape {
        (297.0, 210.0)
    } else {
        (210.0, 297.0)
    };
    let capacity = args.rows * args.cols;
    let mut specs = Vec::new();
    if args.page_layouts.is_empty() {
        for start in (0..args.images.len()).step_by(capacity) {
            specs.push((
                start,
                (start + capacity).min(args.images.len()),
                args.rows,
                args.cols,
            ));
        }
    } else {
        let mut start = 0;
        for page in &args.page_layouts {
            if !(1..=4).contains(&page.rows)
                || !(1..=4).contains(&page.cols)
                || page.count == 0
                || page.count > 16
            {
                bail!("图片排列参数无效，每页支持 1～16 张图片");
            }
            // Grid dimensions describe arrangement, not a reason to discard or split images.
            // The UI normally adjusts both controls together; tolerate older/in-flight requests
            // using the same safe row-first expansion, and keep the requested physical scale.
            let rows = page.rows.max(page.count.div_ceil(page.cols).min(4));
            let cols = page.cols.max(page.count.div_ceil(rows));
            specs.push((start, start + page.count, rows, cols));
            start += page.count;
        }
        if start != args.images.len() {
            bail!("图片分页数量与列表不一致，请重新预览");
        }
    }
    let mut pages = Vec::new();
    let mut page_index = 0;
    for (index, edit) in args.images.iter().enumerate() {
        crate::operations::check_current_cancelled()?;
        while index >= specs[page_index].1 {
            page_index += 1;
        }
        let (page_start, _, rows, cols) = specs[page_index];
        let cw = (width - args.margin * 2.0 - args.gap * (cols - 1) as f64) / cols as f64;
        let ch = (height - args.margin * 2.0 - args.gap * (rows - 1) as f64) / rows as f64;
        if cw < 1.0 || ch < 1.0 {
            bail!("边距和间距过大，纸张已放不下图片");
        }
        crate::util::images::validate_rotation(edit.rotation)?;
        if !edit.scale.is_finite()
            || !(0.01..=4.0).contains(&edit.scale)
            || edit
                .crop
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        {
            bail!("图片缩放或裁剪参数无效");
        }
        let (sw, sh) = source_dimensions(Path::new(&edit.path))?;
        let [left, top, right, bottom] = edit.crop;
        if right <= left || bottom <= top {
            bail!("裁剪区域不能为空");
        }
        let x = (left * sw as f64).round() as u32;
        let y = (top * sh as f64).round() as u32;
        let x2 = (right * sw as f64).round().min(sw as f64) as u32;
        let y2 = (bottom * sh as f64).round().min(sh as f64) as u32;
        if x2 <= x || y2 <= y {
            bail!("裁剪区域不足一个像素");
        }
        let crop = [x, y, x2 - x, y2 - y];
        let (pw, ph) = if edit.rotation % 180 == 90 {
            (crop[3], crop[2])
        } else {
            (crop[2], crop[3])
        };
        let reference_width = edit
            .reference_width
            .unwrap_or_else(|| (cw / sw as f64).min(ch / sh as f64) * sw as f64);
        if !reference_width.is_finite() || !(0.01..=500.0).contains(&reference_width) {
            bail!("图片打印尺寸基准无效");
        }
        let base_ratio = reference_width / sw as f64;
        let max_scale = (cw / (pw as f64 * base_ratio)).min(ch / (ph as f64 * base_ratio));
        let ratio = base_ratio * edit.scale;
        let (dw, dh) = (pw as f64 * ratio, ph as f64 * ratio);
        let slot = index - page_start;
        if slot == 0 {
            pages.push(Vec::new());
        }
        pages.last_mut().unwrap().push(ImagePlacement {
            index,
            x: args.margin + (slot % cols) as f64 * (cw + args.gap) + (cw - dw) / 2.0,
            y: args.margin + (slot / cols) as f64 * (ch + args.gap) + (ch - dh) / 2.0,
            width: dw,
            height: dh,
            crop_px: crop,
            source_width: sw,
            source_height: sh,
            rotation: edit.rotation,
            effective_dpi: pw as f64 / dw * 25.4,
            reference_width,
            max_scale,
            fits: edit.scale <= max_scale + 1e-8,
        });
    }
    Ok(ImagePdfPlan {
        width,
        height,
        pages,
    })
}
pub fn export(args: &ImagePdfArgs) -> Result<String> {
    use printpdf::*;
    let plan = plan(args)?;
    if plan.pages.iter().flatten().any(|p| !p.fits) {
        bail!("图片超出当前格子，请先调整缩放或适应格子");
    }
    let mut doc = PdfDocument::new("Docsy image import");
    for placements in &plan.pages {
        let mut ops = Vec::new();
        for placement in placements {
            crate::operations::check_current_cancelled()?;
            let edit = &args.images[placement.index];
            let source = crate::util::images::open_oriented(Path::new(&edit.path))?;
            // A source changed after planning must never silently change the output geometry.
            if (source.width(), source.height())
                != (placement.source_width, placement.source_height)
            {
                bail!("图片尺寸已变化，请重新打开预览");
            }
            let [x, y, w, h] = placement.crop_px;
            let cropped = source.crop_imm(x, y, w, h);
            let image = crate::util::images::rotate(cropped, placement.rotation)?;
            let pixels = image.width();
            let raw = RawImage::from_dynamic_image(image).map_err(|e| anyhow::anyhow!(e))?;
            let id = doc.add_image(&raw);
            let scale = (placement.width * 72.0 / 25.4) / (pixels as f64 * 72.0 / 300.0);
            ops.push(Op::SaveGraphicsState);
            ops.push(Op::UseXobject {
                id,
                transform: XObjectTransform {
                    translate_x: Some(Pt((placement.x * 72.0 / 25.4) as f32)),
                    translate_y: Some(Pt(
                        ((plan.height - placement.y - placement.height) * 72.0 / 25.4) as f32
                    )),
                    scale_x: Some(scale as f32),
                    scale_y: Some(scale as f32),
                    dpi: Some(300.0),
                    ..Default::default()
                },
            });
            ops.push(Op::RestoreGraphicsState);
        }
        doc.pages.push(PdfPage::new(
            Mm(plan.width as f32),
            Mm(plan.height as f32),
            ops,
        ));
    }
    // Keep original cropped pixels; no resampling or additional lossy compression.
    let bytes = doc.save(
        &PdfSaveOptions {
            image_optimization: None,
            ..Default::default()
        },
        &mut Vec::new(),
    );
    let mut pdf = lopdf::Document::load_mem(&bytes)?;
    for (_, page) in pdf.get_pages() {
        pdf.get_object_mut(page)?.as_dict_mut()?.set(
            "MediaBox",
            vec![
                0.0.into(),
                0.0.into(),
                lopdf::Object::Real((plan.width * 72.0 / 25.4) as f32),
                lopdf::Object::Real((plan.height * 72.0 / 25.4) as f32),
            ],
        );
    }
    let source = Path::new(&args.images[0].path);
    let parent = args
        .output_dir
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| source.parent().unwrap_or_else(|| Path::new(".")));
    std::fs::create_dir_all(parent)?;
    let stem = crate::util::fs::safe_file_stem(
        source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("images"),
    );
    let temp_dir = crate::util::fs::temp_named_path("docsy-image-import", "dir");
    std::fs::create_dir_all(&temp_dir)?;
    let _guard = crate::util::fs::TempDirGuard::new(temp_dir.clone())?;
    let temp = temp_dir.join("images.pdf");
    pdf.save(&temp)?;
    crate::operations::check_current_cancelled()?;
    let output =
        crate::util::fs::copy_unique_with_stem(&temp, parent, &format!("{stem}_A4"), "pdf")?;
    Ok(output.to_string_lossy().into_owned())
}
pub fn convert_image_to_pdf(input: &str) -> Result<String> {
    let (w, h) = source_dimensions(Path::new(input))?;
    export(&ImagePdfArgs {
        page_layouts: Vec::new(),
        images: vec![ImageEdit {
            reference_width: None,
            path: input.into(),
            rotation: 0,
            crop: full_crop(),
            scale: 1.0,
        }],
        landscape: w > h,
        rows: 1,
        cols: 1,
        margin: 10.0,
        gap: 4.0,
        output_dir: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_pages_preserve_individual_pages_until_grouped() {
        let dir = crate::util::fs::temp_named_path("docsy-image-pages", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let source = dir.join("sample.png");
        image::RgbImage::from_pixel(200, 120, image::Rgb([10, 20, 30]))
            .save(&source)
            .unwrap();
        let edit = ImageEdit {
            reference_width: None,
            path: source.to_string_lossy().into(),
            rotation: 0,
            crop: full_crop(),
            scale: 1.0,
        };
        let mut args = ImagePdfArgs {
            page_layouts: (0..3)
                .map(|_| ImagePageLayout {
                    count: 1,
                    rows: 1,
                    cols: 1,
                })
                .collect(),
            images: vec![edit.clone(), edit.clone(), edit],
            landscape: false,
            rows: 1,
            cols: 1,
            margin: 10.0,
            gap: 4.0,
            output_dir: None,
        };
        assert_eq!(plan(&args).unwrap().pages.len(), 3);
        args.page_layouts = vec![
            ImagePageLayout {
                count: 1,
                rows: 1,
                cols: 1,
            },
            ImagePageLayout {
                count: 2,
                rows: 2,
                cols: 1,
            },
        ];
        let preview = plan(&args).unwrap();
        assert_eq!(
            preview
                .pages
                .iter()
                .map(|page| page.iter().map(|image| image.index).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![0], vec![1, 2]]
        );
        let pdf = lopdf::Document::load(export(&args).unwrap()).unwrap();
        assert_eq!(pdf.get_pages().len(), 2);
        args.page_layouts[1].rows = 1;
        let expanded = plan(&args).unwrap();
        assert_eq!(expanded.pages.len(), 2);
        assert_eq!(expanded.pages[1].len(), 2);
        assert!(expanded.pages[1][1].y > expanded.pages[1][0].y);
        assert_eq!(
            lopdf::Document::load(export(&args).unwrap())
                .unwrap()
                .get_pages()
                .len(),
            2
        );
    }

    #[test]
    fn shrinking_images_keeps_an_overfull_grid_on_one_page_without_changing_scale() {
        let dir = crate::util::fs::temp_named_path("docsy-image-shrink", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let source = dir.join("wide.png");
        image::RgbImage::from_pixel(600, 360, image::Rgb([20, 30, 40]))
            .save(&source)
            .unwrap();
        let edit = ImageEdit {
            path: source.to_string_lossy().into(),
            reference_width: Some(190.0),
            rotation: 90,
            crop: full_crop(),
            scale: 1.0,
        };
        let mut args = ImagePdfArgs {
            images: vec![edit.clone(), edit],
            page_layouts: vec![ImagePageLayout {
                count: 2,
                rows: 1,
                cols: 1,
            }],
            landscape: false,
            rows: 1,
            cols: 1,
            margin: 10.0,
            gap: 4.0,
            output_dir: None,
        };
        let oversized = plan(&args).unwrap();
        assert_eq!(oversized.pages.len(), 1);
        assert!(oversized.pages[0].iter().all(|p| !p.fits));
        assert!(export(&args).is_err());
        for edit in &mut args.images {
            edit.scale = 0.5;
        }
        let small = plan(&args).unwrap();
        assert_eq!(small.pages[0].len(), 2);
        for (small, large) in small.pages[0].iter().zip(&oversized.pages[0]) {
            assert!(small.fits);
            assert!((small.width * 2.0 - large.width).abs() < 1e-8);
            assert!((small.height * 2.0 - large.height).abs() < 1e-8);
            assert_eq!(small.reference_width, 190.0);
            assert!(small.x >= args.margin && small.y >= args.margin);
            assert!(small.x + small.width <= 210.0 - args.margin);
            assert!(small.y + small.height <= 297.0 - args.margin);
        }
        let pdf = lopdf::Document::load(export(&args).unwrap()).unwrap();
        assert_eq!(pdf.get_pages().len(), 1);
        let page_id = *pdf.get_pages().values().next().unwrap();
        let content =
            lopdf::content::Content::decode(&pdf.get_page_content(page_id).unwrap()).unwrap();
        assert_eq!(
            content
                .operations
                .iter()
                .filter(|op| op.operator == "Do")
                .count(),
            2
        );
    }

    #[test]
    fn rotation_keeps_print_scale_and_reports_when_it_does_not_fit() {
        let dir = crate::util::fs::temp_named_path("docsy-image-scale", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let source = dir.join("wide.png");
        image::RgbImage::from_pixel(600, 360, image::Rgb([10, 20, 30]))
            .save(&source)
            .unwrap();
        let mut args = ImagePdfArgs {
            page_layouts: Vec::new(),
            images: vec![ImageEdit {
                reference_width: Some(90.0),
                path: source.to_string_lossy().into(),
                rotation: 0,
                crop: full_crop(),
                scale: 1.0,
            }],
            landscape: false,
            rows: 1,
            cols: 2,
            margin: 10.0,
            gap: 4.0,
            output_dir: None,
        };
        let before = plan(&args).unwrap();
        args.images[0].rotation = 90;
        let after = plan(&args).unwrap();
        assert!((before.pages[0][0].width - after.pages[0][0].height).abs() < 0.001);
        assert!((before.pages[0][0].height - after.pages[0][0].width).abs() < 0.001);
        assert!(after.pages[0][0].fits);
        args.images[0].reference_width = Some(190.0);
        let overflow = plan(&args).unwrap();
        assert!(!overflow.pages[0][0].fits);
        assert!(export(&args).is_err());
        args.images[0].scale = overflow.pages[0][0].max_scale;
        assert!(plan(&args).unwrap().pages[0][0].fits);
    }

    #[test]
    fn reviewed_crop_rotation_grid_and_pixels_match_pdf_output() {
        let dir = crate::util::fs::temp_named_path("docsy-image-plan", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let source = dir.join("pattern.png");
        image::RgbImage::from_fn(1200, 800, |x, y| {
            image::Rgb([(x % 251) as u8, (y % 251) as u8, 90])
        })
        .save(&source)
        .unwrap();
        let edit = ImageEdit {
            reference_width: None,
            path: source.to_string_lossy().into(),
            rotation: 90,
            crop: [0.25, 0.125, 0.75, 0.875],
            scale: 0.65,
        };
        let args = ImagePdfArgs {
            page_layouts: Vec::new(),
            images: vec![edit.clone(), edit.clone(), edit],
            landscape: false,
            rows: 1,
            cols: 2,
            margin: 12.0,
            gap: 6.0,
            output_dir: None,
        };
        let preview = plan(&args).unwrap();
        assert_eq!(
            preview.pages.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(preview.pages[0][0].crop_px, [300, 100, 600, 600]);
        let output = export(&args).unwrap();
        let pdf = lopdf::Document::load(output).unwrap();
        for (page_index, (_, page_id)) in pdf.get_pages().into_iter().enumerate() {
            let bounds = pdf
                .get_object(page_id)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap();
            assert!(
                (bounds[2].as_float().unwrap() as f64 * 25.4 / 72.0 - preview.width).abs() < 0.001
            );
            let content =
                lopdf::content::Content::decode(&pdf.get_page_content(page_id).unwrap()).unwrap();
            let matrices: Vec<_> = content
                .operations
                .iter()
                .filter(|op| op.operator == "cm" && op.operands[0].as_float().unwrap_or(0.0) > 2.0)
                .collect();
            assert_eq!(matrices.len(), preview.pages[page_index].len());
            for (matrix, placement) in matrices.iter().zip(&preview.pages[page_index]) {
                let m: Vec<_> = matrix
                    .operands
                    .iter()
                    .map(|v| v.as_float().unwrap() as f64 * 25.4 / 72.0)
                    .collect();
                assert!((m[0] - placement.width).abs() < 0.001);
                assert!((m[3] - placement.height).abs() < 0.001);
                assert!((m[4] - placement.x).abs() < 0.001);
                assert!((m[5] - (preview.height - placement.y - placement.height)).abs() < 0.001);
            }
        }
        let embedded: Vec<_> = pdf
            .objects
            .values()
            .filter_map(|object| object.as_stream().ok())
            .filter(|stream| {
                stream.dict.get(b"Subtype").and_then(|o| o.as_name()).ok()
                    == Some(b"Image".as_slice())
            })
            .collect();
        assert!(!embedded.is_empty());
        for stream in embedded {
            assert_eq!(stream.dict.get(b"Width").unwrap().as_i64().unwrap(), 600);
            assert_eq!(stream.dict.get(b"Height").unwrap().as_i64().unwrap(), 600);
            let filter = format!("{:?}", stream.dict.get(b"Filter"));
            assert!(
                !filter.contains("DCTDecode") && !filter.contains("JPXDecode"),
                "unexpected lossy encoding: {filter}"
            );
        }
        let mut invalid = args.clone();
        invalid.images[0].crop = [0.8, 0.0, 0.2, 1.0];
        assert!(plan(&invalid).is_err());
        invalid = args;
        invalid.images[0].scale = f64::NAN;
        assert!(plan(&invalid).is_err());
    }

    #[test]
    fn a4_import_keeps_aspect_margins_orientation_and_originals() {
        let dir = crate::util::fs::temp_named_path("docsy-a4-images", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        for (name, dimensions, expected_page) in [
            ("wide.png", (240, 120), (297.0, 210.0)),
            ("tall.png", (120, 240), (210.0, 297.0)),
            ("phone.jpg", (60, 120), (210.0, 297.0)),
        ] {
            let input = dir.join(name);
            if name.ends_with("jpg") {
                std::fs::write(&input, include_bytes!("fixtures/oriented-phone.jpg")).unwrap();
            } else {
                image::RgbImage::from_fn(dimensions.0, dimensions.1, |x, _| {
                    if x < dimensions.0 / 2 {
                        image::Rgb([255, 0, 0])
                    } else {
                        image::Rgb([0, 0, 255])
                    }
                })
                .save(&input)
                .unwrap();
            }
            let original = std::fs::read(&input).unwrap();
            assert_eq!(
                crate::util::images::oriented_dimensions(&input).unwrap(),
                dimensions
            );
            let existing = dir.join(format!(
                "{}_A4.pdf",
                input.file_stem().unwrap().to_str().unwrap()
            ));
            std::fs::write(&existing, b"keep-existing").unwrap();
            let output = convert_image_to_pdf(input.to_str().unwrap()).unwrap();
            assert_ne!(Path::new(&output), existing);
            assert_eq!(std::fs::read(&existing).unwrap(), b"keep-existing");
            assert_eq!(std::fs::read(&input).unwrap(), original);
            let pdf = lopdf::Document::load(&output).unwrap();
            assert_eq!(pdf.get_pages().len(), 1);
            let page = *pdf.get_pages().values().next().unwrap();
            let bounds = pdf
                .get_object(page)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap();
            let width = bounds[2].as_float().unwrap() as f64;
            let height = bounds[3].as_float().unwrap() as f64;
            assert!((width * 25.4 / 72.0 - expected_page.0).abs() < 0.1);
            assert!((height * 25.4 / 72.0 - expected_page.1).abs() < 0.1);
            let content =
                lopdf::content::Content::decode(&pdf.get_page_content(page).unwrap()).unwrap();
            let m = content
                .operations
                .iter()
                .find(|op| op.operator == "cm" && op.operands[0].as_float().unwrap_or(0.0) > 2.0)
                .unwrap();
            let n: Vec<_> = m
                .operands
                .iter()
                .map(|v| v.as_float().unwrap() as f64)
                .collect();
            let margin = 10.0 * 72.0 / 25.4;
            assert!(n[4] >= margin - 0.1 && n[5] >= margin - 0.1);
            assert!(n[4] + n[0] <= width - margin + 0.1 && n[5] + n[3] <= height - margin + 0.1);
            assert!((n[3] / n[0] - dimensions.1 as f64 / dimensions.0 as f64).abs() < 0.001);
        }
    }
}
