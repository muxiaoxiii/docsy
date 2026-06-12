use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub history_max: usize,
    pub menu_visibility: std::collections::HashMap<String, bool>,
    pub libreoffice_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            history_max: 50,
            menu_visibility: std::collections::HashMap::new(),
            libreoffice_path: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerationRecord {
    pub id: String,
    pub template_id: String,
    pub timestamp: String,
    pub values: serde_json::Value,
    pub output_path: Option<String>,
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveRecordArgs {
    pub template_id: String,
    pub values: serde_json::Value,
    pub output_path: Option<String>,
    pub label: Option<String>,
}

use std::sync::Mutex;
use std::sync::OnceLock;
static DB: OnceLock<Mutex<Option<rusqlite::Connection>>> = OnceLock::new();

pub fn get_db() -> Result<&'static Mutex<Option<rusqlite::Connection>>> {
    let m = DB.get_or_init(|| Mutex::new(None));
    {
        let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        if guard.is_some() {
            return Ok(m);
        }
    }
    // Initialize
    let db_path = data_dir().join("docsy.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = rusqlite::Connection::open(&db_path)?;
    init_schema(&conn)?;
    let mut guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    *guard = Some(conn);
    Ok(m)
}

fn init_schema(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS global_dictionaries (
            dict_name       TEXT NOT NULL,
            entry_key       TEXT NOT NULL,
            label           TEXT NOT NULL,
            pinyin          TEXT,
            extra_json      TEXT,
            frequency       INTEGER NOT NULL DEFAULT 0,
            last_used_at    TEXT,
            created_at      TEXT NOT NULL,
            PRIMARY KEY (dict_name, entry_key)
        );

        CREATE TABLE IF NOT EXISTS template_dictionaries (
            template_id     TEXT NOT NULL,
            dict_name       TEXT NOT NULL,
            entry_key       TEXT NOT NULL,
            label           TEXT NOT NULL,
            pinyin          TEXT,
            extra_json      TEXT,
            frequency       INTEGER NOT NULL DEFAULT 0,
            last_used_at    TEXT,
            PRIMARY KEY (template_id, dict_name, entry_key)
        );

        CREATE TABLE IF NOT EXISTS field_history (
            template_id     TEXT NOT NULL,
            field_key       TEXT NOT NULL,
            value_json      TEXT NOT NULL,
            frequency       INTEGER NOT NULL DEFAULT 1,
            last_used_at    TEXT NOT NULL,
            PRIMARY KEY (template_id, field_key, value_json)
        );

        CREATE TABLE IF NOT EXISTS parties (
            id              TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            subject_type    TEXT NOT NULL,
            aliases_json    TEXT,
            frequency       INTEGER NOT NULL DEFAULT 0,
            last_used_at    TEXT,
            created_at      TEXT NOT NULL,
            UNIQUE(name, subject_type)
        );

        CREATE TABLE IF NOT EXISTS generation_records (
            id              TEXT PRIMARY KEY,
            template_id     TEXT NOT NULL,
            values_json     TEXT NOT NULL,
            output_path     TEXT,
            label           TEXT NOT NULL,
            created_at      TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS template_meta (
            id              TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            builtin         INTEGER NOT NULL DEFAULT 0,
            pinned_to_tab   INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

pub fn get_settings() -> Result<AppSettings> {
    let path = data_dir().join("settings.json");
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(AppSettings::default())
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let path = data_dir().join("settings.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn save_record(args: SaveRecordArgs) -> Result<GenerationRecord> {
    let m = get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let values_json = serde_json::to_string(&args.values)?;
    let label = args.label.unwrap_or_else(|| format!("记录 {}", now));

    db.execute(
        "INSERT INTO generation_records (id, template_id, values_json, output_path, label, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, args.template_id, values_json, args.output_path, label, now],
    )?;

    Ok(GenerationRecord {
        id,
        template_id: args.template_id,
        timestamp: now,
        values: args.values,
        output_path: args.output_path,
        label,
    })
}

pub fn list_records(template_id: &str) -> Result<Vec<GenerationRecord>> {
    let m = get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;
    let mut stmt = db.prepare(
        "SELECT id, template_id, values_json, output_path, label, created_at
         FROM generation_records WHERE template_id = ?1 ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map(rusqlite::params![template_id], |row| {
        Ok(GenerationRecord {
            id: row.get(0)?,
            template_id: row.get(1)?,
            timestamp: row.get(5)?,
            values: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
            output_path: row.get(3)?,
            label: row.get(4)?,
        })
    })?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn read_record(template_id: &str, record_id: &str) -> Result<GenerationRecord> {
    let m = get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;
    db.query_row(
        "SELECT id, template_id, values_json, output_path, label, created_at
         FROM generation_records WHERE template_id = ?1 AND id = ?2",
        rusqlite::params![template_id, record_id],
        |row| {
            Ok(GenerationRecord {
                id: row.get(0)?,
                template_id: row.get(1)?,
                timestamp: row.get(5)?,
                values: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                output_path: row.get(3)?,
                label: row.get(4)?,
            })
        },
    ).map_err(|e| anyhow::anyhow!("{}", e))
}

pub fn delete_record(template_id: &str, record_id: &str) -> Result<()> {
    let m = get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;
    db.execute(
        "DELETE FROM generation_records WHERE template_id = ?1 AND id = ?2",
        rusqlite::params![template_id, record_id],
    )?;
    Ok(())
}

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Docsy")
}
