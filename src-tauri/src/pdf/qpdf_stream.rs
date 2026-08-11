use crate::external::ExternalTool;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Object, StringFormat};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::temp_named_path;

#[derive(Debug, Clone)]
pub(crate) struct QpdfEditableStream {
    pub dictionary: Value,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QpdfBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct QpdfIndexedPage {
    pub number: u32,
    pub contents: Vec<String>,
    pub fonts: BTreeMap<String, String>,
    pub xobjects: BTreeMap<String, String>,
    pub properties: Dictionary,
    pub page_box: Option<QpdfBox>,
}

#[derive(Debug, Clone)]
pub(crate) struct QpdfObjectIndex {
    objects: Map<String, Value>,
    pages: Vec<QpdfIndexedPage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PredefinedFontInfo {
    pub encoding: Option<String>,
    pub ordering: Option<String>,
}

impl QpdfObjectIndex {
    pub(crate) fn load(input: &Path) -> Result<Self> {
        let qpdf_bin = crate::external::QpdfTool.binary_path()?;
        let output = crate::external::hidden_command(qpdf_bin)
            .arg("--json=1")
            .arg("--json-key=pages")
            .arg("--json-key=objects")
            .arg(input)
            .output()
            .context("读取 qpdf 内容流索引失败")?;
        if !super::qpdf::status_is_success(&output.status) {
            anyhow::bail!(
                "qpdf 内容流索引失败：{}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let json: Value =
            serde_json::from_slice(&output.stdout).context("解析 qpdf 内容流索引失败")?;
        let objects = json
            .get("objects")
            .and_then(Value::as_object)
            .cloned()
            .context("qpdf 内容流索引缺少 objects")?;
        let raw_pages = json
            .get("pages")
            .and_then(Value::as_array)
            .context("qpdf 内容流索引缺少 pages")?;
        let mut index = Self {
            objects,
            pages: Vec::with_capacity(raw_pages.len()),
        };
        index.pages = raw_pages
            .iter()
            .enumerate()
            .filter_map(|(page_index, page)| {
                let page_ref = page.get("object").and_then(Value::as_str)?;
                let resources = index.inherited_dictionary(page_ref, "/Resources");
                Some(QpdfIndexedPage {
                    number: page_index as u32 + 1,
                    contents: page
                        .get("contents")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect(),
                    fonts: index.fonts_from_resources(resources),
                    xobjects: index.xobjects_from_resources(resources),
                    properties: index.properties_from_resources(resources),
                    page_box: index
                        .inherited_value(page_ref, "/CropBox")
                        .and_then(|value| index.parse_box_value(value))
                        .or_else(|| {
                            index
                                .inherited_value(page_ref, "/MediaBox")
                                .and_then(|value| index.parse_box_value(value))
                        }),
                })
            })
            .collect();
        Ok(index)
    }

    pub(crate) fn pages(&self) -> &[QpdfIndexedPage] {
        &self.pages
    }

    pub(crate) fn is_form(&self, object_ref: &str) -> bool {
        self.object_dictionary(object_ref)
            .and_then(|dict| dict.get("/Subtype"))
            .and_then(Value::as_str)
            == Some("/Form")
    }

    pub(crate) fn form_xobjects(&self, object_ref: &str) -> BTreeMap<String, String> {
        let resources = self
            .object_dictionary(object_ref)
            .and_then(|dict| dict.get("/Resources"))
            .and_then(|value| self.resolve_dictionary(value));
        self.xobjects_from_resources(resources)
    }

    pub(crate) fn form_fonts(&self, object_ref: &str) -> BTreeMap<String, String> {
        let resources = self
            .object_dictionary(object_ref)
            .and_then(|dict| dict.get("/Resources"))
            .and_then(|value| self.resolve_dictionary(value));
        self.fonts_from_resources(resources)
    }

    pub(crate) fn font_references(&self) -> BTreeSet<String> {
        let mut references = BTreeSet::new();
        for page in &self.pages {
            references.extend(page.fonts.values().cloned());
        }
        for (reference, object) in &self.objects {
            if object
                .as_object()
                .and_then(|dict| dict.get("/Subtype"))
                .and_then(Value::as_str)
                == Some("/Form")
            {
                references.extend(self.form_fonts(reference).into_values());
            }
        }
        references
    }

    pub(crate) fn to_unicode_stream_references(
        &self,
        font_refs: &BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        font_refs
            .iter()
            .filter_map(|font_ref| {
                let cmap_ref = self
                    .object_dictionary(font_ref)?
                    .get("/ToUnicode")
                    .and_then(Value::as_str)?;
                Some((font_ref.clone(), cmap_ref.to_string()))
            })
            .collect()
    }

    /// Return embedded font streams that can safely provide a CID -> GID map.
    ///
    /// This is intentionally limited to Type0 fonts using Identity-H/V and a
    /// CIDFontType2 descendant with an identity CIDToGIDMap.  Under those
    /// conditions the CID in the content stream is the TrueType GID, so the
    /// embedded font cmap can be used as a deterministic fallback when the
    /// PDF omitted ToUnicode.
    pub(crate) fn identity_cid_font_stream_references(
        &self,
        font_refs: &BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        font_refs
            .iter()
            .filter_map(|font_ref| {
                let font = self.object_dictionary(font_ref)?;
                if font.get("/Subtype").and_then(Value::as_str) != Some("/Type0") {
                    return None;
                }
                if !matches!(
                    font.get("/Encoding").and_then(Value::as_str),
                    Some("/Identity-H") | Some("/Identity-V")
                ) {
                    return None;
                }
                if font.get("/ToUnicode").is_some() {
                    return None;
                }
                let descendants = self.resolve_array(font.get("/DescendantFonts")?)?;
                let descendant_ref = descendants.first()?.as_str()?;
                let descendant = self.object_dictionary(descendant_ref)?;
                if descendant.get("/Subtype").and_then(Value::as_str) != Some("/CIDFontType2") {
                    return None;
                }
                if let Some(cid_to_gid_map) = descendant.get("/CIDToGIDMap") {
                    if cid_to_gid_map.as_str() != Some("/Identity") {
                        return None;
                    }
                }
                let descriptor = self.resolve_dictionary(descendant.get("/FontDescriptor")?)?;
                let font_file = descriptor
                    .get("/FontFile2")
                    .or_else(|| descriptor.get("/FontFile3"))
                    .and_then(Value::as_str)?;
                Some((font_ref.clone(), font_file.to_string()))
            })
            .collect()
    }

    /// Return the PDF-defined Encoding and CIDSystemInfo/Ordering for Type0
    /// fonts.  These values are metadata for selecting a predefined CMap; they
    /// are never treated as text by themselves.
    pub(crate) fn predefined_font_info(
        &self,
        font_refs: &BTreeSet<String>,
    ) -> BTreeMap<String, PredefinedFontInfo> {
        font_refs
            .iter()
            .filter_map(|font_ref| {
                let font = self.object_dictionary(font_ref)?;
                if font.get("/Subtype").and_then(Value::as_str) != Some("/Type0") {
                    return None;
                }
                let descendants = self.resolve_array(font.get("/DescendantFonts")?)?;
                let descendant_ref = descendants.first()?.as_str()?;
                let descendant = self.object_dictionary(descendant_ref)?;
                let system_info = self.resolve_dictionary(descendant.get("/CIDSystemInfo")?);
                let ordering = system_info
                    .and_then(|dict| dict.get("/Ordering"))
                    .and_then(qpdf_string)
                    .map(|value| value.trim_start_matches('/').to_string());
                let encoding = font
                    .get("/Encoding")
                    .and_then(Value::as_str)
                    .map(|value| value.trim_start_matches('/').to_string());
                if encoding.is_none() && ordering.is_none() {
                    return None;
                }
                Some((font_ref.clone(), PredefinedFontInfo { encoding, ordering }))
            })
            .collect()
    }

    pub(crate) fn form_properties(&self, object_ref: &str) -> Dictionary {
        let resources = self
            .object_dictionary(object_ref)
            .and_then(|dict| dict.get("/Resources"))
            .and_then(|value| self.resolve_dictionary(value));
        self.properties_from_resources(resources)
    }

    /// 读取 Form XObject 的 /Matrix（六位矩阵 [a b c d e f]），缺失时返回 None
    /// （调用方按 PDF 规范视为单位矩阵）。约定与 compress.rs 的 CTM 跟踪一致。
    pub(crate) fn form_matrix(&self, object_ref: &str) -> Option<[f64; 6]> {
        let value = self.object_dictionary(object_ref)?.get("/Matrix")?;
        let values = match value {
            Value::Array(values) => values,
            Value::String(reference) => self.objects.get(reference)?.as_array()?,
            _ => return None,
        };
        if values.len() != 6 {
            return None;
        }
        let mut matrix = [0.0f64; 6];
        for (i, value) in values.iter().enumerate() {
            matrix[i] = f64::from(json_number(value)?);
        }
        Some(matrix)
    }

    fn parse_box_value(&self, value: &Value) -> Option<QpdfBox> {
        match value {
            Value::String(reference) if reference.ends_with(" R") => {
                self.objects.get(reference).and_then(parse_box)
            }
            _ => parse_box(value),
        }
    }

    fn object_dictionary(&self, object_ref: &str) -> Option<&Map<String, Value>> {
        self.objects.get(object_ref)?.as_object()
    }

    fn resolve_dictionary<'a>(&'a self, value: &'a Value) -> Option<&'a Map<String, Value>> {
        match value {
            Value::Object(dict) => Some(dict),
            Value::String(reference) => self.object_dictionary(reference),
            _ => None,
        }
    }

    fn resolve_array<'a>(&'a self, value: &'a Value) -> Option<&'a Vec<Value>> {
        match value {
            Value::Array(values) => Some(values),
            Value::String(reference) => self.objects.get(reference)?.as_array(),
            _ => None,
        }
    }

    fn inherited_dictionary(&self, object_ref: &str, key: &str) -> Option<&Map<String, Value>> {
        self.inherited_value(object_ref, key)
            .and_then(|value| self.resolve_dictionary(value))
    }

    fn inherited_value<'a>(&'a self, object_ref: &str, key: &str) -> Option<&'a Value> {
        let mut current = Some(object_ref);
        let mut remaining = self.objects.len().saturating_add(1);
        while let Some(reference) = current {
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            let dictionary = self.object_dictionary(reference)?;
            if let Some(value) = dictionary.get(key) {
                return Some(value);
            }
            current = dictionary.get("/Parent").and_then(Value::as_str);
        }
        None
    }

    fn xobjects_from_resources(
        &self,
        resources: Option<&Map<String, Value>>,
    ) -> BTreeMap<String, String> {
        let Some(xobjects) = resources
            .and_then(|resources| resources.get("/XObject"))
            .and_then(|value| self.resolve_dictionary(value))
        else {
            return BTreeMap::new();
        };
        xobjects
            .iter()
            .filter_map(|(name, value)| {
                value.as_str().map(|reference| {
                    (
                        name.trim_start_matches('/').to_string(),
                        reference.to_string(),
                    )
                })
            })
            .collect()
    }

    fn fonts_from_resources(
        &self,
        resources: Option<&Map<String, Value>>,
    ) -> BTreeMap<String, String> {
        let Some(fonts) = resources
            .and_then(|resources| resources.get("/Font"))
            .and_then(|value| self.resolve_dictionary(value))
        else {
            return BTreeMap::new();
        };
        fonts
            .iter()
            .filter_map(|(name, value)| {
                value.as_str().map(|reference| {
                    (
                        name.trim_start_matches('/').to_string(),
                        reference.to_string(),
                    )
                })
            })
            .collect()
    }

    fn properties_from_resources(&self, resources: Option<&Map<String, Value>>) -> Dictionary {
        let Some(properties) = resources
            .and_then(|resources| resources.get("/Properties"))
            .and_then(|value| self.resolve_dictionary(value))
        else {
            return Dictionary::new();
        };
        let mut output = Dictionary::new();
        for (name, value) in properties {
            if let Some(object) = json_to_lopdf_object(value, &self.objects) {
                output.set(name.trim_start_matches('/').as_bytes().to_vec(), object);
            }
        }
        output
    }
}

fn qpdf_string(value: &Value) -> Option<&str> {
    value
        .as_str()
        .map(|value| value.strip_prefix("u:").unwrap_or(value))
}

fn json_to_lopdf_object(value: &Value, objects: &Map<String, Value>) -> Option<Object> {
    match value {
        Value::Null => Some(Object::Null),
        Value::Bool(value) => Some(Object::Boolean(*value)),
        Value::Number(value) => value
            .as_i64()
            .map(Object::Integer)
            .or_else(|| value.as_f64().map(|value| Object::Real(value as f32))),
        Value::String(value) if value.starts_with('/') => {
            Some(Object::Name(value.as_bytes()[1..].to_vec()))
        }
        Value::String(value) if value.ends_with(" R") => objects
            .get(value)
            .and_then(|value| json_to_lopdf_object(value, objects)),
        Value::String(value) => Some(Object::String(
            value
                .strip_prefix("u:")
                .unwrap_or(value)
                .as_bytes()
                .to_vec(),
            StringFormat::Literal,
        )),
        Value::Array(values) => Some(Object::Array(
            values
                .iter()
                .filter_map(|value| json_to_lopdf_object(value, objects))
                .collect(),
        )),
        Value::Object(values) => {
            let mut dictionary = Dictionary::new();
            for (name, value) in values {
                if let Some(object) = json_to_lopdf_object(value, objects) {
                    dictionary.set(name.trim_start_matches('/').as_bytes().to_vec(), object);
                }
            }
            Some(Object::Dictionary(dictionary))
        }
    }
}

fn parse_box(value: &Value) -> Option<QpdfBox> {
    let values = value.as_array()?;
    if values.len() < 4 {
        return None;
    }
    Some(QpdfBox {
        x0: json_number(&values[0])?,
        y0: json_number(&values[1])?,
        x1: json_number(&values[2])?,
        y1: json_number(&values[3])?,
    })
}

fn json_number(value: &Value) -> Option<f32> {
    value
        .as_f64()
        .map(|number| number as f32)
        .or_else(|| value.as_i64().map(|number| number as f32))
}

pub(crate) fn object_selector(reference: &str) -> Option<String> {
    let mut parts = reference.split_whitespace();
    let object = parts.next()?.parse::<u32>().ok()?;
    let generation = parts.next()?.parse::<u16>().ok()?;
    Some(format!("{object},{generation}"))
}

pub(crate) fn load_editable_streams(
    input: &Path,
    references: &[String],
    operation_name: &str,
) -> Result<BTreeMap<String, QpdfEditableStream>> {
    if references.is_empty() {
        return Ok(BTreeMap::new());
    }
    let qpdf_bin = crate::external::QpdfTool.binary_path()?;
    let mut result = BTreeMap::new();
    for chunk in references.chunks(128) {
        let mut command = crate::external::hidden_command(&qpdf_bin);
        command
            .arg("--json-output=2")
            .arg("--json-stream-data=inline")
            .arg("--decode-level=all");
        for reference in chunk {
            let selector = object_selector(reference)
                .with_context(|| format!("无法解析 qpdf 对象引用 {reference}"))?;
            command.arg(format!("--json-object={selector}"));
        }
        let output = command
            .arg(input)
            .output()
            .with_context(|| format!("读取待{operation_name}的 PDF 内容流失败"))?;
        if !super::qpdf::status_is_success(&output.status) {
            anyhow::bail!(
                "读取待{}的 PDF 内容流失败：{}",
                operation_name,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let json: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("解析待{operation_name}的 PDF 内容流失败"))?;
        let entries = json
            .get("qpdf")
            .and_then(Value::as_array)
            .context("qpdf 内容流结果缺少对象表")?;
        for objects in entries.iter().skip(1).filter_map(Value::as_object) {
            for (key, value) in objects {
                let Some(reference) = key.strip_prefix("obj:") else {
                    continue;
                };
                let Some(stream) = value.get("stream") else {
                    continue;
                };
                let dictionary = stream
                    .get("dict")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                let data = stream
                    .get("data")
                    .and_then(Value::as_str)
                    .context("qpdf 内容流缺少内联数据")?;
                let decoded = general_purpose::STANDARD
                    .decode(data)
                    .context("解码 qpdf 内容流失败")?;
                let operations = Content::decode(&decoded)
                    .context("解析 qpdf PDF 内容流失败")?
                    .operations;
                result.insert(
                    reference.to_string(),
                    QpdfEditableStream {
                        dictionary,
                        operations,
                    },
                );
            }
        }
    }
    for reference in references {
        if !result.contains_key(reference) {
            anyhow::bail!("qpdf 未返回 PDF 内容流 {reference}");
        }
    }
    Ok(result)
}

pub(crate) fn load_raw_streams(
    input: &Path,
    references: &[String],
    operation_name: &str,
) -> Result<BTreeMap<String, Vec<u8>>> {
    if references.is_empty() {
        return Ok(BTreeMap::new());
    }
    let qpdf_bin = crate::external::QpdfTool.binary_path()?;
    let mut result = BTreeMap::new();
    for chunk in references.chunks(128) {
        let mut command = crate::external::hidden_command(&qpdf_bin);
        command
            .arg("--json-output=2")
            .arg("--json-stream-data=inline")
            .arg("--decode-level=all");
        for reference in chunk {
            let selector = object_selector(reference)
                .with_context(|| format!("无法解析 qpdf 对象引用 {reference}"))?;
            command.arg(format!("--json-object={selector}"));
        }
        let output = command
            .arg(input)
            .output()
            .with_context(|| format!("读取{operation_name}失败"))?;
        if !super::qpdf::status_is_success(&output.status) {
            anyhow::bail!(
                "读取{}失败：{}",
                operation_name,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let json: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("解析{}结果失败", operation_name))?;
        let entries = json
            .get("qpdf")
            .and_then(Value::as_array)
            .context("qpdf 原始流结果缺少对象表")?;
        for objects in entries.iter().skip(1).filter_map(Value::as_object) {
            for (key, value) in objects {
                let Some(reference) = key.strip_prefix("obj:") else {
                    continue;
                };
                let Some(data) = value
                    .get("stream")
                    .and_then(|stream| stream.get("data"))
                    .and_then(Value::as_str)
                else {
                    continue;
                };
                result.insert(
                    reference.to_string(),
                    general_purpose::STANDARD
                        .decode(data)
                        .with_context(|| format!("解码{}失败", operation_name))?,
                );
            }
        }
    }
    for reference in references {
        if !result.contains_key(reference) {
            anyhow::bail!("qpdf 未返回{} {reference}", operation_name);
        }
    }
    Ok(result)
}

pub(crate) fn update_streams(
    input: &Path,
    output: &Path,
    streams: &BTreeMap<String, QpdfEditableStream>,
    operation_name: &str,
) -> Result<()> {
    let mut objects = Map::new();
    for (reference, stream) in streams {
        let data = Content {
            operations: stream.operations.clone(),
        }
        .encode()
        .with_context(|| format!("编码{operation_name}后的 PDF 内容流失败"))?;
        objects.insert(
            format!("obj:{reference}"),
            serde_json::json!({
                "stream": {
                    "dict": stream.dictionary,
                    "data": general_purpose::STANDARD.encode(data),
                }
            }),
        );
    }
    let update = serde_json::json!({
        "qpdf": [
            { "jsonversion": 2 },
            Value::Object(objects),
        ]
    });
    let update_path = temp_named_path("docsy_qpdf_stream_update", "json");
    std::fs::write(&update_path, serde_json::to_vec(&update)?)
        .with_context(|| format!("写入{operation_name}的 qpdf 更新文件失败"))?;
    let qpdf_bin = crate::external::QpdfTool.binary_path()?;
    let command_result = crate::external::hidden_command(qpdf_bin)
        .arg(input)
        .arg(format!("--update-from-json={}", update_path.display()))
        .arg(output)
        .output()
        .with_context(|| format!("qpdf 写回{operation_name}失败"));
    let command_result = command_result?;
    if !super::qpdf::status_is_success(&command_result.status) {
        let _ = std::fs::remove_file(output);
        let detail = String::from_utf8_lossy(&command_result.stderr)
            .trim()
            .to_string();
        let _ = std::fs::remove_file(&update_path);
        anyhow::bail!("qpdf 写回{}失败：{}", operation_name, detail);
    }
    let _ = std::fs::remove_file(&update_path);
    Ok(())
}
