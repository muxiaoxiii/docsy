use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use zip::write::FileOptions;

use office_oxide::Document as OoDocument;

static PARAGRAPH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<w:p\b[^>]*>.*?</w:p>"#).unwrap());
static TABLE_ROW_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<w:tr\b[^>]*>.*?</w:tr>"#).unwrap());
static RUN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<w:r\b[^>]*>.*?</w:r>"#).unwrap());
static TEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<w:t(?:\s[^>]*)?>(.*?)</w:t>"#).unwrap());
static TEXT_NODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)(<w:t(?:\s[^>]*)?>)(.*?)(</w:t>)"#).unwrap());
static HIGHLIGHT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<w:highlight\b[^>]*\bw:val\s*=\s*["']yellow["'][^>]*/>|<w:highlight\b[^>]*\bw:val\s*=\s*["']yellow["'][^>]*>.*?</w:highlight>"#,
    )
    .unwrap()
});
static RPR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<w:rPr\b[^>]*>.*?</w:rPr>"#).unwrap());
static SDT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<w:sdt\b[^>]*>.*?</w:sdt>"#).unwrap());

const MANIFEST_PATH: &str = "manifest.json";
const TEMPLATE_DOCX_PATH: &str = "template.docx";
const CHECKBOX_MARKER_CHARS: &[char] = &['□', '☐', '☑', '☒', '✓', '√', '✔', '✗', '○', '●'];
const CHECKBOX_MARKER_TEXTS: &[&str] = &["(√)", "（√）", "( )", "（ ）"];
const MAX_ZIP_ENTRIES: usize = 4096;
const MAX_DOCX_BYTES: u64 = 512 * 1024 * 1024;
const MAX_DOCSYTPL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_XML_ENTRY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_BINARY_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;

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
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct StructureOverride {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
}

#[derive(Debug, Clone)]
struct MarkTagSpec {
    tag: String,
    placeholder: String,
    start: Option<usize>,
    end: Option<usize>,
    delete: bool,
}

pub fn inspect_docx_template(path: &str) -> Result<TemplateInspection> {
    let input_path = Path::new(path);
    let mut _converted_guard = None;
    let (docx_path, converted_path) = if is_legacy_doc(input_path) {
        let converted = convert_doc_to_docx(input_path).with_context(|| {
            format!(
                "暂不支持旧版 .doc 格式，自动转换也失败。请先用 Word 或 WPS 将文件另存为 .docx 格式，再导入 Docsy。\n文件：{}",
                input_path.display()
            )
        })?;
        let display = converted.display().to_string();
        _converted_guard = Some(TempPathGuard::new(converted.clone()));
        (converted, Some(display))
    } else {
        (input_path.to_path_buf(), None)
    };
    let document_runs = scan_docx_text_runs(&docx_path)?;
    let marks = highlighted_runs_as_marks(&document_runs);
    let document_text = extract_docx_plain_text(&docx_path)?;
    let checkbox_like_count = marks.iter().filter(|mark| mark.checkbox_like).count();
    Ok(TemplateInspection {
        input: path.to_string(),
        converted_path,
        document_text,
        document_runs,
        summary: InspectionSummary {
            mark_count: marks.len(),
            checkbox_like_count,
        },
        marks,
    })
}

fn is_legacy_doc(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("doc"))
        .unwrap_or(false)
}

fn convert_doc_to_docx(path: &Path) -> Result<PathBuf> {
    let doc = OoDocument::open(&path.display().to_string())
        .with_context(|| format!("无法读取旧版 .doc 文件: {}", path.display()))?;
    let output = unique_docx_output_path(
        &std::env::temp_dir()
            .join(format!(
                "docsy-template-{:016x}",
                fnv1a_hash(&path.display().to_string())
            ))
            .with_extension("docx"),
    )?;
    doc.save_as(&output.display().to_string())
        .with_context(|| format!("转换 .doc → .docx 失败: {}", path.display()))?;
    Ok(output)
}

struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn save_docx_template(args: SaveTemplateArgs) -> Result<SaveTemplateResult> {
    let mut _converted_guard = None;
    let source = if is_legacy_doc(Path::new(&args.source_docx)) {
        let converted = convert_doc_to_docx(Path::new(&args.source_docx)).with_context(|| {
            format!(
                "旧版 .doc 转换失败，无法保存模板。请先用 Word 或 WPS 另存为 .docx。\n文件：{}",
                args.source_docx
            )
        })?;
        _converted_guard = Some(TempPathGuard::new(converted.clone()));
        converted
    } else {
        PathBuf::from(&args.source_docx)
    };
    let output = unique_docx_output_path(Path::new(&args.output_path))?;
    let mut manifest = TemplateManifest {
        format_version: 1,
        template: TemplateMeta {
            id: format!("tpl_{:016x}", fnv1a_hash(&output.display().to_string())),
            name: args.template_name,
            created: chrono::Utc::now().to_rfc3339(),
            updated: chrono::Utc::now().to_rfc3339(),
        },
        fields: args.fields,
    };
    let text_runs = scan_docx_text_runs(&source)?;
    let marks = text_runs_as_marks(&text_runs);
    sanitize_manifest_private_labels(&mut manifest, &marks);
    normalize_manifest_options(&mut manifest);
    let mark_specs = build_mark_tag_specs(&manifest.fields);
    validate_mark_specs(&marks, &mark_specs)?;
    validate_template_source_docx(&source, &marks, &mark_specs)?;

    let template_docx = rewrite_docx(&source, |name, xml| {
        if is_word_xml_part(name) {
            wrap_marked_runs(xml, name, &marks, &mark_specs)
        } else {
            xml.to_string()
        }
    })?;

    write_template_package(&output, &manifest, &template_docx)?;
    Ok(SaveTemplateResult {
        output_path: output.display().to_string(),
        manifest,
    })
}

pub fn save_docx_template_to_library(mut args: SaveTemplateArgs) -> Result<SaveTemplateResult> {
    let file_name = safe_template_file_name(&args.template_name);
    args.output_path = template_library_dir()
        .join(format!("{file_name}.docsytpl"))
        .display()
        .to_string();
    save_docx_template(args)
}

pub fn list_template_library() -> Result<Vec<TemplateLibraryItem>> {
    list_template_library_items(&template_library_dir(), false)
}

pub fn list_template_trash() -> Result<Vec<TemplateLibraryItem>> {
    list_template_library_items(&template_trash_dir(), true)
}

fn list_template_library_items(dir: &Path, trashed: bool) -> Result<Vec<TemplateLibraryItem>> {
    std::fs::create_dir_all(&dir)?;
    let mut items = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("docsytpl"))
            .unwrap_or(false)
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
    items.sort_by(|a, b| b.updated.cmp(&a.updated).then_with(|| a.name.cmp(&b.name)));
    Ok(items)
}

pub fn move_template_to_trash(args: TemplateDeleteArgs) -> Result<String> {
    let source = PathBuf::from(args.path);
    let manifest = read_template_manifest(&source)?;
    let trash_dir = template_trash_dir();
    std::fs::create_dir_all(&trash_dir)?;
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("template.docsytpl");
    let target = unique_docx_output_path(&trash_dir.join(file_name))?;
    std::fs::rename(&source, &target)
        .or_else(|_| -> Result<(), std::io::Error> {
            std::fs::copy(&source, &target)?;
            std::fs::remove_file(&source)
        })
        .with_context(|| format!("移动模板到回收站失败: {}", source.display()))?;
    crate::template_history::mark_template_trashed(&manifest.template.id, true)?;
    Ok(target.display().to_string())
}

pub fn restore_template_from_trash(args: TemplateRestoreArgs) -> Result<String> {
    let source = PathBuf::from(args.path);
    let manifest = read_template_manifest(&source)?;
    let dir = template_library_dir();
    std::fs::create_dir_all(&dir)?;
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("template.docsytpl");
    let target = unique_docx_output_path(&dir.join(file_name))?;
    std::fs::rename(&source, &target)
        .or_else(|_| -> Result<(), std::io::Error> {
            std::fs::copy(&source, &target)?;
            std::fs::remove_file(&source)
        })
        .with_context(|| format!("恢复模板失败: {}", source.display()))?;
    crate::template_history::mark_template_trashed(&manifest.template.id, false)?;
    Ok(target.display().to_string())
}

pub fn permanently_delete_template(args: TemplatePermanentDeleteArgs) -> Result<()> {
    let source = PathBuf::from(args.path);
    let manifest = read_template_manifest(&source)?;
    if source.exists() {
        std::fs::remove_file(&source)
            .with_context(|| format!("彻底删除模板文件失败: {}", source.display()))?;
    }
    if args.migrate_to_common {
        crate::template_history::migrate_template_data_to_common(&manifest.template.id)?;
    } else {
        crate::template_history::delete_template_data(&manifest.template.id)?;
    }
    Ok(())
}

pub fn inspect_template_package(path: &str) -> Result<TemplateManifest> {
    read_template_manifest(Path::new(path))
}

pub fn render_docx_template(args: RenderTemplateArgs) -> Result<String> {
    let template_path = Path::new(&args.template_path);
    let output_path = unique_docx_output_path(Path::new(&args.output_path))?;
    let (manifest, template_docx) = read_template_package(template_path)?;
    validate_template_docx_layout(&template_docx, &manifest)?;

    let rendered = rewrite_docx_bytes(&template_docx, |name, xml| {
        if is_word_xml_part(name) {
            render_template_xml_with_overrides(
                xml,
                &manifest,
                &args.values,
                &args.structure_overrides,
            )
        } else {
            xml.to_string()
        }
    })?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, rendered)?;
    let output_path_str = output_path.display().to_string();
    if let Err(err) = crate::template_history::record_generation(
        &args.template_path,
        &manifest,
        &output_path_str,
        &args.values,
    ) {
        eprintln!("Docsy template history write failed: {err:#}");
    }
    Ok(output_path_str)
}

fn build_mark_tag_specs(fields: &[TemplateField]) -> HashMap<String, Vec<MarkTagSpec>> {
    let mut map: HashMap<String, Vec<MarkTagSpec>> = HashMap::new();
    for field in fields {
        if is_marker_field(&field.field_type) {
            for option in &field.options {
                if !option.marker_mark_id.is_empty() {
                    map.entry(option.marker_mark_id.clone())
                        .or_default()
                        .push(MarkTagSpec {
                            tag: option_marker_tag(&field.id, &option.id),
                            placeholder: option_marker_tag(&field.id, &option.id),
                            start: None,
                            end: None,
                            delete: false,
                        });
                }
            }
        } else if field.field_type == "delete_text" {
            for mark_ref in &field.mark_refs {
                if !mark_ref.mark_id.is_empty() {
                    map.entry(mark_ref.mark_id.clone())
                        .or_default()
                        .push(MarkTagSpec {
                            tag: String::new(),
                            placeholder: String::new(),
                            start: mark_ref.start,
                            end: mark_ref.end,
                            delete: true,
                        });
                }
            }
        } else if !field.mark_refs.is_empty() {
            for (index, mark_ref) in field.mark_refs.iter().enumerate() {
                if !mark_ref.mark_id.is_empty() {
                    map.entry(mark_ref.mark_id.clone())
                        .or_default()
                        .push(MarkTagSpec {
                            tag: mark_ref_tag(field, index),
                            placeholder: mark_ref_tag(field, index),
                            start: mark_ref.start,
                            end: mark_ref.end,
                            delete: false,
                        });
                }
            }
        } else {
            for mark in &field.marks {
                map.entry(mark.clone()).or_default().push(MarkTagSpec {
                    tag: field.id.clone(),
                    placeholder: field.id.clone(),
                    start: None,
                    end: None,
                    delete: false,
                });
            }
        }
    }
    map
}

fn validate_mark_specs(
    marks: &[TemplateMark],
    mark_specs: &HashMap<String, Vec<MarkTagSpec>>,
) -> Result<()> {
    let known: HashSet<&str> = marks.iter().map(|mark| mark.id.as_str()).collect();
    let missing: Vec<&str> = mark_specs
        .keys()
        .map(String::as_str)
        .filter(|mark_id| !known.contains(mark_id))
        .collect();
    if !missing.is_empty() {
        anyhow::bail!(
            "模板保存失败：{} 个标黄片段在 Word 文件中已找不到，请重新扫描后再保存",
            missing.len()
        )
    }

    for (mark_id, specs) in mark_specs {
        let field_specs = specs.iter().filter(|spec| !spec.delete).count();
        let whole_run_specs = specs
            .iter()
            .filter(|spec| spec.start.is_none() && spec.end.is_none())
            .count();
        if field_specs > 1 && whole_run_specs > 0 {
            anyhow::bail!(
                "模板保存失败：同一段标黄文字被多个字段重复使用，请先在确认表中拆分或只保留一个字段。\n片段：{}",
                mark_id
            );
        }
    }
    Ok(())
}

fn validate_template_source_docx(
    path: &Path,
    marks: &[TemplateMark],
    mark_specs: &HashMap<String, Vec<MarkTagSpec>>,
) -> Result<()> {
    let mut targeted: HashMap<&str, HashSet<usize>> = HashMap::new();
    for mark in marks
        .iter()
        .filter(|mark| mark_specs.contains_key(&mark.id))
    {
        targeted
            .entry(mark.part.as_str())
            .or_default()
            .insert(mark.run_index);
    }
    if targeted.is_empty() {
        return Ok(());
    }

    let file = std::fs::File::open(path)
        .with_context(|| format!("打开 Word 文件失败: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("读取 Word 文件失败")?;
    ensure_zip_entry_count(archive.len(), "Word 文件")?;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let name = file.name().to_string();
        let Some(target_runs) = targeted.get(name.as_str()) else {
            continue;
        };
        let size = file.size();
        let xml = read_string_with_limit(
            &mut file,
            size,
            MAX_XML_ENTRY_BYTES,
            &format!("Word XML: {name}"),
        )?;
        for (run_index, mat) in RUN_RE.find_iter(&xml).enumerate() {
            if !target_runs.contains(&run_index) {
                continue;
            }
            if xml_position_inside_tag(&xml, mat.start(), "w:sdt") {
                anyhow::bail!(
                    "模板保存失败：标黄文字位于 Word 内容控件内。Docsy 不能再嵌套内容控件，请在 Word 中移除该内容控件后重新导入。\n位置：{}",
                    name
                );
            }
        }
    }
    Ok(())
}

fn validate_template_docx_layout(template_docx: &[u8], manifest: &TemplateManifest) -> Result<()> {
    let party_tag_owners = party_tag_owner_map(manifest);
    if party_tag_owners.is_empty() {
        return Ok(());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(template_docx))?;
    ensure_zip_entry_count(archive.len(), "模板 Word 文件")?;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let name = file.name().to_string();
        if !is_word_xml_part(&name) {
            continue;
        }
        let size = file.size();
        let xml = read_string_with_limit(
            &mut file,
            size,
            MAX_XML_ENTRY_BYTES,
            &format!("模板 Word XML: {name}"),
        )?;
        for row in TABLE_ROW_RE.find_iter(&xml) {
            let block = row.as_str();
            let owners = party_fields_in_block(block, &party_tag_owners);
            if owners.is_empty() {
                continue;
            }
            if owners.len() > 1 {
                anyhow::bail!(
                    "模板生成失败：同一表格行中包含多个当事人列表字段。请把这些列表拆到不同表格行，避免生成交叉重复内容。"
                );
            }
            if block.contains("<w:tbl") {
                anyhow::bail!(
                    "模板生成失败：当事人列表所在表格行内还有嵌套表格。当前版本不能安全复制这种行，请改成普通表格行后再生成。"
                );
            }
        }
    }
    Ok(())
}

fn party_tag_owner_map(manifest: &TemplateManifest) -> HashMap<String, String> {
    let mut owners = HashMap::new();
    for field in manifest
        .fields
        .iter()
        .filter(|field| field.field_type == "party_list")
    {
        for tag in field_replacement_tags(field) {
            owners.insert(tag, field.id.clone());
        }
    }
    owners
}

fn party_fields_in_block(
    block: &str,
    party_tag_owners: &HashMap<String, String>,
) -> HashSet<String> {
    party_tag_owners
        .iter()
        .filter_map(|(tag, owner)| sdt_has_tag(block, tag).then_some(owner.clone()))
        .collect()
}

fn xml_position_inside_tag(xml: &str, pos: usize, tag: &str) -> bool {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let before = &xml[..pos.min(xml.len())];
    let last_open = before.rfind(&open);
    let last_close = before.rfind(&close);
    matches!(last_open, Some(open_pos) if last_close.map(|close_pos| close_pos < open_pos).unwrap_or(true))
}

fn normalize_manifest_options(manifest: &mut TemplateManifest) {
    for field in &mut manifest.fields {
        if is_marker_field(&field.field_type) {
            for option in &mut field.options {
                option.marker_tag = option_marker_tag(&field.id, &option.id);
                if option.checked_text.is_empty() {
                    option.checked_text = "☑".into();
                }
                if option.unchecked_text.is_empty() {
                    option.unchecked_text = "☐".into();
                }
            }
        } else {
            let field_id = field.id.clone();
            let multiple_refs = field.mark_refs.len() > 1;
            for (index, mark_ref) in field.mark_refs.iter_mut().enumerate() {
                if mark_ref.tag.is_empty() {
                    mark_ref.tag = if multiple_refs || mark_ref.optional_rule.is_some() {
                        field_ref_tag(&field_id, index)
                    } else {
                        field_id.clone()
                    };
                }
            }
        }
    }
}

fn sanitize_manifest_private_labels(manifest: &mut TemplateManifest, marks: &[TemplateMark]) {
    let marked_texts = marks
        .iter()
        .map(|mark| mark.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<HashSet<_>>();
    for field in &mut manifest.fields {
        let fallback = if field.name.trim().is_empty() {
            field.id.as_str()
        } else {
            field.name.as_str()
        };
        if is_generated_field_name(&field.name) || marked_texts.contains(field.label.trim()) {
            field.label = fallback.to_string();
        }
        if marked_texts.contains(field.semantic_key.trim()) {
            field.semantic_key = fallback.to_string();
        }
    }
}

fn is_generated_field_name(name: &str) -> bool {
    let value = name.trim();
    if let Some(rest) = value.strip_prefix("字段") {
        return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
    }
    if let Some(rest) = value.strip_prefix("field_") {
        return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
    }
    false
}

fn option_marker_tag(field_id: &str, option_id: &str) -> String {
    format!("{field_id}.option.{option_id}")
}

fn field_ref_tag(field_id: &str, index: usize) -> String {
    format!("{field_id}.ref.{}", index + 1)
}

fn mark_ref_tag(field: &TemplateField, index: usize) -> String {
    field
        .mark_refs
        .get(index)
        .map(|mark_ref| mark_ref.tag.as_str())
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| field.id.clone())
}

fn unique_docx_output_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("docx");
    for idx in 1..1000 {
        let candidate = parent.join(format!("{stem}-{idx}.{ext}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("无法生成不重名的输出文件")
}

fn is_marker_field(field_type: &str) -> bool {
    matches!(field_type, "checkbox" | "radio_group" | "checkbox_group")
}

fn extract_docx_plain_text(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("打开 Word 文件失败: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("读取 Word 文件失败")?;
    ensure_zip_entry_count(archive.len(), "Word 文件")?;
    let mut paragraphs = Vec::new();
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let name = file.name().to_string();
        if !is_word_xml_part(&name) {
            continue;
        }
        let size = file.size();
        let xml = read_string_with_limit(
            &mut file,
            size,
            MAX_XML_ENTRY_BYTES,
            &format!("Word XML: {name}"),
        )?;
        for paragraph in PARAGRAPH_RE.find_iter(&xml) {
            let text = extract_text(paragraph.as_str());
            if !text.trim().is_empty() {
                paragraphs.push(text);
            }
        }
    }
    Ok(paragraphs.join("\n"))
}

#[cfg(test)]
fn scan_xml_marks(part: &str, xml: &str) -> Vec<TemplateMark> {
    highlighted_runs_as_marks(&scan_xml_text_runs(part, xml))
}

fn scan_docx_text_runs(path: &Path) -> Result<Vec<TemplateTextRun>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("打开 Word 文件失败: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("读取 Word 文件失败")?;
    ensure_zip_entry_count(archive.len(), "Word 文件")?;
    let mut runs = Vec::new();
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let name = file.name().to_string();
        if !is_word_xml_part(&name) {
            continue;
        }
        let size = file.size();
        let xml = read_string_with_limit(
            &mut file,
            size,
            MAX_XML_ENTRY_BYTES,
            &format!("Word XML: {name}"),
        )?;
        runs.extend(scan_xml_text_runs(&name, &xml));
    }
    Ok(runs)
}

fn scan_xml_text_runs(part: &str, xml: &str) -> Vec<TemplateTextRun> {
    let mut runs = Vec::new();
    let paragraphs = PARAGRAPH_RE
        .find_iter(xml)
        .enumerate()
        .map(|(index, mat)| {
            (
                mat.start(),
                mat.end(),
                index,
                extract_text(mat.as_str()),
                mat.as_str().to_string(),
            )
        })
        .collect::<Vec<_>>();
    for (run_index, run) in RUN_RE.find_iter(xml).enumerate() {
        let run_xml = run.as_str();
        let text = extract_text(run_xml);
        if text.trim().is_empty() {
            continue;
        }
        let (paragraph_index, paragraph_text, text_offset) =
            run_paragraph_context(run.start(), &text, &paragraphs);
        let id = mark_id(part, run_index);
        runs.push(TemplateTextRun {
            id,
            part: part.to_string(),
            run_index,
            paragraph_index,
            text: text.clone(),
            paragraph_text: paragraph_text.clone(),
            checkbox_like: is_checkbox_like(&text),
            option_label: checkbox_option_label(&paragraph_text, &text, text_offset),
            highlighted: has_yellow_highlight(run_xml),
            bold: run_has_property(run_xml, "w:b"),
            italic: run_has_property(run_xml, "w:i"),
            underline: run_has_property(run_xml, "w:u"),
        });
    }
    runs
}

fn run_paragraph_context(
    run_start: usize,
    text: &str,
    paragraphs: &[(usize, usize, usize, String, String)],
) -> (usize, String, usize) {
    for (start, end, index, paragraph_text, paragraph_xml) in paragraphs {
        if run_start >= *start && run_start <= *end {
            let local_start = run_start.saturating_sub(*start).min(paragraph_xml.len());
            let offset = extract_text(&paragraph_xml[..local_start]).len();
            return (*index, paragraph_text.clone(), offset);
        }
    }
    (0, text.to_string(), 0)
}

fn highlighted_runs_as_marks(runs: &[TemplateTextRun]) -> Vec<TemplateMark> {
    runs.iter()
        .filter(|run| run.highlighted)
        .map(text_run_as_mark)
        .collect()
}

fn text_runs_as_marks(runs: &[TemplateTextRun]) -> Vec<TemplateMark> {
    runs.iter().map(text_run_as_mark).collect()
}

fn text_run_as_mark(run: &TemplateTextRun) -> TemplateMark {
    TemplateMark {
        id: run.id.clone(),
        part: run.part.clone(),
        run_index: run.run_index,
        text: run.text.clone(),
        context: compact_context(&run.paragraph_text, &run.text),
        checkbox_like: run.checkbox_like,
        option_label: run.option_label.clone(),
    }
}

fn run_has_property(run_xml: &str, tag: &str) -> bool {
    let self_closing = format!("<{tag}/");
    let with_attrs = format!("<{tag} ");
    let explicit_false = format!(r#"<{tag} w:val="false""#);
    let explicit_zero = format!(r#"<{tag} w:val="0""#);
    (run_xml.contains(&self_closing) || run_xml.contains(&with_attrs))
        && !run_xml.contains(&explicit_false)
        && !run_xml.contains(&explicit_zero)
}

fn checkbox_option_label(paragraph_text: &str, marker_text: &str, marker_start: usize) -> String {
    if !is_checkbox_like(marker_text) {
        return String::new();
    }
    let Some(after_start) = marker_start.checked_add(marker_text.len()) else {
        return String::new();
    };
    if after_start > paragraph_text.len() || !paragraph_text.is_char_boundary(after_start) {
        return String::new();
    }
    let after = paragraph_text[after_start..].trim_start();
    let label = after
        .split(|ch| is_checkbox_marker_char(ch) || ch == '\n' || ch == '\r')
        .next()
        .unwrap_or("")
        .trim();
    label.to_string()
}

fn is_checkbox_marker_char(ch: char) -> bool {
    CHECKBOX_MARKER_CHARS.contains(&ch)
}

fn wrap_marked_runs(
    xml: &str,
    part: &str,
    marks: &[TemplateMark],
    mark_specs: &HashMap<String, Vec<MarkTagSpec>>,
) -> String {
    let known: HashMap<usize, &TemplateMark> = marks
        .iter()
        .filter(|mark| mark.part == part)
        .map(|mark| (mark.run_index, mark))
        .collect();
    if known.is_empty() {
        return xml.to_string();
    }

    let mut out = String::with_capacity(xml.len() + 1024);
    let mut last = 0usize;
    let mut run_index = 0usize;
    for mat in RUN_RE.find_iter(xml) {
        out.push_str(&xml[last..mat.start()]);
        let run_xml = mat.as_str();
        if let Some(mark) = known.get(&run_index) {
            if let Some(specs) = mark_specs.get(&mark.id) {
                out.push_str(&wrap_run_with_specs(run_xml, specs));
            } else {
                out.push_str(&strip_highlight(run_xml));
            }
        } else {
            out.push_str(run_xml);
        }
        last = mat.end();
        run_index += 1;
    }
    out.push_str(&xml[last..]);
    out
}

fn wrap_run_with_specs(run_xml: &str, specs: &[MarkTagSpec]) -> String {
    if specs.is_empty() {
        return strip_highlight(run_xml);
    }
    if specs.len() == 1 && specs[0].delete && specs[0].start.is_none() && specs[0].end.is_none() {
        return String::new();
    }
    if specs.len() == 1 && specs[0].start.is_none() && specs[0].end.is_none() {
        return wrap_run_as_sdt(run_xml, &specs[0].tag, &specs[0].tag, &specs[0].placeholder);
    }

    let text = extract_text(run_xml);
    let char_len = text.chars().count();
    if char_len == 0 {
        return strip_highlight(run_xml);
    }

    let mut ranges = specs
        .iter()
        .filter_map(|spec| {
            let start = spec.start.unwrap_or(0).min(char_len);
            let end = spec.end.unwrap_or(char_len).min(char_len);
            (start < end).then_some((
                start,
                end,
                spec.tag.as_str(),
                spec.placeholder.as_str(),
                spec.delete,
            ))
        })
        .collect::<Vec<_>>();
    ranges.sort_by_key(|(start, end, _, _, _)| (*start, *end));

    if ranges.is_empty() {
        return strip_highlight(run_xml);
    }

    let rpr = run_properties_without_highlight(run_xml);
    let mut out = String::new();
    let mut cursor = 0usize;
    for (start, end, tag, placeholder, delete) in ranges {
        if start < cursor {
            continue;
        }
        if cursor < start {
            out.push_str(&build_run_from_text(
                &rpr,
                &slice_chars(&text, cursor, start),
            ));
        }
        if !delete {
            let segment = build_run_from_text(&rpr, &slice_chars(&text, start, end));
            out.push_str(&wrap_run_as_sdt(&segment, tag, tag, placeholder));
        }
        cursor = end;
    }
    if cursor < char_len {
        out.push_str(&build_run_from_text(
            &rpr,
            &slice_chars(&text, cursor, char_len),
        ));
    }
    out
}

fn wrap_run_as_sdt(run_xml: &str, tag: &str, alias: &str, placeholder: &str) -> String {
    let placeholder_text = if placeholder.trim().is_empty() {
        tag
    } else {
        placeholder
    };
    let run = replace_text_nodes_preserving_runs(
        &strip_highlight(run_xml),
        &format!("{{{{{placeholder_text}}}}}"),
    );
    format!(
        r#"<w:sdt><w:sdtPr><w:alias w:val="{}"/><w:tag w:val="{}"/></w:sdtPr><w:sdtContent>{}</w:sdtContent></w:sdt>"#,
        escape_attr(alias),
        escape_attr(tag),
        run
    )
}

fn strip_highlight(run_xml: &str) -> String {
    HIGHLIGHT_RE.replace_all(run_xml, "").to_string()
}

fn run_properties_without_highlight(run_xml: &str) -> String {
    RPR_RE
        .find(run_xml)
        .map(|mat| strip_highlight(mat.as_str()))
        .unwrap_or_default()
}

fn build_run_from_text(rpr: &str, text: &str) -> String {
    format!(
        r#"<w:r>{}<w:t xml:space="preserve">{}</w:t></w:r>"#,
        rpr,
        escape_text(text)
    )
}

fn slice_chars(value: &str, start: usize, end: usize) -> String {
    value
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

#[cfg(test)]
fn render_template_xml(
    xml: &str,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
) -> String {
    render_template_xml_with_overrides(xml, manifest, values, &HashMap::new())
}

fn render_template_xml_with_overrides(
    xml: &str,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
    structure_overrides: &HashMap<String, StructureOverride>,
) -> String {
    let (xml, mut removed_optional_tags) = remove_optional_empty_fields(xml, manifest, values);
    let (xml, repeated_party_fields) = render_party_list_blocks(&xml, manifest, values);
    let mut xml = xml;
    let mut replacements = HashMap::new();
    for field in &manifest.fields {
        if field.field_type == "delete_text" {
            continue;
        } else if is_marker_field(&field.field_type) {
            append_marker_replacements(field, values, &mut replacements);
        } else if field.field_type == "party_list" && repeated_party_fields.contains(&field.id) {
            continue;
        } else if field.field_type == "party_list" {
            append_party_list_replacements(&xml, field, values, &mut replacements);
        } else {
            let value = field_value(field, values);
            let clusters = field_tag_clusters(&xml, &field_replacement_tags(field));
            if clusters.is_empty() {
                for tag in field_replacement_tags(field) {
                    if !removed_optional_tags.contains(&tag) {
                        replacements.insert(tag, value.clone());
                    }
                }
            } else {
                for cluster in clusters {
                    for (index, tag) in cluster.iter().enumerate() {
                        if !removed_optional_tags.contains(tag) {
                            replacements.insert(
                                tag.clone(),
                                if index == 0 {
                                    value.clone()
                                } else {
                                    String::new()
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    remove_empty_mark_ref_optional_slots(
        &mut xml,
        manifest,
        &replacements,
        &mut removed_optional_tags,
    );
    remove_explicit_party_item_template_suffixes(&mut xml, manifest, values);
    remove_reference_static_role_prefixes(&mut xml, manifest, &replacements);
    apply_structure_overrides(&mut xml, manifest, values, structure_overrides);
    replace_sdt_values(&xml, &replacements)
}

fn apply_structure_overrides(
    xml: &mut String,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
    structure_overrides: &HashMap<String, StructureOverride>,
) {
    if structure_overrides.is_empty() {
        return;
    }
    for field in &manifest.fields {
        let Some(override_rule) = structure_overrides
            .get(&field.id)
            .or_else(|| structure_overrides.get(&field.name))
        else {
            continue;
        };
        let raw = values
            .get(&field.id)
            .or_else(|| values.get(&field.name))
            .unwrap_or(&Value::Null);
        if is_empty_field_value(field, raw) {
            continue;
        }
        if let Some(rule) = field.optional_rule.as_ref().filter(|rule| rule.enabled) {
            for tag in field_replacement_tags(field) {
                let (next, changed) = replace_sdt_affixes(xml, &tag, rule, override_rule);
                if changed {
                    *xml = next;
                }
            }
        }
        for (index, mark_ref) in field.mark_refs.iter().enumerate() {
            let Some(rule) = mark_ref.optional_rule.as_ref().filter(|rule| rule.enabled) else {
                continue;
            };
            let tag = mark_ref_tag(field, index);
            let (next, changed) = replace_sdt_affixes(xml, &tag, rule, override_rule);
            if changed {
                *xml = next;
            }
        }
    }
}

fn replace_sdt_affixes(
    xml: &str,
    tag: &str,
    rule: &OptionalFieldRule,
    override_rule: &StructureOverride,
) -> (String, bool) {
    let mut current = xml.to_string();
    let mut changed = false;
    if let Some(prefix) = override_rule.prefix.as_ref() {
        let (next, did_change) =
            replace_prefix_before_sdt(&current, tag, &rule.remove_empty_prefix, prefix);
        current = next;
        changed = changed || did_change;
    }
    if let Some(suffix) = override_rule.suffix.as_ref() {
        let (next, did_change) =
            replace_suffix_after_sdt(&current, tag, &rule.remove_empty_suffix, suffix);
        current = next;
        changed = changed || did_change;
    }
    (current, changed)
}

fn remove_reference_static_role_prefixes(
    xml: &mut String,
    manifest: &TemplateManifest,
    replacements: &HashMap<String, String>,
) {
    for field in manifest
        .fields
        .iter()
        .filter(|field| field.reference.is_some())
    {
        for tag in field_replacement_tags(field) {
            let Some(value) = replacements.get(&tag) else {
                continue;
            };
            let Some(role) = leading_party_role(value) else {
                continue;
            };
            let (next, changed) = remove_party_role_before_sdt(xml, &tag, role);
            if changed {
                *xml = next;
            }
        }
    }
}

fn leading_party_role(value: &str) -> Option<&'static str> {
    const ROLES: [&str; 7] = [
        "被申请人",
        "被上诉人",
        "申请人",
        "上诉人",
        "第三人",
        "原告",
        "被告",
    ];
    let compact = value.trim();
    ROLES.iter().copied().find(|role| {
        compact
            .strip_prefix(role)
            .and_then(|rest| rest.chars().next())
            .map(|next| !next.is_ascii_digit())
            .unwrap_or(false)
    })
}

fn remove_explicit_party_item_template_suffixes(
    xml: &mut String,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
) {
    for field in manifest
        .fields
        .iter()
        .filter(|field| field.field_type == "party_list")
    {
        let raw = values
            .get(&field.name)
            .or_else(|| values.get(&field.id))
            .unwrap_or(&Value::Null);
        let explicit_suffixes = party_list_explicit_suffix_flags(raw);
        for (index, mark_ref) in field.mark_refs.iter().enumerate() {
            if !explicit_suffixes.get(index).copied().unwrap_or(false) {
                continue;
            }
            let Some(rule) = mark_ref.optional_rule.as_ref().filter(|rule| rule.enabled) else {
                continue;
            };
            if rule.remove_empty_suffix.is_empty() {
                continue;
            }
            let tag = mark_ref_tag(field, index);
            let (next, changed) = remove_suffix_after_sdt(xml, &tag, &rule.remove_empty_suffix);
            if changed {
                *xml = next;
            }
        }
    }
}

fn remove_empty_mark_ref_optional_slots(
    xml: &mut String,
    manifest: &TemplateManifest,
    replacements: &HashMap<String, String>,
    removed_optional_tags: &mut HashSet<String>,
) {
    for field in &manifest.fields {
        for (index, mark_ref) in field.mark_refs.iter().enumerate() {
            let tag = mark_ref_tag(field, index);
            if removed_optional_tags.contains(&tag) {
                continue;
            }
            let Some(value) = replacements.get(&tag) else {
                continue;
            };
            if !value.trim().is_empty() {
                continue;
            }
            let Some(rule) = mark_ref.optional_rule.as_ref().filter(|rule| rule.enabled) else {
                continue;
            };
            let (next, changed) = remove_optional_sdt(
                xml,
                &tag,
                &rule.remove_empty_prefix,
                &rule.remove_empty_suffix,
            );
            if changed {
                *xml = next;
                removed_optional_tags.insert(tag);
            }
        }
    }
}

fn remove_optional_empty_fields(
    xml: &str,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
) -> (String, HashSet<String>) {
    let mut current = xml.to_string();
    let mut removed = HashSet::new();
    for field in &manifest.fields {
        let raw = values
            .get(&field.name)
            .or_else(|| values.get(&field.id))
            .unwrap_or(&Value::Null);
        if !is_empty_field_value(field, raw) {
            continue;
        }
        if let Some(rule) = field.optional_rule.as_ref().filter(|rule| rule.enabled) {
            for tag in field_replacement_tags(field) {
                let (next, changed) = remove_optional_sdt(
                    &current,
                    &tag,
                    &rule.remove_empty_prefix,
                    &rule.remove_empty_suffix,
                );
                if changed {
                    current = next;
                    removed.insert(tag);
                }
            }
        }
        for (index, mark_ref) in field.mark_refs.iter().enumerate() {
            let Some(rule) = mark_ref.optional_rule.as_ref().filter(|rule| rule.enabled) else {
                continue;
            };
            let tag = mark_ref_tag(field, index);
            let (next, changed) = remove_optional_sdt(
                &current,
                &tag,
                &rule.remove_empty_prefix,
                &rule.remove_empty_suffix,
            );
            if changed {
                current = next;
                removed.insert(tag);
            }
        }
    }
    (current, removed)
}

fn field_replacement_tags(field: &TemplateField) -> Vec<String> {
    if field.mark_refs.is_empty() {
        return vec![field.id.clone()];
    }
    field
        .mark_refs
        .iter()
        .enumerate()
        .map(|(index, _)| mark_ref_tag(field, index))
        .collect()
}

fn render_party_list_blocks(
    xml: &str,
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
) -> (String, HashSet<String>) {
    let mut current = xml.to_string();
    let mut repeated = HashSet::new();
    for field in manifest
        .fields
        .iter()
        .filter(|field| field.field_type == "party_list")
    {
        let raw = values
            .get(&field.name)
            .or_else(|| values.get(&field.id))
            .unwrap_or(&Value::Null);
        let items = party_list_items(raw);
        if items.is_empty() {
            continue;
        }
        let (next, changed) = replace_blocks_for_party_list(&current, field, &items);
        if changed {
            current = next;
            repeated.insert(field.id.clone());
        }
    }
    (current, repeated)
}

fn replace_blocks_for_party_list(
    xml: &str,
    field: &TemplateField,
    items: &[String],
) -> (String, bool) {
    replace_matching_blocks(xml, &TABLE_ROW_RE, &field_replacement_tags(field), items)
}

fn replace_matching_blocks(
    xml: &str,
    regex: &Regex,
    tags: &[String],
    items: &[String],
) -> (String, bool) {
    let mut out = String::with_capacity(xml.len() + items.len().saturating_mul(128));
    let mut last = 0usize;
    let mut changed = false;
    for mat in regex.find_iter(xml) {
        out.push_str(&xml[last..mat.start()]);
        let block = mat.as_str();
        if tags.iter().any(|tag| sdt_has_tag(block, tag)) {
            changed = true;
            for item in items {
                let replacements = tags.iter().map(|tag| (tag.clone(), item.clone())).collect();
                out.push_str(&replace_sdt_values(block, &replacements));
            }
        } else {
            out.push_str(block);
        }
        last = mat.end();
    }
    out.push_str(&xml[last..]);
    (out, changed)
}

fn append_marker_replacements(
    field: &TemplateField,
    values: &HashMap<String, Value>,
    replacements: &mut HashMap<String, String>,
) {
    let raw = values
        .get(&field.name)
        .or_else(|| values.get(&field.id))
        .cloned()
        .unwrap_or(Value::Null);
    let selected = selected_option_ids(&raw);

    for option in &field.options {
        let tag = if option.marker_tag.is_empty() {
            option_marker_tag(&field.id, &option.id)
        } else {
            option.marker_tag.clone()
        };
        let checked = match field.field_type.as_str() {
            "checkbox" => value_as_bool(&raw),
            "radio_group" => {
                selected.contains(&option.id) || raw == Value::String(option.label.clone())
            }
            "checkbox_group" => selected.contains(&option.id) || selected.contains(&option.label),
            _ => false,
        };
        replacements.insert(
            tag,
            if checked {
                option.checked_text.clone()
            } else {
                option.unchecked_text.clone()
            },
        );
    }
}

fn append_party_list_replacements(
    xml: &str,
    field: &TemplateField,
    values: &HashMap<String, Value>,
    replacements: &mut HashMap<String, String>,
) {
    let tags = field_replacement_tags(field);
    let raw = values
        .get(&field.name)
        .or_else(|| values.get(&field.id))
        .unwrap_or(&Value::Null);
    let items = party_list_render_items(raw);
    let clusters = field_tag_clusters(xml, &tags);
    if clusters.is_empty() {
        let value = items.join("、");
        for tag in tags {
            replacements.insert(tag, value.clone());
        }
        return;
    }

    for cluster in clusters {
        if cluster.len() == 1 {
            let value = if field.mark_refs.len() <= 1 {
                items.join("、")
            } else {
                items.first().cloned().unwrap_or_default()
            };
            replacements.insert(cluster[0].clone(), value);
            continue;
        }

        for (index, tag) in cluster.iter().enumerate() {
            let value = if index >= items.len() {
                String::new()
            } else if index == cluster.len() - 1 && items.len() > cluster.len() {
                items[index..].join("、")
            } else {
                items[index].clone()
            };
            replacements.insert(tag.clone(), value);
        }
    }
}

#[derive(Debug, Clone)]
struct TaggedSdt {
    tag: String,
    start: usize,
    end: usize,
}

fn field_tag_clusters(xml: &str, tags: &[String]) -> Vec<Vec<String>> {
    let tag_set: HashSet<&str> = tags.iter().map(String::as_str).collect();
    let mut items = Vec::new();
    for mat in SDT_RE.find_iter(xml) {
        let block = mat.as_str();
        let Some(tag) = sdt_tag_value(block) else {
            continue;
        };
        if tag_set.contains(tag.as_str()) {
            items.push(TaggedSdt {
                tag,
                start: mat.start(),
                end: mat.end(),
            });
        }
    }
    if items.is_empty() {
        return Vec::new();
    }

    let mut clusters: Vec<Vec<String>> = Vec::new();
    let mut current = vec![items[0].tag.clone()];
    for pair in items.windows(2) {
        let between = &xml[pair[0].end..pair[1].start];
        if is_same_inline_field_cluster(between) {
            current.push(pair[1].tag.clone());
        } else {
            clusters.push(current);
            current = vec![pair[1].tag.clone()];
        }
    }
    clusters.push(current);
    clusters
}

fn is_same_inline_field_cluster(xml_between: &str) -> bool {
    let text = extract_text(xml_between);
    let compact = text.split_whitespace().collect::<String>();
    if compact.is_empty() {
        return true;
    }
    compact.chars().count() <= 6 && compact.chars().any(is_list_connector_char)
}

fn is_list_connector_char(ch: char) -> bool {
    matches!(ch, '、' | '，' | ',' | '；' | ';' | '和' | '与' | '及')
}

fn field_value(field: &TemplateField, values: &HashMap<String, Value>) -> String {
    if let Some(reference) = field.reference.as_ref() {
        if let Some(value) = values
            .get(&field.id)
            .or_else(|| values.get(&field.name))
            .filter(|value| !value_to_string(value).trim().is_empty())
        {
            return value_to_string(value);
        }
        if reference.source_field.trim().is_empty()
            && reference.source_semantic_key.trim().is_empty()
        {
            return values
                .get(&field.name)
                .or_else(|| values.get(&field.id))
                .map(value_to_string)
                .unwrap_or_default();
        }
        return referenced_field_value(reference, values);
    }
    let raw = values
        .get(&field.name)
        .or_else(|| values.get(&field.id))
        .unwrap_or(&Value::Null);
    match field.field_type.as_str() {
        "date" => date_value(raw),
        "party_list" => party_list_render_items(raw).join("、"),
        _ => value_to_string(raw),
    }
}

fn referenced_field_value(
    reference: &TemplateFieldReference,
    values: &HashMap<String, Value>,
) -> String {
    let raw = if !reference.source_semantic_key.trim().is_empty() {
        values
            .get(&reference.source_semantic_key)
            .unwrap_or(&Value::Null)
    } else {
        values.get(&reference.source_field).unwrap_or(&Value::Null)
    };
    if let Some(index) = reference.source_index {
        return party_list_items(raw)
            .get(index)
            .cloned()
            .unwrap_or_default();
    }
    value_to_string(raw)
}

fn is_empty_field_value(field: &TemplateField, value: &Value) -> bool {
    match field.field_type.as_str() {
        "party_list" | "checkbox_group" => match value {
            Value::Array(items) => items
                .iter()
                .all(|item| value_to_string(item).trim().is_empty()),
            _ => value_to_string(value).trim().is_empty(),
        },
        "checkbox" => !value_as_bool(value),
        _ => value_to_string(value).trim().is_empty(),
    }
}

fn date_value(value: &Value) -> String {
    let text = value_to_string(value);
    if text.contains('年') || text.trim().is_empty() {
        return text;
    }
    let date_part = text.split('T').next().unwrap_or(text.as_str());
    let mut parts = date_part.split('-');
    let Some(year) = parts.next() else {
        return text;
    };
    let Some(month) = parts.next() else {
        return text;
    };
    let Some(day) = parts.next() else {
        return text;
    };
    if parts.next().is_some() || year.len() != 4 {
        return text;
    }
    let Ok(month_value) = month.parse::<u8>() else {
        return text;
    };
    let Ok(day_value) = day.parse::<u8>() else {
        return text;
    };
    if !(1..=12).contains(&month_value) || !(1..=31).contains(&day_value) {
        return text;
    }
    format!("{year}年{month_value}月{day_value}日")
}

fn selected_option_ids(value: &Value) -> HashSet<String> {
    match value {
        Value::String(text) => [text.clone()].into_iter().collect(),
        Value::Array(items) => items
            .iter()
            .map(value_to_string)
            .filter(|s| !s.is_empty())
            .collect(),
        _ => HashSet::new(),
    }
}

fn value_as_bool(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::String(value) => matches!(value.as_str(), "true" | "1" | "是" | "选中"),
        _ => false,
    }
}

fn party_list_items(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.clone()),
                Value::Object(map) => map
                    .get("name")
                    .or_else(|| map.get("label"))
                    .map(value_to_string),
                _ => None,
            })
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>(),
        Value::String(text) => text
            .split(['、', '\n'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => {
            let value = value_to_string(value);
            if value.is_empty() {
                Vec::new()
            } else {
                vec![value]
            }
        }
    }
}

fn party_list_render_items(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::Object(map) => {
                    let name = map
                        .get("name")
                        .or_else(|| map.get("label"))
                        .or_else(|| map.get("text"))
                        .map(value_to_string)
                        .unwrap_or_default();
                    if name.trim().is_empty() {
                        None
                    } else {
                        let suffix = map.get("suffix").map(value_to_string).unwrap_or_default();
                        Some(format!("{name}{suffix}"))
                    }
                }
                _ => Some(value_to_string(item)),
            })
            .filter(|text| !text.trim().is_empty())
            .collect(),
        _ => party_list_items(value),
    }
}

fn party_list_explicit_suffix_flags(value: &Value) -> Vec<bool> {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                matches!(
                    item,
                    Value::Object(map)
                        if map.contains_key("suffix")
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => {
            if *value {
                "是".into()
            } else {
                "否".into()
            }
        }
        Value::Array(items) => items
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join("、"),
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn replace_sdt_values(xml: &str, replacements: &HashMap<String, String>) -> String {
    if replacements.is_empty() {
        return xml.to_string();
    }
    let xml = remove_empty_replacement_connectors(xml, replacements);
    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    for mat in SDT_RE.find_iter(&xml) {
        out.push_str(&xml[last..mat.start()]);
        let block = mat.as_str();
        if let Some((_, value)) = replacements.iter().find(|(tag, _)| sdt_has_tag(block, tag)) {
            out.push_str(&replace_sdt_content(block, value));
        } else {
            out.push_str(block);
        }
        last = mat.end();
    }
    out.push_str(&xml[last..]);
    out
}

fn remove_empty_replacement_connectors(
    xml: &str,
    replacements: &HashMap<String, String>,
) -> String {
    let empty_tags: HashSet<&str> = replacements
        .iter()
        .filter_map(|(tag, value)| {
            if value.trim().is_empty() {
                Some(tag.as_str())
            } else {
                None
            }
        })
        .collect();
    if empty_tags.is_empty() {
        return xml.to_string();
    }

    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    let mut pending_next_connector_removal = false;
    for mat in SDT_RE.find_iter(xml) {
        let mut before = xml[last..mat.start()].to_string();
        if pending_next_connector_removal {
            let (next, removed) = remove_leading_list_connector_from_first_text_node(&before);
            before = next;
            pending_next_connector_removal = !removed;
        }
        let block = mat.as_str();
        let should_clean = sdt_tag_value(block)
            .as_deref()
            .map(|tag| empty_tags.contains(tag))
            .unwrap_or(false);
        if should_clean {
            let (next, removed) = remove_trailing_list_connector_from_last_text_node(&before);
            before = next;
            pending_next_connector_removal = !removed;
        }
        out.push_str(&before);
        out.push_str(block);
        last = mat.end();
    }
    let mut tail = xml[last..].to_string();
    if pending_next_connector_removal {
        tail = remove_leading_list_connector_from_first_text_node(&tail).0;
    }
    out.push_str(&tail);
    out
}

fn remove_trailing_list_connector_from_last_text_node(xml: &str) -> (String, bool) {
    let Some(mat) = TEXT_NODE_RE.find_iter(xml).last() else {
        return (xml.to_string(), false);
    };
    let Some(caps) = TEXT_NODE_RE.captures(mat.as_str()) else {
        return (xml.to_string(), false);
    };
    let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    let Some((start, _)) = trailing_list_connector_range(content) else {
        return (xml.to_string(), false);
    };
    (
        format!(
            "{}{}{}{}{}",
            &xml[..mat.start()],
            caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            &content[..start],
            caps.get(3).map(|m| m.as_str()).unwrap_or(""),
            &xml[mat.end()..]
        ),
        true,
    )
}

fn remove_leading_list_connector_from_first_text_node(xml: &str) -> (String, bool) {
    let Some(mat) = TEXT_NODE_RE.find(xml) else {
        return (xml.to_string(), false);
    };
    let Some(caps) = TEXT_NODE_RE.captures(mat.as_str()) else {
        return (xml.to_string(), false);
    };
    let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    let Some((_, end)) = leading_list_connector_range(content) else {
        return (xml.to_string(), false);
    };
    (
        format!(
            "{}{}{}{}{}",
            &xml[..mat.start()],
            caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            &content[end..],
            caps.get(3).map(|m| m.as_str()).unwrap_or(""),
            &xml[mat.end()..]
        ),
        true,
    )
}

fn trailing_list_connector_range(text: &str) -> Option<(usize, usize)> {
    let trimmed_end = text.trim_end_matches(char::is_whitespace).len();
    let prefix = &text[..trimmed_end];
    let (start, ch) = prefix.char_indices().last()?;
    if is_empty_inline_slot_connector_char(ch) {
        Some((start, trimmed_end))
    } else {
        None
    }
}

fn leading_list_connector_range(text: &str) -> Option<(usize, usize)> {
    let trimmed_start = text.len() - text.trim_start_matches(char::is_whitespace).len();
    let rest = &text[trimmed_start..];
    let ch = rest.chars().next()?;
    if is_empty_inline_slot_connector_char(ch) {
        Some((trimmed_start, trimmed_start + ch.len_utf8()))
    } else {
        None
    }
}

fn is_empty_inline_slot_connector_char(ch: char) -> bool {
    ch == '、'
}

fn remove_optional_sdt(xml: &str, tag: &str, prefix: &str, suffix: &str) -> (String, bool) {
    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    let mut changed = false;
    let mut pending_suffix: Option<String> = None;
    for mat in SDT_RE.find_iter(xml) {
        let mut before = xml[last..mat.start()].to_string();
        if let Some(value) = pending_suffix.take() {
            let (next, removed) = remove_text_from_first_text_node(&before, &value);
            before = next;
            if !removed {
                pending_suffix = Some(value);
            }
        }
        let block = mat.as_str();
        if !sdt_has_tag(block, tag) {
            out.push_str(&before);
            out.push_str(block);
            last = mat.end();
            continue;
        }
        if !prefix.is_empty() {
            before = remove_text_from_last_text_node(&before, prefix);
        }
        out.push_str(&before);
        last = mat.end();
        changed = true;
        if !suffix.is_empty() {
            pending_suffix = Some(suffix.to_string());
        }
    }
    if !changed {
        return (xml.to_string(), false);
    }
    let mut tail = xml[last..].to_string();
    if let Some(value) = pending_suffix {
        tail = remove_text_from_first_text_node(&tail, &value).0;
    }
    out.push_str(&tail);
    (out, true)
}

fn remove_suffix_after_sdt(xml: &str, tag: &str, suffix: &str) -> (String, bool) {
    if suffix.is_empty() {
        return (xml.to_string(), false);
    }
    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    let mut changed = false;
    let mut pending_suffix = false;
    for mat in SDT_RE.find_iter(xml) {
        let mut before = xml[last..mat.start()].to_string();
        if pending_suffix {
            let (next, removed) = remove_text_from_first_text_node(&before, suffix);
            before = next;
            pending_suffix = !removed;
            changed = changed || removed;
        }
        let block = mat.as_str();
        out.push_str(&before);
        out.push_str(block);
        last = mat.end();
        if sdt_has_tag(block, tag) {
            pending_suffix = true;
        }
    }
    let mut tail = xml[last..].to_string();
    if pending_suffix {
        let (next, removed) = remove_text_from_first_text_node(&tail, suffix);
        tail = next;
        changed = changed || removed;
    }
    if !changed {
        return (xml.to_string(), false);
    }
    out.push_str(&tail);
    (out, true)
}

fn replace_prefix_before_sdt(
    xml: &str,
    tag: &str,
    old_prefix: &str,
    new_prefix: &str,
) -> (String, bool) {
    let mut out = String::with_capacity(xml.len() + new_prefix.len());
    let mut last = 0usize;
    let mut changed = false;
    for mat in SDT_RE.find_iter(xml) {
        let mut before = xml[last..mat.start()].to_string();
        let block = mat.as_str();
        if sdt_has_tag(block, tag) {
            if !old_prefix.is_empty() {
                before = remove_text_from_last_text_node(&before, old_prefix);
            }
            before = append_text_to_last_text_node(&before, new_prefix);
            changed = true;
        }
        out.push_str(&before);
        out.push_str(block);
        last = mat.end();
    }
    if !changed {
        return (xml.to_string(), false);
    }
    out.push_str(&xml[last..]);
    (out, true)
}

fn replace_suffix_after_sdt(
    xml: &str,
    tag: &str,
    old_suffix: &str,
    new_suffix: &str,
) -> (String, bool) {
    let mut out = String::with_capacity(xml.len() + new_suffix.len());
    let mut last = 0usize;
    let mut changed = false;
    let mut pending_suffix: Option<(&str, &str)> = None;
    for mat in SDT_RE.find_iter(xml) {
        let mut before = xml[last..mat.start()].to_string();
        if let Some((old, new)) = pending_suffix.take() {
            if !old.is_empty() {
                before = remove_text_from_first_text_node(&before, old).0;
            }
            before = prepend_text_to_first_text_node(&before, new);
        }
        let block = mat.as_str();
        out.push_str(&before);
        out.push_str(block);
        last = mat.end();
        if sdt_has_tag(block, tag) {
            pending_suffix = Some((old_suffix, new_suffix));
            changed = true;
        }
    }
    let mut tail = xml[last..].to_string();
    if let Some((old, new)) = pending_suffix {
        if !old.is_empty() {
            tail = remove_text_from_first_text_node(&tail, old).0;
        }
        tail = prepend_text_to_first_text_node(&tail, new);
    }
    if !changed {
        return (xml.to_string(), false);
    }
    out.push_str(&tail);
    (out, true)
}

fn remove_prefix_before_sdt(xml: &str, tag: &str, prefix: &str) -> (String, bool) {
    if prefix.is_empty() {
        return (xml.to_string(), false);
    }
    let mut out = String::with_capacity(xml.len());
    let mut last = 0usize;
    let mut changed = false;
    for mat in SDT_RE.find_iter(xml) {
        let mut before = xml[last..mat.start()].to_string();
        let block = mat.as_str();
        if sdt_has_tag(block, tag) {
            let next = remove_text_from_last_text_node(&before, prefix);
            changed = changed || next != before;
            before = next;
        }
        out.push_str(&before);
        out.push_str(block);
        last = mat.end();
    }
    if !changed {
        return (xml.to_string(), false);
    }
    out.push_str(&xml[last..]);
    (out, true)
}

fn remove_party_role_before_sdt(xml: &str, tag: &str, _new_role: &str) -> (String, bool) {
    const ROLES: [&str; 7] = [
        "被申请人",
        "被上诉人",
        "申请人",
        "上诉人",
        "第三人",
        "原告",
        "被告",
    ];
    let mut current = xml.to_string();
    for role in ROLES {
        let (next, changed) = remove_prefix_before_sdt(&current, tag, role);
        if changed {
            current = next;
            return (current, true);
        }
    }
    (xml.to_string(), false)
}

fn append_text_to_last_text_node(xml: &str, text: &str) -> String {
    if text.is_empty() {
        return xml.to_string();
    }
    let escaped = escape_text(text);
    let Some(mat) = TEXT_NODE_RE.find_iter(xml).last() else {
        return xml.to_string();
    };
    let Some(caps) = TEXT_NODE_RE.captures(mat.as_str()) else {
        return xml.to_string();
    };
    let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    format!(
        "{}{}{}{}{}{}",
        &xml[..mat.start()],
        caps.get(1).map(|m| m.as_str()).unwrap_or(""),
        content,
        escaped,
        caps.get(3).map(|m| m.as_str()).unwrap_or(""),
        &xml[mat.end()..]
    )
}

fn prepend_text_to_first_text_node(xml: &str, text: &str) -> String {
    if text.is_empty() {
        return xml.to_string();
    }
    let escaped = escape_text(text);
    let Some(mat) = TEXT_NODE_RE.find(xml) else {
        return xml.to_string();
    };
    let Some(caps) = TEXT_NODE_RE.captures(mat.as_str()) else {
        return xml.to_string();
    };
    let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    format!(
        "{}{}{}{}{}{}",
        &xml[..mat.start()],
        caps.get(1).map(|m| m.as_str()).unwrap_or(""),
        escaped,
        content,
        caps.get(3).map(|m| m.as_str()).unwrap_or(""),
        &xml[mat.end()..]
    )
}

fn remove_text_from_last_text_node(xml: &str, text: &str) -> String {
    let escaped = escape_text(text);
    let Some(mat) = TEXT_NODE_RE.find_iter(xml).last() else {
        return xml.to_string();
    };
    let Some(caps) = TEXT_NODE_RE.captures(mat.as_str()) else {
        return xml.to_string();
    };
    let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    if !content.ends_with(&escaped) {
        return xml.to_string();
    }
    let trimmed_content = &content[..content.len() - escaped.len()];
    format!(
        "{}{}{}{}{}",
        &xml[..mat.start()],
        caps.get(1).map(|m| m.as_str()).unwrap_or(""),
        trimmed_content,
        caps.get(3).map(|m| m.as_str()).unwrap_or(""),
        &xml[mat.end()..]
    )
}

fn remove_text_from_first_text_node(xml: &str, text: &str) -> (String, bool) {
    let escaped = escape_text(text);
    let Some(mat) = TEXT_NODE_RE.find(xml) else {
        return (xml.to_string(), false);
    };
    let Some(caps) = TEXT_NODE_RE.captures(mat.as_str()) else {
        return (xml.to_string(), false);
    };
    let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    if !content.starts_with(&escaped) {
        return (xml.to_string(), false);
    }
    let trimmed_content = &content[escaped.len()..];
    (
        format!(
            "{}{}{}{}{}",
            &xml[..mat.start()],
            caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            trimmed_content,
            caps.get(3).map(|m| m.as_str()).unwrap_or(""),
            &xml[mat.end()..]
        ),
        true,
    )
}

fn sdt_has_tag(block: &str, tag: &str) -> bool {
    let needle = format!(r#"w:val="{}""#, escape_attr(tag));
    block.contains(&needle)
}

fn sdt_tag_value(block: &str) -> Option<String> {
    let tag_pos = block.find("<w:tag")?;
    let tag_end = block[tag_pos..]
        .find('>')
        .map(|offset| tag_pos + offset)
        .unwrap_or(block.len());
    let tag_xml = &block[tag_pos..tag_end];
    let marker = "w:val=\"";
    let value_start = tag_xml.find(marker)? + marker.len();
    let value_end = tag_xml[value_start..].find('"')? + value_start;
    Some(decode_xml_text(&tag_xml[value_start..value_end]))
}

fn replace_sdt_content(block: &str, value: &str) -> String {
    let Some(start_tag) = block.find("<w:sdtContent>") else {
        return block.to_string();
    };
    let content_start = start_tag + "<w:sdtContent>".len();
    let Some(relative_end) = block[content_start..].find("</w:sdtContent>") else {
        return block.to_string();
    };
    let content_end = content_start + relative_end;
    let original = &block[content_start..content_end];
    let replacement = replace_text_nodes_preserving_runs(original, value);
    replacement
}

fn replace_text_nodes_preserving_runs(original_content: &str, value: &str) -> String {
    if !TEXT_NODE_RE.is_match(original_content) {
        return build_run_from_text("", value);
    }
    let mut wrote_value = false;
    TEXT_NODE_RE
        .replace_all(original_content, |caps: &regex::Captures| {
            let replacement = if wrote_value {
                String::new()
            } else {
                wrote_value = true;
                escape_text(value)
            };
            format!("{}{}{}", &caps[1], replacement, &caps[3])
        })
        .to_string()
}

fn rewrite_docx<F>(path: &Path, transform: F) -> Result<Vec<u8>>
where
    F: Fn(&str, &str) -> String,
{
    let bytes = read_file_with_limit(path, MAX_DOCX_BYTES, "Word 文件")?;
    rewrite_docx_bytes(&bytes, transform)
}

fn rewrite_docx_bytes<F>(bytes: &[u8], transform: F) -> Result<Vec<u8>>
where
    F: Fn(&str, &str) -> String,
{
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    ensure_zip_entry_count(archive.len(), "Word 文件")?;
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx)?;
            let name = file.name().to_string();
            if file.is_dir() {
                writer.add_directory(&name, options)?;
                continue;
            }
            let size = file.size();
            let data = read_vec_with_limit(
                &mut file,
                size,
                if is_word_xml_part(&name) {
                    MAX_XML_ENTRY_BYTES
                } else {
                    MAX_BINARY_ENTRY_BYTES
                },
                &format!("Word 条目: {name}"),
            )?;
            writer.start_file(&name, options)?;
            if is_word_xml_part(&name) {
                let xml = String::from_utf8(data)
                    .with_context(|| format!("Word XML 不是 UTF-8: {name}"))?;
                writer.write_all(transform(&name, &xml).as_bytes())?;
            } else {
                writer.write_all(&data)?;
            }
        }
        writer.finish()?;
    }
    Ok(output.into_inner())
}

fn write_template_package(
    path: &Path,
    manifest: &TemplateManifest,
    template_docx: &[u8],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer.start_file(MANIFEST_PATH, options)?;
    writer.write_all(serde_json::to_string_pretty(manifest)?.as_bytes())?;
    writer.start_file(TEMPLATE_DOCX_PATH, options)?;
    writer.write_all(template_docx)?;
    writer.finish()?;
    Ok(())
}

fn template_library_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("Docsy")
        .join("templates")
}

fn template_trash_dir() -> PathBuf {
    template_library_dir().join("_trash")
}

fn safe_template_file_name(name: &str) -> String {
    let cleaned = name
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
    if cleaned.is_empty() {
        "未命名模板".to_string()
    } else {
        cleaned
    }
}

fn read_template_manifest(path: &Path) -> Result<TemplateManifest> {
    let bytes = read_file_with_limit(path, MAX_DOCSYTPL_BYTES, "Docsy 模板文件")?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("读取 Docsy 模板失败")?;
    ensure_zip_entry_count(archive.len(), "Docsy 模板文件")?;
    let mut file = archive
        .by_name(MANIFEST_PATH)
        .context("模板文件缺少 manifest.json，无法打开")?;
    let size = file.size();
    let manifest =
        read_string_with_limit(&mut file, size, MAX_MANIFEST_BYTES, "模板 manifest.json")?;
    parse_template_manifest(&manifest)
}

fn read_template_package(path: &Path) -> Result<(TemplateManifest, Vec<u8>)> {
    let bytes = read_file_with_limit(path, MAX_DOCSYTPL_BYTES, "Docsy 模板文件")?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("读取 Docsy 模板失败")?;
    ensure_zip_entry_count(archive.len(), "Docsy 模板文件")?;
    let mut manifest_file = archive
        .by_name(MANIFEST_PATH)
        .context("模板文件缺少 manifest.json，无法打开")?;
    let manifest_size = manifest_file.size();
    let manifest_json = read_string_with_limit(
        &mut manifest_file,
        manifest_size,
        MAX_MANIFEST_BYTES,
        "模板 manifest.json",
    )?;
    drop(manifest_file);
    let mut docx_file = archive
        .by_name(TEMPLATE_DOCX_PATH)
        .context("模板文件缺少 template.docx，无法生成文档")?;
    let docx_size = docx_file.size();
    let template_docx = read_vec_with_limit(
        &mut docx_file,
        docx_size,
        MAX_DOCX_BYTES,
        "模板内置 Word 文件",
    )?;
    Ok((parse_template_manifest(&manifest_json)?, template_docx))
}

fn parse_template_manifest(manifest_json: &str) -> Result<TemplateManifest> {
    let manifest: TemplateManifest =
        serde_json::from_str(manifest_json).context("模板 manifest.json 格式不正确")?;
    if manifest.format_version != 1 {
        anyhow::bail!(
            "模板版本不兼容：当前 Docsy 支持版本 1，但该模板是版本 {}",
            manifest.format_version
        );
    }
    for field in &manifest.fields {
        if !is_supported_field_type(&field.field_type) {
            anyhow::bail!(
                "模板字段类型不兼容：字段“{}”使用了未知类型“{}”",
                if field.label.is_empty() {
                    field.name.as_str()
                } else {
                    field.label.as_str()
                },
                field.field_type
            );
        }
    }
    Ok(manifest)
}

fn is_supported_field_type(field_type: &str) -> bool {
    matches!(
        field_type,
        "text"
            | "date"
            | "party_list"
            | "reference"
            | "checkbox"
            | "radio_group"
            | "checkbox_group"
            | "delete_text"
    )
}

fn read_file_with_limit(path: &Path, limit: u64, label: &str) -> Result<Vec<u8>> {
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
    std::fs::read(path).with_context(|| format!("读取{label}失败: {}", path.display()))
}

fn ensure_zip_entry_count(count: usize, label: &str) -> Result<()> {
    if count > MAX_ZIP_ENTRIES {
        anyhow::bail!(
            "{}包含过多内部文件，无法安全处理：{} 个，限制 {} 个",
            label,
            count,
            MAX_ZIP_ENTRIES
        );
    }
    Ok(())
}

fn read_string_with_limit<R: Read>(
    reader: R,
    declared_size: u64,
    limit: u64,
    label: &str,
) -> Result<String> {
    let data = read_vec_with_limit(reader, declared_size, limit, label)?;
    String::from_utf8(data).with_context(|| format!("{label} 不是 UTF-8 文本"))
}

fn read_vec_with_limit<R: Read>(
    reader: R,
    declared_size: u64,
    limit: u64,
    label: &str,
) -> Result<Vec<u8>> {
    if declared_size > limit {
        anyhow::bail!(
            "{}过大，无法安全读取：{} MB，限制 {} MB",
            label,
            declared_size / 1024 / 1024,
            limit / 1024 / 1024
        );
    }
    let mut limited = reader.take(limit + 1);
    let mut data = Vec::new();
    limited
        .read_to_end(&mut data)
        .with_context(|| format!("读取{label}失败"))?;
    if data.len() as u64 > limit {
        anyhow::bail!("{}解压后超过安全限制", label);
    }
    Ok(data)
}

fn is_word_xml_part(name: &str) -> bool {
    name == "word/document.xml"
        || (name.starts_with("word/header") && name.ends_with(".xml"))
        || (name.starts_with("word/footer") && name.ends_with(".xml"))
}

fn has_yellow_highlight(run_xml: &str) -> bool {
    run_xml.contains("<w:highlight")
        && (run_xml.contains(r#"w:val="yellow""#) || run_xml.contains(r#"w:val='yellow'"#))
}

fn extract_text(xml: &str) -> String {
    TEXT_RE
        .captures_iter(xml)
        .map(|caps| decode_xml_text(&caps[1]))
        .collect::<Vec<_>>()
        .join("")
}

fn compact_context(paragraph_text: &str, _mark_text: &str) -> String {
    let context = paragraph_text.trim().to_string();
    if context.chars().count() > 120 {
        return context.chars().take(120).collect::<String>() + "…";
    }
    context
}

fn is_checkbox_like(text: &str) -> bool {
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        return is_checkbox_marker_char(ch);
    }
    CHECKBOX_MARKER_TEXTS.contains(&trimmed)
}

fn mark_id(part: &str, run_index: usize) -> String {
    format!("mark_{:016x}_{run_index}", fnv1a_hash(part))
}

fn fnv1a_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn decode_xml_text(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_yellow_runs_without_merging() {
        let xml = r#"<w:document><w:body><w:p>
        <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>张三</w:t></w:r>
        <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>李四</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0].text, "张三");
        assert_eq!(marks[1].text, "李四");
    }

    #[test]
    fn sanitizes_generated_field_labels_from_marked_text() {
        let marks = vec![TemplateMark {
            id: "mark_1".into(),
            part: "word/document.xml".into(),
            run_index: 0,
            text: "F事务所".into(),
            context: "F事务所".into(),
            checkbox_like: false,
            option_label: String::new(),
        }];
        let mut manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_text_16".into(),
                name: "字段16".into(),
                label: "F事务所".into(),
                semantic_key: "F事务所".into(),
                field_type: "text".into(),
                required: false,
                marks: vec!["mark_1".into()],
                mark_refs: Vec::new(),
                options: Vec::new(),
                optional_rule: None,
                reference: None,
            }],
        };

        sanitize_manifest_private_labels(&mut manifest, &marks);

        assert_eq!(manifest.fields[0].label, "字段16");
        assert_eq!(manifest.fields[0].semantic_key, "字段16");
    }

    #[test]
    fn wraps_and_renders_content_control() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:b/><w:highlight w:val="yellow"/></w:rPr><w:t>张三</w:t></w:r></w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        let mut map = HashMap::new();
        map.insert(
            marks[0].id.clone(),
            vec![MarkTagSpec {
                tag: "fld_client".to_string(),
                placeholder: "fld_client".to_string(),
                start: None,
                end: None,
                delete: false,
            }],
        );
        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);
        assert!(wrapped.contains(r#"w:tag w:val="fld_client""#));
        assert!(wrapped.contains("{{fld_client}}"));
        assert!(!wrapped.contains(">张三<"));
        assert!(!wrapped.contains("<w:highlight"));

        let mut replacements = HashMap::new();
        replacements.insert("fld_client".to_string(), "李四".to_string());
        let rendered = replace_sdt_values(&wrapped, &replacements);
        assert!(rendered.contains(">李四<"));
        assert!(rendered.contains("<w:b/>"));
    }

    #[test]
    fn renders_checkbox_marker_without_touching_run_properties() {
        let block = r#"<w:sdt><w:sdtPr><w:tag w:val="fld_auth.option.special"/></w:sdtPr><w:sdtContent><w:r><w:rPr><w:rFonts w:ascii="Wingdings 2"/></w:rPr><w:t>☐</w:t></w:r></w:sdtContent></w:sdt>"#;
        let mut replacements = HashMap::new();
        replacements.insert("fld_auth.option.special".to_string(), "☑".to_string());
        let rendered = replace_sdt_values(block, &replacements);
        assert!(rendered.contains(">☑<"));
        assert!(rendered.contains(r#"w:ascii="Wingdings 2""#));
    }

    #[test]
    fn scans_checkbox_marker_with_following_label() {
        let xml = r#"<w:document><w:body><w:p>
        <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>□</w:t></w:r>
        <w:r><w:t> 一般授权  </w:t></w:r>
        <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>□</w:t></w:r>
        <w:r><w:t> 特别授权</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0].option_label, "一般授权");
        assert_eq!(marks[1].option_label, "特别授权");
    }

    #[test]
    fn scans_checkmark_marker_with_following_label() {
        let xml = r#"<w:document><w:body><w:p>
        <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>✓</w:t></w:r>
        <w:r><w:t> 一般授权  </w:t></w:r>
        <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>□</w:t></w:r>
        <w:r><w:t> 特别授权</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0].option_label, "一般授权");
        assert_eq!(marks[1].option_label, "特别授权");
    }

    #[test]
    fn formats_iso_date_as_chinese_legal_date() {
        assert_eq!(
            date_value(&Value::String("2026-04-23".into())),
            "2026年4月23日"
        );
        assert_eq!(
            date_value(&Value::String("2026-04-23T10:00:00".into())),
            "2026年4月23日"
        );
        assert_eq!(
            date_value(&Value::String("2026年4月23日".into())),
            "2026年4月23日"
        );
    }

    #[test]
    fn leaves_invalid_date_text_unchanged() {
        assert_eq!(
            date_value(&Value::String("2026-13-45".into())),
            "2026-13-45"
        );
    }

    #[test]
    fn removes_highlight_from_disabled_marks() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>保留文字</w:t></w:r></w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        let map = HashMap::new();
        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);
        assert!(wrapped.contains(">保留文字<"));
        assert!(!wrapped.contains("<w:highlight"));
        assert!(!wrapped.contains("<w:sdt"));
    }

    #[test]
    fn preserves_non_yellow_highlight_when_saving_template() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:highlight w:val="green"/></w:rPr><w:t>审阅高亮</w:t></w:r></w:p></w:body></w:document>"#;
        let runs = scan_xml_text_runs("word/document.xml", xml);
        let marks = text_runs_as_marks(&runs);
        let map = HashMap::new();
        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);
        assert!(wrapped.contains(r#"<w:highlight w:val="green"/>"#));
        assert!(wrapped.contains(">审阅高亮<"));
    }

    #[test]
    fn scan_uses_global_run_index_inside_text_boxes() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>正文</w:t></w:r><w:pict><v:shape><w:txbxContent><w:p><w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>文本框字段</w:t></w:r></w:p></w:txbxContent></v:shape></w:pict><w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>后文</w:t></w:r></w:p></w:body></w:document>"#;
        let runs = scan_xml_text_runs("word/document.xml", xml);
        let text_box = runs.iter().find(|run| run.text == "文本框字段").unwrap();
        let after = runs.iter().find(|run| run.text == "后文").unwrap();
        assert_eq!(text_box.run_index, 1);
        assert_eq!(after.run_index, 2);
    }

    #[test]
    fn detects_target_mark_inside_existing_content_control() {
        let xml = r#"<w:document><w:body><w:p><w:sdt><w:sdtContent><w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>已有控件</w:t></w:r></w:sdtContent></w:sdt></w:p></w:body></w:document>"#;
        let runs = scan_xml_text_runs("word/document.xml", xml);
        let marks = text_runs_as_marks(&runs);
        let mut specs = HashMap::new();
        specs.insert(
            marks[0].id.clone(),
            vec![MarkTagSpec {
                tag: "fld_existing".into(),
                placeholder: "fld_existing".into(),
                start: None,
                end: None,
                delete: false,
            }],
        );
        let targeted = [marks[0].run_index].into_iter().collect::<HashSet<_>>();
        let run = RUN_RE.find(xml).unwrap();
        assert!(targeted.contains(&marks[0].run_index));
        assert!(xml_position_inside_tag(xml, run.start(), "w:sdt"));
        assert!(validate_mark_specs(&marks, &specs).is_ok());
    }

    #[test]
    fn detects_multiple_party_list_fields_in_one_table_row() {
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![
                TemplateField {
                    id: "fld_a".into(),
                    name: "原告".into(),
                    label: "原告".into(),
                    field_type: "party_list".into(),
                    ..Default::default()
                },
                TemplateField {
                    id: "fld_b".into(),
                    name: "被告".into(),
                    label: "被告".into(),
                    field_type: "party_list".into(),
                    ..Default::default()
                },
            ],
        };
        let row = r#"<w:tr><w:tc><w:sdt><w:sdtPr><w:tag w:val="fld_a"/></w:sdtPr><w:sdtContent><w:r><w:t>A</w:t></w:r></w:sdtContent></w:sdt></w:tc><w:tc><w:sdt><w:sdtPr><w:tag w:val="fld_b"/></w:sdtPr><w:sdtContent><w:r><w:t>B</w:t></w:r></w:sdtContent></w:sdt></w:tc></w:tr>"#;
        let owners = party_tag_owner_map(&manifest);
        assert_eq!(party_fields_in_block(row, &owners).len(), 2);
    }

    #[test]
    fn wraps_split_mark_ranges() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:b/><w:highlight w:val="yellow"/></w:rPr><w:t>张三李四</w:t></w:r></w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        let mut map = HashMap::new();
        map.insert(
            marks[0].id.clone(),
            vec![
                MarkTagSpec {
                    tag: "fld_a".to_string(),
                    placeholder: "fld_a".to_string(),
                    start: Some(0),
                    end: Some(2),
                    delete: false,
                },
                MarkTagSpec {
                    tag: "fld_b".to_string(),
                    placeholder: "fld_b".to_string(),
                    start: Some(2),
                    end: Some(4),
                    delete: false,
                },
            ],
        );
        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);
        assert!(wrapped.contains(r#"w:tag w:val="fld_a""#));
        assert!(wrapped.contains(r#"w:tag w:val="fld_b""#));
        assert!(wrapped.contains("{{fld_a}}"));
        assert!(wrapped.contains("{{fld_b}}"));
        assert!(!wrapped.contains("张三"));
        assert!(!wrapped.contains("李四"));
        assert!(wrapped.contains("<w:b/>"));
        assert!(!wrapped.contains("<w:highlight"));
    }

    #[test]
    fn deletes_selected_text_without_rebuilding_surrounding_format() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:b/><w:u w:val="single"/></w:rPr><w:t>原告张三律师</w:t></w:r></w:p></w:body></w:document>"#;
        let runs = scan_xml_text_runs("word/document.xml", xml);
        let marks = text_runs_as_marks(&runs);
        let mut map = HashMap::new();
        map.insert(
            marks[0].id.clone(),
            vec![MarkTagSpec {
                tag: String::new(),
                placeholder: String::new(),
                start: Some(2),
                end: Some(4),
                delete: true,
            }],
        );
        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);
        assert!(wrapped.contains(">原告<"));
        assert!(wrapped.contains(">律师<"));
        assert!(!wrapped.contains("张三"));
        assert_eq!(wrapped.matches("<w:b/>").count(), 2);
        assert_eq!(wrapped.matches(r#"<w:u w:val="single"/>"#).count(), 2);
    }

    #[test]
    fn deletes_entire_run_when_delete_spec_has_no_range() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:b/><w:highlight w:val="yellow"/></w:rPr><w:t>整段删除</w:t></w:r><w:r><w:t>保留</w:t></w:r></w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        let mut map = HashMap::new();
        map.insert(
            marks[0].id.clone(),
            vec![MarkTagSpec {
                tag: String::new(),
                placeholder: String::new(),
                start: None,
                end: None,
                delete: true,
            }],
        );

        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);

        assert!(!wrapped.contains("整段删除"));
        assert!(wrapped.contains(">保留<"));
        assert!(!wrapped.contains("<w:highlight"));
        assert!(!wrapped.contains("<w:sdt"));
    }

    #[test]
    fn mixes_delete_and_content_control_ranges_in_one_run() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:b/><w:u w:val="single"/></w:rPr><w:t>原告张三律师</w:t></w:r></w:p></w:body></w:document>"#;
        let runs = scan_xml_text_runs("word/document.xml", xml);
        let marks = text_runs_as_marks(&runs);
        let mut map = HashMap::new();
        map.insert(
            marks[0].id.clone(),
            vec![
                MarkTagSpec {
                    tag: "fld_role".to_string(),
                    placeholder: "fld_role".to_string(),
                    start: Some(0),
                    end: Some(2),
                    delete: false,
                },
                MarkTagSpec {
                    tag: String::new(),
                    placeholder: String::new(),
                    start: Some(2),
                    end: Some(4),
                    delete: true,
                },
                MarkTagSpec {
                    tag: "fld_suffix".to_string(),
                    placeholder: "fld_suffix".to_string(),
                    start: Some(4),
                    end: Some(6),
                    delete: false,
                },
            ],
        );

        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);

        assert!(wrapped.contains(r#"w:tag w:val="fld_role""#));
        assert!(wrapped.contains(r#"w:tag w:val="fld_suffix""#));
        assert!(wrapped.contains("{{fld_role}}"));
        assert!(wrapped.contains("{{fld_suffix}}"));
        assert!(!wrapped.contains(">原告<"));
        assert!(!wrapped.contains(">律师<"));
        assert!(!wrapped.contains("张三"));
        assert_eq!(wrapped.matches("<w:b/>").count(), 2);
        assert_eq!(wrapped.matches(r#"<w:u w:val="single"/>"#).count(), 2);
    }

    #[test]
    fn preserves_run_formatting_when_splitting_one_mark() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:rPr><w:rFonts w:eastAsia="宋体" w:ascii="Times New Roman"/><w:b/><w:u w:val="single"/><w:sz w:val="24"/><w:highlight w:val="yellow"/></w:rPr><w:t>吕晗律师</w:t></w:r></w:p></w:body></w:document>"#;
        let marks = scan_xml_marks("word/document.xml", xml);
        let mut map = HashMap::new();
        map.insert(
            marks[0].id.clone(),
            vec![
                MarkTagSpec {
                    tag: "fld_lawyer".to_string(),
                    placeholder: "fld_lawyer".to_string(),
                    start: Some(0),
                    end: Some(2),
                    delete: false,
                },
                MarkTagSpec {
                    tag: "fld_lawyer_suffix".to_string(),
                    placeholder: "fld_lawyer_suffix".to_string(),
                    start: Some(2),
                    end: Some(4),
                    delete: false,
                },
            ],
        );

        let wrapped = wrap_marked_runs(xml, "word/document.xml", &marks, &map);

        assert!(wrapped.contains("{{fld_lawyer}}"));
        assert!(wrapped.contains("{{fld_lawyer_suffix}}"));
        assert!(!wrapped.contains(">吕晗<"));
        assert!(!wrapped.contains(">律师<"));
        assert_eq!(wrapped.matches(r#"w:eastAsia="宋体""#).count(), 2);
        assert_eq!(wrapped.matches(r#"w:ascii="Times New Roman""#).count(), 2);
        assert_eq!(wrapped.matches("<w:b/>").count(), 2);
        assert_eq!(wrapped.matches(r#"<w:u w:val="single"/>"#).count(), 2);
        assert_eq!(wrapped.matches(r#"<w:sz w:val="24"/>"#).count(), 2);
        assert!(!wrapped.contains("<w:highlight"));
    }

    #[test]
    fn preserves_runs_when_replacing_content() {
        let block = r#"<w:sdt><w:sdtPr><w:tag w:val="fld_client"/></w:sdtPr><w:sdtContent><w:r><w:rPr><w:b/></w:rPr><w:t>张</w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t>三</w:t></w:r></w:sdtContent></w:sdt>"#;
        let mut replacements = HashMap::new();
        replacements.insert("fld_client".to_string(), "李四".to_string());
        let rendered = replace_sdt_values(block, &replacements);
        assert!(rendered.contains(">李四<"));
        assert!(rendered.contains("<w:b/>"));
        assert!(rendered.contains("<w:i/>"));
    }

    #[test]
    fn joins_inline_party_list_with_dunhao() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>当事人：</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_parties"/></w:sdtPr><w:sdtContent><w:r><w:rPr><w:b/></w:rPr><w:t>张三</w:t></w:r></w:sdtContent></w:sdt></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_parties".into(),
                name: "parties".into(),
                label: "当事人".into(),
                field_type: "party_list".into(),
                ..Default::default()
            }],
        };
        let values = [(
            "parties".to_string(),
            Value::Array(vec![
                Value::String("张三".into()),
                Value::String("李四".into()),
            ]),
        )]
        .into_iter()
        .collect();
        let rendered = render_template_xml(xml, &manifest, &values);
        assert_eq!(rendered.matches("<w:p>").count(), 1);
        assert!(rendered.contains(">张三、李四<"));
        assert!(rendered.contains("<w:b/>"));
    }

    #[test]
    fn fills_inline_party_list_slots_by_position() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>原告</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_plaintiff.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>A公司</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>、</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_plaintiff.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>B公司</w:t></w:r></w:sdtContent></w:sdt><w:sdt><w:sdtPr><w:tag w:val="fld_plaintiff.ref.3"/></w:sdtPr><w:sdtContent><w:r><w:t>公司</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>与被告</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_plaintiff".into(),
                name: "原告".into(),
                label: "原告".into(),
                field_type: "party_list".into(),
                mark_refs: vec![
                    TemplateMarkRef {
                        mark_id: "mark_1".into(),
                        start: None,
                        end: None,
                        tag: "fld_plaintiff.ref.1".into(),
                        optional_rule: None,
                    },
                    TemplateMarkRef {
                        mark_id: "mark_2".into(),
                        start: None,
                        end: None,
                        tag: "fld_plaintiff.ref.2".into(),
                        optional_rule: None,
                    },
                    TemplateMarkRef {
                        mark_id: "mark_3".into(),
                        start: None,
                        end: None,
                        tag: "fld_plaintiff.ref.3".into(),
                        optional_rule: None,
                    },
                ],
                ..Default::default()
            }],
        };
        let values = [(
            "原告".to_string(),
            Value::Array(vec![
                Value::String("原告1".into()),
                Value::String("原告2".into()),
            ]),
        )]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);

        assert!(extract_text(&rendered).contains("原告原告1、原告2与被告"));
        assert!(!extract_text(&rendered).contains("原告1、原告2、原告1"));
        assert!(!rendered.contains("<w:sdt"));
    }

    #[test]
    fn removes_connector_before_empty_inline_party_slot() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>原告</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_plaintiff.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>A公司</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>、</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_plaintiff.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>B公司</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>与被告</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_defendant.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>C公司</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>、</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_defendant.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>D公司</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>之间</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![
                TemplateField {
                    id: "fld_plaintiff".into(),
                    name: "原告".into(),
                    label: "原告".into(),
                    field_type: "party_list".into(),
                    mark_refs: vec![
                        TemplateMarkRef {
                            mark_id: "mark_1".into(),
                            start: None,
                            end: None,
                            tag: "fld_plaintiff.ref.1".into(),
                            optional_rule: None,
                        },
                        TemplateMarkRef {
                            mark_id: "mark_2".into(),
                            start: None,
                            end: None,
                            tag: "fld_plaintiff.ref.2".into(),
                            optional_rule: None,
                        },
                    ],
                    ..Default::default()
                },
                TemplateField {
                    id: "fld_defendant".into(),
                    name: "被告".into(),
                    label: "被告".into(),
                    field_type: "party_list".into(),
                    mark_refs: vec![
                        TemplateMarkRef {
                            mark_id: "mark_3".into(),
                            start: None,
                            end: None,
                            tag: "fld_defendant.ref.1".into(),
                            optional_rule: None,
                        },
                        TemplateMarkRef {
                            mark_id: "mark_4".into(),
                            start: None,
                            end: None,
                            tag: "fld_defendant.ref.2".into(),
                            optional_rule: None,
                        },
                    ],
                    ..Default::default()
                },
            ],
        };
        let values = [
            (
                "原告".to_string(),
                Value::Array(vec![Value::String("A公司".into())]),
            ),
            (
                "被告".to_string(),
                Value::Array(vec![Value::String("C公司".into())]),
            ),
        ]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);
        let text = extract_text(&rendered);

        assert!(text.contains("原告A公司与被告C公司之间"));
        assert!(!text.contains("A公司、与"));
        assert!(!text.contains("C公司、之间"));
        assert!(!rendered.contains("<w:sdt"));
    }

    #[test]
    fn collapses_adjacent_duplicate_text_refs_to_one_value() {
        let xml = r#"<w:document><w:body><w:p><w:sdt><w:sdtPr><w:tag w:val="fld_court.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>法院A</w:t></w:r></w:sdtContent></w:sdt><w:sdt><w:sdtPr><w:tag w:val="fld_court.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>法院B</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>：</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_court".into(),
                name: "法院".into(),
                label: "法院".into(),
                field_type: "text".into(),
                mark_refs: vec![
                    TemplateMarkRef {
                        mark_id: "mark_1".into(),
                        start: None,
                        end: None,
                        tag: "fld_court.ref.1".into(),
                        optional_rule: None,
                    },
                    TemplateMarkRef {
                        mark_id: "mark_2".into(),
                        start: None,
                        end: None,
                        tag: "fld_court.ref.2".into(),
                        optional_rule: None,
                    },
                ],
                ..Default::default()
            }],
        };
        let values = [("法院".to_string(), Value::String("北京知识产权法院".into()))]
            .into_iter()
            .collect();

        let rendered = render_template_xml(xml, &manifest, &values);

        assert!(extract_text(&rendered).contains("北京知识产权法院："));
        assert!(!extract_text(&rendered).contains("北京知识产权法院北京知识产权法院"));
        assert!(!rendered.contains("<w:sdt"));
    }

    #[test]
    fn renders_reference_field_from_party_list_item() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>，第三人</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_principal"/></w:sdtPr><w:sdtContent><w:r><w:t>委托人</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>委托本所</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_principal".into(),
                name: "委托人".into(),
                label: "委托人".into(),
                field_type: "reference".into(),
                reference: Some(TemplateFieldReference {
                    source_mode: "field".into(),
                    source_field: "第三人".into(),
                    source_semantic_key: String::new(),
                    source_index: Some(0),
                }),
                mark_refs: vec![TemplateMarkRef {
                    mark_id: "mark_1".into(),
                    start: None,
                    end: None,
                    tag: "fld_principal".into(),
                    optional_rule: None,
                }],
                ..Default::default()
            }],
        };
        let values = [(
            "第三人".to_string(),
            Value::Array(vec![
                Value::String("第三人1".into()),
                Value::String("第三人2".into()),
            ]),
        )]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);

        assert!(extract_text(&rendered).contains("，第三人第三人1委托本所"));
        assert!(!rendered.contains("<w:sdt"));
    }

    #[test]
    fn renders_reference_field_from_semantic_key() {
        let xml = r#"<w:document><w:body><w:p><w:sdt><w:sdtPr><w:tag w:val="fld_principal"/></w:sdtPr><w:sdtContent><w:r><w:t>委托人</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>委托本所</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_principal".into(),
                name: "委托人".into(),
                label: "委托人".into(),
                field_type: "reference".into(),
                reference: Some(TemplateFieldReference {
                    source_mode: "semantic".into(),
                    source_field: String::new(),
                    source_semantic_key: "当前委托人".into(),
                    source_index: None,
                }),
                mark_refs: vec![TemplateMarkRef {
                    mark_id: "mark_1".into(),
                    start: None,
                    end: None,
                    tag: "fld_principal".into(),
                    optional_rule: None,
                }],
                ..Default::default()
            }],
        };
        let values = [(
            "当前委托人".to_string(),
            Value::String("浦项股份有限公司".into()),
        )]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);

        assert!(extract_text(&rendered).contains("浦项股份有限公司委托本所"));
        assert!(!rendered.contains("<w:sdt"));
    }

    #[test]
    fn direct_reference_value_overrides_semantic_reference_source() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>第三人</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_principal"/></w:sdtPr><w:sdtContent><w:r><w:t>委托人</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>委托本所</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_principal".into(),
                name: "委托人".into(),
                label: "委托人".into(),
                field_type: "reference".into(),
                reference: Some(TemplateFieldReference {
                    source_mode: "semantic".into(),
                    source_field: String::new(),
                    source_semantic_key: "当事人".into(),
                    source_index: None,
                }),
                mark_refs: vec![TemplateMarkRef {
                    mark_id: "mark_1".into(),
                    start: None,
                    end: None,
                    tag: "fld_principal".into(),
                    optional_rule: None,
                }],
                ..Default::default()
            }],
        };
        let values = [
            (
                "当事人".to_string(),
                Value::Array(vec![
                    Value::String("第一个原告".into()),
                    Value::String("第一个被告".into()),
                    Value::String("第一个第三人".into()),
                ]),
            ),
            ("委托人".to_string(), Value::String("第一个原告".into())),
        ]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);
        let text = extract_text(&rendered);

        assert!(text.contains("第三人第一个原告委托本所"));
        assert!(!text.contains("第一个原告、第一个被告"));
    }

    #[test]
    fn reference_role_value_replaces_static_role_prefix() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>，第三人</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_principal"/></w:sdtPr><w:sdtContent><w:r><w:t>委托人</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>委托本所</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_principal".into(),
                name: "委托人".into(),
                label: "委托人".into(),
                field_type: "reference".into(),
                reference: Some(TemplateFieldReference {
                    source_mode: "semantic".into(),
                    source_field: String::new(),
                    source_semantic_key: "当事人".into(),
                    source_index: None,
                }),
                mark_refs: vec![TemplateMarkRef {
                    mark_id: "mark_1".into(),
                    start: None,
                    end: None,
                    tag: "fld_principal".into(),
                    optional_rule: None,
                }],
                ..Default::default()
            }],
        };
        let values = [("委托人".to_string(), Value::String("原告第一个原告".into()))]
            .into_iter()
            .collect();

        let rendered = render_template_xml(xml, &manifest, &values);
        let text = extract_text(&rendered);

        assert!(text.contains("，原告第一个原告委托本所"));
        assert!(!text.contains("，第三人原告"));
    }

    #[test]
    fn explicit_party_suffix_replaces_template_suffix() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>本所</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_lawyer.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>锑</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师</w:t></w:r><w:r><w:t>、</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_lawyer.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>铁</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>实习律师</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_lawyer".into(),
                name: "律师".into(),
                label: "律师".into(),
                field_type: "party_list".into(),
                mark_refs: vec![
                    TemplateMarkRef {
                        mark_id: "mark_1".into(),
                        start: None,
                        end: None,
                        tag: "fld_lawyer.ref.1".into(),
                        optional_rule: Some(OptionalFieldRule {
                            enabled: true,
                            remove_empty_prefix: String::new(),
                            remove_empty_suffix: "律师".into(),
                        }),
                    },
                    TemplateMarkRef {
                        mark_id: "mark_2".into(),
                        start: None,
                        end: None,
                        tag: "fld_lawyer.ref.2".into(),
                        optional_rule: Some(OptionalFieldRule {
                            enabled: true,
                            remove_empty_prefix: String::new(),
                            remove_empty_suffix: "实习律师".into(),
                        }),
                    },
                ],
                ..Default::default()
            }],
        };
        let values = [(
            "律师".to_string(),
            Value::Array(vec![serde_json::json!({ "name": "锑", "suffix": "律师" })]),
        )]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);
        let text = extract_text(&rendered);

        assert!(text.contains("本所锑律师"));
        assert!(!text.contains("锑律师律师"));
        assert!(!text.contains("实习律师"));
    }

    #[test]
    fn renders_reference_field_from_direct_value_when_source_is_auto() {
        let xml = r#"<w:document><w:body><w:p><w:sdt><w:sdtPr><w:tag w:val="fld_principal"/></w:sdtPr><w:sdtContent><w:r><w:t>委托人</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>委托本所</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_principal".into(),
                name: "委托人".into(),
                label: "委托人".into(),
                field_type: "reference".into(),
                reference: Some(TemplateFieldReference {
                    source_mode: "auto".into(),
                    source_field: String::new(),
                    source_semantic_key: String::new(),
                    source_index: None,
                }),
                mark_refs: vec![TemplateMarkRef {
                    mark_id: "mark_1".into(),
                    start: None,
                    end: None,
                    tag: "fld_principal".into(),
                    optional_rule: None,
                }],
                ..Default::default()
            }],
        };
        let values = [(
            "委托人".to_string(),
            Value::String("安赛乐米塔尔公司".into()),
        )]
        .into_iter()
        .collect();

        let rendered = render_template_xml(xml, &manifest, &values);

        assert!(extract_text(&rendered).contains("安赛乐米塔尔公司委托本所"));
        assert!(!rendered.contains("<w:sdt"));
    }

    #[test]
    fn single_inline_party_list_has_no_separator() {
        let field = TemplateField {
            id: "fld_parties".into(),
            name: "parties".into(),
            label: "当事人".into(),
            field_type: "party_list".into(),
            ..Default::default()
        };
        let values = [(
            "parties".to_string(),
            Value::Array(vec![Value::String("张三".into())]),
        )]
        .into_iter()
        .collect();
        assert_eq!(field_value(&field, &values), "张三");
    }

    #[test]
    fn repeats_party_list_table_rows() {
        let xml = r#"<w:document><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>当事人：</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_parties"/></w:sdtPr><w:sdtContent><w:r><w:rPr><w:b/></w:rPr><w:t>张三</w:t></w:r></w:sdtContent></w:sdt></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_parties".into(),
                name: "parties".into(),
                label: "当事人".into(),
                field_type: "party_list".into(),
                ..Default::default()
            }],
        };
        let values = [(
            "parties".to_string(),
            Value::Array(vec![
                Value::String("张三".into()),
                Value::String("李四".into()),
            ]),
        )]
        .into_iter()
        .collect();
        let rendered = render_template_xml(xml, &manifest, &values);
        assert_eq!(rendered.matches("<w:tr>").count(), 2);
        assert!(rendered.contains(">张三<"));
        assert!(rendered.contains(">李四<"));
        assert!(rendered.contains("<w:b/>"));
    }

    #[test]
    fn removes_optional_prefix_when_field_is_empty() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>被告甲公司，第三人</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_third"/></w:sdtPr><w:sdtContent><w:r><w:t>占位</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>之间纠纷</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_third".into(),
                name: "third_parties".into(),
                label: "第三人".into(),
                field_type: "party_list".into(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: "，第三人".into(),
                    remove_empty_suffix: String::new(),
                }),
                ..Default::default()
            }],
        };
        let values = [("third_parties".to_string(), Value::Array(vec![]))]
            .into_iter()
            .collect();
        let rendered = render_template_xml(xml, &manifest, &values);
        assert!(rendered.contains(">被告甲公司<"));
        assert!(rendered.contains(">之间纠纷<"));
        assert!(!rendered.contains("第三人"));
        assert!(!rendered.contains("占位"));
    }

    #[test]
    fn removes_optional_role_prefix_when_field_is_empty() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>原告</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_plaintiff"/></w:sdtPr><w:sdtContent><w:r><w:t>占位</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>与被告甲公司纠纷</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_plaintiff".into(),
                name: "plaintiffs".into(),
                label: "原告".into(),
                field_type: "party_list".into(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: "原告".into(),
                    remove_empty_suffix: String::new(),
                }),
                ..Default::default()
            }],
        };
        let values = [("plaintiffs".to_string(), Value::Array(vec![]))]
            .into_iter()
            .collect();
        let rendered = render_template_xml(xml, &manifest, &values);
        assert!(rendered.contains(">与被告甲公司纠纷<"));
        assert!(!rendered.contains("原告"));
        assert!(!rendered.contains("占位"));
    }

    #[test]
    fn removes_optional_suffix_when_field_is_empty() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>委托</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_lawyer"/></w:sdtPr><w:sdtContent><w:r><w:t>占位</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师为代理人</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_lawyer".into(),
                name: "lawyer".into(),
                label: "律师".into(),
                field_type: "text".into(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: String::new(),
                    remove_empty_suffix: "律师".into(),
                }),
                ..Default::default()
            }],
        };
        let values = [("lawyer".to_string(), Value::String(String::new()))]
            .into_iter()
            .collect();
        let rendered = render_template_xml(xml, &manifest, &values);
        assert!(rendered.contains(">委托<"));
        assert!(rendered.contains(">为代理人<"));
        assert!(!rendered.contains("律师"));
        assert!(!rendered.contains("占位"));
    }

    #[test]
    fn removes_position_specific_optional_text_for_same_field() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>原告</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_party.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>占位</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>委托</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="fld_party.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>占位</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师办理</w:t></w:r></w:p></w:body></w:document>"#;
        let manifest = TemplateManifest {
            format_version: 1,
            template: TemplateMeta {
                id: "tpl".into(),
                name: "模板".into(),
                created: String::new(),
                updated: String::new(),
            },
            fields: vec![TemplateField {
                id: "fld_party".into(),
                name: "party".into(),
                label: "当事人".into(),
                field_type: "text".into(),
                mark_refs: vec![
                    TemplateMarkRef {
                        tag: "fld_party.ref.1".into(),
                        optional_rule: Some(OptionalFieldRule {
                            enabled: true,
                            remove_empty_prefix: "原告".into(),
                            remove_empty_suffix: String::new(),
                        }),
                        ..Default::default()
                    },
                    TemplateMarkRef {
                        tag: "fld_party.ref.2".into(),
                        optional_rule: Some(OptionalFieldRule {
                            enabled: true,
                            remove_empty_prefix: String::new(),
                            remove_empty_suffix: "律师".into(),
                        }),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
        };
        let values = [("party".to_string(), Value::String(String::new()))]
            .into_iter()
            .collect();
        let rendered = render_template_xml(xml, &manifest, &values);
        assert!(rendered.contains(">委托<"));
        assert!(rendered.contains(">办理<"));
        assert!(!rendered.contains("原告"));
        assert!(!rendered.contains("律师"));
        assert!(!rendered.contains("占位"));
    }
}
