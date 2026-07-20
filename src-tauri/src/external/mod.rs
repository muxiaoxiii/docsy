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
    match name {
        "qpdf" => QpdfTool.try_install(),
        "ffmpeg" => FfmpegTool.try_install(),
        "poppler" => PopplerTool.try_install(),
        _ => anyhow::bail!("不支持自动安装 {}", name),
    }
}

pub fn command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> anyhow::Result<Output> {
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
        loop {
            let Ok(read) = reader.read(&mut buffer) else {
                break;
            };
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
