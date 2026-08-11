use super::{ExternalTool, ToolStatus};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct FfmpegTool;

const FFMPEG_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

impl FfmpegTool {
    /// Resolve a usable FFmpeg binary which also provides the drawtext filter.
    /// This matters on macOS where an older managed build may coexist with a
    /// Homebrew `ffmpeg-full` installation.
    pub fn binary_path_with_drawtext(&self) -> Result<PathBuf> {
        let mut failures = Vec::new();
        for path in candidate_paths() {
            if !path.is_file() {
                continue;
            }
            match filter_list_has_drawtext(&path) {
                Ok(true) => return Ok(path),
                Ok(false) => failures.push(format!("{} 不含 drawtext", path.display())),
                Err(error) => failures.push(format!("{} 无法执行: {error}", path.display())),
            }
        }
        let detail = if failures.is_empty() {
            "未找到候选程序".to_string()
        } else {
            failures.join("；")
        };
        anyhow::bail!("未找到支持 drawtext 的 FFmpeg：{detail}")
    }

    fn usable_binary_with_version(&self) -> Result<(PathBuf, std::process::Output)> {
        let mut failures = Vec::new();
        for path in candidate_paths() {
            if !path.is_file() {
                continue;
            }
            match version_output(&path) {
                Ok(output) => return Ok((path, output)),
                Err(error) => failures.push(format!("{}: {error}", path.display())),
            }
        }
        let detail = if failures.is_empty() {
            "未找到候选程序".to_string()
        } else {
            failures.join("；")
        };
        anyhow::bail!("未找到可执行的 ffmpeg：{detail}")
    }
}

impl ExternalTool for FfmpegTool {
    fn check(&self) -> ToolStatus {
        match self.usable_binary_with_version() {
            Ok((path, output)) => {
                let version = String::from_utf8_lossy(&output.stdout);
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
            Err(error) => ToolStatus {
                available: false,
                path: None,
                version: None,
                install_hint: error.to_string(),
                managed: false,
                source: "missing".into(),
            },
        }
    }

    fn try_install(&self) -> Result<String> {
        super::managed::install_tool("ffmpeg")
    }

    fn binary_path(&self) -> Result<PathBuf> {
        self.usable_binary_with_version().map(|(path, _)| path)
    }
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // A manually installed/managed build is an explicit user choice and must
    // take precedence over a system Homebrew build.
    if let Some(path) = super::managed::managed_binary_path("ffmpeg", binary_name("ffmpeg")) {
        push_unique_path(&mut paths, path);
    }

    // Prefer an explicitly installed full Homebrew build on macOS over the
    // generic Homebrew/PATH binary, because it includes drawtext.
    #[cfg(target_os = "macos")]
    for candidate in [
        "/opt/homebrew/opt/ffmpeg-full/bin/ffmpeg",
        "/usr/local/opt/ffmpeg-full/bin/ffmpeg",
    ] {
        push_unique_path(&mut paths, PathBuf::from(candidate));
    }

    for candidate in ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"] {
        push_unique_path(&mut paths, PathBuf::from(candidate));
    }

    if let Some(path) = super::managed::find_on_path(binary_name("ffmpeg")) {
        push_unique_path(&mut paths, path);
    }
    paths
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|known| known == &path) {
        paths.push(path);
    }
}

fn version_output(path: &Path) -> Result<std::process::Output> {
    let mut command = super::hidden_command(path);
    command.arg("-version");
    let output = super::command_output_with_timeout(&mut command, FFMPEG_PROBE_TIMEOUT)?;
    if output.status.success() {
        Ok(output)
    } else {
        anyhow::bail!("{}", super::command_failure_detail(&output))
    }
}

fn filter_list_has_drawtext(path: &Path) -> Result<bool> {
    let mut command = super::hidden_command(path);
    command.args(["-hide_banner", "-filters"]);
    let output = super::command_output_with_timeout(&mut command, FFMPEG_PROBE_TIMEOUT)?;
    if !output.status.success() {
        anyhow::bail!("{}", super::command_failure_detail(&output));
    }
    Ok(output_contains_drawtext(&output))
}

fn output_contains_drawtext(output: &std::process::Output) -> bool {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .chain(String::from_utf8_lossy(&output.stderr).lines())
        .any(|line| line.split_whitespace().any(|part| part == "drawtext"))
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

fn is_managed_path(path: &Path) -> bool {
    path.starts_with(super::managed::tools_root())
}

#[cfg(test)]
mod tests {
    use super::output_contains_drawtext;
    use std::process::{ExitStatus, Output};

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    #[cfg(unix)]
    fn output(stdout: &str, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn detects_drawtext_from_either_output_stream() {
        assert!(output_contains_drawtext(&output(" T. drawtext V->V", "")));
        assert!(output_contains_drawtext(&output("", " T. drawtext V->V")));
        assert!(!output_contains_drawtext(&output(" T. subtitles V->V", "")));
    }
}
