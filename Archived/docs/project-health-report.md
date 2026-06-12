# Docsy 项目健康报告

更新时间：2026-06-09

本文档是 Docsy 项目的健康状态汇总和修改方向指引。不是唯一真相源——设计细节以 `Docsy软件设计文档.md` 为准，编辑器重构细节以 `docs/template-editor-refactor.md` 为准。

---

## 0. 项目概况

Docsy 是 Tauri 2 + Vue 3 + Rust 的本地文档处理工具箱，面向中国法律从业者。

**技术栈**：Rust 后端 + Vue 3 前端 + Element Plus UI + Tauri 2 桌面壳

**核心功能**：Word 模板生成 / 模板编辑 / 字典复用 / 历史记录 / PDF 证据整理 / 图片排版 / 视频抽帧

**测试状态**（2026-06-09 实测）：
- Rust：`cargo test` 45 passed, 0 failed
- 前端：`npm test` 34 passed（4 个测试文件，Vitest）
- 构建：`npx vite build` 通过，但有 VueUse `#__PURE__` 注释 warning 和 chunk 超 500KB warning（非致命）

---

## 1. 文档地图

| 文档 | 状态 | 说明 |
|------|------|------|
| `README.md` | ✅ 已更新 | 功能状态表、目录结构、启动方式、文档索引 |
| `Docsy软件设计文档.md` | 📖 参考 | 产品全貌、架构设计、字段 schema、接手约定（1522 行，偏长但完整） |
| `docs/project-health-report.md` | ✅ 本文档 | 项目健康状态、问题清单、修复方向 |
| `docs/template-editor-refactor.md` | 📦 归档 | 模板编辑器重构执行记录（Phase 1-8 已完成，保留供回溯） |
| `docs/template-presets.md` | 📦 归档 | 预置模板设计方案（未实现，保留供后续参考） |
| `docs/pdf-evidence-workflow.md` | ✅ 已更新 | PDF 证据整理工作流设计与实现状态 |
| `docs/docx-renderer-research-officecli.md` | ✅ 已更新 | OfficeCLI 研究 + Docsy 改造方案（含实现状态标注） |
| `docs/docx-research.md` | 📖 参考 | docx XML 结构研究笔记 |
| `docs/logging-observability.md` | 📖 参考 | 日志规范 |
| `docs/docsy-module-status-and-improvement-plan.md` | 📦 归档 | GPT 生成的模块分析（已被本文档吸收） |

**建议阅读顺序**：README → 本文档 → 设计文档（按需查阅具体章节）

---

## 2. 模块健康状态

### 2.1 模板生成模块 ✅ 健康

**前端**：`LetterView.vue`（797 行）
**后端**：`render.rs`（617 行）、`template_builder.rs`（1375 行中的生成相关部分）

功能完整度：
- ✅ 字段表单渲染（text/date/select/party/reference/list）
- ✅ 字典复用
- ✅ docx 生成（含 party 拆 run、hideable 删除、条件前缀、行重复）
- ✅ 生成历史保存/加载/删除
- ✅ 内置 letter 模板 + 用户 .docsytpl 模板

**问题**：
- `LetterView.vue` 797 行偏大，表单渲染/字典/历史/生成混在一起
- 字段 schema 没有独立类型定义，靠约定传播

**方向**：拆为 `useTemplateForm`、`useGenerationHistory`、`generatorApi` composable

---

### 2.2 模板编辑器模块 ✅ 健康（有已知限制）

**前端**：`features/template-editor/`（8 个文件，~2200 行）
**后端**：`template_builder.rs`（1375 行）、`docx/model.rs`（830 行）

功能完整度：
- ✅ 从 docx 创建模板
- ✅ 编辑已有模板
- ✅ 字段标记（mark）+ 复用字段
- ✅ 固有字编辑（contenteditable + path-based patch）
- ✅ 三种预览模式（marked/labels/edit）
- ✅ source map（DocxDocumentModel + data-source-path）
- ✅ 保存 .docsytpl（含 builder_state.json）
- ✅ 跨 text node / cross-run fallback

**已知限制**：
- 页眉页脚/文本框/脚注的文本节点未纳入 source map
- 跨 text node 选区仍 fallback 到 plainText 匹配
- 基础格式工具栏（字体/字号/加粗）未实现
- 前端测试覆盖不完整：已有 textRange、templateEditorMappers、docxModel、useTemplatePreview 共 34 个测试；但 useTemplateMarks、useTemplateTextEdit、useTemplateSave、字段组件无测试

**方向**：
- 模式收敛：编辑正文 / 标记字段 / 核对标签
- source-indexed preview 作为主路径，mammoth 只做 fallback
- 拆分 `template_builder.rs`：package.rs / marks.rs / text_patch.rs

---

### 2.3 模板管理模块 ⚠️ 可用但需优化

**前端**：`ManageView.vue`（1136 行）
**后端**：`templates.rs`（235 行）、`dict_xlsx.rs`（287 行）

功能完整度：
- ✅ 模板列表
- ✅ 启用/禁用
- ✅ 字段配置编辑
- ✅ 字典编辑 + Excel 导入导出
- ✅ 归档/恢复
- ✅ 生成记录

**问题**：
- `ManageView.vue` 1136 行，是整个项目最大的前端文件
- 字段删除应改为"隐藏优先"，物理删除只在编辑器中
- 27 处 Tauri invoke 调用，逻辑高度集中

**方向**：
- 拆为 `TemplateListPanel`、`FieldConfigPanel`、`DictionaryPanel`、`HistoryPanel`
- 字段删除策略：`enabled: false` 或 `hidden_in_form: true`

---

### 2.4 PDF 工具模块 ✅ 健康

**前端**：`PdfToolsView.vue`（50 行壳）、`PdfUnlock.vue`（310 行）、`PdfEvidenceView.vue`（788 行）
**后端**：`pdf/qpdf.rs`（346 行）、`pdf/evidence.rs`（543 行）、`pdf/overlay.rs`（649 行）

功能完整度：
- ✅ PDF 解锁（qpdf）
- ✅ 证据整理：扫描 → 按子文件夹合成 → 整理身份 → 页眉页脚 → 合并
- ✅ 页眉来源：文件名/自定义/序号/中文序号/固定前缀+序号
- ✅ 页脚页码：当前页/总页数（全局计算）
- ✅ 页数范围预览
- ✅ A4 尺寸检查 + 横向检测
- ✅ CJK 字体支持（自动加载系统字体）
- ✅ 拖拽排序 + 手动添加 PDF
- ✅ 12 个单元测试（evidence 8 + overlay 4）

**问题**：
- DOC/DOCX 转 PDF 依赖系统 LibreOffice，无内置转换器
- 页面规范化（自动旋转/缩放到 A4）未实现

**方向**：
- 设置页统一显示 qpdf / ffmpeg / LibreOffice 检测状态
- 考虑内置 LibreOffice 或提供安装引导

---

### 2.5 图片排版模块 ✅ 健康

**前端**：`ImagePaddlerView.vue`（765 行）
**后端**：`image_paddler.rs`（1568 行）

功能完整度：
- ✅ 文件夹分析 + 分组
- ✅ docx/pdf 输出
- ✅ A4 布局 + fit/fill/original 缩放
- ✅ 文件名标注
- ✅ 3 个单元测试

**问题**：
- `image_paddler.rs` 1568 行是最大的 Rust 文件
- UI 和运行逻辑混在一起

**方向**：保持独立，后续可拆分分析/生成两部分

---

### 2.6 视频抽帧模块 ✅ 健康

**前端**：`VideoExtractView.vue`（516 行）
**后端**：`ffmpeg/`（3 个文件，~870 行）

功能完整度：
- ✅ ffmpeg 检测 + brew 安装
- ✅ 视频信息读取
- ✅ 抽帧 + 时间戳水印
- ✅ 系统字体列表

**问题**：无重大问题

---

### 2.7 基础设施 ✅ 健康

| 组件 | 文件 | 状态 |
|------|------|------|
| 日志 | `app_log.rs` + `appLogger.js` | ✅ JSON Lines + 14 天保留 |
| 设置 | `history.rs` + `SettingsView.vue` | ✅ 基础设置可用 |
| 路由 | `App.vue` 手动 shallowRef | ⚠️ 无 Vue Router，够用但不标准 |
| 测试 | Rust 45 + Vitest 34 | ✅ 核心路径有覆盖 |

---

## 3. 代码质量问题清单

### 3.1 大文件（需拆分）

| 文件 | 行数 | 拆分方向 |
|------|------|---------|
| `image_paddler.rs` | 1568 | 拆为 analyze + generate |
| `lib.rs` | 1386 | commands 拆到 `commands/*.rs` |
| `template_builder.rs` | 1375 | 拆为 package / marks / text_patch / repository |
| `ManageView.vue` | 1136 | 拆为 4 个 Panel 组件 |
| `LetterView.vue` | 797 | 拆为 3 个 composable |
| `PdfEvidenceView.vue` | 788 | 可接受，但 overlay 逻辑可提取 |

### 3.2 过时工件

| 文件 | 处理 | 状态 |
|------|------|------|
| `src/features/template-editor.zip` | 删除（28KB 备份归档） | ✅ 已删除 |
| `.codex-pet-runs/` | 删除 + .gitignore | ✅ 已清理 |

`.gitignore` 已添加 `*.zip` 规则防止再次入库。

### 3.3 架构债务

| 问题 | 影响 | 优先级 |
|------|------|--------|
| regex 解析 OOXML | 复杂文档脆弱 | P2（当前够用，中期迁移 quick-xml） |
| 手写 base64 | 非标准但可用 | P3（可引入 base64 crate） |
| 无 Vue Router | 够用但不标准 | P3（当前规模不需要） |
| 前端无类型系统 | 字段 schema 变更风险 | P2（可加 JSDoc typedef） |

---

## 4. 具体代码修改方向

### 4.1 立即可做（低风险、高价值）

**A. 清理过时文件** ✅ 已完成
```
已删除：src/features/template-editor.zip
已删除：.codex-pet-runs/
已更新：.gitignore 添加 *.zip 规则
```

**B. lib.rs 命令分组**
当前 lib.rs 有 48 个 Tauri command，建议按模块分文件：
```
src-tauri/src/commands/
  pdf.rs        # check_qpdf, inspect_pdf, unlock_pdf, overlay_*, check_pdf_pages
  evidence.rs   # scan_evidence_folder, build_evidence_group_pdfs, merge_evidence_pdfs
  template.rs   # extract_docx_text, read_template_for_edit, save_user_template, ...
  editor.rs     # parse_docx_model_from_base64, edit_docx_text_range, edit_docx_text_node
  image.rs      # analyze_image_paddler_folder, run_image_paddler
  video.rs      # probe_video, extract_frames, check_ffmpeg
  settings.rs   # get_app_settings, set_app_settings, get_diagnostic_info
  logging.rs    # write_frontend_log, get_log_file_path, open_log_file, open_log_dir
```
`lib.rs` 只保留模块声明 + `run()` 函数。

**C. template_builder.rs 拆分**
```
src-tauri/src/template/
  package.rs     # save_user_template, read_template_for_edit, list_user_templates
  marks.rs       # FieldMark, rewrite_with_placeholders, rewrite_with_path_placeholders
  text_patch.rs  # replace_text_range, replace_text_range_xml
  repository.rs  # delete_user_template, rename_user_template
```

**D. ManageView.vue 拆分**
```
src/features/template-management/
  TemplateListPanel.vue    # 模板列表 + 启用/禁用
  FieldConfigPanel.vue     # 字段配置
  DictionaryPanel.vue      # 字典编辑 + Excel 导入导出
  HistoryPanel.vue         # 生成记录
  useTemplateManagement.js # 共享状态和操作
```

### 4.2 中期改进（需设计）

**E. 字段 schema 类型化**
新增 `src/types/templateFields.js`，用 JSDoc typedef 定义字段结构：
```js
/** @typedef {{ key: string, label: string, type: FieldType, ... }} TemplateField */
```
所有模块引用同一 typedef。

**F. LetterView composable 拆分**
```
src/features/document-generator/
  useTemplateForm.js       # 字段表单状态
  useGenerationHistory.js  # 历史记录
  generatorApi.js          # Tauri 调用
```

**G. 前端测试扩展**
优先为以下模块添加 Vitest 测试：
- `useTemplateMarks.js`（mark CRUD 逻辑）
- `useTemplateTextEdit.js`（文本替换 + mark 平移）
- 字段组件的基本渲染测试

### 4.3 长期方向

**H. regex → quick-xml 迁移**
`docx/model.rs` 和 `template_builder.rs` 中的 XML 解析逐步迁移到 `quick-xml` event parser。先从 `parse_docx_model` 开始。

**I. 预置模板机制**
实现 `template-presets.md` 中的设计：首次启动复制 letter.docsytpl，支持恢复出厂。

**J. 设置页统一诊断**
集中显示 qpdf / ffmpeg / LibreOffice 状态、数据目录、日志目录。

---

## 5. 验证命令

每次修改后必须通过：

```bash
# Rust 测试 + 零 warning
cd src-tauri && cargo test --quiet && cargo check 2>&1 | grep -c "warning:"

# 前端测试
cd /Users/only/Documents/PythonProgram/Docsy && npm test

# 前端构建
cd /Users/only/Documents/PythonProgram/Docsy && npx vite build
```
