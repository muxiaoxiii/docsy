use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

pub struct LibreOfficeTool;

impl ExternalTool for LibreOfficeTool {
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
                install_hint: "安装 LibreOffice: https://www.libreoffice.org".into(),
                managed: false,
                source: "manual".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        anyhow::bail!("请手动安装 LibreOffice")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        if let Ok(settings) = crate::services::history::get_settings() {
            if let Some(path) = settings.libreoffice_path {
                if let Some(binary) = resolve_libreoffice_path(PathBuf::from(path)) {
                    return Ok(binary);
                }
            }
        }

        for path in known_libreoffice_paths() {
            if path.is_file() {
                return Ok(path);
            }
        }

        let mut command = if cfg!(windows) {
            let mut command = super::hidden_command("where");
            command.arg("soffice");
            command
        } else {
            let mut command = super::hidden_command("which");
            command.arg("soffice");
            command
        };
        if let Ok(output) = super::command_output_with_timeout(&mut command, Duration::from_secs(2))
        {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let path = PathBuf::from(line.trim());
                    if path.is_file() {
                        return Ok(path);
                    }
                }
            }
        }

        anyhow::bail!("LibreOffice 未找到")
    }
}

fn resolve_libreoffice_path(path: PathBuf) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path);
    }
    if !path.is_dir() {
        return None;
    }

    let candidates = if cfg!(target_os = "macos") {
        vec![path.join("Contents/MacOS/soffice"), path.join("soffice")]
    } else if cfg!(windows) {
        vec![path.join("program/soffice.exe"), path.join("soffice.exe")]
    } else {
        vec![path.join("program/soffice"), path.join("soffice")]
    };
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn known_libreoffice_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        ));
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(
                PathBuf::from(home).join("Applications/LibreOffice.app/Contents/MacOS/soffice"),
            );
        }
    }

    #[cfg(windows)]
    {
        for name in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(root) = std::env::var_os(name) {
                let root = PathBuf::from(root);
                paths.push(root.join("LibreOffice/program/soffice.exe"));
                paths.push(root.join("Programs/LibreOffice/program/soffice.exe"));
            }
        }
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        paths.push(PathBuf::from("/usr/bin/soffice"));
        paths.push(PathBuf::from("/usr/local/bin/soffice"));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::resolve_libreoffice_path;
    use std::fs;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let unique = format!(
            "docsy-libreoffice-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn accepts_direct_binary_path() {
        let root = test_dir("binary");
        let binary = root.join(if cfg!(windows) {
            "soffice.exe"
        } else {
            "soffice"
        });
        fs::write(&binary, b"test").unwrap();
        assert_eq!(resolve_libreoffice_path(binary.clone()), Some(binary));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_install_directory_to_binary() {
        let root = test_dir("directory");
        let binary = if cfg!(target_os = "macos") {
            root.join("Contents/MacOS/soffice")
        } else if cfg!(windows) {
            root.join("program/soffice.exe")
        } else {
            root.join("program/soffice")
        };
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"test").unwrap();
        assert_eq!(resolve_libreoffice_path(root.clone()), Some(binary));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_existing_directory_without_binary() {
        let root = test_dir("missing");
        assert_eq!(resolve_libreoffice_path(root.clone()), None);
        fs::remove_dir_all(root).unwrap();
    }
}
