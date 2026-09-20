use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AppSettings {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub recovery_warning: Option<String>,
    pub menu_visibility: std::collections::HashMap<String, bool>,
    pub menu_order: Vec<String>,
    pub libreoffice_path: Option<String>,
    pub tool_manifest_url: Option<String>,
    pub onboarding_completed: bool,
    pub custom_gh_proxies: Vec<String>,
    pub selected_gh_proxy: Option<String>,
}

pub fn get_settings() -> Result<AppSettings> {
    read_settings(&data_dir().join("settings.json"))
}

fn read_settings(path: &std::path::Path) -> Result<AppSettings> {
    if !path.exists() && !path.with_extension("json.backup").exists() {
        return Ok(AppSettings::default());
    }
    let read = |p: &std::path::Path| -> Result<AppSettings> {
        Ok(serde_json::from_slice(&std::fs::read(p)?)?)
    };
    match read(path) {
        Ok(settings) => Ok(settings),
        Err(error) => {
            log::warn!("设置文件读取失败，尝试保留的备份：{error}");
            read(&path.with_extension("json.backup"))
                .map(|mut settings| {
                    settings.recovery_warning = Some(
                        "设置文件损坏，已读取上次有效备份。损坏文件已保留，请核对设置后保存。"
                            .into(),
                    );
                    settings
                })
                .map_err(|backup| {
                    anyhow::anyhow!("设置及备份读取失败；原文件已保留：{error}; {backup}")
                })
        }
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = LOCK.lock().map_err(|_| anyhow::anyhow!("设置保存锁异常"))?;
    write_settings(&data_dir().join("settings.json"), settings)
}

fn write_settings(path: &std::path::Path, settings: &AppSettings) -> Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = crate::util::fs::sibling_temp_path(path, "settings");
    let _guard = crate::util::fs::TempPathGuard::new(temporary.clone());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    crate::util::fs::set_private_permissions(&temporary)?;
    let mut persisted = settings.clone();
    persisted.recovery_warning = None;
    file.write_all(&serde_json::to_vec_pretty(&persisted)?)?;
    file.sync_all()?;
    drop(file);
    // Never replace a known-good backup with corrupt bytes.
    if let Ok(bytes) = std::fs::read(path) {
        if serde_json::from_slice::<AppSettings>(&bytes).is_ok() {
            let backup = path.with_extension("json.backup");
            let staged = crate::util::fs::sibling_temp_path(&backup, "settings");
            let _backup_guard = crate::util::fs::TempPathGuard::new(staged.clone());
            let mut file = std::fs::File::create(&staged)?;
            crate::util::fs::set_private_permissions(&staged)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            if backup.exists() {
                crate::util::fs::replace_file(&staged, &backup)?;
            } else {
                std::fs::rename(&staged, &backup)?;
            }
        } else {
            let corrupt = crate::util::fs::sibling_temp_path(path, "settings-corrupt");
            std::fs::copy(path, &corrupt)?;
            crate::util::fs::set_private_permissions(&corrupt)?;
        }
    }
    if path.exists() {
        crate::util::fs::replace_file(&temporary, path)?;
    } else {
        std::fs::rename(&temporary, path)?;
    }
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        std::fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recover_previous_valid_settings_without_destroying_corrupt_file() {
        let dir = crate::util::fs::temp_named_path("docsy-settings-test", "dir");
        std::fs::create_dir(&dir).unwrap();
        let _guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
        let path = dir.join("settings.json");
        let mut settings = AppSettings::default();
        settings.onboarding_completed = true;
        write_settings(&path, &settings).unwrap();
        settings.onboarding_completed = false;
        write_settings(&path, &settings).unwrap();
        std::fs::write(&path, b"broken").unwrap();
        assert!(read_settings(&path).unwrap().onboarding_completed);
        assert_eq!(std::fs::read(&path).unwrap(), b"broken");
        assert!(read_settings(&path).unwrap().recovery_warning.is_some());
        write_settings(&path, &settings).unwrap();
        assert!(read_settings(&path).unwrap().recovery_warning.is_none());
        assert!(std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .contains("settings-corrupt")));
    }
}

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("Docsy")
}
