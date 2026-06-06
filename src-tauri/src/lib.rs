mod app_log;
mod dict_xlsx;
mod docx;
mod ffmpeg;
mod history;
mod image_paddler;
mod pdf;
mod template_builder;
mod templates;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use docx::render::{FieldOptions, FieldValue, RenderRequest};
use pdf::qpdf;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BUILTIN_LETTER_DOCX: &[u8] = include_bytes!("../templates/letter.docx");
const BUILTIN_LETTER_FIELDS: &str = include_str!("../templates/letter.fields.json");
const BUILTIN_DICTIONARIES: &str = include_str!("../templates/dictionaries.json");

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
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("文件不存在：{path}"));
    }

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(&p).status();
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd")
        .args(["/C", "start", "", &path])
        .status();
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
    app_log::info(
        "video",
        "probe_video.start",
        serde_json::json!({ "path": path }),
    );
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
fn get_letter_fields() -> Result<Value, String> {
    if template_builder::user_template_exists("letter") {
        return get_template_meta("letter".to_string()).map(|m| m.fields);
    }
    let default: Value = serde_json::from_str(BUILTIN_LETTER_FIELDS)
        .map_err(|e| format!("字段定义解析失败：{e}"))?;
    let ov = templates::read_config("letter");
    Ok(templates::merge_letter_config(default, ov))
}

#[tauri::command]
fn get_dictionaries(template_id: Option<String>) -> Result<Value, String> {
    let id = template_id.unwrap_or_else(|| "letter".to_string());
    if let Some(path) = template_builder::user_template_path(&id).filter(|p| p.exists()) {
        let builtin: Value = template_builder::read_template_file(&path, "dictionaries.json")
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let ov_id = if id == "letter" {
            "dictionaries".to_string()
        } else {
            format!("dict_{id}")
        };
        let ov = templates::read_config(&ov_id);
        Ok(sanitize_dictionaries_for_ui(templates::merge_dictionaries(
            builtin, ov,
        )))
    } else if id == "letter" {
        let default: Value =
            serde_json::from_str(BUILTIN_DICTIONARIES).map_err(|e| format!("字典解析失败：{e}"))?;
        let ov = templates::read_config("dictionaries");
        Ok(sanitize_dictionaries_for_ui(templates::merge_dictionaries(
            default, ov,
        )))
    } else {
        // 用户模板：从 docsytpl 读 dictionaries.json（如果有），叠加用户编辑覆盖
        let dir = template_builder::user_templates_dir().ok_or("无法解析模板目录")?;
        let path = dir.join(format!("{id}.docsytpl"));
        let builtin: Value = template_builder::read_template_file(&path, "dictionaries.json")
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let ov = templates::read_config(&format!("dict_{id}"));
        Ok(sanitize_dictionaries_for_ui(templates::merge_dictionaries(
            builtin, ov,
        )))
    }
}

fn sanitize_dictionaries_for_ui(value: Value) -> Value {
    let Value::Object(map) = value else {
        return value;
    };
    let mut out = serde_json::Map::new();
    for (key, items) in map {
        let Value::Array(list) = items else {
            out.insert(key, items);
            continue;
        };
        let cleaned = list
            .into_iter()
            .filter(|item| !is_generated_dict_value(item))
            .collect::<Vec<_>>();
        if !cleaned.is_empty() {
            out.insert(key, Value::Array(cleaned));
        }
    }
    Value::Object(out)
}

fn is_generated_dict_value(item: &Value) -> bool {
    let text = item
        .as_str()
        .or_else(|| item.get("name").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim();
    if text.is_empty() {
        return true;
    }
    if text.starts_with("{{") && text.ends_with("}}") {
        return true;
    }
    let Ok(re) = regex::Regex::new(r"^field_\d+$") else {
        return false;
    };
    re.is_match(text)
}

#[tauri::command]
fn get_builtin_letter_fields() -> Result<Value, String> {
    serde_json::from_str(BUILTIN_LETTER_FIELDS).map_err(|e| format!("解析失败：{e}"))
}

#[tauri::command]
fn get_builtin_dictionaries() -> Result<Value, String> {
    serde_json::from_str(BUILTIN_DICTIONARIES).map_err(|e| format!("解析失败：{e}"))
}

#[derive(Debug, Deserialize)]
struct SaveTemplateArgs {
    template_id: String,
    config: Value,
    timestamp: String,
    max_archives: Option<usize>,
}

#[tauri::command]
fn save_template_config(args: SaveTemplateArgs) -> Result<templates::SaveResult, String> {
    templates::save_config(
        &args.template_id,
        &args.config,
        args.max_archives,
        &args.timestamp,
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserTemplateConfigArgs {
    template_id: String,
    fields: Value,
    dictionaries: Value,
    timestamp: String,
    max_archives: Option<usize>,
}

#[tauri::command]
fn update_user_template_config(args: UpdateUserTemplateConfigArgs) -> Result<(), String> {
    app_log::info(
        "template.config",
        "update_user_template_config.start",
        serde_json::json!({
            "templateId": args.template_id,
            "fieldsCount": args.fields.get("fields").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            "dictionaryCount": args.dictionaries.as_object().map(|m| m.len()).unwrap_or(0)
        }),
    );

    template_builder::update_user_template_config(
        &args.template_id,
        &args.fields,
        &args.dictionaries,
    )?;

    // 兼容历史归档视图：仍保存一份字段和字典归档，但当前版本以 docsytpl 为准。
    let _ = templates::save_config(
        &args.template_id,
        &args.fields,
        args.max_archives,
        &args.timestamp,
    );
    let _ = templates::save_config(
        &format!("dict_{}", args.template_id),
        &args.dictionaries,
        args.max_archives,
        &args.timestamp,
    );

    app_log::info(
        "template.config",
        "update_user_template_config.success",
        serde_json::json!({ "templateId": args.template_id }),
    );
    Ok(())
}

#[tauri::command]
fn list_template_archives(template_id: String) -> Vec<templates::ArchiveInfo> {
    templates::list_archives(&template_id)
}

#[tauri::command]
fn restore_template_archive(template_id: String, archive_id: String) -> Result<(), String> {
    templates::restore_archive(&template_id, &archive_id)
}

#[tauri::command]
fn read_template_archive(template_id: String, archive_id: String) -> Result<Value, String> {
    let path = templates::archive_dir(&template_id)
        .ok_or("无法解析归档目录")?
        .join(format!("{archive_id}.json"));
    let bytes = std::fs::read(&path).map_err(|e| format!("读取失败：{e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("解析失败：{e}"))
}

#[tauri::command]
fn delete_template_archive(template_id: String, archive_id: String) -> Result<(), String> {
    let path = templates::archive_dir(&template_id)
        .ok_or("无法解析归档目录")?
        .join(format!("{archive_id}.json"));
    std::fs::remove_file(&path).map_err(|e| format!("删除失败：{e}"))
}

#[tauri::command]
fn is_template_enabled(template_id: String) -> bool {
    templates::is_enabled(&template_id)
}

#[tauri::command]
fn set_template_enabled(template_id: String, enabled: bool) -> Result<(), String> {
    templates::set_enabled(&template_id, enabled)
}

#[tauri::command]
fn export_dictionaries_xlsx(path: String) -> Result<String, String> {
    let default: Value =
        serde_json::from_str(BUILTIN_DICTIONARIES).map_err(|e| format!("内置字典解析失败：{e}"))?;
    let ov = templates::read_config("dictionaries");
    let merged = templates::merge_dictionaries(default, ov);
    dict_xlsx::export_to_xlsx(&PathBuf::from(&path), &merged)?;
    Ok(path)
}

#[derive(Debug, Deserialize)]
struct ImportDictArgs {
    path: String,
    /// "merge"：与现有合并（同名字典整体替换），"replace"：完全替换
    mode: String,
}

#[tauri::command]
fn import_dictionaries_xlsx(args: ImportDictArgs) -> Result<Value, String> {
    let imported = dict_xlsx::import_from_xlsx(&PathBuf::from(&args.path))?;
    let final_value = if args.mode == "replace" {
        imported.clone()
    } else {
        let default: Value = serde_json::from_str(BUILTIN_DICTIONARIES)
            .map_err(|e| format!("内置字典解析失败：{e}"))?;
        let existing = templates::read_config("dictionaries");
        let cur = templates::merge_dictionaries(default, existing);
        templates::merge_dictionaries(cur, Some(imported))
    };
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    templates::save_config("dictionaries", &final_value, Some(10), &ts)?;
    Ok(final_value)
}

#[tauri::command]
fn extract_docx_text(path: String) -> Result<template_builder::DocxText, String> {
    app_log::info(
        "template.docx",
        "extract_docx_text.start",
        serde_json::json!({ "path": path }),
    );
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            let msg = format!("读取失败：{e}");
            app_log::error(
                "template.docx",
                "extract_docx_text.failed",
                serde_json::json!({ "path": path, "error": msg }),
            );
            return Err(msg);
        }
    };
    match template_builder::extract_plain_text(&bytes) {
        Ok(text) => {
            app_log::info(
                "template.docx",
                "extract_docx_text.success",
                serde_json::json!({
                    "path": path,
                    "bytes": bytes.len(),
                    "plainChars": text.plain_text.chars().count(),
                    "paragraphCount": text.paragraph_count
                }),
            );
            Ok(text)
        }
        Err(err) => {
            app_log::error(
                "template.docx",
                "extract_docx_text.failed",
                serde_json::json!({ "path": path, "bytes": bytes.len(), "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| format!("读取失败：{e}"))
}

#[tauri::command]
fn list_user_templates() -> Vec<template_builder::UserTemplate> {
    template_builder::list_user_templates()
}

#[tauri::command]
fn save_user_template(
    args: template_builder::SaveTemplateArgs,
) -> Result<template_builder::UserTemplate, String> {
    let id = args
        .manifest
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let fields_count = args
        .fields
        .get("fields")
        .and_then(|v| v.as_array())
        .map(|items| items.len())
        .unwrap_or(0);
    let marks_count = args.marks.len();
    let has_builder_state = args.builder_state.is_some();
    let source_base64_len = args.original_docx_base64.len();
    let fields_snapshot = args.fields.clone();
    let dictionaries_snapshot = args.dictionaries.clone();
    app_log::info(
        "template.save",
        "save_user_template.start",
        serde_json::json!({
            "id": id,
            "fieldsCount": fields_count,
            "marksCount": marks_count,
            "hasBuilderState": has_builder_state,
            "sourceBase64Length": source_base64_len
        }),
    );
    match template_builder::save_user_template(args) {
        Ok(template) => {
            let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
            let _ = templates::save_config(&id, &fields_snapshot, Some(10), &timestamp);
            if let Some(dictionaries) = &dictionaries_snapshot {
                let dict_key = if id == "letter" {
                    "dictionaries".to_string()
                } else {
                    format!("dict_{id}")
                };
                let _ = templates::save_config(&dict_key, dictionaries, Some(10), &timestamp);
            }
            app_log::info(
                "template.save",
                "save_user_template.success",
                serde_json::json!({ "id": template.id, "path": template.path }),
            );
            Ok(template)
        }
        Err(err) => {
            app_log::error(
                "template.save",
                "save_user_template.failed",
                serde_json::json!({
                    "id": id,
                    "fieldsCount": fields_count,
                    "marksCount": marks_count,
                    "error": err
                }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn delete_user_template(id: String) -> Result<(), String> {
    template_builder::delete_user_template(&id)
}

#[tauri::command]
fn rename_user_template(id: String, new_name: String) -> Result<(), String> {
    template_builder::rename_user_template(&id, &new_name)
}

#[tauri::command]
fn read_template_for_edit(id: String) -> Result<template_builder::TemplateEditData, String> {
    app_log::info(
        "template.edit",
        "read_template_for_edit.start",
        serde_json::json!({ "id": id }),
    );
    match template_builder::read_template_for_edit(&id) {
        Ok(data) => {
            app_log::info(
                "template.edit",
                "read_template_for_edit.success",
                serde_json::json!({
                    "id": id,
                    "marksCount": data.marks.len(),
                    "hasBuilderState": data.builder_state.is_some(),
                    "docxBase64Length": data.docx_base64.len()
                }),
            );
            Ok(data)
        }
        Err(err) => {
            app_log::error(
                "template.edit",
                "read_template_for_edit.failed",
                serde_json::json!({ "id": id, "error": err }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn extract_docx_text_from_base64(base64: String) -> Result<template_builder::DocxText, String> {
    app_log::debug(
        "template.docx",
        "extract_docx_text_from_base64.start",
        serde_json::json!({ "base64Length": base64.len() }),
    );
    let bytes = match template_builder::base64_decode::decode_base64(&base64) {
        Ok(bytes) => bytes,
        Err(err) => {
            app_log::error(
                "template.docx",
                "extract_docx_text_from_base64.failed",
                serde_json::json!({ "base64Length": base64.len(), "error": err }),
            );
            return Err(err);
        }
    };
    match template_builder::extract_plain_text(&bytes) {
        Ok(text) => {
            app_log::debug(
                "template.docx",
                "extract_docx_text_from_base64.success",
                serde_json::json!({
                    "bytes": bytes.len(),
                    "plainChars": text.plain_text.chars().count(),
                    "paragraphCount": text.paragraph_count
                }),
            );
            Ok(text)
        }
        Err(err) => {
            app_log::error(
                "template.docx",
                "extract_docx_text_from_base64.failed",
                serde_json::json!({ "bytes": bytes.len(), "error": err }),
            );
            Err(err)
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditDocxTextArgs {
    docx_base64: String,
    start: usize,
    end: usize,
    replacement: String,
}

#[tauri::command]
fn edit_docx_text_range(args: EditDocxTextArgs) -> Result<String, String> {
    let replacement_chars = args.replacement.chars().count();
    app_log::info(
        "template.docx",
        "edit_docx_text_range.start",
        serde_json::json!({
            "start": args.start,
            "end": args.end,
            "replacementChars": replacement_chars,
            "docxBase64Length": args.docx_base64.len()
        }),
    );
    let bytes = match template_builder::base64_decode::decode_base64(&args.docx_base64) {
        Ok(bytes) => bytes,
        Err(err) => {
            app_log::error(
                "template.docx",
                "edit_docx_text_range.failed",
                serde_json::json!({
                    "start": args.start,
                    "end": args.end,
                    "replacementChars": replacement_chars,
                    "error": err
                }),
            );
            return Err(err);
        }
    };
    let edited =
        match template_builder::replace_text_range(&bytes, args.start, args.end, &args.replacement)
        {
            Ok(edited) => edited,
            Err(err) => {
                app_log::error(
                    "template.docx",
                    "edit_docx_text_range.failed",
                    serde_json::json!({
                        "start": args.start,
                        "end": args.end,
                        "replacementChars": replacement_chars,
                        "bytes": bytes.len(),
                        "error": err
                    }),
                );
                return Err(err);
            }
        };
    app_log::info(
        "template.docx",
        "edit_docx_text_range.success",
        serde_json::json!({
            "start": args.start,
            "end": args.end,
            "replacementChars": replacement_chars,
            "inputBytes": bytes.len(),
            "outputBytes": edited.len()
        }),
    );
    Ok(template_builder::base64_encode::encode_base64(&edited))
}

#[tauri::command]
fn get_user_template_fields(id: String) -> Result<Value, String> {
    let path = template_builder::user_template_path(&id).ok_or("无法解析模板目录")?;
    let bytes = template_builder::read_template_file(&path, "fields.json")?;
    let fields: Value = serde_json::from_slice(&bytes).map_err(|e| format!("解析失败：{e}"))?;
    Ok(templates::merge_letter_config(
        fields,
        templates::read_config(&id),
    ))
}

#[tauri::command]
fn save_generation_record(
    args: history::SaveRecordArgs,
) -> Result<history::GenerationRecord, String> {
    history::save_record(args)
}

#[tauri::command]
fn list_generation_records(template_id: String) -> Vec<history::GenerationRecord> {
    history::list_records(&template_id)
}

#[tauri::command]
fn read_generation_record(
    template_id: String,
    record_id: String,
) -> Result<history::GenerationRecord, String> {
    history::read_record(&template_id, &record_id)
}

#[tauri::command]
fn delete_generation_record(template_id: String, record_id: String) -> Result<(), String> {
    history::delete_record(&template_id, &record_id)
}

#[tauri::command]
fn get_app_settings() -> history::AppSettings {
    history::read_settings()
}

#[tauri::command]
fn set_app_settings(settings: history::AppSettings) -> Result<(), String> {
    history::write_settings(&settings)
}

#[derive(Debug, Deserialize)]
struct GenerateLetterArgs {
    values: HashMap<String, FieldValue>,
    field_opts: HashMap<String, FieldOptions>,
    output_path: String,
    #[serde(default)]
    template_id: Option<String>,
}

fn resolve_template_bytes(template_id: &str) -> Result<Vec<u8>, String> {
    if let Some(path) = template_builder::user_template_path(template_id).filter(|p| p.exists()) {
        template_builder::read_template_file(&path, "template.docx")
    } else if template_id == "letter" || template_id.is_empty() {
        Ok(BUILTIN_LETTER_DOCX.to_vec())
    } else {
        let dir = template_builder::user_templates_dir().ok_or("无法解析模板目录")?;
        let path = dir.join(format!("{template_id}.docsytpl"));
        template_builder::read_template_file(&path, "template.docx")
    }
}

#[tauri::command]
fn generate_letter(args: GenerateLetterArgs) -> Result<String, String> {
    let id = args
        .template_id
        .clone()
        .unwrap_or_else(|| "letter".to_string());
    app_log::info(
        "generation",
        "generate_letter.start",
        serde_json::json!({
            "templateId": id,
            "fieldCount": args.values.len(),
            "fieldOptionCount": args.field_opts.len(),
            "outputPath": &args.output_path
        }),
    );
    let bytes = match resolve_template_bytes(&id) {
        Ok(bytes) => bytes,
        Err(err) => {
            app_log::error(
                "generation",
                "generate_letter.failed",
                serde_json::json!({ "templateId": id, "outputPath": &args.output_path, "error": err }),
            );
            return Err(err);
        }
    };
    let req = RenderRequest {
        template_bytes: &bytes,
        values: args.values,
        field_opts: args.field_opts,
    };
    let bytes = match docx::render::render(req).map_err(|e| e.to_string()) {
        Ok(bytes) => bytes,
        Err(err) => {
            app_log::error(
                "generation",
                "generate_letter.failed",
                serde_json::json!({ "templateId": id, "outputPath": &args.output_path, "error": err }),
            );
            return Err(err);
        }
    };
    if let Err(err) =
        std::fs::write(&args.output_path, &bytes).map_err(|e| format!("写入失败：{e}"))
    {
        app_log::error(
            "generation",
            "generate_letter.failed",
            serde_json::json!({ "templateId": id, "outputPath": &args.output_path, "error": err }),
        );
        return Err(err);
    }
    app_log::info(
        "generation",
        "generate_letter.success",
        serde_json::json!({ "templateId": id, "outputPath": &args.output_path, "bytes": bytes.len() }),
    );
    Ok(args.output_path)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateMeta {
    id: String,
    name: String,
    builtin: bool,
    fields: Value,
}

#[tauri::command]
fn get_template_meta(template_id: String) -> Result<TemplateMeta, String> {
    if let Some(path) = template_builder::user_template_path(&template_id).filter(|p| p.exists()) {
        let manifest_bytes = template_builder::read_template_file(&path, "manifest.json")?;
        let fields_bytes = template_builder::read_template_file(&path, "fields.json")?;
        let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
        let fields: Value = serde_json::from_slice(&fields_bytes).map_err(|e| e.to_string())?;
        let fields = templates::merge_letter_config(fields, templates::read_config(&template_id));
        let name = manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&template_id)
            .to_string();
        Ok(TemplateMeta {
            id: template_id.clone(),
            name,
            builtin: template_builder::is_builtin_template_id(&template_id),
            fields,
        })
    } else if template_id == "letter" || template_id.is_empty() {
        let default: Value = serde_json::from_str(BUILTIN_LETTER_FIELDS)
            .map_err(|e| format!("字段定义解析失败：{e}"))?;
        let ov = templates::read_config("letter");
        let merged = templates::merge_letter_config(default, ov);
        let name = merged
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("律师事务所函")
            .to_string();
        Ok(TemplateMeta {
            id: "letter".to_string(),
            name,
            builtin: true,
            fields: merged,
        })
    } else {
        let dir = template_builder::user_templates_dir().ok_or("无法解析模板目录")?;
        let path = dir.join(format!("{template_id}.docsytpl"));
        let manifest_bytes = template_builder::read_template_file(&path, "manifest.json")?;
        let fields_bytes = template_builder::read_template_file(&path, "fields.json")?;
        let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
        let fields: Value = serde_json::from_slice(&fields_bytes).map_err(|e| e.to_string())?;
        let name = manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&template_id)
            .to_string();
        Ok(TemplateMeta {
            id: template_id,
            name,
            builtin: false,
            fields,
        })
    }
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
            get_letter_fields,
            get_dictionaries,
            get_builtin_letter_fields,
            get_builtin_dictionaries,
            save_template_config,
            update_user_template_config,
            list_template_archives,
            restore_template_archive,
            read_template_archive,
            delete_template_archive,
            is_template_enabled,
            set_template_enabled,
            export_dictionaries_xlsx,
            import_dictionaries_xlsx,
            extract_docx_text,
            read_file_bytes,
            list_user_templates,
            save_user_template,
            delete_user_template,
            rename_user_template,
            read_template_for_edit,
            extract_docx_text_from_base64,
            edit_docx_text_range,
            get_user_template_fields,
            get_template_meta,
            save_generation_record,
            list_generation_records,
            read_generation_record,
            delete_generation_record,
            get_app_settings,
            set_app_settings,
            generate_letter
        ])
        .run(tauri::generate_context!())
        .expect("error while running Docsy");
}
