pub mod image_paddler;
pub mod markdown;
pub mod pdf;
pub mod settings;
pub mod system;
pub mod template;
pub mod video;
pub mod workspace;

fn anyhow_to_json_string(err: anyhow::Error) -> String {
    let docsy_err: crate::error::DocsyError = err.into();
    serde_json::to_string(&docsy_err).unwrap_or_else(|_| docsy_err.to_string())
}

pub async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| e.to_string())?
        .map_err(anyhow_to_json_string)
}

/// 通用操作管理封装。替代 run_blocking，自动注册/注销操作。
///
/// 与 run_blocking 并存：
/// - 快速命令（inspect、get_page_count 等）继续用 run_blocking
/// - 长时间命令（批量渲染、PDF 处理等）用 run_managed 获得取消能力
///
/// # 用法（示意）
/// ```ignore
/// run_managed(&manager, "my_long_operation", Some("op-1".into()), |token| {
///     for item in items {
///         if token.is_cancelled() {
///             return Err(anyhow::anyhow!("操作已取消"));
///         }
///         process(item)?;
///     }
///     Ok(result)
/// }).await
/// ```
pub async fn run_managed<T, F>(
    manager: &crate::operations::OperationManager,
    command: &str,
    operation_id: Option<String>,
    task: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(tokio_util::sync::CancellationToken) -> anyhow::Result<T> + Send + 'static,
{
    let op_id = operation_id.unwrap_or_else(|| format!("{}:auto", command));
    let token = manager.begin(&op_id, command);

    // 不在 spawn_blocking 上使用 ?，确保 finish() 一定被调用
    let join_result = tauri::async_runtime::spawn_blocking(move || task(token)).await;
    let result = match join_result {
        Ok(inner) => inner.map_err(anyhow_to_json_string),
        Err(join_err) => Err(join_err.to_string()),
    };

    // 无论成功失败都必须 finish，否则操作永远留在 map 里
    manager.finish(&op_id, result.is_err());
    result
}

pub fn build_handler() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        // pdf
        pdf::check_qpdf,
        pdf::inspect_pdf,
        pdf::unlock_pdf,
        pdf::merge_pdfs,
        pdf::extract_pdf_pages,
        pdf::compress_pdf,
        pdf::optimize_pdf_lossless,
        pdf::split_merged_evidence_pdf,
        pdf::scan_evidence_folder,
        pdf::build_evidence_group_pdfs,
        pdf::merge_evidence_pdfs,
        pdf::overlay_pdf_text,
        pdf::batch_overlay_pdf_text,
        pdf::apply_evidence_pdf_rules,
        pdf::preview_pdf_header_footer,
        pdf::detect_pdf_header_footer,
        pdf::inspect_merged_evidence_pdf,
        pdf::delete_pdf_annotations,
        pdf::delete_pdf_header_footer_artifacts,
        pdf::render_pdf_preview,
        pdf::get_pdf_page_count,
        pdf::detect_anti_copy,
        pdf::apply_anti_copy,
        pdf::remove_anti_copy,
        pdf::has_pdf_bookmarks,
        pdf::remove_pdf_bookmarks,
        // image_paddler
        image_paddler::analyze_image_paddler_folder,
        image_paddler::run_image_paddler,
        // video
        video::check_ffmpeg,
        video::probe_video,
        video::extract_frames,
        video::list_output_frames,
        video::analyze_frame_selection,
        // unified workspace persistence
        workspace::get_workspace_preference,
        workspace::set_workspace_preference,
        workspace::save_media_workspace_session,
        workspace::find_media_workspace_session,
        // markdown
        markdown::convert_markdown,
        markdown::convert_markdown_text,
        markdown::convert_pdf_text_layer,
        // settings
        settings::get_app_settings,
        settings::set_app_settings,
        settings::check_external_tool,
        settings::install_external_tool,
        settings::install_external_tool_from_package,
        settings::get_managed_tools_dir,
        settings::open_managed_tools_dir,
        settings::remove_managed_tool,
        // system
        system::open_path,
        system::open_external_url,
        system::write_frontend_log,
        system::get_log_file_path,
        system::read_image_data_url,
        system::open_log_file,
        system::open_log_dir,
        system::compose_log_email,
        system::export_diagnostic_report,
        system::get_diagnostic_info,
        system::list_system_fonts,
        system::respond_conversion_timeout,
        system::cancel_operation,
        system::list_active_operations,
        system::confirm_app_close,
        // template
        template::inspect_docx_template,
        template::save_docx_template,
        template::save_docx_template_to_library,
        template::list_template_library,
        template::list_template_trash,
        template::move_template_to_trash,
        template::restore_template_from_trash,
        template::permanently_delete_template,
        template::inspect_docsytpl,
        template::inspect_docsytpl_content,
        template::preview_docx_template,
        template::render_docx_template,
        template::get_template_history_context,
        template::list_template_generation_runs,
        template::seed_template_history,
        template::clear_template_history,
        template::merge_template_field_history,
        template::save_template_field_settings,
        template::save_batch_history_rows,
        template::list_template_database,
        template::delete_template_database_entry,
        template::export_template_fields_xlsx,
        template::validate_batch_import,
        template::batch_render_from_xlsx,
        template::import_template_to_library,
        template::export_templates,
    ]
}
