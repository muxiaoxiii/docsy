use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::PathBuf;

use chrono::Local;
use serde::Deserialize;
use serde_json::{json, Value};

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
    write_result(&entry.level, &entry.target, &entry.message, entry.context)
}

pub fn debug(target: &str, message: &str, context: Value) {
    write("debug", target, message, Some(context));
}

pub fn info(target: &str, message: &str, context: Value) {
    write("info", target, message, Some(context));
}

pub fn error(target: &str, message: &str, context: Value) {
    write("error", target, message, Some(context));
}

pub fn trace(target: &str, message: &str, context: Value) {
    write("trace", target, message, Some(context));
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
        let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y%m%d") else {
            continue;
        };
        if date < cutoff {
            let _ = fs::remove_file(path);
        }
    }
}

fn write(level: &str, target: &str, message: &str, context: Option<Value>) {
    if let Err(err) = write_result(level, target, message, context) {
        eprintln!("Docsy log write failed: {err}");
    }
}

fn write_result(
    level: &str,
    target: &str,
    message: &str,
    context: Option<Value>,
) -> Result<(), String> {
    let path = log_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建日志目录失败：{e}"))?;
    }

    let line = json!({
        "ts": Local::now().to_rfc3339(),
        "level": level,
        "target": target,
        "message": message,
        "context": context.unwrap_or(Value::Null),
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("打开日志文件失败：{e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("写入日志失败：{e}"))
}
