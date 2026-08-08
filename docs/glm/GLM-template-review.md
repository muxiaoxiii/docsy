# template 模块审阅

> 审阅日期：2026-08-08
> 模块规模：前端 10718 行 + 后端 6608 行（docx_template 5659 + template_history 949）
> 审阅标准：与 MDG-016 页眉页脚审计相同

---

## 1. 模块概述

template 模块是 Docsy 的第二大功能模块，核心能力是给 Word 模板（`.docsytpl` 格式）填充字段值并批量生成法律文书。

### 前端结构
```
src/modules/template/
├── views/
│   └── TemplateView.vue          (2651 行 ⚠️ 超载)
├── components/
│   ├── TemplateBuildTab.vue      (1517 行 — 模板构建/标黄)
│   ├── TemplateRenderTab.vue     (1185 行 — 字段填写/渲染)
│   ├── TemplateHistoryTab.vue    (218 行 — 历史记录)
│   └── TemplateSettingsTab.vue   (344 行 — 回收站/数据库)
├── composables/
│   ├── fieldRowUtils.js          (1198 行 — 纯工具)
│   ├── useFieldNormalization.js  (728 行 — 字段规范化链)
│   ├── usePreviewSelection.js    (422 行 — 预览选区)
│   ├── useBatchFill.js           (296 行 — Excel 批量)
│   └── useTemplateSettings.js    (141 行 — 回收站)
├── rules/
│   ├── causeActions2025.js       (983 行 — 案由库)
│   ├── courtNames.js             (554 行 — 法院名)
│   └── publicRules.js            (320 行 — 角色/前后缀)
└── index.js
```

### 后端结构
```
src-tauri/src/docx_template/
├── mod.rs        (657 行 — 数据模型 + 安全常量)
├── render.rs     (1467 行 — 核心渲染)
├── save.rs       (908 行 — sdt 标记保存)
├── engine.rs     (818 行 — 入口)
├── batch.rs      (866 行 — Excel 批量)
├── scan.rs       (380 行 — 文档扫描)
├── ooxml.rs      (243 行 — XML 解析)
├── package.rs    (178 行 — zip 读写)
└── index.rs      (142 行 — TextIndex)
+ template_history.rs (949 行 — SQLite 历史)
```

---

## 2. 核心数据流

```
用户选择 .docsytpl 模板
  → engine::inspect_docx (engine.rs:155) 解析模板结构
  → 前端展示字段列表（TemplateRenderTab）
  → 用户填写字段值
  → normalizeValues() (TemplateView.vue:2248) 按类型规整
    ├─ party_list → [{name, suffix}]
    ├─ date → 按 dateFormat 格式化
    └─ reference → 解析 sourceKey
  → tauriCallSafe('render_docx_template', { args }) (TemplateView.vue:2211)
  → 后端 engine::render_docx (engine.rs:201)
    ├─ 读 docsytpl 包 (package.rs)
    ├─ render::render_docx → render_tree (render.rs:85)
    │   ├─ 递归遍历 XML 节点
    │   ├─ 命中 w:sdt → find_sdt_tag 读 w:tag
    │   ├─ tag_map 查字段 → replace_sdt_content
    │   │   ├─ 保留首个 w:r 的 w:rPr
    │   │   ├─ 渲染文本写入 w:t
    │   │   └─ unwrap_sdt_content 把 sdt 子节点 splice 替换
    │   └─ try_expand_table_row (party_list 多值克隆 w:tr)
    └─ write_docx_package 输出
```

### 批量渲染流程
```
Excel 文件 → validate_imported_xlsx (batch.rs:171)
  ├─ 行0: 隐藏元数据 (templateId\tfieldId\tfieldType)
  ├─ 行1: 字段标签
  ├─ 行2: 样本行 (是否生成=否 不渲染)
  └─ 行3+: 数据
→ batch_render (batch.rs:467) 逐行
  → build_row_values (batch.rs:606) 类型映射
    ├─ party_list 按、分割
    ├─ checkbox 按真值符号
    └─ radio/select 按 option label/id 匹配
  → engine::render_docx(args, "") 空 source 跳过历史
  → 前端 submitBatchSave 显式录入历史
```

---

## 3. 问题清单

### P0 — 安全漏洞

**XML 注入风险**（ooxml.rs:190）

```rust
writer.write_event(Event::Text(BytesText::new(text)))?;
```

quick-xml 0.37 的 `BytesText::new` **不进行转义**，直接把字符串原样写入。用户在字段值中输入 `<`、`&`、`>` 会破坏 XML 结构，可能注入任意 OOXML 元素（如新的 `w:sdt`、`w:fldSimple`）。

**修复**：改用 `BytesText::from_escaped` 配合手动转义，或：
```rust
let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
writer.write_event(Event::Text(BytesText::from_escaped(escaped.as_bytes())))?;
```

建议增加针对含 `<`/`&` 字段值的回归测试。

### P1 — 代码质量

**TemplateView.vue 2651 行超载**：承担状态管理 + 4 Tab 协调 + 字段推断 + 拆分逻辑等过多职责。三层 computed 链（`renderableTemplateFields` → `fillPositionEntries` → `filteredFillPositionEntries`，:312/:367/:416）调试困难。

**建议**：拆出 `useTemplateState.js`（状态管理）、`useFieldInference.js`（字段推断），将 Tab 协调逻辑移入各 Tab 组件。

**直接 invoke 绕过 tauriBridge**（TemplateView.vue:1299）

```js
await invoke('get_log_file_path')  // 绕过统一错误处理
```

与同文件其他 17 处 `tauriCallSafe` 不一致。

**死代码**：7 处 `#[allow(dead_code)]`（index.rs:21/45/69/79、template_history.rs:66/566/635）。

**重复逻辑**：
- `cell_to_string`（batch.rs:99）与 `value_to_display`（batch.rs:402）类似
- party_list 分割逻辑在 export（batch.rs:104）与 build_row_values（batch.rs:619）重复

### P2 — 一致性问题

**checkbox 符号硬编码不一致**：`mod.rs:301-304` 默认 `☑`/`☐`，与前端 `fieldRowUtils.js:92` 的 `checkedSymbolOptions` 不一致。

**参数风格不统一**：`useBatchFill.js:35` 传 args，`useTemplateSettings.js:23` 的 `list_template_trash` 不传 args。

### P3 — UI/UX

- 模板库卡片操作按钮（TemplateRenderTab.vue:26-27）为 `text` size=small，命中区域窄
- 批量填充缺少进度反馈（逐行渲染时无进度条）
- 历史记录 Tab 的关联建议展示不够直观

---

## 4. 模板历史系统（template_history.rs）

### SQLite Schema
```sql
generation_runs (id, template_id, template_name, template_path, output_path,
                  generated_at, field_values(JSON), source)
field_history   (run_id, template_id, field_id, field_name, field_label,
                  semantic_key, value_json, display_value, generated_at)
template_meta   (template_id PK, template_name, template_path, trashed, updated_at)
-- 4 个索引：template_runs / template_field / semantic / run
```

### 建议机制
| 类型 | 逻辑 | 代码位置 |
|------|------|---------|
| last_values | 该模板最近一次运行的整体字段值 | :245 |
| field_suggestions | 按 field_name 跨模板检索（含 `__template_common__` 共享桶） | :657 |
| semantic_suggestions | 按 semantic_key 在其他模板中检索 | :689 |
| association_suggestions | JOIN 同一 run 中其他字段值作为 trigger → target 共现统计 | :718 |

设计合理，四种建议互补。迁移函数（`ensure_field_history_id_column` :530、`ensure_generation_source_column` :552）通过 PRAGMA + ALTER TABLE 做版本升级，方案正确。

---

## 5. 安全评估

| 维度 | 评估 | 代码位置 |
|------|------|---------|
| 文件大小限制 | ✅ 完善（512MB docx/128MB XML/256MB binary/1GB 解压/4096 zip 条目） | mod.rs:23-30 |
| TOCTOU 防护 | ✅ `read_file_with_limit` 用 `take(limit+1)` 二次校验 | mod.rs:580 |
| 敏感部件拒绝 | ✅ 拒绝 comments/customXml/vbaProject/embeddings/signatures | engine.rs:255 |
| 路径安全 | ✅ `safe_template_file_name` 过滤分隔符 | mod.rs:365 |
| XML 编码 | ✅ 拒绝 UTF-16BE/LE，检测声明中的非 ASCII | ooxml.rs:25-44,:61 |
| **XML 注入** | ❌ `BytesText::new` 不转义 | ooxml.rs:190 |

---

## 6. 改进建议

| 优先级 | 问题 | 建议 | 工作量 |
|--------|------|------|--------|
| **P0** | XML 注入 | `BytesText::from_escaped` + 回归测试 | 0.5 天 |
| **P1** | TemplateView.vue 超载 | 拆出 composables | 2-3 天 |
| **P1** | 裸 invoke | 改走 tauriBridge + ESLint 规则 | 0.2 天 |
| **P2** | checkbox 符号不一致 | 前后端统一常量 | 0.2 天 |
| **P2** | 死代码 | 清理 7 处 #[allow(dead_code)] | 0.3 天 |
| **P2** | 重复逻辑 | 抽取公共函数 | 0.5 天 |
| **P3** | UI 命中区域 | 卡片操作按钮加大 | 0.2 天 |
| **P3** | 批量进度 | 增加进度事件 | 1 天 |
