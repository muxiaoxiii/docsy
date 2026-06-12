use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

/// Render a template docx by replacing placeholders with values.
/// Supports: {{key}}, {{?key:text}}, {{*key}}, {{#row}}
pub fn render_document(template_bytes: &[u8], values: &serde_json::Value) -> Result<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(template_bytes))?;

    let mut document_xml = String::new();
    {
        let mut file = archive.by_name("word/document.xml")?;
        std::io::Read::read_to_string(&mut file, &mut document_xml)?;
    }

    let xml = process_row_repeats(&document_xml, values)?;
    let xml = process_runs(&xml, values)?;
    let xml = process_text(&xml, values)?;

    let mut output = Vec::new();
    {
        let mut out_zip = zip::ZipWriter::new(std::io::Cursor::new(&mut output));
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            let options = zip::write::FileOptions::default()
                .compression_method(file.compression());

            if name == "word/document.xml" {
                out_zip.start_file(&name, options)?;
                std::io::Write::write_all(&mut out_zip, xml.as_bytes())?;
            } else {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut bytes)?;
                out_zip.start_file(&name, options)?;
                std::io::Write::write_all(&mut out_zip, &bytes)?;
            }
        }
        out_zip.finish()?;
    }

    Ok(output)
}

/// Serialize an XML event into a string buffer.
fn write_event(buf: &mut String, event: &Event<'_>) {
    match event {
        Event::Start(e) => {
            buf.push('<');
            buf.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));
            for a in e.attributes().flatten() {
                buf.push(' ');
                buf.push_str(std::str::from_utf8(a.key.as_ref()).unwrap_or(""));
                buf.push('=');
                buf.push('"');
                buf.push_str(&escape_attr(std::str::from_utf8(&a.value).unwrap_or("")));
                buf.push('"');
            }
            buf.push('>');
        }
        Event::End(e) => {
            buf.push_str("</");
            buf.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));
            buf.push('>');
        }
        Event::Empty(e) => {
            buf.push('<');
            buf.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));
            for a in e.attributes().flatten() {
                buf.push(' ');
                buf.push_str(std::str::from_utf8(a.key.as_ref()).unwrap_or(""));
                buf.push('=');
                buf.push('"');
                buf.push_str(&escape_attr(std::str::from_utf8(&a.value).unwrap_or("")));
                buf.push('"');
            }
            buf.push_str("/>");
        }
        Event::Text(e) => {
            buf.push_str(&e.unescape().unwrap_or_default());
        }
        Event::CData(e) => {
            buf.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
        }
        Event::Comment(e) => {
            buf.push_str("<!--");
            buf.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
            buf.push_str("-->");
        }
        Event::Decl(e) => {
            buf.push_str("<?xml");
            if let Ok(v) = e.version() {
                buf.push_str(&format!(" version=\"{}\"", std::str::from_utf8(&v).unwrap_or("")));
            }
            if let Some(Ok(v)) = e.encoding() {
                buf.push_str(&format!(" encoding=\"{}\"", std::str::from_utf8(&v).unwrap_or("")));
            }
            buf.push_str("?>");
        }
        Event::PI(e) => {
            buf.push_str("<?");
            buf.push_str(std::str::from_utf8(e.content().as_ref()).unwrap_or(""));
            buf.push_str("?>");
        }
        Event::DocType(e) => {
            buf.push_str("<!DOCTYPE ");
            buf.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
            buf.push('>');
        }
        Event::Eof => {}
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Process {{*key}} row repeats: clone <w:tr> for each item in a list field.
fn process_row_repeats(xml: &str, values: &serde_json::Value) -> Result<String> {
    let re = Regex::new(r"\{\{\*(\w+)\}\}")?;
    let idx_re = Regex::new(r"\{\{#row\}\}")?;
    let mut reader = Reader::from_str(xml);
    let mut out = String::with_capacity(xml.len());

    let mut row_buf = String::new();
    let mut in_tr = false;

    loop {
        let event = reader.read_event()?;
        match &event {
            Event::Start(e) if e.name().as_ref() == b"w:tr" => {
                in_tr = true;
                row_buf.clear();
                write_event(&mut row_buf, &event);
            }
            Event::End(e) if e.name().as_ref() == b"w:tr" => {
                write_event(&mut row_buf, &event);
                if in_tr {
                    if let Some(caps) = re.captures(&row_buf) {
                        let key = &caps[1];
                        if let Some(list) = values.get(key).and_then(|v| v.as_array()) {
                            for (i, item) in list.iter().enumerate() {
                                let mut row = row_buf.clone();
                                row = re.replace(&row, item.as_str().unwrap_or("")).to_string();
                                row = idx_re.replace_all(&row, (i + 1).to_string()).to_string();
                                out.push_str(&row);
                            }
                        } else {
                            out.push_str(&row_buf);
                        }
                    } else {
                        out.push_str(&row_buf);
                    }
                    in_tr = false;
                }
            }
            Event::Start(_) if in_tr => {
                write_event(&mut row_buf, &event);
            }
            Event::End(_) if in_tr => {
                write_event(&mut row_buf, &event);
            }
            _ if in_tr => {
                write_event(&mut row_buf, &event);
            }
            _ => {
                write_event(&mut out, &event);
            }
        }
        if matches!(event, Event::Eof) {
            break;
        }
    }
    Ok(out)
}

/// Process {{?key:text}} conditional runs: show text if value non-empty, delete <w:r> otherwise.
fn process_runs(xml: &str, values: &serde_json::Value) -> Result<String> {
    let re = Regex::new(r"\{\{\?(\w+):([^}]+)\}\}")?;
    let mut reader = Reader::from_str(xml);
    let mut out = String::with_capacity(xml.len());

    let mut run_buf = String::new();
    let mut run_text = String::new();
    let mut in_run = false;
    let mut in_text = false;

    loop {
        let event = reader.read_event()?;
        match &event {
            Event::Start(e) if e.name().as_ref() == b"w:r" => {
                in_run = true;
                run_buf.clear();
                run_text.clear();
                write_event(&mut run_buf, &event);
            }
            Event::End(e) if e.name().as_ref() == b"w:r" => {
                write_event(&mut run_buf, &event);
                if in_run {
                    if let Some(caps) = re.captures(&run_text) {
                        let key = &caps[1];
                        let text = &caps[2];
                        let full = caps[0].to_string();
                        let val = values.get(key).and_then(|v| v.as_str()).unwrap_or("");
                        if !val.is_empty() {
                            let new_text = run_text.replace(&full, text);
                            write_run(&mut out, &run_buf, &new_text);
                        }
                    } else {
                        out.push_str(&run_buf);
                    }
                    in_run = false;
                    in_text = false;
                }
            }
            Event::Start(e) if in_run && e.name().as_ref() == b"w:t" => {
                in_text = true;
                write_event(&mut run_buf, &event);
            }
            Event::End(e) if in_run && e.name().as_ref() == b"w:t" => {
                in_text = false;
                write_event(&mut run_buf, &event);
            }
            Event::Text(e) if in_run => {
                let t = e.unescape().unwrap_or_default();
                if in_text {
                    run_text.push_str(&t);
                }
                write_event(&mut run_buf, &event);
            }
            _ if in_run => {
                write_event(&mut run_buf, &event);
            }
            _ => {
                write_event(&mut out, &event);
            }
        }
        if matches!(event, Event::Eof) {
            break;
        }
    }
    Ok(out)
}

/// Write a reconstructed <w:r> with run properties + single <w:t>.
fn write_run(out: &mut String, original_run: &str, new_text: &str) {
    let mut reader = Reader::from_str(original_run);
    let mut in_rpr = false;
    let mut rpr_buf = String::new();

    loop {
        let event = reader.read_event().unwrap_or(Event::Eof);
        match &event {
            Event::Start(e) if e.name().as_ref() == b"w:rPr" => {
                in_rpr = true;
                rpr_buf.clear();
                write_event(&mut rpr_buf, &event);
            }
            Event::End(e) if e.name().as_ref() == b"w:rPr" => {
                write_event(&mut rpr_buf, &event);
                in_rpr = false;
            }
            _ if in_rpr => {
                write_event(&mut rpr_buf, &event);
            }
            Event::Start(e) if e.name().as_ref() == b"w:r" => {
                out.push_str("<w:r>");
                out.push_str(&rpr_buf);
            }
            Event::End(e) if e.name().as_ref() == b"w:r" => {
                out.push_str("<w:t xml:space=\"preserve\">");
                out.push_str(&crate::docx::utils::xml_escape(new_text));
                out.push_str("</w:t></w:r>");
            }
            _ => {}
        }
        if matches!(event, Event::Eof) {
            break;
        }
    }
}

/// Process simple {{key}} text replacements (skip special placeholders).
fn process_text(xml: &str, values: &serde_json::Value) -> Result<String> {
    let re = Regex::new(r"\{\{([^}]+)\}\}")?;
    let mut result = xml.to_string();

    for caps in re.captures_iter(xml) {
        let full = &caps[0];
        let key = caps[1].trim();

        if key.starts_with('?') || key.starts_with('*') || key.starts_with('#') {
            continue;
        }

        let value = values.get(key).and_then(|v| v.as_str()).unwrap_or("");
        let escaped = crate::docx::utils::xml_escape(value);
        result = result.replace(full, &escaped);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};

    #[test]
    fn test_process_text_simple() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body><w:p><w:r><w:t>案件：{{case_number}}</w:t></w:r></w:p></w:body></w:document>"#;
        let vals = json!({"case_number": "2024民初100号"});
        let out = process_text(xml, &vals).unwrap();
        assert!(out.contains("2024民初100号"));
        assert!(!out.contains("{{case_number}}"));
    }

    #[test]
    fn test_process_text_skips_special() {
        let xml = r#"{{?key:text}} {{*list}} {{#row}} {{normal}}"#;
        let vals = json!({"normal": "OK"});
        let out = process_text(xml, &vals).unwrap();
        assert!(out.contains("{{?key:text}}"));
        assert!(out.contains("{{*list}}"));
        assert!(out.contains("{{#row}}"));
        assert!(out.contains("OK"));
    }

    #[test]
    fn test_process_runs_conditional_show() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>原告：{{?plaintiffs:原告 }}</w:t></w:r></w:p></w:body></w:document>"#;
        let vals = json!({"plaintiffs": "张三"});
        let out = process_runs(xml, &vals).unwrap();
        assert!(out.contains("原告 "));
        assert!(!out.contains("{{?plaintiffs"));
    }

    #[test]
    fn test_process_runs_conditional_delete() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body><w:p><w:r><w:t>前缀</w:t></w:r><w:r><w:t>{{?third_parties:第三人 }}</w:t></w:r><w:r><w:t>后缀</w:t></w:r></w:p></w:body></w:document>"#;
        let vals = json!({"third_parties": ""});
        let out = process_runs(xml, &vals).unwrap();
        assert!(out.contains("前缀"));
        assert!(out.contains("后缀"));
        assert!(!out.contains("第三人"));
        assert!(!out.contains("{{?third_parties"));
    }

    #[test]
    fn test_process_row_repeats() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>{{*names}}</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let vals = json!({"names": ["Alice", "Bob", "Charlie"]});
        let out = process_row_repeats(xml, &vals).unwrap();
        assert!(out.contains("Alice"));
        assert!(out.contains("Bob"));
        assert!(out.contains("Charlie"));
        assert!(!out.contains("{{*names}}"));
    }

    #[test]
    fn test_process_row_repeats_with_index() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>{{#row}}. {{*items}}</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let vals = json!({"items": ["甲", "乙"]});
        let out = process_row_repeats(xml, &vals).unwrap();
        assert!(out.contains("1. 甲"));
        assert!(out.contains("2. 乙"));
    }

    #[test]
    fn test_full_pipeline() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = zip::write::FileOptions::default();
            zip.start_file("[Content_Types].xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?>").unwrap();
            zip.start_file("word/document.xml", options).unwrap();
            zip.write_all(br#"<?xml version="1.0"?><w:document><w:body><w:p><w:r><w:t>{{name}}</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
            zip.finish().unwrap();
        }
        let bytes = buf.into_inner();
        let vals = json!({"name": "测试案号"});
        let out = render_document(&bytes, &vals).unwrap();
        let mut out_archive = zip::ZipArchive::new(std::io::Cursor::new(&out)).unwrap();
        let mut doc_xml = String::new();
        out_archive.by_name("word/document.xml").unwrap().read_to_string(&mut doc_xml).unwrap();
        assert!(doc_xml.contains("测试案号"));
        assert!(!doc_xml.contains("{{name}}"));
    }
}
