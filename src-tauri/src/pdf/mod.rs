pub mod annotations;
pub mod anti_ocr;
pub mod artifacts;
pub(crate) mod cmap;
pub mod compress;
pub mod content_text;
pub mod detection;
pub mod evidence;
pub mod evidence_session;
mod glyph_names;
pub mod header_footer;
pub mod normalize;
pub mod overlay;
pub mod page_info;
pub mod preview;
pub mod qpdf;
pub(crate) mod qpdf_stream;
pub mod split;
pub(crate) mod text_utils;

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
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normalize_path_components(&absolute)
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

/// FNV-1a 64-bit hash，用于生成确定性 ID。
pub fn fnv1a_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
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
