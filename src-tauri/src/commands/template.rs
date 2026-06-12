use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub builtin: bool,
    pub pinned_to_tab: bool,
    pub field_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub fn list_templates() -> Result<Vec<TemplateInfo>, String> {
    crate::services::template_store::list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_template_meta(template_id: String) -> Result<serde_json::Value, String> {
    crate::services::template_store::get_meta(&template_id).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct SaveTemplateConfigArgs {
    pub template_id: String,
    pub fields: serde_json::Value,
    pub dictionaries: Option<serde_json::Value>,
}

#[tauri::command]
pub fn save_template_config(args: SaveTemplateConfigArgs) -> Result<(), String> {
    crate::services::template_store::save_config(&args.template_id, &args.fields, args.dictionaries.as_ref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_template(template_id: String) -> Result<(), String> {
    crate::services::template_store::delete(&template_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_template(template_id: String, new_name: String) -> Result<(), String> {
    crate::services::template_store::rename(&template_id, &new_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_to_tab(template_id: String) -> Result<(), String> {
    crate::services::template_store::set_pinned(&template_id, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unpin_from_tab(template_id: String) -> Result<(), String> {
    crate::services::template_store::set_pinned(&template_id, false).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct ArchiveInfo {
    pub id: String,
    pub timestamp: String,
    pub label: String,
}

#[tauri::command]
pub fn list_template_archives(template_id: String) -> Result<Vec<ArchiveInfo>, String> {
    crate::services::template_store::list_archives(&template_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_template_archive(template_id: String, archive_id: String) -> Result<(), String> {
    crate::services::template_store::restore_archive(&template_id, &archive_id).map_err(|e| e.to_string())
}
