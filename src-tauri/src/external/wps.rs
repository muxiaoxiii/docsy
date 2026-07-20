use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct WpsTool;

impl ExternalTool for WpsTool {
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
                install_hint: "安装 Windows 版 WPS Office，或安装 LibreOffice 作为备用引擎".into(),
                managed: false,
                source: "manual".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        anyhow::bail!("请手动安装 WPS Office")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        #[cfg(windows)]
        {
            if is_wps_writer_com_registered() {
                return Ok(PathBuf::from("KWPS.Application"));
            }
        }

        anyhow::bail!("WPS Writer COM 未找到")
    }
}

#[cfg(windows)]
fn is_wps_writer_com_registered() -> bool {
    let output = std::process::Command::new("reg")
        .args(["query", r"HKCR\KWPS.Application\CLSID"])
        .output();
    matches!(output, Ok(output) if output.status.success())
}
