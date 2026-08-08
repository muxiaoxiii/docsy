use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::external::ExternalTool;
use crate::sort_utils::natural_cmp;

use super::fnv1a_hash;
use super::safe_file_stem;
use crate::ConversionState;

const SUPPORTED_EXTS: &[&str] = &["pdf", "doc", "docx", "docm"];

/// Run a child process with interactive timeout.
/// When the initial timeout expires, emits a Tauri event and waits for user response.
/// If the user chooses to continue, waits another `timeout` period, repeating up to `max_rounds`.
/// Returns Ok(output) on success, Err on cancel or max rounds exceeded.
fn run_process_with_interactive_timeout(
    cmd: &mut std::process::Command,
    initial_timeout: std::time::Duration,
    conversion_state: &Arc<ConversionState>,
    app: &tauri::AppHandle,
) -> Result<std::process::Output> {
    conversion_state.reset();

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("启动转换进程失败")?;

    conversion_state
        .pid
        .store(child.id() as u64, Ordering::SeqCst);

    let timeout = initial_timeout;
    let mut wait_started = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_secs(1);

    loop {
        // Check if process finished
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child.stdout.take().map(|mut s| {
                    let mut buf = Vec::new();
                    let _ = std::io::Read::read_to_end(&mut s, &mut buf);
                    buf
                });
                let stderr = child.stderr.take().map(|mut s| {
                    let mut buf = Vec::new();
                    let _ = std::io::Read::read_to_end(&mut s, &mut buf);
                    buf
                });
                return Ok(std::process::Output {
                    status,
                    stdout: stdout.unwrap_or_default(),
                    stderr: stderr.unwrap_or_default(),
                });
            }
            Ok(None) => {
                // Still running
            }
            Err(e) => {
                let _ = child.kill();
                anyhow::bail!("检查转换进程状态失败: {e}");
            }
        }

        // Check timeout
        if wait_started.elapsed() >= timeout {
            // Ask user whether to continue
            let should_continue = conversion_state.wait_for_user_response(app);
            if !should_continue {
                let _ = child.kill();
                anyhow::bail!("用户取消了转换");
            }
            wait_started = std::time::Instant::now();
        }

        std::thread::sleep(poll_interval);
    }
}

#[derive(Debug, Clone, Copy)]
enum FileType {
    Pdf,
    Word,
}

impl FileType {
    fn from_ext(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "doc" | "docx" | "docm" => Some(Self::Word),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Word => "word",
        }
    }
}

// TODO: GLM-P2 迁移到 header_footer 模块 — 使用 HeaderFooterJob + OverlayTextConfig 替代
#[derive(Debug, Deserialize)]
struct OverlayConfig {
    header: Option<HeaderConfig>,
    footer: Option<FooterConfig>,
}

#[derive(Debug, Deserialize)]
struct HeaderConfig {
    enabled: bool,
    #[serde(default)]
    content: String,
    custom_text: Option<String>,
    start_number: Option<u32>,
    font_size: Option<f64>,
    y_offset: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct FooterConfig {
    enabled: bool,
    #[serde(default)]
    content: String,
    font_size: Option<f64>,
    y_offset: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct IdentityConfig {
    prefix: Option<String>,
    start_number: Option<u32>,
}

fn has_supported_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn collect_supported_files(
    dir: &Path,
    out: &mut Vec<(String, PathBuf, FileType, u64)>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            // Evidence folders are often assembled with Finder/Explorer aliases.
            // Never follow them: a link to an ancestor would recurse forever and
            // a link outside the selected folder should not be imported silently.
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_supported_files(&path, out)?;
            continue;
        }
        if !has_supported_ext(&path) {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if let Some(ft) = FileType::from_ext(&ext) {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            out.push((name, path, ft, size));
        }
    }
    Ok(())
}

pub fn scan_folder(root: &str) -> Result<serde_json::Value> {
    let root_path = Path::new(root);
    if !root_path.is_dir() {
        anyhow::bail!("路径不是有效目录: {}", root);
    }

    let mut groups: BTreeMap<String, Vec<(String, PathBuf, FileType, u64)>> = BTreeMap::new();

    for entry in fs::read_dir(root_path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if dir_name.starts_with('_') || dir_name.starts_with('.') {
            continue;
        }

        let mut files = Vec::new();
        collect_supported_files(&path, &mut files)?;
        if files.is_empty() {
            continue;
        }
        files.sort_by(|a, b| natural_cmp(&a.0, &b.0));
        groups.insert(dir_name, files);
    }

    // Fallback: if no subdirectory groups found, treat root as a single group
    if groups.is_empty() {
        let mut root_files = Vec::new();
        collect_supported_files(root_path, &mut root_files)?;
        // Filter out files in subdirectories (only keep root-level files)
        root_files.retain(|(_, path, _, _)| {
            path.parent().map_or(false, |p| p == root_path)
        });
        root_files.sort_by(|a, b| natural_cmp(&a.0, &b.0));
        if !root_files.is_empty() {
            let group_name = root_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("证据")
                .to_string();
            groups.insert(group_name, root_files);
        }
    }

    let mut groups_json = Vec::new();
    for (group_name, files) in &groups {
        let group_id = format!("{:016x}", fnv1a_hash(group_name));
        let files_json: Vec<serde_json::Value> = files
            .iter()
            .map(|(name, path, ft, size)| {
                let id = format!("{:016x}", fnv1a_hash(&path.display().to_string()));
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "path": path.display().to_string(),
                    "fileType": ft.label(),
                    "size": size,
                })
            })
            .collect();
        groups_json.push(serde_json::json!({
            "id": group_id,
            "name": group_name,
            "files": files_json,
        }));
    }

    Ok(serde_json::json!({
        "groups": groups_json,
        "root": root,
    }))
}

pub fn build_group_pdfs(
    args: &serde_json::Value,
    conversion_state: &Arc<ConversionState>,
) -> Result<serde_json::Value> {
    let root = args["root"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少 root 参数"))?;
    let groups = args["groups"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("缺少 groups 参数"))?;

    let evidence_dir = Path::new(root).join("_evidence_output");
    fs::create_dir_all(&evidence_dir)?;

    let qpdf_bin = crate::external::QpdfTool.binary_path()?;

    let mut results = Vec::new();
    let mut failed_conversions = Vec::new();

    for group in groups {
        let group_name = group["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("分组缺少 name"))?;
        let group_id = group["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("分组缺少 id"))?;
        let files = group["files"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("分组缺少 files"))?;

        let mut pdf_paths: Vec<String> = Vec::new();

        for file in files {
            let file_path = file["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("文件缺少 path"))?;
            let file_type = file["fileType"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("文件缺少 fileType"))?;

            match file_type {
                "pdf" => {
                    pdf_paths.push(file_path.to_string());
                }
                "word" => match convert_word_to_pdf(file_path, &evidence_dir, conversion_state) {
                    Ok(converted) => pdf_paths.push(converted),
                    Err(err) => failed_conversions.push(serde_json::json!({
                        "groupId": group_id,
                        "groupName": group_name,
                        "path": file_path,
                        "name": Path::new(file_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(file_path),
                        "reason": err.to_string(),
                    })),
                },
                _ => {}
            }
        }

        if pdf_paths.is_empty() {
            continue;
        }

        let group_output = evidence_dir.join(format!(
            "{}-{:016x}.pdf",
            safe_file_stem(group_name),
            fnv1a_hash(group_id)
        ));
        merge_pdfs_with_qpdf(&qpdf_bin, &pdf_paths, &group_output)?;

        let page_count = qpdf_page_count(&qpdf_bin, &group_output.display().to_string())?;

        results.push(serde_json::json!({
            "groupId": group_id,
            "name": group_name,
            "outputPath": group_output.display().to_string(),
            "pageCount": page_count,
            "conversionFailures": failed_conversions
                .iter()
                .filter(|item| item["groupId"].as_str() == Some(group_id))
                .cloned()
                .collect::<Vec<_>>(),
        }));
    }

    Ok(serde_json::json!({
        "evidenceDir": evidence_dir.display().to_string(),
        "results": results,
        "failedConversions": failed_conversions,
    }))
}

pub fn merge_all(args: &serde_json::Value) -> Result<String> {
    let evidence_dir = args["evidenceDir"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少 evidenceDir 参数"))?;
    let group_pdfs = args["groupPdfs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("缺少 groupPdfs 参数"))?;

    if group_pdfs.is_empty() {
        anyhow::bail!("没有可合并的分组 PDF");
    }

    let output_path_str = args["outputPath"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Path::new(evidence_dir)
                .join(format!("evidence_merged_{}.pdf", ts))
                .display()
                .to_string()
        });

    let identity: Option<IdentityConfig> = args
        .get("identity")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let overlay_cfg: Option<OverlayConfig> = args
        .get("overlay")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let qpdf_bin = crate::external::QpdfTool.binary_path()?;

    let inputs: Vec<String> = group_pdfs
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let renamed_paths = if let Some(ref ident) = identity {
        apply_identity_rename(&inputs, evidence_dir, ident)?
    } else {
        inputs
    };

    // TODO: GLM-P2 迁移到 header_footer::batch_overlay 替代 apply_overlay_batch
    let overlaid_paths = if let Some(ref cfg) = overlay_cfg {
        apply_overlay_batch(&qpdf_bin, &renamed_paths, evidence_dir, cfg)?
    } else {
        renamed_paths
    };

    merge_pdfs_with_qpdf(&qpdf_bin, &overlaid_paths, Path::new(&output_path_str))?;

    Ok(output_path_str)
}

fn convert_word_to_pdf(
    doc_path: &str,
    output_dir: &Path,
    conversion_state: &Arc<ConversionState>,
) -> Result<String> {
    let mut attempts = Vec::new();
    let canonical = std::fs::canonicalize(doc_path)
        .with_context(|| format!("读取 Word 文件失败: {doc_path}"))?;
    let conversion_dir = output_dir.join("_converted").join(format!(
        "{:016x}",
        fnv1a_hash(&canonical.display().to_string())
    ));
    fs::create_dir_all(&conversion_dir).context("创建 Word 转换工作目录失败")?;

    #[cfg(any(windows, target_os = "macos"))]
    {
        match convert_doc_to_pdf_with_word(doc_path, &conversion_dir, conversion_state) {
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
        match convert_doc_to_pdf_with_wps(doc_path, &conversion_dir, conversion_state) {
            Ok(path) => return Ok(path),
            Err(err) => attempts.push(format!("WPS Writer 转换失败: {err}")),
        }
    }

    match crate::external::LibreOfficeTool.binary_path() {
        Ok(lo_bin) => match convert_doc_to_pdf_with_libreoffice(
            &lo_bin,
            doc_path,
            &conversion_dir,
            conversion_state,
        ) {
            Ok(path) => Ok(path),
            Err(err) => {
                attempts.push(format!("LibreOffice 转换失败: {err}"));
                anyhow::bail!(
                    "没有可用的 Word 转 PDF 引擎，文件未转换: {}。{}",
                    doc_path,
                    attempts.join("；")
                );
            }
        },
        Err(err) => {
            attempts.push(format!("未检测到 LibreOffice: {err}"));
            anyhow::bail!(
                "没有可用的 Word 转 PDF 引擎，文件未转换: {}。{}",
                doc_path,
                attempts.join("；")
            );
        }
    }
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
    let output_result = run_process_with_interactive_timeout(
        &mut cmd,
        std::time::Duration::from_secs(60),
        conversion_state,
        app,
    )
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
    let output_result = run_process_with_interactive_timeout(
        &mut cmd,
        std::time::Duration::from_secs(60),
        conversion_state,
        app,
    )
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
    let output = output_dir.join(format!("{stem}.pdf"));
    let script = r#"
on run argv
  set inputPath to item 1 of argv
  set outputPath to item 2 of argv
  set inputHfsPath to POSIX file inputPath as text
  set outputFile to POSIX file outputPath
  set docRef to missing value
  tell application "Microsoft Word"
    set visible to false
    try
      open file inputHfsPath
      set docRef to active document
      save as docRef file name outputFile file format format PDF
    on error errMsg number errNum
      try
        if docRef is not missing value then close docRef saving no
      end try
      error errMsg number errNum
    end try
    close docRef saving no
  end tell
end run
"#;

    let mut command = crate::external::hidden_command("osascript");
    command
        .arg("-e")
        .arg(script)
        .arg(input.display().to_string())
        .arg(output.display().to_string());
    let app = crate::get_app_handle().ok_or_else(|| anyhow::anyhow!("应用未初始化"))?;
    let result = run_process_with_interactive_timeout(
        &mut command,
        std::time::Duration::from_secs(60),
        conversion_state,
        app,
    )
    .context("启动 Microsoft Word 转换失败")?;

    if !result.status.success() || !output.exists() {
        anyhow::bail!(
            "Microsoft Word 转 PDF 失败: {doc_path}（{}）",
            crate::external::command_failure_detail(&result)
        );
    }
    Ok(output.display().to_string())
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
        conversion_state,
        app,
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

fn merge_pdfs_with_qpdf(qpdf_bin: &Path, inputs: &[String], output: &Path) -> Result<()> {
    if inputs.is_empty() {
        anyhow::bail!("没有可合并的 PDF");
    }

    let status = crate::external::hidden_command(qpdf_bin)
        .arg("--empty")
        .arg("--pages")
        .args(inputs)
        .arg("--")
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !super::qpdf::status_is_success(&status) {
        anyhow::bail!("qpdf 合并失败");
    }

    Ok(())
}

fn qpdf_page_count(qpdf_bin: &Path, path: &str) -> Result<u32> {
    let output = crate::external::hidden_command(qpdf_bin)
        .arg("--show-npages")
        .arg(path)
        .output()?;
    if !super::qpdf::status_is_success(&output.status) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("qpdf 读取页数失败 {}: {}", path, stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().parse::<u32>()?)
}

fn apply_identity_rename(
    inputs: &[String],
    output_dir: &str,
    identity: &IdentityConfig,
) -> Result<Vec<String>> {
    let prefix = identity.prefix.as_deref().unwrap_or("evidence");
    let start = identity.start_number.unwrap_or(1);

    let rename_dir = Path::new(output_dir).join("_renamed");
    fs::create_dir_all(&rename_dir)?;

    let mut result = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        let num = start + i as u32;
        let new_name = format!("{}{:04}.pdf", prefix, num);
        let new_path = rename_dir.join(&new_name);
        fs::copy(input, &new_path)?;
        result.push(new_path.display().to_string());
    }

    Ok(result)
}

// TODO: GLM-P2 迁移到 header_footer::batch_overlay
fn apply_overlay_batch(
    qpdf_bin: &Path,
    inputs: &[String],
    output_dir: &str,
    config: &OverlayConfig,
) -> Result<Vec<String>> {
    let overlay_dir = Path::new(output_dir).join("_overlaid");
    fs::create_dir_all(&overlay_dir)?;

    let init_seq = config
        .header
        .as_ref()
        .and_then(|h| {
            if h.content == "sequence" {
                h.start_number
            } else {
                None
            }
        })
        .unwrap_or(1);

    let mut result = Vec::new();
    let mut global_seq: u32 = init_seq;

    for input in inputs {
        let stem = Path::new(input)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("page");
        let output = overlay_dir.join(format!(
            "{}-{:016x}_overlay.pdf",
            safe_file_stem(stem),
            fnv1a_hash(input)
        ));

        global_seq = apply_overlay_single(
            qpdf_bin,
            input,
            &output.display().to_string(),
            config,
            global_seq,
        )?;
        result.push(output.display().to_string());
    }

    Ok(result)
}

// TODO: GLM-P2 迁移到 header_footer::overlay_text
fn apply_overlay_single(
    qpdf_bin: &Path,
    input: &str,
    output: &str,
    config: &OverlayConfig,
    mut seq: u32,
) -> Result<u32> {
    let page_count = qpdf_page_count(qpdf_bin, input)?;

    let dims = qpdf_all_page_dimensions(input);
    let default_dim = (595.276, 841.89);

    let mut overlay_pages: Vec<printpdf::PdfPage> = Vec::new();

    for page_idx in 0..page_count {
        let page_num = page_idx + 1;
        let (width_pt, height_pt) = dims.get(page_idx as usize).copied().unwrap_or(default_dim);

        let overlay_ops = build_overlay_ops(
            config, input, page_num, page_count, seq, width_pt, height_pt,
        )?;

        if !overlay_ops.is_empty() {
            use printpdf::*;
            let page = PdfPage::new(
                Mm(width_pt as f32 * 25.4 / 72.0),
                Mm(height_pt as f32 * 25.4 / 72.0),
                overlay_ops,
            );
            overlay_pages.push(page);
        }

        if matches!(
            config.header.as_ref().map(|h| h.content.as_str()),
            Some("sequence")
        ) {
            seq += 1;
        }
    }

    if overlay_pages.is_empty() {
        fs::copy(input, output)?;
        return Ok(seq);
    }

    let overlay_bytes = create_overlay_pdf_multi(overlay_pages)?;
    let temp_overlay = unique_temp_pdf(
        Path::new(output).parent().unwrap_or(Path::new(".")),
        "_overlay_temp",
    );
    fs::write(&temp_overlay, &overlay_bytes)?;

    let status = crate::external::hidden_command(qpdf_bin)
        .arg(input)
        .arg("--overlay")
        .arg(&temp_overlay)
        .arg("--")
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let _ = fs::remove_file(&temp_overlay);
    let status = status?;

    if !super::qpdf::status_is_success(&status) {
        anyhow::bail!("qpdf overlay 失败: {}", input);
    }

    Ok(seq)
}

fn build_overlay_ops(
    config: &OverlayConfig,
    file_path: &str,
    page_num: u32,
    page_count: u32,
    seq: u32,
    _width_pt: f64,
    height_pt: f64,
) -> Result<Vec<printpdf::Op>> {
    use printpdf::*;

    let mut ops: Vec<Op> = Vec::new();

    let has_header = config.header.as_ref().is_some_and(|h| h.enabled);
    let has_footer = config.footer.as_ref().is_some_and(|f| f.enabled);

    if !has_header && !has_footer {
        return Ok(ops);
    }

    ops.push(Op::StartTextSection);

    let font_handle = PdfFontHandle::Builtin(BuiltinFont::Helvetica);

    if let Some(ref header) = config.header {
        if header.enabled {
            let font_size = header.font_size.unwrap_or(10.0) as f32;
            let y_pt = header.y_offset.unwrap_or(height_pt - 30.0) as f32;

            ops.push(Op::SetFont {
                font: font_handle.clone(),
                size: Pt(font_size),
            });
            ops.push(Op::SetTextCursor {
                pos: Point {
                    x: Pt(36.0),
                    y: Pt(y_pt),
                },
            });

            let text = match header.content.as_str() {
                "filename" => Path::new(file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
                "custom" => header.custom_text.clone().unwrap_or_default(),
                "sequence" => format!("{}", seq),
                _ => String::new(),
            };

            if !text.is_empty() {
                ensure_legacy_overlay_text_supported(&text)?;
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(text)],
                });
            }
        }
    }

    if let Some(ref footer) = config.footer {
        if footer.enabled {
            let font_size = footer.font_size.unwrap_or(9.0) as f32;
            let y_pt = footer.y_offset.unwrap_or(20.0) as f32;

            ops.push(Op::SetFont {
                font: font_handle,
                size: Pt(font_size),
            });
            ops.push(Op::SetTextCursor {
                pos: Point {
                    x: Pt(36.0),
                    y: Pt(y_pt),
                },
            });

            let text = match footer.content.as_str() {
                "page_total" => format!("{} / {}", page_num, page_count),
                _ => String::new(),
            };

            if !text.is_empty() {
                ensure_legacy_overlay_text_supported(&text)?;
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(text)],
                });
            }
        }
    }

    ops.push(Op::EndTextSection);

    Ok(ops)
}

/// The legacy evidence-overlay path only has PDF base-14 fonts available.
/// Helvetica cannot render CJK text reliably, so fail before producing a PDF
/// with invisible or corrupted evidence labels. The current evidence workflow
/// uses the embedded-font header/footer renderer instead.
// TODO: GLM-P2 迁移后删除 — header_footer 使用嵌入字体，无需 ASCII 限制
fn ensure_legacy_overlay_text_supported(text: &str) -> Result<()> {
    if !text.is_ascii() {
        anyhow::bail!(
            "旧版证据叠加不支持中文或其他非 ASCII 文本；请使用“分项证据处理”的页眉页脚设置"
        );
    }
    Ok(())
}

fn create_overlay_pdf_multi(pages: Vec<printpdf::PdfPage>) -> Result<Vec<u8>> {
    use printpdf::*;

    let mut doc = PdfDocument::new("overlay");
    doc.with_pages(pages);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

    Ok(bytes)
}

fn qpdf_all_page_dimensions(path: &str) -> Vec<(f64, f64)> {
    super::page_info::get_page_infos(path)
        .map(|pages| {
            pages
                .into_iter()
                .map(|page| (page.width_pt as f64, page.height_pt as f64))
                .collect()
        })
        .unwrap_or_default()
}


fn unique_temp_pdf(dir: &Path, stem: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    dir.join(format!("{stem}_{pid}_{ts}.pdf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_stem_blocks_path_segments() {
        assert_eq!(safe_file_stem("../证据1/../../x"), "证据1_______x");
    }

    #[test]
    fn safe_file_stem_keeps_common_chinese_names() {
        assert_eq!(safe_file_stem("证据 1（聊天记录）"), "证据 1_聊天记录");
    }
}
