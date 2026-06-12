use anyhow::Result;

/// Render a template docx by replacing placeholders with values.
/// Supports: {{key}}, {{?key:text}}, {{*key}}, {{#row}}
pub fn render_document(template_bytes: &[u8], values: &serde_json::Value) -> Result<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(template_bytes))?;

    let mut document_xml = String::new();
    {
        let mut file = archive.by_name("word/document.xml")?;
        std::io::Read::read_to_string(&mut file, &mut document_xml)?;
    }

    // Process row repeats first
    let xml = process_row_repeats(&document_xml, values)?;
    // Process conditional prefixes and party runs
    let xml = process_runs(&xml, values)?;
    // Process simple text replacements
    let xml = process_text(&xml, values)?;

    // Rebuild zip with modified document.xml
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

fn process_row_repeats(xml: &str, _values: &serde_json::Value) -> Result<String> {
    // Find {{*key}} patterns and duplicate table rows
    // TODO: implement with quick-xml
    Ok(xml.to_string())
}

fn process_runs(xml: &str, _values: &serde_json::Value) -> Result<String> {
    // Handle {{?key:text}} conditional prefixes and party field run splitting
    // TODO: implement with quick-xml
    Ok(xml.to_string())
}

fn process_text(xml: &str, values: &serde_json::Value) -> Result<String> {
    use regex::Regex;
    let re = Regex::new(r"\{\{([^}]+)\}\}")?;
    let mut result = xml.to_string();

    for caps in re.captures_iter(xml) {
        let full = &caps[0];
        let key = caps[1].trim();

        // Skip special placeholders
        if key.starts_with('?') || key.starts_with('*') || key.starts_with('#') {
            continue;
        }

        let value = values.get(key).and_then(|v| v.as_str()).unwrap_or("");
        let escaped = crate::docx::utils::xml_escape(value);
        result = result.replace(full, &escaped);
    }

    Ok(result)
}
