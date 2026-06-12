use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

#[derive(Debug, Serialize)]
pub struct EditorSession {
    pub template_id: Option<String>,
    pub manifest: serde_json::Value,
    pub source_docx_base64: String,
    pub plain_text: String,
    pub marks: serde_json::Value,
    pub fields: serde_json::Value,
    pub dictionaries: Option<serde_json::Value>,
}

pub fn create_session(docx_path: &str) -> Result<EditorSession> {
    let bytes = std::fs::read(docx_path)?;
    let base64 = base64_encode(&bytes);
    let plain_text = extract_plain_text(&bytes)?;

    Ok(EditorSession {
        template_id: None,
        manifest: serde_json::json!({
            "name": "",
            "type": "custom",
            "version": "1.0.0",
        }),
        source_docx_base64: base64,
        plain_text,
        marks: serde_json::json!([]),
        fields: serde_json::json!([]),
        dictionaries: None,
    })
}

pub fn load_session(template_id: &str) -> Result<EditorSession> {
    let tpl = crate::services::template_store::resolve(template_id)?;

    let bytes = if tpl.docx_path.exists() {
        std::fs::read(&tpl.docx_path)?
    } else {
        vec![]
    };
    let base64 = base64_encode(&bytes);
    let plain_text = extract_plain_text(&bytes)?;

    Ok(EditorSession {
        template_id: Some(template_id.to_string()),
        manifest: serde_json::json!({
            "name": tpl.name,
            "type": "custom",
            "version": "1.0.0",
        }),
        source_docx_base64: base64,
        plain_text,
        marks: serde_json::json!([]),
        fields: tpl.fields,
        dictionaries: tpl.dictionaries,
    })
}

pub fn save(session_json: &serde_json::Value) -> Result<String> {
    let template_id = session_json
        .get("template_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let manifest = session_json
        .get("manifest")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let source_docx_base64 = session_json
        .get("source_docx_base64")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let marks = session_json
        .get("marks")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    let fields = session_json
        .get("fields")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    let dictionaries = session_json.get("dictionaries").cloned();

    use base64::Engine;
    let docx_bytes = base64::engine::general_purpose::STANDARD.decode(source_docx_base64)?;

    let template_docx = write_placeholders(&docx_bytes, &marks)?;

    let now = chrono::Utc::now().to_rfc3339();
    let name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tpl_type = manifest
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("custom")
        .to_string();
    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    let manifest_json = serde_json::json!({
        "id": template_id,
        "name": name,
        "type": tpl_type,
        "version": version,
        "created_at": now,
    });

    let fields_json = serde_json::json!({ "fields": fields });

    let output_dir = crate::services::template_store::user_templates_dir();
    std::fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join(format!("{}.docsytpl", template_id));

    let mut contents: Vec<(&str, Vec<u8>)> = vec![
        ("manifest.json", serde_json::to_vec_pretty(&manifest_json)?),
        ("template.docx", template_docx),
        ("fields.json", serde_json::to_vec_pretty(&fields_json)?),
        ("builder_state.json", serde_json::to_vec_pretty(session_json)?),
    ];

    if let Some(ref dicts) = dictionaries {
        contents.push(("dictionaries.json", serde_json::to_vec_pretty(dicts)?));
    }

    pack_docsytpl(&output_path, &contents)?;

    Ok(template_id)
}

fn pack_docsytpl(path: &std::path::Path, contents: &[(&str, Vec<u8>)]) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, data) in contents {
        zip.start_file(*name, options)?;
        zip.write_all(data)?;
    }
    zip.finish()?;
    Ok(())
}

fn write_placeholders(docx_bytes: &[u8], marks: &serde_json::Value) -> Result<Vec<u8>> {
    let marks_array = match marks.as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => return Ok(docx_bytes.to_vec()),
    };

    let doc = crate::docx::model::parse(docx_bytes)?;

    struct CharLoc {
        para_idx: usize,
        run_idx: usize,
        char_pos: usize,
    }
    let mut char_locs: Vec<CharLoc> = Vec::new();

    for (pi, para) in doc.paragraphs.iter().enumerate() {
        for (_ri, run) in para.runs.iter().enumerate() {
            for ci in 0..run.text.chars().count() {
                char_locs.push(CharLoc {
                    para_idx: pi,
                    run_idx: _ri,
                    char_pos: ci,
                });
            }
        }
        if pi < doc.paragraphs.len() - 1 {
            char_locs.push(CharLoc {
                para_idx: pi,
                run_idx: usize::MAX,
                char_pos: 0,
            });
        }
    }

    let mut run_replacements: HashMap<(usize, usize), Vec<(String, String)>> = HashMap::new();

    for mark in marks_array {
        let key = match mark.get("key").and_then(|v| v.as_str()) {
            Some(k) if !k.is_empty() => k,
            _ => continue,
        };
        let start = mark.get("start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let end = mark.get("end").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        if start >= end || start >= char_locs.len() {
            continue;
        }
        let end = end.min(char_locs.len());

        let placeholder = format!("{{{{{}}}}}", key);

        let mut affected_runs: Vec<(usize, usize)> = Vec::new();
        for loc in &char_locs[start..end] {
            if loc.run_idx == usize::MAX {
                continue;
            }
            let pair = (loc.para_idx, loc.run_idx);
            if affected_runs.last() != Some(&pair) {
                affected_runs.push(pair);
            }
        }

        if affected_runs.is_empty() {
            continue;
        }

        if affected_runs.len() == 1 {
            let (pi, ri) = affected_runs[0];
            let run = &doc.paragraphs[pi].runs[ri];
            let start_char = char_locs[start].char_pos;
            let end_char = char_locs[end - 1].char_pos;

            let chars: Vec<char> = run.text.chars().collect();
            let before: String = chars[..start_char].iter().collect();
            let after: String = chars[end_char + 1..].iter().collect();
            let replacement = format!("{}{}{}", before, placeholder, after);

            run_replacements
                .entry((pi, ri))
                .or_default()
                .push((run.text.clone(), replacement));
        } else {
            for (idx, &(pi, ri)) in affected_runs.iter().enumerate() {
                let run = &doc.paragraphs[pi].runs[ri];
                let chars: Vec<char> = run.text.chars().collect();

                if idx == 0 {
                    let start_char = char_locs[start].char_pos;
                    let before: String = chars[..start_char].iter().collect();
                    let replacement = format!("{}{}", before, placeholder);
                    run_replacements
                        .entry((pi, ri))
                        .or_default()
                        .push((run.text.clone(), replacement));
                } else if idx == affected_runs.len() - 1 {
                    let end_char = char_locs[end - 1].char_pos;
                    let after: String = chars[end_char + 1..].iter().collect();
                    run_replacements
                        .entry((pi, ri))
                        .or_default()
                        .push((run.text.clone(), after));
                } else {
                    run_replacements
                        .entry((pi, ri))
                        .or_default()
                        .push((run.text.clone(), String::new()));
                }
            }
        }
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(docx_bytes))?;
    let mut document_xml = String::new();
    {
        let mut file = archive.by_name("word/document.xml")?;
        file.read_to_string(&mut document_xml)?;
    }

    let processed_xml = apply_run_replacements(&document_xml, &run_replacements)?;

    let mut output = Vec::new();
    {
        let mut out_zip = zip::ZipWriter::new(Cursor::new(&mut output));
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            let options = zip::write::FileOptions::default()
                .compression_method(file.compression());

            if name == "word/document.xml" {
                out_zip.start_file(&name, options)?;
                out_zip.write_all(processed_xml.as_bytes())?;
            } else {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                out_zip.start_file(&name, options)?;
                out_zip.write_all(&bytes)?;
            }
        }
        out_zip.finish()?;
    }

    Ok(output)
}

fn apply_run_replacements(
    xml: &str,
    replacements: &HashMap<(usize, usize), Vec<(String, String)>>,
) -> Result<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut out = String::with_capacity(xml.len());
    let mut para_idx = 0usize;
    let mut run_idx = 0usize;
    let mut in_paragraph = false;
    let mut in_run = false;
    let mut in_t = false;
    let mut t_content = String::new();

    loop {
        let event = reader.read_event()?;
        match &event {
            Event::Start(e) => match e.name().as_ref() {
                b"w:p" => {
                    in_paragraph = true;
                    run_idx = 0;
                    write_event(&mut out, &event);
                }
                b"w:r" => {
                    if in_paragraph {
                        in_run = true;
                    }
                    write_event(&mut out, &event);
                }
                b"w:t" => {
                    if in_run {
                        in_t = true;
                        t_content.clear();
                    }
                    write_event(&mut out, &event);
                }
                _ => {
                    write_event(&mut out, &event);
                }
            },
            Event::End(e) => match e.name().as_ref() {
                b"w:t" => {
                    if in_t {
                        in_t = false;
                        let key = (para_idx, run_idx);
                        let mut text = t_content.clone();
                        if let Some(repls) = replacements.get(&key) {
                            for (from, to) in repls {
                                text = text.replace(from, to);
                            }
                        }
                        out.push_str(&crate::docx::utils::xml_escape(&text));
                        out.push_str("</w:t>");
                    } else {
                        write_event(&mut out, &event);
                    }
                }
                b"w:r" => {
                    if in_run {
                        run_idx += 1;
                        in_run = false;
                    }
                    write_event(&mut out, &event);
                }
                b"w:p" => {
                    if in_paragraph {
                        para_idx += 1;
                        in_paragraph = false;
                    }
                    write_event(&mut out, &event);
                }
                _ => {
                    write_event(&mut out, &event);
                }
            },
            Event::Text(e) => {
                if in_t {
                    t_content.push_str(&e.unescape().unwrap_or_default());
                } else {
                    write_event(&mut out, &event);
                }
            }
            Event::CData(e) => {
                if in_t {
                    t_content.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
                } else {
                    write_event(&mut out, &event);
                }
            }
            Event::Eof => break,
            _ => {
                write_event(&mut out, &event);
            }
        }
    }

    Ok(out)
}

fn write_event(buf: &mut String, event: &quick_xml::events::Event<'_>) {
    use quick_xml::events::Event;
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
                buf.push_str(&format!(
                    " version=\"{}\"",
                    std::str::from_utf8(&v).unwrap_or("")
                ));
            }
            if let Some(Ok(v)) = e.encoding() {
                buf.push_str(&format!(
                    " encoding=\"{}\"",
                    std::str::from_utf8(&v).unwrap_or("")
                ));
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

fn extract_plain_text(docx_bytes: &[u8]) -> Result<String> {
    if docx_bytes.is_empty() {
        return Ok(String::new());
    }
    let doc = crate::docx::model::parse(docx_bytes)?;
    let text: String = doc
        .paragraphs
        .iter()
        .map(|p| {
            p.runs
                .iter()
                .map(|r| r.text.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(text)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
