use anyhow::Result;
use serde::{Deserialize, Serialize};

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

    // TODO: pack .docsytpl and save
    Ok(template_id)
}

fn extract_plain_text(docx_bytes: &[u8]) -> Result<String> {
    let text = mammoth::extract_raw_text(docx_bytes).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(text)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
