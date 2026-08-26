use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct SaveMediaSessionArgs {
    #[serde(default)]
    pub id: Option<String>,
    pub module_id: String,
    pub source_path: String,
    pub items: Vec<SaveMediaItem>,
    #[serde(default)]
    pub settings: Value,
}

#[derive(Debug, Deserialize)]
pub struct SaveMediaItem {
    pub path: String,
    #[serde(default)]
    pub engine_decision: Option<String>,
    #[serde(default)]
    pub user_decision: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub metrics: Value,
    #[serde(default)]
    pub order_index: usize,
}

#[derive(Debug, Serialize)]
pub struct SavedMediaSession {
    pub id: String,
    pub source_key: String,
    pub file_count: usize,
    pub total_size: u64,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct FindMediaSessionArgs {
    pub module_id: String,
    pub source_path: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MediaSessionMatch {
    pub found: bool,
    pub session_id: Option<String>,
    pub match_kind: String,
    pub source_path: String,
    pub updated_at: Option<String>,
    pub settings: Value,
    pub matched: usize,
    pub moved_or_renamed: usize,
    pub added: usize,
    pub missing: usize,
    pub changed: usize,
    pub items: Vec<RestoredMediaItem>,
}

#[derive(Debug, Serialize)]
pub struct RestoredMediaItem {
    pub path: String,
    pub previous_path: Option<String>,
    pub match_status: String,
    pub engine_decision: Option<String>,
    pub user_decision: Option<String>,
    pub reason: Option<String>,
    pub metrics: Value,
    pub order_index: usize,
}

#[derive(Debug, Clone)]
struct FileFingerprint {
    canonical_path: String,
    relative_path: String,
    file_name: String,
    file_size: u64,
    modified_ms: i64,
    content_hash: String,
    width: u32,
    height: u32,
}

#[derive(Debug)]
struct PreviousItem {
    canonical_path: String,
    content_hash: String,
    engine_decision: Option<String>,
    user_decision: Option<String>,
    reason: Option<String>,
    metrics: Value,
    order_index: usize,
}

pub fn get_preference(scope: &str) -> Result<Option<Value>> {
    let conn = open_db()?;
    init_db(&conn)?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value_json FROM workspace_preferences WHERE scope = ?1",
            params![scope],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(|text| serde_json::from_str(&text).context("工作区设置数据损坏"))
        .transpose()
}

pub fn set_preference(scope: &str, value: &Value) -> Result<()> {
    let conn = open_db()?;
    init_db(&conn)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO workspace_preferences(scope, value_json, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(scope) DO UPDATE SET
           value_json = excluded.value_json,
           updated_at = excluded.updated_at",
        params![scope, serde_json::to_string(value)?, now],
    )?;
    Ok(())
}

pub fn save_media_session(args: SaveMediaSessionArgs) -> Result<SavedMediaSession> {
    let mut conn = open_db()?;
    init_db(&conn)?;
    let source = canonical_string(Path::new(&args.source_path));
    let paths = args
        .items
        .iter()
        .map(|item| item.path.clone())
        .collect::<Vec<_>>();
    let fingerprints = fingerprints_for_paths(&conn, &source, &paths)?;
    let by_path = fingerprints
        .iter()
        .map(|item| (item.canonical_path.clone(), item))
        .collect::<HashMap<_, _>>();
    let source_key = source_key(&fingerprints);
    let total_size = fingerprints.iter().map(|item| item.file_size).sum::<u64>();
    let now = chrono::Utc::now().to_rfc3339();
    let id = args
        .id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("media-{}", chrono::Utc::now().timestamp_millis()));
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO workspace_sessions(
           id, module_id, source_path, source_key, file_count, total_size,
           settings_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(id) DO UPDATE SET
           module_id = excluded.module_id,
           source_path = excluded.source_path,
           source_key = excluded.source_key,
           file_count = excluded.file_count,
           total_size = excluded.total_size,
           settings_json = excluded.settings_json,
           updated_at = excluded.updated_at",
        params![
            id,
            args.module_id,
            source,
            source_key,
            fingerprints.len() as i64,
            total_size as i64,
            serde_json::to_string(&args.settings)?,
            now,
        ],
    )?;
    tx.execute(
        "DELETE FROM media_session_items WHERE session_id = ?1",
        params![id],
    )?;
    for item in args.items {
        let canonical = canonical_string(Path::new(&item.path));
        let Some(fingerprint) = by_path.get(&canonical) else {
            continue;
        };
        tx.execute(
            "INSERT INTO media_session_items(
               session_id, canonical_path, relative_path, file_name, file_size,
               modified_ms, content_hash, width, height, engine_decision,
               user_decision, reason, metrics_json, order_index
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id,
                fingerprint.canonical_path,
                fingerprint.relative_path,
                fingerprint.file_name,
                fingerprint.file_size as i64,
                fingerprint.modified_ms,
                fingerprint.content_hash,
                fingerprint.width as i64,
                fingerprint.height as i64,
                item.engine_decision,
                item.user_decision,
                item.reason,
                serde_json::to_string(&item.metrics)?,
                item.order_index as i64,
            ],
        )?;
    }
    tx.commit()?;
    Ok(SavedMediaSession {
        id,
        source_key,
        file_count: fingerprints.len(),
        total_size,
        updated_at: now,
    })
}

pub fn find_media_session(args: FindMediaSessionArgs) -> Result<MediaSessionMatch> {
    let conn = open_db()?;
    init_db(&conn)?;
    let source = canonical_string(Path::new(&args.source_path));
    let current = fingerprints_for_paths(&conn, &source, &args.paths)?;
    let key = source_key(&current);
    let candidate = conn
        .query_row(
            "SELECT id, source_path, source_key, settings_json, updated_at
             FROM workspace_sessions
             WHERE module_id = ?1 AND source_path = ?2
             ORDER BY updated_at DESC LIMIT 1",
            params![args.module_id, source],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    let candidate = if candidate.is_some() {
        candidate
    } else {
        conn.query_row(
            "SELECT id, source_path, source_key, settings_json, updated_at
                 FROM workspace_sessions
                 WHERE module_id = ?1 AND source_key = ?2
                 ORDER BY updated_at DESC LIMIT 1",
            params![args.module_id, key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
    };
    let Some((session_id, previous_source, previous_key, settings_json, updated_at)) = candidate
    else {
        return Ok(MediaSessionMatch {
            found: false,
            session_id: None,
            match_kind: "none".into(),
            source_path: source,
            updated_at: None,
            settings: Value::Null,
            matched: 0,
            moved_or_renamed: 0,
            added: current.len(),
            missing: 0,
            changed: 0,
            items: current
                .into_iter()
                .enumerate()
                .map(|(index, item)| RestoredMediaItem {
                    path: item.canonical_path,
                    previous_path: None,
                    match_status: "added".into(),
                    engine_decision: None,
                    user_decision: None,
                    reason: None,
                    metrics: Value::Null,
                    order_index: index,
                })
                .collect(),
        });
    };
    let previous = load_previous_items(&conn, &session_id)?;
    let mut previous_by_path = HashMap::new();
    let mut previous_by_hash: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, item) in previous.iter().enumerate() {
        previous_by_path.insert(item.canonical_path.clone(), index);
        previous_by_hash
            .entry(item.content_hash.clone())
            .or_default()
            .push(index);
    }
    let mut used = HashSet::new();
    let mut restored = Vec::with_capacity(current.len());
    let mut matched = 0;
    let mut moved_or_renamed = 0;
    let mut changed = 0;
    let mut added = 0;
    for (current_index, file) in current.iter().enumerate() {
        let path_match = previous_by_path.get(&file.canonical_path).copied();
        let (previous_index, status) = if let Some(index) = path_match {
            if previous[index].content_hash == file.content_hash {
                (Some(index), "matched")
            } else {
                (Some(index), "changed")
            }
        } else {
            let hash_match = previous_by_hash
                .get(&file.content_hash)
                .and_then(|indices| indices.iter().copied().find(|index| !used.contains(index)));
            if hash_match.is_some() {
                (hash_match, "moved_or_renamed")
            } else {
                (None, "added")
            }
        };
        let previous_item = previous_index.map(|index| {
            used.insert(index);
            &previous[index]
        });
        match status {
            "matched" => matched += 1,
            "moved_or_renamed" => moved_or_renamed += 1,
            "changed" => changed += 1,
            _ => added += 1,
        }
        let can_restore = status == "matched" || status == "moved_or_renamed";
        restored.push(RestoredMediaItem {
            path: file.canonical_path.clone(),
            previous_path: previous_item.map(|item| item.canonical_path.clone()),
            match_status: status.into(),
            engine_decision: previous_item
                .filter(|_| can_restore)
                .and_then(|item| item.engine_decision.clone()),
            user_decision: previous_item
                .filter(|_| can_restore)
                .and_then(|item| item.user_decision.clone()),
            reason: previous_item
                .filter(|_| can_restore)
                .and_then(|item| item.reason.clone()),
            metrics: previous_item
                .filter(|_| can_restore)
                .map(|item| item.metrics.clone())
                .unwrap_or(Value::Null),
            order_index: previous_item
                .filter(|_| can_restore)
                .map(|item| item.order_index)
                .unwrap_or(current_index),
        });
    }
    let missing = previous.len().saturating_sub(used.len());
    restored.sort_by_key(|item| item.order_index);
    let match_kind = if previous_key == key && missing == 0 && added == 0 && changed == 0 {
        if previous_source == source {
            "exact"
        } else {
            "content_exact"
        }
    } else {
        "partial"
    };
    Ok(MediaSessionMatch {
        found: true,
        session_id: Some(session_id),
        match_kind: match_kind.into(),
        source_path: source,
        updated_at: Some(updated_at),
        settings: serde_json::from_str(&settings_json).unwrap_or(Value::Null),
        matched,
        moved_or_renamed,
        added,
        missing,
        changed,
        items: restored,
    })
}

fn load_previous_items(conn: &Connection, session_id: &str) -> Result<Vec<PreviousItem>> {
    let mut statement = conn.prepare(
        "SELECT canonical_path, content_hash, engine_decision, user_decision,
                reason, metrics_json, order_index
         FROM media_session_items WHERE session_id = ?1 ORDER BY order_index",
    )?;
    let rows = statement.query_map(params![session_id], |row| {
        let metrics_json: String = row.get(5)?;
        Ok(PreviousItem {
            canonical_path: row.get(0)?,
            content_hash: row.get(1)?,
            engine_decision: row.get(2)?,
            user_decision: row.get(3)?,
            reason: row.get(4)?,
            metrics: serde_json::from_str(&metrics_json).unwrap_or(Value::Null),
            order_index: row.get::<_, i64>(6)?.max(0) as usize,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn fingerprints_for_paths(
    conn: &Connection,
    source: &str,
    paths: &[String],
) -> Result<Vec<FileFingerprint>> {
    let source_path = Path::new(source);
    let mut result = Vec::new();
    for raw in paths {
        let path = Path::new(raw);
        if !path.is_file() {
            continue;
        }
        let canonical_path = canonical_string(path);
        let canonical = Path::new(&canonical_path);
        let metadata = std::fs::metadata(canonical)?;
        let file_size = metadata.len();
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0);
        let cached = conn
            .query_row(
                "SELECT content_hash, width, height FROM media_file_cache
                 WHERE canonical_path = ?1 AND file_size = ?2 AND modified_ms = ?3",
                params![canonical_path, file_size as i64, modified_ms],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)? as u32,
                        row.get::<_, i64>(2)? as u32,
                    ))
                },
            )
            .optional()?;
        let (content_hash, width, height) = if let Some(cached) = cached {
            cached
        } else {
            let hash = hash_file(canonical)?;
            let (width, height) = image::image_dimensions(canonical).unwrap_or((0, 0));
            conn.execute(
                "INSERT INTO media_file_cache(
                   canonical_path, file_size, modified_ms, content_hash, width, height, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(canonical_path) DO UPDATE SET
                   file_size = excluded.file_size,
                   modified_ms = excluded.modified_ms,
                   content_hash = excluded.content_hash,
                   width = excluded.width,
                   height = excluded.height,
                   updated_at = excluded.updated_at",
                params![
                    canonical_path,
                    file_size as i64,
                    modified_ms,
                    hash,
                    width as i64,
                    height as i64,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )?;
            (hash, width, height)
        };
        let relative_path = canonical
            .strip_prefix(source_path)
            .unwrap_or(canonical)
            .to_string_lossy()
            .to_string();
        let file_name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        result.push(FileFingerprint {
            canonical_path,
            relative_path,
            file_name,
            file_size,
            modified_ms,
            content_hash,
            width,
            height,
        });
    }
    Ok(result)
}

fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("无法读取文件: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn source_key(items: &[FileFingerprint]) -> String {
    let mut hashes = items
        .iter()
        .map(|item| item.content_hash.as_str())
        .collect::<Vec<_>>();
    hashes.sort_unstable();
    let mut hasher = Sha256::new();
    for hash in hashes {
        hasher.update(hash.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn canonical_string(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn open_db() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
    )?;
    Ok(conn)
}

fn db_path() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("Docsy")
        .join("workspace_state.sqlite3")
}

fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS workspace_preferences (
          scope TEXT PRIMARY KEY,
          value_json TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS workspace_sessions (
          id TEXT PRIMARY KEY,
          module_id TEXT NOT NULL,
          source_path TEXT NOT NULL,
          source_key TEXT NOT NULL,
          file_count INTEGER NOT NULL,
          total_size INTEGER NOT NULL,
          settings_json TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS media_session_items (
          session_id TEXT NOT NULL,
          canonical_path TEXT NOT NULL,
          relative_path TEXT NOT NULL,
          file_name TEXT NOT NULL,
          file_size INTEGER NOT NULL,
          modified_ms INTEGER NOT NULL,
          content_hash TEXT NOT NULL,
          width INTEGER NOT NULL,
          height INTEGER NOT NULL,
          engine_decision TEXT,
          user_decision TEXT,
          reason TEXT,
          metrics_json TEXT NOT NULL DEFAULT '{}',
          order_index INTEGER NOT NULL,
          PRIMARY KEY(session_id, canonical_path),
          FOREIGN KEY(session_id) REFERENCES workspace_sessions(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS media_file_cache (
          canonical_path TEXT PRIMARY KEY,
          file_size INTEGER NOT NULL,
          modified_ms INTEGER NOT NULL,
          content_hash TEXT NOT NULL,
          width INTEGER NOT NULL,
          height INTEGER NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workspace_sessions_source
          ON workspace_sessions(module_id, source_path, updated_at);
        CREATE INDEX IF NOT EXISTS idx_workspace_sessions_key
          ON workspace_sessions(module_id, source_key, updated_at);
        CREATE INDEX IF NOT EXISTS idx_media_session_hash
          ON media_session_items(session_id, content_hash);
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_key_ignores_order() {
        let item = |path: &str, hash: &str| FileFingerprint {
            canonical_path: path.into(),
            relative_path: path.into(),
            file_name: path.into(),
            file_size: 1,
            modified_ms: 0,
            content_hash: hash.into(),
            width: 1,
            height: 1,
        };
        assert_eq!(
            source_key(&[item("a", "1"), item("b", "2")]),
            source_key(&[item("b", "2"), item("a", "1")])
        );
    }
}
