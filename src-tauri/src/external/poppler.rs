use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

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
                let path_text = format!(
                    "pdftoppm: {}; pdftotext: {}",
                    pdftoppm.display(),
                    pdftotext.display()
                );
                let managed = is_managed_path(&pdftoppm) && is_managed_path(&pdftotext);
                match probe_binary(&pdftoppm)
                    .and_then(|version| probe_binary(&pdftotext).map(|_| version))
                {
                    Ok(version) => ToolStatus {
                        available: true,
                        path: Some(path_text),
                        version: Some(version),
                        install_hint: String::new(),
                        managed,
                        source: if managed { "docsy" } else { "system" }.into(),
                    },
                    Err(error) => ToolStatus {
                        available: false,
                        path: Some(path_text),
                        version: None,
                        install_hint: error,
                        managed,
                        source: "broken".into(),
                    },
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

fn probe_binary(path: &std::path::Path) -> std::result::Result<String, String> {
    let mut command = super::hidden_command(path);
    command.arg("-v");
    let output = super::command_output_with_timeout(&mut command, Duration::from_secs(8))
        .map_err(|error| format!("{} 存在但无法执行：{error}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} 存在但启动失败：{}",
            path.display(),
            super::command_failure_detail(&output)
        ));
    }
    let version_output = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    Ok(String::from_utf8_lossy(version_output)
        .lines()
        .next()
        .unwrap_or("Poppler（版本未知）")
        .to_string())
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
