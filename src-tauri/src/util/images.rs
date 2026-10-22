use anyhow::{bail, Result};
use image::DynamicImage;
use image::{ImageDecoder, metadata::Orientation};
use std::path::Path;

pub fn oriented_dimensions(path: &Path) -> Result<(u32, u32)> {
    let mut decoder = image::ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    let (w, h) = decoder.dimensions();
    Ok(match decoder.orientation()? {
        Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH => (h, w),
        _ => (w, h),
    })
}

pub fn open_oriented(path: &Path) -> Result<DynamicImage> {
    let mut decoder = image::ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn has_orientation_transform(path: &Path) -> Result<bool> {
    let mut decoder = image::ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    Ok(decoder.orientation()? != Orientation::NoTransforms)
}

pub fn validate_rotation(degrees: u16) -> Result<()> {
    if ![0, 90, 180, 270].contains(&degrees) {
        bail!("图片旋转角度必须是 0、90、180 或 270 度");
    }
    Ok(())
}

/// Clockwise, in memory only. The source file is never rewritten.
pub fn rotate(image: DynamicImage, degrees: u16) -> Result<DynamicImage> {
    validate_rotation(degrees)?;
    Ok(match degrees {
        90 => image.rotate90(),
        180 => image.rotate180(),
        270 => image.rotate270(),
        _ => image,
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_exif_orientation_matches_browser_pixels_without_rewriting_source() {
        let path = crate::util::fs::temp_named_path("docsy-exif-phone", "jpg");
        let _guard = crate::util::fs::TempPathGuard::new(path.clone());
        let source = include_bytes!("../pdf/fixtures/oriented-phone.jpg");
        std::fs::write(&path, source).unwrap();
        let corrected = open_oriented(&path).unwrap().to_rgb8();
        assert_eq!(corrected.dimensions(), (60, 120));
        assert_eq!(oriented_dimensions(&path).unwrap(), (60, 120));
        assert!(corrected.get_pixel(30, 10)[0] > 200);
        assert!(corrected.get_pixel(30, 110)[2] > 200);
        assert_eq!(std::fs::read(path).unwrap(), source);
    }
    #[test]
    fn clockwise_rotations_preserve_pixels_and_reject_arbitrary_angles() {
        let source = image::RgbImage::from_fn(3, 2, |x, y| image::Rgb([(y * 3 + x) as u8, 0, 0]));
        for (angle, dimensions, expected) in [
            (0, (3, 2), vec![0, 1, 2, 3, 4, 5]),
            (90, (2, 3), vec![3, 0, 4, 1, 5, 2]),
            (180, (3, 2), vec![5, 4, 3, 2, 1, 0]),
            (270, (2, 3), vec![2, 5, 1, 4, 0, 3]),
        ] {
            let rotated = rotate(DynamicImage::ImageRgb8(source.clone()), angle).unwrap().to_rgb8();
            assert_eq!(rotated.dimensions(), dimensions);
            assert_eq!(rotated.pixels().map(|p| p[0]).collect::<Vec<_>>(), expected);
        }
        assert!(rotate(DynamicImage::ImageRgb8(source), 45).is_err());
    }
}
