use crate::external::ExternalTool;
use anyhow::Result;
use std::path::Path;

pub struct InspectResult {
    pub encrypted: bool,
    pub pages: Option<u32>,
}

pub struct UnlockResult {
    pub output_path: String,
}

pub fn inspect(path: &str) -> Result<InspectResult> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output = std::process::Command::new(&bin)
        .arg("--show-encryption")
        .arg(path)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let encrypted = stderr.contains("encrypted") || stderr.contains("password");

    let pages = page_count(path).ok();

    Ok(InspectResult { encrypted, pages })
}

pub fn unlock(input: &Path) -> Result<UnlockResult> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let output_path = unique_output_path(input, "_unlocked");
    let status = std::process::Command::new(&bin)
        .arg("--decrypt")
        .arg("--password=")
        .arg(input)
        .arg(&output_path)
        .status()?;

    if !status.success() {
        anyhow::bail!("qpdf 解锁失败");
    }

    Ok(UnlockResult {
        output_path: output_path.display().to_string(),
    })
}

pub fn merge(inputs: &[String], output: &str) -> Result<String> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;

    let mut cmd = std::process::Command::new(&bin);
    cmd.arg("--empty").arg("--pages");
    for input in inputs {
        cmd.arg(input);
    }
    cmd.arg("--").arg(output);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("qpdf 合并失败");
    }

    Ok(output.to_string())
}

pub fn split(input: &str, output_dir: &str) -> Result<Vec<String>> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let input_path = Path::new(input);
    std::fs::create_dir_all(output_dir)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let pages = page_count(input)?;
    let mut outputs = Vec::new();

    for i in 1..=pages {
        let output = Path::new(output_dir).join(format!("{}-{}.pdf", stem, i));
        let command_output = std::process::Command::new(&bin)
            .arg("--empty")
            .arg("--pages")
            .arg(input)
            .arg(i.to_string())
            .arg("--")
            .arg(&output)
            .output()?;
        if !command_output.status.success() {
            let stderr = String::from_utf8_lossy(&command_output.stderr);
            anyhow::bail!("qpdf 拆分第 {i} 页失败: {}", stderr.trim());
        }
        if !output.exists() {
            anyhow::bail!("qpdf 未生成第 {i} 页输出文件");
        }
        outputs.push(output.display().to_string());
    }

    Ok(outputs)
}

pub fn page_count(input: &str) -> Result<u32> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output = std::process::Command::new(&bin)
        .arg("--show-npages")
        .arg(input)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("qpdf 读取页数失败: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.trim().parse::<u32>()?;
    Ok(count)
}

fn unique_output_path(input: &Path, suffix: &str) -> std::path::PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("pdf");

    let mut path = parent.join(format!("{}{}.{}", stem, suffix, ext));
    let mut i = 1;
    while path.exists() {
        path = parent.join(format!("{}{}-{}.{}", stem, suffix, i, ext));
        i += 1;
    }
    path
}
