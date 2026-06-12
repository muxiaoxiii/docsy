use serde::Serialize;

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

#[tauri::command]
pub fn create_editor_session(docx_path: String) -> Result<EditorSession, String> {
    crate::services::template_builder::create_session(&docx_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_editor_session(template_id: String) -> Result<EditorSession, String> {
    crate::services::template_builder::load_session(&template_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_template(session: serde_json::Value) -> Result<String, String> {
    crate::services::template_builder::save(&session).map_err(|e| e.to_string())
}
