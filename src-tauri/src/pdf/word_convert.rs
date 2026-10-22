use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::external::ExternalTool;
use crate::ConversionState;

/// Check if a path or filename corresponds to a supported Word document (.doc, .docx, .docm).
pub fn is_word_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "doc" | "docx" | "docm")
}

/// Run a child process with interactive timeout.
/// When the initial timeout expires, calls `on_timeout` to ask the user whether to continue.
/// If the user chooses to continue, waits another `timeout` period.
/// Returns Ok(output) on success and Err when the user cancels.
fn run_process_with_interactive_timeout(
    cmd: &mut std::process::Command,
    initial_timeout: std::time::Duration,
    on_timeout: &dyn Fn() -> bool,
) -> Result<std::process::Output> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("启动转换进程失败")?;
    let mut stdout_reader = child
        .stdout
        .take()
        .map(crate::external::spawn_bounded_output_reader);
    let mut stderr_reader = child
        .stderr
        .take()
        .map(crate::external::spawn_bounded_output_reader);

    let timeout = initial_timeout;
    let mut wait_started = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_secs(1);

    loop {
        // Check if process finished
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = crate::external::finish_bounded_output_reader(stdout_reader.take());
                let stderr = crate::external::finish_bounded_output_reader(stderr_reader.take());
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                // Still running
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                crate::external::finish_bounded_output_reader(stdout_reader.take());
                crate::external::finish_bounded_output_reader(stderr_reader.take());
                anyhow::bail!("检查转换进程状态失败: {e}");
            }
        }

        // Check timeout
        if wait_started.elapsed() >= timeout {
            let should_continue = on_timeout();
            if !should_continue {
                let _ = child.kill();
                let _ = child.wait();
                crate::external::finish_bounded_output_reader(stdout_reader.take());
                crate::external::finish_bounded_output_reader(stderr_reader.take());
                anyhow::bail!("用户取消了转换");
            }
            wait_started = std::time::Instant::now();
        }

        std::thread::sleep(poll_interval);
    }
}

/// Extract the file stem safely from a path and sanitize it.
fn path_safe_stem(path: &str) -> String {
    let raw = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    crate::util::fs::safe_file_stem(raw)
}

/// Convert a Word document (.doc, .docx, .docm) to PDF in `output_dir` using atomic non-overwriting publishing.
/// Eliminates the concurrency window by exclusively creating the destination file at publish time.
pub fn convert_word_to_pdf(
    doc_path: &str,
    output_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    if !Path::new(doc_path).is_file() { anyhow::bail!("原文件不存在或不是文件：{doc_path}"); }
    fs::create_dir_all(output_dir)
        .with_context(|| format!("无法创建输出目录: {}", output_dir.display()))?;

    let stem = path_safe_stem(doc_path);

    if !is_word_file(doc_path) {
        let published = crate::util::fs::copy_unique_with_stem(
            Path::new(doc_path),
            output_dir,
            &stem,
            "pdf",
        )
        .with_context(|| format!("复制 PDF 到输出目录失败: {}", output_dir.display()))?;
        return Ok(published.display().to_string());
    }

    // Isolated conversion directory
    let temp_conv_dir = crate::util::fs::temp_named_path("docsy_word_conv", "");
    fs::create_dir_all(&temp_conv_dir).context("创建临时转换目录失败")?;
    let _temp_dir_guard = crate::util::fs::TempDirGuard::new(temp_conv_dir.clone())
        .context("创建临时目录守卫失败")?;

    let converted_path = convert_doc_or_docx_to_pdf_in_dir(doc_path, &temp_conv_dir, conversion_state)?;
    let converted = Path::new(&converted_path);

    // Atomically publish into output_dir without overwriting any concurrent results
    let published = crate::util::fs::copy_unique_with_stem(
        converted,
        output_dir,
        &stem,
        "pdf",
    )
    .with_context(|| format!("发布转换后的 PDF 失败: {}", output_dir.display()))?;

    Ok(published.display().to_string())
}

/// Convert a Word document to a specific target PDF file safely without deleting existing target prematurely,
/// and with RAII guard on the sibling temporary file so no orphaned files are left in the destination directory.
pub fn convert_word_to_pdf_file(
    doc_path: &str,
    output_pdf_path: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    if !Path::new(doc_path).is_file() { anyhow::bail!("原文件不存在或不是文件：{doc_path}"); }
    let parent = output_pdf_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("无法创建输出目录: {}", parent.display()))?;

    if !is_word_file(doc_path) {
        if crate::util::fs::same_path(Path::new(doc_path), output_pdf_path) {
            return Ok(doc_path.to_string());
        }
        let sibling = crate::util::fs::sibling_temp_path(output_pdf_path, "docsy-copy-pdf");
        let _sibling_guard = crate::util::fs::TempPathGuard::new(sibling.clone());
        fs::copy(doc_path, &sibling)
            .with_context(|| format!("复制 PDF 文件失败: {doc_path} -> {}", sibling.display()))?;
        crate::util::fs::replace_file(&sibling, output_pdf_path)
            .with_context(|| format!("替换目标 PDF 文件失败: {}", output_pdf_path.display()))?;
        return Ok(output_pdf_path.display().to_string());
    }

    // Create an isolated temporary directory for the conversion
    let temp_conv_dir = crate::util::fs::temp_named_path("docsy_word_conv", "");
    fs::create_dir_all(&temp_conv_dir).context("创建临时转换目录失败")?;
    let _temp_dir_guard = crate::util::fs::TempDirGuard::new(temp_conv_dir.clone())
        .context("创建临时目录守卫失败")?;

    let converted_path = convert_doc_or_docx_to_pdf_in_dir(doc_path, &temp_conv_dir, conversion_state)?;
    let converted = Path::new(&converted_path);

    if !crate::util::fs::same_path(converted, output_pdf_path) {
        let sibling = crate::util::fs::sibling_temp_path(output_pdf_path, "docsy-word-pdf");
        let _sibling_guard = crate::util::fs::TempPathGuard::new(sibling.clone());
        fs::copy(converted, &sibling)
            .with_context(|| format!("复制转换后的 PDF 失败: {} -> {}", converted.display(), sibling.display()))?;
        crate::util::fs::replace_file(&sibling, output_pdf_path)
            .with_context(|| format!("替换目标 PDF 失败: {}", output_pdf_path.display()))?;
    }

    Ok(output_pdf_path.display().to_string())
}

fn convert_doc_or_docx_to_pdf_in_dir(
    doc_path: &str,
    conversion_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    let mut attempts = Vec::new();

    // 1. Try available engines directly
    if let Ok(path) = try_available_engines(doc_path, conversion_dir, conversion_state, &mut attempts) {
        return Ok(path);
    }

    // 2. If it's a legacy .doc, try doc2x to convert to .docx, then retry available engines
    let ext = Path::new(doc_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "doc" {
        match try_doc2x_to_docx(doc_path, conversion_dir) {
            Ok(temp_docx) => {
                let temp_docx_path = temp_docx.path().to_string_lossy().to_string();
                match try_available_engines(&temp_docx_path, conversion_dir, conversion_state, &mut attempts) {
                    Ok(path) => return Ok(path),
                    Err(err) => {
                        attempts.push(format!("doc2x 转为 docx 后重试转换失败: {err}"));
                    }
                }
            }
            Err(err) => {
                attempts.push(format!("doc2x 格式转换失败: {err}"));
            }
        }
    }

    anyhow::bail!(
        "Word 自动转 PDF 失败，原文件未修改: {}。请使用 Word 或 WPS 另存为 PDF，再添加转换后的 PDF 继续。尝试记录：{}",
        doc_path,
        attempts.join("；")
    )
}

fn try_available_engines(
    doc_path: &str,
    conversion_dir: &Path,
    conversion_state: &Arc<ConversionState>,
    attempts: &mut Vec<String>,
) -> Result<String> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        match convert_doc_to_pdf_with_word(doc_path, conversion_dir, conversion_state) {
            Ok(path) => return Ok(path),
            Err(err) => attempts.push(format!("Microsoft Word 转换失败: {err}")),
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        attempts.push("当前平台不支持 Microsoft Word 自动转换".to_string());
    }

    #[cfg(windows)]
    {
        match convert_doc_to_pdf_with_wps(doc_path, conversion_dir, conversion_state) {
            Ok(path) => return Ok(path),
            Err(err) => attempts.push(format!("WPS Writer 转换失败: {err}")),
        }
    }

    match crate::external::LibreOfficeTool.binary_path() {
        Ok(lo_bin) => match convert_doc_to_pdf_with_libreoffice(
            &lo_bin,
            doc_path,
            conversion_dir,
            conversion_state,
        ) {
            Ok(path) => Ok(path),
            Err(err) => {
                attempts.push(format!("LibreOffice 转换失败: {err}"));
                anyhow::bail!("LibreOffice 转换失败: {err}")
            }
        },
        Err(err) => {
            attempts.push(format!("未检测到 LibreOffice: {err}"));
            anyhow::bail!("未检测到 LibreOffice: {err}")
        }
    }
}

fn try_doc2x_to_docx(doc_path: &str, conversion_dir: &Path) -> Result<crate::util::fs::TempPathGuard> {
    let bytes = fs::read(doc_path).context("读取 .doc 文件失败")?;
    let docx_bytes = crate::doc2docx::convert_doc_to_docx(&bytes)
        .context("doc2x 转为临时 docx 失败")?;
    let stem = path_safe_stem(doc_path);
    let temp_docx = conversion_dir.join(format!("{stem}_fallback.docx"));
    let guard = crate::util::fs::TempPathGuard::new(temp_docx.clone());
    fs::write(&temp_docx, docx_bytes).context("写入临时 docx 失败")?;
    Ok(guard)
}

#[cfg(windows)]
fn convert_doc_to_pdf_with_word(
    doc_path: &str,
    output_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    let input = std::fs::canonicalize(doc_path)
        .with_context(|| format!("读取 DOC/DOCX 文件失败: {doc_path}"))?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let output = output_dir.join(format!("{stem}.pdf"));
    let script = format!(
        "$ErrorActionPreference='Stop';\
         $word=$null;$doc=$null;\
         try {{\
           $word=New-Object -ComObject Word.Application;\
           $word.Visible=$false;\
           $doc=$word.Documents.Open('{input}', $false, $true);\
           $doc.ExportAsFixedFormat('{output}', 17);\
         }} finally {{\
           if ($doc -ne $null) {{ $doc.Close([ref]$false) | Out-Null }};\
           if ($word -ne $null) {{ $word.Quit() | Out-Null }};\
         }}",
        input = powershell_escape(&input.display().to_string()),
        output = powershell_escape(&output.display().to_string()),
    );

    let mut cmd = crate::external::hidden_command("powershell");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &script,
    ]);

    let app = crate::get_app_handle().ok_or_else(|| anyhow::anyhow!("应用未初始化"))?;
    let output_result =
        run_process_with_interactive_timeout(&mut cmd, std::time::Duration::from_secs(60), &|| {
            conversion_state.wait_for_user_response(app)
        })
        .context("Microsoft Word 转换失败")?;

    if !output_result.status.success() || !output.exists() {
        anyhow::bail!(
            "Microsoft Word 转 PDF 失败: {doc_path}（{}）",
            crate::external::command_failure_detail(&output_result)
        );
    }
    Ok(output.display().to_string())
}

#[cfg(windows)]
fn convert_doc_to_pdf_with_wps(
    doc_path: &str,
    output_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    let input = std::fs::canonicalize(doc_path)
        .with_context(|| format!("读取 Word 文件失败: {doc_path}"))?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let output = output_dir.join(format!("{stem}.pdf"));
    let script = format!(
        "$ErrorActionPreference='Stop';\
         $wps=$null;$doc=$null;\
         try {{\
           $wps=New-Object -ComObject KWPS.Application;\
           $wps.Visible=$false;\
           try {{ $wps.DisplayAlerts=$false }} catch {{ }};\
           $doc=$wps.Documents.Open('{input}');\
           try {{\
             $doc.ExportAsFixedFormat('{output}', 17);\
           }} catch {{\
             $doc.SaveAs('{output}', 17);\
           }};\
         }} finally {{\
           if ($doc -ne $null) {{ try {{ $doc.Close([ref]$false) | Out-Null }} catch {{ }} }};\
           if ($wps -ne $null) {{ try {{ $wps.Quit() | Out-Null }} catch {{ }} }};\
           [System.GC]::Collect();\
           [System.GC]::WaitForPendingFinalizers();\
         }}",
        input = powershell_escape(&input.display().to_string()),
        output = powershell_escape(&output.display().to_string()),
    );

    let mut cmd = crate::external::hidden_command("powershell");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &script,
    ]);

    let app = crate::get_app_handle().ok_or_else(|| anyhow::anyhow!("应用未初始化"))?;
    let output_result =
        run_process_with_interactive_timeout(&mut cmd, std::time::Duration::from_secs(60), &|| {
            conversion_state.wait_for_user_response(app)
        })
        .context("WPS Writer 转换失败")?;

    if !output_result.status.success() || !output.exists() {
        anyhow::bail!(
            "WPS Writer 转 PDF 失败: {doc_path}（{}）",
            crate::external::command_failure_detail(&output_result)
        );
    }
    Ok(output.display().to_string())
}

#[cfg(target_os = "macos")]
fn convert_doc_to_pdf_with_word(
    doc_path: &str,
    output_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    let input = std::fs::canonicalize(doc_path)
        .with_context(|| format!("读取 Word 文件失败: {doc_path}"))?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    // A stable exchange folder lets Word reuse its user-granted folder access.
    // Input and output share this folder; Word never needs the final destination.
    let source_dir = dirs::cache_dir().context("无法定位转换缓存目录")?.join("Docsy").join("WordConversion");
    fs::create_dir_all(&source_dir)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&source_dir, fs::Permissions::from_mode(0o700))?;
    let lock = fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(source_dir.join(".conversion.lock"))?;
    lock.try_lock().context("另一个 Word 转换正在进行，请完成后重试")?;
    // Never operate on an already-open original; only close our isolated copy.
    let staged = crate::util::fs::copy_unique(&input, &source_dir).context("创建 Word 转换副本失败")?;
    let _source_guard = crate::util::fs::TempPathGuard::new(staged.clone());
    let output = crate::util::fs::unique_output_path(&source_dir, stem, "pdf");
    let _output_guard = crate::util::fs::TempPathGuard::new(output.clone());
    // Use a lazy document reference as in docx2pdf's macOS adapter. Fetching
    // `active document` as an AppleScript value fails in some Word releases.
    // Keep a stable named reference and verify its full path before touching it.
    let script = r#"
ObjC.import('Foundation');
function canonical(value) {
  var path = String(value);
  if (path.charAt(0) !== '/') path = '/' + path.split(':').slice(1).join('/');
  return ObjC.unwrap($(path).stringByStandardizingPath.stringByResolvingSymlinksInPath);
}
function run(argv) {
  var input = argv[0], output = argv[1];
  var word = Application('Microsoft Word');
  if (!word.running()) word.launch();
  var previousVisibility = word.visible();
  var owned = null, step = '打开文档副本', lastError = 'Word 尚未返回文档';
  try {
    word.open(Path(input), {readOnly: true, addToRecentFiles: false});
    step = '取得转换副本';
    for (var attempt = 0; attempt < 100; attempt++) {
      try {
        var candidate = word.activeDocument;
        if (canonical(candidate.fullName()) === canonical(input)) {
          owned = word.documents.byName(candidate.name());
          if (canonical(owned.fullName()) === canonical(input)) break;
          owned = null;
        }
      } catch (error) { lastError = String(error); }
      delay(0.1);
    }
    if (!owned) throw new Error('未取得转换副本，未操作其他已打开文档：' + lastError);
    step = '导出 PDF';
    owned.saveAs({fileName: output, fileFormat: 'format PDF', addToRecentFiles: false});
  } catch (error) {
    throw new Error(step + '：' + String(error));
  } finally {
    if (owned) { try { owned.close({saving: 'no'}); } catch (_) {} }
    try { word.visible = previousVisibility; } catch (_) {}
  }
}
"#;

    let mut command = crate::external::hidden_command("osascript");
    command
        .args(["-l", "JavaScript", "-e"])
        .arg(script)
        .arg(staged.display().to_string())
        .arg(output.display().to_string());
    let app = crate::get_app_handle().ok_or_else(|| anyhow::anyhow!("应用未初始化"))?;
    let result = run_process_with_interactive_timeout(
        &mut command,
        std::time::Duration::from_secs(60),
        &|| conversion_state.wait_for_user_response(app),
    )
    .context("启动 Microsoft Word 转换失败")?;

    if !result.status.success() || !output.exists() {
        anyhow::bail!(
            "Microsoft Word 转 PDF 失败: {doc_path}（{}）",
            crate::external::command_failure_detail(&result)
        );
    }
    let published = crate::util::fs::copy_unique_with_stem(&output, output_dir, stem, "pdf")?;
    Ok(published.display().to_string())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn convert_doc_to_pdf_with_word(
    _doc_path: &str,
    _output_dir: &Path,
    _conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    anyhow::bail!("当前平台不支持 Microsoft Word 自动转换")
}

fn convert_doc_to_pdf_with_libreoffice(
    lo_bin: &Path,
    doc_path: &str,
    output_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    let mut command = crate::external::hidden_command(lo_bin);
    command
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(output_dir)
        .arg(doc_path);
    let app = crate::get_app_handle().ok_or_else(|| anyhow::anyhow!("应用未初始化"))?;
    let result = run_process_with_interactive_timeout(
        &mut command,
        std::time::Duration::from_secs(60),
        &|| conversion_state.wait_for_user_response(app),
    )?;

    if !result.status.success() {
        anyhow::bail!(
            "Word 文件转换失败: {}（{}）",
            doc_path,
            crate::external::command_failure_detail(&result)
        );
    }

    let stem = Path::new(doc_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let pdf_path = output_dir.join(format!("{}.pdf", stem));

    if !pdf_path.exists() {
        anyhow::bail!("Word 文件转换输出未找到: {}", doc_path);
    }

    Ok(pdf_path.display().to_string())
}

#[cfg(windows)]
fn powershell_escape(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_word_file() {
        assert!(is_word_file("document.docx"));
        assert!(is_word_file("DOCUMENT.DOC"));
        assert!(is_word_file("letter.docm"));
        assert!(is_word_file("/path/to/some/nested/file.DOCX"));
        assert!(!is_word_file("contract.pdf"));
        assert!(!is_word_file("notes.txt"));
        assert!(!is_word_file("image.png"));
        assert!(!is_word_file("no_extension"));
    }

    #[test]
    fn test_convert_pdf_passthrough() {
        let temp_dir = std::env::temp_dir().join(format!("docsy_wc_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let src_pdf = temp_dir.join("source.pdf");
        fs::write(&src_pdf, b"%PDF-1.4 test").unwrap();

        let dst_pdf = temp_dir.join("sub").join("dest.pdf");
        let state = Arc::new(ConversionState::new());
        let res = convert_word_to_pdf_file(&src_pdf.to_string_lossy(), &dst_pdf, &state);
        assert!(res.is_ok());
        assert!(dst_pdf.is_file());
        assert_eq!(fs::read(&dst_pdf).unwrap(), b"%PDF-1.4 test");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_passthrough_does_not_corrupt_existing_target_on_same_source() {
        let temp_dir = std::env::temp_dir().join(format!("docsy_wc_test2_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let src_pdf = temp_dir.join("exist.pdf");
        fs::write(&src_pdf, b"%PDF-1.4 original").unwrap();

        let state = Arc::new(ConversionState::new());
        let res = convert_word_to_pdf_file(&src_pdf.to_string_lossy(), &src_pdf, &state);
        assert!(res.is_ok());
        assert_eq!(fs::read(&src_pdf).unwrap(), b"%PDF-1.4 original");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sibling_guard_cleans_up_on_replace_failure() {
        let temp_dir = std::env::temp_dir().join(format!("docsy_wc_fail_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let src_pdf = temp_dir.join("source.pdf");
        fs::write(&src_pdf, b"%PDF-1.4 data").unwrap();

        // Target path is an existing directory, which will cause replace_file to fail
        let invalid_dst = temp_dir.join("target_dir");
        let _ = fs::create_dir(&invalid_dst);

        let state = Arc::new(ConversionState::new());
        let res = convert_word_to_pdf_file(&src_pdf.to_string_lossy(), &invalid_dst, &state);
        assert!(res.is_err());

        // Ensure no leftover temporary files in temp_dir other than source.pdf and target_dir
        let entries: Vec<_> = fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            entries.iter().all(|name| name == "source.pdf" || name == "target_dir"),
            "Sibling temporary file was not cleaned up on failure: {:?}",
            entries
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_convert_word_to_pdf_publishes_unique_on_collision() {
        let temp_dir = std::env::temp_dir().join(format!("docsy_wc_uniq_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        // Pre-create contract.pdf in destination
        let existing = temp_dir.join("contract.pdf");
        fs::write(&existing, b"original content").unwrap();

        let src_pdf = temp_dir.join("contract.tmp");
        fs::write(&src_pdf, b"new content").unwrap();

        let state = Arc::new(ConversionState::new());
        let res = convert_word_to_pdf(&src_pdf.to_string_lossy(), &temp_dir, &state);
        assert!(res.is_ok());

        let published_path = res.unwrap();
        assert_ne!(published_path, existing.display().to_string());
        assert!(published_path.ends_with("contract-2.pdf"));

        // Original file must remain untouched
        assert_eq!(fs::read(&existing).unwrap(), b"original content");
        assert_eq!(fs::read(&published_path).unwrap(), b"new content");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
