use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

use crate::docx_template::package;

use super::{
    fnv1a_hash, is_word_xml_part, unique_docx_output_path, RenderTemplateArgs, SaveTemplateArgs,
    SaveTemplateResult, TemplateInspection, TemplateManifest, TemplateMark, TemplateMeta,
    TemplateTextRun,
};
use super::{render, save, scan};

#[cfg(test)]
use super::{TemplateField, TemplateMarkRef};

/// Inspect a docx template using the quick-xml engine
pub fn inspect_docx(path: &str) -> Result<TemplateInspection> {
    let input_path = std::path::Path::new(path);
    let mut _converted_guard = None;
    let (docx_path, converted_path) = if path.to_lowercase().ends_with(".doc") {
        let converted = convert_doc_to_docx(input_path)?;
        let display = converted.display().to_string();
        _converted_guard = Some(TempPathGuard::new(converted.clone()));
        (converted, Some(display))
    } else {
        (input_path.to_path_buf(), None)
    };

    let pkg = package::read_docx_package(&docx_path)?;
    let (document_runs, marks, document_text) = scan_package_to_runs_and_marks(&pkg)?;

    let checkbox_like_count = marks.iter().filter(|m| m.checkbox_like).count();
    Ok(TemplateInspection {
        input: path.to_string(),
        converted_path,
        document_text,
        document_runs,
        summary: super::InspectionSummary {
            mark_count: marks.len(),
            checkbox_like_count,
        },
        marks,
    })
}

/// Save a docx template using the quick-xml engine
pub fn save_docx(args: SaveTemplateArgs) -> Result<SaveTemplateResult> {
    let mut converted_guard = None;
    let source = if args.source_docx.to_lowercase().ends_with(".doc") {
        let converted = convert_doc_to_docx(std::path::Path::new(&args.source_docx))?;
        converted_guard = Some(TempPathGuard::new(converted.clone()));
        converted
    } else {
        std::path::PathBuf::from(&args.source_docx)
    };
    let output = unique_docx_output_path(std::path::Path::new(&args.output_path))?;

    let mut manifest = TemplateManifest {
        // The application version is 0.8.x; the package schema remains v2.
        // It is still the same manifest shape, with stable per-mark tags.
        format_version: 2,
        template: TemplateMeta {
            id: format!("tpl_{:016x}", fnv1a_hash(&output.display().to_string())),
            name: args.template_name,
            created: chrono::Utc::now().to_rfc3339(),
            updated: chrono::Utc::now().to_rfc3339(),
        },
        fields: args.fields,
    };

    let pkg = package::read_docx_package(&source)?;
    ensure_template_package_safe(&pkg)?;
    let (runs, marks, _) = scan_package_to_runs_and_marks(&pkg)?;
    prune_stray_punctuation_refs(&mut manifest.fields, &runs);
    super::normalize_manifest_options(&mut manifest);
    super::sanitize_manifest_private_labels(&mut manifest, &marks);
    super::validate_manifest(&manifest)?;
    let xml_parts: Vec<(String, Vec<u8>)> = pkg
        .iter()
        .filter(|(name, _)| is_word_xml_part(name))
        .map(|(name, data)| (name.clone(), data.clone()))
        .collect();

    let scan_parts: Vec<(&str, &[u8])> = xml_parts
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let index = scan::scan_package_index_to_document_index(&scan_parts)?;
    let template_docx_parts = save::build_template_docx(&xml_parts, &manifest.fields, &index)?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build the output package: manifest + all XML parts + binary parts
    let mut out_pkg = HashMap::new();
    for (part_name, xml_data) in template_docx_parts {
        out_pkg.insert(part_name, xml_data);
    }
    for (name, data) in &pkg {
        if name == "manifest.json" || !copy_template_package_entry(name) {
            continue;
        }
        if !is_word_xml_part(name) && !out_pkg.contains_key(name) {
            out_pkg.insert(name.clone(), data.clone());
        }
    }

    package::write_docsytpl_package(&output, &manifest, &out_pkg)?;
    drop(converted_guard);
    Ok(SaveTemplateResult {
        output_path: output.display().to_string(),
        manifest,
    })
}

/// A text field may span several Word runs when its visible text uses multiple
/// character formats.  A separately selected colon, comma, or bracket is not
/// part of that field value, though: keeping it as a second reference causes
/// rendering to clear the punctuation after it writes the field value into the
/// first reference.  Keep a standalone punctuation field intact, but remove
/// accidental punctuation-only references from a multi-reference value field.
fn prune_stray_punctuation_refs(fields: &mut [super::TemplateField], runs: &[TemplateTextRun]) {
    let run_text: HashMap<&str, &str> = runs
        .iter()
        .map(|run| (run.id.as_str(), run.text.as_str()))
        .collect();

    for field in fields {
        if field.mark_refs.len() < 2 || is_structural_field_type(&field.field_type) {
            continue;
        }

        let removable: HashSet<String> = field
            .mark_refs
            .iter()
            .filter(|reference| {
                let Some(text) = run_text.get(reference.mark_id.as_str()) else {
                    return false;
                };
                reference.start.is_none() && reference.end.is_none() && is_punctuation_only(text)
            })
            .map(|reference| reference.mark_id.clone())
            .collect();
        if removable.is_empty() || removable.len() == field.mark_refs.len() {
            continue;
        }

        field
            .mark_refs
            .retain(|reference| !removable.contains(&reference.mark_id));
        field.marks.retain(|mark_id| !removable.contains(mark_id));
    }
}

fn is_structural_field_type(field_type: &str) -> bool {
    matches!(
        field_type,
        "prefix" | "suffix" | "connector" | "delete_text" | "ignore"
    )
}

fn is_punctuation_only(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed.chars().all(|ch| {
            matches!(
                ch,
                '：' | ':'
                    | '，'
                    | ','
                    | '、'
                    | '。'
                    | '；'
                    | ';'
                    | '（'
                    | '）'
                    | '('
                    | ')'
                    | '【'
                    | '】'
                    | '['
                    | ']'
                    | '《'
                    | '》'
            )
        })
}

/// Render a docx template using the quick-xml engine
pub fn render_docx(args: RenderTemplateArgs) -> Result<String> {
    let template_path = std::path::Path::new(&args.template_path);
    let output_path = unique_docx_output_path(std::path::Path::new(&args.output_path))?;
    let (manifest, pkg) = package::read_docsytpl_package(template_path)?;
    ensure_template_package_safe(&pkg)?;

    let xml_parts: Vec<(String, Vec<u8>)> = pkg
        .iter()
        .filter(|(name, _)| is_word_xml_part(name))
        .map(|(name, data)| (name.clone(), data.clone()))
        .collect();

    let rendered_parts = render::render_docx(
        &xml_parts,
        &manifest,
        &args.values,
        &args.structure_overrides,
    )?;

    let mut out_pkg = HashMap::new();
    for (part_name, xml_data) in rendered_parts {
        out_pkg.insert(part_name, xml_data);
    }
    for (name, data) in &pkg {
        if name == "manifest.json" || !copy_template_package_entry(name) {
            continue;
        }
        if !is_word_xml_part(name) && !out_pkg.contains_key(name) {
            out_pkg.insert(name.clone(), data.clone());
        }
    }

    package::write_docx_package(&output_path, &out_pkg)?;

    let output_path_str = output_path.display().to_string();
    // History recording is best-effort; file is already written
    let _ = crate::template_history::record_generation(
        &args.template_path,
        &manifest,
        &output_path_str,
        &args.values,
    );
    Ok(output_path_str)
}

fn copy_template_package_entry(name: &str) -> bool {
    !name.starts_with("docProps/")
}

fn ensure_template_package_safe(pkg: &HashMap<String, Vec<u8>>) -> Result<()> {
    const UNSUPPORTED_SENSITIVE_PREFIXES: &[&str] = &[
        "word/comments",
        "customXml/",
        "word/embeddings/",
        "word/vbaProject",
        "_xmlsignatures/",
    ];
    if let Some(name) = pkg.keys().find(|name| {
        UNSUPPORTED_SENSITIVE_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
    }) {
        anyhow::bail!(
            "模板包含无法安全保留的批注、嵌入对象、宏或签名文件（{}）。请在 Word 中移除后重新导入",
            name
        );
    }
    Ok(())
}

/// Convert old .doc to .docx using office_oxide
fn convert_doc_to_docx(path: &std::path::Path) -> Result<std::path::PathBuf> {
    let doc = office_oxide::Document::open(path.display().to_string())
        .with_context(|| format!("无法读取旧版 .doc 文件: {}", path.display()))?;
    let output = unique_docx_output_path(
        &std::env::temp_dir()
            .join(format!(
                "docsy-template-{:016x}",
                fnv1a_hash(&path.display().to_string())
            ))
            .with_extension("docx"),
    )?;
    doc.save_as(output.display().to_string())
        .with_context(|| format!("转换 .doc → .docx 失败: {}", path.display()))?;
    Ok(output)
}

struct TempPathGuard {
    path: std::path::PathBuf,
}

impl TempPathGuard {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Scan all XML parts and produce runs + marks for the Tauri inspect response
fn scan_package_to_runs_and_marks(
    pkg: &HashMap<String, Vec<u8>>,
) -> Result<(Vec<TemplateTextRun>, Vec<TemplateMark>, String)> {
    use crate::docx_template::scan;

    let xml_parts: Vec<_> = pkg
        .iter()
        .filter(|(name, _)| is_word_xml_part(name))
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();

    let doc_index = scan::scan_package_index_to_document_index(&xml_parts)?;

    let mut runs = Vec::new();
    let mut marks = Vec::new();
    let mut flat_text = String::new();

    for (part_name, part_index) in &doc_index.parts {
        for node in &part_index.nodes {
            let id = format!(
                "{}-p{}-r{}",
                part_name, node.paragraph_index, node.run_index
            );
            runs.push(TemplateTextRun {
                id: id.clone(),
                part: part_name.clone(),
                run_index: node.run_index,
                paragraph_index: node.paragraph_index,
                text: node.text.clone(),
                paragraph_text: String::new(),
                checkbox_like: node.checkbox_like,
                option_label: node.option_label.clone(),
                highlighted: node.highlighted,
                bold: node.bold,
                italic: node.italic,
                underline: node.underline,
            });

            if node.highlighted {
                marks.push(TemplateMark {
                    id: id.clone(),
                    part: part_name.clone(),
                    run_index: node.run_index,
                    text: node.text.clone(),
                    context: String::new(),
                    checkbox_like: node.checkbox_like,
                    option_label: node.option_label.clone(),
                });
            }

            if !flat_text.is_empty() {
                flat_text.push('\n');
            }
            flat_text.push_str(&node.text);
        }
    }

    Ok((runs, marks, flat_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_minimal_docx() -> Result<std::path::PathBuf> {
        let dir = std::env::temp_dir().join(format!("docsy-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;

        let docx_path = dir.join("test.docx");

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:rPr><w:highlight w:val="yellow"/><w:b/></w:rPr>
        <w:t>张三</w:t>
      </w:r>
    </w:p>
    <w:p>
      <w:r><w:t>案件编号：</w:t></w:r>
      <w:r>
        <w:rPr><w:highlight w:val="yellow"/></w:rPr>
        <w:t>(2026)沪01民初100号</w:t>
      </w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:rPr><w:highlight w:val="yellow"/></w:rPr>
        <w:t>原告方</w:t>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

        let file = std::fs::File::create(&docx_path)?;
        let mut writer = zip::ZipWriter::new(file);
        let opts =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        writer.start_file("[Content_Types].xml", opts)?;
        writer.write_all(content_types.as_bytes())?;
        writer.start_file("_rels/.rels", opts)?;
        writer.write_all(rels.as_bytes())?;
        writer.start_file("word/document.xml", opts)?;
        writer.write_all(doc_xml.as_bytes())?;
        writer.finish()?;

        Ok(docx_path)
    }

    #[test]
    fn end_to_end_inspect_save_render() {
        let docx_path = create_minimal_docx().unwrap();
        let output_dir = std::env::temp_dir().join(format!("docsy-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&output_dir).unwrap();

        // 1. Inspect the docx
        let inspection = inspect_docx(&docx_path.display().to_string()).unwrap();

        assert_eq!(
            inspection.marks.len(),
            3,
            "should find 3 yellow-highlighted marks"
        );
        assert_eq!(inspection.summary.mark_count, 3);

        // Verify coordinate-based mark IDs
        for mark in &inspection.marks {
            assert!(
                mark.id.contains("-p"),
                "mark id should be coordinate-based: {}",
                mark.id
            );
            assert!(
                mark.id.contains("-r"),
                "mark id should include run index: {}",
                mark.id
            );
        }

        // 2. Create field definitions for the marks
        let name_mark = inspection.marks.iter().find(|m| m.text == "张三").unwrap();
        let case_mark = inspection
            .marks
            .iter()
            .find(|m| m.text.contains("沪01民初"))
            .unwrap();

        let fields = vec![
            TemplateField {
                id: "name".to_string(),
                name: "姓名".to_string(),
                label: "姓名".to_string(),
                field_type: "text".to_string(),
                marks: vec![name_mark.id.clone()],
                mark_refs: vec![TemplateMarkRef {
                    mark_id: name_mark.id.clone(),
                    tag: "name".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            TemplateField {
                id: "case_no".to_string(),
                name: "案号".to_string(),
                label: "案号".to_string(),
                field_type: "text".to_string(),
                marks: vec![case_mark.id.clone()],
                mark_refs: vec![TemplateMarkRef {
                    mark_id: case_mark.id.clone(),
                    tag: "case_no".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ];

        // 3. Save as template
        let tpl_path = output_dir.join("test.docsytpl");
        let save_args = SaveTemplateArgs {
            source_docx: docx_path.display().to_string(),
            output_path: tpl_path.display().to_string(),
            template_name: "E2E Test".to_string(),
            fields: fields.clone(),
        };
        let saved = save_docx(save_args).unwrap();
        assert_eq!(saved.manifest.format_version, 2);
        let (_, template_package) =
            package::read_docsytpl_package(std::path::Path::new(&saved.output_path)).unwrap();
        let template_xml =
            String::from_utf8_lossy(template_package.get("word/document.xml").unwrap());
        assert!(
            !template_xml.contains("张三"),
            "saved template must not retain marked private text"
        );
        assert!(
            !template_xml.contains("沪01民初100号"),
            "saved template must not retain marked private text"
        );

        // 4. Render template with values
        let mut values = HashMap::new();
        values.insert(
            "name".to_string(),
            serde_json::Value::String("新姓名".to_string()),
        );
        values.insert(
            "case_no".to_string(),
            serde_json::Value::String("(2026)沪01民初999号".to_string()),
        );

        let render_args = RenderTemplateArgs {
            template_path: saved.output_path.clone(),
            output_path: output_dir.join("output.docx").display().to_string(),
            values,
            structure_overrides: HashMap::new(),
        };
        let output_path = render_docx(render_args).unwrap();

        // 5. Verify rendered output
        let rendered_pkg = package::read_docx_package(std::path::Path::new(&output_path)).unwrap();
        let doc_xml = rendered_pkg.get("word/document.xml").unwrap();
        let doc_str = String::from_utf8_lossy(doc_xml);

        assert!(
            doc_str.contains("新姓名"),
            "rendered name field value missing"
        );
        assert!(
            doc_str.contains("沪01民初999号"),
            "rendered case number missing"
        );
        assert!(
            !doc_str.contains("张三"),
            "name field original text replaced"
        );
        assert!(
            !doc_str.contains("沪01民初100号"),
            "case_no field original text replaced"
        );
        assert!(
            doc_str.contains("<w:b"),
            "bold format preserved in template"
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&output_dir);
        let _ = std::fs::remove_dir_all(docx_path.parent().unwrap());
    }

    #[test]
    fn ignores_accidental_punctuation_ref_in_multi_run_value_field() {
        let mut fields = vec![TemplateField {
            id: "court".to_string(),
            name: "法院".to_string(),
            label: "法院".to_string(),
            field_type: "text".to_string(),
            marks: vec![
                "word/document.xml-p0-r0".to_string(),
                "word/document.xml-p0-r1".to_string(),
            ],
            mark_refs: vec![
                TemplateMarkRef {
                    mark_id: "word/document.xml-p0-r0".to_string(),
                    tag: "court.ref.1".to_string(),
                    ..Default::default()
                },
                TemplateMarkRef {
                    mark_id: "word/document.xml-p0-r1".to_string(),
                    tag: "court.ref.2".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }];
        let runs = vec![
            TemplateTextRun {
                id: "word/document.xml-p0-r0".to_string(),
                part: "word/document.xml".to_string(),
                run_index: 0,
                paragraph_index: 0,
                text: "北京知识产权法院".to_string(),
                paragraph_text: String::new(),
                checkbox_like: false,
                option_label: String::new(),
                highlighted: true,
                bold: false,
                italic: false,
                underline: false,
            },
            TemplateTextRun {
                id: "word/document.xml-p0-r1".to_string(),
                part: "word/document.xml".to_string(),
                run_index: 1,
                paragraph_index: 0,
                text: "：".to_string(),
                paragraph_text: String::new(),
                checkbox_like: false,
                option_label: String::new(),
                highlighted: false,
                bold: false,
                italic: false,
                underline: false,
            },
        ];

        prune_stray_punctuation_refs(&mut fields, &runs);
        assert_eq!(fields[0].mark_refs.len(), 1);
        assert_eq!(fields[0].marks, ["word/document.xml-p0-r0"]);
    }

    #[test]
    fn end_to_end_table_party_list_replication() {
        let docx_path = create_table_docx().unwrap();
        let output_dir =
            std::env::temp_dir().join(format!("docsy-e2e-table-{}", std::process::id()));
        std::fs::create_dir_all(&output_dir).unwrap();

        // 1. Inspect
        let inspection = inspect_docx(&docx_path.display().to_string()).unwrap();
        assert!(
            !inspection.marks.is_empty(),
            "should find party_list marks in table"
        );

        let party_mark = inspection
            .marks
            .iter()
            .find(|m| m.text == "原告方")
            .unwrap();
        let date_mark = inspection.marks.iter().find(|m| m.text == "2026").unwrap();

        // 2. Define fields
        let fields = vec![
            TemplateField {
                id: "parties".to_string(),
                name: "当事人".to_string(),
                label: "当事人".to_string(),
                field_type: "party_list".to_string(),
                marks: vec![party_mark.id.clone()],
                mark_refs: vec![TemplateMarkRef {
                    mark_id: party_mark.id.clone(),
                    tag: "parties".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            TemplateField {
                id: "case_date".to_string(),
                name: "日期".to_string(),
                label: "日期".to_string(),
                field_type: "text".to_string(),
                marks: vec![date_mark.id.clone()],
                mark_refs: vec![TemplateMarkRef {
                    mark_id: date_mark.id.clone(),
                    tag: "case_date".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ];

        // 3. Save
        let tpl_path = output_dir.join("table.docsytpl");
        let saved = save_docx(SaveTemplateArgs {
            source_docx: docx_path.display().to_string(),
            output_path: tpl_path.display().to_string(),
            template_name: "Table E2E".to_string(),
            fields,
        })
        .unwrap();
        assert_eq!(saved.manifest.format_version, 2);

        // 4. Render — party_list with 3 items should replicate 3 rows
        let mut values = HashMap::new();
        values.insert(
            "parties".to_string(),
            serde_json::json!([
                { "name": "张三", "suffix": "" },
                { "name": "李四", "suffix": "" },
                { "name": "王五", "suffix": "" }
            ]),
        );
        values.insert(
            "case_date".to_string(),
            serde_json::Value::String("2026年7月20日".to_string()),
        );

        let output_path = render_docx(RenderTemplateArgs {
            template_path: saved.output_path.clone(),
            output_path: output_dir.join("rendered.docx").display().to_string(),
            values,
            structure_overrides: HashMap::new(),
        })
        .unwrap();

        let rendered_pkg = package::read_docx_package(std::path::Path::new(&output_path)).unwrap();
        let doc_str = String::from_utf8_lossy(rendered_pkg.get("word/document.xml").unwrap());

        assert!(doc_str.contains("张三"), "party item 1 rendered");
        assert!(doc_str.contains("李四"), "party item 2 rendered");
        assert!(doc_str.contains("王五"), "party item 3 rendered");
        assert!(doc_str.contains("2026年7月20日"), "date field rendered");
        assert!(
            !doc_str.contains("原告方"),
            "party_list placeholder replaced"
        );

        let _ = std::fs::remove_dir_all(&output_dir);
        let _ = std::fs::remove_dir_all(docx_path.parent().unwrap());
    }

    fn create_table_docx() -> Result<std::path::PathBuf> {
        let dir = std::env::temp_dir().join(format!("docsy-test-table-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let docx_path = dir.join("test_table.docx");

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>日期：</w:t></w:r>
      <w:r>
        <w:rPr><w:highlight w:val="yellow"/></w:rPr>
        <w:t>2026</w:t>
      </w:r>
      <w:r><w:t>年</w:t></w:r>
    </w:p>
    <w:tbl>
      <w:tr>
        <w:tc><w:p>
          <w:r>
            <w:rPr><w:highlight w:val="yellow"/></w:rPr>
            <w:t>原告方</w:t>
          </w:r>
        </w:p></w:tc>
        <w:tc><w:p>
          <w:r><w:t>诉讼地位</w:t></w:r>
        </w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

        let file = std::fs::File::create(&docx_path)?;
        let mut writer = zip::ZipWriter::new(file);
        let opts =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        writer.start_file("[Content_Types].xml", opts)?;
        writer.write_all(content_types.as_bytes())?;
        writer.start_file("_rels/.rels", opts)?;
        writer.write_all(rels.as_bytes())?;
        writer.start_file("word/document.xml", opts)?;
        writer.write_all(doc_xml.as_bytes())?;
        writer.finish()?;

        Ok(docx_path)
    }
}
