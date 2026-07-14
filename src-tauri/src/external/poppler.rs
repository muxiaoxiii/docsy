use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct PopplerTool;

impl PopplerTool {
    pub fn binary_path_for(name: &str) -> Result<PathBuf> {
        if let Some(path) = super::managed::managed_binary_path("poppler", binary_name(name)) {
            return Ok(path);
        }

        let known = if cfg!(target_os = "macos") {
            vec![
                format!("/opt/homebrew/bin/{name}"),
                format!("/usr/local/bin/{name}"),
            ]
        } else {
            Vec::new()
        };
        for candidate in known {
            let path = PathBuf::from(candidate);
            if path.exists() {
                return Ok(path);
            }
        }

        if let Some(path) = super::managed::find_on_path(binary_name(name)) {
            return Ok(path);
        }

        anyhow::bail!("{name} 未找到")
    }
}

impl ExternalTool for PopplerTool {
    fn check(&self) -> ToolStatus {
        let pdftoppm = Self::binary_path_for("pdftoppm");
        let pdftotext = Self::binary_path_for("pdftotext");
        match (pdftoppm, pdftotext) {
            (Ok(pdftoppm), Ok(pdftotext)) => {
                let output = std::process::Command::new(&pdftoppm).arg("-v").output();
                let version = output.ok().and_then(|out| {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    stderr.lines().next().map(ToString::to_string)
                });
                ToolStatus {
                    available: true,
                    path: Some(format!(
                        "pdftoppm: {}; pdftotext: {}",
                        pdftoppm.display(),
                        pdftotext.display()
                    )),
                    version,
                    install_hint: String::new(),
                    managed: is_managed_path(&pdftoppm) && is_managed_path(&pdftotext),
                    source: if is_managed_path(&pdftoppm) && is_managed_path(&pdftotext) {
                        "docsy"
                    } else {
                        "system"
                    }
                    .into(),
                }
            }
            _ => ToolStatus {
                available: false,
                path: None,
                version: None,
                install_hint: "可下载安装到 Docsy 工具目录".into(),
                managed: false,
                source: "missing".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        super::managed::install_tool("poppler")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        Self::binary_path_for("pdftoppm")
    }
}

fn binary_name(name: &str) -> &str {
    if cfg!(windows) {
        match name {
            "pdftoppm" => "pdftoppm.exe",
            "pdftotext" => "pdftotext.exe",
            _ => name,
        }
    } else {
        name
    }
}

fn is_managed_path(path: &std::path::Path) -> bool {
    path.starts_with(super::managed::tools_root())
}
