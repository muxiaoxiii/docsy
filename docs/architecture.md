# Docsy 当前架构

更新时间：2026-06-23

## 当前边界

模板生成、内置模板、模板编辑、模板管理、字典推荐和生成记录已从当前应用移除。当前应用只保留可独立运行的工具模块：

- PDF 工具
- 图片排版
- 视频抽帧
- 设置与诊断

## 前端结构

模块仍通过 `src/modules/<name>/index.js` 注册路由、菜单和首页卡片。

当前有效模块：

```text
src/modules/
├── home/
├── pdf-tools/
├── image-paddler/
├── video-extract/
└── settings/
```

`src/core/moduleRegistry.js` 使用 `import.meta.glob` 自动收集模块。新增模块时，应保持自包含目录，不要把业务状态散落到全局。

## PDF 证据处理设计

PDF 工具中的证据 PDF 合并、拆分、页眉页脚处理、A4 规范化、批注删除和预览能力，统一以 `docs/pdf-evidence-processing-design.md` 为设计依据。

后续实现应围绕“证据文件列表 + 页码范围 + 输出规则”的整体模型收敛，不应继续把页眉页脚插入、PDF 合并、拆分、页面规范化做成互相割裂的临时功能。

## 后端结构

Tauri 命令集中注册在 `src-tauri/src/commands/mod.rs`。

当前命令域：

- `pdf`
- `image_paddler`
- `video`
- `settings`
- `system`

业务实现放在 `src-tauri/src/services/`、`src-tauri/src/pdf/`、`src-tauri/src/ffmpeg/`、`src-tauri/src/external/` 等目录。`services/history.rs` 现在只负责应用设置读写，不再维护模板生成历史。

## 已移除的旧边界

以下内容不应被新代码继续引用：

- `.docsytpl`
- `doc-gen`
- `template-editor`
- `template-mgmt`
- `list_templates`
- `get_template_meta`
- `generate_document`
- `save_template`
- `query_dictionary`
- `generation_records`

后续模板模块要从新的设计文档开始，不应在现有工具模块中渐进复活旧逻辑。
