use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct QpdfTool;

impl ExternalTool for QpdfTool {
    fn check(&self) -> ToolStatus {
        match self.binary_path() {
            Ok(path) => {
                let output = std::process::Command::new(&path).arg("--version").output();
                match output {
                    Ok(out) => {
                        let version_output = if out.stdout.is_empty() {
                            &out.stderr
                        } else {
                            &out.stdout
                        };
                        let version = String::from_utf8_lossy(version_output);
                        let version = version.lines().next().unwrap_or("unknown").to_string();
                        let managed = is_managed_path(&path);
                        ToolStatus {
                            available: true,
                            path: Some(path.display().to_string()),
                            version: Some(version),
                            install_hint: String::new(),
                            managed,
                            source: if managed { "docsy" } else { "system" }.into(),
                        }
                    }
                    Err(_) => ToolStatus {
                        available: false,
                        path: Some(path.display().to_string()),
                        version: None,
                        install_hint: "qpdf 存在但无法执行".into(),
                        managed: is_managed_path(&path),
                        source: "broken".into(),
                    },
                }
            }
            Err(_) => ToolStatus {
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
        super::managed::install_tool("qpdf")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        if let Some(path) = super::managed::managed_binary_path("qpdf", binary_name("qpdf")) {
            return Ok(path);
        }

        let known = vec!["/opt/homebrew/bin/qpdf", "/usr/local/bin/qpdf"];
        for p in known {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        if let Some(path) = super::managed::find_on_path(binary_name("qpdf")) {
            return Ok(path);
        }

        anyhow::bail!("qpdf 未找到")
    }
}

fn binary_name(name: &str) -> &'static str {
    if cfg!(windows) {
        match name {
            "qpdf" => "qpdf.exe",
            _ => "qpdf.exe",
        }
    } else {
        "qpdf"
    }
}

fn is_managed_path(path: &std::path::Path) -> bool {
    path.starts_with(super::managed::tools_root())
}
