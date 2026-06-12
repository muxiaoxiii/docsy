# Docsy 更新日志

本文件记录 Docsy 每个版本的核心变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

## [0.5.2] - 2026-06-12

### 新增
- 设置页：外部工具状态检测（qpdf/ffmpeg/libreoffice）、诊断信息展示
- 首页：真实模板列表和最近记录、快捷入口对接模块注册表
- PDF 子页面：独立的解锁、合并、证据整理视图
- 图片排版视图：文件夹选择、参数配置、分析结果展示

### 改进
- 首页卡片和菜单从 moduleRegistry 动态生成
- 设置页集中管理外部工具路径和状态
- PDF 工具页子路由完善

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
- docx 引擎：quick-xml 模型解析 + 占位符渲染（支持 {{key}}, {{?key:text}}, {{*key}}, {{#row}}），7 个单元测试
- PDF 证据整理：文件夹扫描、自然排序、DOC/DOCX 转换、分组合并、身份重命名、页眉页脚叠加
- PDF 页眉页脚：printpdf 文字层生成 + qpdf --overlay 合成、CJK 字体自动检测、{page}/{total} 占位符
- 图片排版：文件夹分析、维度检测、前缀分组、docx/pdf 输出、fit/fill/original 缩放，4 个单元测试
- 模板编辑器：加载 docx、标记字段、配置属性、保存 .docsytpl
- 视频抽帧：FFmpeg 检测、视频信息、抽帧设置、时间戳叠加
- 模板管理：模板列表、字段/字典/记录查看、固定/删除
- 外部工具检测/安装统一接口

### 测试
- Rust: 11 个测试通过（docx render 7 + image paddler 4）
- Frontend: vite build 通过

### Git 提交记录
- `ce73fb5` 归档旧代码 + 建立版本基础
- `fe09a13` 完整后端服务层 + 前端模块骨架
- `9bacfd0` 修复编译错误，验证构建
- `5d94962` 实现 PDF 证据整理和页眉页脚模块
- `d4ad609` 实现 docx 渲染引擎（quick-xml）
- `6628179` 实现图片排版模块
- `9ac7e77` 改进前端表单（完整字段类型支持）
- `4499e2c` 更新 CHANGELOG
- `dad36fb` 实现模板编辑器和视频抽帧模块
- `a34f5cc` 实现模板管理页面
