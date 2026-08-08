use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

pub struct QpdfTool;

impl ExternalTool for QpdfTool {
    fn check(&self) -> ToolStatus {
        match self.binary_path() {
            Ok(path) => {
                let mut command = super::hidden_command(&path);
                command.arg("--version");
                // A freshly downloaded executable may be held briefly by Windows
                // Defender on its first launch. This is only a version probe, so
                // waiting a little longer is safe and avoids marking it missing.
                let output =
                    super::command_output_with_timeout(&mut command, Duration::from_secs(8));
                match output {
                    Ok(out) if out.status.success() => {
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
                    Ok(out) => ToolStatus {
                        available: false,
                        path: Some(path.display().to_string()),
                        version: None,
                        install_hint: format!(
                            "qpdf 存在但启动失败：{}",
                            super::command_failure_detail(&out)
                        ),
                        managed: is_managed_path(&path),
                        source: "broken".into(),
                    },
                    Err(error) => ToolStatus {
                        available: false,
                        path: Some(path.display().to_string()),
                        version: None,
                        install_hint: format!("qpdf 存在但无法执行：{error}"),
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
