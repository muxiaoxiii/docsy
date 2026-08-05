use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::docx_template::{TemplateField, TemplateManifest};

const TEMPLATE_COMMON_ID: &str = "__template_common__";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateHistoryContext {
    pub last_values: HashMap<String, Value>,
    pub field_suggestions: HashMap<String, Vec<ValueSuggestion>>,
    pub semantic_suggestions: HashMap<String, Vec<ValueSuggestion>>,
    pub association_suggestions: HashMap<String, Vec<AssociationSuggestion>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ValueSuggestion {
    pub value: Value,
    pub display: String,
    pub count: u32,
    pub last_used: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssociationSuggestion {
    pub value: Value,
    pub display: String,
    pub count: u32,
    pub trigger_field: String,
    pub trigger_value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateHistoryRun {
    pub id: i64,
    pub template_id: String,
    pub template_name: String,
    pub template_path: String,
    pub output_path: String,
    pub generated_at: String,
    pub field_values: HashMap<String, Value>,
    pub field_summaries: Vec<TemplateHistoryFieldSummary>,
    /// "single" | "batch" | "seed" — where the run came from
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateHistoryFieldSummary {
    pub name: String,
    pub label: String,
    pub display: String,
}

#[allow(dead_code)]
pub fn record_generation(
    template_path: &str,
    manifest: &TemplateManifest,
    output_path: &str,
    values: &HashMap<String, Value>,
) -> Result<()> {
    record_history_run(template_path, manifest, output_path, values, "single")
}

pub fn record_template_seed(
    template_path: &str,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    record_history_run(template_path, manifest, "[template-seed]", values, "seed")
}

/// Delete every recorded fill run and its field values (history database only;
/// templates and their files are untouched). Returns the number of deleted runs.
pub fn clear_history() -> Result<usize> {
    let conn = open_db()?;
    init_db(&conn)?;
    let deleted = conn.execute("DELETE FROM generation_runs", [])?;
    conn.execute("DELETE FROM field_history", [])?;
    Ok(deleted)
}

/// Copy field-history rows of `source_template_id` whose field name also
/// exists in `target_template_id` into the target (skipping rows the target
/// already has for the same field+value). Rows only present in the source
/// stay there — the cross-template field-name lookup surfaces them anyway.
/// Returns the number of copied rows.
pub fn merge_template_field_history(
    source_template_id: &str,
    target_template_id: &str,
) -> Result<usize> {
    let conn = open_db()?;
    init_db(&conn)?;
    let mut stmt = conn.prepare(
        "INSERT INTO field_history
            (run_id, template_id, field_id, field_name, field_label, semantic_key, value_json, display_value, generated_at)
         SELECT run_id, ?2, field_id, field_name, field_label, semantic_key, value_json, display_value, generated_at
         FROM field_history AS src
         WHERE src.template_id = ?1
           AND src.field_name IN (SELECT DISTINCT field_name FROM field_history WHERE template_id = ?2)
           AND NOT EXISTS (
             SELECT 1 FROM field_history AS tgt
             WHERE tgt.template_id = ?2
               AND tgt.field_name = src.field_name
               AND tgt.value_json = src.value_json
               AND tgt.display_value = src.display_value
           )",
    )?;
    let copied = stmt.execute(params![source_template_id, target_template_id])?;
    Ok(copied)
}

pub fn list_database_entries() -> Result<Vec<Value>> {
    let conn = open_db()?;
    init_db(&conn)?;
    let mut stmt = conn.prepare(
        "SELECT m.template_id, m.template_name, m.updated_at,
                (SELECT COUNT(*) FROM field_history fh WHERE fh.template_id = m.template_id) as field_count
         FROM template_meta m WHERE m.trashed = 0 ORDER BY m.updated_at DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "templateId": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "updatedAt": row.get::<_, String>(2)?,
                "fieldCount": row.get::<_, i64>(3)?,
            }))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn delete_database_entry(template_path: &str) -> Result<()> {
    let conn = open_db()?;
    init_db(&conn)?;
    conn.execute(
        "DELETE FROM field_history WHERE template_id = (SELECT template_id FROM template_meta WHERE template_id = ?1)",
        [template_path],
    )?;
    conn.execute(
        "DELETE FROM generation_runs WHERE template_id = (SELECT template_id FROM template_meta WHERE template_id = ?1)",
        [template_path],
    )?;
    conn.execute("DELETE FROM template_meta WHERE template_id = ?1", [template_path])?;
    Ok(())
}

pub fn record_history_run(
    template_path: &str,
    manifest: &TemplateManifest,
    output_path: &str,
    values: &HashMap<String, Value>,
    source: &str,
) -> Result<()> {
    let mut conn = open_db()?;
    init_db(&conn)?;
    // 不改变 trashed 状态：已回收站的模板不应因历史记录写入而被恢复
    ensure_template_meta(&conn, manifest, template_path)?;
    let now = chrono::Utc::now().to_rfc3339();
    let stored_values = canonical_history_values(manifest, values);
    let values_json = serde_json::to_string(&stored_values)?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO generation_runs (template_id, template_name, template_path, output_path, generated_at, field_values, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            manifest.template.id,
            manifest.template.name,
            template_path,
            output_path,
            now,
            values_json,
            source
        ],
    )?;
    let run_id = tx.last_insert_rowid();
    for field in &manifest.fields {
        let Some(value) = stored_values
            .get(&field.id)
            .or_else(|| stored_values.get(&field.name))
        else {
            continue;
        };
        for atom in flatten_field_values(field, value) {
            if atom.display.trim().is_empty() {
                continue;
            }
            tx.execute(
                "INSERT INTO field_history (run_id, template_id, field_id, field_name, field_label, semantic_key, value_json, display_value, generated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    run_id,
                    manifest.template.id,
                    field.id,
                    field.name,
                    field.label,
                    field.semantic_key,
                    serde_json::to_string(&atom.value)?,
                    atom.display,
                    now
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn canonical_history_values(
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut stored = HashMap::new();
    let mut name_counts = HashMap::<&str, usize>::new();
    for field in &manifest.fields {
        *name_counts.entry(field.name.as_str()).or_default() += 1;
    }
    for field in &manifest.fields {
        let Some(value) = values.get(&field.id).or_else(|| values.get(&field.name)) else {
            continue;
        };
        stored.insert(field.id.clone(), value.clone());
        if !field.name.trim().is_empty() && name_counts.get(field.name.as_str()) == Some(&1) {
            stored.insert(field.name.clone(), value.clone());
        }
    }
    stored
}

pub fn history_context(
    manifest: &TemplateManifest,
    current_values: Option<&HashMap<String, Value>>,
    full_refresh: bool,
) -> Result<TemplateHistoryContext> {
    let conn = open_db()?;
    init_db(&conn)?;
    let mut field_suggestions = HashMap::new();
    let mut semantic_suggestions = HashMap::new();
    let mut association_suggestions = HashMap::new();

    if full_refresh || current_values.is_none() {
        for field in &manifest.fields {
            field_suggestions.insert(
                field.id.clone(),
                query_field_suggestions(&conn, &manifest.template.id, field)?,
            );
            if !field.semantic_key.trim().is_empty() {
                semantic_suggestions.insert(
                    field.id.clone(),
                    query_semantic_suggestions(&conn, &manifest.template.id, field)?,
                );
            }
        }
    }

    if let Some(values) = current_values {
        for target in &manifest.fields {
            let suggestions =
                query_association_suggestions(&conn, &manifest.template.id, target, values)?;
            if !suggestions.is_empty() {
                association_suggestions.insert(target.id.clone(), suggestions);
            }
        }
    }

    Ok(TemplateHistoryContext {
        last_values: query_last_values(&conn, &manifest.template.id)?,
        field_suggestions,
        semantic_suggestions,
        association_suggestions,
    })
}

pub fn list_generation_runs(limit: usize) -> Result<Vec<TemplateHistoryRun>> {
    let conn = open_db()?;
    init_db(&conn)?;
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn.prepare(
        "SELECT runs.id,
                runs.template_id,
                runs.template_name,
                runs.template_path,
                runs.output_path,
                runs.generated_at,
                runs.field_values,
                runs.source
         FROM generation_runs AS runs
         JOIN template_meta AS meta ON meta.template_id = runs.template_id
         WHERE runs.output_path != '[template-seed]'
           AND meta.trashed = 0
         ORDER BY runs.generated_at DESC, runs.id DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        let values_json: String = row.get(6)?;
        Ok(TemplateHistoryRun {
            id: row.get(0)?,
            template_id: row.get(1)?,
            template_name: row.get(2)?,
            template_path: row.get(3)?,
            output_path: row.get(4)?,
            generated_at: row.get(5)?,
            field_values: serde_json::from_str(&values_json).unwrap_or_default(),
            field_summaries: Vec::new(),
            source: row.get(7).unwrap_or_else(|_| "single".to_string()),
        })
    })?;

    let all_runs: Vec<TemplateHistoryRun> = collect_rows(rows)?;

    // 自动清理：模板文件已不存在于磁盘的记录标记为已删除
    let mut orphaned_ids = Vec::new();
    let mut valid_runs = Vec::new();
    for run in all_runs {
        if !run.template_path.is_empty() && !std::path::Path::new(&run.template_path).exists() {
            orphaned_ids.push(run.template_id.clone());
        } else {
            valid_runs.push(run);
        }
    }
    if !orphaned_ids.is_empty() {
        for template_id in &orphaned_ids {
            let _ = conn.execute(
                "UPDATE template_meta SET trashed = 1, updated_at = ?2 WHERE template_id = ?1 AND trashed = 0",
                params![template_id, chrono::Utc::now().to_rfc3339()],
            );
        }
    }

    let mut runs = valid_runs;
    // Batch-load summaries for all runs with a single query (avoids N+1).
    let run_ids: Vec<i64> = runs.iter().map(|run| run.id).collect();
    let summaries_by_run = query_run_field_summaries_for_runs(&conn, &run_ids)?;
    for run in &mut runs {
        run.field_summaries = summaries_by_run.get(&run.id).cloned().unwrap_or_default();
    }
    Ok(runs)
}

fn query_run_field_summaries_for_runs(
    conn: &Connection,
    run_ids: &[i64],
) -> Result<std::collections::HashMap<i64, Vec<TemplateHistoryFieldSummary>>> {
    let mut result = std::collections::HashMap::new();
    if run_ids.is_empty() {
        return Ok(result);
    }
    let placeholders = run_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT run_id, field_name, field_label, display_value
         FROM field_history
         WHERE run_id IN ({placeholders})
         ORDER BY run_id ASC, id ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(run_ids.iter());
    let rows = stmt.query_map(params, |row| {
        Ok((
            row.get::<_, i64>(0)?,
            TemplateHistoryFieldSummary {
                name: row.get(1)?,
                label: row.get(2)?,
                display: row.get(3)?,
            },
        ))
    })?;
    for item in rows {
        let (run_id, summary) = item?;
        result.entry(run_id).or_insert_with(Vec::new).push(summary);
    }
    Ok(result)
}

/// Point all history records of a template at its current library path
/// (used after restoring a template from trash, where the path may change).
pub fn update_template_path(template_id: &str, new_path: &str) -> Result<()> {
    let conn = open_db()?;
    init_db(&conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE generation_runs SET template_path = ?2 WHERE template_id = ?1",
        params![template_id, new_path],
    )?;
    conn.execute(
        "UPDATE template_meta SET template_path = ?2, updated_at = ?3 WHERE template_id = ?1",
        params![template_id, new_path, now],
    )?;
    Ok(())
}

pub fn mark_template_trashed(template_id: &str, trashed: bool) -> Result<()> {
    let conn = open_db()?;
    init_db(&conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO template_meta (template_id, template_name, template_path, trashed, updated_at)
         VALUES (?1, '', '', ?2, ?3)
         ON CONFLICT(template_id) DO UPDATE SET trashed = excluded.trashed, updated_at = excluded.updated_at",
        params![template_id, if trashed { 1 } else { 0 }, now],
    )?;
    Ok(())
}

pub fn delete_template_data(template_id: &str) -> Result<()> {
    let mut conn = open_db()?;
    init_db(&conn)?;
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM field_history WHERE template_id = ?1",
        params![template_id],
    )?;
    tx.execute(
        "DELETE FROM generation_runs WHERE template_id = ?1",
        params![template_id],
    )?;
    tx.execute(
        "DELETE FROM template_meta WHERE template_id = ?1",
        params![template_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn migrate_template_data_to_common(template_id: &str) -> Result<()> {
    let mut conn = open_db()?;
    init_db(&conn)?;
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE field_history
         SET template_id = ?2,
             field_label = CASE WHEN field_label = '' THEN field_name ELSE field_label END
         WHERE template_id = ?1",
        params![template_id, TEMPLATE_COMMON_ID],
    )?;
    tx.execute(
        "DELETE FROM generation_runs WHERE template_id = ?1",
        params![template_id],
    )?;
    tx.execute(
        "DELETE FROM template_meta WHERE template_id = ?1",
        params![template_id],
    )?;
    tx.commit()?;
    Ok(())
}

fn open_db() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    // WAL allows concurrent readers (multiple windows) without write blocking;
    // synchronous=NORMAL keeps durability with far fewer fsyncs.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    Ok(conn)
}

fn db_path() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("Docsy")
        .join("template_history.sqlite3")
}

fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS generation_runs (
            id INTEGER PRIMARY KEY,
            template_id TEXT NOT NULL,
            template_name TEXT NOT NULL,
            template_path TEXT NOT NULL,
            output_path TEXT NOT NULL,
            generated_at TEXT NOT NULL,
            field_values TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'single'
        );
        CREATE TABLE IF NOT EXISTS field_history (
            id INTEGER PRIMARY KEY,
            run_id INTEGER NOT NULL,
            template_id TEXT NOT NULL,
            field_id TEXT NOT NULL DEFAULT '',
            field_name TEXT NOT NULL,
            field_label TEXT NOT NULL,
            semantic_key TEXT,
            value_json TEXT NOT NULL,
            display_value TEXT NOT NULL,
            generated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS template_meta (
            template_id TEXT PRIMARY KEY,
            template_name TEXT NOT NULL,
            template_path TEXT NOT NULL,
            trashed INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_template_runs ON generation_runs(template_id, generated_at);
        CREATE INDEX IF NOT EXISTS idx_field_history_template_field ON field_history(template_id, field_id);
        CREATE INDEX IF NOT EXISTS idx_field_history_semantic ON field_history(semantic_key);
        CREATE INDEX IF NOT EXISTS idx_field_history_run ON field_history(run_id);
        ",
    )?;
    ensure_field_history_id_column(conn)?;
    ensure_generation_source_column(conn)?;
    Ok(())
}

fn ensure_field_history_id_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(field_history)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !columns.iter().any(|column| column == "field_id") {
        conn.execute(
            "ALTER TABLE field_history ADD COLUMN field_id TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    conn.execute(
        "UPDATE field_history SET field_id = field_name WHERE field_id = ''",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_field_history_template_field_id ON field_history(template_id, field_id)",
        [],
    )?;
    Ok(())
}

fn ensure_generation_source_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(generation_runs)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !columns.iter().any(|column| column == "source") {
        conn.execute(
            "ALTER TABLE generation_runs ADD COLUMN source TEXT NOT NULL DEFAULT 'single'",
            [],
        )?;
    }
    Ok(())
}

#[allow(dead_code)]
fn upsert_template_meta(
    conn: &Connection,
    manifest: &TemplateManifest,
    template_path: &str,
    trashed: bool,
) -> Result<()> {
    conn.execute(
        "INSERT INTO template_meta (template_id, template_name, template_path, trashed, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(template_id) DO UPDATE SET
           template_name = excluded.template_name,
           template_path = excluded.template_path,
           trashed = excluded.trashed,
           updated_at = excluded.updated_at",
        params![
            manifest.template.id,
            manifest.template.name,
            template_path,
            if trashed { 1 } else { 0 },
            chrono::Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

/// Ensure template_meta exists without changing the trashed flag.
/// Used by history recording so that trashed templates stay trashed.
fn ensure_template_meta(
    conn: &Connection,
    manifest: &TemplateManifest,
    template_path: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    // Single atomic upsert — the former UPDATE-then-INSERT raced under
    // concurrent writers and could hit the PRIMARY KEY conflict.
    conn.execute(
        "INSERT INTO template_meta (template_id, template_name, template_path, trashed, updated_at)
         VALUES (?1, ?2, ?3, 0, ?4)
         ON CONFLICT(template_id) DO UPDATE SET
            template_name = excluded.template_name,
            template_path = excluded.template_path,
            updated_at = excluded.updated_at",
        params![manifest.template.id, manifest.template.name, template_path, now],
    )?;
    Ok(())
}

pub fn query_last_values(conn: &Connection, template_id: &str) -> Result<HashMap<String, Value>> {
    let mut stmt = conn.prepare(
        "SELECT field_values FROM generation_runs
         WHERE template_id = ?1
         ORDER BY generated_at DESC, id DESC
         LIMIT 1",
    )?;
    let result: Result<String, _> = stmt.query_row(params![template_id], |row| row.get(0));
    match result {
        Ok(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(HashMap::new()),
        Err(err) => Err(err.into()),
    }
}

/// Last recorded field values for a template (empty map when none exist yet).
pub fn last_field_values_for_template(template_id: &str) -> Result<HashMap<String, Value>> {
    let conn = open_db()?;
    query_last_values(&conn, template_id)
}

#[allow(dead_code)]
fn query_run_field_summaries(
    conn: &Connection,
    run_id: i64,
) -> Result<Vec<TemplateHistoryFieldSummary>> {
    let mut stmt = conn.prepare(
        "SELECT field_name, field_label, display_value
         FROM field_history
         WHERE run_id = ?1
         ORDER BY id ASC
         LIMIT 8",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        Ok(TemplateHistoryFieldSummary {
            name: row.get(0)?,
            label: row.get(1)?,
            display: row.get(2)?,
        })
    })?;
    collect_rows(rows)
}

fn query_field_suggestions(
    conn: &Connection,
    template_id: &str,
    field: &TemplateField,
) -> Result<Vec<ValueSuggestion>> {
    // Field-name-keyed suggestions span templates: the current template first,
    // then the shared common bucket, then any non-trashed template with the
    // same field name. This lets two templates with similar terms share fill
    // history without any manual merge step.
    let mut stmt = conn.prepare(
        "SELECT value_json, display_value, COUNT(*) AS freq, MAX(generated_at) AS last_used
         FROM field_history
         WHERE field_name = ?2
           AND (
             template_id = ?1
             OR template_id = ?3
             OR NOT EXISTS (
               SELECT 1 FROM template_meta
               WHERE template_meta.template_id = field_history.template_id
                 AND template_meta.trashed = 1
             )
           )
         GROUP BY value_json, display_value
         ORDER BY freq DESC, last_used DESC
         LIMIT 8",
    )?;
    let rows = stmt.query_map(params![template_id, field.name, TEMPLATE_COMMON_ID], |row| {
        suggestion_from_row(row, "field")
    })?;
    collect_rows(rows)
}

fn query_semantic_suggestions(
    conn: &Connection,
    current_template_id: &str,
    field: &TemplateField,
) -> Result<Vec<ValueSuggestion>> {
    let mut stmt = conn.prepare(
        "SELECT value_json, display_value, COUNT(*) AS freq, MAX(generated_at) AS last_used
         FROM field_history
         WHERE semantic_key = ?1 AND semantic_key != ''
           AND template_id != ?2
           AND (
             template_id = ?3
             OR NOT EXISTS (
               SELECT 1 FROM template_meta
               WHERE template_meta.template_id = field_history.template_id
                 AND template_meta.trashed = 1
             )
           )
         GROUP BY value_json, display_value
         ORDER BY freq DESC, last_used DESC
         LIMIT 8",
    )?;
    let rows = stmt.query_map(
        params![field.semantic_key, current_template_id, TEMPLATE_COMMON_ID],
        |row| suggestion_from_row(row, "semantic"),
    )?;
    collect_rows(rows)
}

fn query_association_suggestions(
    conn: &Connection,
    template_id: &str,
    target: &TemplateField,
    values: &HashMap<String, Value>,
) -> Result<Vec<AssociationSuggestion>> {
    let mut suggestions = Vec::new();
    // The frontend injects the same value under id/name/semantic keys; dedupe
    // by display value so one trigger is only queried once.
    let mut seen_triggers = std::collections::HashSet::new();
    for (trigger_field, trigger_value) in values {
        if trigger_field == &target.id {
            continue;
        }
        let trigger_display = value_display(trigger_value);
        if trigger_display.trim().is_empty() {
            continue;
        }
        if !seen_triggers.insert(trigger_display.clone()) {
            continue;
        }
        let mut stmt = conn.prepare(
            "SELECT target.value_json, target.display_value, COUNT(*) AS freq
             FROM field_history AS trigger
             JOIN field_history AS target ON trigger.run_id = target.run_id
             WHERE trigger.template_id = ?1
               AND trigger.field_id = ?2
               AND trigger.display_value = ?3
               AND target.template_id = ?1
               AND target.field_id = ?4
             GROUP BY target.value_json, target.display_value
             ORDER BY freq DESC, MAX(target.generated_at) DESC
             LIMIT 3",
        )?;
        let rows = stmt.query_map(
            params![template_id, trigger_field, trigger_display, target.id],
            |row| {
                let value_json: String = row.get(0)?;
                let display: String = row.get(1)?;
                let count: u32 = row.get::<_, i64>(2)? as u32;
                Ok(AssociationSuggestion {
                    value: serde_json::from_str(&value_json)
                        .unwrap_or(Value::String(display.clone())),
                    display,
                    count,
                    trigger_field: trigger_field.clone(),
                    trigger_value: trigger_display.clone(),
                })
            },
        )?;
        suggestions.extend(collect_rows(rows)?);
    }
    suggestions.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.display.cmp(&b.display))
    });
    suggestions.truncate(5);
    Ok(suggestions)
}

fn suggestion_from_row(row: &rusqlite::Row<'_>, source: &str) -> rusqlite::Result<ValueSuggestion> {
    let value_json: String = row.get(0)?;
    let display: String = row.get(1)?;
    let count: u32 = row.get::<_, i64>(2)? as u32;
    let last_used: String = row.get(3)?;
    Ok(ValueSuggestion {
        value: serde_json::from_str(&value_json).unwrap_or(Value::String(display.clone())),
        display,
        count,
        last_used,
        source: source.into(),
    })
}

fn collect_rows<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

struct FieldAtom {
    value: Value,
    display: String,
}

fn flatten_field_values(field: &TemplateField, value: &Value) -> Vec<FieldAtom> {
    match field.field_type.as_str() {
        "checkbox_group" | "party_list" => match value {
            Value::Array(items) => items
                .iter()
                .map(|item| FieldAtom {
                    value: item.clone(),
                    display: value_display(item),
                })
                .collect(),
            _ => vec![FieldAtom {
                value: value.clone(),
                display: value_display(value),
            }],
        },
        _ => vec![FieldAtom {
            value: value.clone(),
            display: value_display(value),
        }],
    }
}

fn value_display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(value) => {
            if *value {
                "是".into()
            } else {
                "否".into()
            }
        }
        Value::Number(number) => number.to_string(),
        Value::Array(items) => items
            .iter()
            .map(value_display)
            .collect::<Vec<_>>()
            .join("、"),
        Value::Object(map) => map
            .get("name")
            .or_else(|| map.get("label"))
            .map(value_display)
            .unwrap_or_else(|| json!(map).to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docx_template::{
        TemplateField, TemplateFieldReference, TemplateManifest, TemplateMeta,
    };

    #[test]
    fn value_display_handles_party_like_objects() {
        let value = json!({ "name": "张三", "address": "北京" });
        assert_eq!(value_display(&value), "张三");
    }

    #[test]
    fn flattens_party_list_values() {
        let field = TemplateField {
            field_type: "party_list".into(),
            ..Default::default()
        };
        let atoms = flatten_field_values(&field, &json!(["张三", "李四"]));
        assert_eq!(atoms.len(), 2);
        assert_eq!(atoms[1].display, "李四");
    }

    #[test]
    fn canonical_history_keeps_reference_value_separate_from_same_name_field() {
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![
                TemplateField {
                    id: "party".into(),
                    name: "第三人".into(),
                    field_type: "party_list".into(),
                    ..Default::default()
                },
                TemplateField {
                    id: "principal_ref".into(),
                    name: "第三人".into(),
                    field_type: "reference".into(),
                    reference: Some(TemplateFieldReference {
                        source_mode: "semantic".into(),
                        source_semantic_key: "当事人".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
        };
        let values = [
            ("第三人".to_string(), json!(["真实第三人1", "真实第三人2"])),
            ("principal_ref".to_string(), json!("被上诉人A公司")),
            ("当事人".to_string(), json!(["不应保存的语义别名"])),
        ]
        .into_iter()
        .collect();

        let stored = canonical_history_values(&manifest, &values);

        assert_eq!(
            stored.get("party"),
            Some(&json!(["真实第三人1", "真实第三人2"]))
        );
        assert_eq!(stored.get("第三人"), None);
        assert_eq!(stored.get("principal_ref"), Some(&json!("被上诉人A公司")));
        assert_eq!(stored.get("当事人"), None);
    }

    #[test]
    fn context_shape_is_serializable() {
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![],
        };
        let context = TemplateHistoryContext {
            last_values: HashMap::new(),
            field_suggestions: HashMap::new(),
            semantic_suggestions: HashMap::new(),
            association_suggestions: HashMap::new(),
        };
        assert!(serde_json::to_value(context).is_ok());
        assert_eq!(manifest.template.id, "tpl");
    }
}
