//! Decode once with an explicit encoding policy shared by planning and export.
use anyhow::{bail, Context, Result};
use std::{io::Read, path::Path};

pub fn read_markdown(path: &Path, encoding: Option<&str>) -> Result<String> {
    const LIMIT: usize = 16 * 1024 * 1024;
    let file = std::fs::File::open(path)
        .with_context(|| format!("无法读取 Markdown 文件: {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        bail!("Markdown 文件过大，请分批转换");
    }
    decode(&bytes, encoding)
}

fn decode(bytes: &[u8], selected: Option<&str>) -> Result<String> {
    use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
    let selected = selected.unwrap_or("auto");
    let explicit = match selected {
        "auto" => None,
        "utf-8" => Some(UTF_8),
        "utf-16le" => Some(UTF_16LE),
        "utf-16be" => Some(UTF_16BE),
        "windows-1252" => Some(encoding_rs::WINDOWS_1252),
        "shift_jis" => Some(encoding_rs::SHIFT_JIS),
        "euc-kr" => Some(encoding_rs::EUC_KR),
        "gb18030" => Some(encoding_rs::GB18030),
        "big5" => Some(encoding_rs::BIG5),
        _ => bail!("不支持的 Markdown 文件编码: {selected}"),
    };
    // UTF-32LE shares the UTF-16LE prefix, so reject it before BOM detection.
    if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        bail!("暂不支持 UTF-32，请将 Markdown 另存为 UTF-8 后转换");
    }
    let (encoding, offset) = match Encoding::for_bom(bytes) {
        Some((bom, offset)) => {
            if explicit.is_some_and(|value| value != bom) {
                bail!("文件 BOM 与所选编码不一致，请选择自动识别或 {}", bom.name());
            }
            (bom, offset)
        }
        None => (explicit.unwrap_or(UTF_8), 0),
    };
    let text = encoding.decode_without_bom_handling_and_without_replacement(&bytes[offset..])
        .context("Markdown 文件编码不匹配，请选择正确的文件编码（如 Shift-JIS、EUC-KR、Windows-1252），或另存为 UTF-8；未生成乱码文件")?;
    if text.contains('\0') {
        bail!(
            "Markdown 含空字符，可能是无 BOM 的 UTF-16 文件，请选择 UTF-16 LE / BE 或另存为 UTF-8"
        );
    }
    if text.len() > 16 * 1024 * 1024 {
        bail!("Markdown 文本过大，请分批转换");
    }
    Ok(text.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEXT: &str =
        "Français : café, cœur, e\u{301} — 日本語・かな・カナ — 한국어 한글 — 中文 😀 $x^2$";

    #[test]
    fn unicode_and_boms_preserve_every_character() {
        assert_eq!(decode(TEXT.as_bytes(), None).unwrap(), TEXT);
        let mut utf8 = vec![0xef, 0xbb, 0xbf];
        utf8.extend(TEXT.as_bytes());
        assert_eq!(decode(&utf8, None).unwrap(), TEXT);
        for (bom, little, label) in [
            ([0xff, 0xfe], true, "utf-16le"),
            ([0xfe, 0xff], false, "utf-16be"),
        ] {
            let mut bytes = bom.to_vec();
            for unit in TEXT.encode_utf16() {
                bytes.extend(if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            assert_eq!(decode(&bytes, None).unwrap(), TEXT);
            assert_eq!(decode(&bytes[2..], Some(label)).unwrap(), TEXT);
            assert!(decode(&bytes, Some("utf-8")).is_err());
        }
    }

    #[test]
    fn selected_encoding_is_used_by_file_export_and_preserves_image_base_directory() {
        let dir = crate::util::fs::temp_named_path("docsy-encoding-test", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let text = "日本語の文書\n\n![画像](local.png)";
        image::DynamicImage::new_rgb8(8, 8)
            .save(dir.join("local.png"))
            .unwrap();
        let path = dir.join("日本語.md");
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
        assert!(!errors);
        std::fs::write(&path, bytes).unwrap();
        assert!(super::super::convert(path.to_str().unwrap(), None, None, None, None).is_err());
        let result = super::super::convert_with_media(
            path.to_str().unwrap(),
            None,
            None,
            None,
            None,
            None,
            Some("shift_jis"),
        )
        .unwrap();
        let file = std::fs::File::open(result.output_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut xml = String::new();
        zip.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(xml.contains("日本語の文書"));
        assert!(xml.contains("<w:drawing>"));
        let mut bom = vec![0xff, 0xfe];
        for unit in TEXT.encode_utf16() {
            bom.extend(unit.to_le_bytes());
        }
        std::fs::write(&path, bom).unwrap();
        assert_eq!(
            super::super::media::prepare(&read_markdown(&path, None).unwrap()).source_hash,
            super::super::media::prepare(TEXT).source_hash
        );
    }

    #[test]
    fn legacy_encodings_are_explicit_and_strict() {
        for (label, encoding, text) in [
            (
                "windows-1252",
                encoding_rs::WINDOWS_1252,
                "Français : café, cœur, Noël — 20 €",
            ),
            (
                "shift_jis",
                encoding_rs::SHIFT_JIS,
                "日本語：ひらがな・カタカナ",
            ),
            ("euc-kr", encoding_rs::EUC_KR, "한국어 한글 문서"),
            ("gb18030", encoding_rs::GB18030, "中文简体 😀"),
            ("big5", encoding_rs::BIG5, "繁體中文"),
        ] {
            let (bytes, _, errors) = encoding.encode(text);
            assert!(!errors);
            assert_eq!(decode(&bytes, Some(label)).unwrap(), text);
            assert!(decode(&bytes, None).is_err(), "must not guess {label}");
        }
        assert!(decode(&[0x82], Some("shift_jis")).is_err());
        assert!(decode(&[0x41, 0, 0x42, 0], None).is_err());
        assert!(decode(&[0xff, 0xfe, 0x00, 0xd8], None).is_err());
        assert!(decode(&[0xff, 0xfe, 0x41], None).is_err());
        assert!(decode(&[0xff, 0xfe, 0, 0], None).is_err());
    }
}
