use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;
#[cfg(windows)]
use std::time::Duration;

pub struct WpsTool;

impl ExternalTool for WpsTool {
    fn check(&self) -> ToolStatus {
        match wps_installation() {
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
        wps_installation()
    }
}

fn wps_installation() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(path) = find_wps_exe_from_registry() {
            return Ok(path);
        }
        if let Some(path) = find_wps_in_known_locations() {
            return Ok(path);
        }
        let mut cmd = super::hidden_command("where");
        cmd.arg("wps");
        if let Ok(output) = super::command_output_with_timeout(&mut cmd, Duration::from_secs(2)) {
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
        if windows_com_registered("KWPS.Application") || windows_com_registered("kwps.Application")
        {
            return Ok(PathBuf::from("KWPS.Application (COM)"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Check known macOS locations
        let candidates = [
            std::path::PathBuf::from("/Applications/wpsoffice.app"),
            std::path::PathBuf::from("/Applications/WPS Office.app"),
        ];
        for path in &candidates {
            if path.join("Contents/MacOS/wpsoffice").is_file()
                || path.join("Contents/MacOS/wpscli").is_file()
            {
                return Ok(path.clone());
            }
        }
        // Try mdfind
        let mut cmd = super::hidden_command("mdfind");
        cmd.arg("kMDItemCFBundleIdentifier == 'cn.wps.macos.wpsoffice'");
        if let Ok(output) = super::command_output_with_timeout(
            &mut cmd,
            std::time::Duration::from_secs(2),
        ) {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let path = std::path::PathBuf::from(line.trim());
                    if path.exists() {
                        return Ok(path);
                    }
                }
            }
        }
    }

    anyhow::bail!("WPS Writer 未找到")
}

#[cfg(windows)]
fn find_wps_exe_from_registry() -> Result<PathBuf> {
    for prog_id in ["KWPS.Application", "kwps.Application"] {
        for view in ["/reg:64", "/reg:32"] {
            let mut command = super::hidden_command("reg");
            command.args(["query", &format!(r"HKCR\{}\CLSID", prog_id), "/ve", view]);
            if let Ok(output) =
                super::command_output_with_timeout(&mut command, Duration::from_secs(5))
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if let Some(clsid) = registry_value(&text) {
                        let mut server = super::hidden_command("reg");
                        server.args([
                            "query",
                            &format!(r"HKCR\CLSID\{}\LocalServer32", clsid),
                            "/ve",
                            view,
                        ]);
                        if let Ok(server_output) =
                            super::command_output_with_timeout(&mut server, Duration::from_secs(5))
                        {
                            if server_output.status.success() {
                                if let Some(path) = extract_reg_path(&String::from_utf8_lossy(
                                    &server_output.stdout,
                                )) {
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for view in ["/reg:64", "/reg:32"] {
        let mut command = super::hidden_command("reg");
        command.args([
            "query",
            r"HKCR\KWPS.Application\shell\open\command",
            "/ve",
            view,
        ]);
        if let Ok(output) = super::command_output_with_timeout(&mut command, Duration::from_secs(5))
        {
            if output.status.success() {
                if let Some(path) = extract_reg_path(&String::from_utf8_lossy(&output.stdout)) {
                    return Ok(path);
                }
            }
        }
    }

    anyhow::bail!("WPS Writer 注册表路径无效")
}

/// Extract an executable path from registry output.
/// Handles both "C:\...\wps.exe" "%1" (quoted) and C:\...\wps.exe (unquoted).
#[cfg(windows)]
fn extract_reg_path(text: &str) -> Option<PathBuf> {
    let value = registry_value(text)?;
    let path = executable_path_from_command(&value)?;
    path.is_file().then_some(path)
}

#[cfg(any(windows, test))]
fn registry_value(text: &str) -> Option<String> {
    for line in text.lines() {
        for value_type in ["REG_EXPAND_SZ", "REG_SZ"] {
            if let Some(position) = line.find(value_type) {
                let value = line[position + value_type.len()..].trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[cfg(any(windows, test))]
fn executable_path_from_command(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let lower = value.to_ascii_lowercase();
    let end = lower.find(".exe").map(|position| position + 4)?;
    Some(PathBuf::from(value[..end].trim()))
}

#[cfg(windows)]
fn windows_com_registered(prog_id: &str) -> bool {
    let escaped = prog_id.replace('\'', "''");
    let script = format!(
        "$type=[type]::GetTypeFromProgID('{escaped}',$false); if ($null -eq $type) {{ exit 1 }}"
    );
    let mut command = super::hidden_command("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    super::command_output_with_timeout(&mut command, Duration::from_secs(5))
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn find_wps_in_known_locations() -> Option<PathBuf> {
    let mut roots = Vec::new();
    for name in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(value) = std::env::var_os(name) {
            roots.push(PathBuf::from(value));
        }
    }
    let fixed = roots
        .into_iter()
        .flat_map(|root| [root.join("Kingsoft/WPS Office/office6/wps.exe")]);
    fixed.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::{executable_path_from_command, registry_value};

    #[test]
    fn parses_quoted_wps_local_server_command() {
        let value = r#""C:\Program Files\Kingsoft\WPS Office\office6\wps.exe" /Automation"#;
        assert_eq!(
            executable_path_from_command(value)
                .unwrap()
                .to_string_lossy(),
            r"C:\Program Files\Kingsoft\WPS Office\office6\wps.exe"
        );
    }

    #[test]
    fn parses_unquoted_wps_command_with_spaces() {
        let value = r"C:\Program Files\Kingsoft\WPS Office\office6\wps.exe /Automation";
        assert_eq!(
            executable_path_from_command(value)
                .unwrap()
                .to_string_lossy(),
            r"C:\Program Files\Kingsoft\WPS Office\office6\wps.exe"
        );
    }

    #[test]
    fn extracts_registry_value() {
        let output = r#"    (Default)    REG_SZ    "C:\WPS\wps.exe" /Automation"#;
        assert_eq!(
            registry_value(output).as_deref(),
            Some(r#""C:\WPS\wps.exe" /Automation"#)
        );
    }
}
