pub mod ffmpeg;
pub mod libreoffice;
pub mod managed;
pub mod poppler;
pub mod qpdf;
pub mod word;
pub mod wps;

use serde::Serialize;
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub install_hint: String,
    pub managed: bool,
    pub source: String,
}

pub trait ExternalTool: Send + Sync {
    fn check(&self) -> ToolStatus;
    fn try_install(&self) -> anyhow::Result<String>;
    fn binary_path(&self) -> anyhow::Result<std::path::PathBuf>;
}

pub use ffmpeg::FfmpegTool;
pub use libreoffice::LibreOfficeTool;
pub use poppler::PopplerTool;
pub use qpdf::QpdfTool;
pub use word::WordTool;
pub use wps::WpsTool;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn hidden_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    hide_command_window(&mut command);
    command
}

pub fn hide_command_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

pub fn check_by_name(name: &str) -> ToolStatus {
    match name {
        "qpdf" => QpdfTool.check(),
        "ffmpeg" => FfmpegTool.check(),
        "poppler" => PopplerTool.check(),
        "libreoffice" => LibreOfficeTool.check(),
        "word" => WordTool.check(),
        "wps" => WpsTool.check(),
        _ => ToolStatus {
            available: false,
            path: None,
            version: None,
            install_hint: "未知工具".into(),
            managed: false,
            source: "unknown".into(),
        },
    }
}

pub fn install_by_name(name: &str) -> anyhow::Result<String> {
    let installed = match name {
        "qpdf" => QpdfTool.try_install(),
        "ffmpeg" => FfmpegTool.try_install(),
        "poppler" => PopplerTool.try_install(),
        _ => anyhow::bail!("不支持自动安装 {}", name),
    }?;
    validate_tool(name)?;
    Ok(installed)
}

pub fn validate_tool(name: &str) -> anyhow::Result<ToolStatus> {
    let status = check_by_name(name);
    if status.available {
        Ok(status)
    } else {
        anyhow::bail!("工具文件已安装但无法运行：{}", status.install_hint.trim())
    }
}

pub fn command_failure_detail(output: &Output) -> String {
    let code = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "未知".to_string());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = stderr
        .lines()
        .chain(stdout.lines())
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if detail.is_empty() {
        format!("退出码 {code}，命令未返回错误文本")
    } else {
        let detail = detail.chars().take(300).collect::<String>();
        format!("退出码 {code}：{detail}")
    }
}

pub fn command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> anyhow::Result<Output> {
    hide_command_window(command);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout).ok();
            }
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr).ok();
            }
            let status = child.wait()?;
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if start.elapsed() >= timeout {
            child.kill().ok();
            child.wait().ok();
            anyhow::bail!("命令执行超时");
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

pub fn command_output_with_idle_timeout(
    command: &mut Command,
    idle_timeout: Duration,
) -> anyhow::Result<Output> {
    hide_command_window(command);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (tx, rx) = mpsc::channel();
    let stdout_reader = child
        .stdout
        .take()
        .map(|pipe| spawn_stream_reader(pipe, OutputStream::Stdout, tx.clone()));
    let stderr_reader = child
        .stderr
        .take()
        .map(|pipe| spawn_stream_reader(pipe, OutputStream::Stderr, tx));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut last_activity = Instant::now();

    loop {
        drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut last_activity);
        if child.try_wait()?.is_some() {
            let status = child.wait()?;
            join_reader(stdout_reader);
            join_reader(stderr_reader);
            drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut last_activity);
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if last_activity.elapsed() >= idle_timeout {
            child.kill().ok();
            child.wait().ok();
            join_reader(stdout_reader);
            join_reader(stderr_reader);
            drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut last_activity);
            anyhow::bail!("命令连续 {} 秒没有输出，已中止", idle_timeout.as_secs());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

enum OutputStream {
    Stdout,
    Stderr,
}

enum OutputChunk {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
}

fn spawn_stream_reader<R: Read + Send + 'static>(
    mut reader: R,
    stream: OutputStream,
    sender: mpsc::Sender<OutputChunk>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            let chunk = match stream {
                OutputStream::Stdout => OutputChunk::Stdout(buffer[..read].to_vec()),
                OutputStream::Stderr => OutputChunk::Stderr(buffer[..read].to_vec()),
            };
            if sender.send(chunk).is_err() {
                break;
            }
        }
    })
}

fn drain_output_chunks(
    receiver: &mpsc::Receiver<OutputChunk>,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
    last_activity: &mut Instant,
) {
    while let Ok(chunk) = receiver.try_recv() {
        match chunk {
            OutputChunk::Stdout(bytes) => stdout.extend(bytes),
            OutputChunk::Stderr(bytes) => stderr.extend(bytes),
        }
        *last_activity = Instant::now();
    }
}

fn join_reader(handle: Option<thread::JoinHandle<()>>) {
    if let Some(handle) = handle {
        handle.join().ok();
    }
}

/// 支持用户主动取消的子进程执行。
///
/// MDG-001: 不设固定超时——大文件慢就慢，不会被误杀。
/// 通过 CancellationToken 支持用户主动取消。
/// 通过 idle_timeout 可选检测无响应（不自动 kill，返回错误让前端提示用户）。
pub fn command_output_cancellable(
    command: &mut Command,
    token: &tokio_util::sync::CancellationToken,
    idle_timeout: Option<Duration>,
) -> anyhow::Result<Output> {
    hide_command_window(command);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut last_activity = Instant::now();
    let (tx, rx) = mpsc::channel();
    let stdout_reader = child
        .stdout
        .take()
        .map(|pipe| spawn_stream_reader(pipe, OutputStream::Stdout, tx.clone()));
    let stderr_reader = child
        .stderr
        .take()
        .map(|pipe| spawn_stream_reader(pipe, OutputStream::Stderr, tx));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    loop {
        // 收集新输出
        drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut last_activity);

        // 正常结束
        if child.try_wait()?.is_some() {
            drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut last_activity);
            join_reader(stdout_reader);
            join_reader(stderr_reader);
            let status = child.wait()?;
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }

        // 用户主动取消 → kill 子进程
        if token.is_cancelled() {
            child.kill().ok();
            child.wait().ok();
            join_reader(stdout_reader);
            join_reader(stderr_reader);
            drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut last_activity);
            anyhow::bail!("用户取消操作");
        }

        // 无响应检测（可选，不自动 kill）
        if let Some(idle) = idle_timeout {
            if last_activity.elapsed() >= idle {
                // 不 kill，让用户决定
                anyhow::bail!("进程可能无响应（{} 秒无输出）", idle.as_secs());
            }
        }

        thread::sleep(Duration::from_millis(30));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: u32) -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code)
    }

    #[test]
    fn command_failure_includes_exit_code_when_stderr_is_empty() {
        let output = Output {
            status: exit_status(7),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        assert_eq!(
            command_failure_detail(&output),
            "退出码 7，命令未返回错误文本"
        );
    }

    #[test]
    fn command_failure_prefers_first_stderr_line() {
        let output = Output {
            status: exit_status(2),
            stdout: b"stdout detail".to_vec(),
            stderr: b"stderr detail\nmore".to_vec(),
        };
        assert_eq!(command_failure_detail(&output), "退出码 2：stderr detail");
    }
}
