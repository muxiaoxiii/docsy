#[tauri::command]
pub async fn analyze_image_paddler_folder(
    folder: String,
    folders: Option<Vec<String>>,
) -> Result<crate::image_paddler::AnalyzeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::image_paddler::analyze(&crate::image_paddler::AnalyzeArgs { folder, folders })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_image_paddler(
    args: crate::image_paddler::RunArgs,
) -> Result<crate::image_paddler::RunResult, String> {
    tauri::async_runtime::spawn_blocking(move || crate::image_paddler::run(&args))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
