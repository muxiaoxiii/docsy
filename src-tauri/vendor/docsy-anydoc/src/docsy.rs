//! Docsy-specific Office to Markdown policy.
//!
//! The upstream parser owns the format frontends and normalized document model.
//! This module is the deliberately narrow integration boundary used by Docsy:
//! it never accepts PDFs, never executes macros, and exposes enough metadata
//! for the application to tell users when embedded material needs review.

use crate::error::ConvertError;
use crate::model::AssetId;
use crate::render::markdown::document_to_markdown_with_asset_paths;
use crate::{Format, to_document};
use std::collections::HashMap;
use std::path::Path;

/// Parsed Markdown plus document facts that Docsy uses for user-facing notices.
#[derive(Debug, Clone)]
pub struct OfficeMarkdown {
    /// Clean GitHub-Flavored Markdown representing the source document.
    pub markdown: String,
    /// Resolved source format after content-based detection with extension fallback.
    pub format: Format,
    /// Number of source assets that were referenced by the document model.
    pub asset_count: usize,
    /// Number of footnotes or endnotes represented in the document model.
    pub note_count: usize,
    /// Embedded payloads that should be written beside the Markdown output.
    pub assets: Vec<ExportAsset>,
}

/// One extracted asset prepared for a Docsy-managed output directory.
#[derive(Debug, Clone)]
pub struct ExportAsset {
    /// Relative file name referenced by the generated Markdown.
    pub file_name: String,
    /// MIME type declared by the source package.
    pub media_type: String,
    /// Original payload bytes.
    pub bytes: Vec<u8>,
}

/// Read a supported Office-like document and render it with Docsy's Markdown policy.
///
/// Unlike the generic upstream convenience function, this returns model metadata and
/// rejects PDFs. PDF conversion has its own loss-aware workflow in Docsy's PDF module.
pub fn extract_office_markdown(
    path: impl AsRef<Path>,
    asset_directory_name: &str,
) -> Result<OfficeMarkdown, ConvertError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let format = Format::from_bytes(&bytes)
        .or_else(|| Format::from_path(path))
        .ok_or_else(|| {
            ConvertError::Unsupported(format!(
                "unrecognized file content and extension: {}",
                path.display()
            ))
        })?;
    let document = to_document(&bytes, format)?;
    let mut asset_paths = HashMap::<AssetId, String>::new();
    let mut assets = Vec::with_capacity(document.assets.len());
    for (index, asset) in document.assets.iter().enumerate() {
        let file_name = format!("asset-{:03}.{}", index + 1, extension_for_media_type(&asset.media_type));
        asset_paths.insert(asset.id, format!("{asset_directory_name}/{file_name}"));
        assets.push(ExportAsset {
            file_name,
            media_type: asset.media_type.clone(),
            bytes: asset.bytes.clone(),
        });
    }
    Ok(OfficeMarkdown {
        markdown: normalize_markdown(document_to_markdown_with_asset_paths(&document, asset_paths)),
        format,
        asset_count: document.assets.len(),
        note_count: document.notes.len(),
        assets,
    })
}

fn extension_for_media_type(media_type: &str) -> &'static str {
    match media_type.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        "image/svg+xml" => "svg",
        "image/x-emf" | "image/emf" => "emf",
        "image/x-wmf" | "image/wmf" => "wmf",
        _ => "bin",
    }
}

/// Keep the output stable across source programs without applying language-specific
/// rewrite rules. In particular, Chinese punctuation and intentional paragraph breaks
/// must remain source-owned content.
fn normalize_markdown(markdown: String) -> String {
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{extension_for_media_type, normalize_markdown};

    #[test]
    fn preserves_paragraphs_and_normalizes_line_endings() {
        assert_eq!(normalize_markdown("甲\r\n\r\n乙\r\n".to_string()), "甲\n\n乙\n");
    }

    #[test]
    fn derives_stable_asset_extensions() {
        assert_eq!(extension_for_media_type("image/png"), "png");
        assert_eq!(extension_for_media_type("IMAGE/JPEG"), "jpg");
        assert_eq!(extension_for_media_type("application/octet-stream"), "bin");
    }
}
