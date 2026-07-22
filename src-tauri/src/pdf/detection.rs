use anyhow::{Context, Result};
use lopdf::content::{Content, Operation};
use lopdf::decode_text_string;
use lopdf::{Document, Object};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::LazyLock;

use crate::external::ExternalTool;

// Full bbox XML plus content-stream inspection is intentionally bounded. A
// merged evidence file can have thousands of scanned pages, and keeping every
// page's XML/text/object sample in memory is not safe for a desktop workflow.
const MAX_SPLIT_ANALYSIS_PAGES: u32 = 600;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionArgs {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(default = "default_max_pages")]
    max_pages: u32,
    #[serde(default = "default_header_zone_ratio")]
    header_zone_ratio: f32,
    #[serde(default = "default_footer_zone_ratio")]
    footer_zone_ratio: f32,
    #[serde(default)]
    header_zone_mm: Option<f32>,
    #[serde(default)]
    footer_zone_mm: Option<f32>,
    #[serde(default = "default_scan_artifacts")]
    scan_artifacts: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    input_path: String,
    pages_analyzed: u32,
    artifact: ArtifactSummary,
    pages: Vec<PageDetection>,
    header_candidates: Vec<HeaderFooterCandidate>,
    footer_candidates: Vec<HeaderFooterCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitSuggestionArgs {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(default)]
    max_pages: Option<u32>,
    #[serde(default)]
    header_zone_mm: Option<f32>,
    #[serde(default)]
    footer_zone_mm: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitSuggestionResult {
    input_path: String,
    total_pages: u32,
    pages_analyzed: u32,
    header_pages: usize,
    page_number_footer_pages: usize,
    warnings: Vec<String>,
    items: Vec<SplitSuggestionItem>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SplitSuggestionItem {
    name: String,
    page_start: u32,
    page_end: u32,
    source: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSummary {
    has_header: bool,
    has_footer: bool,
    header_count: usize,
    footer_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDetection {
    page: u32,
    width: f32,
    height: f32,
    headers: Vec<TextLineDetection>,
    footers: Vec<TextLineDetection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextLineDetection {
    text: String,
    normalized_text: String,
    bbox: BBox,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooterCandidate {
    text: String,
    normalized_text: String,
    region: String,
    page_range: PageRange,
    count: usize,
    repeating: bool,
    labels: Vec<String>,
    confidence: f32,
    bbox: BBox,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<f32>,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRange {
    start: u32,
    end: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BBox {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    page: u32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
struct WordBox {
    page: u32,
    width: f32,
    height: f32,
    text: String,
    bbox: BBox,
}

#[derive(Debug, Clone)]
struct LineBox {
    text: String,
    bbox: BBox,
    font_size: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
struct ParsedPageSize {
    page: u32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
struct ContentFontSample {
    page: u32,
    normalized_text: String,
    x: f32,
    y_top: f32,
    font_size: f32,
}

fn default_max_pages() -> u32 {
    20
}

fn default_header_zone_ratio() -> f32 {
    0.12
}

fn default_footer_zone_ratio() -> f32 {
    0.12
}

fn default_scan_artifacts() -> bool {
    true
}

pub fn detect(args: &serde_json::Value) -> Result<DetectionResult> {
    let args: DetectionArgs =
        serde_json::from_value(args.clone()).context("解析页眉页脚检测参数失败")?;
    let input = Path::new(&args.input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }

    let artifact = if args.scan_artifacts {
        inspect_artifacts(input).unwrap_or_default()
    } else {
        ArtifactSummary::default()
    };
    let xml = run_pdftotext_bbox(input, args.max_pages)?;
    let words = parse_pdftotext_bbox(&xml)?;
    let page_sizes = parse_pdftotext_page_sizes(&xml)?;
    let mut pages = build_page_detections(&words, &page_sizes, &args);
    if let Ok(samples) = inspect_content_font_samples(input, &page_sizes) {
        attach_content_font_sizes(&mut pages, &samples);
    }
    let pages_analyzed = pages.len() as u32;
    let header_candidates = build_candidates(&pages, "header", pages_analyzed);
    let footer_candidates = build_candidates(&pages, "footer", pages_analyzed);

    Ok(DetectionResult {
        input_path: args.input_path,
        pages_analyzed,
        artifact,
        pages,
        header_candidates,
        footer_candidates,
    })
}

pub fn suggest_split_ranges(args: &serde_json::Value) -> Result<SplitSuggestionResult> {
    let args: SplitSuggestionArgs =
        serde_json::from_value(args.clone()).context("解析拆分建议参数失败")?;
    let total_pages =
        super::qpdf::page_count(&args.input_path).context("读取合并 PDF 总页数失败")?;
    let requested_pages = args.max_pages.unwrap_or(total_pages).min(total_pages).max(1);
    let max_pages = requested_pages.min(MAX_SPLIT_ANALYSIS_PAGES);
    let detection = detect(&serde_json::json!({
        "inputPath": args.input_path,
        "maxPages": max_pages,
        "headerZoneMm": args.header_zone_mm,
        "footerZoneMm": args.footer_zone_mm.unwrap_or(25.0),
        "footerZoneRatio": 0.03,
        "scanArtifacts": false
    }))?;
    let items = build_split_suggestions_from_pages(&detection.pages);
    let header_pages = count_split_header_pages(&detection.pages);
    let page_number_footer_pages = count_page_number_footers(&detection.pages);
    let mut warnings = split_suggestion_warnings(
        &items,
        total_pages,
        detection.pages_analyzed,
        header_pages,
        page_number_footer_pages,
    );
    if max_pages < total_pages {
        warnings.push(format!(
            "为避免大文件卡顿，本次只自动识别前 {max_pages} 页；后续页段请手动补充或分批处理"
        ));
    }
    Ok(SplitSuggestionResult {
        input_path: detection.input_path,
        total_pages,
        pages_analyzed: detection.pages_analyzed,
        header_pages,
        page_number_footer_pages,
        warnings,
        items,
    })
}

fn inspect_artifacts(input: &Path) -> Result<ArtifactSummary> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let qdf = temp_named_path("docsy_artifact_scan", "pdf");
    let mut command = Command::new(&bin);
    command
        .arg("--qdf")
        .arg("--object-streams=disable")
        .arg(input)
        .arg(&qdf);
    let output = run_command_output(command, "qpdf Artifact 检测")?;

    if !output.status.success() {
        let _ = fs::remove_file(&qdf);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("qpdf Artifact 检测失败: {}", stderr.trim());
    }

    let bytes = fs::read(&qdf).context("读取 qdf 临时文件失败")?;
    let _ = fs::remove_file(&qdf);
    let text = String::from_utf8_lossy(&bytes);
    Ok(parse_artifact_summary(&text))
}

fn parse_artifact_summary(text: &str) -> ArtifactSummary {
    let header = Regex::new(r"(?is)/Artifact\b.*?/Subtype\s*/Header\b.*?BDC").unwrap();
    let footer = Regex::new(r"(?is)/Artifact\b.*?/Subtype\s*/Footer\b.*?BDC").unwrap();
    let header_count = header.find_iter(text).count();
    let footer_count = footer.find_iter(text).count();
    ArtifactSummary {
        has_header: header_count > 0,
        has_footer: footer_count > 0,
        header_count,
        footer_count,
    }
}

fn run_pdftotext_bbox(input: &Path, max_pages: u32) -> Result<String> {
    let pdftotext = find_pdftotext().context("未找到 pdftotext，无法检测页眉页脚")?;
    let mut command = Command::new(pdftotext);
    command
        .arg("-bbox")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg(max_pages.max(1).to_string())
        .arg(input)
        .arg("-");
    let output = run_command_output(command, "pdftotext 检测")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pdftotext 检测失败: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_command_output(mut command: Command, label: &str) -> Result<Output> {
    command
        .output()
        .with_context(|| format!("执行 {label} 失败"))
}

fn parse_pdftotext_bbox(xml: &str) -> Result<Vec<WordBox>> {
    let page_re = Regex::new(r#"<page\b[^>]*\bwidth="([0-9.]+)"[^>]*\bheight="([0-9.]+)"[^>]*>"#)?;
    let word_re = Regex::new(
        r#"<word\b[^>]*\bxMin="([0-9.]+)"[^>]*\byMin="([0-9.]+)"[^>]*\bxMax="([0-9.]+)"[^>]*\byMax="([0-9.]+)"[^>]*>(.*?)</word>"#,
    )?;

    let mut words = Vec::new();
    let mut page = 0_u32;
    let mut width = 0_f32;
    let mut height = 0_f32;

    for line in xml.lines() {
        if let Some(caps) = page_re.captures(line) {
            page += 1;
            width = parse_f32(&caps[1]);
            height = parse_f32(&caps[2]);
            continue;
        }
        if let Some(caps) = word_re.captures(line) {
            if page == 0 {
                continue;
            }
            let text = decode_xml_text(&caps[5]).trim().to_string();
            if text.is_empty() {
                continue;
            }
            words.push(WordBox {
                page,
                width,
                height,
                text,
                bbox: BBox {
                    x0: parse_f32(&caps[1]),
                    y0: parse_f32(&caps[2]),
                    x1: parse_f32(&caps[3]),
                    y1: parse_f32(&caps[4]),
                    page,
                    width,
                    height,
                },
            });
        }
    }

    Ok(words)
}

fn parse_pdftotext_page_sizes(xml: &str) -> Result<Vec<ParsedPageSize>> {
    let page_re = Regex::new(r#"<page\b[^>]*\bwidth="([0-9.]+)"[^>]*\bheight="([0-9.]+)"[^>]*>"#)?;
    let mut pages = Vec::new();
    for caps in page_re.captures_iter(xml) {
        let page = pages.len() as u32 + 1;
        pages.push(ParsedPageSize {
            page,
            width: parse_f32(&caps[1]),
            height: parse_f32(&caps[2]),
        });
    }
    Ok(pages)
}

fn build_page_detections(
    words: &[WordBox],
    page_sizes: &[ParsedPageSize],
    args: &DetectionArgs,
) -> Vec<PageDetection> {
    let mut by_page: BTreeMap<u32, Vec<&WordBox>> = BTreeMap::new();
    for word in words {
        by_page.entry(word.page).or_default().push(word);
    }

    let pages = if page_sizes.is_empty() {
        fallback_page_sizes(words)
    } else {
        page_sizes.to_vec()
    };

    pages
        .into_iter()
        .map(|size| {
            let page_words = by_page.remove(&size.page).unwrap_or_default();
            let width = size.width;
            let height = size.height;
            let header_zone_pt =
                resolve_zone_pt(args.header_zone_mm, args.header_zone_ratio, height);
            let footer_zone_pt =
                resolve_zone_pt(args.footer_zone_mm, args.footer_zone_ratio, height);
            let header_words: Vec<&WordBox> = page_words
                .iter()
                .copied()
                .filter(|word| word.bbox.y0 <= header_zone_pt)
                .collect();
            let footer_words: Vec<&WordBox> = page_words
                .iter()
                .copied()
                .filter(|word| word.bbox.y1 >= height - footer_zone_pt)
                .collect();

            PageDetection {
                page: size.page,
                width,
                height,
                headers: group_words_into_lines(&header_words)
                    .into_iter()
                    .map(line_to_detection)
                    .collect(),
                footers: group_words_into_lines(&footer_words)
                    .into_iter()
                    .map(line_to_detection)
                    .collect(),
            }
        })
        .collect()
}

fn fallback_page_sizes(words: &[WordBox]) -> Vec<ParsedPageSize> {
    let mut sizes = BTreeMap::new();
    for word in words {
        sizes.entry(word.page).or_insert(ParsedPageSize {
            page: word.page,
            width: word.width,
            height: word.height,
        });
    }
    sizes.into_values().collect()
}

fn resolve_zone_pt(zone_mm: Option<f32>, ratio: f32, page_height_pt: f32) -> f32 {
    let ratio_pt = page_height_pt * ratio.clamp(0.03, 0.30);
    zone_mm
        .filter(|value| *value > 0.0)
        .map(|value| mm_to_pt(value).clamp(8.0, page_height_pt * 0.30))
        .unwrap_or(ratio_pt)
}

fn group_words_into_lines(words: &[&WordBox]) -> Vec<LineBox> {
    let mut buckets: BTreeMap<i32, Vec<&WordBox>> = BTreeMap::new();
    for word in words {
        let key = ((word.bbox.y0 + word.bbox.y1) / 2.0 / 3.0).round() as i32;
        buckets.entry(key).or_default().push(*word);
    }

    buckets
        .into_values()
        .filter_map(|mut line_words| {
            line_words.sort_by(|a, b| a.bbox.x0.total_cmp(&b.bbox.x0));
            let first = line_words.first()?;
            let mut text = String::new();
            let mut bbox = first.bbox;
            let mut last_x = first.bbox.x0;

            for word in line_words {
                if !text.is_empty() && word.bbox.x0 - last_x > 1.5 {
                    text.push(' ');
                }
                text.push_str(&word.text);
                bbox.x0 = bbox.x0.min(word.bbox.x0);
                bbox.y0 = bbox.y0.min(word.bbox.y0);
                bbox.x1 = bbox.x1.max(word.bbox.x1);
                bbox.y1 = bbox.y1.max(word.bbox.y1);
                last_x = word.bbox.x1;
            }

            Some(LineBox {
                text,
                bbox,
                font_size: None,
            })
        })
        .collect()
}

fn line_to_detection(line: LineBox) -> TextLineDetection {
    TextLineDetection {
        text: line.text.clone(),
        normalized_text: normalize_header_footer_text(&line.text),
        bbox: line.bbox,
        font_size: line.font_size,
    }
}

fn attach_content_font_sizes(pages: &mut [PageDetection], samples: &[ContentFontSample]) {
    for page in pages {
        for line in page.headers.iter_mut().chain(page.footers.iter_mut()) {
            if line.font_size.is_some() {
                continue;
            }
            line.font_size = samples
                .iter()
                .filter(|sample| sample.page == line.bbox.page)
                .filter(|sample| content_sample_matches_line(sample, line))
                .map(|sample| sample.font_size)
                .next();
        }
    }
}

fn content_sample_matches_line(sample: &ContentFontSample, line: &TextLineDetection) -> bool {
    let sample_norm = normalize_for_content_match(&sample.normalized_text);
    let line_norm = normalize_for_content_match(&line.normalized_text);
    let line_text = normalize_for_content_match(&line.text);
    if sample_norm.is_empty() {
        return false;
    }
    let text_matches = sample_norm == line_norm
        || sample_norm == line_text
        || line_norm.contains(&sample_norm)
        || sample_norm.contains(&line_norm);
    if !text_matches {
        return false;
    }
    let y_matches = sample.y_top >= line.bbox.y0 - 24.0 && sample.y_top <= line.bbox.y1 + 24.0;
    let x_matches = sample.x >= line.bbox.x0 - 36.0 && sample.x <= line.bbox.x1 + 36.0;
    y_matches && x_matches
}

fn inspect_content_font_samples(
    input: &Path,
    page_sizes: &[ParsedPageSize],
) -> Result<Vec<ContentFontSample>> {
    let doc = Document::load(input).context("读取 PDF 内容流失败")?;
    let pages = doc.get_pages();
    let mut samples = Vec::new();
    for (page_index, page_id) in pages.into_values().enumerate() {
        let page_number = page_index as u32 + 1;
        let Some(size) = page_sizes
            .iter()
            .find(|size| size.page == page_number)
            .or_else(|| page_sizes.get(page_index))
        else {
            continue;
        };
        let content_bytes = match doc.get_page_content(page_id) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let content = match Content::decode(&content_bytes) {
            Ok(content) => content,
            Err(_) => continue,
        };
        samples.extend(content_font_samples_from_operations(
            page_number,
            size.height,
            &content.operations,
        ));
    }
    Ok(samples)
}

#[derive(Debug, Clone, Copy)]
struct ContentTextState {
    in_text: bool,
    x: f32,
    y: f32,
    leading: f32,
    font_size: f32,
    scale_y: f32,
}

impl Default for ContentTextState {
    fn default() -> Self {
        Self {
            in_text: false,
            x: 0.0,
            y: 0.0,
            leading: 0.0,
            font_size: 0.0,
            scale_y: 1.0,
        }
    }
}

fn content_font_samples_from_operations(
    page: u32,
    page_height: f32,
    operations: &[Operation],
) -> Vec<ContentFontSample> {
    let mut state = ContentTextState::default();
    let mut samples = Vec::new();
    for operation in operations {
        update_content_text_state_before_show(&mut state, operation);
        if state.in_text {
            if let Some(text) = shown_content_text(operation) {
                let font_size = (state.font_size * state.scale_y.abs()).abs();
                if font_size > 0.0 && !text.trim().is_empty() {
                    samples.push(ContentFontSample {
                        page,
                        normalized_text: normalize_header_footer_text(&text),
                        x: state.x,
                        y_top: page_height - state.y,
                        font_size,
                    });
                }
            }
        }
    }
    samples
}

fn update_content_text_state_before_show(state: &mut ContentTextState, operation: &Operation) {
    match operation.operator.as_str() {
        "BT" => {
            state.in_text = true;
            state.x = 0.0;
            state.y = 0.0;
            state.scale_y = 1.0;
        }
        "ET" => {
            state.in_text = false;
        }
        "Tf" => {
            if let Some(size) = number_operand(operation, 1) {
                state.font_size = size;
            }
        }
        "Td" => {
            if let (Some(tx), Some(ty)) =
                (number_operand(operation, 0), number_operand(operation, 1))
            {
                state.x += tx;
                state.y += ty;
            }
        }
        "TD" => {
            if let (Some(tx), Some(ty)) =
                (number_operand(operation, 0), number_operand(operation, 1))
            {
                state.leading = -ty;
                state.x += tx;
                state.y += ty;
            }
        }
        "Tm" => {
            if let (Some(c), Some(d), Some(x), Some(y)) = (
                number_operand(operation, 2),
                number_operand(operation, 3),
                number_operand(operation, 4),
                number_operand(operation, 5),
            ) {
                state.x = x;
                state.y = y;
                let scale = (c * c + d * d).sqrt();
                state.scale_y = if scale > 0.0 { scale } else { 1.0 };
            }
        }
        "TL" => {
            if let Some(leading) = number_operand(operation, 0) {
                state.leading = leading;
            }
        }
        "T*" | "'" | "\"" => {
            state.y -= state.leading;
        }
        _ => {}
    }
}

fn shown_content_text(operation: &Operation) -> Option<String> {
    match operation.operator.as_str() {
        "Tj" | "'" => operation.operands.first().and_then(content_object_text),
        "\"" => operation.operands.get(2).and_then(content_object_text),
        "TJ" => {
            let Object::Array(items) = operation.operands.first()? else {
                return None;
            };
            let mut text = String::new();
            for item in items {
                if let Some(part) = content_object_text(item) {
                    text.push_str(&part);
                }
            }
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

fn number_operand(operation: &Operation, index: usize) -> Option<f32> {
    match operation.operands.get(index)? {
        Object::Integer(value) => Some(*value as f32),
        Object::Real(value) => Some(*value),
        _ => None,
    }
}

fn content_object_text(object: &Object) -> Option<String> {
    let Object::String(bytes, _) = object else {
        return None;
    };
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).ok();
    }
    String::from_utf8(bytes.clone())
        .ok()
        .or_else(|| decode_text_string(object).ok())
}

static RE_PAGE_TOTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d+\s*[/／∕]\s*\d+").unwrap());
static RE_CN_TOTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"第\s*\d+\s*页\s*[，,]\s*共\s*\d+\s*页").unwrap());
static RE_CN_PAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"第\s*\d+\s*页").unwrap());
static RE_CN_NUMERIC_PAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"第\s*[一二三四五六七八九十百千〇零两]+\s*页").unwrap());
static RE_PAGE_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)page\s+\d+\s+of\s+\d+").unwrap());
static RE_STANDALONE_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{1,4}$").unwrap());

fn is_roman_page_marker(value: &str) -> bool {
    let upper = value.trim().to_ascii_uppercase();
    if upper.is_empty()
        || upper.len() > 8
        || !upper
            .chars()
            .all(|ch| matches!(ch, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'))
    {
        return false;
    }
    parse_roman_page_number(&upper).is_some()
}

fn parse_roman_page_number(value: &str) -> Option<u32> {
    let mut total = 0_i32;
    let mut previous = 0_i32;
    for ch in value.chars().rev() {
        let current = match ch {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            'D' => 500,
            'M' => 1000,
            _ => return None,
        };
        if current < previous {
            total -= current;
        } else {
            total += current;
            previous = current;
        }
    }
    if total <= 0 {
        return None;
    }
    let canonical = roman_page_number(total as u32)?;
    (canonical == value).then_some(total as u32)
}

fn roman_page_number(mut value: u32) -> Option<String> {
    if value == 0 || value > 3999 {
        return None;
    }
    let tokens = [
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
    let mut out = String::new();
    for (amount, token) in tokens {
        while value >= amount {
            out.push_str(token);
            value -= amount;
        }
    }
    Some(out)
}

fn normalize_for_content_match(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn build_candidates(
    pages: &[PageDetection],
    region: &str,
    pages_analyzed: u32,
) -> Vec<HeaderFooterCandidate> {
    let mut grouped: BTreeMap<String, Vec<&TextLineDetection>> = BTreeMap::new();
    for page in pages {
        let lines = if region == "header" {
            &page.headers
        } else {
            &page.footers
        };
        for line in lines {
            if is_noise(&line.text, &line.normalized_text) {
                continue;
            }
            grouped
                .entry(line.normalized_text.clone())
                .or_default()
                .push(line);
        }
    }

    let mut candidates: Vec<HeaderFooterCandidate> = grouped
        .into_iter()
        .filter_map(|(normalized_text, lines)| {
            let first = *lines.first()?;
            let count = lines.len();
            let page_start = lines
                .iter()
                .map(|line| line.bbox.page)
                .min()
                .unwrap_or(first.bbox.page);
            let page_end = lines
                .iter()
                .map(|line| line.bbox.page)
                .max()
                .unwrap_or(first.bbox.page);
            let labels = labels_for(&normalized_text);
            let repeating = count >= 2 || labels.iter().any(|label| label == "page-number");
            let mut confidence = if pages_analyzed <= 1 {
                0.45
            } else {
                (count as f32 / pages_analyzed as f32).min(1.0)
            };
            if repeating {
                confidence += 0.25;
            }
            if labels.iter().any(|label| label == "page-number") {
                confidence += 0.15;
            }
            if normalized_text.contains("证据") {
                confidence += 0.10;
            }
            confidence = confidence.min(1.0);
            Some(HeaderFooterCandidate {
                text: first.text.clone(),
                normalized_text,
                region: region.to_string(),
                page_range: PageRange {
                    start: page_start,
                    end: page_end,
                },
                count,
                repeating,
                labels,
                confidence,
                bbox: first.bbox,
                font_size: lines.iter().find_map(|line| line.font_size),
                source: "content-text".to_string(),
            })
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.confidence
            .total_cmp(&a.confidence)
            .then_with(|| b.count.cmp(&a.count))
    });
    candidates
}

fn normalize_header_footer_text(text: &str) -> String {
    let normalized_digits: String = text
        .chars()
        .map(|ch| match ch {
            '０'..='９' => char::from_u32(ch as u32 - '０' as u32 + '0' as u32).unwrap_or(ch),
            _ => ch,
        })
        .collect();
    let collapsed = normalized_digits
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let mut value = collapsed;
    value = RE_CN_TOTAL
        .replace_all(&value, "第{page}页，共{total}页")
        .to_string();
    value = RE_PAGE_WORD
        .replace_all(&value, "Page {page} of {total}")
        .to_string();
    value = RE_PAGE_TOTAL
        .replace_all(&value, "{page}/{total}")
        .to_string();
    value = RE_CN_PAGE.replace_all(&value, "第{page}页").to_string();
    value = RE_CN_NUMERIC_PAGE
        .replace_all(&value, "第{page}页")
        .to_string();
    if RE_STANDALONE_NUMBER.is_match(&value) {
        value = "{page}".to_string();
    } else if is_roman_page_marker(value.trim()) {
        value = "{roman-page}".to_string();
    }
    value
}

fn labels_for(normalized_text: &str) -> Vec<String> {
    let mut labels = Vec::new();
    if normalized_text.contains("{page}")
        || normalized_text.contains("{total}")
        || normalized_text.contains("{roman-page}")
    {
        labels.push("page-number".to_string());
    }
    if normalized_text.contains("证据") {
        labels.push("evidence-label".to_string());
    }
    if normalized_text.to_lowercase().contains("confidential") {
        labels.push("confidential".to_string());
    }
    labels
}

fn is_noise(text: &str, normalized_text: &str) -> bool {
    if labels_for(normalized_text)
        .iter()
        .any(|label| label == "page-number")
    {
        return false;
    }
    let value = text.trim();
    let char_count = value.chars().count();
    !(2..=120).contains(&char_count)
}

fn build_split_suggestions_from_pages(pages: &[PageDetection]) -> Vec<SplitSuggestionItem> {
    let mut items = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_source = "fallback".to_string();
    let mut current_start = 1_u32;
    let mut previous_page = 0_u32;

    for page in pages {
        let header = best_split_header(page);
        if previous_page == 0 {
            current_start = page.page;
            if let Some(header) = header {
                current_name = Some(header);
                current_source = "header".to_string();
            } else {
                current_name = Some("目录".to_string());
                current_source = "fallback".to_string();
            }
            previous_page = page.page;
            continue;
        }

        if let Some(header) = header {
            if current_name.as_deref() != Some(header.as_str()) {
                let name = current_name
                    .take()
                    .unwrap_or_else(|| format!("文件{}", items.len() + 1));
                items.push(SplitSuggestionItem {
                    name,
                    page_start: current_start,
                    page_end: previous_page,
                    source: current_source.clone(),
                });
                current_start = page.page;
                current_name = Some(header);
                current_source = "header".to_string();
            }
        }
        previous_page = page.page;
    }

    if previous_page > 0 {
        let name = current_name.unwrap_or_else(|| format!("文件{}", items.len() + 1));
        items.push(SplitSuggestionItem {
            name,
            page_start: current_start,
            page_end: previous_page,
            source: current_source,
        });
    }

    items
}

fn count_page_number_footers(pages: &[PageDetection]) -> usize {
    pages
        .iter()
        .filter(|page| {
            page.footers
                .iter()
                .any(|line| labels_for(&line.normalized_text).contains(&"page-number".to_string()))
        })
        .count()
}

fn count_split_header_pages(pages: &[PageDetection]) -> usize {
    pages
        .iter()
        .filter(|page| best_split_header(page).is_some())
        .count()
}

fn split_suggestion_warnings(
    items: &[SplitSuggestionItem],
    total_pages: u32,
    pages_analyzed: u32,
    header_pages: usize,
    page_number_footer_pages: usize,
) -> Vec<String> {
    let mut warnings = Vec::new();
    warnings.push(format!(
        "拆分识别概况：文件共 {total_pages} 页，本次扫描 {pages_analyzed} 页，识别到 {header_pages} 页含页眉、{page_number_footer_pages} 页含页码型页脚"
    ));
    if items.is_empty() {
        warnings.push("未识别到页眉变化，请手动设置拆分页段".to_string());
        return warnings;
    }
    if items.first().map(|item| item.page_start).unwrap_or(1) > 1 {
        warnings.push("首页之前存在未覆盖页段".to_string());
    }
    if items.last().map(|item| item.page_end).unwrap_or(0) < total_pages {
        warnings.push("末尾存在未覆盖页段，请检查扫描页或空白页".to_string());
    }
    if items.iter().all(|item| item.source != "header") {
        warnings.push("未识别到稳定页眉，当前仅生成一个默认页段".to_string());
    }
    warnings
}

fn best_split_header(page: &PageDetection) -> Option<String> {
    page.headers
        .iter()
        .find(|line| {
            !is_noise(&line.text, &line.normalized_text)
                && !labels_for(&line.normalized_text).contains(&"page-number".to_string())
        })
        .map(|line| line.text.trim().to_string())
}

fn decode_xml_text(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn parse_f32(value: &str) -> f32 {
    value.parse::<f32>().unwrap_or_default()
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

fn find_pdftotext() -> Option<PathBuf> {
    crate::external::PopplerTool::binary_path_for("pdftotext").ok()
}

fn temp_named_path(prefix: &str, extension: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pdftotext_bbox_words() {
        let xml = r#"
<doc>
  <page width="595.000000" height="842.000000">
    <word xMin="500.000000" yMin="20.000000" xMax="540.000000" yMax="32.000000">证据</word>
    <word xMin="542.000000" yMin="20.000000" xMax="552.000000" yMax="32.000000">1</word>
    <word xMin="280.000000" yMin="812.000000" xMax="312.000000" yMax="824.000000">1/10</word>
  </page>
</doc>
"#;
        let words = parse_pdftotext_bbox(xml).unwrap();
        let page_sizes = parse_pdftotext_page_sizes(xml).unwrap();
        let pages = build_page_detections(
            &words,
            &page_sizes,
            &DetectionArgs {
                input_path: "/tmp/a.pdf".to_string(),
                max_pages: 20,
                header_zone_ratio: 0.12,
                footer_zone_ratio: 0.12,
                header_zone_mm: None,
                footer_zone_mm: None,
                scan_artifacts: false,
            },
        );
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].headers[0].text, "证据 1");
        assert_eq!(pages[0].footers[0].normalized_text, "{page}/{total}");
    }

    #[test]
    fn limits_detection_with_millimeter_zones() {
        let xml = r#"
<doc>
  <page width="595.000000" height="842.000000">
    <word xMin="500.000000" yMin="20.000000" xMax="552.000000" yMax="32.000000">页眉</word>
    <word xMin="50.000000" yMin="88.000000" xMax="120.000000" yMax="100.000000">正文标题</word>
  </page>
</doc>
"#;
        let words = parse_pdftotext_bbox(xml).unwrap();
        let page_sizes = parse_pdftotext_page_sizes(xml).unwrap();
        let pages = build_page_detections(
            &words,
            &page_sizes,
            &DetectionArgs {
                input_path: "/tmp/a.pdf".to_string(),
                max_pages: 20,
                header_zone_ratio: 0.30,
                footer_zone_ratio: 0.12,
                header_zone_mm: Some(20.0),
                footer_zone_mm: None,
                scan_artifacts: false,
            },
        );
        assert_eq!(pages[0].headers.len(), 1);
        assert_eq!(pages[0].headers[0].text, "页眉");
    }

    #[test]
    fn normalizes_common_page_number_formats() {
        assert_eq!(normalize_header_footer_text("１ / ２０"), "{page}/{total}");
        assert_eq!(normalize_header_footer_text("1／7"), "{page}/{total}");
        assert_eq!(
            normalize_header_footer_text("第 3 页，共 20 页"),
            "第{page}页，共{total}页"
        );
        assert_eq!(normalize_header_footer_text("第四页"), "第{page}页");
        assert_eq!(
            normalize_header_footer_text("Page 3 of 20"),
            "Page {page} of {total}"
        );
        assert_eq!(normalize_header_footer_text("15"), "{page}");
        assert_eq!(normalize_header_footer_text("III"), "{roman-page}");
        assert_eq!(normalize_header_footer_text("mid"), "mid");
        assert_eq!(normalize_header_footer_text("IC"), "IC");
    }

    #[test]
    fn decodes_pdf_doc_encoded_content_strings() {
        let text = content_object_text(&Object::String(
            b"Header \x8dQuoted\x8e".to_vec(),
            lopdf::StringFormat::Literal,
        ))
        .unwrap();

        assert_eq!(text, "Header “Quoted”");
    }

    #[test]
    fn noise_filter_counts_characters_not_bytes() {
        assert!(is_noise("证", "证"));
        assert!(!is_noise("证据", "证据"));
    }

    #[test]
    fn keeps_standalone_numeric_footer_candidates() {
        let page = |page: u32, footer: &str| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![],
            footers: vec![TextLineDetection {
                text: footer.to_string(),
                normalized_text: normalize_header_footer_text(footer),
                bbox: BBox {
                    x0: 540.0,
                    y0: 800.0,
                    x1: 548.0,
                    y1: 822.0,
                    page,
                    width: 595.0,
                    height: 842.0,
                },
                font_size: None,
            }],
        };
        let pages = vec![page(1, "1"), page(2, "2"), page(3, "3")];
        let candidates = build_candidates(&pages, "footer", pages.len() as u32);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].normalized_text, "{page}");
        assert_eq!(candidates[0].count, 3);
        assert!(candidates[0].labels.contains(&"page-number".to_string()));
    }

    #[test]
    fn still_filters_short_non_page_footer_noise() {
        let pages = vec![PageDetection {
            page: 1,
            width: 595.0,
            height: 842.0,
            headers: vec![],
            footers: vec![TextLineDetection {
                text: "*".to_string(),
                normalized_text: normalize_header_footer_text("*"),
                bbox: BBox {
                    x0: 540.0,
                    y0: 800.0,
                    x1: 548.0,
                    y1: 822.0,
                    page: 1,
                    width: 595.0,
                    height: 842.0,
                },
                font_size: None,
            }],
        }];
        let candidates = build_candidates(&pages, "footer", pages.len() as u32);

        assert!(candidates.is_empty());
    }

    #[test]
    fn attaches_font_size_from_content_stream_when_available() {
        let mut pages = vec![PageDetection {
            page: 1,
            width: 595.0,
            height: 842.0,
            headers: vec![TextLineDetection {
                text: "测试页眉".to_string(),
                normalized_text: "测试页眉".to_string(),
                bbox: BBox {
                    x0: 500.0,
                    y0: 20.0,
                    x1: 570.0,
                    y1: 38.0,
                    page: 1,
                    width: 595.0,
                    height: 842.0,
                },
                font_size: None,
            }],
            footers: vec![],
        }];
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    505.into(),
                    812.into(),
                ],
            ),
            Operation::new(
                "Tj",
                vec![Object::String(
                    encode_utf16be_pdf_string_for_test("测试页眉"),
                    lopdf::StringFormat::Hexadecimal,
                )],
            ),
            Operation::new("ET", vec![]),
        ];
        let samples = content_font_samples_from_operations(1, 842.0, &operations);
        attach_content_font_sizes(&mut pages, &samples);

        assert_eq!(pages[0].headers[0].font_size, Some(10.0));
    }

    #[test]
    fn tm_font_size_uses_vertical_matrix_scale() {
        let operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    9.into(),
                    0.into(),
                    2.into(),
                    505.into(),
                    812.into(),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal("Scaled")]),
            Operation::new("ET", vec![]),
        ];

        let samples = content_font_samples_from_operations(1, 842.0, &operations);

        assert_eq!(samples[0].font_size, 20.0);
    }

    fn encode_utf16be_pdf_string_for_test(value: &str) -> Vec<u8> {
        let mut encoded = vec![0xFE, 0xFF];
        for unit in value.encode_utf16() {
            encoded.extend(unit.to_be_bytes());
        }
        encoded
    }

    #[test]
    fn detects_artifact_summary() {
        let text = "/Artifact << /Type /Pagination /Subtype /Header >> BDC q Q EMC";
        let summary = parse_artifact_summary(text);
        assert!(summary.has_header);
        assert_eq!(summary.header_count, 1);
        assert!(!summary.has_footer);
    }

    #[test]
    fn builds_split_suggestions_from_header_changes() {
        let page = |page: u32, text: &str| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![TextLineDetection {
                text: text.to_string(),
                normalized_text: text.to_string(),
                bbox: BBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 10.0,
                    y1: 10.0,
                    page,
                    width: 595.0,
                    height: 842.0,
                },
                font_size: None,
            }],
            footers: vec![],
        };
        let items = build_split_suggestions_from_pages(&[
            page(1, "证据一"),
            page(2, "证据一"),
            page(3, "证据二"),
        ]);
        assert_eq!(
            items,
            vec![
                SplitSuggestionItem {
                    name: "证据一".to_string(),
                    page_start: 1,
                    page_end: 2,
                    source: "header".to_string(),
                },
                SplitSuggestionItem {
                    name: "证据二".to_string(),
                    page_start: 3,
                    page_end: 3,
                    source: "header".to_string(),
                },
            ]
        );
    }

    #[test]
    fn names_first_headerless_split_range_as_catalog() {
        let page = |page: u32, header: Option<&str>| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: header
                .map(|text| {
                    vec![TextLineDetection {
                        text: text.to_string(),
                        normalized_text: text.to_string(),
                        bbox: BBox {
                            x0: 0.0,
                            y0: 0.0,
                            x1: 10.0,
                            y1: 10.0,
                            page,
                            width: 595.0,
                            height: 842.0,
                        },
                        font_size: None,
                    }]
                })
                .unwrap_or_default(),
            footers: vec![],
        };
        let items = build_split_suggestions_from_pages(&[
            page(1, None),
            page(2, Some("证据8-1")),
            page(3, Some("证据8-1")),
        ]);

        assert_eq!(items[0].name, "目录");
        assert_eq!(items[0].page_start, 1);
        assert_eq!(items[0].page_end, 1);
        assert_eq!(items[1].name, "证据8-1");
    }

    #[test]
    fn split_warning_summarizes_header_and_page_number_counts() {
        let page = |page: u32, header: Option<&str>, footer: Option<&str>| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: header
                .map(|text| {
                    vec![TextLineDetection {
                        text: text.to_string(),
                        normalized_text: normalize_header_footer_text(text),
                        bbox: BBox {
                            x0: 0.0,
                            y0: 0.0,
                            x1: 10.0,
                            y1: 10.0,
                            page,
                            width: 595.0,
                            height: 842.0,
                        },
                        font_size: None,
                    }]
                })
                .unwrap_or_default(),
            footers: footer
                .map(|text| {
                    vec![TextLineDetection {
                        text: text.to_string(),
                        normalized_text: normalize_header_footer_text(text),
                        bbox: BBox {
                            x0: 0.0,
                            y0: 820.0,
                            x1: 10.0,
                            y1: 840.0,
                            page,
                            width: 595.0,
                            height: 842.0,
                        },
                        font_size: None,
                    }]
                })
                .unwrap_or_default(),
        };
        let pages = vec![
            page(1, Some("证据一"), Some("1/2")),
            page(2, Some("证据一"), Some("2/2")),
            page(3, None, None),
        ];
        let warnings = split_suggestion_warnings(
            &build_split_suggestions_from_pages(&pages),
            3,
            pages.len() as u32,
            count_split_header_pages(&pages),
            count_page_number_footers(&pages),
        );

        assert_eq!(
            warnings[0],
            "拆分识别概况：文件共 3 页，本次扫描 3 页，识别到 2 页含页眉、2 页含页码型页脚"
        );
    }

    #[test]
    fn preserves_blank_pages_in_detection() {
        let xml = r#"
<doc>
  <page width="595.000000" height="842.000000">
    <word xMin="500.000000" yMin="20.000000" xMax="552.000000" yMax="32.000000">证据一</word>
  </page>
  <page width="595.000000" height="842.000000">
  </page>
</doc>
"#;
        let words = parse_pdftotext_bbox(xml).unwrap();
        let page_sizes = parse_pdftotext_page_sizes(xml).unwrap();
        let pages = build_page_detections(
            &words,
            &page_sizes,
            &DetectionArgs {
                input_path: "/tmp/a.pdf".to_string(),
                max_pages: 20,
                header_zone_ratio: 0.12,
                footer_zone_ratio: 0.12,
                header_zone_mm: None,
                footer_zone_mm: None,
                scan_artifacts: false,
            },
        );
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[1].page, 2);
        assert!(pages[1].headers.is_empty());
    }
}
