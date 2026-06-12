use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub history_max: usize,
    pub menu_visibility: std::collections::HashMap<String, bool>,
    pub libreoffice_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            history_max: 50,
            menu_visibility: std::collections::HashMap::new(),
            libreoffice_path: None,
        }
    }
}

#[tauri::command]
pub fn get_app_settings() -> Result<AppSettings, String> {
    crate::services::history::get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_app_settings(settings: AppSettings) -> Result<(), String> {
    crate::services::history::save_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_module_registry() -> Result<Vec<serde_json::Value>, String> {
    Ok(crate::services::module_registry::all_descriptors())
}

#[derive(Debug, Serialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub install_hint: String,
}

#[tauri::command]
pub fn check_external_tool(tool_name: String) -> ToolStatus {
    crate::external::check_by_name(&tool_name)
}

#[tauri::command]
pub fn install_external_tool(tool_name: String) -> Result<String, String> {
    crate::external::install_by_name(&tool_name).map_err(|e| e.to_string())
}
