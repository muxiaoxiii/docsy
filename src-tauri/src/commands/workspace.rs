use crate::error::DocsyError;
use serde_json::Value;

#[tauri::command]
pub async fn get_workspace_preference(scope: String) -> Result<Option<Value>, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::workspace_state::get_preference(&scope)
    })
    .await
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })?
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })
}

#[tauri::command]
pub async fn set_workspace_preference(scope: String, value: Value) -> Result<(), DocsyError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::workspace_state::set_preference(&scope, &value)
    })
    .await
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })?
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })
}

#[tauri::command]
pub async fn save_media_workspace_session(
    args: crate::services::workspace_state::SaveMediaSessionArgs,
) -> Result<crate::services::workspace_state::SavedMediaSession, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::workspace_state::save_media_session(args)
    })
    .await
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })?
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })
}

#[tauri::command]
pub async fn find_media_workspace_session(
    args: crate::services::workspace_state::FindMediaSessionArgs,
) -> Result<crate::services::workspace_state::MediaSessionMatch, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::workspace_state::find_media_session(args)
    })
    .await
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })?
    .map_err(|error| DocsyError::Unknown {
        message: error.to_string(),
    })
}
