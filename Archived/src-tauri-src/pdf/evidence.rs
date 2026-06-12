use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::qpdf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceScan {
    pub root: String,
    pub groups: Vec<EvidenceGroup>,
    pub root_pdfs: Vec<EvidenceFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceGroup {
    pub id: String,
    pub name: String,
    pub path: String,
    pub files: Vec<EvidenceFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub extension: String,
    pub kind: EvidenceFileKind,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFileKind {
    Pdf,
    Word,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGroupPdfsArgs {
    pub root: String,
    pub selected_paths: Vec<String>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGroupPdfsResult {
    pub output_dir: String,
    pub outputs: Vec<EvidencePdfItem>,
    pub skipped: Vec<EvidenceStepMessage>,
    pub failed: Vec<EvidenceStepMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidencePdfItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: String,
    pub header: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceStepMessage {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeEvidencePdfsArgs {
    pub items: Vec<EvidencePdfItem>,
    pub output_path: String,
}

pub fn scan(root: &Path) -> Result<EvidenceScan, String> {
    if !root.is_dir() {
        return Err(format!("不是有效文件夹：{}", root.display()));
    }
    let mut groups = Vec::new();
    let mut root_pdfs = Vec::new();
    let mut warnings = Vec::new();

    let entries = fs::read_dir(root).map_err(|err| format!("读取母文件夹失败：{err}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if is_hidden(&path) {
            continue;
        }
        if path.is_dir() {
            let mut files = scan_supported_files(&path, 3, &mut warnings);
            files.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
            if !files.is_empty() {
                let name = file_name(&path);
                groups.push(EvidenceGroup {
                    id: stable_id(&path),
                    name,
                    path: path.display().to_string(),
                    files,
                });
            }
        } else if is_pdf(&path) {
            root_pdfs.push(file_item(&path));
        }
    }

    groups.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
    root_pdfs.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));

    // 检查是否有 Word 文件但没有 LibreOffice
    let has_word_files = groups.iter().any(|g| {
        g.files
            .iter()
            .any(|f| matches!(f.kind, EvidenceFileKind::Word))
    });
    if has_word_files && resolve_soffice().is_none() {
        warnings.push("检测到 DOC/DOCX 文件但未安装 LibreOffice，Word 文件无法转换为 PDF。请安装 LibreOffice 后重试。".to_string());
    }

    Ok(EvidenceScan {
        root: root.display().to_string(),
        groups,
        root_pdfs,
        warnings,
    })
}

pub fn build_group_pdfs(args: BuildGroupPdfsArgs) -> Result<BuildGroupPdfsResult, String> {
    let root = PathBuf::from(&args.root);
    if !root.is_dir() {
        return Err(format!("不是有效母文件夹：{}", root.display()));
    }
    let output_dir = args
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("_docsy_evidence_step1"));
    fs::create_dir_all(&output_dir).map_err(|err| format!("创建输出目录失败：{err}"))?;

    let selected = args
        .selected_paths
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let mut groups: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
    for path in selected {
        let Some(parent) = path.parent().map(Path::to_path_buf) else {
            continue;
        };
        if parent == root {
            continue;
        }
        if let Some((_, items)) = groups.iter_mut().find(|(dir, _)| *dir == parent) {
            items.push(path);
        } else {
            groups.push((parent, vec![path]));
        }
    }
    groups.sort_by(|(a, _), (b, _)| natural_key(&file_name(a)).cmp(&natural_key(&file_name(b))));

    let mut outputs = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    for (dir, mut files) in groups {
        files.sort_by(|a, b| natural_key(&file_name(a)).cmp(&natural_key(&file_name(b))));
        let mut pdfs = Vec::new();
        for file in files {
            if is_pdf(&file) {
                pdfs.push(file);
            } else if is_word(&file) {
                match convert_word_to_pdf(&file, &output_dir) {
                    Ok(pdf) => pdfs.push(pdf),
                    Err(message) => failed.push(EvidenceStepMessage {
                        path: file.display().to_string(),
                        message,
                    }),
                }
            } else {
                skipped.push(EvidenceStepMessage {
                    path: file.display().to_string(),
                    message: "不支持的文件类型".to_string(),
                });
            }
        }
        if pdfs.is_empty() {
            continue;
        }
        let group_name = file_name(&dir);
        let output = unique_pdf_path(&output_dir, &sanitize_file_stem(&group_name));
        let result = if pdfs.len() == 1 {
            fs::copy(&pdfs[0], &output).map_err(|err| format!("复制 PDF 失败：{err}"))?;
            qpdf::MergeResult {
                output_path: output.display().to_string(),
                input_count: 1,
            }
        } else {
            qpdf::merge(&pdfs, &output).map_err(|err| err.to_string())?
        };
        outputs.push(EvidencePdfItem {
            id: stable_id(Path::new(&result.output_path)),
            name: file_name(Path::new(&result.output_path)),
            path: result.output_path,
            source: "group".to_string(),
            header: group_name,
        });
    }

    Ok(BuildGroupPdfsResult {
        output_dir: output_dir.display().to_string(),
        outputs,
        skipped,
        failed,
    })
}

pub fn merge_evidence_pdfs(args: MergeEvidencePdfsArgs) -> Result<qpdf::MergeResult, String> {
    let inputs = args
        .items
        .iter()
        .map(|item| PathBuf::from(&item.path))
        .collect::<Vec<_>>();
    qpdf::merge(&inputs, Path::new(&args.output_path)).map_err(|err| err.to_string())
}

fn scan_supported_files(dir: &Path, depth: usize, warnings: &mut Vec<String>) -> Vec<EvidenceFile> {
    if depth == 0 {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(dir) else {
        warnings.push(format!("无法读取：{}", dir.display()));
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if is_hidden(&path) {
            continue;
        }
        if path.is_dir() {
            out.extend(scan_supported_files(&path, depth - 1, warnings));
        } else if is_pdf(&path) || is_word(&path) {
            out.push(file_item(&path));
        }
    }
    out
}

fn file_item(path: &Path) -> EvidenceFile {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    EvidenceFile {
        id: stable_id(path),
        name: file_name(path),
        path: path.display().to_string(),
        kind: if extension == "pdf" {
            EvidenceFileKind::Pdf
        } else {
            EvidenceFileKind::Word
        },
        extension,
        selected: true,
    }
}

fn convert_word_to_pdf(input: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    let soffice = resolve_soffice().ok_or("未检测到 LibreOffice/soffice，无法转换 Word 文件")?;
    let out = Command::new(soffice)
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(output_dir)
        .arg(input)
        .output()
        .map_err(|err| format!("调用 LibreOffice 失败：{err}"))?;
    if !out.status.success() {
        return Err(format!(
            "LibreOffice 转换失败：{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let expected = output_dir.join(format!("{}.pdf", file_stem(input)));
    if expected.exists() {
        Ok(expected)
    } else {
        Err("LibreOffice 未生成 PDF 输出".to_string())
    }
}

fn resolve_soffice() -> Option<PathBuf> {
    // 优先使用用户自定义路径
    let settings = crate::history::read_settings();
    if let Some(ref custom) = settings.libreoffice_path {
        let path = PathBuf::from(custom);
        if path.exists() {
            return Some(path);
        }
    }

    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            "/opt/homebrew/bin/soffice",
            "/usr/local/bin/soffice",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ]
    } else {
        vec!["/usr/bin/soffice", "/usr/local/bin/soffice"]
    };
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("pdf"))
}

fn is_word(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("doc") || s.eq_ignore_ascii_case("docx"))
}

fn is_hidden(path: &Path) -> bool {
    file_name(path).starts_with('.')
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

fn stable_id(path: &Path) -> String {
    format!("ev_{}", fnv1a(path.display().to_string().as_bytes()))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

/// 自然数排序键：将字符串拆成 [文本, 数值, 文本, 数值, ...] 片段，
/// 数值部分按数值比较，使得 "证据2" 排在 "证据10" 前面。
fn natural_key(input: &str) -> Vec<NaturalPart> {
    let lower = input.to_lowercase();
    let mut parts = Vec::new();
    let mut chars = lower.char_indices().peekable();
    let mut seg_start = 0;
    let mut in_digit = false;

    while let Some((i, ch)) = chars.next() {
        let is_digit = ch.is_ascii_digit();
        if i == 0 {
            in_digit = is_digit;
        } else if is_digit != in_digit {
            // 段切换
            let seg = &lower[seg_start..i];
            if in_digit {
                parts.push(NaturalPart::Num(seg.parse::<u64>().unwrap_or(0)));
            } else {
                parts.push(NaturalPart::Text(seg.to_string()));
            }
            seg_start = i;
            in_digit = is_digit;
        }
    }
    // 最后一段
    if seg_start < lower.len() {
        let seg = &lower[seg_start..];
        if in_digit {
            parts.push(NaturalPart::Num(seg.parse::<u64>().unwrap_or(0)));
        } else {
            parts.push(NaturalPart::Text(seg.to_string()));
        }
    }

    parts
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NaturalPart {
    Text(String),
    Num(u64),
}

impl Ord for NaturalPart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (NaturalPart::Text(a), NaturalPart::Text(b)) => a.cmp(b),
            (NaturalPart::Num(a), NaturalPart::Num(b)) => a.cmp(b),
            (NaturalPart::Text(_), NaturalPart::Num(_)) => std::cmp::Ordering::Less,
            (NaturalPart::Num(_), NaturalPart::Text(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for NaturalPart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn sanitize_file_stem(input: &str) -> String {
    let cleaned = input
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    if cleaned.trim().is_empty() {
        "output".to_string()
    } else {
        cleaned
    }
}

fn unique_pdf_path(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.pdf"));
    if !first.exists() {
        return first;
    }
    for i in 1..=9999 {
        let path = dir.join(format!("{stem}_{i}.pdf"));
        if !path.exists() {
            return path;
        }
    }
    dir.join(format!("{stem}_overflow.pdf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_key_sorts_numbers_numerically() {
        let mut keys = vec![
            natural_key("证据10"),
            natural_key("证据2"),
            natural_key("证据1"),
            natural_key("证据20"),
        ];
        keys.sort();
        // 应该按数值排序：1, 2, 10, 20
        let sorted_names = vec!["证据1", "证据2", "证据10", "证据20"];
        let input_names = vec!["证据10", "证据2", "证据1", "证据20"];
        let mut indexed: Vec<(String, usize)> = input_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.to_string(), i))
            .collect();
        indexed.sort_by_key(|(name, _)| natural_key(name));
        let result: Vec<&str> = indexed.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(result, sorted_names);
    }

    #[test]
    fn natural_key_mixed_text_and_numbers() {
        // "附件2-1" < "附件2-2" < "附件2-10"（数字部分按数值比较）
        // "附件10" 的第二段是 Num(10)，"附件2-1" 第二段是 Num(2)
        // Num(2) < Num(10)，所以 "附件2-1" 排在 "附件10" 前面
        let mut names = vec!["附件2-10", "附件2-2", "附件2-1", "附件10"];
        names.sort_by_key(|n| natural_key(n));
        assert_eq!(names, vec!["附件2-1", "附件2-2", "附件2-10", "附件10"]);
    }

    #[test]
    fn natural_key_pure_text() {
        // 纯文本按 Unicode 码位排序
        let mut names = vec!["b文件", "a文件", "c文件"];
        names.sort_by_key(|n| natural_key(n));
        assert_eq!(names, vec!["a文件", "b文件", "c文件"]);
    }

    #[test]
    fn sanitize_file_stem_replaces_special_chars() {
        assert_eq!(sanitize_file_stem("a/b:c*d"), "a_b_c_d");
        assert_eq!(sanitize_file_stem("normal"), "normal");
        assert_eq!(sanitize_file_stem(""), "output");
        assert_eq!(sanitize_file_stem("  "), "output");
    }

    #[test]
    fn unique_pdf_path_no_conflict() {
        let dir = std::env::temp_dir().join("docsy_test_unique");
        let _ = fs::create_dir_all(&dir);
        let path = unique_pdf_path(&dir, "test_no_conflict");
        assert_eq!(path.file_name().unwrap(), "test_no_conflict.pdf");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_pdf_path_with_conflict() {
        let dir = std::env::temp_dir().join("docsy_test_unique_conflict");
        let _ = fs::create_dir_all(&dir);
        // 创建一个已存在的文件
        let existing = dir.join("test_conflict.pdf");
        let _ = fs::write(&existing, "");
        let path = unique_pdf_path(&dir, "test_conflict");
        assert_eq!(path.file_name().unwrap(), "test_conflict_1.pdf");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_pdf_and_is_word() {
        assert!(is_pdf(Path::new("file.pdf")));
        assert!(is_pdf(Path::new("file.PDF")));
        assert!(!is_pdf(Path::new("file.doc")));
        assert!(is_word(Path::new("file.doc")));
        assert!(is_word(Path::new("file.DOCX")));
        assert!(!is_word(Path::new("file.pdf")));
    }

    #[test]
    fn stable_id_deterministic() {
        let a = stable_id(Path::new("/some/path/file.pdf"));
        let b = stable_id(Path::new("/some/path/file.pdf"));
        assert_eq!(a, b);
        assert!(a.starts_with("ev_"));
    }
}
