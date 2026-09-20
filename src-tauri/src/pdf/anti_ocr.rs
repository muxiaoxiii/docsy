use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

const FONT_BACKUP_KEY: &[u8] = b"DocsyOriginalCMap";
const BACKUP_KEY: &[u8] = b"DocsyAntiCopyBackup";
const MARKER: &str = "% Docsy anti-copy protection";

/// Anti-copy method
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum AntiCopyMethod {
    /// Scramble ToUnicode CMap (map chars to PUA range)
    CmapScramble,
    /// Remove ToUnicode CMap entirely
    CmapRemove,
}

impl AntiCopyMethod {
    /// 解析前端传入的 method 字符串；未知值回退到 CmapScramble（与历史行为一致）。
    pub fn parse(method: &str) -> Self {
        match method {
            "cmap_remove" => Self::CmapRemove,
            _ => Self::CmapScramble,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AntiCopyDetection {
    pub has_protection: bool,
    pub method: Option<String>,
    pub total_fonts: usize,
    pub protected_fonts: usize,
    pub has_backup: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct AntiCopyOutput {
    pub output_path: String,
    pub modified_fonts: usize,
    pub detection: AntiCopyDetection,
}

pub fn process_copy(input: &Path, requested_output: &Path, method: Option<AntiCopyMethod>) -> Result<AntiCopyOutput> {
    let before = detect_anti_copy(input)?;
    if method.is_some() && before.has_protection {
        anyhow::bail!("文件已有防复制标记，请先核对或恢复，避免覆盖原始文字映射备份");
    }
    if method.is_none() && !before.has_backup {
        anyhow::bail!("文件没有可恢复的原始文字映射备份");
    }
    let parent = requested_output.parent().unwrap_or_else(|| Path::new("."));
    let stem = requested_output.file_stem().and_then(|name| name.to_str()).unwrap_or("result");
    let output = crate::util::fs::unique_output_path(parent, stem, "pdf");
    let modified_fonts = match method {
        Some(method) => apply_anti_copy(input, &output, method)?,
        None => remove_anti_copy(input, &output)?,
    };
    let detection = detect_anti_copy(&output)?;
    Ok(AntiCopyOutput { output_path: output.to_string_lossy().into_owned(), modified_fonts, detection })
}

/// Detect anti-copy protection on a PDF
pub fn detect_anti_copy(input: &Path) -> Result<AntiCopyDetection> {
    let doc = Document::load(input).context("读取 PDF 失败")?;
    let page_ids = doc.get_pages();
    let has_backup = get_backup_meta(&doc).is_some();

    let mut total_fonts = 0;
    let mut protected_fonts = 0;
    let mut detected_method: Option<String> = None;
    let mut seen: HashSet<ObjectId> = HashSet::new();

    for page_id in page_ids.values() {
        for font_id in page_fonts(&doc, *page_id) {
            if !seen.insert(font_id) {
                continue;
            }
            total_fonts += 1;

            if let Some(cmap_bytes) = get_tounicode_cmap(&doc, font_id) {
                let cmap_text = String::from_utf8_lossy(&cmap_bytes);
                if cmap_text.contains(MARKER) || has_pua_mappings(&cmap_text) {
                    protected_fonts += 1;
                    detected_method.get_or_insert_with(|| "CMap 篡改".to_string());
                }
            } else if has_backup {
                // No CMap but we have backup → CMap was removed
                protected_fonts += 1;
                detected_method.get_or_insert_with(|| "CMap 移除".to_string());
            }
        }
    }

    Ok(AntiCopyDetection {
        has_protection: protected_fonts > 0 || has_backup,
        method: detected_method,
        total_fonts,
        protected_fonts,
        has_backup,
    })
}

/// Apply anti-copy protection
pub fn apply_anti_copy(input: &Path, output: &Path, method: AntiCopyMethod) -> Result<usize> {
    let mut doc = Document::load(input).context("读取 PDF 失败")?;

    // Build backup data before modifying
    let backup = build_backup(&doc);
    // Object numbers change during qpdf rewrites and chunk merging. Carry each
    // original mapping on its font object as well as the document-level backup.
    for (key, cmap) in &backup.cmaps {
        let numbers: Vec<u32> = key.trim_matches(['(', ')']).split(',')
            .filter_map(|part| part.trim().parse().ok()).collect();
        if numbers.len() == 2 {
            if let Ok(font) = doc.get_object_mut((numbers[0], numbers[1] as u16)).and_then(Object::as_dict_mut) {
                font.set(FONT_BACKUP_KEY.to_vec(), Object::string_literal(cmap.as_bytes().to_vec()));
            }
        }
    }

    let page_ids = doc.get_pages();
    let mut modified = 0;
    let mut seen: HashSet<ObjectId> = HashSet::new();

    for page_id in page_ids.values() {
        match method {
            AntiCopyMethod::CmapScramble => {
                for font_id in page_fonts(&doc, *page_id) {
                    if !seen.insert(font_id) {
                        continue;
                    }
                    if let Some(cmap_bytes) = get_tounicode_cmap(&doc, font_id) {
                        let cmap_text = String::from_utf8_lossy(&cmap_bytes);
                        if !is_already_protected(&cmap_text) {
                            let scrambled = scramble_cmap(&cmap_text);
                            set_tounicode_cmap(&mut doc, font_id, scrambled.as_bytes());
                            modified += 1;
                        }
                    }
                }
            }
            AntiCopyMethod::CmapRemove => {
                for font_id in page_fonts(&doc, *page_id) {
                    if !seen.insert(font_id) {
                        continue;
                    }
                    if get_tounicode_cmap(&doc, font_id).is_some() {
                        remove_tounicode_cmap(&mut doc, font_id);
                        modified += 1;
                    }
                }
            }
        }
    }

    if modified > 0 {
        store_backup_meta(&mut doc, &backup);
    }
    doc.save(output).context("保存防复制PDF失败")?;
    Ok(modified)
}

/// Remove anti-copy protection (restore from backup)
pub fn remove_anti_copy(input: &Path, output: &Path) -> Result<usize> {
    let mut doc = Document::load(input).context("读取 PDF 失败")?;

    let backup = get_backup_meta(&doc);
    let page_ids = doc.get_pages();
    let mut restored = 0;
    let mut seen: HashSet<ObjectId> = HashSet::new();

    for page_id in page_ids.values() {
        for font_id in page_fonts(&doc, *page_id) {
            if !seen.insert(font_id) {
                continue;
            }

            let attached = doc.get_dictionary(font_id).ok()
                .and_then(|font| font.get(FONT_BACKUP_KEY).ok())
                .and_then(|value| value.as_str().ok()).map(|bytes| bytes.to_vec());
            if let Some(original) = attached {
                set_tounicode_cmap(&mut doc, font_id, &original);
                if let Ok(font) = doc.get_object_mut(font_id).and_then(Object::as_dict_mut) { font.remove(FONT_BACKUP_KEY); }
                restored += 1;
                continue;
            }
            // Try to restore from backup first
            if let Some(ref backup_data) = backup {
                let font_key = format!("{:?}", font_id);
                if let Some(original_cmap) = backup_data.cmaps.get(&font_key) {
                    set_tounicode_cmap(&mut doc, font_id, original_cmap.as_bytes());
                    restored += 1;
                    continue;
                }
            }

            // If no backup for this font, just clean up our markers
            if let Some(cmap_bytes) = get_tounicode_cmap(&doc, font_id) {
                let cmap_text = String::from_utf8_lossy(&cmap_bytes);
                if is_already_protected(&cmap_text) {
                    // Can't restore original, remove corrupted CMap
                    remove_tounicode_cmap(&mut doc, font_id);
                    restored += 1;
                }
            }
        }
    }

    // Remove backup metadata
    if backup.is_some() {
        remove_backup_meta(&mut doc);
    }

    doc.save(output).context("保存恢复后的PDF失败")?;
    Ok(restored)
}

// --- Backup data stored in PDF metadata ---

#[derive(serde::Serialize, serde::Deserialize)]
struct BackupData {
    #[serde(default)]
    version: u32,
    cmaps: BTreeMap<String, String>, // "font_id:obj_num:gen_num" → original CMap text
}

fn build_backup(doc: &Document) -> BackupData {
    let mut cmaps = BTreeMap::new();
    let mut seen: HashSet<ObjectId> = HashSet::new();
    let page_ids = doc.get_pages();

    for page_id in page_ids.values() {
        for font_id in page_fonts(doc, *page_id) {
            if !seen.insert(font_id) {
                continue;
            }
            let font_key = format!("{:?}", font_id);
            if let Some(cmap_bytes) = get_tounicode_cmap(doc, font_id) {
                let cmap_text = String::from_utf8_lossy(&cmap_bytes).to_string();
                cmaps.insert(font_key, cmap_text);
            }
        }
    }

    BackupData { version: 2, cmaps }
}

fn store_backup_meta(doc: &mut Document, backup: &BackupData) {
    let json = serde_json::to_string(backup).unwrap_or_default();
    let json_bytes = json.as_bytes().to_vec();

    // Try to use existing Info dictionary
    let info_ref = doc.trailer.get(b"Info").ok().cloned();
    match info_ref {
        Some(Object::Reference(info_id)) => {
            if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(info_id) {
                dict.set(BACKUP_KEY.to_vec(), Object::string_literal(json_bytes));
                return;
            }
        }
        Some(Object::Dictionary(mut dict)) => {
            dict.set(BACKUP_KEY.to_vec(), Object::string_literal(json_bytes));
            doc.trailer.set(b"Info", Object::Dictionary(dict));
            return;
        }
        _ => {}
    }

    // No Info dict - create one and add to trailer via indirect reference
    let mut info_dict = Dictionary::new();
    info_dict.set(BACKUP_KEY.to_vec(), Object::string_literal(json_bytes));
    let info_id = doc.add_object(Object::Dictionary(info_dict));
    // lopdf's trailer is a Dictionary - we can set directly
    doc.trailer.set(b"Info", Object::Reference(info_id));
}

fn get_backup_meta(doc: &Document) -> Option<BackupData> {
    let info_ref = doc.trailer.get(b"Info").ok()?;
    let info_dict = match info_ref {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        Object::Dictionary(d) => d,
        _ => return None,
    };
    let backup_obj = info_dict.get(BACKUP_KEY).ok()?;
    let json_bytes = match backup_obj {
        Object::String(bytes, _) => bytes.clone(),
        _ => return None,
    };
    let json = String::from_utf8_lossy(&json_bytes);
    serde_json::from_str(&json).ok()
}

fn remove_backup_meta(doc: &mut Document) {
    let info_ref = doc.trailer.get(b"Info").ok().cloned();
    match info_ref {
        Some(Object::Reference(info_id)) => {
            if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(info_id) {
                dict.remove(BACKUP_KEY);
            }
        }
        Some(Object::Dictionary(mut dict)) => {
            dict.remove(BACKUP_KEY);
            doc.trailer.set(b"Info", Object::Dictionary(dict));
        }
        _ => {}
    }
}

// --- Font helpers ---

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
    stream.get_plain_content().ok()
}

fn set_tounicode_cmap(doc: &mut Document, font_id: ObjectId, data: &[u8]) {
    // Try updating existing ToUnicode stream first
    if let Ok(font_dict) = doc.get_dictionary(font_id) {
        if let Ok(Object::Reference(cmap_id)) = font_dict.get(b"ToUnicode") {
            if let Ok(Object::Stream(stream)) = doc.get_object_mut(*cmap_id) {
                *stream = Stream::new(Dictionary::new(), data.to_vec());
                return;
            }
        }
    }
    // ToUnicode was removed (CmapRemove) — create new stream and add to font dict
    let new_stream_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        data.to_vec(),
    )));
    if let Ok(font_dict) = doc.get_dictionary_mut(font_id) {
        font_dict.set(b"ToUnicode", Object::Reference(new_stream_id));
    }
}

fn remove_tounicode_cmap(doc: &mut Document, font_id: ObjectId) {
    if let Ok(font_dict) = doc.get_dictionary_mut(font_id) {
        font_dict.remove(b"ToUnicode");
    }
}

// --- CMap analysis ---

fn is_already_protected(cmap_text: &str) -> bool {
    cmap_text.contains(MARKER)
}

fn has_pua_mappings(cmap_text: &str) -> bool {
    let mut pua_hits = 0;
    let mut total_mappings = 0;
    for line in cmap_text.lines() {
        if line.contains("<") && line.contains(">") && !line.starts_with('%') {
            total_mappings += 1;
            // Check if any hex value maps to PUA range (E000-F8FF)
            for part in line.split('<') {
                if let Some(end) = part.find('>') {
                    let hex = &part[..end];
                    if let Ok(val) = u32::from_str_radix(hex, 16) {
                        if (0xE000..=0xF8FF).contains(&val) {
                            pua_hits += 1;
                        }
                    }
                }
            }
        }
    }
    total_mappings > 5 && pua_hits > total_mappings / 3
}

// --- CMap scrambling ---

fn scramble_cmap(original: &str) -> String {
    let mut result = String::new();
    result.push_str(&format!("{}\n", MARKER));
    result.push_str("% Original CMap replaced with garbled mappings\n");

    for line in original.lines() {
        if is_structural_line(line) {
            result.push_str(line);
            result.push('\n');
        } else if line.contains("<") && line.contains(">") {
            result.push_str(&scramble_mapping_line(line));
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn is_structural_line(line: &str) -> bool {
    line.contains("beginbfchar")
        || line.contains("endbfchar")
        || line.contains("beginbfrange")
        || line.contains("endbfrange")
        || line.contains("begincmap")
        || line.contains("endcmap")
        || line.contains("CMapName")
        || line.contains("CMapType")
        || line.contains("WMode")
        || line.contains("codespacerange")
        || line.starts_with('%')
        || line.trim().is_empty()
}

fn scramble_mapping_line(line: &str) -> String {
    let parts: Vec<&str> = line.split('<').collect();
    if parts.len() < 3 {
        return line.to_string();
    }

    let mut result = String::new();
    result.push_str(parts[0]);

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let end = part.find('>').unwrap_or(part.len());
        let hex_val = &part[..end];
        let rest = &part[end..];

        if i == parts.len() - 1 {
            let scrambled = scramble_hex(hex_val);
            result.push_str(&format!("<{}>{}", scrambled, rest));
        } else {
            result.push_str(&format!("<{}>{}", hex_val, rest));
        }
    }

    result
}

fn scramble_hex(hex: &str) -> String {
    let hash: u32 = hex
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let pua_char = 0xE000 + (hash % 0x1000);
    format!("{:04X}", pua_char)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    fn create_pdf(path: &Path, with_cmap: bool) -> ObjectId {
        let mut doc = Document::with_version("1.7");
        let pages = doc.new_object_id();
        let cmap = b"begincmap\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n1 beginbfchar\n<41> <0041>\nendbfchar\nendcmap\n";
        let mut stream = Stream::new(Dictionary::new(), cmap.to_vec());
        stream.compress().unwrap();
        let cmap_id = doc.add_object(stream);
        let mut font = dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" };
        if with_cmap {
            font.set("ToUnicode", cmap_id);
        }
        let font_id = doc.add_object(font);
        let content = doc.add_object(Stream::new(Dictionary::new(), b"BT /F1 12 Tf 20 20 Td (A) Tj ET".to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "Contents" => content,
        });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        doc.save(path).unwrap();
        font_id
    }

    #[test]
    fn compressed_cmap_roundtrip_preserves_source_and_existing_outputs() {
        let directory = crate::pdf::temp_named_path("anti_copy_roundtrip", "dir");
        std::fs::create_dir_all(&directory).unwrap();
        for (index, method) in [AntiCopyMethod::CmapScramble, AntiCopyMethod::CmapRemove].into_iter().enumerate() {
            let input = directory.join(format!("source-{index}.pdf"));
            let font_id = create_pdf(&input, true);
            let original = std::fs::read(&input).unwrap();
            let original_cmap = get_tounicode_cmap(&Document::load(&input).unwrap(), font_id).unwrap();
            let protected = process_copy(&input, &input, Some(method)).unwrap();
            assert_ne!(protected.output_path, input.to_string_lossy());
            assert!(protected.detection.has_protection && protected.detection.has_backup);
            assert_eq!(protected.modified_fonts, 1);
            let protected_bytes = std::fs::read(&protected.output_path).unwrap();
            let restored = process_copy(Path::new(&protected.output_path), Path::new(&protected.output_path), None).unwrap();
            assert!(!restored.detection.has_protection);
            assert_eq!(get_tounicode_cmap(&Document::load(restored.output_path).unwrap(), font_id).unwrap(), original_cmap);
            assert_eq!(std::fs::read(&input).unwrap(), original);
            assert_eq!(std::fs::read(&protected.output_path).unwrap(), protected_bytes);
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn no_mapping_is_not_reported_as_protected_and_unbacked_restore_is_rejected() {
        let directory = crate::pdf::temp_named_path("anti_copy_empty", "dir");
        std::fs::create_dir_all(&directory).unwrap();
        let input = directory.join("source.pdf");
        create_pdf(&input, false);
        let result = process_copy(&input, &directory.join("output.pdf"), Some(AntiCopyMethod::CmapScramble)).unwrap();
        assert_eq!(result.modified_fonts, 0);
        assert!(!result.detection.has_protection);
        assert!(process_copy(&input, &directory.join("restore.pdf"), None).is_err());
        assert!(!directory.join("restore.pdf").exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
