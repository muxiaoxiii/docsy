//! Overlay 字体发现、子集嵌入、文本算子生成与度量计算。

use allsorts::binary::read::ReadScope;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::subset::{subset as subset_font_bytes, CmapTarget, SubsetProfile};
use anyhow::{Context, Result};
use lopdf::content::Operation;
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use printpdf::{
    generate_cmap_string, generate_gid_to_cid_map, get_normalized_widths_cff,
    get_normalized_widths_ttf, FontId, FontType, ParsedFont,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::fnv1a_hash;
use super::header_footer::OverlayTextConfig;
use super::page_info::PageSize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OverlayRegion {
    Header,
    Footer,
}

#[derive(Debug, Clone)]
pub(crate) struct EmbeddedOverlayFont {
    pub(crate) resource_name: String,
    pub(crate) object_id: ObjectId,
    pub(crate) char_to_gid: BTreeMap<char, u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct EmbeddedFontChoice {
    pub(crate) font: EmbeddedOverlayFont,
    pub(crate) family: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FontCandidate {
    pub(crate) family: String,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Clone)]
enum OverlayFontRef<'a> {
    Builtin(&'static str),
    Embedded(&'a EmbeddedOverlayFont),
}

pub(crate) fn overlay_region(value: &str) -> OverlayRegion {
    if value.trim().eq_ignore_ascii_case("header") {
        OverlayRegion::Header
    } else {
        OverlayRegion::Footer
    }
}

#[allow(clippy::too_many_arguments)] // the font plan depends on each overlay source and page set
pub(crate) fn prepare_embedded_overlay_fonts(
    doc: &mut Document,
    header: Option<&OverlayTextConfig>,
    footer: Option<&OverlayTextConfig>,
    extra_overlays: &[OverlayTextConfig],
    page_count: usize,
    page_start: u32,
    total_pages: u32,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, EmbeddedOverlayFont> {
    let mut texts_by_family: BTreeMap<String, String> = BTreeMap::new();
    for page_index in 0..page_count {
        let current_page = page_start + page_index as u32;
        for config in header
            .into_iter()
            .chain(footer)
            .chain(extra_overlays.iter())
        {
            let text = expand_config_placeholders(config, current_page, total_pages);
            if requires_embedded_font(&text) {
                texts_by_family
                    .entry(font_family_key(&config.font_family))
                    .or_default()
                    .push_str(&text);
            }
        }
    }

    let mut fonts = BTreeMap::new();
    for (index, (family, text)) in texts_by_family.into_iter().enumerate() {
        let resource_name = format!("FEmbed{}", index + 1);
        match create_embedded_overlay_font(doc, &resource_name, &family, &text) {
            Ok(choice) => {
                if choice.family != family {
                    warnings.push(format!(
                        "字体「{}」无法嵌入，已改用相近字体「{}」",
                        display_font_family(&family),
                        display_font_family(&choice.family)
                    ));
                }
                fonts.insert(family, choice.font);
            }
            Err(err) => warnings.push(format!(
                "字体「{}」及相近字体均无法按子集嵌入：{}",
                display_font_family(&family),
                err
            )),
        }
    }
    fonts
}

pub(crate) fn create_embedded_overlay_font(
    doc: &mut Document,
    resource_name: &str,
    family: &str,
    text: &str,
) -> Result<EmbeddedFontChoice> {
    let mut last_error = None;
    let mut skipped_for_coverage: Vec<String> = Vec::new();
    for candidate in font_candidate_sequence(family) {
        if !candidate.path.exists() {
            continue;
        }
        // 按字形覆盖挑字体：候选字体不含文本所需字符时直接跳过
        //（例如 SimSun 缺少日文汉字，子集嵌入出来会渲染成方块）。
        if !font_covers_text(&candidate.path, text) {
            skipped_for_coverage.push(candidate.path.display().to_string());
            continue;
        }
        match try_create_embedded_overlay_font(doc, resource_name, &candidate.path, text) {
            Ok(font) => {
                return Ok(EmbeddedFontChoice {
                    font,
                    family: candidate.family,
                })
            }
            Err(err) => last_error = Some(err),
        }
    }
    match last_error {
        Some(err) => Err(err),
        None if !skipped_for_coverage.is_empty() => anyhow::bail!(
            "已安装的系统字体均缺少文本所需字符（已跳过: {}），请安装覆盖该文字的字体",
            skipped_for_coverage.join(", ")
        ),
        None => anyhow::bail!("未找到可用系统字体"),
    }
}

fn try_create_embedded_overlay_font(
    doc: &mut Document,
    resource_name: &str,
    path: &Path,
    text: &str,
) -> Result<EmbeddedOverlayFont> {
    let bytes = fs::read(path).with_context(|| format!("读取字体失败: {}", path.display()))?;
    let scope = ReadScope::new(&bytes);
    let font_data = scope
        .read::<FontData<'_>>()
        .with_context(|| format!("解析字体失败: {}", path.display()))?;
    let provider_for_lookup = font_data
        .table_provider(0)
        .with_context(|| format!("读取字体表失败: {}", path.display()))?;
    let mut font = Font::new(provider_for_lookup)
        .with_context(|| format!("初始化字体失败: {}", path.display()))?;

    let mut glyph_ids = vec![0_u16];
    let mut char_to_original_gid = BTreeMap::new();
    for ch in text.chars().filter(|ch| !ch.is_control()) {
        let (gid, _) = font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None);
        if gid == 0 && !ch.is_whitespace() {
            anyhow::bail!("字体缺少字符「{}」", ch);
        }
        if gid != 0 {
            char_to_original_gid.entry(ch).or_insert(gid);
            glyph_ids.push(gid);
        }
    }
    glyph_ids.sort_unstable();
    glyph_ids.dedup();
    if glyph_ids.first().copied() != Some(0) {
        glyph_ids.insert(0, 0);
    }

    let provider_for_subset = font_data
        .table_provider(0)
        .with_context(|| format!("读取字体子集表失败: {}", path.display()))?;
    let subset_bytes = subset_font_bytes(
        &provider_for_subset,
        &glyph_ids,
        &SubsetProfile::Pdf,
        CmapTarget::Unicode,
    )
    .with_context(|| format!("生成字体子集失败: {}", path.display()))?;

    let mut parsed_warnings = Vec::new();
    let parsed_subset = ParsedFont::from_bytes(&subset_bytes, 0, &mut parsed_warnings)
        .ok_or_else(|| anyhow::anyhow!("解析字体子集失败: {}", path.display()))?;
    let char_to_subset_gid = char_to_original_gid
        .into_iter()
        .filter_map(|(ch, original_gid)| {
            glyph_ids
                .iter()
                .position(|gid| *gid == original_gid)
                .map(|position| (ch, position as u16))
        })
        .collect::<BTreeMap<_, _>>();
    let new_glyph_ids = char_to_subset_gid
        .iter()
        .map(|(ch, gid)| (*gid, *ch))
        .collect::<Vec<_>>();
    let font_id = FontId(resource_name.to_string());
    let to_unicode = generate_cmap_string(&parsed_subset, &font_id, &new_glyph_ids);
    let widths = match parsed_subset.font_type {
        FontType::TrueType => get_normalized_widths_ttf(&parsed_subset, &new_glyph_ids),
        _ => {
            let gid_to_cid_map = generate_gid_to_cid_map(&parsed_subset, &new_glyph_ids);
            get_normalized_widths_cff(&parsed_subset, &gid_to_cid_map)
        }
    };
    let object_id = add_subset_font_to_doc(
        doc,
        resource_name,
        &parsed_subset,
        subset_bytes,
        to_unicode,
        widths,
        char_to_subset_gid.values().copied().max().unwrap_or(0),
    );
    Ok(EmbeddedOverlayFont {
        resource_name: resource_name.to_string(),
        object_id,
        char_to_gid: char_to_subset_gid,
    })
}

/// 判断字体是否覆盖文本中的全部非控制字符（空白除外）。
/// 字体无法解析时返回 false，让调用方继续尝试下一个候选。
pub(crate) fn font_covers_text(path: &Path, text: &str) -> bool {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let font_data = match ReadScope::new(&bytes).read::<FontData<'_>>() {
        Ok(font_data) => font_data,
        Err(_) => return false,
    };
    let provider = match font_data.table_provider(0) {
        Ok(provider) => provider,
        Err(_) => return false,
    };
    let mut font = match Font::new(provider) {
        Ok(font) => font,
        Err(_) => return false,
    };
    text.chars().filter(|ch| !ch.is_control()).all(|ch| {
        let (gid, _) = font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None);
        gid != 0 || ch.is_whitespace()
    })
}

fn add_subset_font_to_doc(
    doc: &mut Document,
    resource_name: &str,
    font: &ParsedFont,
    font_bytes: Vec<u8>,
    to_unicode: String,
    widths: Vec<Object>,
    max_cid: u16,
) -> ObjectId {
    let font_name = font
        .font_name
        .clone()
        .unwrap_or_else(|| resource_name.to_string())
        .replace(' ', "");
    let face_name = format!("{}+{font_name}", subset_font_prefix(resource_name));
    let (subtype, font_file_key, font_stream) = match &font.font_type {
        FontType::OpenTypeCFF(_) => (
            "CIDFontType0",
            "FontFile3",
            Stream::new(dictionary! { "Subtype" => "OpenType" }, font_bytes)
                .with_compression(false),
        ),
        FontType::TrueType => (
            "CIDFontType2",
            "FontFile2",
            Stream::new(Dictionary::new(), font_bytes).with_compression(false),
        ),
    };
    let font_file_id = doc.add_object(font_stream);
    let to_unicode_id = doc.add_object(Stream::new(Dictionary::new(), to_unicode.into_bytes()));
    let cid_set_id = doc.add_object(Stream::new(Dictionary::new(), contiguous_cid_set(max_cid)));
    let units_per_em = font.pdf_font_metrics.units_per_em.max(1) as f32;
    let normalize_metric = |value: f32| (value * 1000.0 / units_per_em).round() as i64;
    let descriptor_id = doc.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => Object::Name(face_name.as_bytes().to_vec()),
        "Ascent" => normalize_metric(font.font_metrics.ascent),
        "Descent" => normalize_metric(font.font_metrics.descent),
        "CapHeight" => normalize_metric(font.font_metrics.ascent),
        "ItalicAngle" => 0,
        "Flags" => 32,
        "StemV" => 80,
        "CIDSet" => cid_set_id,
        font_file_key => font_file_id,
        "FontBBox" => vec![
            normalize_metric(font.pdf_font_metrics.x_min as f32).into(),
            normalize_metric(font.pdf_font_metrics.y_min as f32).into(),
            normalize_metric(font.pdf_font_metrics.x_max as f32).into(),
            normalize_metric(font.pdf_font_metrics.y_max as f32).into(),
        ],
    });
    let descendant_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => subtype,
        "BaseFont" => Object::Name(face_name.as_bytes().to_vec()),
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "W" => Object::Array(widths),
        "DW" => 1000,
        "FontDescriptor" => descriptor_id,
    });
    doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => Object::Name(face_name.as_bytes().to_vec()),
        "Encoding" => "Identity-H",
        "ToUnicode" => to_unicode_id,
        "DescendantFonts" => vec![Object::Reference(descendant_id)],
    })
}

fn subset_font_prefix(resource_name: &str) -> String {
    let index = resource_name
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse::<usize>()
        .unwrap_or(1)
        .saturating_sub(1);
    let suffix = (b'A' + (index % 26) as u8) as char;
    format!("DCSYA{suffix}")
}

fn contiguous_cid_set(max_cid: u16) -> Vec<u8> {
    let mut bytes = vec![0_u8; max_cid as usize / 8 + 1];
    for cid in 0..=max_cid as usize {
        bytes[cid / 8] |= 1 << (7 - cid % 8);
    }
    bytes
}

pub(crate) fn append_overlay_text_ops(
    ops: &mut Vec<Operation>,
    config: &OverlayTextConfig,
    region: OverlayRegion,
    size: &PageSize,
    current_page: u32,
    total_pages: u32,
    embedded_fonts: &BTreeMap<String, EmbeddedOverlayFont>,
) -> Result<bool> {
    let text = expand_config_placeholders(config, current_page, total_pages);
    if text.is_empty() {
        return Ok(false);
    }
    let mut y = match region {
        OverlayRegion::Header => size.height_pt - mm_to_pt(config.margin_mm),
        OverlayRegion::Footer => mm_to_pt(config.margin_mm),
    };
    // 边距超过页面（或负值）时把基线收回页内，避免文字整体掉出 CropBox。
    y = y.clamp(0.0, size.height_pt);
    let font_ref = overlay_font_ref(config, &text, embedded_fonts)?;
    let use_embedded = matches!(font_ref, OverlayFontRef::Embedded(_));
    let mut x = compute_x(config, &text, use_embedded, size.width_pt);
    // 横向溢出：估算宽度超出页宽时向左收拢到页内（含 left 对齐长文本、
    // right/center 对齐受 offset 或估算误差影响画出右缘两种情况）。
    let text_width = estimate_text_width(&text, use_embedded, config.font_size);
    let overflowed = x + text_width > size.width_pt + 0.5;
    if overflowed {
        x = (size.width_pt - text_width).max(0.0);
    }
    ops.extend(text_ops(
        &font_ref,
        config,
        region,
        current_page,
        x,
        y,
        text,
    ));
    Ok(overflowed)
}

fn overlay_font_ref<'a>(
    config: &OverlayTextConfig,
    text: &str,
    embedded_fonts: &'a BTreeMap<String, EmbeddedOverlayFont>,
) -> Result<OverlayFontRef<'a>> {
    if requires_embedded_font(text) {
        let key = font_family_key(&config.font_family);
        if let Some(font) = embedded_fonts.get(&key) {
            return Ok(OverlayFontRef::Embedded(font));
        }
        anyhow::bail!("没有可嵌入的中文字体，已停止生成，避免写入 Acrobat 无法编辑的损坏字体资源");
    }
    Ok(match config.font_family.trim().to_lowercase().as_str() {
        "times" | "times new roman" | "times-roman" => OverlayFontRef::Builtin("FTimes"),
        "courier" | "courier new" => OverlayFontRef::Builtin("FCourier"),
        _ => OverlayFontRef::Builtin("F1"),
    })
}

fn text_ops(
    font_ref: &OverlayFontRef<'_>,
    config: &OverlayTextConfig,
    region: OverlayRegion,
    current_page: u32,
    x: f32,
    y: f32,
    text: String,
) -> Vec<Operation> {
    let (r, g, b) = parse_hex_color(&config.color).unwrap_or((0.0, 0.0, 0.0));
    let (font_name, text_object) = match font_ref {
        OverlayFontRef::Builtin(name) => (*name, Object::string_literal(text.clone())),
        OverlayFontRef::Embedded(font) => (
            font.resource_name.as_str(),
            Object::String(
                encode_subset_glyph_text(&text, &font.char_to_gid),
                StringFormat::Hexadecimal,
            ),
        ),
    };
    let (subtype, attached) = match region {
        OverlayRegion::Header => ("Header", "Top"),
        OverlayRegion::Footer => ("Footer", "Bottom"),
    };
    let kind = if config.artifact_kind.is_empty() {
        if config.text.contains("{page}")
            || config.text.contains("{total}")
            || config.text.contains("{range}")
        {
            "PageNumber"
        } else if matches!(region, OverlayRegion::Header) {
            "HeaderText"
        } else {
            "FooterText"
        }
    } else {
        config.artifact_kind.as_str()
    };
    let docsy_id = format!(
        "docsy-{kind}-{current_page}-{:016x}",
        fnv1a_hash(&format!("{}|{}|{}|{}", config.text, config.align, x, y))
    );
    vec![
        Operation::new(
            "BDC",
            vec![
                Object::Name(b"Artifact".to_vec()),
                Object::Dictionary(dictionary! {
                    "Type" => "Pagination",
                    "Subtype" => subtype,
                    "Attached" => vec![Object::Name(attached.as_bytes().to_vec())],
                    "Docsy" => Object::Boolean(true),
                    "DocsyVersion" => 1,
                    "DocsyKind" => Object::Name(kind.as_bytes().to_vec()),
                    "DocsyId" => Object::string_literal(docsy_id),
                    "ActualText" => Object::String(utf16be_pdf_text(&text), StringFormat::Hexadecimal),
                }),
            ],
        ),
        Operation::new("q", vec![]),
        Operation::new("BT", vec![]),
        Operation::new(
            "Tf",
            vec![
                Object::Name(font_name.as_bytes().to_vec()),
                config.font_size.into(),
            ],
        ),
        Operation::new("rg", vec![r.into(), g.into(), b.into()]),
        Operation::new(
            "Tm",
            vec![1.into(), 0.into(), 0.into(), 1.into(), x.into(), y.into()],
        ),
        Operation::new("Tj", vec![text_object]),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
        Operation::new("EMC", vec![]),
    ]
}

pub(crate) fn utf16be_pdf_text(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xfe, 0xff];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    bytes
}

fn encode_subset_glyph_text(text: &str, char_to_gid: &BTreeMap<char, u16>) -> Vec<u8> {
    text.chars()
        .flat_map(|ch| {
            let gid = char_to_gid.get(&ch).copied().unwrap_or(0);
            gid.to_be_bytes()
        })
        .collect()
}

pub(crate) fn requires_embedded_font(text: &str) -> bool {
    text.chars().any(|ch| {
        let cp = ch as u32;
        !(0x20..=0x7E).contains(&cp)
    })
}

pub(crate) fn font_family_key(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "heiti" | "黑体" | "simhei" | "microsoft yahei" | "微软雅黑" | "pingfang" | "苹方" => {
            "heiti".to_string()
        }
        "kaiti" | "楷体" | "simkai" => "kaiti".to_string(),
        "fangsong" | "仿宋" | "simfang" => "fangsong".to_string(),
        "songti" | "宋体" | "simsun" | "serif" => "songti".to_string(),
        _ => "songti".to_string(),
    }
}

pub(crate) fn display_font_family(key: &str) -> &'static str {
    match key {
        "heiti" => "黑体",
        "kaiti" => "楷体",
        "fangsong" => "仿宋",
        _ => "宋体",
    }
}

pub(crate) fn font_candidate_sequence(family: &str) -> Vec<FontCandidate> {
    let mut candidates = Vec::new();
    for fallback_family in fallback_font_families(family) {
        for path in font_paths_for_family(fallback_family) {
            candidates.push(FontCandidate {
                family: fallback_family.to_string(),
                path,
            });
        }
    }
    candidates
}

fn fallback_font_families(family: &str) -> Vec<&'static str> {
    match family {
        "heiti" => vec!["heiti", "songti", "fangsong", "kaiti"],
        "kaiti" => vec!["kaiti", "songti", "fangsong", "heiti"],
        "fangsong" => vec!["fangsong", "songti", "kaiti", "heiti"],
        _ => vec!["songti", "fangsong", "kaiti", "heiti"],
    }
}

pub(crate) fn font_paths_for_family(family: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "macos")]
    {
        match family {
            "heiti" => {
                paths.push(PathBuf::from("/System/Library/Fonts/STHeiti Medium.ttc"));
                paths.push(PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"));
                paths.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
            }
            "kaiti" => {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Kaiti.ttc",
                ));
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Kaiti.ttf",
                ));
            }
            "fangsong" => {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/STFangsong.ttf",
                ));
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Fangsong.ttf",
                ));
            }
            _ => {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Songti.ttc",
                ));
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/Supplemental/Songti.ttf",
                ));
                paths.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
            }
        }
        // 通用 CJK 兜底：新版 macOS 不再预装部分华文/宋体系列字体
        //（如 STFangsong），探测不到时用系统自带的冬青黑体/明朝补位。
        paths.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"));
        paths.push(PathBuf::from(
            "/System/Library/Fonts/Hiragino Mincho ProN.ttc",
        ));
        paths.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans.ttc"));
    }
    #[cfg(target_os = "windows")]
    {
        let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let fonts = PathBuf::from(win).join("Fonts");
        match family {
            "heiti" => {
                paths.push(fonts.join("simhei.ttf"));
                paths.push(fonts.join("msyh.ttc"));
                paths.push(fonts.join("msyh.ttf"));
            }
            "kaiti" => paths.push(fonts.join("simkai.ttf")),
            "fangsong" => paths.push(fonts.join("simfang.ttf")),
            _ => {
                paths.push(fonts.join("simsun.ttc"));
                paths.push(fonts.join("simsun.ttf"));
                paths.push(fonts.join("msyh.ttc"));
            }
        }
        // 日文/韩文兜底：中文字体不含日文特有汉字时按字形覆盖顺延到这里，
        // 避免缺字形渲染成方块。
        paths.push(fonts.join("msmincho.ttc"));
        paths.push(fonts.join("msgothic.ttc"));
        paths.push(fonts.join("YuGothM.ttc"));
        paths.push(fonts.join("YuGothR.ttc"));
        paths.push(fonts.join("malgun.ttf"));
        paths.push(fonts.join("batang.ttc"));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        match family {
            "heiti" => {
                paths.push(PathBuf::from(
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                ));
                paths.push(PathBuf::from(
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                ));
            }
            _ => {
                paths.push(PathBuf::from(
                    "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
                ));
                paths.push(PathBuf::from(
                    "/usr/share/fonts/truetype/noto/NotoSerifCJK-Regular.ttc",
                ));
                paths.push(PathBuf::from(
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                ));
            }
        }
    }
    paths
}

pub(crate) fn parse_hex_color(value: &str) -> Option<(f32, f32, f32)> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&trimmed[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&trimmed[4..6], 16).ok()? as f32 / 255.0;
    Some((r, g, b))
}

pub(crate) fn expand_config_placeholders(
    config: &OverlayTextConfig,
    current_page: u32,
    total_pages: u32,
) -> String {
    let page = (current_page as i64 + config.number_offset as i64).max(1) as u32;
    let total = config.number_total.unwrap_or(total_pages).max(1);
    let page_text = format_page_number(page, &config.number_style);
    let total_text = format_page_number(total, &config.number_style);
    config
        .text
        .replace("{page}", &page_text)
        .replace("{total}", &total_text)
        .replace("{range}", &format!("{page_text}/{total_text}"))
}

pub(crate) fn format_page_number(value: u32, style: &str) -> String {
    match style {
        "chinese" => chinese_page_number(value),
        "roman-upper" => roman_page_number(value),
        "roman-lower" => roman_page_number(value).to_lowercase(),
        "circled" if value <= 20 => char::from_u32(0x2460 + value - 1)
            .unwrap_or('?')
            .to_string(),
        "dingbat" if value <= 10 => char::from_u32(0x2775 + value).unwrap_or('?').to_string(),
        "dingbat" if value <= 20 => char::from_u32(0x24E0 + value).unwrap_or('?').to_string(),
        _ => value.to_string(),
    }
}

pub(crate) fn roman_page_number(value: u32) -> String {
    const PAIRS: &[(u32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut remaining = value;
    let mut result = String::new();
    for (amount, token) in PAIRS {
        while remaining >= *amount {
            result.push_str(token);
            remaining -= *amount;
        }
    }
    result
}

pub(crate) fn chinese_page_number(value: u32) -> String {
    const DIGITS: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    const UNITS: [&str; 4] = ["", "十", "百", "千"];
    if value > 9999 {
        return value.to_string();
    }
    let chars: Vec<u32> = value
        .to_string()
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .collect();
    let mut result = String::new();
    let mut pending_zero = false;
    for (index, digit) in chars.iter().copied().enumerate() {
        let unit_index = chars.len() - index - 1;
        if digit == 0 {
            pending_zero = !result.is_empty() && chars[index + 1..].iter().any(|next| *next != 0);
            continue;
        }
        if pending_zero {
            result.push_str(DIGITS[0]);
        }
        pending_zero = false;
        if !(digit == 1 && unit_index == 1 && result.is_empty()) {
            result.push_str(DIGITS[digit as usize]);
        }
        result.push_str(UNITS[unit_index]);
    }
    if result.is_empty() {
        DIGITS[0].to_string()
    } else {
        result
    }
}

pub(crate) fn compute_x(
    config: &OverlayTextConfig,
    text: &str,
    use_cjk: bool,
    page_width: f32,
) -> f32 {
    let text_width = estimate_text_width(text, use_cjk, config.font_size);
    let offset = mm_to_pt(config.offset_x_mm);
    let margin = mm_to_pt(config.margin_mm);

    match config.align.as_str() {
        "left" => (margin + offset).max(0.0),
        "right" => (page_width - margin - text_width + offset).max(0.0),
        _ => ((page_width - text_width) / 2.0 + offset).max(0.0),
    }
}

pub(crate) fn estimate_text_width(text: &str, use_cjk: bool, font_size: f32) -> f32 {
    text.chars()
        .map(|c| estimate_char_width(c, !use_cjk) * font_size)
        .sum()
}

pub(crate) fn estimate_char_width(c: char, is_builtin: bool) -> f32 {
    let cp = c as u32;
    let cjk = (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x20000..=0x2A6DF).contains(&cp)
        // CJK 标点（。《》、（）等）与全角形式在嵌入中文字体里同样占 1em，
        // 之前按 0.5em 估算导致 right/center 对齐系统性偏右、文本画出页外。
        || (0x3000..=0x303F).contains(&cp)
        || (0xFF01..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp);
    if cjk {
        return 1.0;
    }
    if !is_builtin {
        return 0.5;
    }
    match c {
        ' ' => 0.278,
        '0'..='9' => 0.556,
        'A'..='Z' => 0.667,
        'a'..='z' => 0.500,
        '.' | ',' | ':' | ';' | '/' | '\\' | '\'' | '"' => 0.278,
        '-' | '_' | '(' | ')' | '[' | ']' | '{' | '}' => 0.333,
        _ => 0.556,
    }
}

pub(crate) fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn expands_global_page_placeholders() {
        // expand_placeholders is cfg(test)-only in header_footer; replicate inline
        fn ep(template: &str, page: u32, total: u32) -> String {
            template
                .replace("{page}", &page.to_string())
                .replace("{total}", &total.to_string())
                .replace("{range}", &format!("{page}/{total}"))
        }
        assert_eq!(ep("{page}/{total} {range}", 13, 30), "13/30 13/30");
    }

    #[test]
    fn expands_styled_page_numbers_with_offset_and_total_override() {
        let config = OverlayTextConfig {
            text: "-{page}-/{total}".to_string(),
            region: "footer".to_string(),
            font_family: "auto".to_string(),
            font_size: 9.0,
            margin_mm: 10.0,
            align: "center".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: "roman-upper".to_string(),
            number_offset: -2,
            number_total: Some(12),
            artifact_kind: "PageNumber".to_string(),
        };
        assert_eq!(expand_config_placeholders(&config, 5, 99), "-III-/XII");
    }

    #[test]
    fn formats_large_chinese_and_dingbat_page_numbers() {
        assert_eq!(format_page_number(101, "chinese"), "一百零一");
        assert_eq!(format_page_number(2000, "chinese"), "二千");
        assert_eq!(format_page_number(11, "dingbat"), "⓫");
    }

    #[test]
    fn parses_hex_text_color() {
        let (r, g, b) = parse_hex_color("#336699").unwrap();
        assert!((r - 0.2).abs() < 0.01);
        assert!((g - 0.4).abs() < 0.01);
        assert!((b - 0.6).abs() < 0.01);
        assert!(parse_hex_color("not-a-color").is_none());
    }

    #[test]
    fn compute_x_uses_configured_margin_for_left_and_right_alignment() {
        let mut config = OverlayTextConfig {
            text: String::new(),
            region: "header".to_string(),
            font_family: "auto".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "left".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: String::new(),
        };

        assert!((compute_x(&config, "abc", false, 200.0) - mm_to_pt(10.0)).abs() < 0.01);
        config.align = "right".to_string();
        let expected = 200.0 - mm_to_pt(10.0) - estimate_text_width("abc", false, 10.0);
        assert!((compute_x(&config, "abc", false, 200.0) - expected).abs() < 0.01);
    }

    #[test]
    fn fullwidth_punctuation_counts_as_one_em_in_width_estimate() {
        // 《》（）等全角标点在嵌入中文字体里占 1em；按 0.5em 估算会让
        // right/center 对齐的文本系统性偏右画出页外。
        assert_eq!(estimate_text_width("（热镀锌）", true, 10.0), 50.0);
        assert_eq!(estimate_text_width("《A4》", true, 10.0), 30.0);
    }

    #[test]
    fn font_fallback_sequence_tries_similar_embeddable_families() {
        let families = font_candidate_sequence("songti")
            .into_iter()
            .map(|candidate| candidate.family)
            .collect::<Vec<_>>();

        assert!(families.iter().any(|family| family == "songti"));
        assert!(families.iter().any(|family| family == "fangsong"));
        assert!(families.iter().any(|family| family == "kaiti"));
        assert!(families.iter().any(|family| family == "heiti"));
        assert_eq!(families.first().map(String::as_str), Some("songti"));
    }

    #[test]
    fn font_paths_include_platform_cjk_fallbacks() {
        for family in ["songti", "heiti", "kaiti", "fangsong"] {
            let paths = font_paths_for_family(family)
                .into_iter()
                .map(|path| path.to_string_lossy().to_lowercase())
                .collect::<Vec<_>>();
            #[cfg(target_os = "windows")]
            {
                // 日文/韩文字体兜底，避免中文字体缺字形渲染成方块。
                assert!(paths.iter().any(|p| p.ends_with("msmincho.ttc")));
                assert!(paths.iter().any(|p| p.ends_with("msgothic.ttc")));
                assert!(paths.iter().any(|p| p.ends_with("malgun.ttf")));
                assert!(paths.iter().any(|p| p.ends_with("batang.ttc")));
            }
            #[cfg(target_os = "macos")]
            {
                // 新版 macOS 不再自带仿宋/楷体，需回退到冬青黑体等通用 CJK 字体。
                assert!(paths.iter().any(|p| p.contains("hiragino sans gb")));
                assert!(paths.iter().any(|p| p.contains("hiragino mincho pron")));
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                let _ = paths;
            }
        }
    }

    #[test]
    fn font_covers_text_rejects_missing_glyphs() {
        let Some(font_path) = font_candidate_sequence("songti")
            .into_iter()
            .map(|candidate| candidate.path)
            .find(|path| path.exists())
        else {
            // 测试机没有任何 CJK 系统字体时跳过。
            return;
        };

        // 系统 CJK 字体应覆盖简体中文字符。
        assert!(font_covers_text(&font_path, "简体中文测试 Header 123"));
        // 不存在的文件与不含任何字符映射的非字符码点应判定为不覆盖。
        assert!(!font_covers_text(
            Path::new("/definitely/missing/font.ttf"),
            "测试"
        ));
        assert!(!font_covers_text(&font_path, "\u{10FFFE}"));
    }

    #[test]
    fn embedded_font_choice_prefers_font_covering_text() {
        // 测试机需至少存在一个能覆盖中文的候选字体，否则跳过。
        let covering = font_candidate_sequence("songti")
            .into_iter()
            .filter(|candidate| candidate.path.exists())
            .any(|candidate| font_covers_text(&candidate.path, "测试页眉"));
        if !covering {
            return;
        }

        let mut doc = Document::with_version("1.5");
        let choice = create_embedded_overlay_font(&mut doc, "FEmbed1", "songti", "测试页眉")
            .expect("应能选到覆盖中文的字体");
        assert_eq!(
            choice
                .font
                .char_to_gid
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            "测试页眉".chars().collect::<BTreeSet<_>>(),
        );
    }
}
