use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct FfmpegTool;

impl ExternalTool for FfmpegTool {
    fn check(&self) -> ToolStatus {
        match self.binary_path() {
            Ok(path) => {
                let output = std::process::Command::new(&path).arg("-version").output();
                match output {
                    Ok(out) => {
                        let version = String::from_utf8_lossy(&out.stdout);
                        let version = version.lines().next().unwrap_or("unknown").to_string();
                        ToolStatus {
                            available: true,
                            path: Some(path.display().to_string()),
                            version: Some(version),
                            install_hint: String::new(),
                            managed: is_managed_path(&path),
                            source: if is_managed_path(&path) {
                                "docsy"
                            } else {
                                "system"
                            }
                            .into(),
                        }
                    }
                    Err(_) => ToolStatus {
                        available: false,
                        path: Some(path.display().to_string()),
                        version: None,
                        install_hint: "ffmpeg 存在但无法执行".into(),
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
        super::managed::install_tool("ffmpeg")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        if let Some(path) = super::managed::managed_binary_path("ffmpeg", binary_name("ffmpeg")) {
            return Ok(path);
        }

        let known = vec!["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"];
        for p in known {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        if let Some(path) = super::managed::find_on_path(binary_name("ffmpeg")) {
            return Ok(path);
        }

        anyhow::bail!("ffmpeg 未找到")
    }
}

fn binary_name(name: &str) -> &'static str {
    if cfg!(windows) {
        match name {
            "ffmpeg" => "ffmpeg.exe",
            _ => "ffmpeg.exe",
        }
    } else {
        "ffmpeg"
    }
}

fn is_managed_path(path: &std::path::Path) -> bool {
    path.starts_with(super::managed::tools_root())
}
