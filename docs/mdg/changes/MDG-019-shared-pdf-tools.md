# MDG-019: 跨模块 import 下沉 — 共享 PDF 工具代码迁移到 shared/

## 状态：🟢 已完成
## 优先级：P2
## 来源：MDG-017/018 遗留 — evidence-pdf 模块 16 处跨模块 import pdf-tools

---

## 一、修改目标

evidence-pdf 模块的 16 处 `../../pdf-tools/` 跨模块 import 是模块边界破裂的根本原因。将 pdf-tools 中被 evidence-pdf 依赖的 15 个共享文件下沉到 `src/shared/pdf-tools/`，两个模块都从 shared 引用，消除跨模块依赖。

同时清理 MDG-017 遗留的旧位置残留文件 `src/modules/pdf-tools/views/EvidencePdfWorkbench.vue`（已被迁移到 evidence-pdf/components/ 但旧文件未删除）。

## 二、文件清单（15 个文件）

### Composables（10 个）
| # | 文件 | 依赖（列表内） | 依赖 core/ |
|---|------|---------------|-----------|
| 1 | useEvidencePdfSession.js | 6,7 | filePath, numberFormat, unitConversion |
| 2 | useEvidencePdfPreview.js | 1,3,7 | tauriBridge, appLogger |
| 3 | useEvidencePdfDetection.js | 1,8 | tauriBridge |
| 4 | useEvidencePdfMergedImport.js | 1,3,6 | filePath, tauriBridge, pdfUtils |
| 5 | useEvidencePdfExistingEditing.js | 1 | filePath |
| 6 | splitFileName.js | — | numberFormat |
| 7 | pdfPageNumberRules.js | — | numberFormat |
| 8 | existingPdfElements.js | — | — |
| 9 | pdfPreviewCoordinates.js | — | — |
| 10 | usePdfSplitRanges.js | — | — |

### Components（5 个）
| # | 文件 | 依赖（列表内） | 依赖 core/ |
|---|------|---------------|-----------|
| 11 | PdfJsPreview.vue | — | tauriBridge |
| 12 | HeaderFooterRuleFields.vue | 7 | useHistory, UndoRedoButtons |
| 13 | PageNumberRuleDialog.vue | 7 | — |
| 14 | ExistingPdfElementsDialog.vue | 1,8 | — |
| 15 | TextPlacementFields.vue | — | — |

### 测试文件（5 个，随源文件迁移）
- useEvidencePdfSession.test.js
- useEvidencePdfDetection.test.js
- existingPdfElements.test.js
- pdfPageNumberRules.test.js
- splitFileName.test.js

## 三、影响分析

### 路径深度变化
| 位置 | 当前 core/ 路径 | 下沉后 core/ 路径 |
|------|---------------|-----------------|
| composables/ | `../../../core/` (3 层) | `../../core/` (2 层) |
| components/ | `../../../core/` (3 层) | `../../core/` (2 层) |

### 需要更新 import 的文件
| 文件 | 更新数量 | 说明 |
|------|---------|------|
| evidence-pdf/components/EvidencePdfWorkbench.vue | ~16 处 | `../../pdf-tools/` → `../../../shared/pdf-tools/` |
| evidence-pdf/composables/useHeaderFooterRules.js | ~3 处 | `../../pdf-tools/` → `../../../shared/pdf-tools/` |
| evidence-pdf/composables/useContentRowEditing.js | ~2 处 | 同上 |
| evidence-pdf/composables/useFileOrdering.js | ~1 处 | 同上 |
| pdf-tools/views/PdfToolsView.vue | ~1 处 | `../components/PdfJsPreview` → `../../../shared/pdf-tools/components/PdfJsPreview` |

### 不需要更新的文件
- pdf-tools/index.js — 不引用这 15 个文件
- pdf-tools/views/PdfToolsView.vue 中的其他 import — 不引用这 15 个文件（除 PdfJsPreview）
- core/ 下的文件 — 路径方向不变
- src/styles.css — 不受影响

### 旧文件清理
- `src/modules/pdf-tools/views/EvidencePdfWorkbench.vue` — 删除（MDG-017 残留，123KB，git 跟踪中）

## 四、移动策略

```
src/shared/pdf-tools/
├── composables/
│   ├── useEvidencePdfSession.js (+ test)
│   ├── useEvidencePdfPreview.js
│   ├── useEvidencePdfDetection.js (+ test)
│   ├── useEvidencePdfMergedImport.js
│   ├── useEvidencePdfExistingEditing.js
│   ├── splitFileName.js (+ test)
│   ├── pdfPageNumberRules.js (+ test)
│   ├── existingPdfElements.js (+ test)
│   ├── pdfPreviewCoordinates.js
│   └── usePdfSplitRanges.js
└── components/
    ├── PdfJsPreview.vue
    ├── HeaderFooterRuleFields.vue
    ├── PageNumberRuleDialog.vue
    ├── ExistingPdfElementsDialog.vue
    └── TextPlacementFields.vue
```

### 执行步骤
1. 创建 `src/shared/pdf-tools/composables/` 和 `src/shared/pdf-tools/components/` 目录
2. `git mv` 移动 15 个源文件 + 5 个测试文件
3. 更新 15 个源文件内部的 import 路径（`../../../core/` → `../../core/`，`../../../components/UndoRedoButtons.vue` → `../../components/UndoRedoButtons.vue`，`../../../services/appLogger.js` → `../../services/appLogger.js`）
4. 更新 EvidencePdfWorkbench.vue 的 import 路径
5. 更新 evidence-pdf 3 个 composables 的 import 路径
6. 更新 PdfToolsView.vue 的 PdfJsPreview import 路径
7. 删除旧位置残留 `src/modules/pdf-tools/views/EvidencePdfWorkbench.vue`
8. `cargo test` + `npm test` 验证
9. `npm run build` 验证构建

## 五、测试计划
- [ ] `npm test` 82/82 passed
- [ ] `cargo test` 160/160 passed
- [ ] `npm run build` 成功
- [ ] grep 确认 0 处 `../../pdf-tools/` 残留

## 六、回退方案
- 所有文件移动用 `git mv`，可通过 `git revert` 回退
- 每个 step 独立 commit，失败可回退

## 七、变更日志
- 2026-08-08 12:32 — 变更单创建，深入设计完成
- 2026-08-08 12:35 — 实施完成：22 文件移动 + 路径更新 + 旧文件清理
- 2026-08-08 12:36 — 验证通过：Rust 160/0、前端 82/0、Vite build 成功
- 2026-08-08 12:36 — Commit 2ecc1ed
- 额外修复：diagnostics.js 已有路径 bug（`./tauriBridge.js` → `../core/tauriBridge.js`）
