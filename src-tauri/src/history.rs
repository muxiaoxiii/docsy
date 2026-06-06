//! 生成历史
//!
//! 每生成一份文件保存一份记录，含模板 id、表单值、字段属性快照、输出路径、时间戳。
//! 按模板 id 分目录：~/Library/Application Support/Docsy/history/<template_id>/<timestamp>.json
//! 可配置最大条数（settings.json 里的 history_max，默认 50）。

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::templates;

const HISTORY_DIR: &str = "history";
const SETTINGS_FILE: &str = "settings.json";

pub fn history_dir(template_id: &str) -> Option<PathBuf> {
    templates::user_data_dir().map(|p| p.join(HISTORY_DIR).join(template_id))
}

pub fn settings_path() -> Option<PathBuf> {
    templates::user_data_dir().map(|p| p.join(SETTINGS_FILE))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_history_max")]
    pub history_max: usize,
    /// 一级菜单可见性：缺省的项视为 true
    #[serde(default)]
    pub menu_visibility: serde_json::Map<String, serde_json::Value>,
}

fn default_history_max() -> usize {
    50
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            history_max: default_history_max(),
            menu_visibility: serde_json::Map::new(),
        }
    }
}

pub fn read_settings() -> AppSettings {
    let Some(path) = settings_path() else {
        return AppSettings::default();
    };
    fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn write_settings(settings: &AppSettings) -> Result<(), String> {
    let dir = templates::user_data_dir().ok_or("无法解析数据目录")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(SETTINGS_FILE);
    let s = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, s).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationRecord {
    pub id: String,
    pub template_id: String,
    pub timestamp: String,
    pub values: Value,
    pub field_opts: Value,
    pub required_map: Value,
    pub output_path: String,
    /// 简短标签：从 values 里提取关键字段（如委托人、案号）拼成的人类可读说明
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveRecordArgs {
    pub template_id: String,
    pub timestamp: String,
    pub values: Value,
    pub field_opts: Value,
    pub required_map: Value,
    pub output_path: String,
    pub label: String,
}

pub fn save_record(args: SaveRecordArgs) -> Result<GenerationRecord, String> {
    let dir = history_dir(&args.template_id).ok_or("无法解析历史目录")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let record = GenerationRecord {
        id: args.timestamp.clone(),
        template_id: args.template_id.clone(),
        timestamp: args.timestamp,
        values: args.values,
        field_opts: args.field_opts,
        required_map: args.required_map,
        output_path: args.output_path,
        label: args.label,
    };

    let path = dir.join(format!("{}.json", record.id));
    let s = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    fs::write(&path, s).map_err(|e| e.to_string())?;

    let max = read_settings().history_max;
    prune_records(&args.template_id, max).ok();

    Ok(record)
}

pub fn list_records(template_id: &str) -> Vec<GenerationRecord> {
    let Some(dir) = history_dir(template_id) else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return vec![];
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(rec) = serde_json::from_slice::<GenerationRecord>(&bytes) {
                out.push(rec);
            }
        }
    }
    out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    out
}

pub fn read_record(template_id: &str, id: &str) -> Result<GenerationRecord, String> {
    let dir = history_dir(template_id).ok_or("无法解析历史目录")?;
    let path = dir.join(format!("{id}.json"));
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn delete_record(template_id: &str, id: &str) -> Result<(), String> {
    let dir = history_dir(template_id).ok_or("无法解析历史目录")?;
    let path = dir.join(format!("{id}.json"));
    fs::remove_file(&path).map_err(|e| e.to_string())
}

fn prune_records(template_id: &str, max: usize) -> Result<(), String> {
    let Some(dir) = history_dir(template_id) else {
        return Ok(());
    };
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    if entries.len() <= max {
        return Ok(());
    }
    entries.sort_by_key(|e| e.file_name());
    let drop_count = entries.len() - max;
    for e in entries.into_iter().take(drop_count) {
        let _ = fs::remove_file(e.path());
    }
    Ok(())
}
