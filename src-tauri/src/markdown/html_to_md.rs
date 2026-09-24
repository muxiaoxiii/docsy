//! HTML → Markdown：基于 `htmd`（turndown.js 同源语义）做语义结构转换。
//!
//! 不执行脚本；跳过 `script` / `style` / `noscript` / `template` / `svg` / `canvas`。
//! 支持标题、段落、列表、表格、链接、图片、代码块与常见行内样式。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::util::fs::unique_output_path;

/// Convert an HTML file to Markdown beside the input (or into `output_dir`).
pub fn convert(input: &Path, output_dir: Option<&str>) -> Result<String> {
    let html = std::fs::read_to_string(input)
        .with_context(|| format!("无法读取 HTML 文件: {}", input.display()))?;
    if html.trim().is_empty() {
        anyhow::bail!("HTML 文件没有可提取的内容: {}", input.display());
    }

    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec![
            "script",
            "style",
            "noscript",
            "template",
            "svg",
            "canvas",
            "nav",
            "header",
            "footer",
            "aside",
        ])
        .build();
    let markdown = converter
        .convert(&html)
        .with_context(|| format!("HTML 转换为 Markdown 失败: {}", input.display()))?;

    let normalized = normalize_markdown(&markdown);
    if normalized.trim().is_empty() {
        anyhow::bail!("未从 HTML 中提取到可转换的文本内容: {}", input.display());
    }

    let dir = match output_dir {
        Some(d) => {
            let path = Path::new(d);
            if !path.is_dir() {
                anyhow::bail!("输出目录不存在: {}", d);
            }
            path.to_path_buf()
        }
        None => input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .context("无法解析 HTML 文件名")?;
    let output = unique_output_path(&dir, stem, "md");
    std::fs::write(&output, &normalized)
        .with_context(|| format!("无法写入 Markdown 文件: {}", output.display()))?;
    Ok(output.display().to_string())
}

/// Keep source paragraph breaks stable across producers; trim trailing space only.
fn normalize_markdown(markdown: &str) -> String {
    let text = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_basic_structure() {
        let dir = std::env::temp_dir().join(format!("docsy-html-md-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("sample.html");
        std::fs::write(
            &input,
            r#"<!DOCTYPE html>
<html><head><title>t</title><style>body{color:red}</style></head>
<body>
<nav>站点导航</nav>
<script>console.log(1)</script>
<h1>标题</h1>
<p>正文 <strong>加粗</strong> 与 <a href="https://example.com">链接</a>。</p>
<ul><li>项目一</li><li>项目二</li></ul>
<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>
<pre><code>code_block()</code></pre>
</body></html>"#,
        )
        .unwrap();

        let out = convert(&input, None).unwrap();
        let md = std::fs::read_to_string(&out).unwrap();
        assert!(md.contains("# 标题"), "heading: {md}");
        assert!(md.contains("加粗"), "bold text: {md}");
        assert!(md.contains("[链接](https://example.com)"), "link: {md}");
        assert!(md.contains("项目一"), "list: {md}");
        assert!(md.contains("|"), "table: {md}");
        assert!(md.contains("code_block()"), "code: {md}");
        assert!(!md.contains("console.log"), "script skipped: {md}");
        assert!(!md.contains("color:red"), "style skipped: {md}");
        assert!(!md.contains("站点导航"), "nav skipped: {md}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_empty_html() {
        let dir = std::env::temp_dir().join(format!("docsy-html-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("empty.html");
        std::fs::write(&input, "   \n").unwrap();
        let err = convert(&input, None).unwrap_err().to_string();
        assert!(err.contains("没有可提取") || err.contains("未从 HTML"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
