//! 字典的 Excel 导入导出
//!
//! 单 sheet 横排布局：每个字典占一列（或两列，对当事人这种对象数组而言）。
//! 列头中文标签，方便用户编辑。
//!
//! 导出示例：
//! | 法院               | 律所                | 律师 | 当事人              | 当事人主体类型 | 案由 | 阶段     |
//! | 北京知识产权法院    | 北京志霖律师事务所  | 李月春 | 日本制铁株式会社    | 法人          | ... | 一审阶段 |
//! | 最高人民法院        | ...                 | 李琼   | 浦项股份有限公司    | 法人          | ... | 二审阶段 |
//!
//! 列与字典 key 的映射通过第二行（隐藏）保存——这样导入时知道哪一列对应哪个字典。
//! 实际实现简化：在第一行用中文标签，第二行用 key（隐藏行高度=0），数据从第三行开始。

use std::collections::BTreeMap;
use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Format, Workbook};
use serde_json::{json, Map, Value};

const SHEET_NAME: &str = "字典";

fn label_for(key: &str) -> &str {
    match key {
        "courts" => "法院",
        "firms" => "律所",
        "lawyers" => "律师",
        "parties" => "当事人",
        "causes" => "案由",
        "stages" => "阶段",
        other => other,
    }
}

/// 列定义：(字典 key, 子字段路径, 列头, 是否对象数组的"主"列)
struct Column {
    key: String,
    subfield: Option<String>,
    header: String,
}

fn build_columns(map: &Map<String, Value>) -> Vec<Column> {
    let mut cols = Vec::new();
    // 固定顺序，方便用户阅读
    let order = ["courts", "firms", "lawyers", "parties", "causes", "stages"];
    let mut handled = std::collections::HashSet::new();

    let emit = |key: &str, items: &Value, cols: &mut Vec<Column>| {
        let label = label_for(key).to_string();
        if let Value::Array(arr) = items {
            let is_object = arr
                .first()
                .map(|v| matches!(v, Value::Object(_)))
                .unwrap_or(false);
            if is_object {
                let mut subkeys: Vec<String> = Vec::new();
                for v in arr {
                    if let Value::Object(o) = v {
                        for k in o.keys() {
                            if !subkeys.contains(k) {
                                subkeys.push(k.clone());
                            }
                        }
                    }
                }
                if subkeys.is_empty() {
                    subkeys.push("value".to_string());
                }
                for sk in subkeys {
                    let header = if sk == "name" {
                        label.clone()
                    } else {
                        format!("{}{}", label, sub_label(&sk))
                    };
                    cols.push(Column {
                        key: key.to_string(),
                        subfield: Some(sk.clone()),
                        header,
                    });
                }
            } else {
                cols.push(Column {
                    key: key.to_string(),
                    subfield: None,
                    header: label.clone(),
                });
            }
        }
    };

    for k in order {
        if let Some(v) = map.get(k) {
            emit(k, v, &mut cols);
            handled.insert(k.to_string());
        }
    }
    // 其它字典（用户后加的）按字典序追加
    let extras: BTreeMap<_, _> = map.iter().filter(|(k, _)| !handled.contains(*k)).collect();
    for (k, v) in extras {
        emit(k, v, &mut cols);
    }
    cols
}

fn sub_label(sub: &str) -> &str {
    match sub {
        "name" => "",
        "subject_type" => "主体类型",
        other => other,
    }
}

pub fn export_to_xlsx(path: &Path, dictionaries: &Value) -> Result<(), String> {
    let mut book = Workbook::new();
    let header_fmt = Format::new().set_bold().set_background_color(0xEEEEEE);

    let Value::Object(map) = dictionaries else {
        return Err("字典必须是对象".to_string());
    };
    let columns = build_columns(map);

    let sheet = book
        .add_worksheet()
        .set_name(SHEET_NAME)
        .map_err(|e| e.to_string())?;

    // 第 1 行：中文表头
    // 第 2 行：key|subfield，便于导入时映射；行高度设为 0（隐藏）
    for (col_idx, c) in columns.iter().enumerate() {
        sheet
            .write_string_with_format(0, col_idx as u16, &c.header, &header_fmt)
            .map_err(|e| e.to_string())?;
        let mapping = match &c.subfield {
            Some(s) => format!("{}|{}", c.key, s),
            None => c.key.clone(),
        };
        sheet
            .write_string(1, col_idx as u16, &mapping)
            .map_err(|e| e.to_string())?;
    }
    sheet.set_row_height(1, 0).map_err(|e| e.to_string())?;

    // 数据行：从第 3 行（row=2）开始
    let max_rows = map
        .values()
        .filter_map(|v| v.as_array().map(|a| a.len()))
        .max()
        .unwrap_or(0);

    for row in 0..max_rows {
        for (col_idx, c) in columns.iter().enumerate() {
            let cell = match map.get(&c.key) {
                Some(Value::Array(arr)) => arr.get(row).map(|v| match (v, &c.subfield) {
                    (Value::Object(o), Some(sf)) => o
                        .get(sf)
                        .map(|x| match x {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default(),
                    (Value::String(s), _) => s.clone(),
                    (other, _) => other.to_string(),
                }),
                _ => None,
            };
            if let Some(s) = cell {
                if !s.is_empty() {
                    sheet
                        .write_string((row + 2) as u32, col_idx as u16, &s)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    // 列宽自适应到 16
    for col_idx in 0..columns.len() {
        sheet
            .set_column_width(col_idx as u16, 16.0)
            .map_err(|e| e.to_string())?;
    }

    book.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn import_from_xlsx(path: &Path) -> Result<Value, String> {
    let mut book = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let names: Vec<String> = book.sheet_names().to_vec();
    let sheet_name = names
        .iter()
        .find(|n| n.as_str() == SHEET_NAME)
        .or_else(|| names.first())
        .ok_or("Excel 没有可读 sheet")?
        .clone();
    let range = book
        .worksheet_range(&sheet_name)
        .map_err(|e| e.to_string())?;

    let mut rows = range.rows();
    let _header = rows.next().ok_or("缺少表头")?.to_vec();
    let mapping_row = rows.next().ok_or("缺少 key 映射行（第二行）")?.to_vec();

    // 解析每列 -> (key, subfield)
    let mut cols: Vec<(String, Option<String>)> = Vec::new();
    for cell in &mapping_row {
        let s = cell_to_string(cell);
        if s.is_empty() {
            cols.push(("".to_string(), None));
            continue;
        }
        let mut parts = s.splitn(2, '|');
        let k = parts.next().unwrap_or("").to_string();
        let sub = parts.next().map(|s| s.to_string());
        cols.push((k, sub));
    }
    // 兼容旧版：表头是中文 + 没有第二行映射时，用 header 推断
    if cols.iter().all(|(k, _)| k.is_empty()) {
        // 整个 mapping_row 视为数据行回退（罕见路径）
        return Err("Excel 第二行缺少 key 映射，无法导入".to_string());
    }

    // 收集每个字典的列
    let mut by_dict: BTreeMap<String, Vec<(usize, Option<String>)>> = BTreeMap::new();
    for (idx, (k, sub)) in cols.iter().enumerate() {
        if k.is_empty() {
            continue;
        }
        by_dict
            .entry(k.clone())
            .or_default()
            .push((idx, sub.clone()));
    }

    let mut out = Map::new();
    for (key, col_info) in by_dict {
        let is_object = col_info.iter().any(|(_, sub)| sub.is_some());
        let mut arr: Vec<Value> = Vec::new();
        for row in range.rows().skip(2) {
            if is_object {
                let mut obj = Map::new();
                let mut empty = true;
                for (idx, sub) in &col_info {
                    let s = row.get(*idx).map(cell_to_string).unwrap_or_default();
                    if !s.is_empty() {
                        empty = false;
                    }
                    let key = sub.clone().unwrap_or_else(|| "value".to_string());
                    obj.insert(key, Value::String(s));
                }
                if !empty {
                    arr.push(Value::Object(obj));
                }
            } else {
                if let Some((idx, _)) = col_info.first() {
                    let s = row.get(*idx).map(cell_to_string).unwrap_or_default();
                    if !s.is_empty() {
                        arr.push(Value::String(s));
                    }
                }
            }
        }
        out.insert(key, Value::Array(arr));
    }

    Ok(json!(out))
}

fn cell_to_string(c: &Data) -> String {
    match c {
        Data::String(s) => s.clone(),
        Data::Empty => String::new(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Data::Int(i) => format!("{i}"),
        Data::Bool(b) => format!("{b}"),
        Data::DateTime(d) => format!("{d}"),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{e:?}"),
    }
}
