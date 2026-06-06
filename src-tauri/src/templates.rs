//! 模板配置存储：用户级覆盖 + 归档（按日期保留最近 N 份）
//!
//! 结构：
//! ```text
//! ~/Library/Application Support/Docsy/templates/
//!   ├─ letter.json            当前生效的覆盖（合并到内置之上）
//!   └─ letter/                归档目录（每次保存生成一份）
//!      ├─ 20260603-153012.json
//!      ├─ 20260603-160501.json
//!      └─ ...
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const APP_DIR: &str = "Docsy";
const TEMPLATES_DIR: &str = "templates";

pub fn user_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join(APP_DIR))
}

pub fn templates_dir() -> Option<PathBuf> {
    user_data_dir().map(|p| p.join(TEMPLATES_DIR))
}

fn ensure_dir(p: &Path) -> std::io::Result<()> {
    if !p.exists() {
        fs::create_dir_all(p)?;
    }
    Ok(())
}

pub fn config_path(template_id: &str) -> Option<PathBuf> {
    templates_dir().map(|d| d.join(format!("{template_id}.json")))
}

pub fn archive_dir(template_id: &str) -> Option<PathBuf> {
    templates_dir().map(|d| d.join(template_id))
}

/// 读取当前覆盖配置；不存在返回 None
pub fn read_config(template_id: &str) -> Option<Value> {
    let path = config_path(template_id)?;
    let bytes = fs::read(&path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveResult {
    pub config_path: String,
    pub archive_path: String,
}

/// 保存覆盖配置 + 归档；返回保存路径。
/// max_archives 为 None 时使用默认 10。
pub fn save_config(
    template_id: &str,
    config: &Value,
    max_archives: Option<usize>,
    timestamp: &str,
) -> Result<SaveResult, String> {
    let dir = templates_dir().ok_or("无法解析数据目录")?;
    ensure_dir(&dir).map_err(|e| e.to_string())?;
    let cfg_path = dir.join(format!("{template_id}.json"));

    let pretty = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&cfg_path, &pretty).map_err(|e| e.to_string())?;

    let arch_dir = dir.join(template_id);
    ensure_dir(&arch_dir).map_err(|e| e.to_string())?;
    let arch_path = arch_dir.join(format!("{timestamp}.json"));
    fs::write(&arch_path, &pretty).map_err(|e| e.to_string())?;

    let max = max_archives.unwrap_or(10);
    prune_archives(&arch_dir, max).ok();

    Ok(SaveResult {
        config_path: cfg_path.display().to_string(),
        archive_path: arch_path.display().to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveInfo {
    pub id: String,
    pub saved_at: String,
    pub size: u64,
}

pub fn list_archives(template_id: &str) -> Vec<ArchiveInfo> {
    let Some(dir) = archive_dir(template_id) else {
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
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let meta = entry.metadata().ok();
        out.push(ArchiveInfo {
            saved_at: stem.clone(),
            id: stem,
            size: meta.map(|m| m.len()).unwrap_or(0),
        });
    }
    // 按 id（时间戳）降序
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out
}

fn prune_archives(dir: &Path, max: usize) -> std::io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
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

pub fn restore_archive(template_id: &str, archive_id: &str) -> Result<(), String> {
    let arch = archive_dir(template_id)
        .ok_or("无法解析归档目录")?
        .join(format!("{archive_id}.json"));
    let bytes = fs::read(&arch).map_err(|e| format!("读取归档失败：{e}"))?;
    let cfg = config_path(template_id).ok_or("无法解析配置路径")?;
    fs::write(&cfg, &bytes).map_err(|e| format!("写入配置失败：{e}"))?;
    Ok(())
}

/// 浅合并：用 override 的字段覆盖 default 的同名字段。
/// 对 fields 数组：按 key 匹配深合并；其他键 override 全替换。
pub fn merge_letter_config(default: Value, ov: Option<Value>) -> Value {
    let Some(ov) = ov else { return default };
    let (mut def_obj, ov_obj) = match (default, ov) {
        (Value::Object(d), Value::Object(o)) => (d, o),
        (d, _) => return d,
    };

    // fields 数组按 key 深合并
    if let (Some(Value::Array(def_fields)), Some(Value::Array(ov_fields))) =
        (def_obj.get("fields").cloned(), ov_obj.get("fields"))
    {
        let mut merged: Vec<Value> = def_fields;
        for ov_field in ov_fields {
            let Some(key) = ov_field.get("key").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Some(idx) = merged
                .iter()
                .position(|f| f.get("key").and_then(|v| v.as_str()) == Some(key))
            {
                if let (Value::Object(orig), Value::Object(patch)) = (&mut merged[idx], ov_field) {
                    for (k, v) in patch {
                        orig.insert(k.clone(), v.clone());
                    }
                }
            } else {
                merged.push(ov_field.clone());
            }
        }
        def_obj.insert("fields".to_string(), Value::Array(merged));
    }

    // 其他顶层键直接覆盖
    for (k, v) in ov_obj {
        if k == "fields" {
            continue;
        }
        def_obj.insert(k, v);
    }
    Value::Object(def_obj)
}

/// dictionaries 合并：以 override 为准（每个字典整体替换）
pub fn merge_dictionaries(default: Value, ov: Option<Value>) -> Value {
    let Some(ov) = ov else { return default };
    let (mut def_obj, ov_obj) = match (default, ov) {
        (Value::Object(d), Value::Object(o)) => (d, o),
        (d, _) => return d,
    };
    for (k, v) in ov_obj {
        def_obj.insert(k, v);
    }
    Value::Object(def_obj)
}

/// 模板启用状态：未删除模板返回 true，删除返回 false
pub fn is_enabled(template_id: &str) -> bool {
    let Some(dir) = templates_dir() else {
        return true;
    };
    let path = dir.join("enabled.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return true;
    };
    let Ok(map): Result<serde_json::Map<String, Value>, _> = serde_json::from_slice(&bytes) else {
        return true;
    };
    map.get(template_id)
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

pub fn set_enabled(template_id: &str, enabled: bool) -> Result<(), String> {
    let dir = templates_dir().ok_or("无法解析数据目录")?;
    ensure_dir(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("enabled.json");
    let mut map: serde_json::Map<String, Value> = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    map.insert(template_id.to_string(), Value::Bool(enabled));
    let pretty = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
    std::fs::write(&path, pretty).map_err(|e| e.to_string())
}
