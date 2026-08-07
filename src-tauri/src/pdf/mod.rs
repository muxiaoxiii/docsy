pub mod annotations;
pub mod anti_ocr;
pub mod artifacts;
pub mod content_text;
pub mod detection;
pub mod evidence;
pub mod evidence_session;
pub mod header_footer;
pub mod normalize;
pub mod overlay;
pub mod page_info;
pub mod preview;
pub mod qpdf;
pub mod split;

use std::path::{Path, PathBuf};

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

/// 生成临时文件路径：`{prefix}_{pid}_{timestamp}.{extension}`
pub fn temp_named_path(prefix: &str, extension: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    if extension.is_empty() {
        std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}"))
    } else {
        std::env::temp_dir().join(format!("{prefix}_{pid}_{ts}.{extension}"))
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
