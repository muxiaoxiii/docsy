use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId, Stream};
use std::collections::BTreeMap;
use std::path::Path;

/// Result of anti-OCR detection
#[derive(Debug, Clone, serde::Serialize)]
pub struct AntiOcrDetectionResult {
    pub has_anti_ocr: bool,
    pub total_pages: usize,
    pub pages_with_valid_cmap: usize,
    pub pages_with_scrambled_cmap: usize,
    pub pages_without_cmap: usize,
    pub details: Vec<PageAntiOcrStatus>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PageAntiOcrStatus {
    pub page: u32,
    pub status: String,
    pub extracted_text_preview: String,
}

/// Detect if a PDF has anti-OCR protection applied.
/// Checks the ToUnicode CMap for each page's fonts.
pub fn detect_anti_ocr(input: &Path) -> Result<AntiOcrDetectionResult> {
    let doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids = doc.get_pages();

    let mut details = Vec::new();
    let mut pages_with_valid = 0;
    let mut pages_with_scrambled = 0;
    let mut pages_without = 0;

    for (page_num, page_id) in &page_ids {
        let fonts = page_fonts(&doc, *page_id);
        let mut has_cmap = false;
        let mut cmap_valid = true;
        let mut text_preview = String::new();

        if fonts.is_empty() {
            pages_without += 1;
            details.push(PageAntiOcrStatus {
                page: *page_num,
                status: "无字体信息".to_string(),
                extracted_text_preview: String::new(),
            });
            continue;
        }

        for font_id in &fonts {
            if let Some(cmap_stream) = get_tounicode_cmap(&doc, *font_id) {
                has_cmap = true;
                let cmap_text = String::from_utf8_lossy(&cmap_stream);
                if is_scrambled_cmap(&cmap_text) {
                    cmap_valid = false;
                }
            }
        }

        // Try to extract a small text sample from the page content
        if let Ok(content) = doc.get_and_decode_page_content(*page_id) {
            text_preview = extract_text_sample(&content.operations, &doc, *page_id);
        }

        if !has_cmap {
            pages_without += 1;
            details.push(PageAntiOcrStatus {
                page: *page_num,
                status: "无 ToUnicode CMap".to_string(),
                extracted_text_preview: text_preview,
            });
        } else if !cmap_valid {
            pages_with_scrambled += 1;
            details.push(PageAntiOcrStatus {
                page: *page_num,
                status: "CMap 已篡改（防OCR）".to_string(),
                extracted_text_preview: text_preview,
            });
        } else {
            pages_with_valid += 1;
            details.push(PageAntiOcrStatus {
                page: *page_num,
                status: "正常".to_string(),
                extracted_text_preview: text_preview,
            });
        }
    }

    let has_anti_ocr = pages_with_scrambled > 0;
    let total = page_ids.len();

    Ok(AntiOcrDetectionResult {
        has_anti_ocr,
        total_pages: total,
        pages_with_valid_cmap: pages_with_valid,
        pages_with_scrambled_cmap: pages_with_scrambled,
        pages_without_cmap: pages_without,
        details,
    })
}

/// Apply anti-OCR protection to a PDF by scrambling ToUnicode CMaps.
/// The visual appearance is preserved (font glyphs unchanged),
/// but text extraction will produce garbled output.
pub fn apply_anti_ocr(input: &Path, output: &Path) -> Result<usize> {
    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids = doc.get_pages();

    let mut modified_fonts = 0;
    let mut seen_fonts: std::collections::HashSet<ObjectId> = std::collections::HashSet::new();

    for (_, page_id) in &page_ids {
        let fonts = page_fonts(&doc, *page_id);
        for font_id in fonts {
            if seen_fonts.contains(&font_id) {
                continue;
            }
            seen_fonts.insert(font_id);

            if let Some(cmap_stream) = get_tounicode_cmap(&doc, font_id) {
                let cmap_text = String::from_utf8_lossy(&cmap_stream);
                if !is_scrambled_cmap(&cmap_text) {
                    let scrambled = scramble_cmap(&cmap_text);
                    set_tounicode_cmap(&mut doc, font_id, scrambled.as_bytes());
                    modified_fonts += 1;
                }
            }
        }
    }

    doc.save(output)
        .context("保存防OCR处理后的PDF失败")?;
    Ok(modified_fonts)
}

/// Remove anti-OCR protection by restoring valid ToUnicode CMaps.
/// This is a best-effort restoration based on available font encoding data.
pub fn remove_anti_ocr(input: &Path, output: &Path) -> Result<usize> {
    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids = doc.get_pages();

    let mut restored_fonts = 0;
    let mut seen_fonts: std::collections::HashSet<ObjectId> = std::collections::HashSet::new();

    for (_, page_id) in &page_ids {
        let fonts = page_fonts(&doc, *page_id);
        for font_id in fonts {
            if seen_fonts.contains(&font_id) {
                continue;
            }
            seen_fonts.insert(font_id);

            if let Some(cmap_stream) = get_tounicode_cmap(&doc, font_id) {
                let cmap_text = String::from_utf8_lossy(&cmap_stream);
                if is_scrambled_cmap(&cmap_text) {
                    // Try to rebuild a valid CMap from the font's Encoding/BaseFont
                    if let Some(restored) = try_restore_cmap(&doc, font_id, &cmap_text) {
                        set_tounicode_cmap(&mut doc, font_id, restored.as_bytes());
                        restored_fonts += 1;
                    } else {
                        // If we can't restore, remove the CMap entirely
                        // so OCR tools can fall back to visual recognition
                        remove_tounicode_cmap(&mut doc, font_id);
                        restored_fonts += 1;
                    }
                }
            }
        }
    }

    doc.save(output)
        .context("保存移除防OCR后的PDF失败")?;
    Ok(restored_fonts)
}

// --- Helper functions ---

fn page_fonts(doc: &Document, page_id: ObjectId) -> Vec<ObjectId> {
    let mut fonts = Vec::new();
    if let Ok(page_dict) = doc.get_dictionary(page_id) {
        if let Ok(resources) = page_dict.get(b"Resources") {
            let res_dict = match resources {
                Object::Reference(id) => doc.get_dictionary(*id).ok(),
                Object::Dictionary(d) => Some(d),
                _ => None,
            };
            if let Some(res) = res_dict {
                if let Ok(font_dict) = res.get(b"Font") {
                    let fd = match font_dict {
                        Object::Reference(id) => doc.get_dictionary(*id).ok(),
                        Object::Dictionary(d) => Some(d),
                        _ => None,
                    };
                    if let Some(fd) = fd {
                        for (_, obj) in fd.iter() {
                            if let Object::Reference(id) = obj {
                                fonts.push(*id);
                            }
                        }
                    }
                }
            }
        }
    }
    fonts
}

fn get_tounicode_cmap(doc: &Document, font_id: ObjectId) -> Option<Vec<u8>> {
    let font_dict = doc.get_dictionary(font_id).ok()?;
    let tounicode = font_dict.get(b"ToUnicode").ok()?;
    let stream = match tounicode {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_stream().ok()?,
        Object::Stream(s) => s,
        _ => return None,
    };
    Some(stream.content.clone())
}

fn set_tounicode_cmap(doc: &mut Document, font_id: ObjectId, data: &[u8]) {
    if let Ok(font_dict) = doc.get_dictionary(font_id) {
        if let Ok(Object::Reference(cmap_id)) = font_dict.get(b"ToUnicode") {
            if let Ok(Object::Stream(stream)) = doc.get_object_mut(*cmap_id) {
                *stream = Stream::new(
                    lopdf::Dictionary::new(),
                    data.to_vec(),
                );
            }
        }
    }
}

fn remove_tounicode_cmap(doc: &mut Document, font_id: ObjectId) {
    if let Ok(font_dict) = doc.get_dictionary_mut(font_id) {
        font_dict.remove(b"ToUnicode");
    }
}

/// Check if a CMap has been scrambled (anti-OCR applied).
/// A scrambled CMap maps character codes to non-standard or random Unicode values.
fn is_scrambled_cmap(cmap_text: &str) -> bool {
    // Check for our scramble marker
    if cmap_text.contains("% Anti-OCR protection applied") {
        return true;
    }

    // Check for suspicious patterns:
    // 1. CMap maps to Private Use Area (PUA) characters
    let pua_count = cmap_text.matches("<").count();
    let pua_target_count = cmap_text
        .lines()
        .filter(|line| line.contains("beginbfchar") || line.contains("beginbfrange"))
        .count();

    // If there are many mappings and they target PUA range (U+E000-U+F8FF)
    if pua_count > 10 && pua_target_count > 0 {
        let mut pua_hits = 0;
        for line in cmap_text.lines() {
            // Look for mappings like <XX> <EXXX> (PUA range)
            if line.contains("<E") || line.contains("<F") {
                pua_hits += 1;
            }
        }
        if pua_hits > pua_count / 3 {
            return true;
        }
    }

    false
}

/// Scramble a CMap by remapping character codes to random Unicode values.
/// Uses a deterministic seed based on the original mapping for consistency.
fn scramble_cmap(original: &str) -> String {
    let mut result = String::new();
    result.push_str("% Anti-OCR protection applied by Docsy\n");
    result.push_str("% Original CMap has been scrambled to prevent text extraction\n");

    for line in original.lines() {
        if line.contains("beginbfchar") || line.contains("endbfchar") ||
           line.contains("beginbfrange") || line.contains("endbfrange") ||
           line.contains("begincmap") || line.contains("endcmap") ||
           line.contains("CMapName") || line.contains("CMapType") ||
           line.contains("WMode") || line.contains("codespacerange") ||
           line.starts_with('%') || line.trim().is_empty()
        {
            result.push_str(line);
            result.push('\n');
        } else if line.contains("<") && line.contains(">") {
            // This is a character mapping line - scramble the target
            result.push_str(&scramble_mapping_line(line));
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn scramble_mapping_line(line: &str) -> String {
    // Parse patterns like: <XX> <YYYY> or <XX> <XX> <YYYY>
    let parts: Vec<&str> = line.split('<').collect();
    if parts.len() < 3 {
        return line.to_string();
    }

    let mut result = String::new();
    result.push_str(parts[0]); // leading whitespace

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let end = part.find('>').unwrap_or(part.len());
        let hex_val = &part[..end];
        let rest = &part[end..];

        if i == parts.len() - 1 {
            // Last hex value - scramble it (this is the Unicode target)
            let scrambled = scramble_hex(hex_val);
            result.push_str(&format!("<{}>{}", scrambled, rest));
        } else {
            result.push_str(&format!("<{}>{}", hex_val, rest));
        }
    }

    result
}

fn scramble_hex(hex: &str) -> String {
    // Map to Unicode Private Use Area: E000-EFFF
    // Use a simple hash of the original value for determinism
    let hash: u32 = hex.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let pua_char = 0xE000 + (hash % 0x1000);
    format!("{:04X}", pua_char)
}

fn extract_text_sample(_operations: &[lopdf::content::Operation], _doc: &Document, _page_id: ObjectId) -> String {
    // Simplified: just return empty for now
    // A full implementation would decode text operators (Tj, TJ, etc.)
    String::new()
}

fn try_restore_cmap(_doc: &Document, _font_id: ObjectId, scrambled: &str) -> Option<String> {
    // If the CMap was scrambled by us, we could try to reverse it
    // But since we use a hash-based scramble, it's not reversible
    // Return None to signal "remove the CMap and let OCR handle it"
    if scrambled.contains("% Anti-OCR protection applied") {
        return None;
    }
    None
}

