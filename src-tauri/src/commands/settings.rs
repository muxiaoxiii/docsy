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
pub async fn check_external_tool(tool_name: String) -> Result<crate::external::ToolStatus, String> {
    tauri::async_runtime::spawn_blocking(move || crate::external::check_by_name(&tool_name))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_external_tool(tool_name: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || crate::external::install_by_name(&tool_name))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_external_tool_from_package(
    tool_name: String,
    package_path: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::external::managed::install_tool_from_package(&tool_name, &package_path)
    })
    .await
    .map_err(|e| e.to_string())?
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
