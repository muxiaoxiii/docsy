use crate::docx_template::index::{DocumentIndex, TextIndex, TextNodeRef};
use crate::docx_template::ooxml::{XmlNode, XmlTree};
use anyhow::Result;

const CHECKBOX_CHARS: &[char] = &['□', '☐', '☑', '☒', '✓', '√', '✔', '✗', '○', '●'];
const CHECKBOX_TEXTS: &[&str] = &["(√)", "（√）", "( )", "（ ）"];

/// Scan a document XML tree and build a TextIndex
pub fn scan_document_index(part_name: &str, tree: &XmlTree) -> Result<TextIndex> {
    let mut index = TextIndex::new(part_name);

    if let XmlNode::Element { name, children, .. } = &tree.root {
        // Handle both document fragments (root = w:p) and full documents (root = w:document > w:body)
        if name == "w:p" {
            index.add_paragraph();
            scan_paragraph_runs(children, &mut index, 0);
        } else {
            scan_element_children_recursive(children, &mut index, &mut Vec::new());
        }
    }

    Ok(index)
}

/// Scan all XML parts as raw bytes and build a unified DocumentIndex
pub fn scan_package_index_to_document_index(parts: &[(&str, &[u8])]) -> Result<DocumentIndex> {
    let mut doc = DocumentIndex::new();
    for (part_name, data) in parts {
        let tree = XmlTree::parse(data)?;
        doc.add_part(
            part_name.to_string(),
            scan_document_index(part_name, &tree)?,
        );
    }
    Ok(doc)
}

fn scan_element_children_recursive(
    children: &[XmlNode],
    index: &mut TextIndex,
    _path: &mut Vec<String>,
) {
    for node in children {
        if let XmlNode::Element { name, children, .. } = node {
            if name == "w:p" {
                index.add_paragraph();
                let p_idx = index.paragraph_count - 1;
                scan_paragraph_runs(children, index, p_idx);
            }
            // Always recurse into all children — w:p inside txbxContent are descendants
            scan_element_children_recursive(children, index, _path);
        }
    }
}

fn scan_paragraph_runs(children: &[XmlNode], index: &mut TextIndex, paragraph_idx: usize) {
    let mut run_idx = 0;

    for child in children {
        if let XmlNode::Element {
            name,
            attrs: _,
            children,
        } = child
        {
            if name != "w:r" {
                // Recursively scan nested elements (e.g., w:sdt > w:sdtContent > w:r)
                if name == "w:sdt" {
                    scan_paragraph_runs(children, index, paragraph_idx);
                }
                continue;
            }

            let mut bold = false;
            let mut italic = false;
            let mut underline = false;
            let mut highlighted = false;

            // Check w:rPr for formatting
            for rpr_child in children.iter() {
                if let XmlNode::Element { name, attrs: _, .. } = rpr_child {
                    if name == "w:rPr" {
                        bold = has_element_in(rpr_child, "w:b");
                        italic = has_element_in(rpr_child, "w:i");
                        underline = has_element_in(rpr_child, "w:u");
                        highlighted = has_yellow_highlight(rpr_child);
                    }
                }
            }

            // A run is the stable coordinate shared by scan and save. A run can
            // contain several direct w:t children, but it must still produce one
            // selectable node; producing one node per w:t duplicates the same
            // mark id and makes a later save ambiguous.
            let text = collect_direct_run_text(children);
            if !text.is_empty() {
                let checkbox_like = is_checkbox_text(&text);
                let option_label = if checkbox_like {
                    text.clone()
                } else {
                    String::new()
                };
                index.add_text_node(TextNodeRef {
                    paragraph_index: paragraph_idx,
                    run_index: run_idx,
                    text_index: 0,
                    text,
                    highlighted,
                    bold,
                    italic,
                    underline,
                    checkbox_like,
                    option_label,
                });
                run_idx += 1;
            }
        }
    }
}

fn has_element_in(node: &XmlNode, name: &str) -> bool {
    if let XmlNode::Element {
        name: n, children, ..
    } = node
    {
        if n == name {
            return true;
        }
        for child in children {
            if has_element_in(child, name) {
                return true;
            }
        }
    }
    false
}

fn has_yellow_highlight(node: &XmlNode) -> bool {
    if let XmlNode::Element { name, attrs, .. } = node {
        if name == "w:highlight" {
            return attrs.iter().any(|(k, v)| {
                k == "w:val" && (v == "yellow" || v == "'yellow'" || v == "\"yellow\"")
            });
        }
        if let XmlNode::Element { children, .. } = node {
            for child in children {
                if has_yellow_highlight(child) {
                    return true;
                }
            }
        }
    }
    false
}

fn collect_text(children: &[XmlNode]) -> String {
    let mut buf = String::new();
    for child in children {
        match child {
            XmlNode::Text(text) => buf.push_str(text),
            XmlNode::Element { .. } => {
                buf.push_str(&collect_text(match child {
                    XmlNode::Element { children, .. } => children,
                    _ => continue,
                }));
            }
        }
    }
    buf
}

fn collect_direct_run_text(children: &[XmlNode]) -> String {
    children
        .iter()
        .filter_map(|child| match child {
            XmlNode::Element { name, children, .. } if name == "w:t" => {
                Some(collect_text(children))
            }
            _ => None,
        })
        .collect()
}

fn is_checkbox_text(text: &str) -> bool {
    let trimmed = text.trim();
    CHECKBOX_CHARS.iter().any(|ch| trimmed.contains(*ch))
        || CHECKBOX_TEXTS.iter().any(|t| trimmed.contains(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docx_template::ooxml::XmlTree;

    fn parse_xml(xml_str: &str) -> XmlTree {
        XmlTree::parse(xml_str.as_bytes()).unwrap()
    }

    #[test]
    fn scan_single_paragraph_with_highlight() {
        let tree = parse_xml(
            r#"<w:document><w:body><w:p>
            <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr>
            <w:t>原告</w:t></w:r>
        </w:p></w:body></w:document>"#,
        );

        let index = scan_document_index("word/document.xml", &tree).unwrap();

        assert_eq!(index.total_text_nodes(), 1);
        assert_eq!(index.paragraph_count, 1);
        assert!(index.nodes[0].highlighted);
        assert_eq!(index.nodes[0].text, "原告");
        assert_eq!(index.nodes[0].paragraph_index, 0);
        assert_eq!(index.nodes[0].run_index, 0);
    }

    #[test]
    fn scan_multiple_paragraphs() {
        let tree = parse_xml(
            r#"<w:document><w:body>
            <w:p><w:r><w:t>A</w:t></w:r></w:p>
            <w:p><w:r><w:t>B</w:t></w:r></w:p>
        </w:body></w:document>"#,
        );
        let index = scan_document_index("word/document.xml", &tree).unwrap();

        assert_eq!(index.total_text_nodes(), 2);
        assert_eq!(index.paragraph_count, 2);
        assert_eq!(index.nodes[0].paragraph_index, 0);
        assert_eq!(index.nodes[1].paragraph_index, 1);
    }

    #[test]
    fn scan_split_text_run() {
        let tree = parse_xml(
            r#"<w:document><w:body><w:p>
            <w:r><w:rPr><w:b/></w:rPr><w:t>202</w:t></w:r>
            <w:r><w:t>6</w:t></w:r>
            <w:r><w:t>年</w:t></w:r>
        </w:p></w:body></w:document>"#,
        );
        let index = scan_document_index("word/document.xml", &tree).unwrap();

        assert_eq!(index.total_text_nodes(), 3);
        assert!(index.nodes[0].bold);
        assert!(!index.nodes[1].bold);
        assert_eq!(index.nodes[0].run_index, 0);
        assert_eq!(index.nodes[1].run_index, 1);
    }

    #[test]
    fn scan_checkbox_text() {
        let tree = parse_xml(
            r#"<w:document><w:body><w:p><w:r><w:t>□</w:t></w:r></w:p></w:body></w:document>"#,
        );
        let index = scan_document_index("word/document.xml", &tree).unwrap();

        assert!(index.nodes[0].checkbox_like);
        assert_eq!(index.nodes[0].option_label, "□");
    }

    #[test]
    fn scan_text_box_content() {
        let tree = parse_xml(
            r#"<w:document><w:body><w:p>
            <w:r><w:t>before</w:t></w:r>
            <w:r><w:drawing><w:txbxContent>
                <w:p><w:r><w:t>inside</w:t></w:r></w:p>
            </w:txbxContent></w:drawing></w:r>
            <w:r><w:t>after</w:t></w:r>
        </w:p></w:body></w:document>"#,
        );
        let index = scan_document_index("word/document.xml", &tree).unwrap();

        let texts: Vec<&str> = index.nodes.iter().map(|n| n.text.as_str()).collect();
        eprintln!(
            "scan_text_box: paragraphs={} nodes={} texts={:?}",
            index.paragraph_count,
            index.total_text_nodes(),
            texts
        );
        assert!(texts.contains(&"before"));
        assert!(texts.contains(&"inside"));
        assert!(texts.contains(&"after"));
    }

    #[test]
    fn scan_preserves_non_yellow_highlight() {
        let tree = parse_xml(
            r#"<w:document><w:body><w:p>
            <w:r><w:rPr><w:highlight w:val="green"/></w:rPr>
            <w:t>review</w:t></w:r>
        </w:p></w:body></w:document>"#,
        );
        let index = scan_document_index("word/document.xml", &tree).unwrap();

        assert!(
            !index.nodes[0].highlighted,
            "non-yellow highlight should not be marked"
        );
    }
}
