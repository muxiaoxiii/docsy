use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/muxiaoxiii/docsy/releases/download/toolchain-v1/tools-manifest.json";
const DEFAULT_RELEASE_BASE: &str =
    "https://github.com/muxiaoxiii/docsy/releases/download/toolchain-v1";
const MAX_TOOL_MANIFEST_BYTES: u64 = 10 * 1024 * 1024;
const MAX_HTTPS_REDIRECTS: usize = 5;
const MIB: u64 = 1024 * 1024;

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
    #[serde(default)]
    max_bytes: Option<u64>,
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
    app_data_dir().join("Docsy").join("tools")
}

fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
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
    let max_bytes = package_download_limit(name, &package);
    let archive = download_package_to_temp_file(&package.url, max_bytes)?;
    verify_sha256_file_if_present(archive.path(), &package.sha256)?;
    install_package_file(name, &platform, package, archive.path())
}

pub fn install_tool_from_package(name: &str, package_path: &str) -> Result<String> {
    let platform = platform_key()?;
    let path = PathBuf::from(package_path);
    let max_bytes = default_package_download_limit(name);
    let package = ToolPackage {
        version: "local".into(),
        url: path.display().to_string(),
        sha256: String::new(),
        max_bytes: Some(max_bytes),
        binaries: required_binaries(name)?,
    };
    if fs::metadata(&path)
        .with_context(|| format!("读取本地工具包失败: {}", path.display()))?
        .len()
        > max_bytes
    {
        anyhow::bail!("本地工具包超过当前工具的合理体积上限");
    }
    install_package_file(name, &platform, package, &path)
}

fn install_package_file(
    name: &str,
    platform: &str,
    package: ToolPackage,
    archive_path: &Path,
) -> Result<String> {
    let root = tools_root();
    fs::create_dir_all(&root).context("创建工具目录失败")?;
    let staging = root.join(format!("._install_{}_{}", name, unique_suffix()));
    if staging.exists() {
        fs::remove_dir_all(&staging).ok();
    }
    fs::create_dir_all(&staging).context("创建临时工具目录失败")?;

    if let Err(err) = extract_zip_file(archive_path, &staging, package_extract_limit(name)) {
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
        platform: platform.to_string(),
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
                if has_sha256(&package) {
                    return Ok(package);
                }
            }
        }
    }

    if let Ok(settings) = crate::services::history::get_settings() {
        if let Some(manifest_url) = settings.tool_manifest_url {
            let manifest_url = manifest_url.trim();
            if !manifest_url.is_empty() {
                if let Ok(package) = fetch_manifest_package(manifest_url, name, platform) {
                    if has_sha256(&package) {
                        return Ok(package);
                    }
                }
            }
        }
    }

    if let Some(package) = embedded_package_spec(name, platform).filter(has_sha256) {
        return Ok(package);
    }

    if let Ok(package) = fetch_manifest_package(DEFAULT_MANIFEST_URL, name, platform) {
        if has_sha256(&package) {
            return Ok(package);
        }
    }

    anyhow::bail!(
        "无法取得带 SHA256 校验的 {name} 工具包清单。请检查网络或在设置中配置可信 HTTPS 工具清单；也可以选择已下载的本地工具包安装"
    )
}

fn has_sha256(package: &ToolPackage) -> bool {
    !package.sha256.trim().is_empty()
}

fn fetch_manifest_package(url: &str, name: &str, platform: &str) -> Result<ToolPackage> {
    let bytes = download_package(url, MAX_TOOL_MANIFEST_BYTES)?;
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
        "qpdf" => ("12.2.0", required_binaries("qpdf").ok()?),
        "ffmpeg" => ("8.1", required_binaries("ffmpeg").ok()?),
        "poppler" => ("25.12.0", required_binaries("poppler").ok()?),
        _ => return None,
    };
    Some(ToolPackage {
        version: version.to_string(),
        url: format!("{DEFAULT_RELEASE_BASE}/{name}-{version}-{platform}.zip"),
        sha256: String::new(),
        max_bytes: None,
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
            max_bytes: None,
            binaries: vec![binary_name("qpdf")],
        }),
        "ffmpeg" => Some(ToolPackage {
            version: "release-essentials".into(),
            url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip".into(),
            sha256: String::new(),
            max_bytes: None,
            binaries: vec![binary_name("ffmpeg"), binary_name("ffprobe")],
        }),
        "poppler" => Some(ToolPackage {
            version: "26.02.0-0".into(),
            url: "https://github.com/oschwartz10612/poppler-windows/releases/download/v26.02.0-0/Release-26.02.0-0.zip"
                .into(),
            sha256: String::new(),
            max_bytes: None,
            binaries: vec![binary_name("pdftoppm"), binary_name("pdftotext")],
        }),
        _ => None,
    }
}

fn download_package(url: &str, max_bytes: u64) -> Result<Vec<u8>> {
    validate_download_url(url)?;
    let mut current =
        reqwest::Url::parse(url).with_context(|| format!("工具下载地址无效: {url}"))?;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("初始化下载客户端失败")?;

    for redirect_count in 0..=MAX_HTTPS_REDIRECTS {
        let response = client
            .get(current.clone())
            .send()
            .with_context(|| format!("下载失败: {current}"))?;
        let status = response.status();
        if status.is_redirection() {
            if redirect_count == MAX_HTTPS_REDIRECTS {
                anyhow::bail!("工具下载重定向次数过多，已中止下载");
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .context("工具下载重定向缺少有效的 Location 地址")?;
            current = resolve_https_redirect(&current, location)?;
            continue;
        }
        if !status.is_success() {
            anyhow::bail!(
                "下载失败: {current} 返回 {status}。可在设置中改用国内镜像清单，或从本地 zip 工具包安装。"
            );
        }
        if let Some(length) = response.content_length() {
            if length > max_bytes {
                anyhow::bail!("工具包过大，已拒绝下载");
            }
        }
        let mut reader = response.take(max_bytes + 1);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).context("读取下载内容失败")?;
        if bytes.len() as u64 > max_bytes {
            anyhow::bail!("工具包过大，已中止下载");
        }
        return Ok(bytes);
    }

    unreachable!("重定向循环必须在上方返回或报错")
}

struct TempArchive {
    path: PathBuf,
}

impl TempArchive {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempArchive {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn download_package_to_temp_file(url: &str, max_bytes: u64) -> Result<TempArchive> {
    validate_download_url(url)?;
    let mut current = reqwest::Url::parse(url).with_context(|| format!("工具下载地址无效: {url}"))?;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("初始化下载客户端失败")?;

    for redirect_count in 0..=MAX_HTTPS_REDIRECTS {
        let response = client.get(current.clone()).send().with_context(|| format!("下载失败: {current}"))?;
        let status = response.status();
        if status.is_redirection() {
            if redirect_count == MAX_HTTPS_REDIRECTS {
                anyhow::bail!("工具下载重定向次数过多，已中止下载");
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .context("工具下载重定向缺少有效的 Location 地址")?;
            current = resolve_https_redirect(&current, location)?;
            continue;
        }
        if !status.is_success() {
            anyhow::bail!("下载失败: {current} 返回 {status}");
        }
        if response.content_length().is_some_and(|length| length > max_bytes) {
            anyhow::bail!("工具包过大，已拒绝下载");
        }
        let path = std::env::temp_dir().join(format!("docsy-tool-{}.zip", unique_suffix()));
        let mut output = fs::File::create(&path).context("创建工具下载临时文件失败")?;
        let mut reader = response.take(max_bytes + 1);
        let mut total = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = reader.read(&mut buffer).context("读取下载内容失败")?;
            if count == 0 {
                break;
            }
            total = total.saturating_add(count as u64);
            if total > max_bytes {
                let _ = fs::remove_file(&path);
                anyhow::bail!("工具包过大，已中止下载");
            }
            std::io::Write::write_all(&mut output, &buffer[..count]).context("写入工具下载临时文件失败")?;
        }
        return Ok(TempArchive { path });
    }
    unreachable!("重定向循环必须在上方返回或报错")
}

fn validate_download_url(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url).with_context(|| format!("工具下载地址无效: {url}"))?;
    validate_https_url(&parsed)
}

fn resolve_https_redirect(current: &reqwest::Url, location: &str) -> Result<reqwest::Url> {
    let target = current
        .join(location)
        .with_context(|| format!("工具下载重定向地址无效: {location}"))?;
    validate_https_url(&target)?;
    Ok(target)
}

fn validate_https_url(parsed: &reqwest::Url) -> Result<()> {
    if parsed.scheme() != "https" {
        anyhow::bail!("工具下载地址必须使用 HTTPS");
    }
    if parsed.host_str().is_none() {
        anyhow::bail!("工具下载地址缺少主机名");
    }
    Ok(())
}

fn package_download_limit(name: &str, package: &ToolPackage) -> u64 {
    package
        .max_bytes
        .unwrap_or_else(|| default_package_download_limit(name))
}

fn default_package_download_limit(name: &str) -> u64 {
    match name {
        "ffmpeg" => 2048 * MIB,
        "poppler" => 1536 * MIB,
        "qpdf" => 512 * MIB,
        _ => 1024 * MIB,
    }
}

fn package_extract_limit(name: &str) -> u64 {
    match name {
        "ffmpeg" => 4096 * MIB,
        "poppler" => 3072 * MIB,
        "qpdf" => 1024 * MIB,
        _ => 2048 * MIB,
    }
}

fn required_binaries(name: &str) -> Result<Vec<String>> {
    match name {
        "qpdf" => Ok(vec![binary_name("qpdf")]),
        "ffmpeg" => Ok(vec![binary_name("ffmpeg"), binary_name("ffprobe")]),
        "poppler" => Ok(vec![binary_name("pdftoppm"), binary_name("pdftotext")]),
        _ => anyhow::bail!("不支持安装工具 {name}"),
    }
}

fn verify_sha256_file_if_present(path: &Path, expected: &str) -> Result<()> {
    let expected = expected.trim();
    if expected.is_empty() {
        anyhow::bail!("自动下载的工具包缺少 SHA256 校验值，已拒绝安装");
    }
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = hex_lower(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        anyhow::bail!("工具包校验失败: sha256 不匹配");
    }
    Ok(())
}

fn extract_zip_file(archive_path: &Path, output_dir: &Path, max_total_uncompressed: u64) -> Result<()> {
    let reader = fs::File::open(archive_path).context("读取工具 zip 失败")?;
    let mut archive = zip::ZipArchive::new(reader).context("读取工具 zip 失败")?;
    let mut total_uncompressed = 0_u64;
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
        let entry_size = file.size();
        if entry_size > max_total_uncompressed {
            anyhow::bail!("工具包内文件过大，已拒绝解压");
        }
        total_uncompressed = total_uncompressed.saturating_add(entry_size);
        if total_uncompressed > max_total_uncompressed {
            anyhow::bail!("工具包解压后体积过大，已拒绝解压");
        }
        let mut output = fs::File::create(&output_path).context("创建解压文件失败")?;
        std::io::copy(&mut file, &mut output).context("解压 zip 文件失败")?;
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
    let mut command = std::process::Command::new(command);
    command.arg(binary);
    let output = super::command_output_with_timeout(&mut command, Duration::from_secs(2)).ok()?;
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

    #[test]
    fn download_url_must_be_https() {
        assert!(validate_download_url("https://example.com/tools.zip").is_ok());
        assert!(validate_download_url("http://example.com/tools.zip").is_err());
        assert!(validate_download_url("file:///tmp/tools.zip").is_err());
    }

    #[test]
    fn redirects_must_remain_https() {
        let source = reqwest::Url::parse("https://example.com/tools/latest.zip").unwrap();
        let relative = resolve_https_redirect(&source, "files/tool.zip").unwrap();
        assert_eq!(
            relative.as_str(),
            "https://example.com/tools/files/tool.zip"
        );
        assert!(resolve_https_redirect(&source, "http://mirror.example.com/tool.zip").is_err());
        assert!(resolve_https_redirect(&source, "file:///tmp/tool.zip").is_err());
    }

    #[test]
    fn package_limits_are_tool_aware_and_manifest_overridable() {
        let package = ToolPackage {
            version: "test".into(),
            url: "https://example.com/tool.zip".into(),
            sha256: String::new(),
            max_bytes: Some(123),
            binaries: vec![],
        };

        assert_eq!(package_download_limit("ffmpeg", &package), 123);
        let default_package = ToolPackage {
            max_bytes: None,
            ..package
        };
        assert!(package_download_limit("ffmpeg", &default_package) > 512 * MIB);
        assert!(
            package_extract_limit("ffmpeg") > package_download_limit("ffmpeg", &default_package)
        );
    }
}
