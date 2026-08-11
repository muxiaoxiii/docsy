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
        sample_values
            .entry(field.clone())
            .or_insert_with(|| value.clone());
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
                ws.write_string(2, col as u16, value_to_display(value))?;
            }
        }
        ws.write_string(2, renderable.len() as u16, SAMPLE_ROW_FLAG)?;
    }

    // Auto-fit column widths (approximate)
    for (col, field) in renderable.iter().enumerate() {
        let label_len = field.label.chars().count().max(field.name.chars().count());
        let width = (label_len as f64 * 2.0 + 4.0).clamp(10.0, 40.0);
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
            if !text.trim().is_empty() && mapping.field_type == "date" && !is_valid_date_text(&text)
            {
                errors.push(BatchValidationError {
                    row: row_idx,
                    col: mapping.col,
                    field_name: mapping.field_name.clone(),
                    message: format!("日期格式不正确：{}", text),
                });
                row_valid = false;
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
    static DATE_RE2: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^\d{4}年\d{1,2}月\d{1,2}日$").unwrap());
    if DATE_RE1.is_match(t) || DATE_RE2.is_match(t) {
        return true;
    }
    // Chinese date with blanks
    if t.contains('年') && t.contains('月') {
        return true;
    }
    false
}

// ── 日期格式化（与前端 fieldRowUtils.js formatDateValue 规则一致）─────────────

const CN_DIGITS: &[char] = &['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
const EN_MONTHS: &[&str] = &[
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const EN_MONTHS_SHORT: &[&str] = &[
    "Jan.", "Feb.", "Mar.", "Apr.", "May", "Jun.", "Jul.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
];

/// 解析日期输入为 (年, 月, 日)；任一部分为 0 表示"留空"。
/// 支持 20260805、2026-08-05、2026/8/5、2026.8.5、2026年8月5日、"留空"；
/// 无法解析返回 None（调用方原样透传，不报错）。
fn parse_date_parts(value: &str) -> Option<(u32, u32, u32)> {
    let s = value.trim();
    if s.is_empty() {
        return None;
    }
    if s == "留空" {
        return Some((0, 0, 0));
    }
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() == 8 && compact.chars().all(|c| c.is_ascii_digit()) {
        let y = compact[0..4].parse().ok()?;
        let m = compact[4..6].parse().ok()?;
        let d = compact[6..8].parse().ok()?;
        return Some((y, m, d));
    }
    let parts: Vec<&str> = compact
        .split(['-', '/', '年', '月', '日', '.'])
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() >= 3 {
        let y = parts[0].parse().ok()?;
        let m = parts[1].parse().ok()?;
        let d = parts[2].parse().ok()?;
        return Some((y, m, d));
    }
    // 裸数字不足 8 位或段数不够：含义不明，按无法解析处理
    None
}

fn cn_number(n: u32) -> String {
    if n == 0 {
        return String::new();
    }
    n.to_string()
        .chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|d| CN_DIGITS.get(d as usize).copied())
                .unwrap_or(c)
        })
        .collect()
}

fn en_ordinal(n: u32) -> String {
    let rem100 = n % 100;
    if (11..=13).contains(&rem100) {
        return format!("{n}th");
    }
    match n % 10 {
        1 => format!("{n}st"),
        2 => format!("{n}nd"),
        3 => format!("{n}rd"),
        _ => format!("{n}th"),
    }
}

/// 按指定格式渲染日期值，规则与前端 formatDateValue 一致：
/// 任一部分为 0 时该段留空（blank 即全留空，供打印后手写）。
fn format_date_value(value: &str, format: &str) -> String {
    let Some((y, m, d)) = parse_date_parts(value) else {
        return value.to_string();
    };
    let fmt = if format.is_empty() { "iso" } else { format };
    let y_str = if y > 0 { y.to_string() } else { "    ".to_string() };
    let m_str = if m > 0 { m.to_string() } else { "  ".to_string() };
    let d_str = if d > 0 { d.to_string() } else { "  ".to_string() };
    if fmt == "cn" || fmt == "blank" {
        return format!("{y_str}年{m_str}月{d_str}日");
    }
    if fmt == "cn_full" {
        return format!(
            "{}年{}月{}日",
            cn_number(y),
            if m > 0 { cn_number(m) } else { "  ".to_string() },
            if d > 0 { cn_number(d) } else { "  ".to_string() }
        );
    }
    let month_long = if (1..=12).contains(&m) {
        EN_MONTHS[(m - 1) as usize]
    } else {
        " "
    };
    let month_short = if (1..=12).contains(&m) {
        EN_MONTHS_SHORT[(m - 1) as usize]
    } else {
        " "
    };
    // 对应前端的 .replace(/\s+/g, ' ').trim()
    let squash = |s: String| s.split_whitespace().collect::<Vec<_>>().join(" ");
    if fmt == "en_long" {
        return squash(format!("{month_long} {d_str}, {y_str}"));
    }
    if fmt == "en_short" {
        return squash(format!("{month_short} {d_str}, {y_str}"));
    }
    if fmt == "en_dmy" {
        return squash(format!("{d_str} {month_long} {y_str}"));
    }
    if fmt == "en_ordinal" {
        let d_ord = if d > 0 { en_ordinal(d) } else { String::new() };
        return squash(format!("{y_str} {month_long} {d_ord}"));
    }
    // iso 默认：2026-8-5（留空部分保持空白段）
    format!("{y_str}-{m_str}-{d_str}").trim().to_string()
}

fn cell_to_string(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => {
            // 仅在 i64 可精确表示的范围内（|f| <= 2^53）按整数输出；
            // 超出范围时 `as i64` 会静默饱和成 i64::MAX，改用原始浮点格式
            if f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_992.0 {
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
                // Fallback: preserve calamine's raw representation.
                dt.to_string()
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
// This command-facing API mirrors the independently configurable batch options.
// Grouping them would require a coordinated Tauri payload migration.
#[allow(clippy::too_many_arguments)]
pub fn batch_render(
    manifest: &TemplateManifest,
    xlsx_path: &str,
    output_dir: &str,
    template_path: &str,
    name_pattern: &str,
    skip_rows: &[usize],
    structure_overrides: &HashMap<String, super::StructureOverride>,
    item_separator: &str,
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
            item_separator: item_separator.to_string(),
            template_path: template_path.to_string(),
            output_path: output_path.display().to_string(),
            values,
            history_values: None,
            structure_overrides: structure_overrides.clone(),
        };

        match engine::render_docx(args, "") {
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
            "checkbox" => {
                let trimmed = text.trim().to_lowercase();
                // 与 scan.rs CHECKBOX_CHARS 对齐："√" 和 "☒"（带叉勾选框）都算勾选
                serde_json::Value::Bool(matches!(
                    trimmed.as_str(),
                    "true" | "1" | "是" | "yes" | "☑" | "☒" | "✓" | "✔" | "√"
                ))
            }
            "radio_group" | "select" => {
                // Match against field options by label or id
                let trimmed = text.trim();
                let matched_option = field
                    .options
                    .iter()
                    .find(|opt| opt.label == trimmed || opt.id == trimmed);
                match matched_option {
                    Some(opt) => serde_json::Value::String(opt.id.clone()),
                    None => serde_json::Value::String(trimmed.to_string()),
                }
            }
            "checkbox_group" => {
                // Parse multiple selections separated by 、 or ,
                let items: Vec<serde_json::Value> = text
                    .split(['、', ','])
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        // Match against options by label
                        let matched = field.options.iter().find(|opt| opt.label == s);
                        match matched {
                            Some(opt) => serde_json::Value::String(opt.id.clone()),
                            None => serde_json::Value::String(s.to_string()),
                        }
                    })
                    .collect();
                serde_json::Value::Array(items)
            }
            // date 字段带 date_format 时按前端 formatDateValue 同款规则格式化；
            // 未设置或无法解析时原样透传
            "date" => {
                let trimmed = text.trim();
                if field.date_format.is_empty() {
                    serde_json::Value::String(trimmed.to_string())
                } else {
                    serde_json::Value::String(format_date_value(trimmed, &field.date_format))
                }
            }
            // reference, marker, prefix, suffix, text, and others: store as string
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
    // If manifest has filenameTemplate, use it
    if let Some(ref ft) = manifest.filename_template {
        if !ft.tokens.is_empty() {
            crate::app_log::debug(
                "batch",
                "使用 filenameTemplate 生成文件名",
                serde_json::json!({ "index": index, "token_count": ft.tokens.len() }),
            );
            let sep = if ft.separator.is_empty() {
                "-".to_string()
            } else {
                ft.separator.clone()
            };
            let parts: Vec<String> = ft
                .tokens
                .iter()
                .map(|token| match token.token_type.as_str() {
                    "literal" => token.value.clone(),
                    "field" => {
                        let field = manifest.fields.iter().find(|f| f.name == token.value);
                        if let Some(f) = field {
                            values.get(&f.id).map(value_to_display).unwrap_or_default()
                        } else {
                            values
                                .get(&token.value)
                                .map(value_to_display)
                                .unwrap_or_default()
                        }
                    }
                    "preset" => match token.value.as_str() {
                        "日期" => chrono::Local::now().format("%Y%m%d").to_string(),
                        "日期-" => chrono::Local::now().format("%Y-%m-%d").to_string(),
                        "日期短" => chrono::Local::now().format("%m%d").to_string(),
                        "模板名" => manifest.template.name.clone(),
                        "序号" => index.to_string(),
                        "序号01" => format!("{:02}", index),
                        "序号001" => format!("{:03}", index),
                        "中文序号" => to_chinese_number(index),
                        _ => token.value.clone(),
                    },
                    _ => token.value.clone(),
                })
                .collect();
            let raw = parts.join(&sep);
            let clean = sanitize_filename(&raw);
            if clean.trim().is_empty() || clean == ".docx" {
                return format!("{}-{}.docx", manifest.template.name, index);
            }
            return if clean.ends_with(".docx") {
                clean
            } else {
                format!("{clean}.docx")
            };
        }
    }

    // Fallback: existing pattern logic
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

// 收敛说明：统一委托 crate::util::fs::unique_output_path（filename 固定为 .docx）。
// 与原本地实现的差异仅在撞名场景：原实现从 `-1` 起编号，现从 `-2` 起。
fn unique_output_path(dir: &Path, filename: &str) -> PathBuf {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    crate::util::fs::unique_output_path(dir, stem, "docx")
}

fn sanitize_filename(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect()
}

fn to_chinese_number(n: usize) -> String {
    const DIGITS: &[char] = &['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    const UNITS: &[&str] = &["", "十", "百", "千", "万"];
    if n == 0 {
        return "零".to_string();
    }
    if n >= 10000 {
        return n.to_string();
    } // fallback for large numbers
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let digit = ch.to_digit(10).unwrap() as usize;
        let unit_idx = len - i - 1;
        if digit == 0 {
            if !result.is_empty() && !result.ends_with('零') {
                result.push('零');
            }
        } else {
            result.push(DIGITS[digit]);
            result.push_str(UNITS[unit_idx]);
        }
    }
    // Remove trailing zero
    if result.ends_with('零') {
        result.pop();
    }
    // Special case: 一十 → 十
    if result.starts_with('一')
        && result.len() > 1
        && result.chars().nth(1).map(|c| c == '十').unwrap_or(false)
    {
        result = result[3..].to_string(); // skip '一' (3 bytes in UTF-8)
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_field(id: &str, ftype: &str) -> TemplateField {
        TemplateField {
            id: id.to_string(),
            name: id.to_string(),
            label: id.to_string(),
            field_type: ftype.to_string(),
            ..Default::default()
        }
    }

    fn test_manifest(fields: Vec<TemplateField>) -> TemplateManifest {
        TemplateManifest {
            format_version: 2,
            template: super::super::TemplateMeta {
                id: "t0".to_string(),
                name: "test".to_string(),
                created: String::new(),
                updated: String::new(),
            },
            fields,
            filename_template: None,
        }
    }

    fn single_cell_value(field: &TemplateField, cell: calamine::Data) -> serde_json::Value {
        let manifest = test_manifest(vec![field.clone()]);
        let col_map: Vec<(usize, &TemplateField)> = vec![(0, &manifest.fields[0])];
        let values = build_row_values(&[cell], &col_map, &manifest);
        values.get(&field.id).cloned().unwrap_or(serde_json::Value::Null)
    }

    #[test]
    fn checkbox_truth_table_includes_cn_check_marks() {
        let field = test_field("c", "checkbox");
        // "√" 与 "☒"（带叉勾选框）均为勾选，与 scan.rs CHECKBOX_CHARS 对齐
        for checked in ["√", "☒", "☑", "✓", "✔", "是", "true", "1", "yes"] {
            assert_eq!(
                single_cell_value(&field, calamine::Data::String(checked.to_string())),
                serde_json::Value::Bool(true),
                "{checked} 应判定为勾选"
            );
        }
        for unchecked in ["☐", "□", "否", "false", ""] {
            assert_eq!(
                single_cell_value(&field, calamine::Data::String(unchecked.to_string())),
                serde_json::Value::Bool(false),
                "{unchecked} 应判定为未勾选"
            );
        }
    }

    #[test]
    fn float_cell_does_not_saturate_to_i64_max() {
        // 超出 i64 精确范围的大浮点不得静默饱和成 i64::MAX
        assert_eq!(
            cell_to_string(&calamine::Data::Float(1e20)),
            "100000000000000000000"
        );
        // 2^53 边界仍按整数输出（无 ".0" 后缀）
        assert_eq!(
            cell_to_string(&calamine::Data::Float(9_007_199_254_740_992.0)),
            "9007199254740992"
        );
        assert_eq!(cell_to_string(&calamine::Data::Float(42.0)), "42");
        assert_eq!(cell_to_string(&calamine::Data::Float(-42.0)), "-42");
        assert_eq!(cell_to_string(&calamine::Data::Float(3.5)), "3.5");
    }

    #[test]
    fn batch_date_values_apply_field_date_format() {
        let mut field = test_field("d", "date");
        field.date_format = "cn".to_string();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026-08-05".to_string())),
            serde_json::Value::String("2026年8月5日".to_string())
        );
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026年8月5日".to_string())),
            serde_json::Value::String("2026年8月5日".to_string())
        );
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("20260805".to_string())),
            serde_json::Value::String("2026年8月5日".to_string())
        );

        field.date_format = "cn_full".to_string();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026-08-05".to_string())),
            serde_json::Value::String("二零二六年八月五日".to_string())
        );

        field.date_format = "iso".to_string();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026年8月5日".to_string())),
            serde_json::Value::String("2026-8-5".to_string())
        );

        field.date_format = "en_long".to_string();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026-08-05".to_string())),
            serde_json::Value::String("August 5, 2026".to_string())
        );

        field.date_format = "en_ordinal".to_string();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026-08-05".to_string())),
            serde_json::Value::String("2026 August 5th".to_string())
        );

        field.date_format = "blank".to_string();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("留空".to_string())),
            serde_json::Value::String("    年  月  日".to_string())
        );
    }

    #[test]
    fn batch_date_values_passthrough_when_unparsable_or_no_format() {
        let mut field = test_field("d", "date");
        field.date_format = "cn".to_string();
        // 无法解析的日期原样透传，不报错
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("待定".to_string())),
            serde_json::Value::String("待定".to_string())
        );
        // 未设置 date_format 时不做格式转换
        field.date_format = String::new();
        assert_eq!(
            single_cell_value(&field, calamine::Data::String("2026年8月5日".to_string())),
            serde_json::Value::String("2026年8月5日".to_string())
        );
    }
}
