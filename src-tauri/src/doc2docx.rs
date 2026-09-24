//! `.doc` → `.docx` sidecar conversion (b2xtranslator-derived `doc2x`).
//! Engineering shell only: type sniffing, temp workspace, timeout, product validation.
//! Does not reimplement MS-DOC parsing.

use anyhow::{anyhow, bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CFB_MAGIC: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
const ZIP_MAGIC: [u8; 2] = [0x50, 0x4b];
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Docx,
    Doc,
}

pub fn timeout() -> Duration {
    let ms = std::env::var("DOCSY_DOC2X_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    Duration::from_millis(ms)
}

/// Resolve sidecar path: env override, then bundled runtime, then debug crate runtime dir.
pub fn converter_path() -> Option<PathBuf> {
    let explicit = std::env::var_os("DOCSY_DOC2X_PATH")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    if let Some(path) = explicit {
        return path.is_file().then_some(path);
    }
    let relative = if cfg!(windows) { "bin/doc2x.exe" } else { "bin/doc2x" };
    if let Some(dir) = tauri_runtime_dir() {
        let candidate = dir.join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if cfg!(debug_assertions) {
        let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("runtime")
            .join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn tauri_runtime_dir() -> Option<PathBuf> {
    // Prefer the app resource bundle when running inside Tauri; fall back to env for tests.
    if let Ok(dir) = std::env::var("DOCSY_RUNTIME_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return Some(path);
        }
    }
    None
}

fn contains_utf16_le_ascii(haystack: &[u8], ascii: &str) -> bool {
    let needle: Vec<u8> = ascii
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|window| window == needle)
}

/// Classify buffer: ZIP passthrough, CFB Word binary, or reject.
pub fn classify_input(bytes: &[u8]) -> Result<InputKind> {
    if bytes.len() < 8 {
        bail!("文件内容为空或过短，无法识别格式");
    }
    if bytes[..2] == ZIP_MAGIC {
        return Ok(InputKind::Docx);
    }
    if bytes[..8] != CFB_MAGIC {
        bail!("该文件既不是 .doc（OLE 复合文档）也不是 .docx（OOXML），请确认文件未损坏");
    }
    if contains_utf16_le_ascii(bytes, "EncryptedPackage") {
        bail!("该文件受密码保护／已加密，无法解析。请先去除密码");
    }
    if !contains_utf16_le_ascii(bytes, "WordDocument") {
        bail!("该文件是旧版 Office 二进制文档，但不是 Word 文档（未找到 WordDocument 流）");
    }
    Ok(InputKind::Doc)
}

fn validate_docx(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 2 || bytes[..2] != ZIP_MAGIC {
        bail!("转换产物不是有效的 .docx（OOXML/ZIP）包");
    }
    if !bytes.windows(b"word/document.xml".len()).any(|w| w == b"word/document.xml") {
        bail!("转换产物缺少主文档部件 word/document.xml");
    }
    Ok(())
}

fn describe_output(stdout: &str, stderr: &str, killed: bool, limit_ms: u64) -> String {
    if killed {
        return format!("转换进程超时被终止（上限 {limit_ms}ms）");
    }
    let combined = format!("{stdout}\n{stderr}");
    let lines: Vec<&str> = combined
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let flagged: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            line.contains("[E]")
                || lower.contains("error")
                || lower.contains("fail")
                || lower.contains("exception")
        })
        .collect();
    let picked = if flagged.is_empty() { lines } else { flagged };
    let joined = picked
        .iter()
        .rev()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    let out: String = joined
        .chars()
        .rev()
        .take(400)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if out.is_empty() {
        "转换器未产出文件且未给出错误信息".into()
    } else {
        out
    }
}

fn utf16le(ascii: &str) -> Vec<u8> {
    ascii.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect()
}

fn run_doc2x(bin: &Path, input: &Path, output: &Path) -> Result<Vec<u8>> {
    let limit = timeout();
    let limit_ms = limit.as_millis() as u64;
    let mut child = Command::new(bin)
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("-v")
        .arg("error")
        .current_dir(input.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("无法启动 doc2x 转换器：{}", bin.display()))?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            use std::io::Read;
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            use std::io::Read;
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + limit;
    let mut killed = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    killed = true;
                    let _ = child.kill();
                    let _ = child.wait();
                    break std::process::ExitStatus::default();
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("等待 doc2x 失败：{e}");
            }
        }
    };

    let stdout_buf = stdout_reader.join().unwrap_or_default();
    let stderr_buf = stderr_reader.join().unwrap_or_default();
    let _ = status;

    if killed || stdout_buf.contains("[E]") || stderr_buf.contains("[E]") {
        bail!(
            "doc2x 转换失败：{}",
            describe_output(&stdout_buf, &stderr_buf, killed, limit_ms)
        );
    }

    let produced = std::fs::read(output)
        .ok()
        .filter(|bytes| !bytes.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "doc2x 未产出文件：{}",
                describe_output(&stdout_buf, &stderr_buf, false, limit_ms)
            )
        })?;
    validate_docx(&produced)?;
    Ok(produced)
}

/// Convert `.doc` bytes to `.docx` bytes. ZIP input is passed through unchanged.
pub fn convert_doc_to_docx(bytes: &[u8]) -> Result<Vec<u8>> {
    match classify_input(bytes)? {
        InputKind::Docx => Ok(bytes.to_vec()),
        InputKind::Doc => {
            let bin = converter_path().ok_or_else(|| {
                anyhow!("未找到 doc2x 转换器；请运行 node scripts/prepare-doc2x.mjs 或设置 DOCSY_DOC2X_PATH")
            })?;
            let temp = std::env::temp_dir().join(format!(
                "docsy-doc2x-{:016x}",
                crate::pdf::fnv1a_hash(&format!("{}:{}",
                    bytes.as_ptr() as usize,
                    std::process::id()))
            ));
            std::fs::create_dir_all(&temp).context("无法创建 doc2x 临时目录")?;
            let input = temp.join("input.doc");
            let output = temp.join("output.docx");
            let result = (|| {
                let mut file = std::fs::File::create(&input).context("无法写入临时 .doc")?;
                file.write_all(bytes)?;
                file.flush()?;
                run_doc2x(&bin, &input, &output)
            })();
            let _ = std::fs::remove_dir_all(&temp);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_zip_as_docx_passthrough() {
        let mut bytes = ZIP_MAGIC.to_vec();
        bytes.extend_from_slice(b"PK\x03\x04rest");
        assert_eq!(classify_input(&bytes).unwrap(), InputKind::Docx);
    }

    #[test]
    fn rejects_non_word_cfb_and_encrypted() {
        assert!(classify_input(b"short").is_err());
        assert!(classify_input(&[0u8; 16]).is_err());
        let mut cfb = CFB_MAGIC.to_vec();
        cfb.extend_from_slice(&[0u8; 32]);
        cfb.extend_from_slice(&utf16le("Workbook"));
        let err = classify_input(&cfb).unwrap_err().to_string();
        assert!(err.contains("不是 Word 文档"), "{err}");

        let mut encrypted = CFB_MAGIC.to_vec();
        encrypted.extend_from_slice(&[0u8; 8]);
        encrypted.extend_from_slice(&utf16le("EncryptedPackage"));
        let err = classify_input(&encrypted).unwrap_err().to_string();
        assert!(err.contains("加密"), "{err}");
    }

    #[test]
    fn accepts_cfb_with_worddocument_stream_anywhere() {
        let mut bytes = CFB_MAGIC.to_vec();
        bytes.extend_from_slice(&[0u8; 64]);
        bytes.extend_from_slice(&vec![0u8; 4096]);
        bytes.extend_from_slice(&utf16le("WordDocument"));
        assert_eq!(classify_input(&bytes).unwrap(), InputKind::Doc);
    }

    #[test]
    fn validates_docx_structure_bytes() {
        let mut ok = ZIP_MAGIC.to_vec();
        ok.extend_from_slice(b"PK..word/document.xml..");
        assert!(validate_docx(&ok).is_ok());
        assert!(validate_docx(b"not-zip").is_err());
        assert!(validate_docx(b"PK\x03\x04no-doc").is_err());
    }

    #[test]
    fn end_to_end_when_sidecar_present() {
        let Some(bin) = converter_path() else {
            eprintln!("skip: doc2x not prepared");
            return;
        };
        let sample = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/doc-import/sample.doc");
        if !sample.is_file() {
            eprintln!("skip: sample.doc missing");
            return;
        }
        let bytes = std::fs::read(&sample).unwrap();
        let docx = convert_doc_to_docx(&bytes).expect("convert sample.doc");
        validate_docx(&docx).unwrap();
        let _ = bin;
    }
}
