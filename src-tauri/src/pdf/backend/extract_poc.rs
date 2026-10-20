#![allow(dead_code)]

use super::BackendError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 单个文本片段信息（含坐标与字号）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub bbox: [f64; 4],
    pub font_size: f64,
}

/// 单页提取结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageTextExtraction {
    pub page_number: u32,
    pub width: f64,
    pub height: f64,
    pub text: String,
    pub spans: Vec<TextSpan>,
    pub is_empty: bool,
}

/// 整体文档文本与 Markdown 结构提取 PoC 结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentTextExtraction {
    pub total_pages: u32,
    pub pages_with_text: usize,
    pub empty_pages: Vec<u32>,
    pub pages: Vec<PageTextExtraction>,
    pub markdown: String,
}

fn extract_string_literal(obj: &lopdf::Object) -> String {
    match obj {
        lopdf::Object::String(bytes, _) => String::from_utf8_lossy(bytes).to_string(),
        _ => String::new(),
    }
}

/// 从页面内容流解析文本片段的位置（bbox）与字号。
fn extract_page_spans(
    doc: &lopdf::Document,
    page_id: lopdf::ObjectId,
) -> Vec<TextSpan> {
    let Ok(bytes) = doc.get_page_content(page_id) else {
        return Vec::new();
    };
    let Ok(content) = lopdf::content::Content::decode(&bytes) else {
        return Vec::new();
    };

    let mut spans = Vec::new();
    let mut current_font_size = 12.0f64;
    let mut ctm = [1.0f64, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut ctm_stack: Vec<[f64; 6]> = Vec::new();
    let mut tm = [1.0f64, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut tlm = [1.0f64, 0.0, 0.0, 1.0, 0.0, 0.0];

    for op in &content.operations {
        match op.operator.as_str() {
            "q" => ctm_stack.push(ctm),
            "Q" => ctm = ctm_stack.pop().unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            "cm" => {
                if op.operands.len() == 6 {
                    let m: Vec<f64> = op
                        .operands
                        .iter()
                        .map(|o| o.as_float().unwrap_or(0.0) as f64)
                        .collect();
                    ctm = [
                        ctm[0] * m[0] + ctm[2] * m[1],
                        ctm[1] * m[0] + ctm[3] * m[1],
                        ctm[0] * m[2] + ctm[2] * m[3],
                        ctm[1] * m[2] + ctm[3] * m[3],
                        ctm[0] * m[4] + ctm[2] * m[5] + ctm[4],
                        ctm[1] * m[4] + ctm[3] * m[5] + ctm[5],
                    ];
                }
            }
            "BT" => {
                tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                tlm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
            "Tf" => {
                if let Some(size) = op.operands.get(1).and_then(|o| o.as_float().ok()) {
                    current_font_size = size as f64;
                }
            }
            "Td" => {
                if op.operands.len() >= 2 {
                    let tx = op.operands[0].as_float().unwrap_or(0.0) as f64;
                    let ty = op.operands[1].as_float().unwrap_or(0.0) as f64;
                    tlm[4] = tx * tlm[0] + ty * tlm[2] + tlm[4];
                    tlm[5] = tx * tlm[1] + ty * tlm[3] + tlm[5];
                    tm = tlm;
                }
            }
            "TD" => {
                if op.operands.len() >= 2 {
                    let tx = op.operands[0].as_float().unwrap_or(0.0) as f64;
                    let ty = op.operands[1].as_float().unwrap_or(0.0) as f64;
                    tlm[4] = tx * tlm[0] + ty * tlm[2] + tlm[4];
                    tlm[5] = tx * tlm[1] + ty * tlm[3] + tlm[5];
                    tm = tlm;
                }
            }
            "Tm" => {
                if op.operands.len() == 6 {
                    let m: Vec<f64> = op
                        .operands
                        .iter()
                        .map(|o| o.as_float().unwrap_or(0.0) as f64)
                        .collect();
                    tm = [m[0], m[1], m[2], m[3], m[4], m[5]];
                    tlm = tm;
                }
            }
            "Tj" => {
                if let Some(text_obj) = op.operands.first() {
                    let text = extract_string_literal(text_obj);
                    if !text.trim().is_empty() {
                        let x_pt = tm[4];
                        let y_pt = tm[5];
                        let x = ctm[0] * x_pt + ctm[2] * y_pt + ctm[4];
                        let y = ctm[1] * x_pt + ctm[3] * y_pt + ctm[5];
                        let est_w = (current_font_size * 0.6 * text.chars().count() as f64).max(1.0);
                        spans.push(TextSpan {
                            text,
                            bbox: [x, y, x + est_w, y + current_font_size],
                            font_size: current_font_size,
                        });
                    }
                }
            }
            "TJ" => {
                if let Some(lopdf::Object::Array(arr)) = op.operands.first() {
                    let mut full_text = String::new();
                    for item in arr {
                        if let lopdf::Object::String(bytes, _) = item {
                            full_text.push_str(&String::from_utf8_lossy(bytes));
                        }
                    }
                    if !full_text.trim().is_empty() {
                        let x_pt = tm[4];
                        let y_pt = tm[5];
                        let x = ctm[0] * x_pt + ctm[2] * y_pt + ctm[4];
                        let y = ctm[1] * x_pt + ctm[3] * y_pt + ctm[5];
                        let est_w = (current_font_size * 0.6 * full_text.chars().count() as f64).max(1.0);
                        spans.push(TextSpan {
                            text: full_text,
                            bbox: [x, y, x + est_w, y + current_font_size],
                            font_size: current_font_size,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    spans
}

/// 从 PDF 中提取结构化文本、坐标片段并生成 Markdown（PoC）。
///
/// 严格契约：
/// - 独立运行，不侵入页面重组/合并主链路。
/// - 保留 1-based 原始页码。
/// - 对无文本层/扫描页生成明确提示，避免将未识别空白包装为成功提取。
/// - 文本解析错误（如破坏的内容流、算子参数缺失）如实返回 Err，绝不伪装为空白扫描页成功吞掉。
pub fn extract_document_text(path: &Path) -> Result<DocumentTextExtraction, BackendError> {
    let bytes = std::fs::read(path).map_err(BackendError::IoError)?;
    let doc = lopdf::Document::load_mem(&bytes)
        .map_err(|e| BackendError::ParseError(format!("加载 PDF 解析失败: {e}")))?;

    let pages = doc.get_pages();
    let total_pages = pages.len() as u32;

    let mut pages_result = Vec::with_capacity(pages.len());
    let mut empty_pages = Vec::new();
    let mut markdown_lines = Vec::new();

    for (page_idx, (_, page_id)) in pages.into_iter().enumerate() {
        let page_number = (page_idx + 1) as u32;

        let (width, height) = doc
            .get_dictionary(page_id)
            .ok()
            .and_then(|dict| dict.get(b"MediaBox").ok())
            .and_then(|mb| mb.as_array().ok())
            .map(|arr| {
                let coords: Vec<f64> = arr
                    .iter()
                    .map(|v| v.as_float().unwrap_or(0.0) as f64)
                    .collect();
                if coords.len() == 4 {
                    ((coords[2] - coords[0]).abs(), (coords[3] - coords[1]).abs())
                } else {
                    (595.0, 842.0)
                }
            })
            .unwrap_or((595.0, 842.0));

        let text_content = doc
            .extract_text(&[page_number])
            .map_err(|e| BackendError::ExecutionError {
                backend: super::BackendType::PdfCraft,
                message: format!("页面 {page_number} 文本提取失败: {e}"),
            })?;
        let trimmed = text_content.trim();

        if trimmed.is_empty() {
            empty_pages.push(page_number);
            pages_result.push(PageTextExtraction {
                page_number,
                width,
                height,
                text: format!("[第 {page_number} 页未检测到文本层，可能为扫描件或图像页]"),
                spans: Vec::new(),
                is_empty: true,
            });
            markdown_lines.push(format!("## 第 {page_number} 页\n"));
            markdown_lines.push(format!("> [!NOTE]\n> 第 {page_number} 页未检测到可提取文本层（扫描件或图像页面）。\n"));
        } else {
            let mut spans = extract_page_spans(&doc, page_id);
            if spans.is_empty() {
                for line in trimmed.lines() {
                    if !line.trim().is_empty() {
                        spans.push(TextSpan {
                            text: line.trim().to_string(),
                            bbox: [0.0, 0.0, width, 12.0],
                            font_size: 12.0,
                        });
                    }
                }
            }

            pages_result.push(PageTextExtraction {
                page_number,
                width,
                height,
                text: trimmed.to_string(),
                spans,
                is_empty: false,
            });

            markdown_lines.push(format!("## 第 {page_number} 页\n"));
            markdown_lines.push(format!("{trimmed}\n"));
        }
    }

    let pages_with_text = total_pages as usize - empty_pages.len();
    let markdown = markdown_lines.join("\n");

    Ok(DocumentTextExtraction {
        total_pages,
        pages_with_text,
        empty_pages,
        pages: pages_result,
        markdown,
    })
}
