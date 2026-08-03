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
            // Try to find the actual wps.exe executable via registry
            if let Ok(path) = find_wps_exe_from_registry() {
                return Ok(path);
            }
            // Fallback: try 'where wps' command
            let mut cmd = super::hidden_command("where");
            cmd.arg("wps");
            if let Ok(output) =
                super::command_output_with_timeout(&mut cmd, std::time::Duration::from_secs(2))
            {
                if output.status.success() {
                    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if let Some(first_line) = path_str.lines().next() {
                        let path = PathBuf::from(first_line.trim());
                        if path.exists() {
                            return Ok(path);
                        }
                    }
                }
            }
        }

        anyhow::bail!("WPS Writer 未找到")
    }
}

#[cfg(windows)]
fn find_wps_exe_from_registry() -> Result<PathBuf> {
    // Try CLSID/LocalServer32 first (the canonical COM registration location)
    for clsid in ["KWPS.Application", "kwps.Application"] {
        let mut command = super::hidden_command("reg");
        command.args(["query", &format!(r"HKCR\{}\CLSID", clsid)]);
        if let Ok(output) =
            super::command_output_with_timeout(&mut command, std::time::Duration::from_secs(5))
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                // Extract CLSID value
                for line in text.lines() {
                    if let Some(pos) = line.find("REG_SZ") {
                        let clsid_val = line[pos + 6..].trim().to_string();
                        // Now query LocalServer32 for that CLSID
                        let mut cmd2 = super::hidden_command("reg");
                        cmd2.args(["query", &format!(r"HKCR\CLSID\{}\LocalServer32", clsid_val)]);
                        if let Ok(out2) = super::command_output_with_timeout(
                            &mut cmd2,
                            std::time::Duration::from_secs(5),
                        ) {
                            if out2.status.success() {
                                let text2 = String::from_utf8_lossy(&out2.stdout);
                                if let Some(path) = extract_reg_path(&text2) {
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: shell\open\command
    let mut command = super::hidden_command("reg");
    command.args(["query", r"HKCR\KWPS.Application\shell\open\command"]);
    if let Ok(output) =
        super::command_output_with_timeout(&mut command, std::time::Duration::from_secs(5))
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(path) = extract_reg_path(&text) {
                return Ok(path);
            }
        }
    }

    anyhow::bail!("WPS Writer 注册表路径无效")
}

/// Extract an executable path from registry output.
/// Handles both "C:\...\wps.exe" "%1" (quoted) and C:\...\wps.exe (unquoted).
#[cfg(windows)]
fn extract_reg_path(text: &str) -> Option<PathBuf> {
    for line in text.lines() {
        if !line.contains("REG_") {
            continue;
        }
        let value_part = if let Some(pos) = line.rfind("REG_SZ") {
            line[pos + 6..].trim()
        } else if let Some(pos) = line.rfind("REG_EXPAND_SZ") {
            line[pos + 13..].trim()
        } else {
            continue;
        };
        // Try quoted path first: "C:\...\wps.exe" "%1"
        if let Some(start) = value_part.find('"') {
            let rest = &value_part[start + 1..];
            if let Some(end) = rest.find('"') {
                let path = PathBuf::from(&rest[..end]);
                if path.exists() {
                    return Some(path);
                }
            }
        }
        // Try unquoted path: take everything before first space that looks like an arg
        let first_token = value_part.split_whitespace().next().unwrap_or("");
        if !first_token.is_empty() {
            let path = PathBuf::from(first_token);
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}
