use crate::error::DocsyError;

#[tauri::command]
pub fn get_app_settings() -> Result<crate::services::history::AppSettings, DocsyError> {
    crate::services::history::get_settings().map_err(|e| DocsyError::Unknown { message: e.to_string() })
}

#[tauri::command]
pub fn set_app_settings(settings: crate::services::history::AppSettings) -> Result<(), DocsyError> {
    crate::services::history::save_settings(&settings).map_err(|e| DocsyError::Unknown { message: e.to_string() })
}

#[tauri::command]
pub async fn check_external_tool(tool_name: String) -> Result<crate::external::ToolStatus, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || crate::external::check_by_name(&tool_name))
        .await
        .map_err(|e| DocsyError::Unknown { message: e.to_string() })
}

#[tauri::command]
pub async fn install_external_tool(tool_name: String) -> Result<String, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || crate::external::install_by_name(&tool_name))
        .await
        .map_err(|e| DocsyError::Unknown { message: e.to_string() })?
        .map_err(|e| DocsyError::Unknown { message: e.to_string() })
}

#[tauri::command]
pub async fn install_external_tool_from_package(
    tool_name: String,
    package_path: String,
) -> Result<String, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<String> {
        let installed =
            crate::external::managed::install_tool_from_package(&tool_name, &package_path)?;
        crate::external::validate_tool(&tool_name)?;
        Ok(installed)
    })
    .await
    .map_err(|e| DocsyError::Unknown { message: e.to_string() })?
    .map_err(|e| DocsyError::Unknown { message: e.to_string() })
}

#[tauri::command]
pub fn get_managed_tools_dir() -> String {
    crate::external::managed::tools_root().display().to_string()
}

#[tauri::command]
pub fn open_managed_tools_dir() -> Result<(), DocsyError> {
    crate::external::managed::open_tools_root().map_err(|e| DocsyError::Unknown { message: e.to_string() })
}

#[tauri::command]
pub async fn remove_managed_tool(tool_name: String) -> Result<String, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || -> Result<String, DocsyError> {
        crate::external::managed::remove_managed_tool(&tool_name)
            .map_err(|e| DocsyError::Unknown { message: e.to_string() })?;
        Ok(format!("已清除 {} 的托管安装", tool_name))
    })
    .await
    .map_err(|e| DocsyError::Unknown { message: e.to_string() })?
}
