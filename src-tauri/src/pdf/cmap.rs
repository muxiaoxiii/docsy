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

static BUILTIN_CMAPS: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/external/bcmaps");

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
        for gid in 0..face.number_of_glyphs() {
            let glyph_id = ttf_parser::GlyphId(gid);
            let Some(name) = face.glyph_name(glyph_id) else {
                continue;
            };
            if let Some(character) = super::glyph_names::glyph_to_char(name) {
                gid_to_unicode.entry(gid).or_insert(character);
            }
        }
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
    let predefined_font_info = index.predefined_font_info(&font_refs);
    if cmap_refs.is_empty() && embedded_font_refs.is_empty() && predefined_font_info.is_empty() {
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

    // Finally use the PDF specification's predefined CMap chain.  This is
    // deliberately last: an explicit ToUnicode map or a deterministic
    // embedded-font map has stronger evidence.  A predefined map is accepted
    // only when both halves of the chain are present and parse completely.
    for (font_ref, info) in predefined_font_info {
        if maps.contains_key(&font_ref) {
            continue;
        }
        if let Some(cmap) = build_predefined_font_cmap(&info.encoding, &info.ordering) {
            log::debug!(
                "从 PDF 预定义 CMap 建立字体映射 font={} entries={}",
                font_ref,
                cmap.mappings.len()
            );
            maps.insert(font_ref, cmap);
        }
    }
    Ok(maps)
}

fn build_predefined_font_cmap(
    encoding: &Option<String>,
    ordering: &Option<String>,
) -> Option<ToUnicodeCMap> {
    let encoding_name = encoding.as_deref()?;

    let ordering = ordering.as_deref()?;
    let cid_to_unicode = load_builtin_tounicode_cmap(&format!("Adobe-{ordering}-UCS2"))?;
    if matches!(encoding_name, "Identity-H" | "Identity-V") {
        return Some(cid_to_unicode);
    }
    let code_to_cid = load_builtin_encoding_cmap(encoding_name)?;
    compose_encoding_and_unicode(code_to_cid, cid_to_unicode)
}

fn load_builtin_tounicode_cmap(name: &str) -> Option<ToUnicodeCMap> {
    let data = BUILTIN_CMAPS.get_file(format!("{name}.bcmap"))?.contents();
    let mut cmap = match parse_binary_tounicode_cmap(data) {
        Ok(cmap) => cmap,
        Err(error) => {
            log::debug!("跳过无法解析的预定义 CMap {name}: {error:#}");
            return None;
        }
    };
    if cmap.mappings.is_empty() || cmap.code_spaces.is_empty() {
        return None;
    }
    cmap.code_spaces
        .sort_by_key(|range| std::cmp::Reverse(range.start.len()));
    Some(cmap)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EncodingCMap {
    mappings: BTreeMap<Vec<u8>, Vec<u8>>,
    code_byte_length: usize,
}

fn load_builtin_encoding_cmap(name: &str) -> Option<EncodingCMap> {
    let data = BUILTIN_CMAPS.get_file(format!("{name}.bcmap"))?.contents();
    parse_binary_encoding_cmap(data).ok()
}

fn compose_encoding_and_unicode(
    encoding: EncodingCMap,
    cid_to_unicode: ToUnicodeCMap,
) -> Option<ToUnicodeCMap> {
    let mut mappings = BTreeMap::new();
    for (code, cid_bytes) in encoding.mappings {
        let unicode = cid_to_unicode.decode(&cid_bytes)?;
        mappings.insert(code, unicode);
    }
    if mappings.is_empty() {
        return None;
    }
    Some(ToUnicodeCMap {
        code_spaces: vec![CodeSpaceRange {
            start: vec![0; encoding.code_byte_length],
            end: vec![u8::MAX; encoding.code_byte_length],
        }],
        mappings,
    })
}

fn parse_binary_tounicode_cmap(data: &[u8]) -> Result<ToUnicodeCMap> {
    let mut stream = BinaryCMapStream::new(data);
    stream.read_byte().context("bcmap 缺少头部")?;
    let mut cmap = ToUnicodeCMap {
        code_spaces: Vec::new(),
        mappings: BTreeMap::new(),
    };
    let mut use_cmap = None;
    while let Some(command) = stream.read_byte() {
        if command >> 5 == 7 {
            match command & 0x1f {
                0 => {
                    let _ = stream.read_string()?;
                }
                1 => use_cmap = Some(stream.read_string()?),
                _ => {}
            }
            continue;
        }
        let kind = command >> 5;
        let sequence = command & 0x10 != 0;
        let data_size = usize::from(command & 0x0f);
        let count = stream.read_number()? as usize;
        match kind {
            0 => {
                for _ in 0..count {
                    let start = stream.read_hex_fixed(data_size)?;
                    let mut end = stream.read_hex_number(data_size)?;
                    add_big_endian_wrap(&mut end, &start);
                    cmap.code_spaces.push(CodeSpaceRange { start, end });
                }
            }
            4 => {
                let mut source = stream.read_hex_fixed(1)?;
                let mut destination = stream.read_hex_fixed(data_size)?;
                cmap.mappings.insert(
                    source.clone(),
                    decode_binary_unicode(&destination).context("bcmap bfchar Unicode 目标无效")?,
                );
                for _ in 1..count {
                    increment_fixed(&mut source)?;
                    if !sequence {
                        let delta = stream.read_hex_number(1)?;
                        add_big_endian_wrap(&mut source, &delta);
                    }
                    increment_fixed(&mut destination)?;
                    let delta = stream.read_hex_signed(data_size)?;
                    add_big_endian_wrap(&mut destination, &delta);
                    cmap.mappings.insert(
                        source.clone(),
                        decode_binary_unicode(&destination)
                            .context("bcmap bfchar Unicode 目标无效")?,
                    );
                }
            }
            5 => {
                let mut start = stream.read_hex_fixed(1)?;
                let mut end = stream.read_hex_number(1)?;
                add_big_endian_wrap(&mut end, &start);
                let destination = stream.read_hex_fixed(data_size)?;
                add_binary_bfrange(&mut cmap.mappings, &start, &end, &destination)?;
                for _ in 1..count {
                    increment_fixed(&mut end)?;
                    if !sequence {
                        start = stream.read_hex_number(1)?;
                        add_big_endian_wrap(&mut start, &end);
                    } else {
                        start = end.clone();
                    }
                    let mut next_end = stream.read_hex_number(1)?;
                    add_big_endian_wrap(&mut next_end, &start);
                    end = next_end;
                    let destination = stream.read_hex_fixed(data_size)?;
                    add_binary_bfrange(&mut cmap.mappings, &start, &end, &destination)?;
                }
            }
            1 => {
                for _ in 0..count {
                    let _ = stream.read_hex_fixed(data_size)?;
                    let _ = stream.read_hex_number(data_size)?;
                    let _ = stream.read_number()?;
                }
            }
            _ => anyhow::bail!("bcmap ToUnicode 包含不支持的类型 {kind}"),
        }
    }
    if cmap.code_spaces.is_empty() {
        cmap.code_spaces.push(CodeSpaceRange {
            start: vec![0, 0],
            end: vec![u8::MAX, u8::MAX],
        });
    }
    if let Some(name) = use_cmap {
        if let Some(base) = load_builtin_tounicode_cmap(&name) {
            cmap = merge_tounicode_cmaps(base, cmap);
        }
    }
    Ok(cmap)
}

fn decode_binary_unicode(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units)
        .ok()
        .filter(|text| !text.is_empty())
}

fn add_binary_bfrange(
    mappings: &mut BTreeMap<Vec<u8>, String>,
    start: &[u8],
    end: &[u8],
    destination: &[u8],
) -> Result<()> {
    let base = decode_binary_unicode(destination).context("bcmap bfrange Unicode 目标无效")?;
    let mut source = start.to_vec();
    let mut offset = 0_u64;
    loop {
        mappings.insert(source.clone(), increment_unicode(base.clone(), offset)?);
        if source == end {
            break;
        }
        increment_fixed(&mut source)?;
        offset += 1;
    }
    Ok(())
}

fn merge_tounicode_cmaps(mut base: ToUnicodeCMap, overlay: ToUnicodeCMap) -> ToUnicodeCMap {
    base.code_spaces.extend(overlay.code_spaces);
    base.mappings.extend(overlay.mappings);
    base
}

fn parse_binary_encoding_cmap(data: &[u8]) -> Result<EncodingCMap> {
    let mut stream = BinaryCMapStream::new(data);
    stream.read_byte().context("bcmap 缺少头部")?;
    let mut mappings = BTreeMap::new();
    let mut code_byte_length = 1_usize;
    let mut use_cmap = None;
    while let Some(command) = stream.read_byte() {
        if command >> 5 == 7 {
            match command & 0x1f {
                0 => {
                    let _ = stream.read_string()?;
                }
                1 => use_cmap = Some(stream.read_string()?),
                _ => {}
            }
            continue;
        }
        let kind = command >> 5;
        let sequence = command & 0x10 != 0;
        let data_size = usize::from(command & 0x0f);
        code_byte_length = code_byte_length.max(data_size + 1);
        let count = stream.read_number()? as usize;
        match kind {
            2 => {
                let mut code = stream.read_hex_fixed(data_size)?;
                let mut cid = stream.read_number()? as i64;
                if !(0..=u16::MAX as i64).contains(&cid) {
                    anyhow::bail!("bcmap CID 超出范围")
                }
                mappings.insert(code.clone(), encode_number(cid as u32, 2));
                for _ in 1..count {
                    increment_fixed(&mut code)?;
                    if !sequence {
                        let delta = stream.read_hex_number(data_size)?;
                        add_big_endian_wrap(&mut code, &delta);
                    }
                    cid += i64::from(stream.read_signed()?) + 1;
                    if !(0..=u16::MAX as i64).contains(&cid) {
                        anyhow::bail!("bcmap CID 超出范围")
                    }
                    mappings.insert(code.clone(), encode_number(cid as u32, 2));
                }
            }
            3 => {
                let mut start = stream.read_hex_fixed(data_size)?;
                let mut end = stream.read_hex_number(data_size)?;
                add_big_endian_wrap(&mut end, &start);
                let mut cid = stream.read_number()? as u16;
                add_binary_cid_range(&mut mappings, &start, &end, cid)?;
                for _ in 1..count {
                    increment_fixed(&mut end)?;
                    if !sequence {
                        start = stream.read_hex_number(data_size)?;
                        add_big_endian_wrap(&mut start, &end);
                    } else {
                        start = end.clone();
                    }
                    let mut next_end = stream.read_hex_number(data_size)?;
                    add_big_endian_wrap(&mut next_end, &start);
                    end = next_end;
                    cid = stream.read_number()? as u16;
                    add_binary_cid_range(&mut mappings, &start, &end, cid)?;
                }
            }
            0 | 1 => {
                for _ in 0..count {
                    let _ = stream.read_hex_fixed(data_size)?;
                    let _ = stream.read_hex_number(data_size)?;
                    if kind == 1 {
                        let _ = stream.read_number()?;
                    }
                }
            }
            _ => anyhow::bail!("bcmap 编码包含不支持的类型 {kind}"),
        }
    }
    if let Some(name) = use_cmap {
        if let Some(base) = load_builtin_encoding_cmap(&name) {
            let mut merged = base.mappings;
            merged.extend(mappings);
            return Ok(EncodingCMap {
                mappings: merged,
                code_byte_length: code_byte_length.max(base.code_byte_length),
            });
        }
    }
    if mappings.is_empty() {
        anyhow::bail!("bcmap 编码映射为空")
    }
    Ok(EncodingCMap {
        mappings,
        code_byte_length,
    })
}

fn add_binary_cid_range(
    mappings: &mut BTreeMap<Vec<u8>, Vec<u8>>,
    start: &[u8],
    end: &[u8],
    mut cid: u16,
) -> Result<()> {
    let mut code = start.to_vec();
    loop {
        mappings.insert(code.clone(), encode_number(u32::from(cid), 2));
        if code == end {
            break;
        }
        increment_fixed(&mut code)?;
        cid = cid.checked_add(1).context("bcmap CID 溢出")?;
    }
    Ok(())
}

fn encode_number(value: u32, width: usize) -> Vec<u8> {
    let mut output = vec![0; width];
    let bytes = value.to_be_bytes();
    let start = bytes.len().saturating_sub(width);
    output.copy_from_slice(&bytes[start..]);
    output
}

fn add_big_endian_wrap(left: &mut [u8], right: &[u8]) {
    if left.len() != right.len() {
        return;
    }
    let mut carry = 0_u16;
    for index in (0..left.len()).rev() {
        let value = u16::from(left[index]) + u16::from(right[index]) + carry;
        left[index] = value as u8;
        carry = value >> 8;
    }
}

fn increment_fixed(bytes: &mut [u8]) -> Result<()> {
    for byte in bytes.iter_mut().rev() {
        if *byte < u8::MAX {
            *byte += 1;
            return Ok(());
        }
        *byte = 0;
    }
    anyhow::bail!("bcmap 数值溢出")
}

struct BinaryCMapStream<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BinaryCMapStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn read_byte(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.position)?;
        self.position += 1;
        Some(byte)
    }

    fn read_hex_fixed(&mut self, width_minus_one: usize) -> Result<Vec<u8>> {
        self.read_bytes(width_minus_one + 1)
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        let end = self
            .position
            .checked_add(length)
            .context("bcmap 数据长度溢出")?;
        let value = self
            .data
            .get(self.position..end)
            .context("bcmap 数据提前结束")?
            .to_vec();
        self.position = end;
        Ok(value)
    }

    fn read_number(&mut self) -> Result<u32> {
        let mut value = 0_u32;
        loop {
            let byte = self.read_byte().context("bcmap 数字提前结束")?;
            value = value
                .checked_shl(7)
                .and_then(|value| value.checked_add(u32::from(byte & 0x7f)))
                .context("bcmap 数字溢出")?;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
    }

    fn read_hex_number(&mut self, width_minus_one: usize) -> Result<Vec<u8>> {
        let mut chunks = Vec::new();
        loop {
            let byte = self.read_byte().context("bcmap 十六进制数字提前结束")?;
            chunks.push(byte & 0x7f);
            if byte & 0x80 == 0 {
                break;
            }
        }
        let width = width_minus_one + 1;
        let mut output = vec![0; width];
        let mut buffer = 0_u32;
        let mut buffered_bits = 0_u32;
        for index in (0..width).rev() {
            while buffered_bits < 8 && !chunks.is_empty() {
                let chunk = chunks.pop().unwrap();
                buffer |= u32::from(chunk) << buffered_bits;
                buffered_bits += 7;
            }
            output[index] = buffer as u8;
            buffer >>= 8;
            buffered_bits = buffered_bits.saturating_sub(8);
        }
        Ok(output)
    }

    fn read_hex_signed(&mut self, width_minus_one: usize) -> Result<Vec<u8>> {
        let mut value = self.read_hex_number(width_minus_one)?;
        let sign = value.last().copied().unwrap_or_default() & 1 != 0;
        let mut carry = 0_u16;
        for byte in &mut value {
            let current = (carry << 8) | u16::from(*byte);
            *byte = (current >> 1) as u8 ^ if sign { u8::MAX } else { 0 };
            carry = current & 1;
        }
        Ok(value)
    }

    fn read_signed(&mut self) -> Result<i32> {
        let value = self.read_number()?;
        let magnitude = (value >> 1) as i32;
        Ok(if value & 1 == 0 {
            magnitude
        } else {
            !magnitude
        })
    }

    fn read_string(&mut self) -> Result<String> {
        let length = self.read_number()? as usize;
        let bytes = (0..length)
            .map(|_| self.read_number().map(|value| value as u8))
            .collect::<Result<Vec<_>>>()?;
        String::from_utf8(bytes).context("bcmap 名称不是 UTF-8")
    }
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

    #[test]
    fn loads_embedded_adobe_ucs2_maps() {
        let cmap = load_builtin_tounicode_cmap("Adobe-GB1-UCS2").unwrap();
        assert!(cmap.mapping_count() > 100);
        assert_eq!(cmap.code_spaces.len(), 1);
    }

    #[test]
    fn composes_predefined_encoding_and_ucs2_maps() {
        let cmap =
            build_predefined_font_cmap(&Some("GB-EUC-H".to_string()), &Some("GB1".to_string()))
                .unwrap();
        assert!(cmap.mapping_count() > 100);
    }

    #[test]
    fn glyph_name_mapping_is_available_for_font_fallbacks() {
        assert_eq!(
            super::super::glyph_names::glyph_to_char("Aacute"),
            Some('Á')
        );
        assert_eq!(
            super::super::glyph_names::glyph_to_char("uni4E2D"),
            Some('中')
        );
    }

    #[test]
    fn bundled_cmap_resources_are_parseable() {
        let mut parsed = 0;
        let mut failures = Vec::new();
        for file in BUILTIN_CMAPS.files() {
            if file.path().extension().and_then(|value| value.to_str()) != Some("bcmap") {
                continue;
            }
            let data = file.contents();
            if parse_binary_tounicode_cmap(data).is_ok() || parse_binary_encoding_cmap(data).is_ok()
            {
                parsed += 1;
            } else {
                failures.push(file.path().display().to_string());
            }
        }
        assert!(failures.is_empty(), "无法解析的内置 CMap: {failures:?}");
        assert!(parsed >= 160, "内置 CMap 资源数量异常: {parsed}");
    }
}
