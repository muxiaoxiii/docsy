use anyhow::Result;
use calamine::{Data, Reader};
use rust_xlsxwriter::Workbook;

const DICT_NAMES: &[&str] = &["courts", "causes", "firms", "lawyers", "stages", "parties"];

pub fn export_dict_xlsx(path: &str) -> Result<String> {
    let m = crate::services::history::get_db()?;

    let sheets_data: Vec<(String, Vec<(String, String, Option<String>, i64, Option<String>)>)> = {
        let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;

        let mut sheets = Vec::new();
        for &dict_name in DICT_NAMES {
            let mut stmt = db.prepare(
                "SELECT entry_key, label, pinyin, frequency, extra_json \
                 FROM global_dictionaries WHERE dict_name = ?1 ORDER BY frequency DESC",
            )?;
            let rows: Vec<(String, String, Option<String>, i64, Option<String>)> = stmt
                .query_map(rusqlite::params![dict_name], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            sheets.push((dict_name.to_string(), rows));
        }
        sheets
    };

    let mut workbook = Workbook::new();

    for (dict_name, rows) in &sheets_data {
        let is_parties = dict_name == "parties";
        let ws = workbook.add_worksheet();
        ws.set_name(dict_name)?;

        ws.write_string(0, 0, "key")?;
        ws.write_string(0, 1, "label")?;
        ws.write_string(0, 2, "pinyin")?;
        ws.write_string(0, 3, "frequency")?;
        if is_parties {
            ws.write_string(0, 4, "subject_type")?;
            ws.write_string(0, 5, "aliases")?;
        }

        for (i, (key, label, pinyin, frequency, extra_json)) in rows.iter().enumerate() {
            let r = (i + 1) as u32;
            ws.write_string(r, 0, key)?;
            ws.write_string(r, 1, label)?;
            if let Some(py) = pinyin {
                ws.write_string(r, 2, py)?;
            }
            ws.write_number(r, 3, *frequency as f64)?;

            if is_parties {
                if let Some(ej) = extra_json {
                    if let Ok(extra) = serde_json::from_str::<serde_json::Value>(ej) {
                        if let Some(st) = extra.get("subject_type").and_then(|v| v.as_str()) {
                            ws.write_string(r, 4, st)?;
                        }
                        if let Some(arr) = extra.get("aliases").and_then(|v| v.as_array()) {
                            let s: String = arr
                                .iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(",");
                            ws.write_string(r, 5, &s)?;
                        }
                    }
                }
            }
        }
    }

    workbook.save(path)?;
    Ok(path.to_string())
}

pub fn import_dict_xlsx(path: &str, mode: &str) -> Result<serde_json::Value> {
    let mut workbook = calamine::open_workbook_auto(path)?;

    let m = crate::services::history::get_db()?;
    let guard = m.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    let db = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;

    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut total_sheets = 0u32;
    let mut total_rows = 0u32;

    let sheet_names = workbook.sheet_names().to_owned();

    for sheet_name in &sheet_names {
        let dict_name = sheet_name.as_str();
        if !DICT_NAMES.contains(&dict_name) {
            continue;
        }

        let is_parties = dict_name == "parties";

        if mode == "overwrite" {
            db.execute(
                "DELETE FROM global_dictionaries WHERE dict_name = ?1",
                rusqlite::params![dict_name],
            )?;
        }

        let range = workbook.worksheet_range(sheet_name)?;

        for (idx, row) in range.rows().enumerate() {
            if idx == 0 {
                continue;
            }
            if row.is_empty() {
                continue;
            }

            let key = cell_str(row.first());
            if key.is_empty() {
                continue;
            }
            let label = cell_str(row.get(1));
            let pinyin = cell_opt_str(row.get(2));
            let frequency = cell_i64(row.get(3));

            let extra_json = if is_parties && row.len() > 5 {
                let subject_type = cell_opt_str(row.get(4)).unwrap_or_default();
                let aliases_str = cell_opt_str(row.get(5)).unwrap_or_default();
                let aliases: Vec<&str> = aliases_str
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                Some(serde_json::to_string(&serde_json::json!({
                    "subject_type": subject_type,
                    "aliases": aliases,
                }))?)
            } else {
                None
            };

            db.execute(
                "INSERT INTO global_dictionaries \
                 (dict_name, entry_key, label, pinyin, extra_json, frequency, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(dict_name, entry_key) DO UPDATE SET \
                     label = excluded.label, \
                     pinyin = excluded.pinyin, \
                     extra_json = excluded.extra_json, \
                     frequency = excluded.frequency",
                rusqlite::params![dict_name, key, label, pinyin, extra_json, frequency, now],
            )?;

            total_rows += 1;
        }

        total_sheets += 1;
    }

    Ok(serde_json::json!({
        "sheets": total_sheets,
        "rows_imported": total_rows,
    }))
}

fn cell_str(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.clone(),
        Some(Data::Int(n)) => n.to_string(),
        Some(Data::Float(f)) => {
            if *f == f.floor() {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Some(Data::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn cell_opt_str(cell: Option<&Data>) -> Option<String> {
    let s = cell_str(cell);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn cell_i64(cell: Option<&Data>) -> i64 {
    match cell {
        Some(Data::Int(n)) => *n,
        Some(Data::Float(f)) => *f as i64,
        Some(Data::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}
