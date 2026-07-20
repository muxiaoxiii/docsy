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
        let xml_str = std::str::from_utf8(xml_bytes).context("XML is not valid UTF-8 encoding")?;

        // Quick check for non-UTF-8 encoding declaration in XML prolog
        if let Some(decl) = xml_str.strip_prefix("<?xml ") {
            let lower = decl.to_lowercase();
            if let Some(enc_start) = lower.find("encoding=") {
                let rest = &decl[enc_start + 9..];
                let enc_val = rest
                    .trim_start_matches(['"', '\''])
                    .split(|c| c == '"' || c == '\'')
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

        let mut reader = Reader::from_str(xml_str);
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
                let attrs = e
                    .attributes()
                    .filter_map(|attr| {
                        let attr = attr.ok()?;
                        let key = std::str::from_utf8(attr.key.as_ref()).ok()?.to_string();
                        let value = attr.decode_and_unescape_value(reader.decoder()).ok();
                        Some((key, value.unwrap_or_default().to_string()))
                    })
                    .collect::<Vec<_>>();
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
            Ok(Event::End(_)) | Ok(Event::Eof) => {
                return Ok(children);
            }
            Ok(Event::Empty(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .context("XML 标签名无效")?
                    .to_string();
                let attrs = e
                    .attributes()
                    .filter_map(|attr| {
                        let attr = attr.ok()?;
                        let key = std::str::from_utf8(attr.key.as_ref()).ok()?.to_string();
                        let value = attr.decode_and_unescape_value(reader.decoder()).ok();
                        Some((key, value.unwrap_or_default().to_string()))
                    })
                    .collect();
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

fn write_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &XmlNode) -> Result<()> {
    match node {
        XmlNode::Element {
            name,
            attrs,
            children,
        } => {
            let mut elem = BytesStart::new(name.as_str());
            for (key, value) in attrs {
                elem.push_attribute((key.as_str(), value.as_str()));
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
            writer.write_event(Event::Text(BytesText::new(text)))?;
        }
    }
    Ok(())
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
}
