mod app_log;
mod ffmpeg;
mod history;
mod image_paddler;
mod pdf;

use std::path::PathBuf;
use std::process::Command;

use pdf::qpdf;
use serde_json::Value;

#[tauri::command]
fn check_qpdf() -> qpdf::QpdfStatus {
    qpdf::check()
}

#[tauri::command]
fn inspect_pdf(input: String) -> qpdf::InspectResult {
    qpdf::inspect(&PathBuf::from(input))
}

#[tauri::command]
fn unlock_pdf(input: String) -> Result<qpdf::UnlockResult, String> {
    app_log::info(
        "pdf",
        "unlock_pdf.start",
        serde_json::json!({ "input": input }),
    );
    match qpdf::unlock(&PathBuf::from(&input)).map_err(|e| e.to_string()) {
        Ok(result) => {
            app_log::info(
                "pdf",
                "unlock_pdf.success",
                serde_json::json!({ "input": input }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "pdf",
                "unlock_pdf.failed",
                serde_json::json!({ "input": input, "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn scan_evidence_folder(root: String) -> Result<pdf::evidence::EvidenceScan, String> {
    app_log::info(
        "pdf.evidence",
        "scan.start",
        serde_json::json!({ "root": root }),
    );
    match pdf::evidence::scan(&PathBuf::from(&root)) {
        Ok(result) => {
            app_log::info(
                "pdf.evidence",
                "scan.success",
                serde_json::json!({
                    "root": root,
                    "groups": result.groups.len(),
                    "rootPdfs": result.root_pdfs.len()
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "pdf.evidence",
                "scan.failed",
                serde_json::json!({ "root": root, "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn build_evidence_group_pdfs(
    args: pdf::evidence::BuildGroupPdfsArgs,
) -> Result<pdf::evidence::BuildGroupPdfsResult, String> {
    app_log::info(
        "pdf.evidence",
        "build_groups.start",
        serde_json::json!({
            "root": &args.root,
            "selected": args.selected_paths.len(),
            "outputDir": &args.output_dir
        }),
    );
    match pdf::evidence::build_group_pdfs(args) {
        Ok(result) => {
            app_log::info(
                "pdf.evidence",
                "build_groups.success",
                serde_json::json!({
                    "outputDir": &result.output_dir,
                    "outputs": result.outputs.len(),
                    "failed": result.failed.len()
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "pdf.evidence",
                "build_groups.failed",
                serde_json::json!({ "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn merge_evidence_pdfs(args: pdf::evidence::MergeEvidencePdfsArgs) -> Result<qpdf::MergeResult, String> {
    app_log::info(
        "pdf.evidence",
        "merge.start",
        serde_json::json!({
            "items": args.items.len(),
            "outputPath": &args.output_path
        }),
    );
    match pdf::evidence::merge_evidence_pdfs(args) {
        Ok(result) => {
            app_log::info(
                "pdf.evidence",
                "merge.success",
                serde_json::json!({
                    "outputPath": &result.output_path,
                    "inputCount": result.input_count
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "pdf.evidence",
                "merge.failed",
                serde_json::json!({ "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn overlay_pdf_text(args: pdf::overlay::OverlayArgs) -> Result<pdf::overlay::OverlayResult, String> {
    app_log::info(
        "pdf.overlay",
        "overlay.start",
        serde_json::json!({
            "input": &args.input_path,
            "output": &args.output_path,
            "hasHeader": args.header.is_some(),
            "hasFooter": args.footer.is_some(),
        }),
    );
    match pdf::overlay::overlay_text(&args) {
        Ok(result) => {
            app_log::info(
                "pdf.overlay",
                "overlay.success",
                serde_json::json!({
                    "output": &result.output_path,
                    "pages": result.pages,
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "pdf.overlay",
                "overlay.failed",
                serde_json::json!({ "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn batch_overlay_pdf_text(
    args: pdf::overlay::BatchOverlayArgs,
) -> Result<pdf::overlay::BatchOverlayResult, String> {
    app_log::info(
        "pdf.overlay",
        "batch_overlay.start",
        serde_json::json!({ "count": args.items.len() }),
    );
    match pdf::overlay::batch_overlay(&args) {
        Ok(result) => {
            app_log::info(
                "pdf.overlay",
                "batch_overlay.success",
                serde_json::json!({
                    "success": result.results.len(),
                    "failed": result.failed.len(),
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "pdf.overlay",
                "batch_overlay.failed",
                serde_json::json!({ "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn get_pdf_page_count(path: String) -> Result<usize, String> {
    pdf::overlay::get_page_count(&path)
}

#[tauri::command]
fn check_pdf_pages(path: String) -> Result<pdf::overlay::PageCheckResult, String> {
    pdf::overlay::check_pdf_pages(&path)
}

#[tauri::command]
fn analyze_image_paddler_folder(
    folder: String,
    depth: Option<usize>,
) -> Result<image_paddler::PaddlerAnalysis, String> {
    let depth = depth.unwrap_or(5);
    app_log::info(
        "image_paddler",
        "analyze_folder.start",
        serde_json::json!({ "folder": folder, "depth": depth }),
    );
    match image_paddler::analyze_folder(&PathBuf::from(&folder), depth) {
        Ok(result) => {
            app_log::info(
                "image_paddler",
                "analyze_folder.success",
                serde_json::json!({
                    "folder": folder,
                    "imageCount": result.image_count,
                    "folderCount": result.folder_count,
                    "groups": result.groups.len()
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "image_paddler",
                "analyze_folder.failed",
                serde_json::json!({ "folder": folder, "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn run_image_paddler(
    args: image_paddler::PaddlerRunArgs,
) -> Result<image_paddler::PaddlerRunResult, String> {
    app_log::info(
        "image_paddler",
        "run.start",
        serde_json::json!({
            "folder": &args.folder,
            "outputDir": &args.output_dir,
            "groupMode": &args.group_mode,
            "selectedPrefixes": args.selected_prefixes.len()
        }),
    );
    match image_paddler::run(args) {
        Ok(result) => {
            app_log::info(
                "image_paddler",
                "run.success",
                serde_json::json!({
                    "outputDir": &result.output_dir,
                    "outputs": result.outputs.len(),
                    "skipped": result.skipped.len()
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "image_paddler",
                "run.failed",
                serde_json::json!({ "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    open_path_impl(&path)
}

fn open_path_impl(path: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(format!("文件不存在：{path}"));
    }

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(&p).status();
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", path]).status();
    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(&p).status();

    match result {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("打开失败，退出码 {s}")),
        Err(e) => Err(format!("打开失败：{e}")),
    }
}

#[tauri::command]
fn write_frontend_log(entry: app_log::FrontendLogEntry) -> Result<(), String> {
    app_log::write_frontend(entry)
}

#[tauri::command]
fn get_log_file_path() -> Result<String, String> {
    app_log::log_file_path().map(|p| p.display().to_string())
}

#[tauri::command]
fn open_log_file() -> Result<(), String> {
    app_log::info("app.log", "open_log_file", serde_json::json!({}));
    let path = app_log::log_file_path()?;
    open_path_impl(&path.display().to_string())
}

#[tauri::command]
fn open_log_dir() -> Result<(), String> {
    app_log::info("app.log", "open_log_dir", serde_json::json!({}));
    let dir = app_log::log_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败：{e}"))?;
    open_path_impl(&dir.display().to_string())
}

#[tauri::command]
fn get_diagnostic_info() -> Result<Value, String> {
    let log_file = app_log::log_file_path()?;
    let log_dir = app_log::log_dir()?;
    let log_files = app_log::list_log_files()
        .into_iter()
        .take(10)
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "appDataDir": log_dir.parent().map(|p| p.display().to_string()).unwrap_or_default(),
        "logDir": log_dir.display().to_string(),
        "currentLogFile": log_file.display().to_string(),
        "recentLogFiles": log_files,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "debug": cfg!(debug_assertions)
    }))
}

#[tauri::command]
fn check_ffmpeg() -> ffmpeg::detect::FfmpegStatus {
    ffmpeg::detect::check_ffmpeg()
}

#[tauri::command]
fn list_system_fonts() -> Vec<ffmpeg::detect::FontInfo> {
    ffmpeg::detect::list_system_fonts()
}

#[tauri::command]
fn probe_video(path: String) -> Result<ffmpeg::probe::VideoInfo, String> {
    app_log::info("video", "probe_video.start", serde_json::json!({ "path": path }));
    match ffmpeg::probe::probe_video(&PathBuf::from(&path)) {
        Ok(info) => {
            app_log::info(
                "video",
                "probe_video.success",
                serde_json::json!({
                    "path": path,
                    "duration": info.duration,
                    "width": info.width,
                    "height": info.height,
                    "fps": info.fps,
                    "codec": &info.codec
                }),
            );
            Ok(info)
        }
        Err(err) => {
            app_log::error(
                "video",
                "probe_video.failed",
                serde_json::json!({ "path": path, "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn extract_frames(
    args: ffmpeg::extract::ExtractArgs,
) -> Result<ffmpeg::extract::ExtractResult, String> {
    app_log::info(
        "video",
        "extract_frames.start",
        serde_json::json!({
            "input": &args.input,
            "outputDir": &args.output_dir,
            "fpsMode": &args.fps_mode,
            "fpsValue": args.fps_value,
            "format": &args.format
        }),
    );
    match ffmpeg::extract::extract_frames(&args) {
        Ok(result) => {
            app_log::info(
                "video",
                "extract_frames.success",
                serde_json::json!({
                    "input": &args.input,
                    "outputDir": &result.output_dir,
                    "totalFrames": result.total_frames,
                    "extractedFrames": result.extracted_frames
                }),
            );
            Ok(result)
        }
        Err(err) => {
            app_log::error(
                "video",
                "extract_frames.failed",
                serde_json::json!({
                    "input": &args.input,
                    "outputDir": &args.output_dir,
                    "error": err
                }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn try_brew_install_ffmpeg() -> Result<String, String> {
    ffmpeg::detect::try_brew_install_ffmpeg()
}

#[tauri::command]
fn try_brew_install_qpdf() -> Result<String, String> {
    ffmpeg::detect::try_brew_install_qpdf()
}

#[tauri::command]
fn get_app_settings() -> history::AppSettings {
    history::read_settings()
}

#[tauri::command]
fn set_app_settings(settings: history::AppSettings) -> Result<(), String> {
    history::write_settings(&settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_log::install_panic_hook();
    app_log::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_qpdf,
            inspect_pdf,
            unlock_pdf,
            scan_evidence_folder,
            build_evidence_group_pdfs,
            merge_evidence_pdfs,
            overlay_pdf_text,
            batch_overlay_pdf_text,
            get_pdf_page_count,
            check_pdf_pages,
            analyze_image_paddler_folder,
            run_image_paddler,
            open_path,
            write_frontend_log,
            get_log_file_path,
            open_log_file,
            open_log_dir,
            get_diagnostic_info,
            check_ffmpeg,
            list_system_fonts,
            probe_video,
            extract_frames,
            try_brew_install_ffmpeg,
            try_brew_install_qpdf,
            get_app_settings,
            set_app_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
