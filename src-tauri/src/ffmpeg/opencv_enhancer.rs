#[derive(Debug, Clone, Copy)]
pub struct OpenCvMatch {
    pub confidence: f64,
    pub match_ratio: f64,
    pub inlier_ratio: f64,
    pub shift_x: f64,
    pub shift_y: f64,
}

#[cfg(docsy_opencv)]
#[repr(C)]
struct OpenCvMatchRaw {
    confidence: f64,
    match_ratio: f64,
    inlier_ratio: f64,
    shift_x: f64,
    shift_y: f64,
}

#[cfg(docsy_opencv)]
unsafe extern "C" {
    fn docsy_opencv_compare_luma(
        previous: *const u8,
        current: *const u8,
        width: i32,
        height: i32,
        output: *mut OpenCvMatchRaw,
    ) -> i32;
}

pub fn is_available() -> bool {
    cfg!(docsy_opencv)
}

#[cfg(docsy_opencv)]
pub fn compare_luma(
    previous: &[u8],
    current: &[u8],
    width: u32,
    height: u32,
) -> Option<OpenCvMatch> {
    let expected = width as usize * height as usize;
    if previous.len() < expected || current.len() < expected {
        return None;
    }
    let mut raw = OpenCvMatchRaw {
        confidence: 0.0,
        match_ratio: 0.0,
        inlier_ratio: 0.0,
        shift_x: 0.0,
        shift_y: 0.0,
    };
    let status = unsafe {
        docsy_opencv_compare_luma(
            previous.as_ptr(),
            current.as_ptr(),
            width as i32,
            height as i32,
            &mut raw,
        )
    };
    if status != 1 {
        return None;
    }
    Some(OpenCvMatch {
        confidence: raw.confidence.clamp(0.0, 1.0),
        match_ratio: raw.match_ratio.clamp(0.0, 1.0),
        inlier_ratio: raw.inlier_ratio.clamp(0.0, 1.0),
        shift_x: raw.shift_x,
        shift_y: raw.shift_y,
    })
}

#[cfg(not(docsy_opencv))]
pub fn compare_luma(
    _previous: &[u8],
    _current: &[u8],
    _width: u32,
    _height: u32,
) -> Option<OpenCvMatch> {
    None
}

#[cfg(all(test, docsy_opencv))]
mod tests {
    use super::*;

    #[test]
    fn embedded_opencv_detects_a_known_horizontal_shift() {
        let width = 96_usize;
        let height = 96_usize;
        let previous = (0..width * height)
            .map(|index| ((index * 37 + index / width * 53 + 19) % 251) as u8)
            .collect::<Vec<_>>();
        let shift = 7_usize;
        let mut current = vec![0_u8; previous.len()];
        for y in 0..height {
            for x in 0..width {
                current[y * width + (x + shift) % width] = previous[y * width + x];
            }
        }
        let result = compare_luma(&previous, &current, width as u32, height as u32).unwrap();
        assert!(result.confidence > 0.20);
        assert!((result.shift_x.abs() - shift as f64).abs() < 1.0);
        assert!(result.shift_y.abs() < 1.0);
    }
}
