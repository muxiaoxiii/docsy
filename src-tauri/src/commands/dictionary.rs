use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DictQuery {
    pub dict_name: String,
    pub template_id: Option<String>,
    pub field_key: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RecommendQuery {
    pub template_id: String,
    pub field_key: String,
    pub context: serde_json::Value,
    pub search: Option<String>,
}

#[tauri::command]
pub fn query_dictionary(query: DictQuery) -> Result<Vec<crate::services::dictionary::DictEntry>, String> {
    let service_query = crate::services::dictionary::DictQuery {
        dict_name: query.dict_name,
        template_id: query.template_id,
        field_key: query.field_key,
        search: query.search,
        limit: query.limit,
    };
    crate::services::dictionary::query(service_query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recommend_values(query: RecommendQuery) -> Result<Vec<crate::services::dictionary::DictEntry>, String> {
    let service_query = crate::services::dictionary::RecommendQuery {
        template_id: query.template_id,
        field_key: query.field_key,
        context: query.context,
        search: query.search,
    };
    crate::services::dictionary::recommend(service_query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_dictionary_xlsx(path: String) -> Result<String, String> {
    crate::services::export::export_dict_xlsx(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct ImportDictArgs {
    pub path: String,
    pub mode: String,
}

#[tauri::command]
pub fn import_dictionary_xlsx(args: ImportDictArgs) -> Result<serde_json::Value, String> {
    crate::services::export::import_dict_xlsx(&args.path, &args.mode).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct RecordFieldUsageArgs {
    pub template_id: String,
    pub field_key: String,
    pub value: serde_json::Value,
}

#[tauri::command]
pub fn record_field_usage(args: RecordFieldUsageArgs) -> Result<(), String> {
    crate::services::dictionary::record_usage(&args.template_id, &args.field_key, &args.value)
        .map_err(|e| e.to_string())
}
