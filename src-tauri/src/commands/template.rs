use super::run_blocking;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn inspect_docx_template(
    path: String,
) -> Result<crate::docx_template::TemplateInspection, String> {
    run_blocking(move || crate::docx_template::engine::inspect_docx(&path)).await
}

#[tauri::command]
pub async fn save_docx_template(
    args: serde_json::Value,
) -> Result<crate::docx_template::SaveTemplateResult, String> {
    run_blocking(move || {
        let args: crate::docx_template::SaveTemplateArgs = serde_json::from_value(args)?;
        crate::docx_template::engine::save_docx(args)
    })
    .await
}

#[tauri::command]
pub async fn save_docx_template_to_library(
    args: serde_json::Value,
) -> Result<crate::docx_template::SaveTemplateResult, String> {
    run_blocking(move || {
        let mut args: crate::docx_template::SaveTemplateArgs = serde_json::from_value(args)?;
        let file_name = crate::docx_template::safe_template_file_name(&args.template_name);
        args.output_path = crate::docx_template::template_library_dir()
            .join(format!("{file_name}.docsytpl"))
            .display()
            .to_string();
        crate::docx_template::engine::save_docx(args)
    })
    .await
}

#[tauri::command]
pub async fn list_template_library(
) -> Result<Vec<crate::docx_template::TemplateLibraryItem>, String> {
    run_blocking(crate::docx_template::list_template_library).await
}

#[tauri::command]
pub async fn list_template_trash() -> Result<Vec<crate::docx_template::TemplateLibraryItem>, String>
{
    run_blocking(crate::docx_template::list_template_trash).await
}

#[tauri::command]
pub async fn move_template_to_trash(args: serde_json::Value) -> Result<String, String> {
    run_blocking(move || {
        let args: crate::docx_template::TemplateDeleteArgs = serde_json::from_value(args)?;
        crate::docx_template::move_template_to_trash(args)
    })
    .await
}

#[tauri::command]
pub async fn restore_template_from_trash(args: serde_json::Value) -> Result<String, String> {
    run_blocking(move || {
        let args: crate::docx_template::TemplateRestoreArgs = serde_json::from_value(args)?;
        crate::docx_template::restore_template_from_trash(args)
    })
    .await
}

#[tauri::command]
pub async fn permanently_delete_template(args: serde_json::Value) -> Result<(), String> {
    run_blocking(move || {
        let args: crate::docx_template::TemplatePermanentDeleteArgs = serde_json::from_value(args)?;
        crate::docx_template::permanently_delete_template(args)
    })
    .await
}

#[tauri::command]
pub async fn inspect_docsytpl(
    path: String,
) -> Result<crate::docx_template::TemplateManifest, String> {
    run_blocking(move || crate::docx_template::inspect_template_package(&path)).await
}

#[tauri::command]
pub async fn render_docx_template(args: serde_json::Value) -> Result<String, String> {
    run_blocking(move || {
        let args: crate::docx_template::RenderTemplateArgs = serde_json::from_value(args)?;
        crate::docx_template::engine::render_docx(args, "single")
    })
    .await
}

#[tauri::command]
pub async fn get_template_history_context(
    template_path: String,
    values: Option<HashMap<String, Value>>,
    full_refresh: Option<bool>,
) -> Result<crate::template_history::TemplateHistoryContext, String> {
    run_blocking(move || {
        let manifest = crate::docx_template::inspect_template_package(&template_path)?;
        crate::template_history::history_context(
            &manifest,
            values.as_ref(),
            full_refresh.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn list_template_generation_runs(
    limit: Option<usize>,
) -> Result<Vec<crate::template_history::TemplateHistoryRun>, String> {
    run_blocking(move || crate::template_history::list_generation_runs(limit.unwrap_or(200))).await
}

#[tauri::command]
pub async fn seed_template_history(
    template_path: String,
    values: HashMap<String, Value>,
) -> Result<(), String> {
    run_blocking(move || {
        let manifest = crate::docx_template::inspect_template_package(&template_path)?;
        crate::template_history::record_template_seed(&template_path, &manifest, &values)
    })
    .await
}

#[tauri::command]
pub async fn export_template_fields_xlsx(
    template_path: String,
    output_path: String,
    default_values: Option<HashMap<String, Value>>,
) -> Result<String, String> {
    run_blocking(move || {
        let manifest = crate::docx_template::inspect_template_package(&template_path)?;
        crate::docx_template::batch::export_fields_xlsx(
            &manifest,
            &default_values.unwrap_or_default(),
            &output_path,
        )
    })
    .await
}

#[tauri::command]
pub async fn validate_batch_import(
    template_path: String,
    xlsx_path: String,
) -> Result<crate::docx_template::batch::BatchValidationResult, String> {
    run_blocking(move || {
        let manifest = crate::docx_template::inspect_template_package(&template_path)?;
        crate::docx_template::batch::validate_imported_xlsx(&manifest, &xlsx_path)
    })
    .await
}

#[tauri::command]
pub async fn batch_render_from_xlsx(
    template_path: String,
    xlsx_path: String,
    output_dir: String,
    name_pattern: Option<String>,
    skip_rows: Option<Vec<usize>>,
    structure_overrides: Option<HashMap<String, crate::docx_template::StructureOverride>>,
) -> Result<crate::docx_template::batch::BatchRenderResult, String> {
    run_blocking(move || {
        let manifest = crate::docx_template::inspect_template_package(&template_path)?;
        crate::docx_template::batch::batch_render(
            &manifest,
            &xlsx_path,
            &output_dir,
            &template_path,
            name_pattern.as_deref().unwrap_or(""),
            &skip_rows.unwrap_or_default(),
            &structure_overrides.unwrap_or_default(),
        )
    })
    .await
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchHistoryRow {
    template_path: String,
    output_path: String,
    values: HashMap<String, serde_json::Value>,
}

/// Save selected batch rows into the template history database (source "manual").
#[tauri::command]
pub async fn save_batch_history_rows(
    rows: Vec<BatchHistoryRow>,
) -> Result<usize, String> {
    run_blocking(move || {
        let mut saved = 0;
        for row in rows {
            let manifest = crate::docx_template::inspect_template_package(&row.template_path)?;
            crate::template_history::record_history_run(
                &row.template_path,
                &manifest,
                &row.output_path,
                &row.values,
                "manual",
            )?;
            saved += 1;
        }
        Ok(saved)
    })
    .await
}
