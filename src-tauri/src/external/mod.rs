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

#[cfg(target_os = "macos")]
fn install_via_homebrew_background(name: &str) -> anyhow::Result<String> {
    let brew_bin = if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
        "/opt/homebrew/bin/brew"
    } else if std::path::Path::new("/usr/local/bin/brew").exists() {
        "/usr/local/bin/brew"
    } else {
        "brew"
    };

    let packages: Vec<&str> = match name {
        "ffmpeg" => vec!["ffmpeg"],
        "poppler" => vec!["poppler"],
        "qpdf" => vec!["qpdf"],
        _ => anyhow::bail!("未知工具: {}", name),
    };

    let mut cmd = hidden_command(brew_bin);
    cmd.arg("install");
    cmd.args(&packages);
    cmd.env("HOMEBREW_API_DOMAIN", "https://mirrors.ustc.edu.cn/homebrew-bottles/api");
    cmd.env("HOMEBREW_BOTTLE_DOMAIN", "https://mirrors.ustc.edu.cn/homebrew-bottles");
    cmd.env("HOMEBREW_NO_AUTO_UPDATE", "1");
    cmd.env("NONINTERACTIVE", "1");

    let output = cmd.output()?;
    if !output.status.success() {
        anyhow::bail!(command_failure_detail(&output));
    }
    validate_tool(name)?;
    Ok(format!("{} 安装成功", name))
}

pub fn install_by_name(name: &str) -> anyhow::Result<String> {
    #[cfg(target_os = "macos")]
    if has_homebrew() {
        return install_via_homebrew_background(name);
    }

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
        let script_path = crate::util::fs::temp_named_path("docsy_run", "sh");
        let script_str = script_path.to_string_lossy().to_string();
        let script_content = format!(
            "#!/bin/bash\nDOCSY_TEMP_SCRIPT=\"{}\"\ntrap 'rm -f \"$DOCSY_TEMP_SCRIPT\"' EXIT INT TERM\n{}\n",
            script_str.replace('"', "\\\""),
            command
        );
        std::fs::write(&script_path, script_content)?;

        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&script_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&script_path, perms);
        }

        let script = r#"on run argv
    set scriptPath to item 1 of argv
    tell application "Terminal"
        activate
        do script "/bin/bash " & quoted form of scriptPath
    end tell
end run"#;
        let mut cmd = hidden_command("osascript");
        cmd.arg("-e").arg(script).arg(&script_str);
        let output = cmd.output()?;
        if !output.status.success() {
            let _ = std::fs::remove_file(&script_path);
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
echo "=== Docsy: {title} ==="
echo ""
{body}
STATUS=$?
echo ""
if [ $STATUS -eq 0 ]; then
    echo "========================================="
    echo "  🎉 {tool_name} 安装成功！"
    echo "  Docsy 正在实时检测并同步状态，窗口即将自动关闭..."
    echo "========================================="
    sleep 1
    MY_TTY=$(tty)
    osascript -e "tell application \"Terminal\" to close (every window whose tty of selected tab is \"$MY_TTY\")"
else
    echo "========================================="
    echo "  ❌ 安装未完成或遇到错误（退出码: $STATUS）。"
    echo "  请查看上方日志排查原因。按回车键退出..."
    echo "========================================="
    read -r
    MY_TTY=$(tty)
    osascript -e "tell application \"Terminal\" to close (every window whose tty of selected tab is \"$MY_TTY\")"
fi
"#
    )
}

fn homebrew_bootstrap_sh() -> &'static str {
    r#": # 自动检查并确保 Homebrew 运行环境与国内极速镜像
if [ "$(uname -m)" = "arm64" ]; then
    BREW_PREFIX="/opt/homebrew"
else
    BREW_PREFIX="/usr/local/Homebrew"
fi

if [ -f /opt/homebrew/bin/brew ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -f /usr/local/bin/brew ]; then
    eval "$(/usr/local/bin/brew shellenv)"
fi

if ! command -v brew >/dev/null 2>&1 && [ ! -f "$BREW_PREFIX/bin/brew" ]; then
    echo "============================================================"
    echo "  Docsy: 正在为您内置部署 Homebrew (中国科大/清华极速镜像免确认版)"
    echo "============================================================"
    echo ""

    if [ ! -d "$BREW_PREFIX" ] || [ ! -w "$BREW_PREFIX" ]; then
        echo "--> 1/3: 正在初始化 Homebrew 安装目录..."
        echo "⚠️  若系统提示 Password:，请直接盲敲输入电脑开机密码并按回车（输密码不显字符属于正常机制）："
        sudo mkdir -p "$BREW_PREFIX"
        sudo chown -R "$(whoami):admin" "$BREW_PREFIX"
    fi

    echo "--> 2/3: 正在通过高速 CDN 节点极速拉取部署 Homebrew 核心..."
    cd "$BREW_PREFIX"
    git init -q
    git config remote.origin.url "https://mirrors.ustc.edu.cn/brew.git"
    git config remote.origin.fetch "+refs/heads/*:refs/remotes/origin/*"
    git fetch --depth=1 origin main || {
        echo "中科大源受阻，自动切换为清华大学镜像重试..."
        git config remote.origin.url "https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
        git fetch --depth=1 origin main
    }
    git reset --hard FETCH_HEAD

    if [ "$(uname -m)" != "arm64" ] && [ ! -f /usr/local/bin/brew ]; then
        sudo mkdir -p /usr/local/bin
        sudo ln -sf "$BREW_PREFIX/bin/brew" /usr/local/bin/brew
    fi

    echo "--> 3/3: 正在配置终端国内极速镜像源与环境变量..."
    eval "$("$BREW_PREFIX/bin/brew" shellenv)"

    ZPROFILE="$HOME/.zprofile"
    touch "$ZPROFILE"
    if ! grep -q 'brew shellenv' "$ZPROFILE" 2>/dev/null; then
        echo "eval \"\$($BREW_PREFIX/bin/brew shellenv)\"" >> "$ZPROFILE"
    fi
    if ! grep -q 'HOMEBREW_API_DOMAIN' "$ZPROFILE" 2>/dev/null; then
        echo 'export HOMEBREW_API_DOMAIN="https://mirrors.ustc.edu.cn/homebrew-bottles/api"' >> "$ZPROFILE"
        echo 'export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.ustc.edu.cn/homebrew-bottles"' >> "$ZPROFILE"
        echo 'export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.ustc.edu.cn/brew.git"' >> "$ZPROFILE"
    fi
fi

# 全局开启非交互及国内极速镜像通道
export HOMEBREW_API_DOMAIN="https://mirrors.ustc.edu.cn/homebrew-bottles/api"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.ustc.edu.cn/homebrew-bottles"
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.ustc.edu.cn/brew.git"
export HOMEBREW_NO_AUTO_UPDATE=1
export HOMEBREW_NO_INSTALL_CLEANUP=1
export NONINTERACTIVE=1
"#
}

fn homebrew_ustc_install_script() -> String {
    format!(
        r#"clear
{bootstrap}
BREW_BIN="/opt/homebrew/bin/brew"
[ ! -f "$BREW_BIN" ] && BREW_BIN="/usr/local/bin/brew"

if [ -f "$BREW_BIN" ]; then
    echo ""
    echo "============================================================"
    echo "  🎉 Homebrew 已成功内置部署就绪（中科大/清华极速镜像）！"
    echo "  Docsy 正在实时检测状态，窗口即将自动关闭..."
    echo "============================================================"
    sleep 1
    MY_TTY=$(tty)
    osascript -e "tell application \"Terminal\" to close (every window whose tty of selected tab is \"$MY_TTY\")"
else
    echo ""
    echo "============================================================"
    echo "  ❌ Homebrew 部署未完成，请查看上方日志排查。"
    echo "  按回车键退出..."
    echo "============================================================"
    read -r
    MY_TTY=$(tty)
    osascript -e "tell application \"Terminal\" to close (every window whose tty of selected tab is \"$MY_TTY\")"
fi
"#,
        bootstrap = homebrew_bootstrap_sh()
    )
}

pub fn install_tools_via_terminal(tools: &[String]) -> anyhow::Result<()> {
    if tools.is_empty() {
        return Ok(());
    }
    if tools.len() == 1 && tools[0] == "homebrew" {
        return run_in_terminal(&homebrew_ustc_install_script());
    }

    let mut packages = Vec::new();
    let mut names = Vec::new();

    for t in tools {
        match t.as_str() {
            "ffmpeg" => {
                packages.push("ffmpeg");
                names.push("FFmpeg");
            }
            "poppler" => {
                packages.push("poppler");
                names.push("Poppler");
            }
            "qpdf" => {
                packages.push("qpdf");
                names.push("Qpdf");
            }
            _ => {}
        }
    }

    if packages.is_empty() {
        return Ok(());
    }

    let pkgs_str = packages.join(" ");
    let names_str = names.join("、");

    let body = format!(
        r#"{bootstrap}
echo "--> 正在通过 Homebrew 安装所选组件: {pkgs_str}..."
yes | brew install {pkgs_str}"#,
        bootstrap = homebrew_bootstrap_sh(),
        pkgs_str = pkgs_str
    );

    let script = wrap_terminal_script(
        &format!("正在安装 {}", names_str),
        &body,
        &names_str,
    );
    run_in_terminal(&script)
}

pub fn install_via_terminal(tool_name: &str) -> anyhow::Result<()> {
    let tools: Vec<String> = tool_name
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    install_tools_via_terminal(&tools)
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
