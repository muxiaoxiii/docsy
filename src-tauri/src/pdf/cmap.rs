//! PDF ToUnicode CMap parsing and CID text decoding.
//!
//! A PDF content stream stores character codes, not Unicode text.  For Type0
//! fonts those codes are only meaningful together with the font's ToUnicode
//! CMap.  This module deliberately returns `None` when a code cannot be
//! decoded completely; callers must then use their existing bbox based
//! fallback instead of guessing a replacement string.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::qpdf_stream::{self, QpdfObjectIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeSpaceRange {
    start: Vec<u8>,
    end: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToUnicodeCMap {
    code_spaces: Vec<CodeSpaceRange>,
    mappings: BTreeMap<Vec<u8>, String>,
}

impl ToUnicodeCMap {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let tokens = tokenize(data)?;
        let mut cmap = Self {
            code_spaces: Vec::new(),
            mappings: BTreeMap::new(),
        };
        let mut index = 0;
        while index < tokens.len() {
            let Token::Word(word) = &tokens[index] else {
                index += 1;
                continue;
            };
            match word.as_str() {
                "begincodespacerange" => {
                    index = parse_code_spaces(&tokens, index + 1, &mut cmap.code_spaces)?;
                }
                "beginbfchar" => {
                    index = parse_bfchar(&tokens, index + 1, &mut cmap.mappings)?;
                }
                "beginbfrange" => {
                    index = parse_bfrange(&tokens, index + 1, &mut cmap.mappings)?;
                }
                _ => index += 1,
            }
        }
        if cmap.code_spaces.is_empty() || cmap.mappings.is_empty() {
            anyhow::bail!("ToUnicode CMap 缺少可用的 codespace 或映射")
        }
        cmap.code_spaces
            .sort_by_key(|range| std::cmp::Reverse(range.start.len()));
        Ok(cmap)
    }

    /// Decode one complete PDF text string. Partial decoding is rejected.
    pub(crate) fn decode(&self, bytes: &[u8]) -> Option<String> {
        if bytes.is_empty() {
            return Some(String::new());
        }
        let mut offset = 0;
        let mut output = String::new();
        while offset < bytes.len() {
            let mut matched = false;
            for range in &self.code_spaces {
                let end = offset.checked_add(range.start.len())?;
                if end > bytes.len() || range.start.len() != range.end.len() {
                    continue;
                }
                let code = &bytes[offset..end];
                if !between(code, &range.start, &range.end) {
                    continue;
                }
                matched = true;
                output.push_str(self.mappings.get(code)?);
                offset = end;
                break;
            }
            if !matched {
                return None;
            }
        }
        Some(output)
    }

    #[cfg(test)]
    fn mapping_count(&self) -> usize {
        self.mappings.len()
    }
}

/// Build a deterministic CID -> Unicode map from an embedded TrueType cmap.
///
/// This is only valid for Identity-H/Identity-V Type0 fonts whose descendant
/// CIDFontType2 uses CIDToGIDMap=Identity.  In that narrow case a content
/// stream CID is the TrueType glyph id.  We do not expose a generic
/// "CID-as-Unicode" fallback because that would silently corrupt legal text.
fn build_identity_cid_cmap(font_data: &[u8]) -> Option<ToUnicodeCMap> {
    let face = ttf_parser::Face::parse(font_data, 0).ok()?;
    let cmap_table = face.tables().cmap?;
    let mut gid_to_unicode = BTreeMap::<u16, char>::new();
    for subtable in cmap_table.subtables {
        let is_windows_symbol =
            subtable.platform_id == ttf_parser::PlatformId::Windows && subtable.encoding_id == 0;
        if !subtable.is_unicode() && !is_windows_symbol {
            continue;
        }
        subtable.codepoints(|codepoint| {
            let Some(gid) = subtable.glyph_index(codepoint) else {
                return;
            };
            let Some(character) = char::from_u32(codepoint) else {
                return;
            };
            // Prefer the first mapping. Duplicate Unicode aliases are normal
            // in fonts and do not provide a better answer for a CID.
            gid_to_unicode.entry(gid.0).or_insert(character);
        });
    }
    if gid_to_unicode.is_empty() {
        return None;
    }

    let mappings = gid_to_unicode
        .into_iter()
        .map(|(gid, character)| (gid.to_be_bytes().to_vec(), character.to_string()))
        .collect();
    Some(ToUnicodeCMap {
        code_spaces: vec![CodeSpaceRange {
            start: vec![0, 0],
            end: vec![u8::MAX, u8::MAX],
        }],
        mappings,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Hex(Vec<u8>),
    Word(String),
    OpenBracket,
    CloseBracket,
}

fn tokenize(data: &[u8]) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < data.len() {
        match data[index] {
            b'%' => {
                while index < data.len() && data[index] != b'\n' && data[index] != b'\r' {
                    index += 1;
                }
            }
            byte if byte.is_ascii_whitespace() => index += 1,
            b'<' if data.get(index + 1) == Some(&b'<') => {
                tokens.push(Token::Word("<<".to_string()));
                index += 2;
            }
            b'<' => {
                let start = index + 1;
                let end = data[start..]
                    .iter()
                    .position(|byte| *byte == b'>')
                    .map(|position| start + position)
                    .context("ToUnicode CMap 的十六进制字符串缺少结束符")?;
                tokens.push(Token::Hex(decode_hex(&data[start..end])?));
                index = end + 1;
            }
            b'[' => {
                tokens.push(Token::OpenBracket);
                index += 1;
            }
            b']' => {
                tokens.push(Token::CloseBracket);
                index += 1;
            }
            _ => {
                let start = index;
                while index < data.len()
                    && !data[index].is_ascii_whitespace()
                    && !matches!(data[index], b'<' | b'>' | b'[' | b']' | b'%')
                {
                    index += 1;
                }
                if start == index {
                    index += 1;
                } else {
                    tokens.push(Token::Word(
                        String::from_utf8_lossy(&data[start..index]).into_owned(),
                    ));
                }
            }
        }
    }
    Ok(tokens)
}

fn decode_hex(value: &[u8]) -> Result<Vec<u8>> {
    let mut digits = value
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if digits.len() % 2 != 0 {
        digits.push(b'0');
    }
    let mut output = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = hex_digit(pair[0]).context("ToUnicode CMap 包含非法十六进制字符")?;
        let low = hex_digit(pair[1]).context("ToUnicode CMap 包含非法十六进制字符")?;
        output.push(high << 4 | low);
    }
    Ok(output)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_code_spaces(
    tokens: &[Token],
    mut index: usize,
    output: &mut Vec<CodeSpaceRange>,
) -> Result<usize> {
    while index < tokens.len() {
        if is_word(&tokens[index], "endcodespacerange") {
            return Ok(index + 1);
        }
        let start = expect_hex(tokens, index, "codespace 起始值")?;
        let end = expect_hex(tokens, index + 1, "codespace 结束值")?;
        if start.is_empty() || start.len() != end.len() || start > end {
            anyhow::bail!("ToUnicode CMap 的 codespace 范围无效")
        }
        output.push(CodeSpaceRange { start, end });
        index += 2;
    }
    anyhow::bail!("ToUnicode CMap 缺少 endcodespacerange")
}

fn parse_bfchar(
    tokens: &[Token],
    mut index: usize,
    output: &mut BTreeMap<Vec<u8>, String>,
) -> Result<usize> {
    while index < tokens.len() {
        if is_word(&tokens[index], "endbfchar") {
            return Ok(index + 1);
        }
        let source = expect_hex(tokens, index, "bfchar 源码")?;
        let destination = expect_hex(tokens, index + 1, "bfchar Unicode 目标")?;
        output.insert(source, decode_unicode(&destination)?);
        index += 2;
    }
    anyhow::bail!("ToUnicode CMap 缺少 endbfchar")
}

fn parse_bfrange(
    tokens: &[Token],
    mut index: usize,
    output: &mut BTreeMap<Vec<u8>, String>,
) -> Result<usize> {
    while index < tokens.len() {
        if is_word(&tokens[index], "endbfrange") {
            return Ok(index + 1);
        }
        let start = expect_hex(tokens, index, "bfrange 起始值")?;
        let end = expect_hex(tokens, index + 1, "bfrange 结束值")?;
        if start.len() != end.len() || start > end {
            anyhow::bail!("ToUnicode CMap 的 bfrange 源码范围无效")
        }
        match tokens.get(index + 2) {
            Some(Token::OpenBracket) => {
                let mut source = start.clone();
                let mut cursor = index + 3;
                while cursor < tokens.len() && source <= end {
                    if matches!(tokens[cursor], Token::CloseBracket) {
                        break;
                    }
                    let destination = expect_hex(tokens, cursor, "bfrange 列表 Unicode 目标")?;
                    output.insert(source.clone(), decode_unicode(&destination)?);
                    increment_bytes(&mut source)?;
                    cursor += 1;
                }
                if !matches!(tokens.get(cursor), Some(Token::CloseBracket)) {
                    anyhow::bail!("ToUnicode CMap 的 bfrange 列表缺少结束符")
                }
                index = cursor + 1;
            }
            Some(Token::Hex(destination)) => {
                let mut source = start.clone();
                let mut number = numeric_value(&start);
                let end_number = numeric_value(&end);
                while number <= end_number {
                    output.insert(
                        source.clone(),
                        increment_unicode(
                            decode_unicode(destination)?,
                            number - numeric_value(&start),
                        )?,
                    );
                    if source == end {
                        break;
                    }
                    increment_bytes(&mut source)?;
                    number += 1;
                }
                index += 3;
            }
            _ => anyhow::bail!("ToUnicode CMap 的 bfrange 目标无效"),
        }
    }
    anyhow::bail!("ToUnicode CMap 缺少 endbfrange")
}

fn expect_hex(tokens: &[Token], index: usize, label: &str) -> Result<Vec<u8>> {
    match tokens.get(index) {
        Some(Token::Hex(value)) => Ok(value.clone()),
        _ => anyhow::bail!("ToUnicode CMap 缺少 {label}"),
    }
}

fn is_word(token: &Token, expected: &str) -> bool {
    matches!(token, Token::Word(value) if value == expected)
}

fn numeric_value(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0_u64, |value, byte| {
        value.saturating_mul(256).saturating_add(u64::from(*byte))
    })
}

fn increment_bytes(bytes: &mut [u8]) -> Result<()> {
    for byte in bytes.iter_mut().rev() {
        if *byte < u8::MAX {
            *byte += 1;
            return Ok(());
        }
        *byte = 0;
    }
    anyhow::bail!("ToUnicode CMap 源码范围溢出")
}

fn between(value: &[u8], start: &[u8], end: &[u8]) -> bool {
    value.len() == start.len() && value >= start && value <= end
}

fn decode_unicode(bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() {
        anyhow::bail!("ToUnicode CMap 目标为空")
    }
    if bytes.len() == 1 {
        return char::from_u32(u32::from(bytes[0]))
            .map(|value| value.to_string())
            .context("ToUnicode CMap 单字节目标无效");
    }
    if !bytes.len().is_multiple_of(2) {
        anyhow::bail!("ToUnicode CMap 目标不是完整的 UTF-16BE 字节序列")
    }
    let units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).context("ToUnicode CMap 目标不是有效的 UTF-16BE")
}

fn increment_unicode(value: String, offset: u64) -> Result<String> {
    if offset == 0 {
        return Ok(value);
    }
    let mut chars = value.chars().collect::<Vec<_>>();
    let last = chars.pop().context("ToUnicode CMap bfrange 目标为空")?;
    let next = (last as u32)
        .checked_add(u32::try_from(offset).context("ToUnicode CMap bfrange 偏移过大")?)
        .and_then(char::from_u32)
        .context("ToUnicode CMap bfrange 目标超出 Unicode 范围")?;
    chars.push(next);
    Ok(chars.into_iter().collect())
}

/// Load all usable ToUnicode maps for font objects referenced by the indexed PDF.
pub(crate) fn load_font_cmaps(
    input: &Path,
    index: &QpdfObjectIndex,
) -> Result<BTreeMap<String, ToUnicodeCMap>> {
    let font_refs = index.font_references();
    let cmap_refs = index.to_unicode_stream_references(&font_refs);
    let embedded_font_refs = index.identity_cid_font_stream_references(&font_refs);
    if cmap_refs.is_empty() && embedded_font_refs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let stream_refs = cmap_refs
        .values()
        .chain(embedded_font_refs.values())
        .cloned()
        .collect::<BTreeSet<_>>();
    let streams = qpdf_stream::load_raw_streams(
        input,
        &stream_refs.into_iter().collect::<Vec<_>>(),
        "读取 ToUnicode CMap",
    )?;
    let mut maps = BTreeMap::new();
    for (font_ref, cmap_ref) in cmap_refs {
        let Some(bytes) = streams.get(&cmap_ref) else {
            continue;
        };
        match ToUnicodeCMap::parse(bytes) {
            Ok(cmap) => {
                maps.insert(font_ref, cmap);
            }
            Err(error) => {
                log::debug!("跳过无法解析的 ToUnicode CMap {}: {}", cmap_ref, error);
            }
        }
    }

    // Only use the embedded-font fallback when the PDF did not yield a
    // usable ToUnicode map.  A valid ToUnicode map remains authoritative.
    for (font_ref, stream_ref) in embedded_font_refs {
        if maps.contains_key(&font_ref) {
            continue;
        }
        let Some(font_data) = streams.get(&stream_ref) else {
            continue;
        };
        if let Some(cmap) = build_identity_cid_cmap(font_data) {
            log::debug!(
                "从嵌入 TrueType cmap 建立 Identity CID 映射 font={} entries={}",
                font_ref,
                cmap.mappings.len()
            );
            maps.insert(font_ref, cmap);
        }
    }
    Ok(maps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_bfchar_and_bfrange() {
        let cmap = ToUnicodeCMap::parse(
            br#"
            1 begincodespacerange
            <0000> <00ff>
            endcodespacerange
            2 beginbfchar
            <0001> <4E2D>
            <0002> <6587>
            endbfchar
            1 beginbfrange
            <0041> <0043> <0061>
            endbfrange
            "#,
        )
        .unwrap();
        assert_eq!(cmap.mapping_count(), 5);
        assert_eq!(cmap.decode(&[0, 1, 0, 2]), Some("中文".to_string()));
        assert_eq!(cmap.decode(&[0, 0x41, 0, 0x43]), Some("ac".to_string()));
        assert_eq!(cmap.decode(&[0, 9]), None);
    }

    #[test]
    fn decodes_variable_length_codespaces_and_range_lists() {
        let cmap = ToUnicodeCMap::parse(
            br#"
            2 begincodespacerange
            <00> <7f>
            <8100> <81ff>
            endcodespacerange
            2 beginbfchar
            <41> <0041>
            <8101> <4E2D>
            endbfchar
            1 beginbfrange
            <42> <44> [<4E00> <4E01> <4E02>]
            endbfrange
            "#,
        )
        .unwrap();
        assert_eq!(
            cmap.decode(&[0x41, 0x42, 0x81, 0x01]),
            Some("A一中".to_string())
        );
        assert_eq!(cmap.decode(&[0x41, 0x82]), None);
    }

    #[test]
    fn rejects_incomplete_or_missing_mappings() {
        let cmap = ToUnicodeCMap::parse(
            br#"
            1 begincodespacerange <00> <ff> endcodespacerange
            1 beginbfchar <01> <0041> endbfchar
            "#,
        )
        .unwrap();
        assert_eq!(cmap.decode(&[0x01, 0x02]), None);
        assert_eq!(cmap.decode(&[0x01]), Some("A".to_string()));
    }

    #[test]
    fn builds_identity_cid_map_with_two_byte_codes() {
        let mappings = [(1_u16, '中'), (2_u16, '文')]
            .into_iter()
            .map(|(gid, character)| (gid.to_be_bytes().to_vec(), character.to_string()))
            .collect();
        let cmap = ToUnicodeCMap {
            code_spaces: vec![CodeSpaceRange {
                start: vec![0, 0],
                end: vec![u8::MAX, u8::MAX],
            }],
            mappings,
        };
        assert_eq!(cmap.decode(&[0, 1, 0, 2]), Some("中文".to_string()));
        assert_eq!(cmap.decode(&[0, 3]), None);
    }

    #[test]
    fn invalid_embedded_font_is_not_guessed() {
        assert!(build_identity_cid_cmap(b"not-a-font").is_none());
    }
}
