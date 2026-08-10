use crate::error::DocsyError;

#[tauri::command]
pub async fn analyze_image_paddler_folder(
    folder: String,
    folders: Option<Vec<String>>,
) -> Result<crate::image_paddler::AnalyzeResult, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::image_paddler::analyze(&crate::image_paddler::AnalyzeArgs { folder, folders })
    })
    .await
    .map_err(|e| DocsyError::Unknown {
        message: e.to_string(),
    })?
    .map_err(|e| DocsyError::Unknown {
        message: e.to_string(),
    })
}

#[tauri::command]
pub async fn run_image_paddler(
    args: crate::image_paddler::RunArgs,
) -> Result<crate::image_paddler::RunResult, DocsyError> {
    tauri::async_runtime::spawn_blocking(move || crate::image_paddler::run(&args))
        .await
        .map_err(|e| DocsyError::Unknown {
            message: e.to_string(),
        })?
        .map_err(|e| DocsyError::Unknown {
            message: e.to_string(),
        })
}
