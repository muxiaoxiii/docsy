use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;

pub struct WordTool;

impl ExternalTool for WordTool {
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
                install_hint: "安装 Microsoft Word，或安装 LibreOffice 作为 Word 转 PDF 备用引擎"
                    .into(),
                managed: false,
                source: "manual".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        anyhow::bail!("请手动安装 Microsoft Word")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        #[cfg(windows)]
        {
            if let Ok(path) = find_windows_word_from_registry() {
                return Ok(path);
            }
            if let Ok(path) = find_windows_word_from_path() {
                return Ok(path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(path) = find_macos_word_with_mdfind() {
                return Ok(path);
            }
            let path = PathBuf::from("/Applications/Microsoft Word.app");
            if path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!("Microsoft Word 未找到")
    }
}

#[cfg(windows)]
fn find_windows_word_from_registry() -> Result<PathBuf> {
    // Try both HKLM native and WoW64 views
    for root in [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\Winword.exe",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths\Winword.exe",
    ] {
        let mut command = std::process::Command::new("reg");
        command.args(["query", root, "/ve"]);
        let output = super::command_output_with_timeout(&mut command, std::time::Duration::from_secs(5))?;
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if !line.contains("REG_") {
                continue;
            }
            let value = if let Some((_, v)) = line.rsplit_once("REG_SZ") {
                v.trim()
            } else if let Some((_, v)) = line.rsplit_once("REG_EXPAND_SZ") {
                v.trim()
            } else {
                continue;
            };
            // Handle quoted or unquoted paths
            let path_str = if value.starts_with('"') {
                value.trim_matches('"')
            } else {
                value.split_whitespace().next().unwrap_or(value)
            };
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("注册表中未找到 Winword.exe")
}

#[cfg(windows)]
fn find_windows_word_from_path() -> Result<PathBuf> {
    let mut command = std::process::Command::new("where");
    command.arg("winword");
    let output =
        super::command_output_with_timeout(&mut command, std::time::Duration::from_secs(2))?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        // `where` may return multiple lines; take the first valid one
        for line in text.lines() {
            let path_str = line.trim();
            if !path_str.is_empty() {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }
    anyhow::bail!("PATH 中未找到 winword")
}

#[cfg(target_os = "macos")]
fn find_macos_word_with_mdfind() -> Result<PathBuf> {
    let mut command = std::process::Command::new("mdfind");
    command.arg("kMDItemCFBundleIdentifier == 'com.microsoft.Word'");
    let output =
        super::command_output_with_timeout(&mut command, std::time::Duration::from_secs(2))?;
    if output.status.success() {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let path = PathBuf::from(line.trim());
            if path.exists() {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("mdfind 未找到 Microsoft Word")
}
