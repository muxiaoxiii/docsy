use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct LibreOfficeTool;

impl ExternalTool for LibreOfficeTool {
    fn check(&self) -> ToolStatus {
        match self.binary_path() {
            Ok(path) => ToolStatus {
                available: true,
                path: Some(path.display().to_string()),
                version: None,
                install_hint: String::new(),
                managed: false,
                source: "system".into(),
            },
            Err(_) => ToolStatus {
                available: false,
                path: None,
                version: None,
                install_hint: "安装 LibreOffice: https://www.libreoffice.org".into(),
                managed: false,
                source: "manual".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        anyhow::bail!("请手动安装 LibreOffice")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        // Check user settings first
        if let Ok(settings) = crate::services::history::get_settings() {
            if let Some(path) = settings.libreoffice_path {
                let p = PathBuf::from(&path);
                if p.exists() {
                    return Ok(p);
                }
            }
        }

        let known = vec![
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            "/usr/bin/soffice",
        ];
        for p in known {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        if let Ok(output) = std::process::Command::new("which").arg("soffice").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        anyhow::bail!("LibreOffice 未找到")
    }
}
