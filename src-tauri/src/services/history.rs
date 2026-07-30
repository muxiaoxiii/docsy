use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AppSettings {
    pub menu_visibility: std::collections::HashMap<String, bool>,
    pub menu_order: Vec<String>,
    pub libreoffice_path: Option<String>,
    pub tool_manifest_url: Option<String>,
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

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("Docsy")
}
