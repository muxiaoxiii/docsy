pub mod annotations;
pub mod anti_ocr;
pub mod artifacts;
pub(crate) mod bookmarks;
pub(crate) mod cmap;
pub mod compress;
pub mod content_text;
pub mod detection;
pub mod evidence;
pub mod evidence_session;
mod glyph_names;
pub mod header_footer;
pub mod normalize;
pub(crate) mod overlay_font;
pub(crate) mod overlay_pdf;
pub mod page_info;
pub mod preview;
pub mod qpdf;
pub(crate) mod qpdf_stream;
pub mod split;
pub(crate) mod text_utils;

// 通用 fs 工具已收敛到 crate::util::fs；此处 re-export 保持原有调用路径不变。
pub use crate::util::fs::{safe_file_stem, same_path, temp_named_path, unique_output_path};

/// FNV-1a 64-bit hash，用于生成确定性 ID。
pub fn fnv1a_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}
