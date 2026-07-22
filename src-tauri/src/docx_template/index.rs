use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct TextNodeRef {
    pub paragraph_index: usize,
    pub run_index: usize,
    pub text_index: usize,
    pub text: String,
    pub highlighted: bool,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub checkbox_like: bool,
    pub option_label: String,
}

#[derive(Debug, Clone)]
pub struct TextIndex {
    #[allow(dead_code)] // retained for diagnostics when indexing standalone XML parts
    pub part_name: String,
    pub nodes: Vec<TextNodeRef>,
    pub paragraph_count: usize,
}

impl TextIndex {
    pub fn new(part_name: &str) -> Self {
        Self {
            part_name: part_name.to_string(),
            nodes: Vec::new(),
            paragraph_count: 0,
        }
    }

    pub fn add_paragraph(&mut self) {
        self.paragraph_count += 1;
    }

    pub fn add_text_node(&mut self, node: TextNodeRef) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }

    #[allow(dead_code)] // exercised by focused index tests and useful to callers during diagnostics
    pub fn total_text_nodes(&self) -> usize {
        self.nodes.len()
    }
}

/// A unified text index for an entire docx (all XML parts)
#[derive(Debug, Clone)]
pub struct DocumentIndex {
    pub parts: HashMap<String, TextIndex>,
}

impl DocumentIndex {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
        }
    }

    pub fn add_part(&mut self, name: String, index: TextIndex) {
        self.parts.insert(name, index);
    }

    #[allow(dead_code)] // retained as the public traversal API for package diagnostics
    pub fn iter_nodes(&self) -> impl Iterator<Item = (&str, &TextNodeRef)> {
        self.parts.iter().flat_map(|(part_name, index)| {
            index
                .nodes
                .iter()
                .map(move |node| (part_name.as_str(), node))
        })
    }

    #[allow(dead_code)] // retained as the public traversal API for package diagnostics
    pub fn iter_highlighted(&self) -> impl Iterator<Item = (&str, &TextNodeRef)> {
        self.iter_nodes().filter(|(_, node)| node.highlighted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_index_basic() {
        let mut index = TextIndex::new("word/document.xml");
        index.add_paragraph();
        let node = TextNodeRef {
            paragraph_index: 0,
            run_index: 0,
            text_index: 0,
            text: "测试".to_string(),
            highlighted: true,
            bold: false,
            italic: false,
            underline: false,
            checkbox_like: false,
            option_label: String::new(),
        };
        assert_eq!(index.add_text_node(node), 0);
        assert_eq!(index.total_text_nodes(), 1);
        assert_eq!(index.paragraph_count, 1);
    }

    #[test]
    fn document_index_iter() {
        let mut doc = DocumentIndex::new();
        let mut idx = TextIndex::new("word/document.xml");
        idx.add_text_node(TextNodeRef {
            paragraph_index: 0,
            run_index: 0,
            text_index: 0,
            text: "A".to_string(),
            highlighted: true,
            bold: false,
            italic: false,
            underline: false,
            checkbox_like: false,
            option_label: String::new(),
        });
        idx.add_text_node(TextNodeRef {
            paragraph_index: 0,
            run_index: 1,
            text_index: 0,
            text: "B".to_string(),
            highlighted: false,
            bold: false,
            italic: false,
            underline: false,
            checkbox_like: false,
            option_label: String::new(),
        });
        doc.add_part("word/document.xml".to_string(), idx);
        assert_eq!(doc.iter_nodes().count(), 2);
        assert_eq!(doc.iter_highlighted().count(), 1);
    }
}
