//! 通用文件系统工具：路径比较、临时路径、唯一输出名、临时文件守卫。
//!
//! 这些实现原散落在 `pdf/mod.rs`、`pdf/qpdf.rs`、`pdf/header_footer.rs`、
//! `docx_template/batch.rs`、`docx_template/engine.rs`、`image_paddler.rs` 中，
//! 已收敛到本模块统一维护。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 比较两个路径是否指向同一文件。
///
/// 优先 canonicalize（解析符号链接 + 绝对化），失败时回退到手动绝对化 + 规范化。
pub fn same_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        return canon;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let normalized = normalize_path_components(&absolute);
    if let Ok(canon) = normalized.canonicalize() {
        return canon;
    }
    if let Some(parent) = normalized.parent() {
        if let Ok(parent_canon) = parent.canonicalize() {
            if let Some(name) = normalized.file_name() {
                return parent_canon.join(name);
            }
        }
    }
    normalized
}

fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

/// 生成进程内和并发任务间都不会碰撞的临时文件路径。
///
/// 注意：只生成路径，不创建文件；文件权限需在后续写入点用
/// [`set_private_permissions`] 收紧。
pub fn temp_named_path(prefix: &str, extension: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let sequence = TEMP_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    if extension.is_empty() {
        std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}_{sequence}"))
    } else {
        std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}_{sequence}.{extension}"))
    }
}

/// 在目标文件同目录创建临时路径，供完成写入后原子替换目标文件使用。
///
/// 同目录可避免跨卷 `rename` 失败，也不会把未完成内容暴露为目标文件。
pub fn sibling_temp_path(target: &Path, prefix: &str) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    unique_output_path(
        parent,
        &format!(".{prefix}-{}-{stamp}", std::process::id()),
        extension,
    )
}

/// 用同目录临时文件安全替换目标文件。
///
/// Windows 不能直接 rename 覆盖已有文件，因此先将旧文件改名为备份；若替换失败，
/// 会尽力恢复旧文件。调用方应确保 `from` 和 `to` 在同一目录。
pub fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        return std::fs::rename(from, to);
    }
    #[cfg(not(unix))]
    {
        let backup = sibling_temp_path(to, "docsy-backup");
        std::fs::rename(to, &backup)?;
        match std::fs::rename(from, to) {
            Ok(()) => {
                let _ = std::fs::remove_file(&backup);
                Ok(())
            }
            Err(error) => {
                let _ = std::fs::rename(&backup, to);
                Err(error)
            }
        }
    }
}

/// 将文件名主干转为文件系统安全字符串。
///
/// - 去除尾部 `.pdf` 后缀
/// - 将 `/\:*?"<>|\0` 及 `.` 替换为 `_`
/// - 去除首尾 `_`、空格、`-`
/// - 空结果回退为 `"output"`
pub fn safe_file_stem(input: &str) -> String {
    let stripped = input.strip_suffix(".pdf").unwrap_or(input);
    let mut out = String::new();
    for ch in stripped.chars() {
        if matches!(
            ch,
            '/' | '\\'
                | ':'
                | '*'
                | '?'
                | '"'
                | '<'
                | '>'
                | '|'
                | '\0'
                | '.'
                | '（'
                | '）'
                | '('
                | ')'
        ) {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim_matches(|c| matches!(c, ' ' | '_' | '-'));
    if trimmed.is_empty() {
        "output".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 在 `dir` 中生成一个不与已有文件冲突的路径：`{dir}/{stem}.{ext}`。
///
/// 若 `stem.{ext}` 已存在，则依次尝试 `stem-2.{ext}`、`stem-3.{ext}` ……；
/// 超过 10 000 次后回退到带时间戳的文件名。
pub fn unique_output_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let candidate = if ext.is_empty() {
        dir.join(stem)
    } else {
        dir.join(format!("{stem}.{ext}"))
    };
    if !candidate.exists() {
        return candidate;
    }
    for i in 2..10_000 {
        let candidate = if ext.is_empty() {
            dir.join(format!("{stem}-{i}"))
        } else {
            dir.join(format!("{stem}-{i}.{ext}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if ext.is_empty() {
        dir.join(format!("{stem}-{stamp}"))
    } else {
        dir.join(format!("{stem}-{stamp}.{ext}"))
    }
}

/// 将文件权限收紧为 0600（仅 unix 生效；非 unix 为 no-op）。
///
/// 用于临时文件：内容可能来自用户 PDF，不应被同机其他用户读取。
pub fn set_private_permissions(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// RAII 临时文件守卫：drop 时尽力删除文件。
pub struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    pub fn new(path: PathBuf) -> Self {
        // 守卫创建时文件通常已存在，尽力收紧权限；
        // 文件尚未创建（稍后写入）时忽略错误，由写入点自行处理。
        let _ = set_private_permissions(&path);
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Owns a private temporary directory, including partial results on failure.
pub struct TempDirGuard {
    path: PathBuf,
}
impl TempDirGuard {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let guard = Self { path };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&guard.path, std::fs::Permissions::from_mode(0o700))?;
        }
        Ok(guard)
    }
}
impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Copy without ever replacing a pre-existing destination, including races.
pub fn copy_unique(source: &Path, directory: &Path) -> std::io::Result<PathBuf> {
    use std::io::Write;
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("template");
    let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("");
    loop {
        let path = unique_output_path(directory, stem, ext);
        let mut output = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let result = (|| {
            let mut input = std::fs::File::open(source)?;
            std::io::copy(&mut input, &mut output)?;
            output.flush()?;
            output.sync_all()
        })();
        drop(output);
        if let Err(error) = result {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        return Ok(path);
    }
}

#[cfg(test)]
mod freeze_tests {
    use super::*;
    #[test]
    fn directory_guard_allows_children_and_cleans_on_failure() {
        let path = temp_named_path("docsy-guard-test", "dir");
        std::fs::create_dir(&path).unwrap();
        {
            let _guard = TempDirGuard::new(path.clone()).unwrap();
            std::fs::write(path.join("partial.pdf"), b"partial").unwrap();
        }
        assert!(!path.exists());
    }
    #[test]
    fn export_copy_keeps_existing_file() {
        let path = temp_named_path("docsy-copy-test", "dir");
        std::fs::create_dir(&path).unwrap();
        let _guard = TempDirGuard::new(path.clone()).unwrap();
        let source = path.join("saved.docsytpl");
        std::fs::write(&source, b"old").unwrap();
        let copied = copy_unique(&source, &path).unwrap();
        assert_ne!(copied, source);
        assert_eq!(std::fs::read(&source).unwrap(), b"old");
        assert_eq!(std::fs::read(copied).unwrap(), b"old");
    }
}
