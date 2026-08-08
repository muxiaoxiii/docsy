# MDG-017/018 修改验证报告（最终版）

> 验证日期：2026-08-08（二次验证）
> 验证范围：9593017..097b517（MDG-017 + MDG-018 全部 commit）
> 验证方法：cargo test + vitest + 代码审查

---

## 一、最终验证结果

| 验证点 | 上次结果 | 本次结果 | 变化 |
|--------|---------|---------|------|
| 1. P0 XML 注入修复 | ✅ 正确 | ✅ 正确 | 无变化 |
| 2. P0 模块边界迁移 | ⚠️ 12 处跨模块 import | ⚠️ **16 处**跨模块 import | ⬆ 增加了 4 处 |
| 3. P1 DocsyError 枚举 | ⚠️ 定义了没用上 | ⚠️ image_paddler 用了（2 命令），其余仍 String | ⬆ 部分改善 |
| 4. P1 TemplateView 拆分 | ✅ 356 行 | ✅ 356 行 | 无变化 |
| 5. P1 EvidencePdfWorkbench 拆分 | ⚠️ 耦合仍在 | ⚠️ 耦合仍在 | 无变化 |
| 6. P2 清理 | ✅ 基本完成 | ✅ **全部完成**（死 CSS 已清） | ⬆ 改善 |
| 7. 4 个失败测试 | ❌ 回归 | ✅ **全部修复**（82/82 passed） | ⬆ 修复 |
| 8. 裸 invoke | ⚠️ TemplateView 有 | ✅ **已修复**（0 处） | ⬆ 修复 |
| 9. 编译警告 | 未检查 | ✅ **0 warnings** | ⬆ 清零 |
| 10. 硬编码颜色 | ⚠️ 6 处 | ⚠️ image-paddler 边框色映射（可接受） | ⬆ 大部分修复 |

---

## 二、测试状态

```
Rust:   160 passed / 0 failed / 1 ignored  ✅
前端:    82 passed / 0 failed (14 files)   ✅
编译:      0 warnings                       ✅
```

**4 个测试回归已修复**。MDG-018 Phase 1 修复了 `buildHeaderText` 的旧 API 字段同步问题：
- `buildHeaderText` 现在正确同步 `rules.headerText` 和 `rules.headerPrefix` 到 group 对象
- `buildEvidencePdfRulePayload` 的 `evidenceLabel` 修复了 mode 覆盖逻辑

---

## 三、逐项详细验证

### 1. P0 XML 注入 — ✅ 正确（无变化）

`ooxml.rs` 用 `BytesText::from_escaped` + 手动转义 `&<>`，无遗漏注入点。

### 2. P0 模块边界 — ⚠️ 跨模块 import 反而增加了

**上次**：12 处 `../../pdf-tools/` 引用
**本次**：**16 处**（增加了 4 处）

新增的 4 处来自拆分出的 composables：
- `useFileOrdering.js:1` — `import { sortByNatural } from '../../pdf-tools/composables/useEvidencePdfSession.js'`
- `useContentRowEditing.js:10` — `from '../../pdf-tools/composables/useEvidencePdfSession.js'`
- `useContentRowEditing.js:11` — `from '../../pdf-tools/composables/pdfPageNumberRules.js'`
- `useHeaderFooterRules.js:9` — `from '../../pdf-tools/composables/useEvidencePdfSession.js'`

**本质问题未解决**：EvidencePdfWorkbench 及其 composables 仍然大量依赖 pdf-tools 的 4 个组件 + 8 个 composables。文件搬家了，但底层依赖未迁移，拆分 composable 反而增加了新的跨模块引用。

**根本修复建议**：将 pdf-tools 下被 evidence-pdf 依赖的共享代码（useEvidencePdfSession.js、pdfPageNumberRules.js、splitFileName.js 等 + 4 个组件）下沉到 `src/shared/pdf-tools/`，两个模块都从 shared 引用。

### 3. P1 DocsyError — ⚠️ 部分使用

**已迁移的命令**（2 个）：
- `image_paddler.rs` — `analyze_images` 和 `run_images` 返回 `Result<T, DocsyError>`

**未迁移的命令**（仍返回 `Result<T, String>`）：
- `pdf.rs` — 26 个命令全部返回 String
- `template.rs` — 26 个命令全部返回 String
- `system.rs` — 13 个命令全部返回 String
- `settings.rs` — 7 个命令全部返回 String
- `video.rs` — 4 个命令全部返回 String

**tauriBridge.js 已兼容**：`tauriBridge.js:79-84` 能解析 DocsyError 的 JSON 对象格式（`{ kind, message/reason }`），提取可读消息。这意味着 image_paddler 的错误前端能拿到结构化信息，但其他命令仍只能拿到 String。

**评估**：DocsyError 的基础设施（枚举定义 + thiserror + tauriBridge 兼容 + mod.rs 转换函数）已就绪，image_paddler 作为试点验证了流程。剩余 76 个命令的迁移是机械性工作，可后续逐步推进。

### 4. P1 TemplateView 拆分 — ✅ 质量好（无变化）

TemplateView.vue 356 行，useTemplateState.js 2579 行。拆分质量好，但 useTemplateState.js 偏大，建议后续按 Build/Render/History 三域再拆。

### 5. P1 EvidencePdfWorkbench 拆分 — ⚠️ 耦合仍在（无变化）

3 个 composable + 2 个组件已拆出，但 16 处跨模块 import 仍在。

### 6. P2 清理 — ✅ 全部完成

| 清理项 | 上次 | 本次 |
|--------|------|------|
| `#[allow(dead_code)]` 7 处 | ✅ 全清 | ✅ |
| TextOverlay 死代码 | ✅ 删除 | ✅ |
| SettingsView 死 CSS | ✅ 清除 | ✅ |
| EvidencePdfView 死 CSS | ❌ 残留 | ✅ **已清理** |

### 7. 4 个测试回归 — ✅ 全部修复

MDG-018 Phase 1（commit `2da491f`）修复了 `buildHeaderText`：

```js
// 修复后：同步旧 API 字段到 group
return buildHeaderTextForGroup(file, index, {
  mode,
  text: rules.headerText,
  prefix: rules.headerPrefix,
}, rules)
```

同时 `headerBaseTextForGroup` 的 custom 分支也做了 fallback：
```js
if (group.mode === 'custom' || group.mode === 'template')
  return (group.text || _rules.headerText) ?? ''
```

测试从 78 passed / 4 failed → **82 passed / 0 failed**。

### 8. 裸 invoke — ✅ 已修复

`src/modules/` 下搜索 `from '@tauri-apps/api/core'` 结果为空。TemplateView.vue 的 `invoke('get_log_file_path')` 已改为 tauriBridge。

### 9. 编译警告 — ✅ 清零

`cargo build` 输出 0 个 warning。MDG-018 最后一个 commit（`2b11edf`）专门做了警告清零。

### 10. 硬编码颜色 — ⚠️ 大部分修复

**已修复**：
- PdfToolsView.vue 的 4 处硬编码色 → 已迁移到 `--docsy-*` 变量
- VideoExtractView.vue 的 2 处硬编码色 → 已迁移
- DocletWorkingPet.vue 的 1 处硬编码色 → 已迁移
- DocumentPreview.vue 的不存在的 CSS 变量 → 已修复
- FilenameTokenInput.vue 的硬编码色 → 已迁移到 `--docsy-token-*` 变量

**仍存在**（可接受）：
- `useImagePaddlerState.js:345-352` — 边框颜色映射表（white/dark_gray/red/yellow/blue/black），这是用户可选的边框颜色值，不适合用 CSS 变量
- `ImagePaddlerView.vue:444` — `#fbfaf8` 预览背景，应该用 `--docsy-preview-paper`
- `ImagePaddlerView.vue:453` — `#303133` 预览边框，应该用 `--docsy-text-strong`

---

## 四、总结

### 已完成 ✅（7 项）

1. XML 注入修复
2. TemplateView 拆分（2651→356 行）
3. 4 个测试回归修复
4. 裸 invoke 修复
5. 编译警告清零
6. P2 清理全部完成（dead_code + TextOverlay + 死 CSS）
7. 硬编码颜色大部分迁移

### 部分完成 ⚠️（3 项）

8. **DocsyError** — 基础设施就绪，image_paddler 试点完成，剩余 76 个命令待迁移
9. **模块边界** — 文件已迁移，但 16 处跨模块 import 未解决（需下沉共享代码到 shared/）
10. **EvidencePdfWorkbench 拆分** — composable/组件已拆出，但耦合仍在

### 未引入新 bug

- Rust 160 passed / 0 failed
- 前端 82 passed / 0 failed
- 编译 0 warnings
- 无新的跨模块循环依赖
- 无新的安全漏洞

### 后续建议

| 优先级 | 建议 | 说明 |
|--------|------|------|
| P2 | 下沉共享代码到 `src/shared/pdf-tools/` | 解决 16 处跨模块 import 的根本方案 |
| P2 | 迁移剩余 76 个命令到 DocsyError | 机械性工作，可分批进行 |
| P3 | useTemplateState.js 进一步拆分 | 2579 行按 Build/Render/History 三域拆 |
| P3 | ImagePaddlerView 预览色用变量 | `#fbfaf8`→`--docsy-preview-paper`，`#303133`→`--docsy-text-strong` |
