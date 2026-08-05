//! Batch fill: export template fields to xlsx, validate import, batch render.

use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::engine;
use super::{RenderTemplateArgs, TemplateField, TemplateManifest};

// ── Export ──────────────────────────────────────────────────────────────────

/// Column label appended to the header row; "否" marks a sample row that must
/// not be rendered (filled from the last recorded values).
const GENERATE_COL_LABEL: &str = "是否生成";
const SAMPLE_ROW_FLAG: &str = "否";

/// Export template fields as an xlsx fill sheet.
/// Row 1 (hidden): metadata — template_id | field_id | field_type per column
/// Row 2: field labels (user-visible headers) + "是否生成" column
/// Row 3: sample row filled from the most recent recorded values (flag "否")
/// Row 4+: empty data rows
pub fn export_fields_xlsx(
    manifest: &TemplateManifest,
    default_values: &HashMap<String, serde_json::Value>,
    output_path: &str,
) -> Result<String> {
    let renderable: Vec<&TemplateField> = manifest
        .fields
        .iter()
        .filter(|f| is_renderable(&f.field_type))
        .collect();

    // Prefer the last recorded run values as the sample row, fall back to the
    // caller-provided defaults.
    let mut sample_values: HashMap<String, serde_json::Value> =
        crate::template_history::last_field_values_for_template(&manifest.template.id)
            .unwrap_or_default();
    for (field, value) in default_values {
        sample_values.entry(field.clone()).or_insert_with(|| value.clone());
    }

    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();

    // Row 0 (hidden): metadata
    ws.set_row_hidden(0)?;
    for (col, field) in renderable.iter().enumerate() {
        let col = col as u16;
        let meta = format!(
            "{}\t{}\t{}",
            manifest.template.id, field.id, field.field_type
        );
        ws.write_string(0, col, &meta)?;
    }

    // Row 1: field labels + generate flag column
    for (col, field) in renderable.iter().enumerate() {
        let col = col as u16;
        let label = if field.label.is_empty() {
            &field.name
        } else {
            &field.label
        };
        ws.write_string(1, col, label)?;
    }
    ws.write_string(1, renderable.len() as u16, GENERATE_COL_LABEL)?;

    // Row 2: sample row from the last recorded values (flag "否", not rendered)
    if !sample_values.is_empty() {
        for (col, field) in renderable.iter().enumerate() {
            if let Some(value) = sample_values.get(&field.id) {
                ws.write_string(2, col as u16, &value_to_display(value))?;
            }
        }
        ws.write_string(2, renderable.len() as u16, SAMPLE_ROW_FLAG)?;
    }

    // Auto-fit column widths (approximate)
    for (col, field) in renderable.iter().enumerate() {
        let label_len = field.label.chars().count().max(field.name.chars().count());
        let width = (label_len as f64 * 2.0 + 4.0).min(40.0).max(10.0);
        ws.set_column_width(col as u16, width)?;
    }

    let path = PathBuf::from(output_path);
    wb.save(&path)
        .with_context(|| format!("写入 Excel 失败: {}", path.display()))?;

    Ok(path.display().to_string())
}

fn is_renderable(field_type: &str) -> bool {
    !matches!(field_type, "delete_text" | "prefix" | "suffix" | "ignore")
}

fn value_to_display(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|item| match item {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Object(obj) => {
                        let text = obj
                            .get("name")
                            .or_else(|| obj.get("text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let suffix = obj.get("suffix").and_then(|v| v.as_str()).unwrap_or("");
                        format!("{text}{suffix}")
                    }
                    _ => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect();
            parts.join("、")
        }
        _ => String::new(),
    }
}

// ── Validation ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchValidationResult {
    pub valid: bool,
    pub template_id_match: bool,
    pub total_rows: usize,
    pub valid_rows: usize,
    pub errors: Vec<BatchValidationError>,
    pub warnings: Vec<BatchValidationWarning>,
    pub column_mapping: Vec<ColumnMapping>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchValidationError {
    pub row: usize,
    pub col: usize,
    pub field_name: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchValidationWarning {
    pub col: usize,
    pub field_name: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub col: usize,
    pub field_id: String,
    pub field_name: String,
    pub field_type: String,
    pub matched: bool,
}

/// Validate an imported xlsx file against the template manifest.
/// Returns validation result with errors/warnings.
pub fn validate_imported_xlsx(
    manifest: &TemplateManifest,
    xlsx_path: &str,
) -> Result<BatchValidationResult> {
    let mut workbook: Xlsx<_> =
        open_workbook(xlsx_path).with_context(|| format!("打开 Excel 失败: {xlsx_path}"))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Excel 文件没有工作表"))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| anyhow::anyhow!("读取工作表失败: {e}"))?;

    let rows: Vec<Vec<calamine::Data>> = range.rows().map(|r| r.to_vec()).collect();

    if rows.len() < 2 {
        anyhow::bail!("Excel 文件至少需要 2 行（表头 + 数据）");
    }

    // Parse metadata from row 0 (hidden row)
    let meta_row = &rows[0];
    let mut column_mapping = Vec::new();
    let mut template_id_match = true;
    let mut id_mismatch_warning: Option<String> = None;
    let mut generate_col_idx: Option<usize> = None;
    let renderable: Vec<&TemplateField> = manifest
        .fields
        .iter()
        .filter(|f| is_renderable(&f.field_type))
        .collect();

    for (col_idx, meta_cell) in meta_row.iter().enumerate() {
        let meta_str = cell_to_string(meta_cell);
        let parts: Vec<&str> = meta_str.split('\t').collect();
        if parts.len() >= 3 {
            let tpl_id = parts[0];
            let field_id = parts[1];
            let field_type = parts[2];

            if tpl_id != manifest.template.id {
                template_id_match = false;
                id_mismatch_warning.get_or_insert_with(|| {
                    "模板 ID 与当前模板不一致，已按字段名称/ID 匹配导入".to_string()
                });
            }

            let matched_field = renderable.iter().find(|f| f.id == field_id);
            column_mapping.push(ColumnMapping {
                col: col_idx,
                field_id: field_id.to_string(),
                field_name: matched_field
                    .map(|f| f.label.clone())
                    .unwrap_or_else(|| field_id.to_string()),
                field_type: field_type.to_string(),
                matched: matched_field.is_some(),
            });
        } else {
            // No metadata — try matching by label from row 1
            let label = cell_to_string(
                &rows[1]
                    .get(col_idx)
                    .cloned()
                    .unwrap_or(calamine::Data::Empty),
            );
            if label == GENERATE_COL_LABEL {
                generate_col_idx = Some(col_idx);
                continue;
            }
            let matched_field = renderable
                .iter()
                .find(|f| f.label == label || f.name == label);
            column_mapping.push(ColumnMapping {
                col: col_idx,
                field_id: matched_field.map(|f| f.id.clone()).unwrap_or_default(),
                field_name: label,
                field_type: matched_field
                    .map(|f| f.field_type.clone())
                    .unwrap_or_else(|| "text".to_string()),
                matched: matched_field.is_some(),
            });
        }
    }

    // Check for missing required fields — these are errors, not warnings
    let mapped_field_ids: Vec<&str> = column_mapping
        .iter()
        .filter(|m| m.matched)
        .map(|m| m.field_id.as_str())
        .collect();
    let mut warnings = Vec::new();
    if let Some(message) = id_mismatch_warning {
        warnings.push(BatchValidationWarning {
            col: 0,
            field_name: "模板".to_string(),
            message,
        });
    }

    // Unmatched columns warning
    for mapping in &column_mapping {
        if !mapping.matched && !mapping.field_name.is_empty() {
            warnings.push(BatchValidationWarning {
                col: mapping.col,
                field_name: mapping.field_name.clone(),
                message: format!("列“{}”未匹配到模板字段，将被忽略", mapping.field_name),
            });
        }
    }

    // Validate data rows (starting from row 2, skipping metadata row 0 and header row 1)
    let mut errors = Vec::new();

    // Missing required fields are fatal errors
    for field in &renderable {
        if field.required && !mapped_field_ids.contains(&field.id.as_str()) {
            errors.push(BatchValidationError {
                row: 0,
                col: 0,
                field_name: field.label.clone(),
                message: format!("必填字段“{}”在 Excel 中未找到对应列", field.label),
            });
        }
    }
    let mut valid_rows = 0;

    // Build field lookup for validation
    let field_by_id: HashMap<&str, &TemplateField> =
        renderable.iter().map(|f| (f.id.as_str(), *f)).collect();

    for (row_idx, row) in rows.iter().enumerate().skip(2) {
        // Skip sample rows (the "是否生成" flag is "否")
        if let Some(generate_col) = generate_col_idx {
            let flag = row
                .get(generate_col)
                .cloned()
                .unwrap_or(calamine::Data::Empty);
            if cell_to_string(&flag).trim() == SAMPLE_ROW_FLAG {
                continue;
            }
        }
        let mut row_valid = true;
        for mapping in &column_mapping {
            if !mapping.matched {
                continue;
            }
            let cell_value = row
                .get(mapping.col)
                .cloned()
                .unwrap_or(calamine::Data::Empty);
            let text = cell_to_string(&cell_value);

            // Check required fields
            if let Some(field) = field_by_id.get(mapping.field_id.as_str()) {
                if field.required && text.trim().is_empty() {
                    errors.push(BatchValidationError {
                        row: row_idx,
                        col: mapping.col,
                        field_name: field.label.clone(),
                        message: format!("必填字段“{}”为空", field.label),
                    });
                    row_valid = false;
                }
            }

            // Type-specific validation
            if !text.trim().is_empty() {
                match mapping.field_type.as_str() {
                    "date" => {
                        if !is_valid_date_text(&text) {
                            errors.push(BatchValidationError {
                                row: row_idx,
                                col: mapping.col,
                                field_name: mapping.field_name.clone(),
                                message: format!("日期格式不正确：{}", text),
                            });
                            row_valid = false;
                        }
                    }
                    _ => {}
                }
            }
        }
        // Skip fully empty rows
        let has_data = column_mapping.iter().any(|m| {
            let cell = row.get(m.col).cloned().unwrap_or(calamine::Data::Empty);
            !cell_to_string(&cell).trim().is_empty()
        });

        if has_data && row_valid {
            valid_rows += 1;
        }
    }

    let total_data_rows = rows.len().saturating_sub(2);
    Ok(BatchValidationResult {
        // ID mismatch is a warning (matched by name); only errors block import
        valid: errors.is_empty(),
        template_id_match,
        total_rows: total_data_rows,
        valid_rows,
        errors,
        warnings,
        column_mapping,
    })
}

fn is_valid_date_text(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }
    static DATE_RE1: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^\d{4}[-/.年]\d{1,2}[-/.月]\d{1,2}日?$").unwrap()
    });
    static DATE_RE2: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^\d{4}年\d{1,2}月\d{1,2}日$").unwrap()
    });
    if DATE_RE1.is_match(t) || DATE_RE2.is_match(t) {
        return true;
    }
    // Chinese date with blanks
    if t.contains('年') && t.contains('月') {
        return true;
    }
    false
}

fn cell_to_string(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Bool(b) => b.to_string(),
        calamine::Data::Error(_) => String::new(),
        calamine::Data::Empty => String::new(),
        // Excel 日期单元格：calamine 返回 DateTime 或 DateTimeIso
        calamine::Data::DateTime(dt) => {
            // ExcelDateTime 有 as_datetime() 方法返回 NaiveDateTime
            if let Some(naive) = dt.as_datetime() {
                naive.format("%Y-%m-%d").to_string()
            } else {
                // Fallback: try to get the raw f64 value
                let serial = dt.to_string();
                serial
            }
        }
        calamine::Data::DateTimeIso(s) => {
            // ISO 格式字符串，取日期部分
            s.split('T').next().unwrap_or(s).to_string()
        }
        calamine::Data::DurationIso(s) => s.clone(),
    }
}

// ── Batch Render ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchRenderResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub outputs: Vec<String>,
    pub errors: Vec<BatchRenderError>,
    /// Structured values per successfully rendered row (for history saving).
    #[serde(default)]
    pub rows: Vec<BatchRenderRow>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchRenderRow {
    pub output_path: String,
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchRenderError {
    pub row: usize,
    pub message: String,
}

/// Batch render: read xlsx, generate one docx per data row.
/// `skip_rows` contains 0-based row indices (relative to data rows starting at row 2)
/// that should be skipped due to validation errors.
pub fn batch_render(
    manifest: &TemplateManifest,
    xlsx_path: &str,
    output_dir: &str,
    template_path: &str,
    name_pattern: &str,
    skip_rows: &[usize],
    structure_overrides: &HashMap<String, super::StructureOverride>,
) -> Result<BatchRenderResult> {
    let mut workbook: Xlsx<_> =
        open_workbook(xlsx_path).with_context(|| format!("打开 Excel 失败: {xlsx_path}"))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Excel 文件没有工作表"))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| anyhow::anyhow!("读取工作表失败: {e}"))?;

    let rows: Vec<Vec<calamine::Data>> = range.rows().map(|r| r.to_vec()).collect();
    if rows.len() < 3 {
        anyhow::bail!("Excel 文件至少需要 3 行（元数据 + 表头 + 数据）");
    }

    // Parse column mapping from metadata row
    let meta_row = &rows[0];
    let renderable: Vec<&TemplateField> = manifest
        .fields
        .iter()
        .filter(|f| is_renderable(&f.field_type))
        .collect();

    let mut col_map: Vec<(usize, &TemplateField)> = Vec::new();
    let mut generate_col_idx: Option<usize> = None;
    for (col_idx, meta_cell) in meta_row.iter().enumerate() {
        let meta_str = cell_to_string(meta_cell);
        let parts: Vec<&str> = meta_str.split('\t').collect();
        if parts.len() >= 3 {
            let field_id = parts[1];
            if let Some(field) = renderable.iter().find(|f| f.id == field_id) {
                col_map.push((col_idx, field));
            }
        } else {
            let label = cell_to_string(
                &rows[1]
                    .get(col_idx)
                    .cloned()
                    .unwrap_or(calamine::Data::Empty),
            );
            if label == GENERATE_COL_LABEL {
                generate_col_idx = Some(col_idx);
            }
        }
    }

    if col_map.is_empty() {
        anyhow::bail!("未能从 Excel 元数据行匹配到任何字段");
    }

    let output_dir_path = Path::new(output_dir);
    std::fs::create_dir_all(output_dir_path)?;

    let mut result = BatchRenderResult {
        total: 0,
        success: 0,
        failed: 0,
        outputs: Vec::new(),
        errors: Vec::new(),
        rows: Vec::new(),
    };

    for (row_idx, row) in rows.iter().enumerate().skip(2) {
        // Skip sample rows (the "是否生成" flag is "否")
        if let Some(generate_col) = generate_col_idx {
            let flag = row
                .get(generate_col)
                .cloned()
                .unwrap_or(calamine::Data::Empty);
            if cell_to_string(&flag).trim() == SAMPLE_ROW_FLAG {
                continue;
            }
        }
        // Check if row has any data
        let has_data = col_map.iter().any(|(col, _)| {
            let cell = row.get(*col).cloned().unwrap_or(calamine::Data::Empty);
            !cell_to_string(&cell).trim().is_empty()
        });
        if !has_data {
            continue;
        }

        // Skip rows that had validation errors
        if skip_rows.contains(&row_idx) {
            continue;
        }

        result.total += 1;

        // Build values HashMap from this row
        let values = build_row_values(row, &col_map, manifest);

        // Generate output filename
        let filename = generate_filename(name_pattern, &values, manifest, result.success + 1);
        let output_path = unique_output_path(output_dir_path, &filename);

        let args = RenderTemplateArgs {
            template_path: template_path.to_string(),
            output_path: output_path.display().to_string(),
            values,
            structure_overrides: structure_overrides.clone(),
        };

        match engine::render_docx(args, "batch") {
            Ok(path) => {
                result.outputs.push(path.clone());
                result.rows.push(BatchRenderRow {
                    output_path: path,
                    values: build_row_values(row, &col_map, manifest),
                });
                result.success += 1;
            }
            Err(e) => {
                result.errors.push(BatchRenderError {
                    row: row_idx,
                    message: e.to_string(),
                });
                result.failed += 1;
            }
        }
    }

    Ok(result)
}

fn build_row_values(
    row: &[calamine::Data],
    col_map: &[(usize, &TemplateField)],
    manifest: &TemplateManifest,
) -> HashMap<String, serde_json::Value> {
    let mut values = HashMap::new();

    for (col, field) in col_map {
        let cell = row.get(*col).cloned().unwrap_or(calamine::Data::Empty);
        let text = cell_to_string(&cell);

        let value = match field.field_type.as_str() {
            "party_list" => {
                let items: Vec<serde_json::Value> = text
                    .split('、')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect();
                serde_json::Value::Array(items)
            }
            "date" => serde_json::Value::String(text.trim().to_string()),
            _ => serde_json::Value::String(text.trim().to_string()),
        };

        values.insert(field.id.clone(), value.clone());
        if !field.name.is_empty() {
            values.insert(field.name.clone(), value);
        }
    }

    // Add semantic key aliases
    let semantic_pairs: Vec<(String, serde_json::Value)> = manifest
        .fields
        .iter()
        .filter(|f| is_renderable(&f.field_type))
        .filter_map(|f| {
            let sk = f.semantic_key.trim();
            if !sk.is_empty() && sk != f.name {
                values.get(&f.id).map(|v| (sk.to_string(), v.clone()))
            } else {
                None
            }
        })
        .collect();
    for (sk, v) in semantic_pairs {
        values.entry(sk).or_insert(v);
    }

    values
}

fn generate_filename(
    pattern: &str,
    values: &HashMap<String, serde_json::Value>,
    manifest: &TemplateManifest,
    index: usize,
) -> String {
    if pattern.is_empty() {
        return format!("{}-{}.docx", manifest.template.name, index);
    }

    let mut result = pattern.to_string();

    // Replace {N} with field values by index
    let renderable: Vec<&TemplateField> = manifest
        .fields
        .iter()
        .filter(|f| is_renderable(&f.field_type))
        .collect();

    for (i, field) in renderable.iter().enumerate() {
        let placeholder = format!("{{{}}}", i + 1);
        if result.contains(&placeholder) {
            let val = values
                .get(&field.id)
                .map(value_to_display)
                .unwrap_or_default();
            result = result.replace(&placeholder, &val);
        }
    }

    // Replace {fieldName} with field values by name
    for field in &renderable {
        let placeholder = format!("{{{}}}", field.name);
        if result.contains(&placeholder) {
            let val = values
                .get(&field.id)
                .map(value_to_display)
                .unwrap_or_default();
            result = result.replace(&placeholder, &val);
        }
    }

    // Clean up filename
    let clean: String = result
        .chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect();

    if clean.trim().is_empty() || clean == ".docx" {
        format!("{}-{}.docx", manifest.template.name, index)
    } else if clean.ends_with(".docx") {
        clean
    } else {
        format!("{clean}.docx")
    }
}

fn unique_output_path(dir: &Path, filename: &str) -> PathBuf {
    let path = dir.join(filename);
    if !path.exists() {
        return path;
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    for i in 1..=9999 {
        let candidate = dir.join(format!("{stem}-{i}.docx"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}
