use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub builtin: bool,
    pub pinned_to_tab: bool,
    pub field_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ArchiveInfo {
    pub id: String,
    pub timestamp: String,
    pub label: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResolvedTemplate {
    pub id: String,
    pub name: String,
    pub docx_path: PathBuf,
    pub fields: serde_json::Value,
    pub dictionaries: Option<serde_json::Value>,
    pub manifest: serde_json::Value,
    pub pinned_to_tab: bool,
    pub builtin: bool,
}

pub fn resolve(template_id: &str) -> Result<ResolvedTemplate> {
    let user_tpl = user_templates_dir().join(format!("{}.docsytpl", template_id));
    if user_tpl.exists() {
        return load_docsytpl(template_id, &user_tpl);
    }
    anyhow::bail!("模板 '{}' 不存在。请先在模板编辑器中创建。", template_id)
}

pub fn list() -> Result<Vec<TemplateInfo>> {
    let mut templates = Vec::new();

    // user templates from .docsytpl files
    let user_dir = user_templates_dir();
    if user_dir.exists() {
        for entry in std::fs::read_dir(&user_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("docsytpl") {
                let id = entry.path().file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if let Ok(tpl) = load_docsytpl(&id, &entry.path()) {
                    templates.push(TemplateInfo {
                        id: tpl.id,
                        name: tpl.name,
                        icon: None,
                        builtin: false,
                        pinned_to_tab: tpl.pinned_to_tab,
                        field_count: tpl.fields.get("fields").and_then(|f| f.as_array()).map(|a| a.len()).unwrap_or(0),
                        created_at: tpl.manifest.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        updated_at: String::new(),
                    });
                }
            }
        }
    }

    Ok(templates)
}

pub fn get_meta(template_id: &str) -> Result<serde_json::Value> {
    let tpl = resolve(template_id)?;
    Ok(serde_json::json!({
        "id": tpl.id,
        "name": tpl.name,
        "builtin": tpl.builtin,
        "pinned_to_tab": tpl.pinned_to_tab,
        "fields": tpl.fields,
        "dictionaries": tpl.dictionaries,
    }))
}

pub fn save_config(
    _template_id: &str,
    _fields: &serde_json::Value,
    _dictionaries: Option<&serde_json::Value>,
) -> Result<()> {
    // TODO: save template config
    Ok(())
}

pub fn delete(_template_id: &str) -> Result<()> {
    // TODO: delete template
    Ok(())
}

pub fn rename(_template_id: &str, _new_name: &str) -> Result<()> {
    // TODO: rename template
    Ok(())
}

pub fn set_pinned(_template_id: &str, _pinned: bool) -> Result<()> {
    // TODO: set pinned status
    Ok(())
}

pub fn list_archives(_template_id: &str) -> Result<Vec<ArchiveInfo>> {
    Ok(vec![])
}

pub fn restore_archive(_template_id: &str, _archive_id: &str) -> Result<()> {
    // TODO: restore archive
    Ok(())
}

fn load_builtin(template_id: &str) -> Result<ResolvedTemplate> {
    let templates_dir = builtin_templates_dir();

    let fields_path = templates_dir.join(format!("{}.fields.json", template_id));
    let fields = if fields_path.exists() {
        let content = std::fs::read_to_string(&fields_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({ "fields": [] })
    };

    let dict_path = templates_dir.join("dictionaries.json");
    let dictionaries = if dict_path.exists() {
        let content = std::fs::read_to_string(&dict_path)?;
        serde_json::from_str(&content)?
    } else {
        None
    };

    let docx_path = templates_dir.join(format!("{}.docx", template_id));

    Ok(ResolvedTemplate {
        id: template_id.to_string(),
        name: template_id.to_string(),
        docx_path,
        fields,
        dictionaries,
        manifest: serde_json::json!({}),
        pinned_to_tab: true,
        builtin: true,
    })
}

fn load_docsytpl(template_id: &str, path: &PathBuf) -> Result<ResolvedTemplate> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;

    let manifest: serde_json::Value = {
        let mut f = zip.by_name("manifest.json")?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s)?;
        serde_json::from_str(&s)?
    };

    let fields: serde_json::Value = {
        let mut f = zip.by_name("fields.json")?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s)?;
        serde_json::from_str(&s)?
    };

    let dictionaries = zip.by_name("dictionaries.json").ok().and_then(|mut f| {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).ok()?;
        serde_json::from_str(&s).ok()
    });

    let name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(template_id)
        .to_string();

    Ok(ResolvedTemplate {
        id: template_id.to_string(),
        name,
        docx_path: path.clone(),
        fields,
        dictionaries,
        manifest,
        pinned_to_tab: false,
        builtin: false,
    })
}

pub fn user_templates_dir() ->PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Docsy")
        .join("user_templates")
}

fn builtin_templates_dir() -> PathBuf {
    // Compiled-in templates via include_bytes! or relative to binary
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
}
