use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DictQuery {
    pub dict_name: String,
    pub template_id: Option<String>,
    pub field_key: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DictEntry {
    pub key: String,
    pub label: String,
    pub pinyin: Option<String>,
    pub extra: Option<serde_json::Value>,
    pub source: String,
    pub frequency: u32,
}

#[derive(Debug, Deserialize)]
pub struct RecommendQuery {
    pub template_id: String,
    pub field_key: String,
    pub context: serde_json::Value,
    pub search: Option<String>,
}

pub fn query(query: DictQuery) -> Result<Vec<DictEntry>> {
    let m = crate::services::history::get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;
    let mut entries = Vec::new();

    // 1. global dictionaries
    let mut stmt = db.prepare(
        "SELECT entry_key, label, pinyin, extra_json, frequency FROM global_dictionaries WHERE dict_name = ?1"
    )?;
    let rows = stmt.query_map(rusqlite::params![query.dict_name], |row| {
        Ok(DictEntry {
            key: row.get(0)?,
            label: row.get(1)?,
            pinyin: row.get(2)?,
            extra: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
            source: "global".to_string(),
            frequency: row.get(4)?,
        })
    })?;
    for row in rows {
        entries.push(row?);
    }

    // 2. template dictionaries
    if let Some(tid) = &query.template_id {
        let mut stmt = db.prepare(
            "SELECT entry_key, label, pinyin, extra_json, frequency FROM template_dictionaries WHERE template_id = ?1 AND dict_name = ?2"
        )?;
        let rows = stmt.query_map(rusqlite::params![tid, query.dict_name], |row| {
            Ok(DictEntry {
                key: row.get(0)?,
                label: row.get(1)?,
                pinyin: row.get(2)?,
                extra: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
                source: "template".to_string(),
                frequency: row.get(4)?,
            })
        })?;
        for row in rows {
            entries.push(row?);
        }
    }

    // 3. field history
    if let (Some(tid), Some(fk)) = (&query.template_id, &query.field_key) {
        let mut stmt = db.prepare(
            "SELECT value_json, frequency FROM field_history WHERE template_id = ?1 AND field_key = ?2"
        )?;
        let rows = stmt.query_map(rusqlite::params![tid, fk], |row| {
            let value_json: String = row.get(0)?;
            Ok(DictEntry {
                key: value_json.clone(),
                label: value_json,
                pinyin: None,
                extra: None,
                source: "history".to_string(),
                frequency: row.get(1)?,
            })
        })?;
        for row in rows {
            entries.push(row?);
        }
    }

    // dedup by key, keep highest frequency
    entries.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| seen.insert(e.key.clone()));

    // search filter
    if let Some(search) = &query.search {
        let search_lower = search.to_lowercase();
        entries.retain(|e| {
            e.label.to_lowercase().contains(&search_lower)
                || e.pinyin.as_ref().map_or(false, |p| p.to_lowercase().contains(&search_lower))
        });
    }

    // limit
    if let Some(limit) = query.limit {
        entries.truncate(limit);
    }

    Ok(entries)
}

pub fn recommend(q: RecommendQuery) -> Result<Vec<DictEntry>> {
    // delegate to query with template + field context
    let dict_query = DictQuery {
        dict_name: q.field_key.clone(),
        template_id: Some(q.template_id),
        field_key: Some(q.field_key),
        search: q.search,
        limit: Some(20),
    };
    query(dict_query)
}

pub fn record_usage(template_id: &str, field_key: &str, value: &serde_json::Value) -> Result<()> {
    let m = crate::services::history::get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;
    let value_json = serde_json::to_string(value)?;
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    db.execute(
        "INSERT INTO field_history (template_id, field_key, value_json, frequency, last_used_at)
         VALUES (?1, ?2, ?3, 1, ?4)
         ON CONFLICT(template_id, field_key, value_json)
         DO UPDATE SET frequency = frequency + 1, last_used_at = ?4",
        rusqlite::params![template_id, field_key, value_json, now],
    )?;

    Ok(())
}
