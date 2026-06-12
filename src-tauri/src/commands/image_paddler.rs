#[tauri::command]
pub fn analyze_image_paddler_folder(folder: String) -> Result<serde_json::Value, String> {
    crate::services::doc_gen::placeholder_analyze(&folder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn run_image_paddler(args: serde_json::Value) -> Result<serde_json::Value, String> {
    crate::services::doc_gen::placeholder_run(&args).map_err(|e| e.to_string())
}
