use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;
use crate::sort_utils::natural_cmp;

const SUPPORTED_EXTS: &[&str] = &["pdf", "doc", "docx", "docm"];

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

fn fnv1a_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
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

pub fn build_group_pdfs(args: &serde_json::Value) -> Result<serde_json::Value> {
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
                "word" => match convert_word_to_pdf(file_path, &evidence_dir) {
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

        let group_output = evidence_dir.join(format!("{}.pdf", safe_file_stem(group_name)));
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

    let overlaid_paths = if let Some(ref cfg) = overlay_cfg {
        apply_overlay_batch(&qpdf_bin, &renamed_paths, evidence_dir, cfg)?
    } else {
        renamed_paths
    };

    merge_pdfs_with_qpdf(&qpdf_bin, &overlaid_paths, Path::new(&output_path_str))?;

    Ok(output_path_str)
}

fn convert_word_to_pdf(doc_path: &str, output_dir: &Path) -> Result<String> {
    let mut attempts = Vec::new();

    if cfg!(windows) || cfg!(target_os = "macos") {
        match crate::external::WordTool.binary_path() {
            Ok(_) => match convert_doc_to_pdf_with_word(doc_path, output_dir) {
                Ok(path) => return Ok(path),
                Err(err) => attempts.push(format!("Microsoft Word 转换失败: {err}")),
            },
            Err(err) => attempts.push(format!("未检测到 Microsoft Word: {err}")),
        }
    } else {
        attempts.push("当前平台不支持 Microsoft Word 自动转换".to_string());
    }

    if cfg!(windows) {
        match crate::external::WpsTool.binary_path() {
            Ok(_) => match convert_doc_to_pdf_with_wps(doc_path, output_dir) {
                Ok(path) => return Ok(path),
                Err(err) => attempts.push(format!("WPS Writer 转换失败: {err}")),
            },
            Err(err) => attempts.push(format!("未检测到 WPS Writer: {err}")),
        }
    }

    match crate::external::LibreOfficeTool.binary_path() {
        Ok(lo_bin) => match convert_doc_to_pdf_with_libreoffice(&lo_bin, doc_path, output_dir) {
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
fn convert_doc_to_pdf_with_word(doc_path: &str, output_dir: &Path) -> Result<String> {
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

    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("启动 Microsoft Word 转换失败")?;

    if !status.success() || !output.exists() {
        anyhow::bail!("Microsoft Word 转 PDF 失败: {doc_path}");
    }
    Ok(output.display().to_string())
}

#[cfg(windows)]
fn convert_doc_to_pdf_with_wps(doc_path: &str, output_dir: &Path) -> Result<String> {
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

    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("启动 WPS Writer 转换失败")?;

    if !status.success() || !output.exists() {
        anyhow::bail!("WPS Writer 转 PDF 失败: {doc_path}");
    }
    Ok(output.display().to_string())
}

#[cfg(not(windows))]
fn convert_doc_to_pdf_with_wps(_doc_path: &str, _output_dir: &Path) -> Result<String> {
    anyhow::bail!("当前平台不支持 WPS Writer 自动转换")
}

#[cfg(target_os = "macos")]
fn convert_doc_to_pdf_with_word(doc_path: &str, output_dir: &Path) -> Result<String> {
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

    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .arg(input.display().to_string())
        .arg(output.display().to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("启动 Microsoft Word 转换失败")?;

    if !status.success() || !output.exists() {
        anyhow::bail!("Microsoft Word 转 PDF 失败: {doc_path}");
    }
    Ok(output.display().to_string())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn convert_doc_to_pdf_with_word(_doc_path: &str, _output_dir: &Path) -> Result<String> {
    anyhow::bail!("当前平台不支持 Microsoft Word 自动转换")
}

fn convert_doc_to_pdf_with_libreoffice(
    lo_bin: &Path,
    doc_path: &str,
    output_dir: &Path,
) -> Result<String> {
    let status = std::process::Command::new(lo_bin)
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(output_dir)
        .arg(doc_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        anyhow::bail!("Word 文件转换失败: {}", doc_path);
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

    let status = std::process::Command::new(qpdf_bin)
        .arg("--empty")
        .arg("--pages")
        .args(inputs)
        .arg("--")
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        anyhow::bail!("qpdf 合并失败");
    }

    Ok(())
}

fn qpdf_page_count(qpdf_bin: &Path, path: &str) -> Result<u32> {
    let output = std::process::Command::new(qpdf_bin)
        .arg("--show-npages")
        .arg(path)
        .output()?;
    if !output.status.success() {
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
        let output = overlay_dir.join(format!("{}_overlay.pdf", safe_file_stem(stem)));

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
        );

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

    let status = std::process::Command::new(qpdf_bin)
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

    if !status.success() {
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
) -> Vec<printpdf::Op> {
    use printpdf::*;

    let mut ops: Vec<Op> = Vec::new();

    let has_header = config.header.as_ref().is_some_and(|h| h.enabled);
    let has_footer = config.footer.as_ref().is_some_and(|f| f.enabled);

    if !has_header && !has_footer {
        return ops;
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
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(text)],
                });
            }
        }
    }

    ops.push(Op::EndTextSection);

    ops
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

fn safe_file_stem(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '\u{4e00}'..='\u{9fff}' | '-' | '_' | ' ' | '(' | ')' | '[' | ']'
            )
        {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches(|c| matches!(c, ' ' | '.' | '_')).trim();
    if trimmed.is_empty() {
        "output".to_string()
    } else {
        trimmed.to_string()
    }
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
