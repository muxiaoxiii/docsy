//! 模板制作器后端
//!
//! 功能：
//! 1. 读 docx，提取纯文本（保留每个 <w:t> 节点对应的字符偏移区间）
//! 2. 按"字符偏移区间 + 替换文本"列表，把 docx 里的对应 <w:t> 文本替换为 {{key}} 占位符
//! 3. 打包为 .docsytpl（zip）：原始 docx + manifest.json + fields.json
//! 4. 扫描用户数据目录的模板列表

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::read::ZipArchive;
use zip::write::{FileOptions, ZipWriter};

use crate::docx::utils::flatten_nested_paragraphs;
use crate::templates;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocxText {
    /// 整篇文档纯文本（按段落用 \n 连接）
    pub plain_text: String,
    /// 段落数
    pub paragraph_count: usize,
}

/// 读 docx 提取纯文本
///
/// 先合并相邻 run 和 text 节点，确保纯文本位置与 mark 位置计算一致。
pub fn extract_plain_text(docx_bytes: &[u8]) -> Result<DocxText, String> {
    let cursor = Cursor::new(docx_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let mut entry = archive
        .by_name("word/document.xml")
        .map_err(|e| e.to_string())?;
    let mut xml = String::new();
    entry.read_to_string(&mut xml).map_err(|e| e.to_string())?;

    // 先合并 run，与 read_template_for_edit 保持一致
    let xml = flatten_nested_paragraphs(&xml);
    let xml = merge_adjacent_runs(&xml);
    let xml = merge_adjacent_text_nodes(&xml);

    let para_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
    let t_re = Regex::new(r"(?s)<w:t[^>]*>([^<]*)</w:t>").unwrap();

    let mut text = String::new();
    let mut count = 0;
    for cap in para_re.captures_iter(&xml) {
        let body = cap.get(1).unwrap().as_str();
        for tcap in t_re.captures_iter(body) {
            text.push_str(tcap.get(1).unwrap().as_str());
        }
        text.push('\n');
        count += 1;
    }

    Ok(DocxText {
        plain_text: text,
        paragraph_count: count,
    })
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FieldMark {
    /// 在纯文本里的起始字符偏移（按 char 计数，不是 byte）
    pub start: usize,
    /// 在纯文本里的结束字符偏移
    pub end: usize,
    /// 字段 key（替换后写入模板的占位符）
    pub key: String,
}

pub fn replace_text_range(
    docx_bytes: &[u8],
    start: usize,
    end: usize,
    replacement: &str,
) -> Result<Vec<u8>, String> {
    if start >= end {
        return Err("编辑范围无效".to_string());
    }

    crate::app_log::debug(
        "template.docx",
        "replace_text_range.start",
        serde_json::json!({
            "start": start,
            "end": end,
            "replacementChars": replacement.chars().count(),
            "inputBytes": docx_bytes.len()
        }),
    );

    let cursor = Cursor::new(docx_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    let mut doc_xml = String::new();
    {
        let mut entry = archive
            .by_name("word/document.xml")
            .map_err(|e| e.to_string())?;
        entry
            .read_to_string(&mut doc_xml)
            .map_err(|e| e.to_string())?;
    }

    let doc_xml = flatten_nested_paragraphs(&doc_xml);
    let doc_xml = merge_adjacent_runs(&doc_xml);
    let doc_xml = merge_adjacent_text_nodes(&doc_xml);

    // 记录源文本窗口，避免替换失败时只能猜偏移是否正确。
    let para_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
    let t_re = Regex::new(r"(?s)<w:t[^>]*>([^<]*)</w:t>").unwrap();
    let mut debug_text = String::new();
    for pm in para_re.captures_iter(&doc_xml) {
        let body = pm.get(1).unwrap().as_str();
        for tc in t_re.captures_iter(body) {
            debug_text.push_str(tc.get(1).unwrap().as_str());
        }
        debug_text.push('\n');
    }
    let debug_chars: Vec<char> = debug_text.chars().collect();
    let sample: String = debug_chars.iter().take(120).collect();
    let selected: String = debug_chars
        .iter()
        .skip(start)
        .take(end.saturating_sub(start).min(120))
        .collect();
    crate::app_log::debug(
        "template.docx",
        "replace_text_range.source_window",
        serde_json::json!({
            "plainChars": debug_chars.len(),
            "sample": sample,
            "selected": selected
        }),
    );

    let new_xml = replace_text_range_xml(&doc_xml, start, end, replacement)?;

    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| e.to_string())?;

            let payload = if name == "word/document.xml" {
                new_xml.as_bytes().to_vec()
            } else {
                data
            };

            writer.start_file(&name, opts).map_err(|e| e.to_string())?;
            writer.write_all(&payload).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
    }

    Ok(out_buf)
}

/// 把 docx 中按偏移区间标注的文本替换为 {{key}} 占位符。
///
/// 算法：扫描 document.xml，对每个 <w:t> 节点，记录它在纯文本里占据的偏移区间。
/// 找到完全或部分被某 mark 区间覆盖的 <w:t>，按 mark 拆分文本，并以 {{key}} 替换。
///
/// 简化：要求 mark 落在单个 <w:t> 内（即不跨 run）。前端在划选时强制按这一约束选择。
pub fn rewrite_with_placeholders(
    docx_bytes: &[u8],
    marks: &[FieldMark],
) -> Result<Vec<u8>, String> {
    let cursor = Cursor::new(docx_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    // 读出 document.xml
    let mut doc_xml = String::new();
    {
        let mut entry = archive
            .by_name("word/document.xml")
            .map_err(|e| e.to_string())?;
        entry
            .read_to_string(&mut doc_xml)
            .map_err(|e| e.to_string())?;
    }

    // 先修复 WPS 产生的嵌套 <w:p> 结构（非法 OOXML，Word 无法解析）
    let doc_xml = flatten_nested_paragraphs(&doc_xml);
    // 合并相邻且 rPr 相同的 <w:r>，再合并同一 <w:r> 内的 <w:t>
    let doc_xml = merge_adjacent_runs(&doc_xml);
    let doc_xml = merge_adjacent_text_nodes(&doc_xml);
    let new_xml = rewrite_xml(&doc_xml, marks)?;

    // 重新打包 zip
    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| e.to_string())?;

            let payload = if name == "word/document.xml" {
                new_xml.as_bytes().to_vec()
            } else {
                data
            };

            writer.start_file(&name, opts).map_err(|e| e.to_string())?;
            writer.write_all(&payload).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
    }

    Ok(out_buf)
}

/// 合并相邻且 `<w:rPr>` 相同的 `<w:r>` 为一个 `<w:r>`。
///
/// WPS/Word 把 "田  力" 拆成 3 个 `<w:r>`（"田"、"  "、"力"），每个 rPr 相同。
/// 合并后变成一个 `<w:r>`，内部的 `<w:t>` 也能被 `merge_adjacent_text_nodes` 进一步合并。
fn merge_adjacent_runs(xml: &str) -> String {
    let run_re = Regex::new(r"(?s)<w:r\b([^>]*)>(.*?)</w:r>").unwrap();
    let rpr_re = Regex::new(r"(?s)<w:rPr>(.*?)</w:rPr>").unwrap();

    // 收集所有 run 的位置和内容
    let runs: Vec<_> = run_re.find_iter(xml).collect();
    if runs.len() <= 1 {
        return xml.to_string();
    }

    // 提取每个 run 的 rPr（用于比较），忽略纯元数据属性
    let normalize_rpr = |run_str: &str| -> String {
        let raw = rpr_re
            .captures(run_str)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        // 去掉不影响格式的元数据属性
        let re_hint = Regex::new(r#"\s*w:hint="[^"]*""#).unwrap();
        let re_rsid = Regex::new(r#"\s*w:rsid\w*="[^"]*""#).unwrap();
        let re_no_proof = Regex::new(r#"\s*<w:noProof\s*/>"#).unwrap();
        let r = re_hint.replace_all(&raw, "");
        let r = re_rsid.replace_all(&r, "");
        let r = re_no_proof.replace_all(&r, "");
        r.trim().to_string()
    };

    // 标记需要合并的 run 组
    let mut merge_groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group: Vec<usize> = vec![0];

    for i in 1..runs.len() {
        let prev_rpr = normalize_rpr(runs[i - 1].as_str());
        let curr_rpr = normalize_rpr(runs[i].as_str());
        if prev_rpr == curr_rpr {
            current_group.push(i);
        } else {
            if current_group.len() > 1 {
                merge_groups.push(current_group);
            }
            current_group = vec![i];
        }
    }
    if current_group.len() > 1 {
        merge_groups.push(current_group);
    }

    if merge_groups.is_empty() {
        return xml.to_string();
    }

    // 执行合并
    let mut result = String::with_capacity(xml.len());
    let mut last_end = 0;
    let t_re = Regex::new(r"(?s)<w:t([^>]*)>([^<]*)</w:t>").unwrap();

    for group in &merge_groups {
        let first_run = runs[group[0]];
        let last_run = runs[*group.last().unwrap()];

        // 输出 group 之前的原样内容
        result.push_str(&xml[last_end..first_run.start()]);

        // 合并所有 run 的 <w:t> 内容
        let first_rpr = rpr_re
            .captures(first_run.as_str())
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mut merged_text = String::new();
        for &idx in group {
            let run_str = runs[idx].as_str();
            for tc in t_re.captures_iter(run_str) {
                merged_text.push_str(tc.get(2).unwrap().as_str());
            }
        }

        // 输出合并后的 run：保留第一个 run 的 rPr，合并文本
        let escaped = merged_text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let preserve = if merged_text.starts_with(' ') || merged_text.ends_with(' ') {
            r#" xml:space="preserve""#
        } else {
            ""
        };

        result.push_str(&format!(
            "<w:r><w:rPr>{}</w:rPr><w:t{}>{}</w:t></w:r>",
            first_rpr, preserve, escaped
        ));

        last_end = last_run.end();
    }

    result.push_str(&xml[last_end..]);
    result
}

/// 合并同一 `<w:r>` 内相邻的 `<w:t>` 节点为一个。
fn merge_adjacent_text_nodes(xml: &str) -> String {
    let run_re = Regex::new(r"(?s)(<w:r\b[^>]*>)(.*?)(</w:r>)").unwrap();
    let t_re = Regex::new(r"(?s)<w:t([^>]*)>([^<]*)</w:t>").unwrap();

    run_re
        .replace_all(xml, |caps: &regex::Captures| {
            let open = &caps[1];
            let inner = &caps[2];
            let close = &caps[3];

            let t_matches: Vec<_> = t_re.find_iter(inner).collect();
            if t_matches.len() <= 1 {
                return caps[0].to_string();
            }

            let first_cap = t_re.captures(inner).unwrap();
            let first_attrs = first_cap.get(1).unwrap().as_str();

            let mut merged = String::new();
            for tm in &t_matches {
                if let Some(tc) = t_re.captures(tm.as_str()) {
                    merged.push_str(tc.get(2).unwrap().as_str());
                }
            }

            let first_t = t_matches.first().unwrap();
            let prefix = &inner[..first_t.start()];
            let last_t = t_matches.last().unwrap();
            let suffix = &inner[last_t.end()..];

            let escaped = merged
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            let attrs_final = if (merged.starts_with(' ') || merged.ends_with(' '))
                && !first_attrs.contains("xml:space")
            {
                format!("{}{}", first_attrs, r#" xml:space="preserve""#)
            } else {
                first_attrs.to_string()
            };

            format!(
                "{}{}<w:t{}>{}</w:t>{}{}",
                open, prefix, attrs_final, escaped, suffix, close
            )
        })
        .into_owned()
}

fn rewrite_xml(xml: &str, marks: &[FieldMark]) -> Result<String, String> {
    let para_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
    let t_re = Regex::new(r"(?s)(<w:t[^>]*>)([^<]*)(</w:t>)").unwrap();

    // 按起始偏移升序，便于扫描
    let mut sorted = marks.iter().enumerate().collect::<Vec<_>>();
    sorted.sort_by_key(|(_, m)| m.start);

    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    let mut char_offset: usize = 0;
    let para_iter = para_re.find_iter(xml);
    let mut matched = vec![false; marks.len()];

    for p in para_iter {
        out.push_str(&xml[last..p.start()]);
        last = p.end();

        let para_xml = p.as_str();

        // 收集段落内所有 <w:t> 节点信息
        struct TextNode {
            full_match_start: usize, // 在 para_xml 中的起始位置
            full_match_end: usize,   // 在 para_xml 中的结束位置
            open: String,
            body: String,
            close: String,
            t_start: usize, // 在纯文本中的起始偏移
            t_end: usize,   // 在纯文本中的结束偏移
        }

        let mut text_nodes: Vec<TextNode> = Vec::new();
        let mut temp_offset = char_offset;

        for cap in t_re.captures_iter(para_xml) {
            let m = cap.get(0).unwrap();
            let open = cap.get(1).unwrap().as_str().to_string();
            let body = cap.get(2).unwrap().as_str().to_string();
            let close = cap.get(3).unwrap().as_str().to_string();
            let body_chars = body.chars().count();

            text_nodes.push(TextNode {
                full_match_start: m.start(),
                full_match_end: m.end(),
                open,
                body,
                close,
                t_start: temp_offset,
                t_end: temp_offset + body_chars,
            });
            temp_offset += body_chars;
        }
        char_offset = temp_offset + 1;

        // 处理每个 mark，支持跨 <w:t> 节点
        // 为每个 <w:t> 节点构建替换计划
        #[derive(Clone)]
        struct NodeAction {
            prefix: String,      // mark 之前的文本
            placeholder: String, // {{key}} 占位符
            suffix: String,      // mark 之后的文本
        }

        let mut node_actions: Vec<Option<NodeAction>> = vec![None; text_nodes.len()];

        for (idx, m) in &sorted {
            if matched[*idx] {
                continue;
            }
            // 检查 mark 是否与此段落的文本节点有交集
            let mark_start = m.start;
            let mark_end = m.end;

            // 找到 mark 起始所在的节点和 mark 结束所在的节点
            let start_node = text_nodes
                .iter()
                .position(|n| mark_start >= n.t_start && mark_start < n.t_end);
            let end_node = text_nodes
                .iter()
                .position(|n| mark_end > n.t_start && mark_end <= n.t_end);

            if let (Some(si), Some(ei)) = (start_node, end_node) {
                if si == ei {
                    // mark 完全在一个节点内
                    let node = &text_nodes[si];
                    let pre_chars = mark_start - node.t_start;
                    let post_chars = node.t_end - mark_end;
                    let body_chars_vec: Vec<char> = node.body.chars().collect();

                    let prefix: String = body_chars_vec[..pre_chars].iter().collect();
                    let suffix: String = if post_chars > 0 {
                        body_chars_vec[body_chars_vec.len() - post_chars..]
                            .iter()
                            .collect()
                    } else {
                        String::new()
                    };

                    // 如果已有 action，需要合并
                    if let Some(ref mut action) = node_actions[si] {
                        // 追加到已有 action 的 suffix 后面
                        action.suffix = format!("{}{}{}", action.suffix, prefix, suffix);
                    } else {
                        node_actions[si] = Some(NodeAction {
                            prefix,
                            placeholder: format!("{{{{{}}}}}", m.key),
                            suffix,
                        });
                    }
                } else {
                    // mark 跨越多个节点
                    let start_node = &text_nodes[si];
                    let end_node = &text_nodes[ei];

                    // 第一个节点：保留 mark 之前的文本
                    let pre_chars = mark_start - start_node.t_start;
                    let body_chars_vec: Vec<char> = start_node.body.chars().collect();
                    let prefix: String = body_chars_vec[..pre_chars].iter().collect();

                    // 最后一个节点：保留 mark 之后的文本
                    let post_chars = end_node.t_end - mark_end;
                    let body_chars_vec: Vec<char> = end_node.body.chars().collect();
                    let suffix: String = if post_chars > 0 {
                        body_chars_vec[body_chars_vec.len() - post_chars..]
                            .iter()
                            .collect()
                    } else {
                        String::new()
                    };

                    // 中间节点：完全删除（输出空）
                    for mid in si + 1..ei {
                        node_actions[mid] = Some(NodeAction {
                            prefix: String::new(),
                            placeholder: String::new(),
                            suffix: String::new(),
                        });
                    }

                    // 第一个节点：prefix + placeholder
                    node_actions[si] = Some(NodeAction {
                        prefix,
                        placeholder: format!("{{{{{}}}}}", m.key),
                        suffix: String::new(),
                    });

                    // 最后一个节点：suffix
                    node_actions[ei] = Some(NodeAction {
                        prefix: String::new(),
                        placeholder: String::new(),
                        suffix,
                    });
                }
                matched[*idx] = true;
            }
        }

        // 重建段落内容
        let mut p_last = 0;
        let mut new_para = String::with_capacity(para_xml.len());

        for (i, node) in text_nodes.iter().enumerate() {
            // 输出节点之前的内容
            new_para.push_str(&para_xml[p_last..node.full_match_start]);
            p_last = node.full_match_end;

            if let Some(ref action) = node_actions[i] {
                // 有替换操作
                if !action.prefix.is_empty() {
                    new_para.push_str(&render_t(&node.open, &action.prefix, &node.close));
                }
                if !action.placeholder.is_empty() {
                    new_para.push_str(&render_t(&node.open, &action.placeholder, &node.close));
                }
                if !action.suffix.is_empty() {
                    new_para.push_str(&render_t(&node.open, &action.suffix, &node.close));
                }
            } else {
                // 无替换，原样输出
                new_para.push_str(&node.open);
                new_para.push_str(&node.body);
                new_para.push_str(&node.close);
            }
        }
        new_para.push_str(&para_xml[p_last..]);

        // 重建段落标签
        out.push_str(&xml[p.start()..p.start()]); // no-op
        out.push_str(&prepend_para_tag(para_xml, &new_para));
    }
    out.push_str(&xml[last..]);

    let missed: Vec<String> = marks
        .iter()
        .zip(matched.iter())
        .filter_map(|(m, ok)| {
            if *ok {
                None
            } else {
                Some(format!("{}({}-{})", m.key, m.start, m.end))
            }
        })
        .collect();
    if !missed.is_empty() {
        return Err(format!(
            "有 {} 个字段未能写入模板，请重新选择这些字段：{}",
            missed.len(),
            missed.join("、")
        ));
    }

    Ok(out)
}

fn replace_text_range_xml(
    xml: &str,
    start: usize,
    end: usize,
    replacement: &str,
) -> Result<String, String> {
    let mark = FieldMark {
        start,
        end,
        key: "__text_edit__".to_string(),
    };
    rewrite_xml_with_renderer(xml, &[mark], |_, _, _| replacement.to_string(), "文字")
}

fn rewrite_xml_with_renderer<F>(
    xml: &str,
    marks: &[FieldMark],
    render_replacement: F,
    missed_label: &str,
) -> Result<String, String>
where
    F: Fn(usize, &FieldMark, &str) -> String,
{
    let para_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
    let t_re = Regex::new(r"(?s)(<w:t[^>]*>)([^<]*)(</w:t>)").unwrap();

    let mut sorted = marks.iter().enumerate().collect::<Vec<_>>();
    sorted.sort_by_key(|(_, m)| m.start);

    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    let mut char_offset: usize = 0;
    let para_iter = para_re.find_iter(xml);
    let mut matched = vec![false; marks.len()];

    for p in para_iter {
        out.push_str(&xml[last..p.start()]);
        last = p.end();

        let para_xml = p.as_str();

        struct TextNode {
            full_match_start: usize,
            full_match_end: usize,
            open: String,
            body: String,
            close: String,
            t_start: usize,
            t_end: usize,
        }

        let mut text_nodes: Vec<TextNode> = Vec::new();
        let mut temp_offset = char_offset;

        for cap in t_re.captures_iter(para_xml) {
            let m = cap.get(0).unwrap();
            let open = cap.get(1).unwrap().as_str().to_string();
            let body = cap.get(2).unwrap().as_str().to_string();
            let close = cap.get(3).unwrap().as_str().to_string();
            let body_chars = body.chars().count();

            text_nodes.push(TextNode {
                full_match_start: m.start(),
                full_match_end: m.end(),
                open,
                body,
                close,
                t_start: temp_offset,
                t_end: temp_offset + body_chars,
            });
            temp_offset += body_chars;
        }
        char_offset = temp_offset + 1;

        #[derive(Clone)]
        struct NodeAction {
            prefix: String,
            placeholder: String,
            suffix: String,
        }

        let mut node_actions: Vec<Option<NodeAction>> = vec![None; text_nodes.len()];

        for (idx, m) in &sorted {
            if matched[*idx] {
                continue;
            }
            let mark_start = m.start;
            let mark_end = m.end;
            let replacement = render_replacement(*idx, m, &m.key);

            let start_node = text_nodes
                .iter()
                .position(|n| mark_start >= n.t_start && mark_start < n.t_end);
            let end_node = text_nodes
                .iter()
                .position(|n| mark_end > n.t_start && mark_end <= n.t_end);

            if let (Some(si), Some(ei)) = (start_node, end_node) {
                if si == ei {
                    let node = &text_nodes[si];
                    let pre_chars = mark_start - node.t_start;
                    let post_chars = node.t_end - mark_end;
                    let body_chars_vec: Vec<char> = node.body.chars().collect();

                    let prefix: String = body_chars_vec[..pre_chars].iter().collect();
                    let suffix: String = if post_chars > 0 {
                        body_chars_vec[body_chars_vec.len() - post_chars..]
                            .iter()
                            .collect()
                    } else {
                        String::new()
                    };

                    if let Some(ref mut action) = node_actions[si] {
                        action.suffix = format!("{}{}{}", action.suffix, prefix, suffix);
                    } else {
                        node_actions[si] = Some(NodeAction {
                            prefix,
                            placeholder: replacement,
                            suffix,
                        });
                    }
                } else {
                    let start_node = &text_nodes[si];
                    let end_node = &text_nodes[ei];

                    let pre_chars = mark_start - start_node.t_start;
                    let body_chars_vec: Vec<char> = start_node.body.chars().collect();
                    let prefix: String = body_chars_vec[..pre_chars].iter().collect();

                    let post_chars = end_node.t_end - mark_end;
                    let body_chars_vec: Vec<char> = end_node.body.chars().collect();
                    let suffix: String = if post_chars > 0 {
                        body_chars_vec[body_chars_vec.len() - post_chars..]
                            .iter()
                            .collect()
                    } else {
                        String::new()
                    };

                    for mid in si + 1..ei {
                        node_actions[mid] = Some(NodeAction {
                            prefix: String::new(),
                            placeholder: String::new(),
                            suffix: String::new(),
                        });
                    }

                    node_actions[si] = Some(NodeAction {
                        prefix,
                        placeholder: replacement,
                        suffix: String::new(),
                    });

                    node_actions[ei] = Some(NodeAction {
                        prefix: String::new(),
                        placeholder: String::new(),
                        suffix,
                    });
                }
                matched[*idx] = true;
            }
        }

        let mut p_last = 0;
        let mut new_para = String::with_capacity(para_xml.len());

        for (i, node) in text_nodes.iter().enumerate() {
            new_para.push_str(&para_xml[p_last..node.full_match_start]);
            p_last = node.full_match_end;

            if let Some(ref action) = node_actions[i] {
                if !action.prefix.is_empty() {
                    new_para.push_str(&render_t(&node.open, &action.prefix, &node.close));
                }
                if !action.placeholder.is_empty() {
                    new_para.push_str(&render_t(&node.open, &action.placeholder, &node.close));
                }
                if !action.suffix.is_empty() {
                    new_para.push_str(&render_t(&node.open, &action.suffix, &node.close));
                }
            } else {
                new_para.push_str(&node.open);
                new_para.push_str(&node.body);
                new_para.push_str(&node.close);
            }
        }
        new_para.push_str(&para_xml[p_last..]);
        out.push_str(&prepend_para_tag(para_xml, &new_para));
    }
    out.push_str(&xml[last..]);

    let missed: Vec<String> = marks
        .iter()
        .zip(matched.iter())
        .filter_map(|(m, ok)| {
            if *ok {
                None
            } else {
                Some(format!("{}({}-{})", m.key, m.start, m.end))
            }
        })
        .collect();
    if !missed.is_empty() {
        return Err(format!(
            "有 {} 个{}未能写入模板，请重新选择：{}",
            missed.len(),
            missed_label,
            missed.join("、")
        ));
    }

    Ok(out)
}

fn render_t(open: &str, body: &str, close: &str) -> String {
    let body_esc = body
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let needs_preserve =
        (body.starts_with(' ') || body.ends_with(' ')) && !open.contains("xml:space");
    if needs_preserve {
        // 在 open 末尾插入 xml:space="preserve"
        let mut o = open.to_string();
        if let Some(pos) = o.rfind('>') {
            o.insert_str(pos, r#" xml:space="preserve""#);
        }
        format!("{}{}{}", o, body_esc, close)
    } else {
        format!("{}{}{}", open, body_esc, close)
    }
}

/// 把 new_para_inner 包回 <w:p ...> ... </w:p>（保留原段落属性）
fn prepend_para_tag(original_para: &str, new_inner: &str) -> String {
    // original_para = <w:p ...>BODY</w:p>，new_inner 已是替换好的 BODY
    // 简化：找到 <w:p ...> 开始标签的结束位置和 </w:p>
    let p_open_re = Regex::new(r"^(<w:p\b[^>]*>)").unwrap();
    let p_close = "</w:p>";
    let open_match = p_open_re.find(original_para);
    if let Some(m) = open_match {
        let open_tag = m.as_str();
        format!("{}{}{}", open_tag, new_inner, p_close)
    } else {
        new_inner.to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserTemplate {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub builtin: bool,
    pub created_at: String,
    pub path: String,
}

pub fn user_templates_dir() -> Option<PathBuf> {
    templates::user_data_dir().map(|p| p.join("user_templates"))
}

pub fn user_template_path(id: &str) -> Option<PathBuf> {
    user_templates_dir().map(|p| p.join(format!("{id}.docsytpl")))
}

pub fn user_template_exists(id: &str) -> bool {
    user_template_path(id).is_some_and(|p| p.exists())
}

pub fn is_builtin_template_id(id: &str) -> bool {
    id == "letter" || id.is_empty()
}

/// 列出用户数据目录里的所有 .docsytpl
pub fn list_user_templates() -> Vec<UserTemplate> {
    let Some(dir) = user_templates_dir() else {
        return vec![];
    };
    let _ = fs::create_dir_all(&dir);
    let Ok(entries) = fs::read_dir(&dir) else {
        return vec![];
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("docsytpl") {
            continue;
        }
        if let Ok(file) = fs::File::open(&path) {
            if let Ok(mut z) = ZipArchive::new(file) {
                if let Ok(mut m) = z.by_name("manifest.json") {
                    let mut s = String::new();
                    if m.read_to_string(&mut s).is_ok() {
                        if let Ok(val) = serde_json::from_str::<Value>(&s) {
                            let id = val
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            if is_builtin_template_id(&id) {
                                continue;
                            }
                            let name = val
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let typ = val
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("custom")
                                .to_string();
                            let created = val
                                .get("created_at")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            out.push(UserTemplate {
                                id,
                                name,
                                r#type: typ,
                                builtin: false,
                                created_at: created,
                                path: path.display().to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

#[derive(Debug, Deserialize)]
pub struct SaveTemplateArgs {
    pub manifest: Value,
    pub fields: Value,
    pub dictionaries: Option<Value>,
    pub original_docx_base64: String,
    /// 模板制作器状态。用于再次编辑时回到原始文档和原始标记，而不是
    /// 从已经写入 {{key}} 的 template.docx 里反推。
    pub builder_state: Option<Value>,
    pub marks: Vec<FieldMark>,
}

pub fn save_user_template(args: SaveTemplateArgs) -> Result<UserTemplate, String> {
    use base64_decode::decode_base64;
    let docx_bytes = decode_base64(&args.original_docx_base64)?;
    let rewritten = rewrite_with_placeholders(&docx_bytes, &args.marks)?;

    let id = args
        .manifest
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("manifest 缺 id")?
        .to_string();
    let dir = user_templates_dir().ok_or("无法解析用户模板目录")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{id}.docsytpl"));

    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(Cursor::new(&mut buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zw.start_file("manifest.json", opts)
            .map_err(|e| e.to_string())?;
        zw.write_all(
            serde_json::to_string_pretty(&args.manifest)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;

        zw.start_file("fields.json", opts)
            .map_err(|e| e.to_string())?;
        zw.write_all(
            serde_json::to_string_pretty(&args.fields)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;

        if let Some(d) = &args.dictionaries {
            zw.start_file("dictionaries.json", opts)
                .map_err(|e| e.to_string())?;
            zw.write_all(
                serde_json::to_string_pretty(d)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
        }

        if let Some(state) = &args.builder_state {
            zw.start_file("builder_state.json", opts)
                .map_err(|e| e.to_string())?;
            zw.write_all(
                serde_json::to_string_pretty(state)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
        }

        zw.start_file("template.docx", opts)
            .map_err(|e| e.to_string())?;
        zw.write_all(&rewritten).map_err(|e| e.to_string())?;

        zw.finish().map_err(|e| e.to_string())?;
    }

    fs::write(&path, &buf).map_err(|e| e.to_string())?;

    Ok(UserTemplate {
        id: id.clone(),
        name: args
            .manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string(),
        r#type: args
            .manifest
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("custom")
            .to_string(),
        builtin: false,
        created_at: args
            .manifest
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        path: path.display().to_string(),
    })
}

/// 简单 base64 解码（避免拉新 crate）
pub mod base64_decode {
    pub fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
        let mut buf = Vec::with_capacity(input.len() * 3 / 4);
        let mut accum: u32 = 0;
        let mut bits: u32 = 0;
        for c in input.chars() {
            let v = match c {
                'A'..='Z' => (c as u32) - ('A' as u32),
                'a'..='z' => (c as u32) - ('a' as u32) + 26,
                '0'..='9' => (c as u32) - ('0' as u32) + 52,
                '+' => 62,
                '/' => 63,
                '=' | '\r' | '\n' | ' ' | '\t' => continue,
                _ => return Err(format!("非法 base64 字符：{c:?}")),
            };
            accum = (accum << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                buf.push(((accum >> bits) & 0xff) as u8);
            }
        }
        Ok(buf)
    }
}

pub fn read_template_file(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut entry = z.by_name(name).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn delete_user_template(id: &str) -> Result<(), String> {
    let dir = user_templates_dir().ok_or("无法解析用户模板目录")?;
    let path = dir.join(format!("{id}.docsytpl"));
    fs::remove_file(&path).map_err(|e| e.to_string())
}

/// 重命名用户模板：更新 docsytpl 内的 manifest.json 中的 name 字段
pub fn rename_user_template(id: &str, new_name: &str) -> Result<(), String> {
    let dir = user_templates_dir().ok_or("无法解析用户模板目录")?;
    let path = dir.join(format!("{id}.docsytpl"));

    // 读取整个 docsytpl
    let file = fs::File::open(&path).map_err(|e| format!("打开模板失败：{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("读取 zip 失败：{e}"))?;

    // 收集所有文件
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        files.push((name, data));
    }

    // 修改 manifest.json 中的 name
    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, data) in &files {
            writer.start_file(name, opts).map_err(|e| e.to_string())?;
            if name == "manifest.json" {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(data).map_err(|e| format!("解析 manifest 失败：{e}"))?;
                if let serde_json::Value::Object(ref mut map) = manifest {
                    map.insert(
                        "name".to_string(),
                        serde_json::Value::String(new_name.to_string()),
                    );
                }
                let pretty = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
                writer
                    .write_all(pretty.as_bytes())
                    .map_err(|e| e.to_string())?;
            } else {
                writer.write_all(data).map_err(|e| e.to_string())?;
            }
        }
        writer.finish().map_err(|e| e.to_string())?;
    }

    fs::write(&path, &out_buf).map_err(|e| format!("写入模板失败：{e}"))
}

/// 更新用户模板包里的字段和字典配置。
///
/// 用户模板的当前版本应以 `.docsytpl` 为主，模板管理和模板编辑器都读写同一份包。
/// 这个函数只替换 `fields.json` / `dictionaries.json`，保留 manifest、template.docx、
/// builder_state.json 和其他 docx 资源。
pub fn update_user_template_config(
    id: &str,
    fields: &Value,
    dictionaries: &Value,
) -> Result<(), String> {
    let path = user_template_path(id).ok_or("无法解析用户模板目录")?;
    if !path.exists() {
        return Err(format!("模板不存在或已被删除：{id}"));
    }

    let file = fs::File::open(&path).map_err(|e| format!("打开模板失败：{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("读取模板包失败：{e}"))?;

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut has_fields = false;
    let mut has_dictionaries = false;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        if name == "fields.json" {
            has_fields = true;
            data = serde_json::to_string_pretty(fields)
                .map_err(|e| e.to_string())?
                .into_bytes();
        } else if name == "dictionaries.json" {
            has_dictionaries = true;
            data = serde_json::to_string_pretty(dictionaries)
                .map_err(|e| e.to_string())?
                .into_bytes();
        }
        files.push((name, data));
    }

    if !has_fields {
        files.push((
            "fields.json".to_string(),
            serde_json::to_string_pretty(fields)
                .map_err(|e| e.to_string())?
                .into_bytes(),
        ));
    }
    if !has_dictionaries {
        files.push((
            "dictionaries.json".to_string(),
            serde_json::to_string_pretty(dictionaries)
                .map_err(|e| e.to_string())?
                .into_bytes(),
        ));
    }

    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in files {
            writer.start_file(name, opts).map_err(|e| e.to_string())?;
            writer.write_all(&data).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
    }

    fs::write(&path, &out_buf).map_err(|e| format!("写入模板失败：{e}"))
}

/// 读取模板完整数据用于编辑：manifest + fields + dictionaries + 原始 docx base64 + 重建的 marks
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateEditData {
    pub manifest: Value,
    pub fields: Value,
    pub dictionaries: Value,
    pub docx_base64: String,
    pub marks: Vec<FieldMark>,
    pub builder_state: Option<Value>,
}

pub fn read_template_for_edit(id: &str) -> Result<TemplateEditData, String> {
    // 内置模板没有用户当前版本时，使用编译进来的出厂资源。
    // 一旦存在 user_templates/<id>.docsytpl，该文件就是该 templateId 的当前版本。
    if is_builtin_template_id(id) && !user_template_exists(id) {
        let docx_bytes = crate::BUILTIN_LETTER_DOCX.to_vec();
        let fields: Value = serde_json::from_str(crate::BUILTIN_LETTER_FIELDS)
            .map_err(|e| format!("解析内置字段失败：{e}"))?;
        let dictionaries: Value = serde_json::from_str(crate::BUILTIN_DICTIONARIES)
            .map_err(|e| format!("解析内置字典失败：{e}"))?;

        let manifest = serde_json::json!({
            "id": "letter",
            "name": "律师事务所函",
            "type": "builtin",
            "builtin": true,
            "created_at": "",
            "version": "1.0.0"
        });

        // 从 docx 中重建 marks
        // 注意：需要先合并 run，与 rewrite_with_placeholders 保持一致
        let doc_xml = {
            let cursor = std::io::Cursor::new(&docx_bytes);
            let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
            let mut entry = archive
                .by_name("word/document.xml")
                .map_err(|e| e.to_string())?;
            let mut xml = String::new();
            entry.read_to_string(&mut xml).map_err(|e| e.to_string())?;
            xml
        };

        // 先合并 run，确保位置计算与写入时一致
        let doc_xml = flatten_nested_paragraphs(&doc_xml);
        let doc_xml = merge_adjacent_runs(&doc_xml);
        let doc_xml = merge_adjacent_text_nodes(&doc_xml);

        let para_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
        let t_re = Regex::new(r"(?s)<w:t[^>]*>([^<]*)</w:t>").unwrap();
        let ph_re = Regex::new(r"\{\{([*#]?\w+)\}\}").unwrap();

        let mut plain_text = String::new();
        let mut marks = Vec::new();

        for pm in para_re.captures_iter(&doc_xml) {
            let body = pm.get(1).unwrap().as_str();
            for tc in t_re.captures_iter(body) {
                let t_content = tc.get(1).unwrap().as_str();
                let offset = plain_text.chars().count();
                for ph in ph_re.captures_iter(t_content) {
                    let key = ph.get(1).unwrap().as_str().to_string();
                    let ph_start = ph.get(0).unwrap().start();
                    let ph_end = ph.get(0).unwrap().end();
                    let char_start = offset + t_content[..ph_start].chars().count();
                    let char_end = offset + t_content[..ph_end].chars().count();
                    marks.push(FieldMark {
                        start: char_start,
                        end: char_end,
                        key,
                    });
                }
                plain_text.push_str(t_content);
            }
            plain_text.push('\n');
        }

        let docx_base64 = crate::template_builder::base64_encode::encode_base64(&docx_bytes);

        return Ok(TemplateEditData {
            manifest,
            fields,
            dictionaries,
            docx_base64,
            marks,
            builder_state: None,
        });
    }

    // 用户模板
    let path = user_template_path(id).ok_or("无法解析用户模板目录")?;
    if !path.exists() {
        return Err(format!("模板不存在或已被删除：{id}"));
    }

    let manifest_bytes = read_template_file(&path, "manifest.json")?;
    let fields_bytes = read_template_file(&path, "fields.json")?;
    let docx_bytes = read_template_file(&path, "template.docx")?;

    let manifest: Value =
        serde_json::from_slice(&manifest_bytes).map_err(|e| format!("解析 manifest 失败：{e}"))?;
    let fields: Value =
        serde_json::from_slice(&fields_bytes).map_err(|e| format!("解析 fields 失败：{e}"))?;
    let fields = templates::merge_letter_config(fields, templates::read_config(id));

    let dictionaries: Value = read_template_file(&path, "dictionaries.json")
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let dictionaries =
        templates::merge_dictionaries(dictionaries, templates::read_config(&format!("dict_{id}")));

    let builder_state: Option<Value> = read_template_file(&path, "builder_state.json")
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());

    // 从 template.docx 中重建 marks：找所有 {{key}} 占位符在纯文本中的位置
    // 注意：需要先合并 run，与 rewrite_with_placeholders 保持一致
    let doc_xml = {
        let cursor = std::io::Cursor::new(&docx_bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
        let mut entry = archive
            .by_name("word/document.xml")
            .map_err(|e| e.to_string())?;
        let mut xml = String::new();
        entry.read_to_string(&mut xml).map_err(|e| e.to_string())?;
        xml
    };

    // 先合并 run，确保位置计算与写入时一致
    let doc_xml = flatten_nested_paragraphs(&doc_xml);
    let doc_xml = merge_adjacent_runs(&doc_xml);
    let doc_xml = merge_adjacent_text_nodes(&doc_xml);

    let para_re = Regex::new(r"(?s)<w:p\b[^>]*>(.*?)</w:p>").unwrap();
    let t_re = Regex::new(r"(?s)<w:t[^>]*>([^<]*)</w:t>").unwrap();
    let ph_re = Regex::new(r"\{\{([*#]?\w+)\}\}").unwrap();

    let mut plain_text = String::new();
    let mut marks = Vec::new();

    for pm in para_re.captures_iter(&doc_xml) {
        let body = pm.get(1).unwrap().as_str();
        for tc in t_re.captures_iter(body) {
            let t_content = tc.get(1).unwrap().as_str();
            let offset = plain_text.chars().count();

            // 在 t_content 中找所有占位符
            for ph in ph_re.captures_iter(t_content) {
                let key = ph.get(1).unwrap().as_str().to_string();
                let ph_start = ph.get(0).unwrap().start();
                let ph_end = ph.get(0).unwrap().end();

                // 计算占位符在纯文本中的字符偏移
                let char_start = offset + t_content[..ph_start].chars().count();
                let char_end = offset + t_content[..ph_end].chars().count();

                marks.push(FieldMark {
                    start: char_start,
                    end: char_end,
                    key,
                });
            }

            plain_text.push_str(t_content);
        }
        plain_text.push('\n');
    }

    // docx 转 base64。新格式优先返回制作器保存的原始 docx，
    // 旧格式没有 builder_state 时只能回退到已经占位符化的 template.docx。
    let docx_base64 = builder_state
        .as_ref()
        .and_then(|s| s.get("sourceDocxBase64"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| base64_encode::encode_base64(&docx_bytes));

    Ok(TemplateEditData {
        manifest,
        fields,
        dictionaries,
        docx_base64,
        marks,
        builder_state,
    })
}

/// base64 编码
pub mod base64_encode {
    pub fn encode_base64(input: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
            out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                out.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(CHARS[(n & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let data = b"Hello, World!";
        let encoded = base64_encode::encode_base64(data);
        let decoded = base64_decode::decode_base64(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_extract_plain_text() {
        // 创建一个简单的 docx 用于测试
        let docx = create_test_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello World</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let result = extract_plain_text(&docx).unwrap();
        assert!(result.plain_text.contains("Hello World"));
        assert_eq!(result.paragraph_count, 1);
    }

    #[test]
    fn test_replace_text_range() {
        let docx = create_test_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello World</w:t></w:r></w:p></w:body></w:document>"#,
        );
        // "Hello World" 的偏移是 0..11
        let result = replace_text_range(&docx, 0, 5, "Hi");
        assert!(result.is_ok());
        let new_docx = result.unwrap();
        let text = extract_plain_text(&new_docx).unwrap();
        assert!(text.plain_text.contains("Hi World"));
    }

    #[test]
    fn test_rewrite_with_placeholders() {
        let docx = create_test_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>法院名称</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let marks = vec![FieldMark {
            start: 0,
            end: 4,
            key: "court".to_string(),
        }];
        let result = rewrite_with_placeholders(&docx, &marks);
        assert!(result.is_ok());
        let new_docx = result.unwrap();
        let text = extract_plain_text(&new_docx).unwrap();
        assert!(text.plain_text.contains("{{court}}"));
    }

    #[test]
    fn test_rewrite_with_placeholders_after_multiple_paragraphs() {
        let docx = create_test_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>第一段</w:t></w:r></w:p><w:p><w:r><w:t>第二段</w:t></w:r></w:p><w:p><w:r><w:t>第三段字段</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let plain = extract_plain_text(&docx).unwrap().plain_text;
        let start_byte = plain.find("字段").unwrap();
        let start = plain[..start_byte].chars().count();
        let end = start + "字段".chars().count();
        let marks = vec![FieldMark {
            start,
            end,
            key: "target".to_string(),
        }];

        let result = rewrite_with_placeholders(&docx, &marks);
        assert!(result.is_ok(), "{result:?}");
        let new_docx = result.unwrap();
        let text = extract_plain_text(&new_docx).unwrap();
        assert!(text.plain_text.contains("第三段{{target}}"));
    }

    #[test]
    fn test_rewrite_with_placeholders_missed() {
        let docx = create_test_docx(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let marks = vec![FieldMark {
            start: 100, // 超出范围
            end: 105,
            key: "test".to_string(),
        }];
        let result = rewrite_with_placeholders(&docx, &marks);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未能写入"));
    }

    #[test]
    fn test_flatten_nested_paragraphs() {
        // 嵌套的 <w:p> 应该被展开
        let xml = r#"<w:p><w:r><w:p><w:r><w:t>test</w:t></w:r></w:p></w:r></w:p>"#;
        let result = flatten_nested_paragraphs(xml);
        // 展开后应该只剩下内层的 <w:p> 内容
        assert!(result.contains("<w:t>test</w:t>"));
    }

    #[test]
    fn test_merge_adjacent_runs() {
        let xml =
            r#"<w:p><w:r><w:rPr/><w:t>Hello </w:t></w:r><w:r><w:rPr/><w:t>World</w:t></w:r></w:p>"#;
        let result = merge_adjacent_runs(xml);
        assert!(result.contains("Hello World"));
    }

    fn create_test_docx(body_xml: &str) -> Vec<u8> {
        use std::io::Cursor;
        use zip::write::{FileOptions, ZipWriter};

        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            let opts = FileOptions::default();

            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).unwrap();

            zip.start_file("_rels/.rels", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#).unwrap();

            zip.start_file("word/document.xml", opts).unwrap();
            zip.write_all(body_xml.as_bytes()).unwrap();

            zip.start_file("word/_rels/document.xml.rels", opts)
                .unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#).unwrap();

            zip.start_file("word/styles.xml", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style></w:styles>"#).unwrap();

            zip.finish().unwrap();
        }
        buf
    }
}
