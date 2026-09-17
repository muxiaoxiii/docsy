use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

use crate::external::ExternalTool;

/// 检测 PDF 是否含电子签名/签章 Widget。
/// 仅读 qpdf 对象字典（不拉流数据），避免扫描件把图像载入内存。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureInspection {
    pub has_signatures: bool,
    pub count: usize,
    /// 供 UI 提示的简短说明
    pub summary: String,
}

pub fn inspect_signatures(input: &Path) -> Result<SignatureInspection> {
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }
    let qpdf_bin = crate::external::QpdfTool.binary_path()?;
    let output = crate::external::hidden_command(qpdf_bin)
        .arg("--json=1")
        .arg("--json-key=objects")
        .arg("--json-stream-data=none")
        .arg(input)
        .output()
        .context("读取 PDF 签名信息失败")?;
    if !super::qpdf::status_is_success(&output.status) {
        anyhow::bail!(
            "读取 PDF 签名信息失败：{}",
            crate::external::command_failure_detail(&output)
        );
    }
    let json: Value = serde_json::from_slice(&output.stdout).context("解析 PDF 签名 JSON 失败")?;
    let count = count_signature_objects(&json);
    Ok(SignatureInspection {
        has_signatures: count > 0,
        count,
        summary: if count > 0 {
            format!("检测到 {count} 处电子签名/签章相关对象")
        } else {
            "未检测到电子签名".to_string()
        },
    })
}

fn count_signature_objects(json: &Value) -> usize {
    let Some(objects) = json.get("objects").and_then(Value::as_object) else {
        return 0;
    };
    let mut count = 0_usize;
    for (_key, object) in objects {
        // qpdf 可能直接给字典，也可能包一层 {"value": {...}}
        let dict = object
            .as_object()
            .and_then(|map| {
                if map.contains_key("/Type") || map.contains_key("/FT") || map.contains_key("/Subtype") {
                    Some(map.clone())
                } else {
                    map.get("value").and_then(Value::as_object).cloned()
                }
            });
        let Some(dict) = dict else {
            continue;
        };
        if object_is_signature(&dict) {
            count += 1;
        }
    }
    count
}

fn object_is_signature(dict: &serde_json::Map<String, Value>) -> bool {
    let name_of = |key: &str| {
        dict.get(key)
            .and_then(Value::as_str)
            .map(|value| value.trim_start_matches('/').to_ascii_lowercase())
    };
    // 显式签名字典
    if name_of("/Type").as_deref() == Some("sig") {
        return true;
    }
    // 表单域类型为签名
    if name_of("/FT").as_deref() == Some("sig") {
        return true;
    }
    // Widget 且带有签名值 /V（可能是字典或引用；对象表里 V 为 dict 时含 /Type /Sig）
    if name_of("/Subtype").as_deref() == Some("widget") {
        if let Some(value) = dict.get("/V") {
            if value.as_object().is_some_and(|v| {
                v.get("/Type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t.trim_start_matches('/').eq_ignore_ascii_case("sig"))
            }) {
                return true;
            }
            // /V 为引用字符串时，同表内已由 /Type /Sig 规则计数，避免重复
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counts_sig_dicts_and_ft_sig_fields() {
        let sample = json!({
            "objects": {
                "obj:1 0": { "value": { "/Type": "/Sig", "/Filter": "/Adobe.PPKLite" } },
                "obj:2 0": { "value": { "/FT": "/Sig", "/Subtype": "/Widget" } },
                "obj:3 0": { "value": { "/Type": "/Page" } },
                "obj:4 0": { "value": { "/Subtype": "/Widget", "/V": { "/Type": "/Sig" } } },
            }
        });
        assert_eq!(count_signature_objects(&sample), 3);
    }

    #[test]
    fn ordinary_widget_without_sig_is_ignored() {
        let sample = json!({
            "objects": {
                "obj:1 0": { "value": { "/Subtype": "/Widget", "/FT": "/Tx" } },
            }
        });
        assert_eq!(count_signature_objects(&sample), 0);
    }
}
