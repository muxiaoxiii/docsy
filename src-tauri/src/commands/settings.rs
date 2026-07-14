#[tauri::command]
pub fn get_app_settings() -> Result<crate::services::history::AppSettings, String> {
    crate::services::history::get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_app_settings(settings: crate::services::history::AppSettings) -> Result<(), String> {
    crate::services::history::save_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_module_registry() -> Result<Vec<serde_json::Value>, String> {
    Ok(crate::services::module_registry::all_descriptors())
}

#[tauri::command]
pub fn check_external_tool(tool_name: String) -> crate::external::ToolStatus {
    crate::external::check_by_name(&tool_name)
}

#[tauri::command]
pub fn install_external_tool(tool_name: String) -> Result<String, String> {
    crate::external::install_by_name(&tool_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_external_tool_from_package(
    tool_name: String,
    package_path: String,
) -> Result<String, String> {
    crate::external::managed::install_tool_from_package(&tool_name, &package_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_managed_tools_dir() -> String {
    crate::external::managed::tools_root().display().to_string()
}

#[tauri::command]
pub fn open_managed_tools_dir() -> Result<(), String> {
    crate::external::managed::open_tools_root().map_err(|e| e.to_string())
}
