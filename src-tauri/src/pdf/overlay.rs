use anyhow::Result;

pub fn overlay_text(args: &serde_json::Value) -> Result<String> {
    // TODO: implement PDF text overlay
    anyhow::bail!("PDF 页眉页脚叠加待实现")
}

pub fn batch_overlay(args: &serde_json::Value) -> Result<Vec<String>> {
    // TODO: implement batch overlay
    anyhow::bail!("PDF 批量页眉页脚叠加待实现")
}
