use crate::external::ExternalTool;
use anyhow::Result;

pub fn list_system_fonts() -> Result<Vec<String>> {
    let mut fonts = Vec::new();

    let dirs: Vec<&str> = if cfg!(target_os = "macos") {
        vec!["/Library/Fonts", "/System/Library/Fonts"]
    } else if cfg!(target_os = "windows") {
        vec!["C:\\Windows\\Fonts"]
    } else {
        vec!["/usr/share/fonts"]
    };

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "ttf" | "otf" | "ttc") {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            fonts.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(fonts)
}

pub fn has_drawtext() -> Result<bool> {
    let bin = crate::external::FfmpegTool.binary_path()?;
    let output = std::process::Command::new(bin)
        .arg("-hide_banner")
        .arg("-filters")
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .any(|line| line.split_whitespace().any(|part| part == "drawtext")))
}
