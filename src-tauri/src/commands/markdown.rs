use crate::commands::run_blocking;
use serde::Serialize;

/// convert_markdown 的返回结果（snake_case JSON，前端按此对接）。
#[derive(Debug, Serialize)]
pub struct ConvertMarkdownResult {
    pub output_path: String,
    pub direction: crate::markdown::Direction,
    pub input_size: u64,
    pub output_size: u64,
}

#[tauri::command]
pub async fn convert_markdown(
    input: String,
    output_dir: Option<String>,
) -> Result<ConvertMarkdownResult, String> {
    let result = run_blocking(move || crate::markdown::convert(&input, output_dir.as_deref())).await?;
    Ok(ConvertMarkdownResult {
        output_path: result.output_path,
        direction: result.direction,
        input_size: result.input_size,
        output_size: result.output_size,
    })
}
