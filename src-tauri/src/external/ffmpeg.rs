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
                        }
                    }
                    Err(_) => ToolStatus {
                        available: false,
                        path: Some(path.display().to_string()),
                        version: None,
                        install_hint: "ffmpeg 存在但无法执行".into(),
                    },
                }
            }
            Err(_) => ToolStatus {
                available: false,
                path: None,
                version: None,
                install_hint: "brew install ffmpeg".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        let status = std::process::Command::new("brew")
            .args(["install", "ffmpeg"])
            .status()?;
        if status.success() {
            Ok("ffmpeg 安装成功".into())
        } else {
            anyhow::bail!("brew install ffmpeg 失败")
        }
    }

    fn binary_path(&self) -> Result<PathBuf> {
        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_ffmpeg = data_dir.join("Docsy").join("ffmpeg").join("ffmpeg");
        if app_ffmpeg.exists() {
            return Ok(app_ffmpeg);
        }

        let known = vec!["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"];
        for p in known {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        if let Ok(output) = std::process::Command::new("which").arg("ffmpeg").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        anyhow::bail!("ffmpeg 未找到")
    }
}
