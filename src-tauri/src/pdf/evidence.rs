use anyhow::Result;

pub fn scan_folder(root: &str) -> Result<serde_json::Value> {
    // TODO: implement evidence folder scanning
    Ok(serde_json::json!({ "groups": [], "root": root }))
}

pub fn build_group_pdfs(args: &serde_json::Value) -> Result<serde_json::Value> {
    // TODO: implement evidence group PDF building
    anyhow::bail!("证据整理分组合成待实现")
}

pub fn merge_all(args: &serde_json::Value) -> Result<String> {
    // TODO: implement evidence final merge
    anyhow::bail!("证据整理合并待实现")
}
