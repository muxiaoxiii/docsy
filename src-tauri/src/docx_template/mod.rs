//! Docsy Word template model and package lifecycle.
//!
//! All OOXML reads and writes are implemented by the quick-xml modules below.
//! This module intentionally contains no XML regular-expression fallback.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

pub mod batch;
pub mod engine;
pub mod index;
pub mod ooxml;
pub mod package;
pub mod render;
pub mod save;
pub mod scan;

pub(super) const MANIFEST_PATH: &str = "manifest.json";
pub(super) const MAX_ZIP_ENTRIES: usize = 4096;
pub(super) const MAX_DOCX_BYTES: u64 = 512 * 1024 * 1024;
pub(super) const MAX_DOCSYTPL_BYTES: u64 = 512 * 1024 * 1024;
pub(super) const MAX_XML_ENTRY_BYTES: u64 = 128 * 1024 * 1024;
pub(super) const MAX_BINARY_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
pub(super) const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const MAX_PACKAGE_UNCOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
pub(super) const MAX_PACKAGE_XML_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInspection {
    pub input: String,
    pub converted_path: Option<String>,
    pub document_text: String,
    pub document_runs: Vec<TemplateTextRun>,
    pub marks: Vec<TemplateMark>,
    pub summary: InspectionSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionSummary {
    pub mark_count: usize,
    pub checkbox_like_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMark {
    pub id: String,
    pub part: String,
    pub run_index: usize,
    pub text: String,
    pub context: String,
    pub checkbox_like: bool,
    #[serde(default)]
    pub option_label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateTextRun {
    pub id: String,
    pub part: String,
    pub run_index: usize,
    pub paragraph_index: usize,
    pub text: String,
    pub paragraph_text: String,
    pub checkbox_like: bool,
    #[serde(default)]
    pub option_label: String,
    pub highlighted: bool,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateManifest {
    pub format_version: u32,
    pub template: TemplateMeta,
    pub fields: Vec<TemplateField>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMeta {
    pub id: String,
    pub name: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateField {
    pub id: String,
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub semantic_key: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub marks: Vec<String>,
    #[serde(default)]
    pub mark_refs: Vec<TemplateMarkRef>,
    #[serde(default)]
    pub options: Vec<TemplateOption>,
    #[serde(default)]
    pub optional_rule: Option<OptionalFieldRule>,
    #[serde(default)]
    pub reference: Option<TemplateFieldReference>,
    /// When true, every document position of this field gets the same value
    /// (a field marked in multiple separate places); when false, only the
    /// first position is filled and later ones stay empty (a single position
    /// split across multiple runs with different formatting).
    #[serde(default)]
    pub fill_all_positions: bool,
    /// Date rendering format for date fields: iso (2026-08-05), cn (2026年8月5日),
    /// cn_full (二零二六年八月五日), en_long (August 5, 2026), en_short (Aug. 5, 2026),
    /// en_dmy (5 August 2026), en_ordinal (2026 August 5th), blank (留空: 年 月 日).
    #[serde(default)]
    pub date_format: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateFieldReference {
    #[serde(default)]
    pub source_mode: String,
    #[serde(default)]
    pub source_field: String,
    #[serde(default)]
    pub source_semantic_key: String,
    #[serde(default)]
    pub source_index: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMarkRef {
    pub mark_id: String,
    #[serde(default)]
    pub start: Option<usize>,
    #[serde(default)]
    pub end: Option<usize>,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub optional_rule: Option<OptionalFieldRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OptionalFieldRule {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub remove_empty_prefix: String,
    #[serde(default)]
    pub remove_empty_suffix: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub marker_mark_id: String,
    #[serde(default)]
    pub marker_tag: String,
    #[serde(default)]
    pub checked_text: String,
    #[serde(default)]
    pub unchecked_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTemplateArgs {
    pub source_docx: String,
    pub output_path: String,
    pub template_name: String,
    pub fields: Vec<TemplateField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTemplateResult {
    pub output_path: String,
    pub manifest: TemplateManifest,
}

/// Document content extracted from a `.docsytpl` package for field-row
/// reconstruction when editing a library template.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocsytplContent {
    pub document_text: String,
    pub document_runs: Vec<TemplateTextRun>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateLibraryItem {
    pub path: String,
    pub name: String,
    pub field_count: usize,
    pub updated: String,
    pub trashed: bool,
    pub manifest: TemplateManifest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDeleteArgs {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateRestoreArgs {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePermanentDeleteArgs {
    pub path: String,
    #[serde(default)]
    pub migrate_to_common: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTemplateArgs {
    pub template_path: String,
    pub output_path: String,
    #[serde(default)]
    pub values: HashMap<String, Value>,
    #[serde(default)]
    pub structure_overrides: HashMap<String, StructureOverride>,
    /// Separator between items of multi-value (party_list) fields.
    #[serde(default = "default_item_separator")]
    pub item_separator: String,
}

fn default_item_separator() -> String {
    "、".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct StructureOverride {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
}

pub(super) fn normalize_manifest_options(manifest: &mut TemplateManifest) {
    for field in &mut manifest.fields {
        if matches!(
            field.field_type.as_str(),
            "checkbox" | "radio_group" | "checkbox_group"
        ) {
            for option in &mut field.options {
                option.marker_tag = format!("{}.option.{}", field.id, option.id);
                if option.checked_text.is_empty() {
                    option.checked_text = "☑".into();
                }
                if option.unchecked_text.is_empty() {
                    option.unchecked_text = "☐".into();
                }
            }
        } else {
            let multi_ref = field.mark_refs.len() > 1;
            for (index, mark_ref) in field.mark_refs.iter_mut().enumerate() {
                if mark_ref.tag.is_empty() {
                    mark_ref.tag = if multi_ref || mark_ref.optional_rule.is_some() {
                        format!("{}.ref.{}", field.id, index + 1)
                    } else {
                        field.id.clone()
                    };
                }
            }
        }
    }
}

pub(super) fn sanitize_manifest_private_labels(
    manifest: &mut TemplateManifest,
    marks: &[TemplateMark],
) {
    let marked: HashSet<&str> = marks
        .iter()
        .map(|mark| mark.text.trim())
        .filter(|text| !text.is_empty())
        .collect();
    for field in &mut manifest.fields {
        let fallback = if field.name.trim().is_empty() {
            field.id.as_str()
        } else {
            field.name.as_str()
        };
        if generated_field_name(&field.name) || marked.contains(field.label.trim()) {
            field.label = fallback.to_string();
        }
        if marked.contains(field.semantic_key.trim()) {
            field.semantic_key = fallback.to_string();
        }
    }
}

fn generated_field_name(value: &str) -> bool {
    value
        .strip_prefix("字段")
        .is_some_and(|tail| !tail.is_empty() && tail.chars().all(|ch| ch.is_ascii_digit()))
        || value
            .strip_prefix("field_")
            .is_some_and(|tail| !tail.is_empty() && tail.chars().all(|ch| ch.is_ascii_digit()))
}

pub fn template_library_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|path| path.join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("Docsy/templates")
}

fn template_trash_dir() -> PathBuf {
    template_library_dir().join("_trash")
}

pub fn safe_template_file_name(name: &str) -> String {
    let name = name
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();
    if name.is_empty() {
        "未命名模板".into()
    } else {
        name
    }
}

pub fn list_template_library() -> Result<Vec<TemplateLibraryItem>> {
    list_template_library_items(&template_library_dir(), false)
}

pub fn list_template_trash() -> Result<Vec<TemplateLibraryItem>> {
    list_template_library_items(&template_trash_dir(), true)
}

fn list_template_library_items(dir: &Path, trashed: bool) -> Result<Vec<TemplateLibraryItem>> {
    std::fs::create_dir_all(dir)?;
    let mut items = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("docsytpl"))
        {
            continue;
        }
        let Ok(manifest) = read_template_manifest(&path) else {
            continue;
        };
        items.push(TemplateLibraryItem {
            path: path.display().to_string(),
            name: manifest.template.name.clone(),
            field_count: manifest.fields.len(),
            updated: manifest.template.updated.clone(),
            trashed,
            manifest,
        });
    }
    items.sort_by(|left, right| {
        right
            .updated
            .cmp(&left.updated)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(items)
}

pub fn move_template_to_trash(args: TemplateDeleteArgs) -> Result<String> {
    let source = PathBuf::from(args.path);
    let manifest = read_template_manifest(&source)?;
    let target = move_template_file(&source, &template_trash_dir())?;
    crate::template_history::mark_template_trashed(&manifest.template.id, true)?;
    Ok(target.display().to_string())
}

pub fn restore_template_from_trash(args: TemplateRestoreArgs) -> Result<String> {
    let source = PathBuf::from(args.path);
    let manifest = read_template_manifest(&source)?;
    let target = move_template_file(&source, &template_library_dir())?;
    crate::template_history::mark_template_trashed(&manifest.template.id, false)?;
    // The restored path may differ (unique suffix); keep history records pointing
    // at the live file so they aren't re-trashed on the next refresh.
    crate::template_history::update_template_path(
        &manifest.template.id,
        &target.display().to_string(),
    )?;
    Ok(target.display().to_string())
}

fn move_template_file(source: &Path, destination_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(destination_dir)?;
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("template.docsytpl");
    let target = unique_docx_output_path(&destination_dir.join(file_name))?;
    std::fs::rename(source, &target)
        .or_else(|_| -> Result<(), std::io::Error> {
            std::fs::copy(source, &target)?;
            std::fs::remove_file(source)
        })
        .with_context(|| format!("移动模板失败: {}", source.display()))?;
    Ok(target)
}

pub fn permanently_delete_template(args: TemplatePermanentDeleteArgs) -> Result<()> {
    let source = PathBuf::from(args.path);
    let manifest = read_template_manifest(&source)?;

    // Delete or migrate the associated records before removing the source
    // package. A failed history operation must not leave a template file whose
    // invisible data can still appear in later suggestions.
    if args.migrate_to_common {
        crate::template_history::migrate_template_data_to_common(&manifest.template.id)?;
    } else {
        crate::template_history::delete_template_data(&manifest.template.id)?;
    }
    if source.exists() {
        std::fs::remove_file(&source)
            .with_context(|| format!("彻底删除模板文件失败: {}", source.display()))?;
    }
    Ok(())
}

pub fn inspect_template_package(path: &str) -> Result<TemplateManifest> {
    read_template_manifest(Path::new(path))
}

/// Update a field's reference (data source) and/or date format in the template
/// manifest and rewrite the docsytpl package (word content untouched). Used to
/// persist reference-source / date-format changes made on the fill page.
pub fn update_template_field_settings(
    template_path: &str,
    field_id: &str,
    reference: Option<TemplateFieldReference>,
    date_format: Option<String>,
) -> Result<()> {
    let path = Path::new(template_path);
    let (mut manifest, pkg) = package::read_docsytpl_package(path)?;
    let field = manifest
        .fields
        .iter_mut()
        .find(|f| f.id == field_id)
        .ok_or_else(|| anyhow::anyhow!("模板中不存在字段: {}", field_id))?;
    if let Some(reference) = reference {
        field.reference = Some(reference);
    }
    if let Some(date_format) = date_format {
        field.date_format = date_format;
    }
    package::write_docsytpl_package(path, &manifest, &pkg)?;
    Ok(())
}

fn read_template_manifest(path: &Path) -> Result<TemplateManifest> {
    let (manifest, _) = package::read_docsytpl_package(path)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub(super) fn validate_manifest(manifest: &TemplateManifest) -> Result<()> {
    let mut field_ids = HashSet::new();
    let mut tags = HashSet::new();
    for field in &manifest.fields {
        if field.id.trim().is_empty() || !field_ids.insert(field.id.trim()) {
            anyhow::bail!("模板包含空白或重复的字段 ID");
        }
        if !matches!(
            field.field_type.as_str(),
            "text"
                | "date"
                | "select"
                | "party_list"
                | "reference"
                | "checkbox"
                | "radio_group"
                | "checkbox_group"
                | "delete_text"
        ) {
            anyhow::bail!(
                "模板字段类型不兼容：字段“{}”使用了未知类型“{}”",
                if field.label.is_empty() {
                    &field.name
                } else {
                    &field.label
                },
                field.field_type
            );
        }
        for mark_ref in &field.mark_refs {
            if mark_ref.mark_id.trim().is_empty() {
                anyhow::bail!("字段“{}”缺少标记坐标", field.label);
            }
            if !mark_ref.tag.trim().is_empty() && !tags.insert(mark_ref.tag.trim()) {
                anyhow::bail!("模板包含重复的字段标记“{}”", mark_ref.tag);
            }
            if matches!((mark_ref.start, mark_ref.end), (Some(start), Some(end)) if start >= end) {
                anyhow::bail!("字段“{}”包含无效文本范围", field.label);
            }
        }
        for option in &field.options {
            // select 类型的 options 不需要 marker_mark_id，只有勾选类型才需要
            if matches!(
                field.field_type.as_str(),
                "checkbox" | "radio_group" | "checkbox_group"
            ) {
                if option.id.trim().is_empty() || option.marker_mark_id.trim().is_empty() {
                    anyhow::bail!("勾选字段“{}”包含不完整选项", field.label);
                }
            } else if option.id.trim().is_empty() {
                anyhow::bail!("字段“{}”包含不完整选项", field.label);
            }
            if !option.marker_tag.trim().is_empty() && !tags.insert(option.marker_tag.trim()) {
                anyhow::bail!("模板包含重复的字段标记“{}”", option.marker_tag);
            }
        }
    }
    Ok(())
}

pub(super) fn read_file_with_limit(path: &Path, limit: u64, label: &str) -> Result<Vec<u8>> {
    let metadata =
        std::fs::metadata(path).with_context(|| format!("读取{label}失败: {}", path.display()))?;
    if metadata.len() > limit {
        anyhow::bail!(
            "{}过大，无法安全读取：{} MB，限制 {} MB",
            label,
            metadata.len() / 1024 / 1024,
            limit / 1024 / 1024
        );
    }
    // Bounded read (TOCTOU-safe): never read more than limit+1 bytes even if
    // the file grows between metadata and read.
    let file = std::fs::File::open(path)
        .with_context(|| format!("读取{label}失败: {}", path.display()))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut reader = std::io::BufReader::new(file).take(limit + 1);
    std::io::Read::read_to_end(&mut reader, &mut bytes)
        .with_context(|| format!("读取{label}失败: {}", path.display()))?;
    if bytes.len() as u64 > limit {
        anyhow::bail!("{}过大，无法安全读取", label);
    }
    Ok(bytes)
}

pub(super) fn read_vec_with_limit<R: Read>(
    mut reader: R,
    declared_size: u64,
    limit: u64,
    label: &str,
) -> Result<Vec<u8>> {
    if declared_size > limit {
        anyhow::bail!("{}解压后超过安全限制", label);
    }
    let mut bytes = Vec::with_capacity(declared_size.min(limit) as usize);
    reader
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("读取{label}失败"))?;
    if bytes.len() as u64 > limit {
        anyhow::bail!("{}解压后超过安全限制", label);
    }
    Ok(bytes)
}

pub(super) fn is_word_xml_part(name: &str) -> bool {
    name == "word/document.xml"
        || (name.starts_with("word/header") && name.ends_with(".xml"))
        || (name.starts_with("word/footer") && name.ends_with(".xml"))
}

pub(super) fn fnv1a_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

pub(super) fn unique_docx_output_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for suffix in 1..=9_999 {
        let file_name = if extension.is_empty() {
            format!("{stem}-{suffix}")
        } else {
            format!("{stem}-{suffix}.{extension}")
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("无法生成不覆盖现有文件的输出路径")
}
