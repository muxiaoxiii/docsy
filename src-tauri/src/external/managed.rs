use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/muxiaoxiii/docsy/releases/download/toolchain-v1/tools-manifest.json";
const DEFAULT_RELEASE_BASE: &str =
    "https://github.com/muxiaoxiii/docsy/releases/download/toolchain-v1";

#[derive(Debug, Clone, Deserialize)]
struct ToolManifest {
    tools: std::collections::BTreeMap<String, std::collections::BTreeMap<String, ToolPackage>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolPackage {
    version: String,
    url: String,
    #[serde(default)]
    sha256: String,
    binaries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallRecord {
    name: String,
    platform: String,
    version: String,
    url: String,
    binaries: Vec<String>,
}

pub fn tools_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Docsy")
        .join("tools")
}

pub fn open_tools_root() -> Result<()> {
    let root = tools_root();
    fs::create_dir_all(&root).context("创建工具目录失败")?;
    open::that(&root).map_err(|e| anyhow::anyhow!(e))
}

pub fn managed_binary_path(tool: &str, binary: &str) -> Option<PathBuf> {
    let root = tools_root().join(tool);
    find_binary_in_dir(&root, binary)
}

pub fn install_tool(name: &str) -> Result<String> {
    let platform = platform_key()?;
    let package = load_package_spec(name, &platform)?;
    let bytes = download_package(&package.url)?;
    verify_sha256_if_present(&bytes, &package.sha256)?;

    let root = tools_root();
    fs::create_dir_all(&root).context("创建工具目录失败")?;
    let staging = root.join(format!("._install_{}_{}", name, unique_suffix()));
    if staging.exists() {
        fs::remove_dir_all(&staging).ok();
    }
    fs::create_dir_all(&staging).context("创建临时工具目录失败")?;

    if let Err(err) = extract_zip(&bytes, &staging) {
        fs::remove_dir_all(&staging).ok();
        return Err(err);
    }

    for binary in &package.binaries {
        let path = find_binary_in_dir(&staging, binary)
            .with_context(|| format!("工具包中未找到可执行文件 {binary}"))?;
        make_executable(&path)?;
    }

    let install_dir = root.join(name);
    let backup = root.join(format!("._backup_{}_{}", name, unique_suffix()));
    if install_dir.exists() {
        fs::rename(&install_dir, &backup).context("备份旧工具目录失败")?;
    }
    if let Err(err) = fs::rename(&staging, &install_dir).context("安装工具目录失败") {
        if backup.exists() {
            fs::rename(&backup, &install_dir).ok();
        }
        return Err(err);
    }
    if backup.exists() {
        fs::remove_dir_all(backup).ok();
    }

    let record = InstallRecord {
        name: name.to_string(),
        platform,
        version: package.version.clone(),
        url: package.url,
        binaries: package.binaries,
    };
    let record_json = serde_json::to_vec_pretty(&record)?;
    fs::write(install_dir.join(".docsy-tool.json"), record_json).context("写入工具安装记录失败")?;

    Ok(format!("{} 已安装到 Docsy 工具目录", name))
}

fn load_package_spec(name: &str, platform: &str) -> Result<ToolPackage> {
    if let Ok(manifest_url) = std::env::var("DOCSY_TOOL_MANIFEST_URL") {
        if !manifest_url.trim().is_empty() {
            if let Ok(package) = fetch_manifest_package(&manifest_url, name, platform) {
                return Ok(package);
            }
        }
    }

    if let Ok(package) = fetch_manifest_package(DEFAULT_MANIFEST_URL, name, platform) {
        return Ok(package);
    }

    embedded_package_spec(name, platform).with_context(|| {
        format!("当前平台 {platform} 暂不支持自动安装 {name}，请先手动安装或发布 Docsy 工具包")
    })
}

fn fetch_manifest_package(url: &str, name: &str, platform: &str) -> Result<ToolPackage> {
    let bytes = download_package(url)?;
    let manifest: ToolManifest = serde_json::from_slice(&bytes).context("解析工具清单失败")?;
    manifest
        .tools
        .get(name)
        .and_then(|by_platform| by_platform.get(platform))
        .cloned()
        .with_context(|| format!("工具清单中没有 {name} / {platform}"))
}

fn embedded_package_spec(name: &str, platform: &str) -> Option<ToolPackage> {
    if platform == "windows-x86_64" {
        return embedded_windows_package_spec(name);
    }

    let (version, binaries) = match name {
        "qpdf" => ("12.2.0", vec![binary_name("qpdf")]),
        "ffmpeg" => ("8.1", vec![binary_name("ffmpeg"), binary_name("ffprobe")]),
        "poppler" => (
            "25.12.0",
            vec![binary_name("pdftoppm"), binary_name("pdftotext")],
        ),
        _ => return None,
    };
    Some(ToolPackage {
        version: version.to_string(),
        url: format!("{DEFAULT_RELEASE_BASE}/{name}-{version}-{platform}.zip"),
        sha256: String::new(),
        binaries,
    })
}

fn embedded_windows_package_spec(name: &str) -> Option<ToolPackage> {
    match name {
        "qpdf" => Some(ToolPackage {
            version: "12.3.2".into(),
            url: "https://github.com/qpdf/qpdf/releases/download/v12.3.2/qpdf-12.3.2-msvc64.zip"
                .into(),
            sha256: "8941870a604e7c87ed24566b038d46c24ce76616254d2383c578f60c0677f202"
                .into(),
            binaries: vec![binary_name("qpdf")],
        }),
        "ffmpeg" => Some(ToolPackage {
            version: "release-essentials".into(),
            url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip".into(),
            sha256: String::new(),
            binaries: vec![binary_name("ffmpeg"), binary_name("ffprobe")],
        }),
        "poppler" => Some(ToolPackage {
            version: "26.02.0-0".into(),
            url: "https://github.com/oschwartz10612/poppler-windows/releases/download/v26.02.0-0/Release-26.02.0-0.zip"
                .into(),
            sha256: String::new(),
            binaries: vec![binary_name("pdftoppm"), binary_name("pdftotext")],
        }),
        _ => None,
    }
}

fn download_package(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::blocking::get(url).with_context(|| format!("下载失败: {url}"))?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!(
            "下载失败: {url} 返回 {status}。如果这是 Docsy 托管工具，请先发布对应工具包。"
        );
    }
    Ok(response.bytes()?.to_vec())
}

fn verify_sha256_if_present(bytes: &[u8], expected: &str) -> Result<()> {
    let expected = expected.trim();
    if expected.is_empty() {
        return Ok(());
    }
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual = hex_lower(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        anyhow::bail!("工具包校验失败: sha256 不匹配");
    }
    Ok(())
}

fn extract_zip(bytes: &[u8], output_dir: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).context("读取工具 zip 失败")?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).context("读取 zip 条目失败")?;
        let Some(enclosed) = file.enclosed_name().map(Path::to_path_buf) else {
            continue;
        };
        let output_path = output_dir.join(enclosed);
        if file.is_dir() {
            fs::create_dir_all(&output_path).context("创建 zip 目录失败")?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).context("创建 zip 输出目录失败")?;
        }
        let mut output = fs::File::create(&output_path).context("创建解压文件失败")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).context("读取 zip 文件失败")?;
        output.write_all(&buffer).context("写入解压文件失败")?;
    }
    Ok(())
}

fn find_binary_in_dir(dir: &Path, binary: &str) -> Option<PathBuf> {
    if !dir.exists() {
        return None;
    }
    let candidates = [dir.join(binary), dir.join("bin").join(binary)];
    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(current).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.file_name().and_then(|s| s.to_str()) == Some(binary) {
                return Some(path);
            }
        }
    }
    None
}

fn platform_key() -> Result<String> {
    let os = match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        other => anyhow::bail!("暂不支持自动安装工具到 {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        other => anyhow::bail!("暂不支持当前 CPU 架构 {other}"),
    };
    Ok(format!("{os}-{arch}"))
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).context("读取工具权限失败")?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions).context("设置工具可执行权限失败")?;
    }
    Ok(())
}

fn unique_suffix() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}_{}", std::process::id(), ts)
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn find_on_path(binary: &str) -> Option<PathBuf> {
    let command = if cfg!(windows) { "where" } else { "which" };
    let output = std::process::Command::new(command)
        .arg(binary)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_platform_binary_name() {
        let name = binary_name("qpdf");
        if cfg!(windows) {
            assert_eq!(name, "qpdf.exe");
        } else {
            assert_eq!(name, "qpdf");
        }
    }

    #[test]
    fn hex_encodes_lowercase() {
        assert_eq!(hex_lower(&[0, 15, 255]), "000fff");
    }
}
