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
                        let version = String::from_utf8_lossy(&out.stderr);
                        let version = version.lines().next().unwrap_or("unknown").to_string();
                        ToolStatus {
                            available: true,
                            path: Some(path.display().to_string()),
                            version: Some(version),
                            install_hint: String::new(),
                        }
                    }
                    Err(_) => ToolStatus {
                        available: false,
                        path: Some(path.display().to_string()),
                        version: None,
                        install_hint: "qpdf 存在但无法执行".into(),
                    },
                }
            }
            Err(_) => ToolStatus {
                available: false,
                path: None,
                version: None,
                install_hint: "brew install qpdf".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        let status = std::process::Command::new("brew")
            .args(["install", "qpdf"])
            .status()?;
        if status.success() {
            Ok("qpdf 安装成功".into())
        } else {
            anyhow::bail!("brew install qpdf 失败")
        }
    }

    fn binary_path(&self) -> Result<PathBuf> {
        // 1. App data dir
        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_qpdf = data_dir.join("Docsy").join("qpdf").join("qpdf");
        if app_qpdf.exists() {
            return Ok(app_qpdf);
        }

        // 2. Known paths
        let known = vec!["/opt/homebrew/bin/qpdf", "/usr/local/bin/qpdf"];
        for p in known {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        // 3. PATH
        if let Ok(output) = std::process::Command::new("which").arg("qpdf").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        anyhow::bail!("qpdf 未找到")
    }
}
