//! Compatibility facade for the PDF header/footer processor.
//!
//! New implementation code lives in `header_footer`, `preview`, `normalize`,
//! and `page_info`. Keep this module so existing Tauri command paths and
//! frontend calls do not break while the public command names are migrated.

pub use super::header_footer::{batch_overlay, overlay_text, preview_overlay};
pub use super::preview::{render_preview, PreviewResult};
