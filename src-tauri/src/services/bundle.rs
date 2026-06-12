use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleManifest {
    pub version: String,
    pub exported_at: String,
    pub docsy_version: String,
    pub contents: BundleContents,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleContents {
    pub templates: Vec<String>,
    pub dictionaries: bool,
    pub field_history: bool,
    pub parties: bool,
    pub settings: bool,
}

/// Export configuration to a .docsybundle file (zip)
pub fn export_bundle(path: &str, options: &serde_json::Value) -> Result<String> {
    let include_templates = options.get("templates").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_dicts = options.get("dictionaries").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_history = options.get("field_history").and_then(|v| v.as_bool()).unwrap_or(false);
    let include_parties = options.get("parties").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_settings = options.get("settings").and_then(|v| v.as_bool()).unwrap_or(false);

    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut manifest = BundleManifest {
        version: "1.0".to_string(),
        exported_at: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        docsy_version: env!("CARGO_PKG_VERSION").to_string(),
        contents: BundleContents {
            templates: vec![],
            dictionaries: include_dicts,
            field_history: include_history,
            parties: include_parties,
            settings: include_settings,
        },
    };

    // Export templates
    if include_templates {
        let user_dir = crate::services::template_store::user_templates_dir();
        if user_dir.exists() {
            for entry in std::fs::read_dir(&user_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|e| e.to_str()) == Some("docsytpl") {
                    let name = entry.file_name().to_string_lossy().to_string();
                    manifest.contents.templates.push(name.clone());
                    zip.start_file(format!("templates/{}", name), options)?;
                    let bytes = std::fs::read(entry.path())?;
                    std::io::Write::write_all(&mut zip, &bytes)?;
                }
            }
        }
    }

    // Export dictionaries
    if include_dicts {
        let db = crate::services::history::get_db()?;
        let guard = db.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        let db_conn = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;

        let mut stmt = db_conn.prepare("SELECT dict_name, entry_key, label, pinyin, extra_json, frequency FROM global_dictionaries")?;
        let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "dict_name": row.get::<_, String>(0)?,
                "entry_key": row.get::<_, String>(1)?,
                "label": row.get::<_, String>(2)?,
                "pinyin": row.get::<_, Option<String>>(3)?,
                "extra": row.get::<_, Option<String>>(4)?,
                "frequency": row.get::<_, i32>(5)?,
            }))
        })?.collect::<Result<Vec<_>, _>>()?;

        zip.start_file("data/dictionaries.json", options)?;
        serde_json::to_writer_pretty(&mut zip, &rows)?;
    }

    // Export parties
    if include_parties {
        let db = crate::services::history::get_db()?;
        let guard = db.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        let db_conn = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;

        let mut stmt = db_conn.prepare("SELECT id, name, subject_type, aliases_json, frequency FROM parties")?;
        let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "subject_type": row.get::<_, String>(2)?,
                "aliases": row.get::<_, Option<String>>(3)?,
                "frequency": row.get::<_, i32>(4)?,
            }))
        })?.collect::<Result<Vec<_>, _>>()?;

        zip.start_file("data/parties.json", options)?;
        serde_json::to_writer_pretty(&mut zip, &rows)?;
    }

    // Export settings
    if include_settings {
        let settings = crate::services::history::get_settings()?;
        zip.start_file("settings.json", options)?;
        serde_json::to_writer_pretty(&mut zip, &settings)?;
    }

    // Write manifest
    zip.start_file("manifest.json", options)?;
    serde_json::to_writer_pretty(&mut zip, &manifest)?;

    zip.finish()?;
    Ok(path.to_string())
}

/// Import configuration from a .docsybundle file
pub fn import_bundle(path: &str, options: &serde_json::Value) -> Result<serde_json::Value> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Read manifest
    let manifest: BundleManifest = {
        let mut f = archive.by_name("manifest.json")?;
        serde_json::from_reader(&mut f)?
    };

    let import_templates = options.get("templates").and_then(|v| v.as_bool()).unwrap_or(true);
    let import_dicts = options.get("dictionaries").and_then(|v| v.as_bool()).unwrap_or(true);
    let import_parties = options.get("parties").and_then(|v| v.as_bool()).unwrap_or(true);
    let import_settings = options.get("settings").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut imported = serde_json::json!({
        "templates": 0,
        "dictionaries": 0,
        "parties": 0,
        "settings": false,
    });

    // Import templates
    if import_templates {
        let user_dir = crate::services::template_store::user_templates_dir();
        std::fs::create_dir_all(&user_dir)?;

        for name in &manifest.contents.templates {
            let src_name = format!("templates/{}", name);
            if let Ok(mut src) = archive.by_name(&src_name) {
                let dest = user_dir.join(name);
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut src, &mut bytes)?;
                std::fs::write(&dest, &bytes)?;
                *imported.get_mut("templates").unwrap() =
                    serde_json::Value::Number(serde_json::Number::from(
                        imported["templates"].as_u64().unwrap_or(0) + 1,
                    ));
            }
        }
    }

    // Import dictionaries
    if import_dicts && manifest.contents.dictionaries {
        if let Ok(mut src) = archive.by_name("data/dictionaries.json") {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut src, &mut s)?;
            let rows: Vec<serde_json::Value> = serde_json::from_str(&s)?;

            let db = crate::services::history::get_db()?;
            let guard = db.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            let db_conn = guard.as_ref().ok_or_else(|| anyhow::anyhow!("db not initialized"))?;

            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            let mut count = 0u64;
            for row in &rows {
                let dict_name = row["dict_name"].as_str().unwrap_or("");
                let entry_key = row["entry_key"].as_str().unwrap_or("");
                let label = row["label"].as_str().unwrap_or("");
                let pinyin = row["pinyin"].as_str();
                let extra = row["extra"].as_str();
                let frequency = row["frequency"].as_i64().unwrap_or(0);

                db_conn.execute(
                    "INSERT INTO global_dictionaries (dict_name, entry_key, label, pinyin, extra_json, frequency, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(dict_name, entry_key) DO UPDATE SET label=?3, pinyin=?4, extra_json=?5, frequency=?6",
                    rusqlite::params![dict_name, entry_key, label, pinyin, extra, frequency as i32, now],
                )?;
                count += 1;
            }
            *imported.get_mut("dictionaries").unwrap() =
                serde_json::Value::Number(serde_json::Number::from(count));
        }
    }

    // Import settings
    if import_settings && manifest.contents.settings {
        if let Ok(mut src) = archive.by_name("settings.json") {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut src, &mut s)?;
            let settings: crate::services::history::AppSettings = serde_json::from_str(&s)?;
            crate::services::history::save_settings(&settings)?;
            *imported.get_mut("settings").unwrap() = serde_json::Value::Bool(true);
        }
    }

    Ok(serde_json::json!({
        "manifest": manifest,
        "imported": imported,
    }))
}
