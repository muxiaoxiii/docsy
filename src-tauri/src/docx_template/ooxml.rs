use anyhow::{Context, Result};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Reader;
use quick_xml::Writer;
use std::io::Cursor;

#[derive(Debug, Clone, PartialEq)]
pub enum XmlNode {
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<XmlNode>,
    },
    Text(String),
}

#[derive(Debug, Clone)]
pub struct XmlTree {
    pub root: XmlNode,
}

impl XmlTree {
    pub fn parse(xml_bytes: &[u8]) -> Result<Self> {
        // OOXML standard uses UTF-8; reject incompatible encodings early
        if xml_bytes.len() >= 2 && xml_bytes[0] == 0xFE && xml_bytes[1] == 0xFF {
            anyhow::bail!(
                "不支持的 XML 编码（UTF-16BE）。请用 Word/WPS 重新保存文档为 docx 格式。"
            );
        }
        if xml_bytes.len() >= 2 && xml_bytes[0] == 0xFF && xml_bytes[1] == 0xFE {
            anyhow::bail!(
                "不支持的 XML 编码（UTF-16LE）。请用 Word/WPS 重新保存文档为 docx 格式。"
            );
        }
        if xml_bytes.len() >= 3
            && xml_bytes[0] == 0xEF
            && xml_bytes[1] == 0xBB
            && xml_bytes[2] == 0xBF
        {
            // UTF-8 BOM — strip it
            let without_bom = &xml_bytes[3..];
            return Self::parse_utf8(without_bom);
        }
        Self::parse_utf8(xml_bytes)
    }

    fn parse_utf8(xml_bytes: &[u8]) -> Result<Self> {
        let preview_len = std::cmp::min(xml_bytes.len(), 256);
        let preview = String::from_utf8_lossy(&xml_bytes[..preview_len]);

        // Quick check for non-UTF-8 encoding declaration in XML prolog
        if let Some(decl) = preview.strip_prefix("<?xml ") {
            let lower = decl.to_lowercase();
            if let Some(enc_start) = lower.find("encoding=") {
                let rest = &decl[enc_start + 9..];
                let enc_val = rest
                    .trim_start_matches(['"', '\''])
                    .split(['"', '\''])
                    .next()
                    .unwrap_or("");
                let enc_lower = enc_val.to_lowercase();
                if enc_lower.contains("utf-16") || enc_lower.contains("iso-2022") {
                    anyhow::bail!(
                        "不支持的 XML 编码（{}）。请用 Word/WPS 重新保存文档为 docx 格式。",
                        enc_val
                    );
                }
                // ASCII-compatible encodings (windows-1252, shift_jis, etc.) are
                // handled by quick-xml's encoding feature, so we let them through
            }
        }

        let mut reader = Reader::from_reader(Cursor::new(xml_bytes));
        // Word uses xml:space="preserve" for meaningful leading and trailing
        // whitespace. Trimming here would silently change the document before
        // any template operation has a chance to preserve it.
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let root = parse_children(&mut reader, &mut buf, 0)?;
        // The XML declaration may be followed by whitespace. Preserve that
        // whitespace inside elements, but never mistake it for the document
        // root.
        let root = root
            .into_iter()
            .find(|node| matches!(node, XmlNode::Element { .. }))
            .unwrap_or(XmlNode::Text(String::new()));
        Ok(Self { root })
    }

    pub fn to_xml(&self) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        write_node(&mut writer, &self.root)?;
        writer.write_event(Event::Eof)?;
        let bytes = writer.into_inner().into_inner();
        String::from_utf8(bytes).context("序列化 XML 失败")
    }
}

fn parse_children<R: std::io::BufRead>(
    reader: &mut Reader<R>,
    buf: &mut Vec<u8>,
    depth: usize,
) -> Result<Vec<XmlNode>> {
    let max_depth = 256;
    if depth > max_depth {
        anyhow::bail!("XML 嵌套层级超出限制（{} 层）", max_depth);
    }

    let mut children = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .context("XML 标签名无效")?
                    .to_string();
                let attrs = parse_attributes(&e, reader.decoder());
                let sub_children = parse_children(reader, buf, depth + 1)?;
                children.push(XmlNode::Element {
                    name,
                    attrs,
                    children: sub_children,
                });
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().context("XML 文本节点转义失败")?.to_string();
                if !text.is_empty() {
                    children.push(XmlNode::Text(text));
                }
            }
            Ok(Event::CData(e)) => {
                // CDATA 内容不再静默丢弃：按文本节点保留（写出时重新转义，
                // 与原文等价）。输入已是 &str，必为合法 UTF-8。
                let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                if !text.is_empty() {
                    children.push(XmlNode::Text(text));
                }
            }
            Ok(Event::End(_)) | Ok(Event::Eof) => {
                return Ok(children);
            }
            Ok(Event::Empty(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .context("XML 标签名无效")?
                    .to_string();
                let attrs = parse_attributes(&e, reader.decoder());
                children.push(XmlNode::Element {
                    name,
                    attrs,
                    children: Vec::new(),
                });
            }
            Err(e) => return Err(anyhow::anyhow!("XML 解析错误: {}", e)),
            _ => {}
        }
    }
}

/// 解析元素属性。畸形属性不再静默丢弃：记录告警日志后跳过该属性；
/// 值反转义失败时按空值处理并告警。
fn parse_attributes(
    e: &BytesStart,
    decoder: quick_xml::encoding::Decoder,
) -> Vec<(String, String)> {
    e.attributes()
        .filter_map(|attr| {
            let attr = match attr {
                Ok(attr) => attr,
                Err(err) => {
                    log::warn!("XML 属性解析失败，已跳过该属性: {err}");
                    return None;
                }
            };
            let key = match std::str::from_utf8(attr.key.as_ref()) {
                Ok(key) => key.to_string(),
                Err(_) => {
                    log::warn!("XML 属性名不是合法 UTF-8，已跳过该属性");
                    return None;
                }
            };
            match attr.decode_and_unescape_value(decoder) {
                Ok(value) => Some((key, value.to_string())),
                Err(err) => {
                    log::warn!("XML 属性 {key} 的值反转义失败，按空值处理: {err}");
                    Some((key, String::new()))
                }
            }
        })
        .collect()
}

fn write_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &XmlNode) -> Result<()> {
    match node {
        XmlNode::Element {
            name,
            attrs,
            children,
        } => {
            let mut elem = BytesStart::new(name.as_str());
            for (key, value) in attrs {
                // (&str, &str) 形式的 push_attribute 内部已对值做 XML 转义
                // （& < > ' "），此处只需剔除 XML 1.0 非法的控制字符——它们
                // 可能来自解析端对 &#x1; 这类数值字符引用的还原。
                let sanitized = sanitize_xml_text(value);
                elem.push_attribute((key.as_str(), sanitized.as_str()));
            }
            if children.is_empty() {
                writer.write_event(Event::Empty(elem))?;
            } else {
                writer.write_event(Event::Start(elem))?;
                for child in children {
                    write_node(writer, child)?;
                }
                writer.write_event(Event::End(BytesEnd::new(name.as_str())))?;
            }
        }
        XmlNode::Text(text) => {
            // 写入前剔除 XML 1.0 非法的控制字符，再转义保留字符
            let escaped = escape_xml_value(&sanitize_xml_text(text));
            writer.write_event(Event::Text(BytesText::from_escaped(escaped)))?;
        }
    }
    Ok(())
}

/// 转义 XML 保留字符（& < > "）。输入必须是已反转义的原始值，
/// 本函数无条件转义，不存在二次转义问题。
fn escape_xml_value(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 剔除 XML 1.0 不允许的控制字符（\x00-\x08 \x0B \x0C \x0E-\x1F），
/// 保留 \t \n \r。
fn sanitize_xml_text(text: &str) -> String {
    text.chars()
        .filter(|&c| !matches!(c, '\x00'..='\x08' | '\x0B' | '\x0C' | '\x0E'..='\x1F'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_document() {
        let xml =
            br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:rPr/><w:t>Hello</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let tree = XmlTree::parse(xml).expect("parse xml");
        let output = tree.to_xml().expect("write xml");
        assert!(output.contains("Hello"));
    }

    #[test]
    fn parse_paragraph_with_multiple_runs() {
        let xml = br#"<w:p>
            <w:r><w:t>AAA</w:t></w:r>
            <w:r><w:t>BBB</w:t></w:r>
        </w:p>"#;

        let tree = XmlTree::parse(xml).expect("parse");
        let out = tree.to_xml().expect("write");
        assert!(out.contains("AAA"));
        assert!(out.contains("BBB"));
    }

    #[test]
    fn xml_special_chars_are_escaped() {
        let mut buf = Vec::new();
        let mut writer = Writer::new(Cursor::new(&mut buf));
        let text = "Tom & Jerry <script>alert('xss')</script>";
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        writer
            .write_event(Event::Text(BytesText::from_escaped(escaped)))
            .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("&amp;"));
        assert!(output.contains("&lt;"));
        assert!(!output.contains("<script>"));
    }

    #[test]
    fn roundtrip_preserves_content() {
        let xml = br#"<w:document>
            <w:body>
                <w:p><w:r><w:t>Test content</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;
        let tree = XmlTree::parse(xml).unwrap();
        let out = tree.to_xml().unwrap();
        // After roundtrip, declarations may differ but content must match
        assert!(out.contains("Test content"));
        assert!(out.contains("<w:document"));
        assert!(out.contains("</w:document>"));
    }

    #[test]
    fn attr_special_chars_roundtrip_stays_valid_xml() {
        // 属性值含 & 和 " 时，往返一次后输出必须仍是合法 XML 且值不变
        let xml = br#"<w:p><w:r><w:rPr><w:rStyle w:val="Tom &amp; Jerry &quot;Style&quot;"/></w:rPr><w:t>x</w:t></w:r></w:p>"#;
        let tree = XmlTree::parse(xml).unwrap();

        // 解析端已反转义为原始值
        if let XmlNode::Element { children, .. } = &tree.root {
            let run = children.iter().find_map(|c| match c {
                XmlNode::Element { name, children, .. } if name == "w:r" => Some(children),
                _ => None,
            });
            let mut found = None;
            if let Some(run_children) = run {
                for c in run_children {
                    if let XmlNode::Element { name, children, .. } = c {
                        if name == "w:rPr" {
                            for rpr_child in children {
                                if let XmlNode::Element { attrs, .. } = rpr_child {
                                    found = attrs
                                        .iter()
                                        .find(|(k, _)| k == "w:val")
                                        .map(|(_, v)| v.clone());
                                }
                            }
                        }
                    }
                }
            }
            assert_eq!(found.as_deref(), Some("Tom & Jerry \"Style\""));
        }

        let out = tree.to_xml().unwrap();
        // 写回时重新转义，且不能二次转义
        assert!(out.contains("Tom &amp; Jerry &quot;Style&quot;"));
        assert!(!out.contains("&amp;amp;"));
        // 输出本身必须能被再次解析且值一致
        let reparsed = XmlTree::parse(out.as_bytes()).unwrap();
        assert_eq!(reparsed.root, tree.root);
    }

    #[test]
    fn illegal_control_chars_are_stripped_on_write() {
        // \x01 \x0B \x1F 在 XML 1.0 中非法，写入前剔除
        let tree = XmlTree {
            root: XmlNode::Element {
                name: "w:r".to_string(),
                attrs: Vec::new(),
                children: vec![XmlNode::Element {
                    name: "w:t".to_string(),
                    attrs: Vec::new(),
                    children: vec![XmlNode::Text("ab\x01\x0B\x1Fcd".to_string())],
                }],
            },
        };
        let out = tree.to_xml().unwrap();
        assert!(out.contains(">abcd<"), "control chars stripped: {out}");
        // \t \n \r 是合法字符，必须保留
        let tree = XmlTree {
            root: XmlNode::Element {
                name: "w:t".to_string(),
                attrs: Vec::new(),
                children: vec![XmlNode::Text("a\tb\nc\rd".to_string())],
            },
        };
        let out = tree.to_xml().unwrap();
        assert!(out.contains("a\tb\nc\rd"), "legal whitespace kept: {out}");
    }
}
