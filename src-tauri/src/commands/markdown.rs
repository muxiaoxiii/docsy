use crate::commands::{run_blocking, run_managed};
use serde::Serialize;

/// convert_pdf_text_layer 的返回结果（snake_case JSON，前端按此对接）。
#[derive(Debug, Serialize)]
pub struct PdfTextLayerResult {
    pub output_path: String,
    pub pages_with_text: usize,
    pub empty_pages: Vec<u32>,
    pub input_size: u64,
    pub output_size: u64,
    /// 空文本层页、扫描件提示等用户提示。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// PDF 文本层 → Markdown，支持页段（startPage/endPage，1-based 闭区间）与取消。
#[tauri::command]
pub async fn convert_pdf_text_layer(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    input: String,
    output_dir: Option<String>,
    start_page: Option<u32>,
    end_page: Option<u32>,
) -> Result<PdfTextLayerResult, String> {
    let result = run_managed(&manager, "convert_pdf_text_layer", None, move |token| {
        if token.is_cancelled() {
            anyhow::bail!("操作已取消");
        }
        crate::markdown::pdf_to_md::convert_pdf_text_layer(
            &input,
            output_dir.as_deref(),
            start_page,
            end_page,
        )
    })
    .await?;
    Ok(PdfTextLayerResult {
        output_path: result.output_path,
        pages_with_text: result.pages_with_text,
        empty_pages: result.empty_pages,
        input_size: result.input_size,
        output_size: result.output_size,
        warning: result.warning,
    })
}

/// convert_markdown 的返回结果（snake_case JSON，前端按此对接）。
#[derive(Debug, Serialize)]
pub struct ConvertMarkdownResult {
    pub output_path: String,
    pub direction: crate::markdown::Direction,
    pub source_format: String,
    pub output_format: String,
    pub input_size: u64,
    pub output_size: u64,
    /// 旧版 Office 或复杂版式转换时的保真度提示。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// convert_markdown_text 的返回结果（snake_case JSON，前端按此对接）。
#[derive(Debug, Serialize)]
pub struct ConvertMarkdownTextResult {
    pub output_path: String,
    pub format: String,
    pub input_size: u64,
    pub output_size: u64,
}

/// 粘贴 Markdown 文本直接转换为 docx / html。
/// Tauri 参数 camelCase 约定：JS 端传 outputDir / fileStem。
#[tauri::command]
pub async fn convert_markdown_text(
    text: String,
    format: String,
    output_dir: Option<String>,
    file_stem: Option<String>,
    docx_style: Option<String>,
) -> Result<ConvertMarkdownTextResult, String> {
    let result = run_blocking(move || {
        crate::markdown::convert_text(
            &text,
            &format,
            output_dir.as_deref(),
            file_stem.as_deref(),
            docx_style.as_deref(),
        )
    })
    .await?;
    Ok(ConvertMarkdownTextResult {
        output_path: result.output_path,
        format: result.format,
        input_size: result.input_size,
        output_size: result.output_size,
    })
}

#[tauri::command]
pub async fn convert_markdown(
    input: String,
    output_dir: Option<String>,
    doc_engine: Option<String>,
    output_format: Option<String>,
    docx_style: Option<String>,
) -> Result<ConvertMarkdownResult, String> {
    let result = run_blocking(move || {
        crate::markdown::convert(
            &input,
            output_dir.as_deref(),
            doc_engine.as_deref(),
            output_format.as_deref(),
            docx_style.as_deref(),
        )
    })
    .await?;
    Ok(ConvertMarkdownResult {
        output_path: result.output_path,
        direction: result.direction,
        source_format: result.source_format,
        output_format: result.output_format,
        input_size: result.input_size,
        output_size: result.output_size,
        warning: result.warning,
    })
}
