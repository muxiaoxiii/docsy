#[tauri::command]
pub fn export_bundle(path: String, options: serde_json::Value) -> Result<String, String> {
    crate::services::bundle::export_bundle(&path, &options).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_bundle(path: String, options: serde_json::Value) -> Result<serde_json::Value, String> {
    crate::services::bundle::import_bundle(&path, &options).map_err(|e| e.to_string())
}
