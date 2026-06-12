use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DictQuery {
    pub dict_name: String,
    pub template_id: Option<String>,
    pub field_key: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct DictEntry {
    pub key: String,
    pub label: String,
    pub pinyin: Option<String>,
    pub extra: Option<serde_json::Value>,
    pub source: String,
    pub frequency: u32,
}

#[tauri::command]
pub fn query_dictionary(query: DictQuery) -> Result<Vec<DictEntry>, String> {
    crate::services::dictionary::query(query).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct RecommendQuery {
    pub template_id: String,
    pub field_key: String,
    pub context: serde_json::Value,
    pub search: Option<String>,
}

#[tauri::command]
pub fn recommend_values(query: RecommendQuery) -> Result<Vec<DictEntry>, String> {
    crate::services::dictionary::recommend(query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_dictionary_xlsx(path: String) -> Result<String, String> {
    crate::services::export::export_dict_xlsx(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct ImportDictArgs {
    pub path: String,
    pub mode: String, // "merge" | "overwrite"
}

#[tauri::command]
pub fn import_dictionary_xlsx(args: ImportDictArgs) -> Result<serde_json::Value, String> {
    crate::services::export::import_dict_xlsx(&args.path, &args.mode).map_err(|e| e.to_string())
}
