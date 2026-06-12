use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SaveRecordArgs {
    pub template_id: String,
    pub values: serde_json::Value,
    pub output_path: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerationRecord {
    pub id: String,
    pub template_id: String,
    pub timestamp: String,
    pub values: serde_json::Value,
    pub output_path: Option<String>,
    pub label: String,
}

#[tauri::command]
pub fn save_generation_record(args: SaveRecordArgs) -> Result<GenerationRecord, String> {
    crate::services::history::save_record(args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_generation_records(template_id: String) -> Result<Vec<GenerationRecord>, String> {
    crate::services::history::list_records(&template_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_generation_record(template_id: String, record_id: String) -> Result<GenerationRecord, String> {
    crate::services::history::read_record(&template_id, &record_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_generation_record(template_id: String, record_id: String) -> Result<(), String> {
    crate::services::history::delete_record(&template_id, &record_id).map_err(|e| e.to_string())
}
