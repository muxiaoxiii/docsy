use anyhow::{Context, Result};
#[cfg(test)]
use lopdf::content::Operation;
#[cfg(test)]
use lopdf::decode_text_string;
#[cfg(test)]
use lopdf::Object;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::LazyLock;
use std::time::Instant;

// Full bbox XML plus content-stream inspection is intentionally bounded. A
// merged evidence file can have thousands of scanned pages, and keeping every
// page's XML/text/object sample in memory is not safe for a desktop workflow.
// 3000 页覆盖绝大多数合并证据文件；超过时前端会提示先分析前 N 页。
const MAX_SPLIT_ANALYSIS_PAGES: u32 = 3000;

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    has_total: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_form: Option<String>,
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
    position_stable: bool,
    position_spread: f32,
    sequence_stable: bool,
    labels: Vec<String>,
    confidence: f32,
    bbox: BBox,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<f32>,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    docsy_kind: Option<String>,
    /// Page-number style of the sequence ("arabic"/"roman"/"chinese"/"circled"/"dingbat"/"fraction"/"page-of"/"page-n").
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_form: Option<String>,
    /// Whether the page number carries a total (N/M, 共M页, Page N of M).
    #[serde(skip_serializing_if = "Option::is_none")]
    has_total: Option<bool>,
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

#[cfg(test)]
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

pub fn detect(args: &DetectionArgs) -> Result<DetectionResult> {
    let input = Path::new(&args.input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }

    let started = Instant::now();
    crate::app_log::info(
        "pdf.detect",
        "start",
        serde_json::json!({
            "file": input.to_string_lossy(),
            "maxPages": args.max_pages,
        }),
    );

    let artifact_started = Instant::now();
    let artifact_inspection = if args.scan_artifacts {
        Some(super::artifacts::inspect_meaningful_header_footer_artifacts(input, args.max_pages)?)
    } else {
        None
    };
    log_detection_stage(
        input,
        "existing-elements",
        artifact_started,
        serde_json::json!({
            "occurrences": artifact_inspection
                .as_ref()
                .map(|inspection| inspection.occurrences.len())
                .unwrap_or(0),
        }),
    );
    let artifact = artifact_inspection
        .as_ref()
        .map(artifact_summary)
        .unwrap_or_default();

    let text_started = Instant::now();
    let xml = run_pdftotext_bbox(input, args.max_pages)?;
    let words = parse_pdftotext_bbox(&xml)?;
    let page_sizes = parse_pdftotext_page_sizes(&xml)?;
    let pages = build_page_detections(&words, &page_sizes, args);
    log_detection_stage(
        input,
        "page-text",
        text_started,
        serde_json::json!({
            "pages": pages.len(),
            "words": words.len(),
        }),
    );

    let candidate_started = Instant::now();
    let pages_analyzed = pages.len() as u32;
    let content_headers = build_candidates(&pages, "header", pages_analyzed);
    let content_footers = build_candidates(&pages, "footer", pages_analyzed);
    let artifact_candidates = artifact_inspection
        .as_ref()
        .map(|inspection| {
            build_artifact_candidates(inspection, &pages, &content_headers, &content_footers)
        })
        .unwrap_or_default();
    let header_candidates = merge_artifact_first_candidates(
        artifact_candidates
            .iter()
            .filter(|candidate| candidate.region == "header")
            .cloned()
            .collect(),
        content_headers,
    );
    let footer_candidates = merge_artifact_first_candidates(
        artifact_candidates
            .into_iter()
            .filter(|candidate| candidate.region == "footer")
            .collect(),
        content_footers,
    );
    log_detection_stage(
        input,
        "candidates",
        candidate_started,
        serde_json::json!({
            "headers": header_candidates.len(),
            "footers": footer_candidates.len(),
            "totalElapsedMs": started.elapsed().as_millis(),
        }),
    );

    Ok(DetectionResult {
        input_path: args.input_path.clone(),
        pages_analyzed,
        artifact,
        pages,
        header_candidates,
        footer_candidates,
    })
}

fn log_detection_stage(input: &Path, stage: &str, started: Instant, details: serde_json::Value) {
    crate::app_log::info(
        "pdf.detect",
        stage,
        serde_json::json!({
            "file": input.to_string_lossy(),
            "elapsedMs": started.elapsed().as_millis(),
            "details": details,
        }),
    );
}

pub fn suggest_split_ranges(args: &SplitSuggestionArgs) -> Result<SplitSuggestionResult> {
    let total_pages =
        super::qpdf::page_count(&args.input_path).context("读取合并 PDF 总页数失败")?;
    let requested_pages = args
        .max_pages
        .unwrap_or(total_pages)
        .min(total_pages)
        .max(1);
    let max_pages = requested_pages.min(MAX_SPLIT_ANALYSIS_PAGES);
    let detection = detect(&DetectionArgs {
        input_path: args.input_path.clone(),
        max_pages,
        header_zone_ratio: default_header_zone_ratio(),
        footer_zone_ratio: 0.03,
        header_zone_mm: args.header_zone_mm,
        footer_zone_mm: args.footer_zone_mm,
        // 与分项证据处理保持同一套三层检测：启用 artifact 层，
        // 否则 docsy 生成的"证据N"页眉（PDF artifact）无法参与切分，
        // 只剩逐页文本对比，容易切得乱七八糟。
        scan_artifacts: true,
    })?;
    let items = build_split_suggestions(&detection);
    let items = augment_splits_with_page_number_boundaries(items, &detection.pages);
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
            "文件共 {total_pages} 页，本次只自动识别前 {max_pages} 页；后续页段请手动补充或分批处理"
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

fn artifact_summary(
    inspection: &super::artifacts::HeaderFooterArtifactInspection,
) -> ArtifactSummary {
    let header_count = inspection.header_count;
    let footer_count = inspection.footer_count;
    ArtifactSummary {
        has_header: header_count > 0,
        has_footer: footer_count > 0,
        header_count,
        footer_count,
    }
}

fn build_artifact_candidates(
    inspection: &super::artifacts::HeaderFooterArtifactInspection,
    pages: &[PageDetection],
    content_headers: &[HeaderFooterCandidate],
    content_footers: &[HeaderFooterCandidate],
) -> Vec<HeaderFooterCandidate> {
    let mut grouped: BTreeMap<String, Vec<&super::artifacts::HeaderFooterArtifactOccurrence>> =
        BTreeMap::new();
    for occurrence in &inspection.occurrences {
        let normalized_text = occurrence
            .text
            .as_deref()
            .map(normalize_header_footer_text)
            .unwrap_or_default();
        let key = format!(
            "{}|{}|{}",
            occurrence.region,
            occurrence.docsy_kind.as_deref().unwrap_or("standard"),
            normalized_text
        );
        grouped.entry(key).or_default().push(occurrence);
    }
    grouped
        .into_values()
        .filter_map(|occurrences| {
            let first = *occurrences.first()?;
            let page_start = occurrences.iter().map(|item| item.page).min()?;
            let page_end = occurrences.iter().map(|item| item.page).max()?;
            let region_candidates = if first.region == "header" {
                content_headers
            } else {
                content_footers
            };
            // BUGFIX: 不再 fallback 到 content-text 候选，
            // 避免正文内容被错误当成页眉/页脚。
            // 没有自身文本的 artifact 直接跳过。
            let text = first
                .text
                .clone()
                .filter(|value| !value.trim().is_empty())?;

            let normalized_text = normalize_header_footer_text(&text);
            // 优先取与 artifact 文本一致的内容候选来提供 bbox：同一页眉区可能
            // 还有另一个重复页眉（如韩国专利每页的“등록특허 10-xxxx”），若只按
            // count 取最大，会把那个页眉的 bbox 错贴到首页“证据N”artifact 上，
            // 导致预览里的删除标记位置偏移（证据2/证据5）。
            let supporting = region_candidates
                .iter()
                .filter(|candidate| {
                    candidate.page_range.end >= page_start && candidate.page_range.start <= page_end
                })
                .filter(|candidate| candidate.normalized_text == normalized_text)
                .max_by_key(|candidate| candidate.count)
                .or_else(|| {
                    region_candidates
                        .iter()
                        .filter(|candidate| {
                            candidate.page_range.end >= page_start
                                && candidate.page_range.start <= page_end
                        })
                        .max_by_key(|candidate| candidate.count)
                });

            let mut labels = labels_for(&normalized_text);
            if first.docsy_kind.as_deref() == Some("PageNumber")
                && !labels.iter().any(|label| label == "page-number")
            {
                labels.push("page-number".to_string());
            }
            let bbox = supporting
                .map(|candidate| candidate.bbox)
                .or_else(|| approximate_artifact_bbox(pages, first.region, page_start))?;
            let count = occurrences
                .iter()
                .map(|item| item.page)
                .collect::<BTreeSet<_>>()
                .len();
            let is_page_number = labels.iter().any(|label| label == "page-number");
            Some(HeaderFooterCandidate {
                text: text.clone(),
                normalized_text,
                region: first.region.to_string(),
                page_range: PageRange {
                    start: page_start,
                    end: page_end,
                },
                count,
                repeating: count >= 2,
                position_stable: true,
                position_spread: 0.0,
                sequence_stable: true,
                labels,
                confidence: 1.0,
                bbox,
                font_size: supporting.and_then(|candidate| candidate.font_size),
                source: "artifact".to_string(),
                artifact_id: first.docsy_id.clone().or_else(|| Some(first.id.clone())),
                docsy_kind: first.docsy_kind.clone(),
                sequence_form: is_page_number.then(|| sequence_form_of(&text).to_string()),
                has_total: is_page_number.then(|| text_has_total(&text)),
            })
        })
        .collect()
}

fn approximate_artifact_bbox(pages: &[PageDetection], region: &str, page: u32) -> Option<BBox> {
    let page_info = pages.iter().find(|item| item.page == page)?;
    let (y0, y1) = if region == "header" {
        (0.0, page_info.height * 0.12)
    } else {
        (page_info.height * 0.88, page_info.height)
    };
    Some(BBox {
        x0: 0.0,
        y0,
        x1: page_info.width,
        y1,
        page,
        width: page_info.width,
        height: page_info.height,
    })
}

fn merge_artifact_first_candidates(
    mut artifacts: Vec<HeaderFooterCandidate>,
    content: Vec<HeaderFooterCandidate>,
) -> Vec<HeaderFooterCandidate> {
    let artifact_keys = artifacts
        .iter()
        .map(|candidate| (candidate.region.clone(), candidate.normalized_text.clone()))
        .collect::<BTreeSet<_>>();
    artifacts.extend(content.into_iter().filter(|candidate| {
        !artifact_keys.contains(&(candidate.region.clone(), candidate.normalized_text.clone()))
    }));
    artifacts
}

fn run_pdftotext_bbox(input: &Path, max_pages: u32) -> Result<String> {
    let pdftotext = find_pdftotext().context("未找到 pdftotext，无法检测页眉页脚")?;
    let mut command = crate::external::hidden_command(pdftotext);
    command.arg("-bbox").arg("-f").arg("1");
    if max_pages > 0 {
        command.arg("-l").arg(max_pages.to_string());
    }
    command.arg(input).arg("-");
    let output = run_command_output(command, "pdftotext 检测")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pdftotext 检测失败: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_command_output(mut command: Command, label: &str) -> Result<Output> {
    crate::external::hide_command_window(&mut command);
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
                    .flat_map(|line| split_mixed_page_number_line(line, width))
                    .map(line_to_detection)
                    .collect(),
                footers: group_words_into_lines(&footer_words)
                    .into_iter()
                    .flat_map(|line| split_mixed_page_number_line(line, width))
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
                // pdftotext already gives us the rendered glyph box. Its
                // height is a reliable and cheap approximation of the visible
                // size, without decoding every page content stream a second
                // time (which is prohibitively expensive for scanned PDFs).
                font_size: Some(estimate_font_size_from_bbox(bbox)),
            })
        })
        .collect()
}

fn estimate_font_size_from_bbox(bbox: BBox) -> f32 {
    (bbox.y1 - bbox.y0).abs().clamp(1.0, 144.0)
}

/// Split a line that contains both header text and a trailing page number pattern.
/// Also split if the line spans more than 30% of page width (likely two separate groups).
fn split_mixed_page_number_line(line: LineBox, page_width: f32) -> Vec<LineBox> {
    static PAGE_NUM_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?ix)\s\d+\s*/\s*\d+\s*页?$|\s第\s*\d+\s*页\s*/\s*共\s*\d+\s*页$|\spage\s*\d+\s*(?:of|/)\s*\d+$").unwrap()
    });
    let text = line.text.trim();
    // Position-based split: line spans > 30% of page width
    let line_width = line.bbox.x1 - line.bbox.x0;
    if line_width > page_width * 0.30 {
        // Try to find a page number pattern at the end
        if let Some(m) = PAGE_NUM_RE.find(text) {
            let prefix = text[..m.start()].trim();
            let suffix = text[m.start()..].trim();
            if !prefix.is_empty() && !suffix.is_empty() {
                // Estimate split x-position from character count ratio
                let total_chars = text.chars().count() as f32;
                let prefix_chars = prefix.chars().count() as f32;
                let ratio = prefix_chars / total_chars;
                let split_x = line.bbox.x0 + line_width * ratio;
                return vec![
                    LineBox {
                        text: prefix.to_string(),
                        bbox: BBox {
                            x0: line.bbox.x0,
                            x1: split_x,
                            ..line.bbox
                        },
                        font_size: line.font_size,
                    },
                    LineBox {
                        text: suffix.to_string(),
                        bbox: BBox {
                            x0: split_x,
                            x1: line.bbox.x1,
                            ..line.bbox
                        },
                        font_size: line.font_size,
                    },
                ];
            }
        }
    }
    // Content-based split even without position trigger: trailing page number
    if let Some(m) = PAGE_NUM_RE.find(text) {
        let prefix = text[..m.start()].trim();
        let suffix = text[m.start()..].trim();
        if !prefix.is_empty() && !suffix.is_empty() {
            let total_chars = text.chars().count() as f32;
            let prefix_chars = prefix.chars().count() as f32;
            let ratio = prefix_chars / total_chars;
            let split_x = line.bbox.x0 + line_width * ratio;
            return vec![
                LineBox {
                    text: prefix.to_string(),
                    bbox: BBox {
                        x0: line.bbox.x0,
                        x1: split_x,
                        ..line.bbox
                    },
                    font_size: line.font_size,
                },
                LineBox {
                    text: suffix.to_string(),
                    bbox: BBox {
                        x0: split_x,
                        x1: line.bbox.x1,
                        ..line.bbox
                    },
                    font_size: line.font_size,
                },
            ];
        }
    }
    vec![line]
}

fn line_to_detection(line: LineBox) -> TextLineDetection {
    TextLineDetection {
        text: line.text.clone(),
        normalized_text: normalize_header_footer_text(&line.text),
        bbox: line.bbox,
        font_size: line.font_size,
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct ContentTextState {
    in_text: bool,
    x: f32,
    y: f32,
    leading: f32,
    font_size: f32,
    scale_y: f32,
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn number_operand(operation: &Operation, index: usize) -> Option<f32> {
    match operation.operands.get(index)? {
        Object::Integer(value) => Some(*value as f32),
        Object::Real(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
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

/// Map circled/dingbat digits to their arabic text. Returns None for non-digit chars.
fn circled_digit_text(ch: char) -> Option<&'static str> {
    const CIRCLED: [&str; 20] = [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        "17", "18", "19", "20",
    ];
    const DINGBAT: [&str; 10] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];
    const CIRCLED_11_20: [&str; 10] = ["11", "12", "13", "14", "15", "16", "17", "18", "19", "20"];
    let code = ch as u32;
    match code {
        0x2460..=0x2473 => Some(CIRCLED[(code - 0x2460) as usize]),
        0x2776..=0x277F => Some(DINGBAT[(code - 0x2776) as usize]),
        0x24EB..=0x24F4 => Some(CIRCLED_11_20[(code - 0x24EB) as usize]),
        _ => None,
    }
}

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

#[cfg(test)]
fn normalize_for_content_match(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn build_candidates(
    pages: &[PageDetection],
    region: &str,
    pages_analyzed: u32,
) -> Vec<HeaderFooterCandidate> {
    let mut grouped: BTreeMap<(String, i32, i32), Vec<&TextLineDetection>> = BTreeMap::new();
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
            let center_x = (line.bbox.x0 + line.bbox.x1) / 2.0 / line.bbox.width.max(1.0);
            let center_y = (line.bbox.y0 + line.bbox.y1) / 2.0 / line.bbox.height.max(1.0);
            let x_bucket = (center_x * 20.0).round() as i32;
            let y_bucket = (center_y * 40.0).round() as i32;
            // Page numbers at the same position form a sequence (different text per page),
            // so use a position-only key. Headers/footers use (text, position) key.
            let labels = labels_for(&line.normalized_text);
            let is_page_num = labels.iter().any(|l| l == "page-number");
            let key = if is_page_num {
                ("__page_number_seq__".to_string(), x_bucket, y_bucket)
            } else {
                (line.normalized_text.clone(), x_bucket, y_bucket)
            };
            grouped.entry(key).or_default().push(line);
        }
    }

    // For __page_number_seq__ groups, split by total value, then by value
    // continuity, before building candidates. E.g. "1/3 页" on pages 1-4 and
    // "1/13 页" on pages 5-17 are separate sequences; a document whose page
    // numbering restarts per section (1..13 then 1..2) must yield one
    // candidate per section instead of one merged "2-19" range.
    let mut split_groups: BTreeMap<(String, i32, i32), Vec<&TextLineDetection>> = BTreeMap::new();
    for ((normalized_text, x_bucket, y_bucket), lines) in grouped {
        if normalized_text == "__page_number_seq__" {
            let mut by_total: BTreeMap<Option<u32>, Vec<&TextLineDetection>> = BTreeMap::new();
            for line in &lines {
                let total = parsed_page_number_total(&line.text);
                by_total.entry(total).or_default().push(line);
            }
            for (total, group_lines) in by_total {
                for (seg_index, segment) in split_page_number_sequence(&group_lines)
                    .into_iter()
                    .enumerate()
                {
                    let suffix = match total {
                        Some(t) => format!(":total:{t}:seg:{seg_index}"),
                        None => format!(":seg:{seg_index}"),
                    };
                    let key = (format!("__page_number_seq__{suffix}"), x_bucket, y_bucket);
                    split_groups.entry(key).or_default().extend(segment);
                }
            }
            continue;
        }
        split_groups
            .entry((normalized_text, x_bucket, y_bucket))
            .or_default()
            .extend(lines);
    }

    let mut candidates: Vec<HeaderFooterCandidate> = split_groups
        .into_iter()
        .filter_map(|((normalized_text, _, _), lines)| {
            let first = *lines.first()?;
            let pages_seen = lines
                .iter()
                .map(|line| line.bbox.page)
                .collect::<BTreeSet<_>>();
            let count = pages_seen.len();
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
            // For page number sequences with VARYING text across pages,
            // derive representative text. Template markers (like "{page}") keep original text.
            let is_seq = normalized_text.starts_with("__page_number_seq__");
            let is_template = first.normalized_text.contains('{');
            let (display_text, effective_normalized) = if is_seq && !is_template {
                let parsed_values: Vec<u32> = lines
                    .iter()
                    .filter_map(|l| parsed_page_number_value(&l.text))
                    .collect();
                let has_total = lines.iter().any(|l| l.text.contains('/'));
                let representative = if has_total && !parsed_values.is_empty() {
                    format!(
                        "页码 1-{}/{}",
                        parsed_values.iter().max().copied().unwrap_or(count as u32),
                        page_end - page_start + 1
                    )
                } else {
                    format!("页码 {}-{}", page_start, page_end)
                };
                (representative.clone(), representative)
            } else {
                (first.text.clone(), first.normalized_text.clone())
            };
            let mut labels = labels_for(&effective_normalized);
            // First-page evidence labels such as "证据1" / "对比文件3" appear
            // only once per document, so they can never prove themselves by
            // repetition; tag them so they survive the repetition gate below.
            if page_start == 1 && is_evidence_label_text(&effective_normalized) {
                labels.push("evidence-label".to_string());
            }
            let is_page_number = labels.iter().any(|label| label == "page-number");
            let position_spread = normalized_position_spread(&lines);
            let position_stable = position_spread <= 0.025;
            let sequence_stable = !is_page_number || page_number_sequence_stable(&lines);
            let is_page_number = labels.iter().any(|label| label == "page-number");
            let minimum_repetitions = if is_page_number || pages_analyzed <= 4 {
                2
            } else {
                3
            };
            let repeating = count >= minimum_repetitions
                && position_stable
                && (!is_page_number || sequence_stable);
            let mut confidence = if pages_analyzed <= 1 {
                0.15
            } else {
                (count as f32 / pages_analyzed as f32).min(1.0)
            };
            if repeating {
                confidence += 0.25;
            }
            if count >= 2 && !position_stable {
                confidence *= 0.5;
            }
            if is_page_number && !sequence_stable {
                confidence *= 0.5;
            }
            confidence = confidence.min(1.0);
            Some(HeaderFooterCandidate {
                text: display_text,
                normalized_text: effective_normalized,
                region: region.to_string(),
                page_range: PageRange {
                    start: page_start,
                    end: page_end,
                },
                count,
                repeating,
                position_stable,
                position_spread,
                sequence_stable,
                labels,
                confidence,
                bbox: first.bbox,
                font_size: lines.iter().find_map(|line| line.font_size),
                source: "content-text".to_string(),
                artifact_id: None,
                docsy_kind: None,
                sequence_form: is_page_number.then(|| sequence_form_of(&first.text).to_string()),
                has_total: is_page_number.then(|| text_has_total(&first.text)),
            })
        })
        .collect();

    // Merge candidates with same normalized_text and very close positions (±2 buckets).
    // Different pages may place the same header at slightly different y positions,
    // causing them to land in adjacent buckets. These should be one group.
    // Page-number candidates are excluded: sectioning was already resolved by
    // total-value and value-reset splits, and merging would collapse distinct
    // sections (1/3, 1/13, 1/2 页) into one fake 2-19 range.
    let mut merged: Vec<HeaderFooterCandidate> = Vec::new();
    for cand in candidates {
        let is_page_number = cand.labels.iter().any(|label| label == "page-number");
        if is_page_number {
            merged.push(cand);
            continue;
        }
        if let Some(existing) = merged.iter_mut().find(|m| {
            m.normalized_text == cand.normalized_text
                && m.region == cand.region
                && (m.bbox.x0 - cand.bbox.x0).abs() < 20.0
                && (m.bbox.y0 - cand.bbox.y0).abs() < 15.0
        }) {
            // Merge: extend page range, increase count, keep higher confidence
            existing.page_range.start = existing.page_range.start.min(cand.page_range.start);
            existing.page_range.end = existing.page_range.end.max(cand.page_range.end);
            existing.count += cand.count;
            existing.confidence = existing.confidence.max(cand.confidence);
            existing.repeating = existing.repeating || cand.repeating;
        } else {
            merged.push(cand);
        }
    }
    candidates = merged;

    // Page content is only promoted to an existing header/footer/page-number
    // candidate when repetition or a stable sequence proves that it is not an
    // incidental body line. Structural Artifact candidates are merged later
    // and are not subject to this heuristic gate. First-page evidence labels
    // ("证据1", "对比文件3") are exempt: they legitimately occur only once.
    candidates.retain(|candidate| {
        candidate.repeating
            || candidate
                .labels
                .iter()
                .any(|label| label == "evidence-label")
    });

    candidates.sort_by(|a, b| {
        b.confidence
            .total_cmp(&a.confidence)
            .then_with(|| b.count.cmp(&a.count))
    });
    candidates
}

fn normalized_position_spread(lines: &[&TextLineDetection]) -> f32 {
    if lines.len() <= 1 {
        return 0.0;
    }
    let positions = lines
        .iter()
        .map(|line| {
            let width = line.bbox.width.max(1.0);
            let height = line.bbox.height.max(1.0);
            (
                ((line.bbox.x0 + line.bbox.x1) / 2.0) / width,
                ((line.bbox.y0 + line.bbox.y1) / 2.0) / height,
            )
        })
        .collect::<Vec<_>>();
    let (mean_x, mean_y) = positions.iter().fold((0.0, 0.0), |(x, y), position| {
        (x + position.0, y + position.1)
    });
    let count = positions.len() as f32;
    let (mean_x, mean_y) = (mean_x / count, mean_y / count);
    positions
        .iter()
        .map(|(x, y)| ((x - mean_x).powi(2) + (y - mean_y).powi(2)).sqrt())
        .fold(0.0, f32::max)
}

fn page_number_sequence_stable(lines: &[&TextLineDetection]) -> bool {
    let mut values = lines
        .iter()
        .filter_map(|line| {
            parsed_page_number_value(&line.text).map(|value| (line.bbox.page, value))
        })
        .collect::<Vec<_>>();
    values.sort_unstable_by_key(|item| item.0);
    values.dedup_by_key(|item| item.0);
    if values.len() < 2 {
        return false;
    }
    let transitions = values.len() - 1;
    let consecutive = values
        .windows(2)
        .filter(|pair| {
            pair[1].1
                == pair[0]
                    .1
                    .saturating_add(pair[1].0.saturating_sub(pair[0].0))
        })
        .count();
    consecutive * 10 >= transitions * 7
}

fn parsed_page_number_value(text: &str) -> Option<u32> {
    // Normalize full-width slash/digits to half-width before matching
    let text = text
        .replace('／', "/")
        .replace('０', "0")
        .replace('１', "1")
        .replace('２', "2")
        .replace('３', "3")
        .replace('４', "4")
        .replace('５', "5")
        .replace('６', "6")
        .replace('７', "7")
        .replace('８', "8")
        .replace('９', "9");
    let trimmed = text.trim();
    // Match the page-number pattern itself (N/M, Page N of M, 第N页, trailing
    // bare number) instead of the first number anywhere, so case numbers like
    // "（2026）X刑初123号" don't pollute sequence analysis.
    static RE_SLASH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\d{1,6})\s*/\s*\d+").expect("valid page number regex"));
    static RE_PAGE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bpage\s*(\d{1,6})\b").expect("valid page number regex"));
    static RE_CN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"第\s*(\d{1,6})\s*页").expect("valid page number regex"));
    static RE_TRAILING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\d{1,6})\s*页?$").expect("valid page number regex"));
    for re in [&RE_SLASH, &RE_PAGE, &RE_CN, &RE_TRAILING] {
        if let Some(caps) = re.captures(trimmed) {
            if let Some(value) = caps.get(1) {
                if let Ok(number) = value.as_str().parse() {
                    return Some(number);
                }
            }
        }
    }
    parse_roman_page_number(&trimmed.to_ascii_uppercase())
        .or_else(|| parse_chinese_page_number(trimmed))
}

/// Classify a page-number's normalized form so format switches (roman → arabic,
/// plain → fraction) can also signal a section boundary.
fn page_number_kind(normalized_text: &str) -> &'static str {
    if normalized_text.contains("{roman-page}") {
        "roman"
    } else if normalized_text.contains("{page}/{total}")
        || normalized_text.contains("第{page}页")
        || normalized_text.contains("Page {page}")
    {
        "fraction"
    } else if normalized_text.contains("{page}") {
        "plain"
    } else {
        "other"
    }
}

/// Best-effort style label of a page-number line, for downstream editing UIs.
fn sequence_form_of(text: &str) -> &'static str {
    let has_circled = |ch: char| {
        let code = ch as u32;
        (0x2460..=0x2473).contains(&code) || (0x24EB..=0x24F4).contains(&code)
    };
    let has_dingbat = |ch: char| {
        let code = ch as u32;
        (0x2776..=0x277F).contains(&code)
    };
    if text.chars().any(has_circled) {
        "circled"
    } else if text.chars().any(has_dingbat) {
        "dingbat"
    } else if is_roman_page_marker(text) {
        "roman"
    } else if text.contains('/') || text.contains('共') {
        "fraction"
    } else if text.to_lowercase().contains("page") {
        "page-of"
    } else if text.contains('第') && text.contains('页') {
        "page-n"
    } else if text
        .chars()
        .any(|ch| "零一二三四五六七八九十百千〇两".contains(ch))
    {
        "chinese"
    } else {
        "arabic"
    }
}

fn text_has_total(text: &str) -> bool {
    text.contains('/') || text.contains('共') || text.to_lowercase().contains(" of ")
}

/// Split a page-number run (lines in page order) into segments whenever the
/// sequence is not strictly continuous (+1), the format switches (roman front
/// matter → arabic body), or a value cannot be parsed. Missing or garbled
/// pages must surface as separate segments instead of being hidden in one
/// merged range; position is the grouping criterion, but continuity decides
/// whether they stay together.
fn split_page_number_sequence<'a>(
    lines: &[&'a TextLineDetection],
) -> Vec<Vec<&'a TextLineDetection>> {
    let mut segments: Vec<Vec<&'a TextLineDetection>> = Vec::new();
    let mut prev_value: Option<u32> = None;
    let mut prev_kind: Option<&str> = None;
    for line in lines {
        let value = parsed_page_number_value(&line.text);
        let kind = page_number_kind(&line.normalized_text);
        let discontinuity = match (prev_value, value) {
            (Some(prev), Some(value)) => value != prev + 1,
            (Some(_), None) => true,
            _ => false,
        };
        let kind_changed = prev_kind.is_some_and(|prev| prev != kind);
        if segments.is_empty() || discontinuity || kind_changed {
            segments.push(Vec::new());
        }
        segments.last_mut().expect("segment exists").push(line);
        if let Some(value) = value {
            prev_value = Some(value);
        }
        prev_kind = Some(kind);
    }
    segments
}

/// Extract the denominator (total) from a page number string like "1/3 页" or "2/13".
/// Returns None if the text doesn't contain a "/" total separator.
fn parsed_page_number_total(text: &str) -> Option<u32> {
    static RE_SLASH_TOTAL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\d+\s*/\s*(\d+)").expect("valid slash-total regex"));
    RE_SLASH_TOTAL
        .captures(text)
        .and_then(|caps| caps.get(1)?.as_str().parse().ok())
}

fn parse_chinese_page_number(text: &str) -> Option<u32> {
    let value = text
        .chars()
        .filter(|ch| {
            matches!(
                ch,
                '零' | '〇'
                    | '一'
                    | '二'
                    | '两'
                    | '三'
                    | '四'
                    | '五'
                    | '六'
                    | '七'
                    | '八'
                    | '九'
                    | '十'
                    | '百'
                    | '千'
            )
        })
        .collect::<Vec<_>>();
    if value.is_empty() {
        return None;
    }
    let digit = |ch| match ch {
        '零' | '〇' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    };
    if !value.iter().any(|ch| matches!(ch, '十' | '百' | '千')) {
        return value.into_iter().try_fold(0_u32, |result, ch| {
            digit(ch).map(|value| result * 10 + value)
        });
    }
    let mut total = 0_u32;
    let mut current = 0_u32;
    for ch in value {
        if let Some(value) = digit(ch) {
            current = value;
            continue;
        }
        let unit = match ch {
            '十' => 10,
            '百' => 100,
            '千' => 1000,
            _ => return None,
        };
        total += current.max(1) * unit;
        current = 0;
    }
    Some(total + current)
}

fn normalize_header_footer_text(text: &str) -> String {
    // Expand circled/dingbat digits (①-⑳, ❶-❿, ⓫-⓴) to arabic so the
    // page-number pipeline recognizes them like any other numbering style.
    let circled_expanded: String = text
        .chars()
        .map(|ch| {
            circled_digit_text(ch)
                .map(str::to_string)
                .unwrap_or_else(|| ch.to_string())
        })
        .collect();
    let normalized_digits: String = circled_expanded
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
    // Recognize actual page number formats: "1/3 页", "第2页/共19页", "Page 5 of 20", "2/19"
    static PAGE_NUM_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?ix)
            \d+\s*/\s*\d+\s*页?$
            | 第\s*\d+\s*页\s*/\s*共\s*\d+\s*页
            | page\s*\d+\s*(?:of|/)\s*\d+
        ",
        )
        .unwrap()
    });
    if !labels.contains(&"page-number".to_string()) && PAGE_NUM_RE.is_match(normalized_text) {
        labels.push("page-number".to_string());
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

/// Match first-page evidence label text such as "证据1"、"对比文件3"、"证据一".
/// Digits may be Arabic or Chinese numerals; anything may follow the number
/// (e.g. "证据1（合同）").
fn is_evidence_label_text(normalized_text: &str) -> bool {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(证据|对比文件)\s*[0-9一二三四五六七八九十百千]+").unwrap());
    RE.is_match(normalized_text.trim())
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
                let (has_total, sequence_form) =
                    page_number_meta_for_range(pages, current_start, previous_page);
                items.push(SplitSuggestionItem {
                    name,
                    page_start: current_start,
                    page_end: previous_page,
                    source: current_source.clone(),
                    has_total,
                    sequence_form,
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
        let (has_total, sequence_form) =
            page_number_meta_for_range(pages, current_start, previous_page);
        items.push(SplitSuggestionItem {
            name,
            page_start: current_start,
            page_end: previous_page,
            source: current_source,
            has_total,
            sequence_form,
        });
    }

    items
}

/// 基于三层检测结果生成拆分建议（与分项证据处理共用同一套候选）。
///
/// 优先级：每页先找覆盖它的页眉候选（artifact 优先，其次重复内容候选），
/// 候选的 page_range 直接来自检测结果，天然携带稳定边界；没有任何页眉
/// 候选时（例如第三方合并的 PDF），回退到逐页第一条页眉文本对比。
fn build_split_suggestions(detection: &DetectionResult) -> Vec<SplitSuggestionItem> {
    let header_candidates: Vec<&HeaderFooterCandidate> = detection
        .header_candidates
        .iter()
        .filter(|candidate| candidate.region == "header")
        .filter(|candidate| !candidate.labels.iter().any(|label| label == "page-number"))
        .collect();

    if header_candidates.is_empty() {
        return build_split_suggestions_from_pages(&detection.pages);
    }

    // 预计算规范化页眉文本的出现页数，避免逐页重复做正则规范化。
    let mut header_text_counts: BTreeMap<String, usize> = BTreeMap::new();
    for page in &detection.pages {
        let mut seen = BTreeSet::new();
        for line in &page.headers {
            let normalized = normalize_header_footer_text(&line.text);
            if seen.insert(normalized.clone()) {
                *header_text_counts.entry(normalized).or_insert(0) += 1;
            }
        }
    }

    let mut items = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_source = "fallback".to_string();
    let mut current_start = 1_u32;
    let mut previous_page = 0_u32;

    for page in &detection.pages {
        let header = split_header_identity(&header_candidates, page, &header_text_counts);
        if previous_page == 0 {
            current_start = page.page;
            if let Some((name, source)) = header {
                current_name = Some(name);
                current_source = source;
            } else {
                current_name = Some("目录".to_string());
                current_source = "fallback".to_string();
            }
            previous_page = page.page;
            continue;
        }

        if let Some((new_name, source)) = header {
            if current_name.as_deref() != Some(new_name.as_str()) {
                let name = current_name
                    .take()
                    .unwrap_or_else(|| format!("文件{}", items.len() + 1));
                let (has_total, sequence_form) =
                    page_number_meta_for_range(&detection.pages, current_start, previous_page);
                items.push(SplitSuggestionItem {
                    name,
                    page_start: current_start,
                    page_end: previous_page,
                    source: current_source.clone(),
                    has_total,
                    sequence_form,
                });
                current_start = page.page;
                current_name = Some(new_name);
                current_source = source;
            }
        }
        previous_page = page.page;
    }

    if previous_page > 0 {
        let name = current_name.unwrap_or_else(|| format!("文件{}", items.len() + 1));
        let (has_total, sequence_form) =
            page_number_meta_for_range(&detection.pages, current_start, previous_page);
        items.push(SplitSuggestionItem {
            name,
            page_start: current_start,
            page_end: previous_page,
            source: current_source,
            has_total,
            sequence_form,
        });
    }

    items
}

/// 返回某页的页眉身份 (文本, source)。
///
/// 候选覆盖优先：同一页被多个候选覆盖时，取 artifact > 重复内容 > 普通候选，
/// 同优先级取出现次数多、置信度高的。无候选覆盖时，只接受"证据N/对比文件N"
/// 这类强信号或整篇重复出现的页眉行，避免正文噪声把页段切碎。
fn split_header_identity(
    candidates: &[&HeaderFooterCandidate],
    page: &PageDetection,
    header_text_counts: &BTreeMap<String, usize>,
) -> Option<(String, String)> {
    let covering = candidates
        .iter()
        .filter(|candidate| {
            candidate.page_range.start <= page.page && page.page <= candidate.page_range.end
        })
        .min_by(|left, right| {
            split_candidate_rank(left)
                .cmp(&split_candidate_rank(right))
                .then_with(|| right.count.cmp(&left.count))
                .then_with(|| right.confidence.total_cmp(&left.confidence))
        });
    if let Some(candidate) = covering {
        return Some((candidate.text.clone(), candidate.source.clone()));
    }

    let text = best_split_header(page)?;
    let normalized = normalize_header_footer_text(&text);
    if is_evidence_label_text(&normalized)
        || header_text_counts.get(&normalized).copied().unwrap_or(0) >= 2
    {
        return Some((text, "header".to_string()));
    }
    None
}

/// 候选身份优先级：数值越小越优先。
fn split_candidate_rank(candidate: &HeaderFooterCandidate) -> u8 {
    if candidate.source == "artifact" {
        0
    } else if candidate.repeating {
        1
    } else {
        2
    }
}

/// Extract page-number has_total and sequence_form from the footers of a page range.
fn page_number_meta_for_range(
    pages: &[PageDetection],
    page_start: u32,
    page_end: u32,
) -> (Option<bool>, Option<String>) {
    for page in pages {
        if page.page >= page_start && page.page <= page_end {
            for line in &page.footers {
                let labels = labels_for(&line.normalized_text);
                if labels.iter().any(|l| l == "page-number") {
                    return (
                        Some(text_has_total(&line.text)),
                        Some(sequence_form_of(&line.text).to_string()),
                    );
                }
            }
        }
    }
    (None, None)
}

/// If a page-number sequence boundary (where the total value changes) falls
/// between two header-based splits, add it as an additional split point.
fn augment_splits_with_page_number_boundaries(
    items: Vec<SplitSuggestionItem>,
    pages: &[PageDetection],
) -> Vec<SplitSuggestionItem> {
    // Collect boundary pages where the page-number total changes
    let mut prev_total: Option<u32> = None;
    let mut boundaries: Vec<u32> = Vec::new();
    for page in pages {
        let page_total = page.footers.iter().find_map(|line| {
            let labels = labels_for(&line.normalized_text);
            if labels.iter().any(|l| l == "page-number") {
                parsed_page_number_total(&line.text)
            } else {
                None
            }
        });
        if let Some(total) = page_total {
            if prev_total.is_some() && prev_total != Some(total) {
                boundaries.push(page.page);
            }
            prev_total = Some(total);
        }
    }

    if boundaries.is_empty() {
        return items;
    }

    // For each boundary, split the existing range that contains it
    let mut new_items: Vec<SplitSuggestionItem> = Vec::new();
    for item in &items {
        let mut cursor = item.page_start;
        let mut boundary_pages_in_range: Vec<u32> = boundaries
            .iter()
            .copied()
            .filter(|&p| p > item.page_start && p <= item.page_end)
            .collect();
        boundary_pages_in_range.sort_unstable();
        boundary_pages_in_range.dedup();

        for boundary in boundary_pages_in_range {
            if cursor < boundary {
                let (has_total, sequence_form) =
                    page_number_meta_for_range(pages, cursor, boundary - 1);
                new_items.push(SplitSuggestionItem {
                    name: item.name.clone(),
                    page_start: cursor,
                    page_end: boundary - 1,
                    source: item.source.clone(),
                    has_total,
                    sequence_form,
                });
            }
            cursor = boundary;
        }
        if cursor <= item.page_end {
            let (has_total, sequence_form) =
                page_number_meta_for_range(pages, cursor, item.page_end);
            new_items.push(SplitSuggestionItem {
                name: item.name.clone(),
                page_start: cursor,
                page_end: item.page_end,
                source: item.source.clone(),
                has_total,
                sequence_form,
            });
        }
    }

    new_items
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
        assert_eq!(parsed_page_number_value("第四页"), Some(4));
        assert_eq!(parsed_page_number_value("第二十三页"), Some(23));
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
        assert!(candidates[0].repeating);
        assert!(candidates[0].position_stable);
        assert!(candidates[0].sequence_stable);
    }

    #[test]
    fn repeated_numbers_require_a_page_sequence() {
        let page = |page: u32, value: &str| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![],
            footers: vec![TextLineDetection {
                text: value.to_string(),
                normalized_text: "{page}".to_string(),
                bbox: BBox {
                    x0: 540.0,
                    y0: 800.0,
                    x1: 550.0,
                    y1: 820.0,
                    page,
                    width: 595.0,
                    height: 842.0,
                },
                font_size: None,
            }],
        };
        let candidates =
            build_candidates(&[page(1, "10"), page(2, "50"), page(3, "3")], "footer", 3);
        assert!(
            candidates.is_empty(),
            "unordered numbers are not a reliable page-number sequence"
        );
    }

    #[test]
    fn page_number_shape_does_not_override_unstable_positions() {
        let page = |page: u32, x0: f32| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![],
            footers: vec![TextLineDetection {
                text: page.to_string(),
                normalized_text: "{page}".to_string(),
                bbox: BBox {
                    x0,
                    y0: 800.0,
                    x1: x0 + 10.0,
                    y1: 820.0,
                    page,
                    width: 595.0,
                    height: 842.0,
                },
                font_size: None,
            }],
        };
        let candidates = build_candidates(
            &[page(1, 20.0), page(2, 290.0), page(3, 540.0)],
            "footer",
            3,
        );
        assert!(
            candidates.is_empty(),
            "page-number-shaped text at unstable positions is not a candidate"
        );
    }

    #[test]
    fn standard_artifact_candidate_precedes_text_heuristics() {
        let inspection = super::super::artifacts::HeaderFooterArtifactInspection {
            header_count: 2,
            footer_count: 0,
            occurrences: vec![
                super::super::artifacts::HeaderFooterArtifactOccurrence {
                    id: "docsy-header-1".to_string(),
                    page: 1,
                    region: "header",
                    text: Some("证据一".to_string()),
                    docsy_kind: Some("HeaderText".to_string()),
                    docsy_id: Some("docsy-header-1".to_string()),
                    object_ref: Some("10 0 R".to_string()),
                    operation_index: 0,
                    visual_object_ref: None,
                },
                super::super::artifacts::HeaderFooterArtifactOccurrence {
                    id: "docsy-header-2".to_string(),
                    page: 2,
                    region: "header",
                    text: Some("证据一".to_string()),
                    docsy_kind: Some("HeaderText".to_string()),
                    docsy_id: Some("docsy-header-2".to_string()),
                    object_ref: Some("11 0 R".to_string()),
                    operation_index: 0,
                    visual_object_ref: None,
                },
            ],
        };
        let pages = vec![
            PageDetection {
                page: 1,
                width: 595.0,
                height: 842.0,
                headers: Vec::new(),
                footers: Vec::new(),
            },
            PageDetection {
                page: 2,
                width: 595.0,
                height: 842.0,
                headers: Vec::new(),
                footers: Vec::new(),
            },
        ];
        let candidates = build_artifact_candidates(&inspection, &pages, &[], &[]);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source, "artifact");
        assert_eq!(candidates[0].artifact_id.as_deref(), Some("docsy-header-1"));
        assert!(candidates[0].repeating);
        assert!(candidates[0].position_stable);
    }

    #[test]
    fn candidate_repetition_counts_distinct_pages() {
        // Use genuinely ordinary text here: "证据一" on page 1 is an evidence
        // label and is promoted without repetition by design.
        let line = |page: u32| TextLineDetection {
            text: "附 页".to_string(),
            normalized_text: "附 页".to_string(),
            bbox: BBox {
                x0: 500.0,
                y0: 20.0,
                x1: 550.0,
                y1: 32.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let pages = vec![PageDetection {
            page: 1,
            width: 595.0,
            height: 842.0,
            headers: vec![line(1), line(1)],
            footers: vec![],
        }];

        // A single page cannot prove that ordinary page content is a header.
        let candidates = build_candidates(&pages, "header", 1);
        assert!(
            candidates.is_empty(),
            "single-page ordinary text must remain page data, not a candidate"
        );

        // In a two-page document, the same stable text on both pages is enough
        // to establish cross-page repetition.
        let pages_2 = vec![
            PageDetection {
                page: 1,
                width: 595.0,
                height: 842.0,
                headers: vec![line(1)],
                footers: vec![],
            },
            PageDetection {
                page: 2,
                width: 595.0,
                height: 842.0,
                headers: vec![line(2)],
                footers: vec![],
            },
        ];
        let candidates_2 = build_candidates(&pages_2, "header", 2);
        assert_eq!(candidates_2.len(), 1);
        assert_eq!(candidates_2[0].count, 2);
        assert!(candidates_2[0].repeating);

        // 3 pages should be repeating
        let pages_3 = vec![
            PageDetection {
                page: 1,
                width: 595.0,
                height: 842.0,
                headers: vec![line(1)],
                footers: vec![],
            },
            PageDetection {
                page: 2,
                width: 595.0,
                height: 842.0,
                headers: vec![line(2)],
                footers: vec![],
            },
            PageDetection {
                page: 3,
                width: 595.0,
                height: 842.0,
                headers: vec![line(3)],
                footers: vec![],
            },
        ];
        let candidates_3 = build_candidates(&pages_3, "header", 3);
        assert_eq!(candidates_3.len(), 1);
        assert_eq!(candidates_3[0].count, 3);
        assert!(candidates_3[0].repeating);
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
                    has_total: None,
                    sequence_form: None,
                },
                SplitSuggestionItem {
                    name: "证据二".to_string(),
                    page_start: 3,
                    page_end: 3,
                    source: "header".to_string(),
                    has_total: None,
                    sequence_form: None,
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
    fn split_suggestions_use_artifact_candidate_ranges() {
        let page = |page: u32, header: &str| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![TextLineDetection {
                text: header.to_string(),
                normalized_text: header.to_string(),
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
        let candidate = |text: &str, start: u32, end: u32| HeaderFooterCandidate {
            text: text.to_string(),
            normalized_text: text.to_string(),
            region: "header".to_string(),
            page_range: PageRange { start, end },
            count: (end - start + 1) as usize,
            repeating: true,
            position_stable: true,
            position_spread: 0.0,
            sequence_stable: true,
            labels: vec![],
            confidence: 1.0,
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 10.0,
                page: start,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
            source: "artifact".to_string(),
            artifact_id: None,
            docsy_kind: None,
            sequence_form: None,
            has_total: None,
        };
        let detection = DetectionResult {
            input_path: "/tmp/merged.pdf".to_string(),
            pages_analyzed: 6,
            artifact: ArtifactSummary::default(),
            pages: vec![
                page(1, "证据一"),
                page(2, "证据一"),
                page(3, "证据二"),
                page(4, "证据二"),
                page(5, "证据三"),
                page(6, "证据三"),
            ],
            header_candidates: vec![
                candidate("证据一", 1, 2),
                candidate("证据二", 3, 4),
                candidate("证据三", 5, 6),
            ],
            footer_candidates: vec![],
        };
        let items = build_split_suggestions(&detection);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name, "证据一");
        assert_eq!((items[0].page_start, items[0].page_end), (1, 2));
        assert_eq!(items[0].source, "artifact");
        assert_eq!(items[1].name, "证据二");
        assert_eq!((items[1].page_start, items[1].page_end), (3, 4));
        assert_eq!(items[2].name, "证据三");
        assert_eq!((items[2].page_start, items[2].page_end), (5, 6));
    }

    #[test]
    fn split_suggestions_fallback_accepts_evidence_label_once() {
        // 无候选（header_candidates 为空）时走文本路径；
        // "证据9" 只出现在一页（首页规则），也应识别为页段边界。
        let page = |page: u32, header: Option<&str>| PageDetection {
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
            footers: vec![],
        };
        let detection = DetectionResult {
            input_path: "/tmp/merged.pdf".to_string(),
            pages_analyzed: 5,
            artifact: ArtifactSummary::default(),
            pages: vec![
                page(1, Some("证据8")),
                page(2, Some("证据8")),
                page(3, Some("证据9")),
                page(4, Some("证据9")),
                page(5, None),
            ],
            header_candidates: vec![],
            footer_candidates: vec![],
        };
        let items = build_split_suggestions(&detection);
        assert_eq!(items.len(), 2);
        assert_eq!((items[0].page_start, items[0].page_end), (1, 2));
        assert_eq!((items[1].page_start, items[1].page_end), (3, 5));
    }

    #[test]
    fn split_suggestions_ignore_single_page_noise_header() {
        // 候选存在但不覆盖第 2 页，且该页页眉是只出现一次的非证据文本：
        // 不应产生切分点（避免正文噪声把页段切碎）。
        let page = |page: u32, header: &str| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![TextLineDetection {
                text: header.to_string(),
                normalized_text: header.to_string(),
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
        let candidate = |text: &str, start: u32, end: u32| HeaderFooterCandidate {
            text: text.to_string(),
            normalized_text: text.to_string(),
            region: "header".to_string(),
            page_range: PageRange { start, end },
            count: (end - start + 1) as usize,
            repeating: true,
            position_stable: true,
            position_spread: 0.0,
            sequence_stable: true,
            labels: vec![],
            confidence: 1.0,
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 10.0,
                page: start,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
            source: "artifact".to_string(),
            artifact_id: None,
            docsy_kind: None,
            sequence_form: None,
            has_total: None,
        };
        let detection = DetectionResult {
            input_path: "/tmp/merged.pdf".to_string(),
            pages_analyzed: 4,
            artifact: ArtifactSummary::default(),
            pages: vec![
                page(1, "证据一"),
                page(2, "一次性噪声页眉"),
                page(3, "证据一"),
                page(4, "证据一"),
            ],
            header_candidates: vec![candidate("证据一", 1, 4)],
            footer_candidates: vec![],
        };
        let items = build_split_suggestions(&detection);
        assert_eq!(items.len(), 1);
        assert_eq!((items[0].page_start, items[0].page_end), (1, 4));
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

    #[test]
    fn labels_for_page_number_patterns() {
        // Template markers
        assert!(labels_for("{page}/{total}").contains(&"page-number".to_string()));
        // With 页
        assert!(labels_for("CN 117457788 B 1/3 页").contains(&"page-number".to_string()));
        // Without 页
        assert!(labels_for("CN 117457788 B 1/3").contains(&"page-number".to_string()));
        // 第X页/共Y页
        assert!(labels_for("第2页/共19页").contains(&"page-number".to_string()));
        // English
        assert!(labels_for("Page 5 of 20").contains(&"page-number".to_string()));
        // Pure number - should NOT be page-number (ambiguous)
        assert!(!labels_for("2").contains(&"page-number".to_string()));
        // Regular header text
        assert!(!labels_for("说 明 书").contains(&"page-number".to_string()));
    }

    #[test]
    fn page_number_sequence_splits_on_section_resets() {
        let line = |page: u32, text: &str| TextLineDetection {
            text: text.to_string(),
            normalized_text: "{page}".to_string(),
            bbox: BBox {
                x0: 260.0,
                y0: 800.0,
                x1: 290.0,
                y1: 812.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let lines = [
            line(2, "1"),
            line(3, "2"),
            line(4, "3"),
            line(5, "1"),
            line(6, "2"),
        ];
        let refs: Vec<&TextLineDetection> = lines.iter().collect();
        let segments = split_page_number_sequence(&refs);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].len(), 3);
        assert_eq!(segments[1].len(), 2);
    }

    #[test]
    fn page_number_sequence_splits_on_format_switch() {
        // Roman front matter → arabic body, values stay continuous (V/VI then 7/8)
        let line = |page: u32, text: &str, norm: &str| TextLineDetection {
            text: text.to_string(),
            normalized_text: norm.to_string(),
            bbox: BBox {
                x0: 260.0,
                y0: 800.0,
                x1: 290.0,
                y1: 812.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let lines = [
            line(1, "I", "{roman-page}"),
            line(2, "II", "{roman-page}"),
            line(3, "III", "{roman-page}"),
            line(4, "IV", "{roman-page}"),
            line(5, "V", "{roman-page}"),
            line(6, "VI", "{roman-page}"),
            line(7, "7", "{page}"),
            line(8, "8", "{page}"),
        ];
        let refs: Vec<&TextLineDetection> = lines.iter().collect();
        let segments = split_page_number_sequence(&refs);
        assert_eq!(
            segments.len(),
            2,
            "roman→arabic must split even without a value reset"
        );
        assert_eq!(segments[0].len(), 6);
        assert_eq!(segments[1].len(), 2);
    }

    #[test]
    fn page_number_sequence_splits_unordered_values() {
        let line = |page: u32, text: &str| TextLineDetection {
            text: text.to_string(),
            normalized_text: "{page}".to_string(),
            bbox: BBox {
                x0: 540.0,
                y0: 800.0,
                x1: 550.0,
                y1: 820.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let lines = [line(1, "10"), line(2, "50"), line(3, "3")];
        let refs: Vec<&TextLineDetection> = lines.iter().collect();
        // 10 → 50 → 3: neither step is +1, so each value becomes its own segment
        let segments = split_page_number_sequence(&refs);
        assert_eq!(
            segments.len(),
            3,
            "unordered values must be surfaced separately"
        );
    }

    #[test]
    fn page_number_sequence_splits_on_gaps() {
        let line = |page: u32, text: &str| TextLineDetection {
            text: text.to_string(),
            normalized_text: "{page}".to_string(),
            bbox: BBox {
                x0: 540.0,
                y0: 800.0,
                x1: 550.0,
                y1: 820.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let lines = [line(1, "1"), line(2, "2"), line(3, "4"), line(4, "5")];
        let refs: Vec<&TextLineDetection> = lines.iter().collect();
        // 2 → 4 skips 3: the missing page must surface as a separate segment
        let segments = split_page_number_sequence(&refs);
        assert_eq!(segments.len(), 2, "gap in the sequence must split segments");
        assert_eq!(segments[0].len(), 2);
        assert_eq!(segments[1].len(), 2);
    }

    #[test]
    fn circled_page_numbers_normalize_and_classify() {
        // Circled digits expand to arabic, then standalone numbers normalize to {page}
        assert_eq!(normalize_header_footer_text("①"), "{page}");
        assert_eq!(normalize_header_footer_text("⑫"), "{page}");
        assert_eq!(normalize_header_footer_text("❶"), "{page}");
        assert_eq!(normalize_header_footer_text("⓫"), "{page}");
        assert!(labels_for("{page}").contains(&"page-number".to_string()));
        assert_eq!(sequence_form_of("①"), "circled");
        assert_eq!(sequence_form_of("❶"), "dingbat");
        assert_eq!(sequence_form_of("1/3 页"), "fraction");
        assert_eq!(sequence_form_of("I"), "roman");
        assert!(text_has_total("1/3 页"));
        assert!(!text_has_total("2"));
    }

    #[test]
    fn sectioned_page_numbers_produce_separate_candidates() {
        let line = |page: u32, text: &str| TextLineDetection {
            text: text.to_string(),
            normalized_text: "{page}".to_string(),
            bbox: BBox {
                x0: 260.0,
                y0: 800.0,
                x1: 290.0,
                y1: 812.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let mk = |page: u32, text: &str| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers: vec![],
            footers: vec![line(page, text)],
        };
        let pages = vec![mk(2, "1"), mk(3, "2"), mk(4, "3"), mk(5, "1"), mk(6, "2")];
        let candidates = build_candidates(&pages, "footer", 5);
        // Pure numbers normalize to the {page} template; page-number candidates
        // are exempt from the merge step, so both sections stay separate.
        let pn: Vec<_> = candidates
            .iter()
            .filter(|c| c.normalized_text.contains("{page}"))
            .collect();
        assert_eq!(pn.len(), 2);
        let starts: Vec<u32> = pn.iter().map(|c| c.page_range.start).collect();
        assert!(starts.contains(&2));
        assert!(starts.contains(&5));
        // No candidate spans across the section boundary
        assert!(pn
            .iter()
            .all(|c| !(c.page_range.start <= 3 && c.page_range.end >= 5)));
    }
    #[test]
    fn first_page_evidence_label_survives_repetition_gate() {
        let line = |page: u32, text: &str, normalized: &str| TextLineDetection {
            text: text.to_string(),
            normalized_text: normalized.to_string(),
            bbox: BBox {
                x0: 260.0,
                y0: 20.0,
                x1: 290.0,
                y1: 32.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let mk = |page: u32, headers: Vec<TextLineDetection>| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers,
            footers: vec![],
        };
        // "证据１" (fullwidth digit) appears only on page 1, like the stamped
        // evidence labels in real merged-evidence PDFs.
        let pages = vec![
            mk(1, vec![line(1, "证据１", "证据1")]),
            mk(2, vec![]),
            mk(3, vec![]),
            mk(4, vec![]),
            mk(5, vec![]),
        ];
        let candidates = build_candidates(&pages, "header", 5);
        let evidence = candidates
            .iter()
            .find(|candidate| candidate.normalized_text == "证据1")
            .expect("first-page evidence label must remain a candidate");
        assert!(evidence
            .labels
            .iter()
            .any(|label| label == "evidence-label"));
        assert_eq!(evidence.count, 1);
        assert!(!evidence.repeating, "a single occurrence is not repetition");
    }

    #[test]
    fn evidence_label_not_on_first_page_is_not_promoted() {
        let line = |page: u32| TextLineDetection {
            text: "证据3".to_string(),
            normalized_text: "证据3".to_string(),
            bbox: BBox {
                x0: 260.0,
                y0: 20.0,
                x1: 290.0,
                y1: 32.0,
                page,
                width: 595.0,
                height: 842.0,
            },
            font_size: None,
        };
        let mk = |page: u32, headers: Vec<TextLineDetection>| PageDetection {
            page,
            width: 595.0,
            height: 842.0,
            headers,
            footers: vec![],
        };
        // A single "证据3" first appearing on page 2 is more likely body text
        // than a stamped label; without repetition it is not a candidate.
        let pages = vec![mk(1, vec![]), mk(2, vec![line(2)]), mk(3, vec![])];
        let candidates = build_candidates(&pages, "header", 3);
        assert!(candidates.is_empty());
    }

    #[test]
    fn evidence_label_text_matches_expected_shapes() {
        assert!(is_evidence_label_text("证据1"));
        assert!(is_evidence_label_text("证据 12"));
        assert!(is_evidence_label_text("对比文件3"));
        assert!(is_evidence_label_text("证据一"));
        assert!(is_evidence_label_text("证据1（合同）"));
        assert!(!is_evidence_label_text("证据"));
        assert!(!is_evidence_label_text("该证据1"));
        assert!(!is_evidence_label_text("证据目录"));
    }

    #[test]
    #[ignore = "requires the real patent PDF and pdftotext"]
    fn detect_evidence9_header_candidates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("test-pdf/4W122724 I D1-D14/09 证据 9. 国家知识产权局第 587099 号无效宣告请求审查决定书.pdf");
        if !path.exists() {
            eprintln!("skipping: file not found");
            return;
        }
        let result = detect(&DetectionArgs {
            input_path: path.to_string_lossy().to_string(),
            max_pages: 35,
            header_zone_mm: Some(25.0),
            footer_zone_mm: Some(25.0),
            header_zone_ratio: default_header_zone_ratio(),
            footer_zone_ratio: default_footer_zone_ratio(),
            scan_artifacts: true,
        })
        .unwrap();
        eprintln!(
            "=== Header Candidates ({} total) ===",
            result.header_candidates.len()
        );
        for (i, c) in result.header_candidates.iter().enumerate() {
            eprintln!(
                "[{}] source={} text={:?} norm={:?} count={} repeating={} conf={:.3} pages={}-{} bbox=({:.1},{:.1},{:.1},{:.1}) page={}",
                i, c.source, c.text, c.normalized_text,
                c.count, c.repeating, c.confidence,
                c.page_range.start, c.page_range.end,
                c.bbox.x0, c.bbox.y0, c.bbox.x1, c.bbox.y1, c.bbox.page,
            );
        }
    }

    #[test]
    #[ignore = "requires the local PDF fixture directory, qpdf, and pdftotext"]
    fn evidence_label_artifact_bbox_comes_from_matching_content_line() {
        // 回归（证据2/证据5）：首页“证据N”artifact 所在页眉区还有每页重复的
        // 专利号页眉（如“등록특허 10-1569508”）。supporting 候选必须按文本匹配，
        // 否则会把专利号页眉的 bbox 错贴到“证据N”上，预览删除标记错位。
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("test-pdf/4W122724 I D1-D14");
        for name in [
            "01 证据 1. 日本专利 JP2017186663A 及其中文译文.pdf",
            "02 证据 2. 韩国专利 KR101569508B1 及其中文译文.pdf",
            "05 证据5. 韩国专利 KR101858868B1 及其中文译文.pdf",
            "04 证据 4. 中国发明专利申请公开文本 CN108431273A.pdf",
        ] {
            let path = dir.join(name);
            if !path.exists() {
                eprintln!("skipping: {} not found", path.display());
                continue;
            }
            let result = detect(&DetectionArgs {
                input_path: path.to_string_lossy().to_string(),
                max_pages: 45,
                header_zone_mm: Some(25.0),
                footer_zone_mm: Some(25.0),
                header_zone_ratio: default_header_zone_ratio(),
                footer_zone_ratio: default_footer_zone_ratio(),
                scan_artifacts: true,
            })
            .unwrap();
            let label = result
                .header_candidates
                .iter()
                .find(|c| c.source == "artifact" && c.page_range.start == 1)
                .unwrap_or_else(|| panic!("{name}: first-page artifact header missing"));
            // 首页“证据N”标签绘制在页面右上约 (493,23)-(523,35)；同页的专利号
            // 页眉在 y≈42-66。错位修复前 02/05 会得到 y0≈56.7 的 bbox。
            assert!(
                label.bbox.y0 < 40.0 && label.bbox.x0 > 480.0,
                "{name}: evidence label bbox misplaced: {:?}",
                label.bbox,
            );
        }
    }

    #[test]
    #[ignore = "requires the local PDF fixture directory, qpdf, and pdftotext"]
    fn detect_fixture_directory_without_unreliable_content_candidates() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("test-pdf/4W122724 I D1-D14");
        if !fixture_dir.exists() {
            eprintln!("skipping: fixture directory not found");
            return;
        }
        let mut paths = std::fs::read_dir(&fixture_dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("pdf"))
            .filter(|path| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with("00_")
                    || name
                        .chars()
                        .take(2)
                        .all(|character| character.is_ascii_digit())
            })
            .filter(|path| !path.to_string_lossy().contains("compressed"))
            .collect::<Vec<_>>();
        paths.sort();

        for path in paths {
            let started = std::time::Instant::now();
            let result = detect(&DetectionArgs {
                input_path: path.to_string_lossy().to_string(),
                max_pages: 40,
                header_zone_mm: Some(25.0),
                footer_zone_mm: Some(25.0),
                header_zone_ratio: default_header_zone_ratio(),
                footer_zone_ratio: default_footer_zone_ratio(),
                scan_artifacts: true,
            })
            .unwrap_or_else(|error| panic!("{}: {error:#}", path.display()));
            for candidate in result
                .header_candidates
                .iter()
                .chain(result.footer_candidates.iter())
                .filter(|candidate| candidate.source == "content-text")
            {
                // First-page evidence labels legitimately occur only once.
                if candidate
                    .labels
                    .iter()
                    .any(|label| label == "evidence-label")
                {
                    continue;
                }
                assert!(candidate.repeating, "{}: {candidate:?}", path.display());
                assert!(
                    candidate.position_stable,
                    "{}: {candidate:?}",
                    path.display()
                );
                if candidate.labels.iter().any(|label| label == "page-number") {
                    assert!(
                        candidate.sequence_stable,
                        "{}: {candidate:?}",
                        path.display()
                    );
                }
            }
            eprintln!(
                "detected {:?} in {}ms: {} headers, {} footers",
                path.file_name().unwrap_or_default(),
                started.elapsed().as_millis(),
                result.header_candidates.len(),
                result.footer_candidates.len(),
            );
        }
    }
}
