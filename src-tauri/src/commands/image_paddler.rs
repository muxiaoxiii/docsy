#[tauri::command]
pub fn analyze_image_paddler_folder(
    folder: String,
    folders: Option<Vec<String>>,
) -> Result<crate::image_paddler::AnalyzeResult, String> {
    crate::image_paddler::analyze(&crate::image_paddler::AnalyzeArgs { folder, folders })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn run_image_paddler(
    args: crate::image_paddler::RunArgs,
) -> Result<crate::image_paddler::RunResult, String> {
    crate::image_paddler::run(&args).map_err(|e| e.to_string())
}
