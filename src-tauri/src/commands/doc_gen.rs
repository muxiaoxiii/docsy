use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GenerateArgs {
    pub template_id: String,
    pub values: serde_json::Value,
    pub output_path: Option<String>,
    pub export_pdf: bool,
}

#[tauri::command]
pub fn generate_document(args: GenerateArgs) -> Result<crate::services::doc_gen::GenerateResult, String> {
    let service_args = crate::services::doc_gen::GenerateArgs {
        template_id: args.template_id,
        values: args.values,
        output_path: args.output_path,
        export_pdf: args.export_pdf,
    };
    crate::services::doc_gen::generate(service_args).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preview_document(template_id: String, values: serde_json::Value) -> Result<String, String> {
    crate::services::doc_gen::preview(&template_id, &values).map_err(|e| e.to_string())
}
