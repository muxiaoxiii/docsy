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

const MAX_CAPTURED_OUTPUT_BYTES: usize = 1024 * 1024;

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
    let mut dummy_activity = Instant::now();
    let start = Instant::now();

    loop {
        drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut dummy_activity);
        if child.try_wait()?.is_some() {
            let status = child.wait()?;
            join_reader(stdout_reader);
            join_reader(stderr_reader);
            drain_output_chunks(&rx, &mut stdout, &mut stderr, &mut dummy_activity);
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if start.elapsed() >= timeout {
            child.kill().ok();
            child.wait().ok();
            join_reader(stdout_reader);
            join_reader(stderr_reader);
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

/// Drain a subprocess pipe without allowing diagnostic output to grow without
/// bound. The reader always consumes the complete stream, but retains at most
/// the first MiB for error reporting.
pub(crate) fn spawn_bounded_output_reader<R: Read + Send + 'static>(
    mut reader: R,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; 8192];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            let remaining = MAX_CAPTURED_OUTPUT_BYTES.saturating_sub(captured.len());
            captured.extend_from_slice(&buffer[..read.min(remaining)]);
        }
        captured
    })
}

pub(crate) fn finish_bounded_output_reader(handle: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

pub fn has_homebrew() -> bool {
    #[cfg(target_os = "macos")]
    {
        for path in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
            if std::path::Path::new(path).exists() {
                return true;
            }
        }
        managed::find_on_path("brew").is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn run_in_terminal(command: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"tell application "Terminal"
    activate
    do script "{}"
end tell"#,
            escaped
        );
        let mut cmd = hidden_command("osascript");
        cmd.arg("-e").arg(script);
        let output = cmd.output()?;
        if !output.status.success() {
            anyhow::bail!(command_failure_detail(&output));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = command;
        anyhow::bail!("当前系统不支持通过终端自动安装")
    }
}

fn wrap_terminal_script(title: &str, body: &str, tool_name: &str) -> String {
    format!(
        r#"clear
export NONINTERACTIVE=1
export HOMEBREW_NO_AUTO_UPDATE=1
export HOMEBREW_NO_INSTALL_CLEANUP=1
export CI=1
echo "=== Docsy: {title} ==="
echo ""
{body}
STATUS=$?
echo ""
if [ $STATUS -eq 0 ]; then
    echo "========================================="
    echo "  🎉 {tool_name} 安装成功！"
    echo "  请返回 Docsy 点击「检测此工具」确认状态。"
    echo "  终端窗口将在 3 秒后自动关闭..."
    echo "========================================="
    sleep 3
    osascript -e 'tell application "Terminal" to close front window' & exit
else
    echo "========================================="
    echo "  ❌ 安装未完成或遇到错误（退出码: $STATUS）。"
    echo "  请查看上方日志排查原因。按任意键关闭此窗口..."
    echo "========================================="
    read -n 1
    osascript -e 'tell application "Terminal" to close front window' & exit
fi
"#
    )
}

fn homebrew_tsinghua_install_script() -> String {
    r#"clear
echo "============================================================"
echo "  Docsy: 正在通过 清华大学开源软件镜像站 安装 Homebrew"
echo "  （教育骨干网高速节点，无 Gitee 限流排队与第三方风险）"
echo "============================================================"
echo ""

# 1. 导出清华大学镜像源环境变量
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"
export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"
export HOMEBREW_PIP_INDEX_URL="https://pypi.tuna.tsinghua.edu.cn/simple"
export NONINTERACTIVE=1
export CI=1

# 2. 从清华大学镜像站快速拉取官方安装程序
TMP_DIR=$(mktemp -d /tmp/docsy-brew-XXXXXX)
cd "$TMP_DIR"
echo "--> 1/3: 正在从清华镜像拉取官方安装脚本..."
git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git brew-install

echo ""
echo "--> 2/3: 开始执行 Homebrew 安装..."
echo "⚠️  若系统弹出开机密码提示，请直接盲敲键盘输入密码并按回车（无星号显示属于正常安全机制）"
echo ""

/bin/bash brew-install/install.sh
INSTALL_STATUS=$?

# 清理临时下载目录
cd /
rm -rf "$TMP_DIR"

if [ $INSTALL_STATUS -eq 0 ]; then
    echo ""
    echo "--> 3/3: 正在配置永久清华大学镜像与终端环境变量..."
    
    BREW_BIN="/opt/homebrew/bin/brew"
    [ ! -f "$BREW_BIN" ] && BREW_BIN="/usr/local/bin/brew"
    
    if [ -f "$BREW_BIN" ]; then
        eval "$($BREW_BIN shellenv)"
        
        ZPROFILE="$HOME/.zprofile"
        touch "$ZPROFILE"
        
        if ! grep -q 'brew shellenv' "$ZPROFILE" 2>/dev/null; then
            echo "eval \"\$($BREW_BIN shellenv)\"" >> "$ZPROFILE"
        fi
        if ! grep -q 'HOMEBREW_BREW_GIT_REMOTE' "$ZPROFILE" 2>/dev/null; then
            echo 'export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"' >> "$ZPROFILE"
            echo 'export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"' >> "$ZPROFILE"
            echo 'export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"' >> "$ZPROFILE"
            echo 'export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"' >> "$ZPROFILE"
        fi
    fi

    echo ""
    echo "============================================================"
    echo "  🎉 Homebrew 安装并配置清华镜像成功！"
    echo "  请返回 Docsy 继续完成外部工具组件配置。"
    echo "  终端窗口将在 3 秒后自动关闭..."
    echo "============================================================"
    sleep 3
    osascript -e 'tell application "Terminal" to close front window' & exit
else
    echo ""
    echo "============================================================"
    echo "  ❌ 安装未完成或遇到错误（退出码: $INSTALL_STATUS）。"
    echo "  请查看上方日志。按任意键关闭此窗口..."
    echo "============================================================"
    read -n 1
    osascript -e 'tell application "Terminal" to close front window' & exit
fi
"#.to_string()
}

pub fn install_via_terminal(tool_name: &str) -> anyhow::Result<()> {
    let script = match tool_name {
        "homebrew" => homebrew_tsinghua_install_script(),
        "ffmpeg" => wrap_terminal_script(
            "正在通过 Homebrew 安装 FFmpeg (Full 完整版，含 drawtext 水印)",
            r#"echo "1/2: 添加 ffmpeg tap 仓库..."
yes | brew tap homebrew-ffmpeg/ffmpeg
echo "2/2: 安装 ffmpeg-full (自动跳过确认)..."
yes | brew install homebrew-ffmpeg/ffmpeg/ffmpeg-full || yes | brew install ffmpeg-full || yes | brew install ffmpeg"#,
            "FFmpeg",
        ),
        "qpdf" => wrap_terminal_script(
            "正在通过 Homebrew 安装 Qpdf",
            r#"yes | brew install qpdf"#,
            "Qpdf",
        ),
        "poppler" => wrap_terminal_script(
            "正在通过 Homebrew 安装 Poppler",
            r#"yes | brew install poppler"#,
            "Poppler",
        ),
        other => anyhow::bail!("不支持通过终端安装工具 {other}"),
    };
    run_in_terminal(&script)
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

    #[test]
    fn bounded_output_reader_drains_but_caps_diagnostics() {
        let input = std::io::Cursor::new(vec![b'x'; MAX_CAPTURED_OUTPUT_BYTES + 4096]);
        let output = finish_bounded_output_reader(Some(spawn_bounded_output_reader(input)));
        assert_eq!(output.len(), MAX_CAPTURED_OUTPUT_BYTES);
    }
}
