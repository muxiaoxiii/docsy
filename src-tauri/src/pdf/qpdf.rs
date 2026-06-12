use anyhow::Result;
use std::path::Path;

pub fn inspect(path: &str) -> Result<crate::commands::pdf::InspectResult> {
    let qpdf = crate::external::QpdfTool;
    let bin = qpdf.binary_path()?;
    let output = std::process::Command::new(&bin)
        .arg("--show-encryption")
        .arg(path)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let encrypted = stderr.contains("encrypted") || stderr.contains("password");

    let pages = page_count(path).ok();

    Ok(crate::commands::pdf::InspectResult { encrypted, pages })
}

pub fn unlock(input: &Path) -> Result<crate::commands::pdf::UnlockResult> {
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

    Ok(crate::commands::pdf::UnlockResult {
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
    let stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");

    let pages = page_count(input)?;
    let mut outputs = Vec::new();

    for i in 1..=pages {
        let output = Path::new(output_dir).join(format!("{}-{}.pdf", stem, i));
        let status = std::process::Command::new(&bin)
            .arg("--split-pages")
            .arg(input)
            .arg("--")
            .arg(&output)
            .status()?;
        if status.success() {
            outputs.push(output.display().to_string());
        }
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.trim().parse::<u32>()?;
    Ok(count)
}

fn unique_output_path(input: &Path, suffix: &str) -> std::path::PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("pdf");

    let mut path = parent.join(format!("{}{}.{}", stem, suffix, ext));
    let mut i = 1;
    while path.exists() {
        path = parent.join(format!("{}{}-{}.{}", stem, suffix, i, ext));
        i += 1;
    }
    path
}
