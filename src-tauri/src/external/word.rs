use super::{ExternalTool, ToolStatus};
use anyhow::Result;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
#[cfg(windows)]
use std::time::Duration;

pub struct WordTool;

impl ExternalTool for WordTool {
    fn check(&self) -> ToolStatus {
        match word_installation() {
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
        word_installation()
    }
}

fn word_installation() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(path) = find_windows_word_from_registry() {
            return Ok(path);
        }
        if let Some(path) = find_windows_word_in_known_locations() {
            return Ok(path);
        }
        if let Ok(path) = find_windows_word_from_path() {
            return Ok(path);
        }
        if windows_com_registered("Word.Application") {
            return Ok(PathBuf::from("Word.Application (COM)"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(path) = find_macos_word_in_known_locations() {
            return Ok(path);
        }
        if let Ok(path) = find_macos_word_with_mdfind() {
            return Ok(path);
        }
        if macos_launch_services_has_word() {
            return Ok(PathBuf::from("Microsoft Word (LaunchServices)"));
        }
    }

    anyhow::bail!("Microsoft Word 未找到")
}

#[cfg(windows)]
fn find_windows_word_from_registry() -> Result<PathBuf> {
    let roots = [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\Winword.exe",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\Winword.exe",
    ];
    for root in roots {
        for view in ["/reg:64", "/reg:32"] {
            let mut command = super::hidden_command("reg");
            command.args(["query", root, "/ve", view]);
            let Ok(output) =
                super::command_output_with_timeout(&mut command, Duration::from_secs(5))
            else {
                continue;
            };
            if !output.status.success() {
                continue;
            }
            if let Some(value) = registry_default_value(&String::from_utf8_lossy(&output.stdout)) {
                let path = PathBuf::from(expand_windows_environment(&value));
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
    }
    anyhow::bail!("注册表中未找到 Winword.exe")
}

#[cfg(windows)]
fn find_windows_word_from_path() -> Result<PathBuf> {
    let mut command = super::hidden_command("where");
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

#[cfg(windows)]
fn find_windows_word_in_known_locations() -> Option<PathBuf> {
    let mut roots = Vec::new();
    for name in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(value) = std::env::var_os(name) {
            roots.push(PathBuf::from(value));
        }
    }
    for root in roots {
        for suffix in [
            ["Microsoft Office", "root", "Office16", "WINWORD.EXE"].as_slice(),
            ["Microsoft Office", "Office16", "WINWORD.EXE"].as_slice(),
        ] {
            let candidate = suffix
                .iter()
                .fold(root.clone(), |path, part| path.join(part));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
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

#[cfg(target_os = "macos")]
fn find_macos_word_with_mdfind() -> Result<PathBuf> {
    let mut command = super::hidden_command("mdfind");
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

#[cfg(target_os = "macos")]
fn find_macos_word_in_known_locations() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from("/Applications/Microsoft Word.app")];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join("Applications/Microsoft Word.app"));
    }
    candidates.into_iter().find(|path| word_app_is_valid(path))
}

#[cfg(target_os = "macos")]
fn macos_launch_services_has_word() -> bool {
    let mut command = super::hidden_command("open");
    command.args(["-Ra", "Microsoft Word"]);
    super::command_output_with_timeout(&mut command, std::time::Duration::from_secs(3))
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn word_app_is_valid(path: &Path) -> bool {
    path.join("Contents/MacOS/Microsoft Word").is_file()
}

#[cfg(any(windows, test))]
fn registry_default_value(text: &str) -> Option<String> {
    for line in text.lines() {
        let value = registry_value_after_type(line, "REG_EXPAND_SZ")
            .or_else(|| registry_value_after_type(line, "REG_SZ"));
        if let Some(value) = value {
            let value = value.trim().trim_matches('"').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(any(windows, test))]
fn registry_value_after_type<'a>(line: &'a str, value_type: &str) -> Option<&'a str> {
    line.find(value_type)
        .map(|position| &line[position + value_type.len()..])
}

#[cfg(windows)]
fn expand_windows_environment(value: &str) -> String {
    let mut expanded = value.to_string();
    for (name, replacement) in std::env::vars() {
        expanded = expanded.replace(&format!("%{name}%"), &replacement);
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::registry_default_value;

    #[test]
    fn parses_unquoted_word_path_with_spaces() {
        let output = r#"    (Default)    REG_SZ    C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE"#;
        assert_eq!(
            registry_default_value(output).as_deref(),
            Some(r"C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE")
        );
    }

    #[test]
    fn parses_quoted_word_path() {
        let output = r#"    (Default)    REG_SZ    "C:\Office\WINWORD.EXE""#;
        assert_eq!(
            registry_default_value(output).as_deref(),
            Some(r"C:\Office\WINWORD.EXE")
        );
    }

    #[test]
    fn prefers_expand_string_type_without_matching_reg_sz_suffix() {
        let output =
            r#"    (Default)    REG_EXPAND_SZ    %ProgramFiles%\Microsoft Office\WINWORD.EXE"#;
        assert_eq!(
            registry_default_value(output).as_deref(),
            Some(r"%ProgramFiles%\Microsoft Office\WINWORD.EXE")
        );
    }
}
