mod doc_gen;
mod dictionary;
mod image_paddler;
mod pdf;
mod record;
mod settings;
mod system;
mod template;
mod template_editor;
mod video;

pub fn build_handler() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        // doc_gen
        doc_gen::generate_document,
        doc_gen::preview_document,
        // template
        template::list_templates,
        template::get_template_meta,
        template::save_template_config,
        template::delete_template,
        template::rename_template,
        template::pin_to_tab,
        template::unpin_from_tab,
        template::list_template_archives,
        template::restore_template_archive,
        // template_editor
        template_editor::create_editor_session,
        template_editor::load_editor_session,
        template_editor::save_template,
        // dictionary
        dictionary::query_dictionary,
        dictionary::recommend_values,
        dictionary::export_dictionary_xlsx,
        dictionary::import_dictionary_xlsx,
        // pdf
        pdf::check_qpdf,
        pdf::inspect_pdf,
        pdf::unlock_pdf,
        pdf::merge_pdfs,
        pdf::split_pdf,
        pdf::scan_evidence_folder,
        pdf::build_evidence_group_pdfs,
        pdf::merge_evidence_pdfs,
        pdf::overlay_pdf_text,
        pdf::batch_overlay_pdf_text,
        pdf::get_pdf_page_count,
        // image_paddler
        image_paddler::analyze_image_paddler_folder,
        image_paddler::run_image_paddler,
        // video
        video::check_ffmpeg,
        video::probe_video,
        video::extract_frames,
        video::try_brew_install_ffmpeg,
        video::try_brew_install_qpdf,
        // record
        record::save_generation_record,
        record::list_generation_records,
        record::read_generation_record,
        record::delete_generation_record,
        // settings
        settings::get_app_settings,
        settings::set_app_settings,
        settings::get_module_registry,
        settings::check_external_tool,
        settings::install_external_tool,
        // system
        system::open_path,
        system::write_frontend_log,
        system::get_log_file_path,
        system::open_log_file,
        system::open_log_dir,
        system::get_diagnostic_info,
        system::list_system_fonts,
    ]
}
