/// Module registry — returns descriptors for all registered modules.
/// Frontend uses this to build menus, routes, and home cards.
pub fn all_descriptors() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "home",
            "name": "首页",
            "icon": "HomeFilled",
            "description": "欢迎页面和快捷入口",
            "category": "system",
            "default_visible": true,
        }),
        serde_json::json!({
            "id": "pdf-tools",
            "name": "PDF 工具",
            "icon": "Files",
            "description": "PDF 解锁、合并、拆分、证据整理",
            "category": "pdf",
            "default_visible": true,
            "sub_modules": [
                { "id": "unlock", "name": "解锁" },
                { "id": "merge", "name": "合并" },
                { "id": "evidence", "name": "证据整理" },
            ]
        }),
        serde_json::json!({
            "id": "image-paddler",
            "name": "图片排版",
            "icon": "Picture",
            "description": "图片批量排版为 A4 文档",
            "category": "media",
            "default_visible": true,
        }),
        serde_json::json!({
            "id": "video-extract",
            "name": "视频抽帧",
            "icon": "VideoCamera",
            "description": "按时间或频率导出视频帧",
            "category": "media",
            "default_visible": true,
        }),
        serde_json::json!({
            "id": "settings",
            "name": "设置",
            "icon": "Setting",
            "description": "应用设置",
            "category": "system",
            "default_visible": true,
        }),
    ]
}
