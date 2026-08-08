use super::run_blocking;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// Small manifest cache keyed by template path + mtime. History suggestions
/// fire on a 350ms debounce; re-parsing the docsytpl package on every keystroke
/// is wasteful, and the manifest rarely changes while editing.
static MANIFEST_CACHE: Mutex<
    Option<(String, std::time::SystemTime, crate::docx_template::TemplateManifest)>,
> = Mutex::new(None);

fn cached_manifest(
    path: &str,
) -> anyhow::Result<crate::docx_template::TemplateManifest> {
    if let Some((cached_path, cached_mtime, manifest)) =
        MANIFEST_CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
    {
        if cached_path == path {
            if let Ok(meta) = std::fs::metadata(path) {
                if let Ok(mtime) = meta.modified() {
                    if mtime == *cached_mtime {
                        return Ok(manifest.clone());
                    }
                }
            }
        }
    }
    let manifest = crate::docx_template::inspect_template_package(path)?;
    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    *MANIFEST_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((path.to_string(), mtime, manifest.clone()));
    Ok(manifest)
}

#[tauri::command]
pub async fn inspect_docx_template(
    path: String,
) -> Result<crate::docx_template::TemplateInspection, String> {
    run_blocking(move || crate::docx_template::engine::inspect_docx(&path)).await
}

#[tauri::command]
pub async fn save_docx_template(
    args: crate::docx_template::SaveTemplateArgs,
) -> Result<crate::docx_template::SaveTemplateResult, String> {
    run_blocking(move || crate::docx_template::engine::save_docx(args)).await
}

#[tauri::command]
pub async fn save_docx_template_to_library(
    mut args: crate::docx_template::SaveTemplateArgs,
) -> Result<crate::docx_template::SaveTemplateResult, String> {
    run_blocking(move || {
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
pub async fn move_template_to_trash(args: crate::docx_template::TemplateDeleteArgs) -> Result<String, String> {
    run_blocking(move || crate::docx_template::move_template_to_trash(args)).await
}

#[tauri::command]
pub async fn restore_template_from_trash(args: crate::docx_template::TemplateRestoreArgs) -> Result<String, String> {
    run_blocking(move || crate::docx_template::restore_template_from_trash(args)).await
}

#[tauri::command]
pub async fn permanently_delete_template(args: crate::docx_template::TemplatePermanentDeleteArgs) -> Result<(), String> {
    run_blocking(move || crate::docx_template::permanently_delete_template(args)).await
}

#[tauri::command]
pub async fn inspect_docsytpl(
    path: String,
) -> Result<crate::docx_template::TemplateManifest, String> {
    run_blocking(move || crate::docx_template::inspect_template_package(&path)).await
}

/// Extract documentRuns + documentText from the docx embedded in a `.docsytpl`
/// package. Used by the frontend to reconstruct fieldRows when editing a
/// library template.
#[tauri::command]
pub async fn inspect_docsytpl_content(
    path: String,
) -> Result<crate::docx_template::DocsytplContent, String> {
    run_blocking(move || {
        let (_manifest, pkg) =
            crate::docx_template::package::read_docsytpl_package(std::path::Path::new(&path))?;
        let (document_runs, _marks, document_text) =
            crate::docx_template::engine::scan_package_to_runs_and_marks(&pkg)?;
        Ok(crate::docx_template::DocsytplContent {
            document_text,
            document_runs,
        })
    })
    .await
}

#[tauri::command]
pub async fn render_docx_template(args: crate::docx_template::RenderTemplateArgs) -> Result<String, String> {
    run_blocking(move || crate::docx_template::engine::render_docx(args, "single")).await
}

#[tauri::command]
pub async fn get_template_history_context(
    template_path: String,
    values: Option<HashMap<String, Value>>,
    full_refresh: Option<bool>,
) -> Result<crate::template_history::TemplateHistoryContext, String> {
    run_blocking(move || {
        let manifest = cached_manifest(&template_path)?;
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

/// Clear the template history database (all recorded fill runs).
#[tauri::command]
pub async fn clear_template_history() -> Result<usize, String> {
    run_blocking(crate::template_history::clear_history).await
}

/// Merge same-name field history from one template into another.
#[tauri::command]
pub async fn merge_template_field_history(
    source_template_id: String,
    target_template_id: String,
) -> Result<usize, String> {
    run_blocking(move || {
        crate::template_history::merge_template_field_history(&source_template_id, &target_template_id)
    })
    .await
}

/// Persist a field's reference (data source) / date format into the template
/// manifest (word content untouched). Used by the fill page's "…" menu.
#[tauri::command]
pub async fn save_template_field_settings(
    template_path: String,
    field_id: String,
    reference: Option<crate::docx_template::TemplateFieldReference>,
    date_format: Option<String>,
) -> Result<(), String> {
    run_blocking(move || {
        crate::docx_template::update_template_field_settings(
            &template_path,
            &field_id,
            reference,
            date_format,
        )
    })
    .await
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
    item_separator: Option<String>,
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
            &item_separator.unwrap_or_else(|| "、".to_string()),
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

#[tauri::command]
pub async fn list_template_database() -> Result<Vec<serde_json::Value>, String> {
    run_blocking(crate::template_history::list_database_entries).await
}

#[tauri::command]
pub async fn delete_template_database_entry(template_path: String) -> Result<(), String> {
    run_blocking(move || crate::template_history::delete_database_entry(&template_path)).await
}

/// Save selected batch rows into the template history database (source "batch").
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
                "batch",
            )?;
            saved += 1;
        }
        Ok(saved)
    })
    .await
}

#[tauri::command]
pub async fn import_template_to_library(source_path: String) -> Result<String, String> {
    run_blocking(move || {
        let source = std::path::Path::new(&source_path);
        let file_name = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("无效文件名"))?;
        let dest = crate::docx_template::template_library_dir().join(file_name);
        if dest.exists() {
            anyhow::bail!(
                "模板库中已存在同名文件：{}，请先删除已有模板或重命名源文件",
                dest.display()
            );
        }
        std::fs::copy(source, &dest)?;
        Ok(dest.display().to_string())
    })
    .await
}

#[tauri::command]
pub async fn export_templates(
    template_paths: Vec<String>,
    output_dir: String,
) -> Result<String, String> {
    run_blocking(move || {
        let output = std::path::Path::new(&output_dir);
        let target_dir = if template_paths.len() > 1 {
            let dir = output.join("Docsy模板");
            std::fs::create_dir_all(&dir)?;
            dir
        } else {
            output.to_path_buf()
        };
        for path in &template_paths {
            let source = std::path::Path::new(path);
            let file_name = source
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("无效文件名"))?;
            std::fs::copy(source, target_dir.join(file_name))?;
        }
        Ok(target_dir.display().to_string())
    })
    .await
}
