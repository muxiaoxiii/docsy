# MDG-017 修改验证报告

> 验证日期：2026-08-08
> 验证范围：9593017..HEAD（MDG-017 全部 12 个 commit）
> 验证方法：cargo test + vitest + 代码审查

---

## 一、验证结果总览

| 验证点 | 结果 | 详情 |
|--------|------|------|
| 1. P0 XML 注入修复 | ✅ **正确** | `BytesText::from_escaped` + 手动转义 `&<>`，无遗漏 |
| 2. P0 模块边界迁移 | ⚠️ **部分完成** | 文件已迁移，但 12 处跨模块 import 仍在 |
| 3. P1 DocsyError 枚举 | ⚠️ **定义了但没用上** | 7 个变体 + thiserror，但命令层仍全返回 String |
| 4. P1 TemplateView 拆分 | ✅ **质量好** | 2651→356 行，但 useTemplateState.js 2579 行偏大 |
| 5. P1 EvidencePdfWorkbench 拆分 | ⚠️ **拆分了但耦合仍在** | 3 个 composable + 2 个组件，12 处跨模块 import |
| 6. P2 清理 | ✅ **基本完成** | dead_code 全清、TextOverlay 删除、Settings 死 CSS 清除 |
| 7. 4 个失败测试 | ❌ **MDG-017 引入的回归** | 改了 buildHeaderText 但没同步旧 API 字段 |

---

## 二、逐项详细验证

### 1. P0 XML 注入 — ✅ 正确

- `ooxml.rs:191` 用 `BytesText::from_escaped` + 手动转义 `&`→`&amp;`、`<`→`&lt;`、`>`→`&gt;`
- 全代码库无 `BytesText::new` 残留
- XML 文本节点按规范只需转义 `&` 和 `<`，`>` 为惯例，`"`/`'` 仅属性值需要——当前用法正确
- `push_attribute` 自动转义属性值，安全
- **无遗漏注入点**

### 2. P0 模块边界 — ⚠️ 部分完成

**已完成**：
- `EvidencePdfWorkbench.vue` 已迁移到 `src/modules/evidence-pdf/components/`
- `src/modules/pdf-tools/views/` 下无残留
- 新增 3 个 composable（useHeaderFooterRules、useContentRowEditing、useFileOrdering）+ 2 个组件（EvidenceMergedImportPlan、EvidenceOverlayTable）
- `EvidencePdfView.vue` 自身无跨模块 import

**未完成**：
- `EvidencePdfWorkbench.vue` 仍有 **12 处** `../../pdf-tools/` 引用（PdfJsPreview、HeaderFooterRuleFields 等 4 个组件 + 8 个 composables）
- 3 个新拆分的 composable 也全部依赖 `../../pdf-tools/composables/useEvidencePdfSession.js`
- 本质上是"文件搬家 + 组件拆分"，底层依赖未迁移，跨模块耦合反而更分散

### 3. P1 DocsyError — ⚠️ 定义了但没用上

**已完成**：
- `src-tauri/src/error.rs` 定义了 7 个变体（FileNotFound、ToolMissing、PdfFailed、Cancelled、TemplateFailed、InvalidArgument、Unknown）
- thiserror 2 已引入 Cargo.toml
- 实现 `From<anyhow::Error>` + Serialize
- `commands/mod.rs` 的 `anyhow_to_json_string` 用 DocsyError 做中间转换

**未完成**：
- 所有命令文件（pdf、template、settings、system、video、image_paddler）签名**仍是 `Result<T, String>`**，无一例外
- DocsyError 仅在 mod.rs 内部用作中间转换类型，命令层未直接暴露
- 距离"前端能区分错误类型"的目标还差一步

### 4. P1 TemplateView 拆分 — ✅ 质量好

- TemplateView.vue：2651 → **356 行**（瘦身 86%）
- 新增 `useTemplateState.js`（2579 行）作为中央编排者
- 职责划分清晰：TemplateView 只做 Tab 布局 + props/events 透传
- **遗留**：useTemplateState.js 2579 行偏大，建议进一步拆分为 useTemplateBuild/Render/History

### 5. P1 EvidencePdfWorkbench 拆分 — ⚠️ 拆分了但耦合仍在

- 拆出 3 个 composable + 2 个组件，拆分方向正确
- 但所有 composable 依赖 `../../pdf-tools/composables/useEvidencePdfSession.js`（含 sortByNatural、createDefault* 等共享核心）
- 跨模块耦合未消除，只是从"视图级直连"变成了"composable 级直连"

### 6. P2 清理 — ✅ 基本完成

| 清理项 | 状态 |
|--------|------|
| `#[allow(dead_code)]` 7 处 | ✅ 全部清除（仅剩 package.rs:27 有合理注释） |
| `AntiCopyMethod::TextOverlay` 死代码 | ✅ 彻底删除（零命中） |
| SettingsView 死 CSS（.bundle-actions / .export-options） | ✅ 已清除 |
| **EvidencePdfView 死 CSS（.group-files / .group-file）** | ❌ **仍残留，漏清** |

### 7. 4 个失败测试 — ❌ MDG-017 引入的回归

**事实**：MDG-017 之前测试全通过（35/35），之后 4 个失败。**这些失败是 MDG-017 引入的，不是"已有测试问题"**。

**根因分析**：MDG-017 重写了 `buildHeaderText`（useEvidencePdfSession.js:325-332）：

```js
// 旧实现：用 rules.headerMode + rules.headerText
export function buildHeaderText(file, index, rules) {
  if (file?.headerEdited) return decorateHeaderText(file.header ?? '', ...)
  if (rules.headerMode === 'none') return ''
  const base = headerBaseText(file, index, rules)  // 用 rules.headerText
  return decorateHeaderText(base, file, index, rules)
}

// 新实现：委托给 buildHeaderTextForGroup
export function buildHeaderText(file, index, rules) {
  const group = selectedGroupFor(file, 'header')
  if (!group) return ''
  const mode = rules.headerMode !== undefined ? rules.headerMode : group.mode
  return buildHeaderTextForGroup(file, index, { ...group, mode }, rules)
}
```

**问题**：新实现只覆盖了 `mode`，但没有把 `rules.headerText`、`rules.headerPrefix`、`rules.headerSuffix` 等"旧 API 字段"同步到 group 对象中。`buildHeaderTextForGroup` 内部用的是 `group.text` 而非 `rules.headerText`。

**两个具体失败**：

**失败 1** — `builds a business-level evidence PDF rules payload`（line 460）：
- 测试设置 `file.header = '证据1 合同'`，`rules.headerMode = 'per_file'`
- 期望 `evidenceLabel = '证据1 合同'`
- 实际得到 `'合同'`
- 原因：`buildEvidencePdfRulePayload` 改用 `buildHeaderTextForGroup(file, index, selectedGroupFor(file, 'header'), rules)`，但 `selectedGroupFor` 返回默认 group（mode='filename'），没有用 `rules.headerMode` 覆盖 mode，所以走了 `filename` 分支返回 `stripPdf(file.name)` = `'合同'`

**失败 2** — `keeps common header naming modes deterministic`（line 509）：
- 测试设置 `rules.headerMode = 'custom'`，`rules.headerText = '固定说明'`
- 期望 `'固定说明'`
- 实际得到 `''`
- 原因：`buildHeaderText` 新实现用 `{ ...group, mode }` 覆盖了 mode 为 `'custom'`，但 `headerBaseTextForGroup` 中 `custom` 分支返回 `group.text || ''`，而 group.text 是默认空字符串，不是 `rules.headerText`

**修复建议**：

方案 A（推荐）：在 `buildHeaderText` 中同步旧 API 字段到 group：

```js
export function buildHeaderText(file, index, rules) {
  const group = selectedGroupFor(file, 'header')
  if (!group) return ''
  const mode = rules.headerMode !== undefined ? rules.headerMode : group.mode
  // 同步旧 API 字段到 group
  const mergedGroup = {
    ...group,
    mode,
    text: rules.headerText ?? group.text,
    prefix: rules.headerPrefix ?? group.prefix,
    suffix: rules.headerSuffix ?? group.suffix,
  }
  return buildHeaderTextForGroup(file, index, mergedGroup, rules)
}
```

方案 B：`buildEvidencePdfRulePayload` 中的 `evidenceLabel` 改回用 `buildHeaderText`：

```js
evidenceLabel: buildHeaderText(file, index, rules),
```

建议两个都改——方案 A 修复 `buildHeaderText` 的行为一致性，方案 B 修复 `buildEvidencePdfRulePayload` 的 mode 覆盖问题。

---

## 三、其他发现

### EvidencePdfView 死 CSS 漏清

`.group-files` 和 `.group-file` CSS 类（EvidencePdfView.vue:188-199）模板中无任何引用，是重构残留，MDG-017 遗漏未清。

### image_paddler.rs 拆分质量

`image_paddler.rs` 从 1587 行变为包含 `LayoutContext` 结构体的版本，17 参数函数问题已解决。新增 219 行变更，测试通过。

### anti_ocr 检测并行化

`anti_ocr.rs` 删除 40 行（TextOverlay 死代码），但"检测串行→并行"的改动需要验证——当前测试通过但可能未覆盖并发场景。

---

## 四、总结

| 类别 | 数量 | 评估 |
|------|------|------|
| ✅ 正确完成 | 3 | XML 注入、TemplateView 拆分、P2 清理 |
| ⚠️ 部分完成 | 3 | 模块迁移（12 处跨模块 import 仍在）、DocsyError（命令层未用）、Workbench 拆分（耦合仍在） |
| ❌ 引入回归 | 1 | 4 个测试失败（buildHeaderText 旧 API 字段未同步） |
| 漏清 | 1 | EvidencePdfView 死 CSS |

**最紧急**：4 个测试失败是真实的代码回归，不是测试问题。`buildHeaderText` 新实现没有正确处理旧 API 的 `rules.headerText` / `rules.headerPrefix` / `rules.headerSuffix` 字段，需要按方案 A 修复。

**次紧急**：`buildEvidencePdfRulePayload` 的 `evidenceLabel` 应该用 `buildHeaderText`（有 mode 覆盖逻辑）而非直接用 `buildHeaderTextForGroup`（无 mode 覆盖）。
