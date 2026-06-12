# Docsy 更新日志

本文件记录 Docsy 每个版本的核心变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

## [0.5.1] - 2026-06-12

### 重构启动
- 将旧代码整体归档至 `Archived/` 目录（前端、后端源码、旧文档）
- 建立 v0.5 版本线，从零重建架构

### 新增
- 统一模块注册系统（前端 `moduleRegistry.js` + 后端 `ModuleDescriptor`）
- vue-router 替代手动 shallowRef 路由
- Pinia 状态管理（app / dictionary / template stores）
- 外部工具统一抽象层（`ExternalTool` trait）：qpdf, ffmpeg, libreoffice
- Tauri 命令自动收集机制（`commands/mod.rs`）
- 前端 `tauriBridge.js` 统一调用封装
- 8 个功能模块注册入口（home, doc-gen, template-editor, template-mgmt, pdf-tools, image-paddler, video-extract, settings）
- SQLite 数据库 schema（global_dictionaries, template_dictionaries, field_history, parties, generation_records, template_meta）
- 字典三层叠加查询引擎（global → template → history）
- docx 引擎骨架（model.rs 用 quick-xml 解析, render.rs 占位符替换）
- 外部工具检测/安装统一接口

### 架构
- 目录结构：`src/modules/` 自包含模块（每个模块含 index.js / views / components / composables）
- 后端分层：`commands/`（薄壳）→ `services/`（业务逻辑）→ `docx/pdf/ffmpeg/external/`（底层）
- 数据存储从 JSON 文件迁移到 SQLite (rusqlite)
- docx XML 解析从 regex 迁移到 quick-xml event parser

### 技术栈
- Tauri 2 + Vue 3 + Element Plus + Pinia + vue-router
- Rust: zip + quick-xml + rusqlite + printpdf + serde + regex + mammoth + base64
- 测试: Vitest (前端) + cargo test (后端)
