pub mod image_paddler;
pub mod pdf;
pub mod settings;
pub mod system;
pub mod video;

pub fn build_handler() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
        // pdf
        pdf::check_qpdf,
        pdf::inspect_pdf,
        pdf::unlock_pdf,
        pdf::merge_pdfs,
        pdf::split_pdf,
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
        // image_paddler
        image_paddler::analyze_image_paddler_folder,
        image_paddler::run_image_paddler,
        // video
        video::check_ffmpeg,
        video::probe_video,
        video::extract_frames,
        video::list_output_frames,
        video::try_brew_install_ffmpeg,
        video::try_brew_install_qpdf,
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
