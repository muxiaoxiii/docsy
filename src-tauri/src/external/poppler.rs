use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct PopplerTool;

impl PopplerTool {
    pub fn binary_path_for(name: &str) -> Result<PathBuf> {
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

        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
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
                }
            }
            _ => ToolStatus {
                available: false,
                path: None,
                version: None,
                install_hint: "brew install poppler；Windows 请安装 Poppler 并加入 PATH".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        let status = std::process::Command::new("brew")
            .args(["install", "poppler"])
            .status()?;
        if status.success() {
            Ok("poppler 安装成功".into())
        } else {
            anyhow::bail!("brew install poppler 失败")
        }
    }

    fn binary_path(&self) -> Result<PathBuf> {
        Self::binary_path_for("pdftoppm")
    }
}
