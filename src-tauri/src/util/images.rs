use anyhow::{bail, Result};
use image::DynamicImage;

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
