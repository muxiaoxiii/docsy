use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use chrono::Local;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};

static DOCUMENT_REFERENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?ix)
        (?:[a-z]:\\|\\\\)[^\r\n\";<>|?*]*?\.(?:pdf|docx?|docm|xlsx?|xlsm|pptx?|pptm|md|txt|csv|tsv|rtf|odt|ods|odp|jpg|jpeg|png|webp|bmp|tiff?|gif|mp4|mov|mkv|avi|wmv|m4v|webm|zip|7z|rar)
        |
        /(?:[^/\r\n\";]+/)+[^/\r\n\";]+?\.(?:pdf|docx?|docm|xlsx?|xlsm|pptx?|pptm|md|txt|csv|tsv|rtf|odt|ods|odp|jpg|jpeg|png|webp|bmp|tiff?|gif|mp4|mov|mkv|avi|wmv|m4v|webm|zip|7z|rar)
        |
        [^\s/\\\r\n\"<>|?*:，。；：()（）]+\.(?:pdf|docx?|docm|xlsx?|xlsm|pptx?|pptm|md|txt|csv|tsv|rtf|odt|ods|odp|jpg|jpeg|png|webp|bmp|tiff?|gif|mp4|mov|mkv|avi|wmv|m4v|webm|zip|7z|rar)
        "#,
    )
    .expect("valid document reference regex")
});

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendLogEntry {
    pub level: String,
    pub target: String,
    pub message: String,
    pub context: Option<Value>,
}

pub fn log_dir() -> Result<PathBuf, String> {
    let mut dir = dirs::data_dir().ok_or("无法解析应用数据目录")?;
    dir.push("Docsy");
    dir.push("logs");
    Ok(dir)
}

pub fn log_file_path() -> Result<PathBuf, String> {
    let mut path = log_dir()?;
    let date = Local::now().format("%Y%m%d").to_string();
    path.push(format!("docsy-{date}.log"));
    Ok(path)
}

pub fn create_sanitized_log_copy(source: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(source).map_err(|e| format!("读取日志文件失败：{e}"))?;
    let mut redactor = LogRedactor::default();
    let mut output = log_dir()?;
    fs::create_dir_all(&output).map_err(|e| format!("创建日志目录失败：{e}"))?;
    output.push(format!(
        "docsy-{}-sanitized.log",
        Local::now().format("%Y%m%d-%H%M%S")
    ));
    let mut writer = fs::File::create(&output).map_err(|e| format!("创建脱敏日志失败：{e}"))?;

    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| format!("读取日志内容失败：{e}"))?;
        if let Ok(mut value) = serde_json::from_str::<Value>(&line) {
            redactor.redact_value(&mut value, None);
            writeln!(writer, "{value}").map_err(|e| format!("写入脱敏日志失败：{e}"))?;
        } else {
            writeln!(writer, "{}", redactor.redact_text(&line))
                .map_err(|e| format!("写入脱敏日志失败：{e}"))?;
        }
    }
    Ok(output)
}

#[derive(Default)]
struct LogRedactor {
    aliases: HashMap<String, String>,
    next_file: usize,
    next_path: usize,
}

impl LogRedactor {
    fn redact_value(&mut self, value: &mut Value, key: Option<&str>) {
        match value {
            Value::Object(map) => {
                for (child_key, child_value) in map {
                    self.redact_value(child_value, Some(child_key));
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.redact_value(item, key);
                }
            }
            Value::String(text) => {
                *text = if key.is_some_and(is_private_location_key) {
                    self.redact_location_value(text, key.unwrap_or_default())
                } else {
                    self.redact_text(text)
                };
            }
            _ => {}
        }
    }

    fn redact_location_value(&mut self, value: &str, key: &str) -> String {
        if is_filename_key(key) {
            return self.alias_for_file(value, path_extension(value));
        }
        if looks_like_path(value) || path_extension(value).is_some() {
            return self.alias_for(value, path_extension(value));
        }
        self.redact_text(value)
    }

    fn redact_text(&mut self, value: &str) -> String {
        DOCUMENT_REFERENCE_RE
            .replace_all(value, |caps: &regex::Captures<'_>| {
                self.alias_for(&caps[0], path_extension(&caps[0]))
            })
            .into_owned()
    }

    fn alias_for(&mut self, original: &str, extension: Option<&str>) -> String {
        if let Some(alias) = self.aliases.get(original) {
            return alias.clone();
        }
        if extension.is_some() {
            return self.alias_for_file(original, extension);
        }
        self.next_path += 1;
        let alias = format!("[路径-{}]", self.next_path);
        self.aliases.insert(original.to_string(), alias.clone());
        alias
    }

    fn alias_for_file(&mut self, original: &str, extension: Option<&str>) -> String {
        if let Some(alias) = self.aliases.get(original) {
            return alias.clone();
        }
        self.next_file += 1;
        let alias = if let Some(extension) = extension {
            format!(
                "[文件-{}].{}",
                self.next_file,
                extension.to_ascii_lowercase()
            )
        } else {
            format!("[文件-{}]", self.next_file)
        };
        self.aliases.insert(original.to_string(), alias.clone());
        alias
    }
}

fn is_private_location_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
    normalized.contains("path")
        || normalized.contains("filename")
        || normalized == "file"
        || normalized.starts_with("input")
        || normalized.starts_with("output")
        || normalized.starts_with("source")
        || normalized.starts_with("destination")
}

fn is_filename_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
    normalized == "file" || normalized.contains("filename")
}

fn looks_like_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("~/")
        || value.starts_with("\\\\")
        || (value.len() >= 3
            && value.as_bytes()[1] == b':'
            && matches!(value.as_bytes()[2], b'\\' | b'/'))
}

fn path_extension(value: &str) -> Option<&str> {
    let leaf = value.rsplit(['/', '\\']).next()?;
    let (_, extension) = leaf.rsplit_once('.')?;
    (!extension.is_empty() && extension.len() <= 8).then_some(extension)
}

pub fn init() {
    cleanup_old_logs(14);
    info(
        "app.lifecycle",
        "backend.start",
        json!({
            "debug": cfg!(debug_assertions),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH
        }),
    );
}

pub fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info.location().map(|loc| {
            json!({
                "file": loc.file(),
                "line": loc.line(),
                "column": loc.column()
            })
        });
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".to_string());
        error(
            "app.panic",
            "backend.panic",
            json!({
                "payload": payload,
                "location": location
            }),
        );
        previous(panic_info);
    }));
}

pub fn write_frontend(entry: FrontendLogEntry) -> Result<(), String> {
    write_result(
        &entry.level,
        &entry.target,
        &entry.message,
        entry.context,
        None,
    )
}

pub fn info(target: &str, message: &str, context: Value) {
    write("info", target, message, Some(context), None);
}

pub fn warn(target: &str, message: &str, context: Value) {
    write("warn", target, message, Some(context), None);
}

pub fn error(target: &str, message: &str, context: Value) {
    write("error", target, message, Some(context), None);
}

pub fn debug(target: &str, message: &str, context: Value) {
    write("debug", target, message, Some(context), None);
}

pub fn info_with_op(target: &str, message: &str, context: Value, operation_id: &str) {
    write("info", target, message, Some(context), Some(operation_id));
}

pub fn error_with_op(target: &str, message: &str, context: Value, operation_id: &str) {
    write("error", target, message, Some(context), Some(operation_id));
}

pub fn list_log_files() -> Vec<PathBuf> {
    let Ok(dir) = log_dir() else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut files = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("log"))
        .collect::<Vec<_>>();
    files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    files
}

fn cleanup_old_logs(retain_days: i64) {
    let cutoff = Local::now().date_naive() - chrono::Duration::days(retain_days);
    for path in list_log_files() {
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(date_part) = stem.strip_prefix("docsy-") else {
            continue;
        };
        let Some(date_token) = date_part.get(..8) else {
            continue;
        };
        let Ok(date) = chrono::NaiveDate::parse_from_str(date_token, "%Y%m%d") else {
            continue;
        };
        if date < cutoff {
            let _ = fs::remove_file(path);
        }
    }
}

fn write(
    level: &str,
    target: &str,
    message: &str,
    context: Option<Value>,
    operation_id: Option<&str>,
) {
    if let Err(err) = write_result(level, target, message, context, operation_id) {
        eprintln!("Docsy log write failed: {err}");
    }
}

fn write_result(
    level: &str,
    target: &str,
    message: &str,
    context: Option<Value>,
    operation_id: Option<&str>,
) -> Result<(), String> {
    let path = log_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建日志目录失败：{e}"))?;
    }

    let mut line = json!({
        "ts": Local::now().to_rfc3339(),
        "level": level,
        "target": target,
        "message": message,
        "context": context.unwrap_or(Value::Null),
    });
    if let Some(op_id) = operation_id {
        line["op"] = Value::String(op_id.to_string());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("打开日志文件失败：{e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("写入日志失败：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_windows_and_macos_document_paths_without_losing_error_text() {
        let mut redactor = LogRedactor::default();
        let message = "qpdf failed: C:\\Users\\Alice\\案件\\证据 1.pdf; retry /Users/alice/Documents/案件/证据 2.pdf";

        let redacted = redactor.redact_text(message);

        assert_eq!(redacted, "qpdf failed: [文件-1].pdf; retry [文件-2].pdf");
        assert!(!redacted.contains("Alice"));
        assert!(!redacted.contains("案件"));
    }

    #[test]
    fn uses_the_same_alias_for_repeated_file_references() {
        let mut redactor = LogRedactor::default();
        let path = "/Users/alice/Documents/客户资料.pdf";

        let redacted = redactor.redact_text(&format!("start {path}; failed {path}"));

        assert_eq!(redacted, "start [文件-1].pdf; failed [文件-1].pdf");
    }

    #[test]
    fn redacts_filename_and_directory_fields_in_structured_logs() {
        let mut redactor = LogRedactor::default();
        let mut value = json!({
            "fileName": "客户 证据清单.xlsx",
            "file": "无扩展名证据",
            "outputDir": "/Users/alice/Documents/客户案件",
            "engine": "qpdf-index",
            "message": "处理 合同.pdf 失败"
        });

        redactor.redact_value(&mut value, None);

        let file_name = value["fileName"].as_str().unwrap_or_default();
        let extensionless_file = value["file"].as_str().unwrap_or_default();
        assert!(file_name.starts_with("[文件-") && file_name.ends_with("].xlsx"));
        assert!(extensionless_file.starts_with("[文件-") && extensionless_file.ends_with(']'));
        assert_ne!(file_name, extensionless_file);
        assert_eq!(value["outputDir"], "[路径-1]");
        assert_eq!(value["engine"], "qpdf-index");
        let message = value["message"].as_str().unwrap_or_default();
        assert!(message.starts_with("处理 [文件-") && message.ends_with("].pdf 失败"));
        assert!(!message.contains("合同"));
    }
}
