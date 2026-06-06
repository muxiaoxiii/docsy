//! docx 模板渲染器
//!
//! 实现策略（详见 docs/docx-research.md 第 5 节）：
//! 1. 把 docx 当 zip 解开，对 word/document.xml 等做文本替换，其余文件原样回写
//! 2. 用 regex 在 XML 上直接找 <w:t>...{{key}}...</w:t>
//! 3. 简单字段：替换 <w:t> 内的占位符文本（XML 转义）
//! 4. party 字段：把单个 <w:r>...{{key}}...</w:r> 拆成 多个 run，顿号 run 去掉 <w:u>
//! 5. hideable 字段且值为空：删掉占位符所在的 <w:r>

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip::read::ZipArchive;
use zip::write::{FileOptions, ZipWriter};

use super::utils::{flatten_nested_paragraphs, xml_escape};

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("读取模板失败：{0}")]
    Read(String),
    #[error("写入输出失败：{0}")]
    Write(String),
    #[error("zip 错误：{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("io 错误：{0}")]
    Io(#[from] std::io::Error),
}

/// 字段值。前端传过来的 JSON 会反序列化成这个枚举。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// 简单文本（text / select / date / reference）
    Text(String),
    /// 当事人列表（party 字段）
    Parties(Vec<String>),
}

impl FieldValue {
    fn is_empty(&self) -> bool {
        match self {
            FieldValue::Text(s) => s.is_empty(),
            FieldValue::Parties(v) => v.is_empty() || v.iter().all(|s| s.is_empty()),
        }
    }
}

/// 单字段在模板中的渲染策略。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FieldOptions {
    #[serde(default)]
    pub hideable: bool,
    #[serde(default)]
    pub separator: String,
    /// party 字段的分隔符 run 是否移除下划线
    #[serde(default)]
    pub separator_drop_underline: bool,
    /// 每个值后面追加的固定文本（例如律师 -> 每个名字后跟"律师"）
    #[serde(default)]
    pub value_suffix: String,
}

/// 渲染请求
pub struct RenderRequest<'a> {
    pub template_bytes: &'a [u8],
    pub values: HashMap<String, FieldValue>,
    pub field_opts: HashMap<String, FieldOptions>,
}

pub fn render(req: RenderRequest) -> Result<Vec<u8>, RenderError> {
    let cursor = Cursor::new(req.template_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(RenderError::Zip)?;

    let mut out_buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut out_buf));
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;

            let new_data = if is_replaceable_xml(&name) {
                render_xml(&data, &req.values, &req.field_opts)?
            } else {
                data
            };

            writer.start_file(&name, opts).map_err(RenderError::Zip)?;
            writer
                .write_all(&new_data)
                .map_err(|e| RenderError::Write(e.to_string()))?;
        }

        writer.finish().map_err(RenderError::Zip)?;
    }

    Ok(out_buf)
}

fn is_replaceable_xml(name: &str) -> bool {
    name == "word/document.xml"
        || (name.starts_with("word/header") && name.ends_with(".xml"))
        || (name.starts_with("word/footer") && name.ends_with(".xml"))
}

/// 在一个 XML 字符串里执行字段替换。
fn render_xml(
    bytes: &[u8],
    values: &HashMap<String, FieldValue>,
    opts: &HashMap<String, FieldOptions>,
) -> Result<Vec<u8>, RenderError> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|e| RenderError::Read(format!("XML 非 UTF-8：{e}")))?
        .to_string();

    // 第 -1 步：修复 WPS 产生的嵌套 <w:p>（非法 OOXML，Word 无法解析）
    let xml = flatten_nested_paragraphs(&xml);

    // 第零步：表格行重复 {{*key}} —— 把含此占位符的 <w:tr> 按 list 项克隆 N 次
    let xml = process_row_repeats(&xml, values);

    // 第一步：处理 party 字段（拆 run）和 hideable（删 run）
    let xml = process_runs(&xml, values, opts)?;

    // 第二步：处理简单文本字段（在 <w:t> 内直接替换）
    let xml = process_text(&xml, values, opts);

    Ok(xml.into_bytes())
}

/// 对每个含 `{{*key}}` 占位符的 `<w:tr>...</w:tr>`：
/// - 取该字段的 list 值（party 或 multiple text）
/// - 把行整体复制 N 份
/// - 第 i 份里把 `{{*key}}` 替换成 `{{key}}`，然后在外层把 values 单值替换为 list[i]
///
/// 实现简化：直接在 XML 上做"按 tr 切片 + 字符串替换"，每行单独把
/// `{{*key}}` 文本替换为对应的 list[i] 值（XML 转义）。这样后续 process_text
/// 和 process_runs 都不需要再处理这些行（它们已经没有占位符）。
fn process_row_repeats(xml: &str, values: &HashMap<String, FieldValue>) -> String {
    let tr_re = Regex::new(r"(?s)<w:tr\b[^>]*>.*?</w:tr>").unwrap();
    let star_re = Regex::new(r"\{\{\*(\w+)\}\}").unwrap();

    let mut out = String::with_capacity(xml.len());
    let mut last = 0;

    for m in tr_re.find_iter(xml) {
        out.push_str(&xml[last..m.start()]);
        last = m.end();
        let row = m.as_str();

        // 找该行用到的所有 *key
        let keys: Vec<String> = star_re
            .captures_iter(row)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if keys.is_empty() {
            out.push_str(row);
            continue;
        }

        // 用第一个 key 决定 list 长度（按用户用法假定它们等长）
        let list_len = keys
            .iter()
            .map(|k| match values.get(k) {
                Some(FieldValue::Parties(v)) => v.len(),
                _ => 0,
            })
            .max()
            .unwrap_or(0);

        if list_len == 0 {
            // 列表为空：行整体删除
            continue;
        }

        for i in 0..list_len {
            let mut row_copy = row.to_string();
            for k in &keys {
                let v_at_i = match values.get(k) {
                    Some(FieldValue::Parties(v)) => v.get(i).cloned().unwrap_or_default(),
                    _ => String::new(),
                };
                row_copy = row_copy.replace(&format!("{{{{*{}}}}}", k), &xml_escape(&v_at_i));
            }
            // 自动编号：{{#row}} 替换为 i+1（基于 1）
            row_copy = row_copy.replace("{{#row}}", &(i + 1).to_string());
            out.push_str(&row_copy);
        }
    }

    out.push_str(&xml[last..]);
    out
}

/// 处理整 run 替换的字段：party / hideable / 条件前缀 {{?key:text}}
///
/// 关键：不能用跨 run 的贪婪 regex，必须精确按 `<w:r>...</w:r>` 切片。
/// 否则 `<w:r>原告</w:r><w:r>{{plaintiffs}}</w:r>` 会被匹配成一个整体，
/// 导致"原告"前缀被吞掉。
fn process_runs(
    xml: &str,
    values: &HashMap<String, FieldValue>,
    opts: &HashMap<String, FieldOptions>,
) -> Result<String, RenderError> {
    let run_re = Regex::new(r"(?s)<w:r\b[^>]*>.*?</w:r>").unwrap();
    let placeholder_in_run =
        Regex::new(r"(?s)<w:t[^>]*>([^<]*?)\{\{(\w+)\}\}([^<]*?)</w:t>").unwrap();
    let conditional_re = Regex::new(r"\{\{\?(\w+):([^}]*)\}\}").unwrap();

    let mut out = String::with_capacity(xml.len());
    let mut last = 0;

    for run_match in run_re.find_iter(xml) {
        out.push_str(&xml[last..run_match.start()]);
        last = run_match.end();

        let run = run_match.as_str();

        // 先看 run 里是否含条件前缀 {{?key:text}}
        if conditional_re.is_match(run) {
            let resolved = conditional_re.replace_all(run, |c: &regex::Captures| {
                let key = c.get(1).unwrap().as_str();
                let text = c.get(2).unwrap().as_str();
                let has_value = values.get(key).map(|v| !v.is_empty()).unwrap_or(false);
                if has_value {
                    text.to_string()
                } else {
                    String::new()
                }
            });
            // 如果替换后 <w:t> 为空 且没有其他占位符，整个 run 删掉
            let r = resolved.into_owned();
            if t_is_empty(&r) {
                continue;
            }
            // 还可能有别的占位符（罕见但允许），交给后续步骤处理
            out.push_str(&r);
            continue;
        }

        // 标准占位符
        let Some(ph) = placeholder_in_run.captures(run) else {
            out.push_str(run);
            continue;
        };

        let prefix = ph.get(1).unwrap().as_str();
        let key = ph.get(2).unwrap().as_str();
        let suffix = ph.get(3).unwrap().as_str();

        match values.get(key) {
            Some(FieldValue::Parties(list)) => {
                let opt = opts.get(key).cloned().unwrap_or_default();
                let sep = if opt.separator.is_empty() {
                    "、".to_string()
                } else {
                    opt.separator.clone()
                };
                if prefix.is_empty() && suffix.is_empty() {
                    if list.is_empty() && opt.hideable {
                        // 隐藏 run
                    } else if list.is_empty() {
                        out.push_str(&replace_placeholder_with_blank(run, key));
                    } else {
                        let rendered = render_party_runs(
                            run,
                            list,
                            &sep,
                            opt.separator_drop_underline,
                            &opt.value_suffix,
                        );
                        out.push_str(&rendered);
                    }
                } else {
                    let joined = list
                        .iter()
                        .map(|n| format!("{}{}", n, opt.value_suffix))
                        .collect::<Vec<_>>()
                        .join(&sep);
                    out.push_str(&run.replace(&format!("{{{{{}}}}}", key), &xml_escape(&joined)));
                }
            }
            Some(v) if v.is_empty() && opts.get(key).is_some_and(|o| o.hideable) => {
                // 隐藏 run
            }
            _ => {
                out.push_str(run);
            }
        }
    }

    out.push_str(&xml[last..]);
    Ok(out)
}

/// 判断一个 run 的 <w:t> 是否为空（用来决定整段 run 是否删除）
fn t_is_empty(run: &str) -> bool {
    let t_re = Regex::new(r"(?s)<w:t[^>]*>([^<]*)</w:t>").unwrap();
    t_re.captures(run)
        .map(|c| c.get(1).unwrap().as_str().is_empty())
        .unwrap_or(false)
}

/// 把单个 <w:r>{{key}}</w:r> 拆成多个 run，姓名继承原 rPr，分隔符 run 可去下划线。
/// 如有 value_suffix（如"律师"），每个名字后追加。
fn render_party_runs(
    original_run: &str,
    names: &[String],
    sep: &str,
    drop_u: bool,
    value_suffix: &str,
) -> String {
    if names.is_empty() {
        return String::new();
    }

    let rpr_re = Regex::new(r"(?s)<w:rPr>(.*?)</w:rPr>").unwrap();
    let rpr_inner = rpr_re
        .captures(original_run)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .unwrap_or_default();

    let sep_rpr_inner = if drop_u {
        let u_re = Regex::new(r"(?s)<w:u\b[^/>]*/?>(?:</w:u>)?").unwrap();
        u_re.replace_all(&rpr_inner, "").to_string()
    } else {
        rpr_inner.clone()
    };

    let mut out = String::new();
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            out.push_str(&build_run(&sep_rpr_inner, sep));
        }
        let combined = format!("{}{}", name, value_suffix);
        out.push_str(&build_run(&rpr_inner, &combined));
    }
    out
}

fn build_run(rpr_inner: &str, text: &str) -> String {
    let rpr = if rpr_inner.is_empty() {
        String::new()
    } else {
        format!("<w:rPr>{}</w:rPr>", rpr_inner)
    };
    let preserve = if text.starts_with(' ') || text.ends_with(' ') {
        r#" xml:space="preserve""#
    } else {
        ""
    };
    format!(
        "<w:r>{}<w:t{}>{}</w:t></w:r>",
        rpr,
        preserve,
        xml_escape(text)
    )
}

/// 简单文本字段：在保留 <w:r> 结构前提下，把 <w:t> 文本里的 {{key}} 直接替换
fn process_text(
    xml: &str,
    values: &HashMap<String, FieldValue>,
    opts: &HashMap<String, FieldOptions>,
) -> String {
    // 查每个 <w:t ...>...</w:t>，把里面的 {{key}} 替换
    let t_re = Regex::new(r"(?s)<w:t([^>]*)>([^<]*)</w:t>").unwrap();
    let placeholder_re = Regex::new(r"\{\{(\w+)\}\}").unwrap();

    t_re.replace_all(xml, |caps: &regex::Captures| {
        let attrs = &caps[1];
        let body = &caps[2];

        let new_body = placeholder_re.replace_all(body, |c: &regex::Captures| {
            let key = &c[1];
            match values.get(key) {
                Some(FieldValue::Text(s)) if s.is_empty() && !is_hideable(opts, key) => {
                    xml_escape(&blank_for_placeholder(key))
                }
                Some(FieldValue::Text(s)) => xml_escape(s),
                Some(FieldValue::Parties(list)) if list.is_empty() && !is_hideable(opts, key) => {
                    xml_escape(&blank_for_placeholder(key))
                }
                Some(FieldValue::Parties(list)) => xml_escape(&list.join("、")),
                None => c.get(0).unwrap().as_str().to_string(),
            }
        });

        // 如果替换后首尾出现空格，确保 xml:space="preserve"
        let needs_preserve =
            (new_body.starts_with(' ') || new_body.ends_with(' ')) && !attrs.contains("xml:space");
        let attrs_final = if needs_preserve {
            format!("{} xml:space=\"preserve\"", attrs)
        } else {
            attrs.to_string()
        };

        format!("<w:t{}>{}</w:t>", attrs_final, new_body)
    })
    .into_owned()
}

fn is_hideable(opts: &HashMap<String, FieldOptions>, key: &str) -> bool {
    opts.get(key).is_some_and(|o| o.hideable)
}

fn blank_for_placeholder(key: &str) -> String {
    " ".repeat(format!("{{{{{key}}}}}").chars().count())
}

fn replace_placeholder_with_blank(run: &str, key: &str) -> String {
    let placeholder = format!("{{{{{key}}}}}");
    let replaced = run.replace(&placeholder, &xml_escape(&blank_for_placeholder(key)));
    ensure_xml_space_preserve(&replaced)
}

fn ensure_xml_space_preserve(run: &str) -> String {
    let t_re = Regex::new(r#"(?s)<w:t([^>]*)>"#).unwrap();
    t_re.replace(run, |caps: &regex::Captures| {
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if attrs.contains("xml:space") {
            caps.get(0).unwrap().as_str().to_string()
        } else {
            format!(r#"<w:t{} xml:space="preserve">"#, attrs)
        }
    })
    .into_owned()
}

/// 修复 WPS 产生的嵌套 `<w:p>` 结构（非法 OOXML）。
///
/// WPS 会在 `<w:p>` 内再套一层相同的 `<w:p>`，内层带 `<w:pPr>` 和内容。
/// 检测 `<w:p ATTR1><w:p ATTR2>` 模式，去掉外层空壳，保留内层。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_basic() {
        assert_eq!(xml_escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn build_run_preserves_spaces() {
        let r = build_run("", " hello ");
        assert!(r.contains(r#"xml:space="preserve""#));
    }

    #[test]
    fn process_text_preserves_empty_non_hideable_placeholder_width() {
        let xml = r#"<w:r><w:t>{{field_1}}</w:t></w:r>"#;
        let mut values = HashMap::new();
        values.insert("field_1".to_string(), FieldValue::Text(String::new()));
        let opts = HashMap::new();

        let rendered = process_text(xml, &values, &opts);

        assert!(rendered.contains(r#"xml:space="preserve""#));
        assert!(rendered.contains("           "));
        assert!(!rendered.contains("{{field_1}}"));
    }

    #[test]
    fn process_text_removes_empty_hideable_placeholder() {
        let xml = r#"<w:r><w:t>{{field_1}}</w:t></w:r>"#;
        let mut values = HashMap::new();
        values.insert("field_1".to_string(), FieldValue::Text(String::new()));
        let mut opts = HashMap::new();
        opts.insert(
            "field_1".to_string(),
            FieldOptions {
                hideable: true,
                ..Default::default()
            },
        );

        let rendered = process_runs(xml, &values, &opts).unwrap();

        assert_eq!(rendered, "");
    }

    #[test]
    fn render_user_template_produces_valid_zip() {
        // Load the actual user template from the data directory
        let data_dir = dirs::data_dir().expect("无法获取数据目录");
        let tpl_path = data_dir.join("Docsy/user_templates/1.docsytpl");
        if !tpl_path.exists() {
            eprintln!("跳过测试：用户模板不存在");
            return;
        }

        let file = std::fs::File::open(&tpl_path).expect("打开模板失败");
        let mut archive = ZipArchive::new(file).expect("读取 zip 失败");
        let mut template_docx = Vec::new();
        archive
            .by_name("template.docx")
            .expect("找不到 template.docx")
            .read_to_end(&mut template_docx)
            .expect("读取 template.docx 失败");

        // Build test values
        let mut values = HashMap::new();
        values.insert("field_1".to_string(), FieldValue::Text("张三".to_string()));
        values.insert("field_3".to_string(), FieldValue::Text("李四".to_string()));
        values.insert(
            "field_4".to_string(),
            FieldValue::Text("13800138000".to_string()),
        );
        values.insert(
            "field_5".to_string(),
            FieldValue::Text("北京志霖律师事务所".to_string()),
        );
        values.insert("field_6".to_string(), FieldValue::Text("11575".to_string()));
        values.insert(
            "field_7".to_string(),
            FieldValue::Text("北京市朝阳区".to_string()),
        );
        values.insert(
            "field_8".to_string(),
            FieldValue::Text("100020".to_string()),
        );
        values.insert("field_9".to_string(), FieldValue::Text("张三".to_string()));
        values.insert(
            "field_11".to_string(),
            FieldValue::Text("特别代理".to_string()),
        );
        values.insert(
            "field_13".to_string(),
            FieldValue::Text("某公司".to_string()),
        );
        values.insert(
            "field_14".to_string(),
            FieldValue::Text("北京志霖律师事务所".to_string()),
        );
        values.insert(
            "field_15".to_string(),
            FieldValue::Text("北京市朝阳区光华东里8号".to_string()),
        );
        values.insert(
            "field_16".to_string(),
            FieldValue::Text("100020".to_string()),
        );
        values.insert(
            "field_17".to_string(),
            FieldValue::Text("某公司".to_string()),
        );
        values.insert(
            "field_18".to_string(),
            FieldValue::Text("010-12345678".to_string()),
        );
        values.insert(
            "field_19".to_string(),
            FieldValue::Text("另一公司".to_string()),
        );
        values.insert(
            "field_20".to_string(),
            FieldValue::Text("第三公司".to_string()),
        );
        values.insert(
            "field_21".to_string(),
            FieldValue::Text("太阳能电池模块".to_string()),
        );
        values.insert(
            "field_22".to_string(),
            FieldValue::Text("201510395223.7".to_string()),
        );
        values.insert(
            "field_23".to_string(),
            FieldValue::Text("4W120035".to_string()),
        );

        let field_opts = HashMap::new();

        let req = RenderRequest {
            template_bytes: &template_docx,
            values,
            field_opts,
        };

        let result = render(req).expect("渲染失败");

        // Verify output is a valid zip
        let cursor = std::io::Cursor::new(&result);
        let mut out_archive = ZipArchive::new(cursor).expect("输出不是有效的 zip");
        assert!(out_archive.len() > 0, "输出 zip 为空");

        // Verify document.xml exists and is valid
        let mut doc_entry = out_archive
            .by_name("word/document.xml")
            .expect("输出缺少 document.xml");
        let mut doc_xml = String::new();
        doc_entry
            .read_to_string(&mut doc_xml)
            .expect("读取 document.xml 失败");

        // Check no remaining placeholders (except for empty fields)
        let remaining: Vec<&str> = doc_xml.matches("{{").collect();
        // field_2, field_10, field_12 are not in values, so their placeholders remain
        assert!(remaining.len() <= 3, "过多残留占位符: {:?}", remaining);

        // Write to file for manual inspection
        std::fs::write("/tmp/test_rust_rendered.docx", &result).expect("写入测试文件失败");
        println!(
            "渲染成功: {} bytes, 输出已写入 /tmp/test_rust_rendered.docx",
            result.len()
        );
    }
}
