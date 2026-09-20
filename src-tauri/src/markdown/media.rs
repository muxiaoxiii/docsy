//! Locate rich Markdown using parser byte ranges; render only requested sources locally.
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, ops::Range};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preparation {
    pub source_hash: String,
    pub items: Vec<MediaRequest>,
}
#[derive(Debug, Serialize)]
pub struct MediaRequest {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub display: bool,
    #[serde(skip)]
    pub range: Range<usize>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedMarkdown {
    pub source_hash: String,
    pub items: Vec<RenderedMedia>,
}
#[derive(Debug, Deserialize)]
pub struct RenderedMedia {
    pub id: String,
    pub png: String,
    pub width: f64,
    pub height: f64,
    pub mathml: Option<String>,
}
pub struct Asset {
    pub png: Vec<u8>,
    pub width: f64,
    pub height: f64,
    pub omml: Option<String>,
    pub equation_token: String,
    pub display: bool,
    pub kind: String,
}
pub struct RichMarkdown {
    pub text: String,
    pub assets: HashMap<String, Asset>,
    pub fallback_count: usize,
}

pub fn prepare(text: &str) -> Preparation {
    let mut items = Vec::new();
    let mut protected = Vec::<Range<usize>>::new();
    let options = Options::ENABLE_TABLES | Options::ENABLE_MATH | Options::ENABLE_STRIKETHROUGH;
    let mut code: Option<(usize, bool, String)> = None;
    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let mermaid = matches!(kind, CodeBlockKind::Fenced(info) if info.split_whitespace().next().is_some_and(|s| s.eq_ignore_ascii_case("mermaid")));
                code = Some((range.start, mermaid, String::new()));
            }
            Event::Text(source) if code.is_some() => code.as_mut().unwrap().2.push_str(&source),
            Event::End(TagEnd::CodeBlock) => {
                if let Some((start, mermaid, source)) = code.take() {
                    let block = start..range.end;
                    protected.push(block.clone());
                    if mermaid {
                        items.push(MediaRequest {
                            id: String::new(),
                            kind: "mermaid".into(),
                            source,
                            display: true,
                            range: block,
                        });
                    }
                }
            }
            Event::InlineMath(source) | Event::DisplayMath(source) => {
                let display = text[range.clone()].starts_with("$$");
                protected.push(range.clone());
                // A price pair such as "$5 and $10" is not a math span.
                if !display
                    && source.starts_with(|c: char| c.is_ascii_digit())
                    && text[range.end..].starts_with(|c: char| c.is_ascii_digit())
                {
                    continue;
                }
                items.push(MediaRequest {
                    id: String::new(),
                    kind: "math".into(),
                    source: source.into_string(),
                    display,
                    range,
                });
            }
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) => protected.push(range),
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => protected.push(range),
            _ => {}
        }
    }
    // Common LaTeX delimiters are escaped punctuation to CommonMark. Find them
    // only outside parsed code/HTML/links/math, preserving original byte offsets.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] != b'\\'
            || !matches!(bytes[i + 1], b'(' | b'[')
            || protected.iter().any(|r| r.contains(&i))
        {
            i += 1;
            continue;
        }
        let preceding = bytes[..i].iter().rev().take_while(|b| **b == b'\\').count();
        if preceding % 2 == 1 {
            i += 2;
            continue;
        }
        let display = bytes[i + 1] == b'[';
        let close = if display { "\\]" } else { "\\)" };
        if let Some(offset) = text[i + 2..].find(close) {
            let end = i + 2 + offset + 2;
            if !protected.iter().any(|r| r.start < end && r.end > i) {
                let source = &text[i + 2..end - 2];
                if !source.trim().is_empty() {
                    items.push(MediaRequest {
                        id: String::new(),
                        kind: "math".into(),
                        source: source.into(),
                        display,
                        range: i..end,
                    });
                    i = end;
                    continue;
                }
            }
        }
        i += 2;
    }
    items.sort_by_key(|item| item.range.start);
    for (index, item) in items.iter_mut().enumerate() {
        item.id = format!("docsy-media-{index}");
    }
    Preparation {
        source_hash: format!("{:x}", Sha256::digest(text.as_bytes())),
        items,
    }
}

/// Apply the same limits at preparation and at the conversion trust boundary.
pub fn prepare_checked(text: &str) -> Result<Preparation> {
    if text.len() > 16 * 1024 * 1024 {
        bail!("Markdown 文本过大，请分批转换");
    }
    let plan = prepare(text);
    if plan.items.len() > 500 || plan.items.iter().any(|item| item.source.len() > 50_000) {
        bail!("公式或图表过多/过长，请拆分后转换");
    }
    Ok(plan)
}

pub fn resolve(text: &str, rendered: Option<&RenderedMarkdown>) -> Result<RichMarkdown> {
    let plan = prepare_checked(text)?;
    if rendered.is_some_and(|data| data.items.len() > 500) {
        bail!("公式或图表过多，请拆分后转换");
    }
    let mut result = RichMarkdown {
        text: text.into(),
        assets: HashMap::new(),
        fallback_count: 0,
    };
    if rendered.is_some_and(|data| data.source_hash != plan.source_hash) {
        bail!("Markdown 在渲染期间发生变化，请重新转换");
    }
    if plan.items.is_empty() {
        return Ok(result);
    }
    let rendered = rendered.context("文档含公式或 Mermaid，请从 MD 转换页面进行本地渲染后导出")?;
    if rendered.source_hash != plan.source_hash {
        bail!("Markdown 在渲染期间发生变化，请重新转换");
    }
    if rendered.items.len() != plan.items.len() {
        bail!("公式或 Mermaid 渲染结果不完整，请重新转换");
    }
    let mut total = 0;
    for request in plan.items.iter().rev() {
        let matches: Vec<_> = rendered
            .items
            .iter()
            .filter(|item| item.id == request.id)
            .collect();
        if matches.len() != 1 {
            bail!("公式或 Mermaid 渲染结果缺失或重复");
        }
        let item = matches[0];
        if !item.width.is_finite()
            || !item.height.is_finite()
            || item.width <= 0.0
            || item.height <= 0.0
            || item.width > 10000.0
            || item.height > 10000.0
        {
            bail!("渲染图片尺寸无效");
        }
        if item.png.len() > 32 * 1024 * 1024 {
            bail!("单张公式或图表过大，请拆分内容");
        }
        let png = STANDARD
            .decode(
                item.png
                    .strip_prefix("data:image/png;base64,")
                    .context("渲染图片必须是 PNG")?,
            )
            .context("渲染图片编码无效")?;
        total += png.len();
        if total > 128 * 1024 * 1024 {
            bail!("公式与图表总量过大，请分批转换");
        }
        let reader =
            image::ImageReader::with_format(std::io::Cursor::new(&png), image::ImageFormat::Png);
        let (w, h) = reader.into_dimensions().context("渲染 PNG 无效")?;
        if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 32_000_000 {
            bail!("渲染图片像素过大");
        }
        if item
            .mathml
            .as_ref()
            .is_some_and(|xml| xml.len() > 1_000_000)
        {
            bail!("公式结构过大，请拆分内容");
        }
        let omml = if request.kind == "math" {
            item.mathml.as_deref().and_then(super::omml::from_mathml)
        } else {
            None
        };
        if request.kind == "math" && omml.is_none() {
            result.fallback_count += 1;
        }
        let replacement = format!(
            "![{}]({})",
            if request.kind == "math" {
                "公式"
            } else {
                "Mermaid 图"
            },
            request.id
        );
        // Code blocks own their closing newline; retain a block boundary.
        let replacement = if request.display {
            format!("\n\n{replacement}\n\n")
        } else {
            replacement
        };
        result
            .text
            .replace_range(request.range.clone(), &replacement);
        result.assets.insert(
            request.id.clone(),
            Asset {
                png,
                equation_token: unused_equation_token(text, &request.id),
                width: item.width,
                height: item.height,
                omml,
                display: request.display,
                kind: request.kind.clone(),
            },
        );
    }
    Ok(result)
}

fn unused_equation_token(text: &str, id: &str) -> String {
    // A token is internal, and must never alias literal user text.
    let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
    let mut token = format!("DOCSYOMML{digest}{}END", id.replace('-', ""));
    while text.contains(&token) {
        token.push('X');
    }
    token
}

pub fn inject_equations(bytes: Vec<u8>, assets: &HashMap<String, Asset>) -> Result<Vec<u8>> {
    use std::io::{Cursor, Read, Write};
    if !assets.values().any(|asset| asset.omml.is_some()) {
        return Ok(bytes);
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let run = regex::Regex::new(r"(?s)<w:r(?:\s[^>]*)?>.*?</w:r>")?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let options = zip::write::FileOptions::default().compression_method(entry.compression());
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        if name == "word/document.xml" {
            let mut xml = String::from_utf8(data)?;
            xml = xml.replacen("<w:document ", "<w:document xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\" ", 1);
            let replacements: HashMap<_, _> = assets
                .values()
                .filter_map(|asset| {
                    asset
                        .omml
                        .as_ref()
                        .map(|omml| (asset.equation_token.as_str(), omml.as_str()))
                })
                .collect();
            let text = regex::Regex::new(r"<w:t(?:\s[^>]*)?>([^<]*)</w:t>")?;
            xml = run
                .replace_all(&xml, |caps: &regex::Captures| {
                    text.captures(&caps[0])
                        .and_then(|t| replacements.get(&t[1]))
                        .map(|omml| (*omml).to_owned())
                        .unwrap_or_else(|| caps[0].to_owned())
                })
                .into_owned();
            if replacements.keys().any(|token| xml.contains(token)) {
                bail!("原生公式写入失败，请重新转换");
            }
            data = xml.into_bytes();
        }
        writer.start_file(name, options)?;
        writer.write_all(&data)?;
    }
    Ok(writer.finish()?.into_inner())
}

/// Excel and PowerPoint can't place arbitrary native Word math inline. Preserve
/// source order using separate visual sheets/slides, alongside editable text.
pub fn office_ir(
    rich: &RichMarkdown,
    format: office_oxide::DocumentFormat,
) -> office_oxide::ir::DocumentIR {
    use office_oxide::ir::{DocumentIR, Element, Image, ImageFormat, Section};
    let pattern = regex::Regex::new(r"!\[(?:公式|Mermaid 图)\]\((docsy-media-\d+)\)").unwrap();
    let mut result = DocumentIR::from_markdown("", format);
    let mut end = 0;
    for caps in pattern.captures_iter(&rich.text) {
        let found = caps.get(0).unwrap();
        if let Some(asset) = rich.assets.get(&caps[1]) {
            let text = &rich.text[end..found.start()];
            if !text.trim().is_empty() {
                result
                    .sections
                    .extend(DocumentIR::from_markdown(text, format).sections);
            }
            let scale = (850.0 / asset.width).min(600.0 / asset.height).min(1.0);
            result.sections.push(Section {
                title: None,
                elements: vec![Element::Image(Image {
                    data: Some(asset.png.clone()),
                    format: Some(ImageFormat::Png),
                    alt_text: Some(
                        if asset.kind == "math" {
                            "公式"
                        } else {
                            "Mermaid 图"
                        }
                        .into(),
                    ),
                    display_width_emu: Some((asset.width * scale * 9525.0) as u64),
                    display_height_emu: Some((asset.height * scale * 9525.0) as u64),
                    ..Default::default()
                })],
                ..Default::default()
            });
            end = found.end();
        }
    }
    if !rich.text[end..].trim().is_empty() {
        result
            .sections
            .extend(DocumentIR::from_markdown(&rich.text[end..], format).sections);
    }
    result
}

pub fn html_fragment(rich: &RichMarkdown) -> String {
    let mut fragment = super::safe_html_fragment(&rich.text);
    for (id, asset) in &rich.assets {
        let style = if asset.display {
            "display:block;margin:1em auto;max-width:100%;height:auto"
        } else {
            "vertical-align:middle;max-width:100%;height:auto"
        };
        fragment = fragment.replace(
            &format!("src=\"{id}\""),
            &format!(
                "src=\"data:image/png;base64,{}\" width=\"{}\" style=\"{style}\"",
                STANDARD.encode(&asset.png),
                asset.width.ceil()
            ),
        );
    }
    fragment
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    fn tiny_png() -> String {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(30, 15)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        format!("data:image/png;base64,{}", STANDARD.encode(bytes.get_ref()))
    }
    fn rendered(text: &str, mathml: &str) -> RenderedMarkdown {
        let prep = prepare(text);
        RenderedMarkdown {
            source_hash: prep.source_hash,
            items: prep
                .items
                .into_iter()
                .map(|r| RenderedMedia {
                    id: r.id,
                    png: tiny_png(),
                    width: 10.0,
                    height: 5.0,
                    mathml: (r.kind == "math").then(|| mathml.into()),
                })
                .collect(),
        }
    }
    fn zip_part(bytes: &[u8], name: &str) -> String {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        zip.by_name(name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        text
    }
    #[test]
    fn parser_preserves_code_currency_and_escaped_delimiters() {
        let text = "正文 $x^2$ 与 \\(a+b\\)。\n\n$$\\frac{1}{2}$$\n\n\\[x+y\\]\n\n```mermaid\ngraph LR\n A-->B\n```\n\n`$code$`\n\n```txt\n$not_math$\n\\(not_math\\)\n```\n\n价格 \\$5 和 \\$10\n\\\\(literal\\\\)";
        let plan = prepare(text);
        let currency = prepare("价格 $5 和 $10，预算 US$20");
        assert!(currency.items.is_empty(), "{currency:?}");
        assert_eq!(plan.items.len(), 5, "{plan:?}");
        assert_eq!(plan.items[0].source, "x^2");
        assert_eq!(plan.items[1].source, "a+b");
        assert_eq!(plan.items[4].kind, "mermaid");
        let data = rendered(text, "<math><mi>x</mi></math>");
        let rich = resolve(text, Some(&data)).unwrap();
        assert!(rich.text.contains("`$code$`"));
        assert!(rich.text.contains("$not_math$"));
        assert!(rich.text.contains("![Mermaid 图](docsy-media-4)"));
    }
    #[test]
    fn multilingual_offsets_text_and_currency_survive_export() {
        let body = "Français café cœur e\u{301} — 日本語 かな カナ — 한국어 한글 😀";
        let text = format!("{body} $x^2$ fin\n\n\\[y+1\\]\n\n```mermaid\ngraph LR\nA[日本語]-->B[한국어]\n```\n\nPrix $5 et $10 ; 価格 US$20 ; 가격 ₩2000");
        let plan = prepare(&text);
        assert_eq!(plan.items.len(), 3, "{plan:?}");
        assert_eq!(plan.items[0].source, "x^2");
        assert_eq!(&text[plan.items[1].range.clone()], "\\[y+1\\]");
        let data = rendered(
            &text,
            "<math><mtext>café cœur 日本語 한국어</mtext><mo>=</mo><mn>2</mn></math>",
        );
        let rich = resolve(&text, Some(&data)).unwrap();
        assert!(rich.text.contains(body));
        assert!(rich
            .text
            .contains("Prix $5 et $10 ; 価格 US$20 ; 가격 ₩2000"));
        let bytes = super::super::md_to_docx::build_docx_bytes_with_media(
            &rich.text,
            std::path::Path::new("."),
            super::super::md_to_docx::DocxStylePreset::Professional,
            Some(&rich.assets),
        )
        .unwrap();
        let xml = zip_part(&bytes, "word/document.xml");
        assert!(xml.contains(body));
        assert!(xml.contains("<m:t xml:space=\"preserve\">café cœur 日本語 한국어</m:t>"));
    }

    #[test]
    fn real_browser_multilingual_export() {
        let text = include_str!("fixtures/multilingual.md");
        let data: RenderedMarkdown =
            serde_json::from_str(include_str!("fixtures/multilingual.json")).unwrap();
        let rich = resolve(&text, Some(&data)).unwrap();
        assert_eq!(rich.assets.len(), 2);
        assert_eq!(rich.fallback_count, 0);
        let temp = crate::util::fs::temp_named_path("docsy-media-fixture", "out");
        let dir = temp.as_path();
        std::fs::create_dir_all(dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(temp.clone()).unwrap();
        for format in ["docx", "html", "xlsx", "pptx"] {
            let result = super::super::convert_text_with_media(
                &text,
                format,
                dir.to_str(),
                Some("multilingual"),
                None,
                Some(&data),
            )
            .unwrap();
            let bytes = std::fs::read(&result.output_path).unwrap();
            let content = if format == "html" {
                String::from_utf8(bytes).unwrap()
            } else {
                let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
                let mut content = String::new();
                for i in 0..zip.len() {
                    let mut entry = zip.by_index(i).unwrap();
                    if entry.name().ends_with(".xml") {
                        entry.read_to_string(&mut content).unwrap();
                    }
                }
                content
            };
            for language in ["café", "cœur", "日本語", "한국어", "e\u{301}", "😀"] {
                assert!(content.contains(language), "{format}: {language}");
            }
            if format == "docx" {
                assert_eq!(content.matches("<m:oMath>").count(), 1);
                assert_eq!(content.matches("<w:drawing>").count(), 1);
                assert!(content.contains("café cœur 日本語 한국어"));
            }
            println!("{}", result.output_path);
        }
    }

    #[test]
    fn conversion_enforces_limits_without_prepare_command() {
        assert!(resolve(&"x".repeat(16 * 1024 * 1024 + 1), None).is_err());
        let many = "$x$ ".repeat(501);
        assert!(resolve(&many, None)
            .err()
            .unwrap()
            .to_string()
            .contains("过多"));
        assert!(resolve(&format!("$${}$$", "x".repeat(50_001)), None)
            .err()
            .unwrap()
            .to_string()
            .contains("过长"));
        let mut data = rendered("plain", "");
        data.items = (0..501)
            .map(|i| RenderedMedia {
                id: i.to_string(),
                png: String::new(),
                width: 1.0,
                height: 1.0,
                mathml: None,
            })
            .collect();
        assert!(resolve("plain", Some(&data)).is_err());
    }

    #[test]
    fn detects_changed_sources_missing_assets_and_invalid_images() {
        let mut data = rendered("$x$", "<math><mi>x</mi></math>");
        assert!(resolve("$y$", Some(&data)).is_err());
        assert!(resolve("$x$", None).is_err());
        data.items[0].png = "data:image/png;base64,bm90IGEgcG5n".into();
        assert!(resolve("$x$", Some(&data)).is_err());
        data.items.clear();
        assert!(resolve("$x$", Some(&data)).is_err());
    }
    #[test]
    fn equation_placeholders_never_replace_literal_body_text() {
        let text = "DOCSYOMMLdocsymedia0END $x$";
        let data = rendered(text, "<math><mi>x</mi></math>");
        let rich = resolve(text, Some(&data)).unwrap();
        let bytes = super::super::md_to_docx::build_docx_bytes_with_media(
            &rich.text,
            std::path::Path::new("."),
            super::super::md_to_docx::DocxStylePreset::Professional,
            Some(&rich.assets),
        )
        .unwrap();
        let xml = zip_part(&bytes, "word/document.xml");
        assert!(xml.contains("DOCSYOMMLdocsymedia0END"));
        assert_eq!(xml.matches("<m:oMath>").count(), 1);
    }

    #[test]
    fn docx_has_editable_math_and_embedded_fallback_and_diagram() {
        let text = "before $x$ after\n\n$$y$$\n\n```mermaid\ngraph LR\nA-->B\n```";
        let mut data = rendered(text, "<math><mfrac><mi>x</mi><mn>2</mn></mfrac></math>");
        data.items[1].mathml = Some("<math><menclose><mi>y</mi></menclose></math>".into());
        let rich = resolve(text, Some(&data)).unwrap();
        assert_eq!(rich.fallback_count, 1);
        let bytes = super::super::md_to_docx::build_docx_bytes_with_media(
            &rich.text,
            std::path::Path::new("."),
            super::super::md_to_docx::DocxStylePreset::Professional,
            Some(&rich.assets),
        )
        .unwrap();
        let xml = zip_part(&bytes, "word/document.xml");
        assert!(xml.contains("<m:f>"));
        assert!(
            xml.contains("xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"")
        );
        assert!(!xml.contains("DOCSYOMML"));
        assert_eq!(xml.matches("<w:drawing>").count(), 2);
        assert!(xml.find("before").unwrap() < xml.find("<m:oMath>").unwrap());
        assert!(xml.find("<m:oMath>").unwrap() < xml.find("after").unwrap());
        let html = html_fragment(&rich);
        assert_eq!(html.matches("data:image/png;base64,").count(), 3);
        assert!(!html.contains("docsy-media-"));
    }
    #[test]
    fn real_browser_media_export() {
        let text = "# 公式与图表验证\n\n能量 $E=mc^2$。\n\n$$\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}$$\n\n$$\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}$$\n\n$$\\sum_{i=1}^{n} i=\\frac{n(n+1)}{2}$$\n\n```mermaid\nflowchart LR\n A[导入 Markdown] --> B{包含公式?}\n B -->|是| C[可编辑 Word 公式]\n B -->|否| D[正常转换]\n```";
        let mut data: RenderedMarkdown =
            serde_json::from_str(include_str!("fixtures/rendered.json")).unwrap();
        data.source_hash = prepare(text).source_hash;
        let temp = crate::util::fs::temp_named_path("docsy-media-fixture", "out");
        let dir = temp.as_path();
        std::fs::create_dir_all(dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(temp.clone()).unwrap();
        let rich = resolve(text, Some(&data)).unwrap();
        assert_eq!(rich.fallback_count, 0, "common formulas must be editable");
        for format in ["docx", "html", "xlsx", "pptx"] {
            let result = super::super::convert_text_with_media(
                text,
                format,
                dir.to_str(),
                Some("rich-markdown"),
                None,
                Some(&data),
            )
            .unwrap();
            let bytes = std::fs::read(&result.output_path).unwrap();
            if format == "docx" {
                let xml = zip_part(&bytes, "word/document.xml");
                assert_eq!(xml.matches("<m:oMath>").count(), 4);
                assert_eq!(xml.matches("<w:drawing>").count(), 1);
            } else if format == "html" {
                assert_eq!(
                    String::from_utf8(bytes)
                        .unwrap()
                        .matches("data:image/png;base64,")
                        .count(),
                    5
                );
            } else {
                let zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
                assert_eq!(
                    zip.file_names()
                        .filter(|name| name.contains("/media/") && name.ends_with(".png"))
                        .count(),
                    5,
                    "{format} embeds every image"
                );
            }
            println!("{}", result.output_path);
        }
    }
}
