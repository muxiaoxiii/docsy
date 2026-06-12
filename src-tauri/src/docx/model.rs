use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxDocument {
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub index: usize,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub index: usize,
    pub text: String,
    pub path: String,
}

/// Parse a docx file into a document model using quick-xml event parser.
pub fn parse(docx_bytes: &[u8]) -> Result<DocxDocument> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(docx_bytes))?;
    let mut document_xml = String::new();
    {
        let mut file = archive.by_name("word/document.xml")?;
        std::io::Read::read_to_string(&mut file, &mut document_xml)?;
    }
    parse_xml(&document_xml)
}

fn parse_xml(xml: &str) -> Result<DocxDocument> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut paragraphs = Vec::new();
    let mut current_runs = Vec::new();
    let mut current_text = String::new();
    let mut in_paragraph = false;
    let mut in_run = false;
    let mut in_text = false;
    let mut run_index = 0;
    let mut para_index = 0;
    let mut depth = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                match e.name().as_ref() {
                    b"w:p" => {
                        in_paragraph = true;
                        current_runs = Vec::new();
                        run_index = 0;
                    }
                    b"w:r" => {
                        if in_paragraph {
                            in_run = true;
                            current_text = String::new();
                        }
                    }
                    b"w:t" => {
                        if in_run {
                            in_text = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    current_text.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                match e.name().as_ref() {
                    b"w:t" => {
                        in_text = false;
                    }
                    b"w:r" => {
                        if in_run && !current_text.is_empty() {
                            current_runs.push(Run {
                                index: run_index,
                                text: current_text.clone(),
                                path: format!("p[{}]/r[{}]", para_index, run_index),
                            });
                            run_index += 1;
                        }
                        in_run = false;
                    }
                    b"w:p" => {
                        if in_paragraph {
                            paragraphs.push(Paragraph {
                                index: para_index,
                                runs: current_runs.clone(),
                            });
                            para_index += 1;
                        }
                        in_paragraph = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        if depth < 0 {
            break;
        }
    }

    Ok(DocxDocument { paragraphs })
}
