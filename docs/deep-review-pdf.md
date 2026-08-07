# PDF 工具模块深度审阅报告

> 审阅范围：`src/modules/pdf-tools/`（19 个前端文件）+ `src/modules/evidence-pdf/`（2 个前端文件）+ `src-tauri/src/pdf/` + `src-tauri/src/commands/pdf.rs` + `src-tauri/src/external/`（18 个 Rust 后端文件）
>
> 审阅日期：2026-08-07

---

## 目录

1. [前端文件逐文件分析](#1-前端文件逐文件分析)
2. [Rust 后端文件逐文件分析](#2-rust-后端文件逐文件分析)
3. [PDF 工具模块完整数据流](#3-pdf-工具模块完整数据流)
4. [evidence-pdf 会话管理机制](#4-evidence-pdf-会话管理机制)
5. [所有发现的 Bug 汇总](#5-所有发现的-bug-汇总)

---

## 1. 前端文件逐文件分析

### 1.1 `src/modules/pdf-tools/views/PdfToolsView.vue`

**行数：** 1155

**功能概述：** PDF 工具主视图，提供 6 个 Tab 页签：解锁、合并、提取页面、压缩、拆分、防复制。支持文件拖放、PDF 预览、页段管理。

**导出/组件：**
- `<script setup>` 内部组件，无显式导出。所有函数为组件内部使用。

**关键函数：**
| 函数 | 作用 |
|---|---|
| `selectUnlockFiles()` | 选择加密 PDF 文件 |
| `inspectUnlockFiles(paths)` | 并行检测 PDF 加密状态（最多 4 个 worker） |
| `batchUnlock()` | 批量解锁加密 PDF |
| `selectMergeFiles()` / `doMerge()` | PDF 合并 |
| `doExtractPages()` | 按页码提取页面 |
| `doCompressPdf()` | PDF 压缩 |
| `loadSplitFile(path)` / `doSplitMerged()` | 拆分功能 |
| `batchAntiOcrApply()` / `batchAntiOcrRemove()` | 防复制添加/移除 |
| `handleDroppedPdfPaths(paths)` | 拖放文件处理 |

**依赖：**
- `@tauri-apps/plugin-dialog`（文件选择）
- `PdfJsPreview`（PDF 预览组件）
- `FileQueuePanel`、`ToolWorkspaceShell`（共享 UI 组件）
- `tauriBridge.js`（Tauri IPC 桥接）
- `usePdfSplitRanges.js`（拆分范围校验）
- `usePointerReorder`（拖拽排序）
- `useWindowFileDrop`（窗口拖放）

**问题：**
- **L370-408 `inspectUnlockFiles`：** 使用闭包变量 `nextIndex` 做并发控制，非原子操作。虽然当前是单线程 JS 环境不会出问题，但模式不够清晰。
- **L555-568 `batchAntiOcrApply`：** 处理超时 30 秒硬编码，无法配置。大文件可能超时。
- **L474 `splitWarnings`：** computed 依赖 `splitRunWarnings.value`，但 `splitRunWarnings` 在拆分执行后才更新，中间状态可能不一致。

---

### 1.2 `src/modules/pdf-tools/views/EvidencePdfWorkbench.vue`

**行数：** 2600+（截断读取到 2584 行）

**功能概述：** 证据 PDF 处理核心工作台。支持分项证据处理（merge 模式）和合并证据处理（split 模式）。功能包括：页眉页脚插入、页码编号、A4 规范化、批注删除、PDF 书签、现有页眉页脚检测与编辑、合并导入拆分、真实预览。

**导出/组件：**
- `props: { workflow: String }` — `'all'` | `'merge'` | `'split'`
- 组件内部管理极其复杂的状态（100+ 个 ref/computed）

**关键 composables 使用：**
| Composable | 作用 |
|---|---|
| `useEvidencePdfDetection` | 页眉页脚检测 |
| `useEvidencePdfPreview` | 预览渲染 |
| `useEvidencePdfMergedImport` | 合并 PDF 导入拆分 |
| `useEvidencePdfExistingEditing` | 现有页眉页脚编辑 |

**依赖：**
- `@tauri-apps/plugin-fs`（文件存在检查）
- `@tauri-apps/plugin-dialog`
- 4 个子组件：`PdfJsPreview`、`HeaderFooterRuleFields`、`PageNumberRuleDialog`、`ExistingPdfElementsDialog`
- 5 个 composables
- `usePointerReorder`、`useHistory`

**问题：**
- **L868-897 `parsePageNumberValue`：** 罗马数字解析中，`vals[upper[i + 1]]` 在 `i === upper.length - 1` 时访问越界返回 `undefined`（值为 0），逻辑正确但不够显式。
- **L933 `quickCleanupPipeline`：** 使用普通变量（非 ref）控制异步流程，依赖 `watch(existingElementsVisible)` 触发后续操作。如果 dialog 关闭事件和 watch 执行有时序问题，可能丢失 pipeline。
- **L1226-1230 `pushEditUndo`：** undo 栈上限 30 硬编码，无法配置。
- **L1293-1311 `watch(selectedHeaderGroup.mode)`：** 使用 `{ immediate: true }` 且在 watch 内修改 `group.mode`，可能触发循环更新。
- **L2576 `reorderOverlayFiles`：** `selectedOverlayIndex.value = selectedPath ? Math.max(0, items.findIndex(...)) : to` — 当 `selectedPath` 找不到时返回 -1，`Math.max(0, -1)` = 0，可能导致意外选中第一个文件。

---

### 1.3 `src/modules/pdf-tools/components/ExistingPdfElementsDialog.vue`

**行数：** 352

**功能概述：** 现有 PDF 元素确认对话框。展示检测到的页眉/页脚/页码元素，支持批量操作（保留/忽略/删除/编辑）、Shift 范围选择、按序列选择、排序筛选。

**Props/Emits：**
- `visible: Boolean`、`rows: Array`、`filter: String`
- emits: `update:visible`、`change`、`preview`、`jump-to-settings`

**关键函数：**
| 函数 | 作用 |
|---|---|
| `handleRowClick(row, col, event)` | 支持 Shift+click 范围选择 |
| `selectBySequence(row)` | 按页码连续序列或同文本选择 |
| `applyDecision(decision)` | 批量应用决策 |
| `setDecision(row, decision)` | 设置单个元素决策 |

**依赖：**
- `existingPdfElements.js`（元素文本/类型映射）
- `useEvidencePdfSession.js`（naturalCompare 排序）

**问题：**
- **L112-119：** 使用 `window.addEventListener` 监听 Shift 键，在 `onUnmounted` 中移除。但如果组件被 keep-alive 缓存，unmount 不会触发，导致事件监听器泄漏。
- **L297 `setDecision`：** `row.element.decision = decision` 直接修改 prop 数据（通过对象引用），违反 Vue 单向数据流。虽然因为是对象引用所以能工作，但不够规范。

---

### 1.4 `src/modules/pdf-tools/components/HeaderFooterRuleFields.vue`

**行数：** 878

**功能概述：** 页眉页脚页码规则配置组件。支持多组（group）配置、文本模板、位置/字号/字体/颜色设置、页码格式预设、重叠检测、undo/redo。

**Props：** 30+ 个 props，涵盖 header、footerText、pageNumber 三组配置。

**关键功能：**
- 多组管理（addGroup/removeGroup）
- 模板标记系统（`[文件名]`、`[序号]`、`[中文序号]`、`[#]`、`[日期]`）
- 页码格式预设（`{page}/{total}`、`第{page}页` 等）
- 重叠检测（header-header、header-pageNumber、footer-pageNumber）
- Undo/Redo 历史（使用 `useHistory` composable）

**依赖：**
- `TextPlacementFields`（位置配置子组件）
- `pdfPageNumberRules.js`（页码样式和模板渲染）
- `useHistory`（撤销重做）

**问题：**
- **L347-353 `debounce`：** 自定义 debounce 实现，没有 cancel 机制。组件卸载时 timer 仍可能触发。
- **L603-609 `pageNumberPresetModel`：** setter 为空函数 `set() {}`，意味着用户无法通过预设 radio 直接设置自定义值。
- **L671 `addGroup`：** 使用 `Date.now()` 生成 group ID，如果快速连续添加可能产生相同 ID（概率极低）。

---

### 1.5 `src/modules/pdf-tools/components/PageNumberRuleDialog.vue`

**行数：** 112

**功能概述：** 页码分段与例外规则对话框。支持按全局/文件内页码范围设置排除或覆盖规则。

**Props/Emits：**
- `visible: Boolean`、`rules: Array`
- emits: `update:visible`、`update:rules`

**问题：**
- **L96 `watch(localRules, ...)` 使用 `{ deep: true }`：** 每次规则内部任何字段变化都会触发 emit，可能导致频繁更新。

---

### 1.6 `src/modules/pdf-tools/components/TextPlacementFields.vue`

**行数：** 65

**功能概述：** 文本位置配置字段组件。提供对齐、字号、字体、边距、水平偏移、颜色的统一配置 UI。

**Props：** `prefix`、`disabled`、`align`、`fontSize`、`fontFamily`、`marginMm`、`offsetXMm`、`color`、`offsetLimitMm`、`marginLabel`

**问题：** 无明显问题。组件简洁、职责单一。

---

### 1.7 `src/modules/pdf-tools/components/PdfJsPreview.vue`

**行数：** 290

**功能概述：** PDF 页面预览组件。支持两种渲染引擎：后端渲染（pdftoppm）和前端渲染（pdf.js）。提供 canvas 和 image 两种显示模式，支持 slot 叠加层。

**Props：** `filePath`、`page`、`scale`、`reloadKey`、`engine`（`'auto'` | `'backend'`）

**关键函数：**
| 函数 | 作用 |
|---|---|
| `renderCurrentPage()` | 主渲染入口，按引擎策略调用 |
| `renderWithBackend(requestId)` | 后端渲染（pdftoppm） |
| `renderWithPdfJs(requestId)` | 前端渲染（pdf.js canvas） |
| `loadDocument(path, reloadKey)` | 加载 PDF 文档（带缓存） |
| `cancelRender()` | 取消进行中的渲染 |

**依赖：**
- `@tauri-apps/plugin-fs`（readFile）
- `pdfjs-dist`（动态导入）
- `tauriBridge.js`

**问题：**
- **L179-198 `loadDocument`：** 文档缓存使用 `__docsyKey` 自定义属性挂在 pdf.js Document 对象上。如果 pdf.js 内部也使用此属性名会冲突（概率极低）。
- **L186 `await pdfDoc.value.destroy()`：** `destroy()` 返回 void，`await` 无意义但无害。
- **L201-211 `loadPdfJs`：** pdf.js 库全局缓存（模块级变量），worker URL 也全局设置。多个组件实例共享同一 worker，这是预期行为。

---

### 1.8 `src/modules/pdf-tools/composables/useEvidencePdfSession.js`

**行数：** 1043

**功能概述：** 证据 PDF 会话核心数据层。管理证据文件列表、页码范围计算、页眉页脚文本构建、输出路径生成、payload 构建（前端→后端）。是整个 evidence-pdf 系统的中枢。

**关键导出函数：**
| 函数 | 作用 |
|---|---|
| `createEvidenceFile(path)` | 创建证据文件对象（40+ 字段） |
| `createDefaultHeaderGroup()` | 创建默认页眉组 |
| `createDefaultFooterTextGroup()` | 创建默认页脚文字组 |
| `createDefaultPageNumberGroup()` | 创建默认页码组 |
| `groupsFor(file, kind)` | 获取文件的某类组列表 |
| `selectedGroupFor(file, kind)` | 获取当前选中的组 |
| `buildHeaderText(file, index, rules)` | 构建页眉文本 |
| `buildHeaderTextForGroup(file, index, group, rules)` | 按组构建页眉文本 |
| `resolveTextTemplate(text, file, index, rules)` | 解析文本模板标记 |
| `buildHeaderFooterItems(files, rules, outputDir)` | 构建完整的 overlay 配置列表 |
| `buildEvidencePdfRulePayload(files, rules, outputDir)` | 构建发送给后端的完整 payload |
| `assignPageRanges(files)` / `updatePageRanges(files)` | 计算连续页码范围 |
| `buildOverlayOutputPath(...)` | 生成输出文件路径 |
| `naturalCompare(left, right)` | 自然排序比较器 |
| `sortByNatural(items, valueGetter, order)` | 自然排序 |
| `buildFileContentRows(file, index, rules)` | 构建展开行数据 |
| `expandPlaceholders(template, page, total, file, index, rules)` | 展开页码占位符 |

**依赖：**
- `splitFileName.js`（日期/序号 token 展开）
- `filePath.js`（文件名/目录工具）
- `numberFormat.js`（中文数字）
- `unitConversion.js`（pt↔mm）
- `pdfPageNumberRules.js`（页码覆盖规则）

**问题：**
- **L125-188 `createEvidenceFile`：** 返回对象有 50+ 个属性，结构扁平。应考虑拆分为子对象（如 `existing.header`、`existing.footer`）。
- **L469-642 `buildHeaderFooterItems`：** 函数体 170+ 行，职责过多（计算页码范围、构建 header/footer overlay、处理 existing 元素、构建 cleanup 配置、构建 bookmarks）。
- **L504 `rules.headerMode !== undefined ? rules.headerMode : headerGroup?.mode`：** 三元表达式嵌套在更大的条件链中，可读性差。
- **L770 `inferDetectedFontSize`：** CJK 字符 ratio 0.58、Latin 0.72 是经验值，无文档说明来源。

---

### 1.9 `src/modules/pdf-tools/composables/useEvidencePdfDetection.js`

**行数：** 503

**功能概述：** 证据 PDF 页眉页脚检测 composable。封装检测流程（调用后端 `detect_pdf_header_footer`）、结果解析、候选评分、角色分配。

**关键导出函数：**
| 函数 | 作用 |
|---|---|
| `detectAllHeaderFooter(options)` | 批量检测所有文件的页眉页脚 |
| `applyDetectionResultToFile(file, data)` | 将检测结果应用到文件对象 |
| `isPageNumberCandidate(candidate)` | 判断是否为页码候选 |
| `bestReliableHeaderCandidate(candidates, totalPages)` | 选择最佳页眉候选 |
| `bestReliablePageNumberCandidate(candidates, totalPages)` | 选择最佳页码候选 |
| `pageNumberCandidateScore(candidate)` | 页码候选评分 |
| `fileExistingStatus(file)` | 获取文件现有元素状态 |
| `hasExistingHeader/Footer/PageNumber(row)` | 检查是否存在现有元素 |

**依赖：**
- `tauriBridge.js`（Tauri IPC）
- `existingPdfElements.js`（元素转换和合并）
- `useEvidencePdfSession.js`（candidateTargetRange）

**问题：**
- **L39-44：** 检测进度使用 `setInterval` 更新，但 `clearInterval` 在 `finally` 中调用。如果 `detectAllHeaderFooter` 被多次调用（虽然有 guard），timer 可能泄漏。
- **L197-205 `bestReliableHeaderCandidate`：** 单页文件（`totalPages <= 1`）直接返回第一个候选，不做可靠性检查。这可能导致不可靠的候选被选中。

---

### 1.10 `src/modules/pdf-tools/composables/useEvidencePdfPreview.js`

**行数：** 419

**功能概述：** 预览渲染 composable。管理实时预览（pdf.js 叠加层）和真实预览（后端渲染）两种模式。计算文本样式、删除标记、转换标记、溢出警告。

**关键导出：**
| 符号 | 作用 |
|---|---|
| `showRulePreviewOverlays` | 是否显示规则预览叠加层 |
| `previewHeaderText/FooterText` | 预览页眉/页脚文本 |
| `previewHeaderStyle/FooterStyle` | 预览样式 |
| `deletionPreviewMarkers` | 删除区域标记 |
| `convertedExistingPreviewOverlays` | 转换后的现有元素叠加层 |
| `headerFooterOverflowWarnings` | 文本溢出警告 |
| `refreshPreview()` / `renderTruePreview()` | 刷新/生成真实预览 |

**问题：**
- **L338-342 `estimateTextWidthPt`：** 逐字符宽度估算，CJK 字符宽度 = fontSize × 1.0，Latin = fontSize × 0.56。这是粗略估算，实际宽度取决于字体度量。
- **L185-227 `convertedExistingPreviewOverlays`：** 每次预览刷新都调用 `buildHeaderFooterItems`（遍历所有文件），性能开销较大。

---

### 1.11 `src/modules/pdf-tools/composables/useEvidencePdfExistingEditing.js`

**行数：** 274

**功能概述：** 现有页眉页脚编辑 composable。管理编辑状态（哪个文件的哪个字段正在编辑）、开始/完成编辑逻辑、编辑后状态同步。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `startHeaderEdit/finishHeaderEdit` | 页眉编辑 |
| `startExistingHeaderEdit/finishExistingHeaderEdit` | 现有页眉编辑 |
| `startExistingFooterEdit/finishExistingFooterEdit` | 现有页脚编辑 |
| `startExistingPageNumberEdit/finishExistingPageNumberEdit` | 现有页码编辑 |
| `displayRowHeader/DisplayRowFooter` | 显示用文本 |

**问题：**
- **L185 `finishExistingPageNumberEdit`：** 条件 `row.existingFooterArtifact && !row.existingFooterText` 将页码编辑结果同时写入 footer 和 pageNumber 字段。这个交叉赋值逻辑复杂且容易出错。

---

### 1.12 `src/modules/pdf-tools/composables/useEvidencePdfMergedImport.js`

**行数：** 491

**功能概述：** 合并 PDF 导入拆分 composable。管理合并 PDF 的导入、自动页段识别、手动调整、拆分执行、批量导入。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `importMergedPdfAsEvidence()` | 导入合并 PDF 并自动识别页段 |
| `executeMergedImportPlan()` | 执行拆分 |
| `selectMergedImportRange(row)` | 选择页段并跳转预览 |
| `formatSplitOutputName(row, index)` | 格式化拆分输出文件名 |

**依赖：**
- `splitFileName.js`（文件名格式化）
- `usePdfSplitRanges.js`（范围校验）
- `useEvidencePdfDetection.js`（检测区域配置）

**问题：**
- **L19 `MERGED_IMPORT_AUTO_SCAN_PAGES = 300`：** 硬编码上限，超过 300 页的合并 PDF 只扫描前 300 页。用户可能不知道后续页段未被分析。

---

### 1.13 `src/modules/pdf-tools/composables/existingPdfElements.js`

**行数：** 103

**功能概述：** 现有 PDF 元素数据模型。定义元素类型、决策类型、身份标识、合并逻辑。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `detectedElementFromCandidate(candidate, kind, index)` | 从候选创建检测元素 |
| `candidateIdentity(candidate)` | 生成候选唯一标识 |
| `mergeExistingElements(previous, detected)` | 合并新旧检测结果（保留旧决策） |
| `elementIdentity(element)` | 生成元素唯一标识 |
| `elementDecisionText/KindText(decision/kind)` | 中文显示文本 |
| `actionableExistingElements(file, kinds)` | 获取需要操作的元素 |

**问题：**
- **L12-17 `lowConfidence` 判断：** `confidence < 0.3 || isSinglePageContentText` — 单页文件的非页码 content-text 候选一律标记为低置信度。这可能导致单页 PDF 的页眉页脚始终需要手动确认。

---

### 1.14 `src/modules/pdf-tools/composables/pdfPageNumberRules.js`

**行数：** 142

**功能概述：** 页码规则引擎。定义页码样式（阿拉伯数字、中文、罗马、带圈、实心带圈）、模板渲染、覆盖规则匹配、per-file overlay 生成。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `formatPageNumber(value, style)` | 格式化页码数字 |
| `renderPageNumberTemplate(template, page, total, style)` | 渲染页码模板 |
| `effectivePageNumberRule(baseRule, globalPage, localPage)` | 计算当前页的有效规则（含覆盖） |
| `pageNumberOverlaysForFile(file, baseRule)` | 为文件生成所有页码 overlay 配置 |

**问题：**
- **L3 `CIRCLED` 数组：** 索引 0 为空字符串，1-20 有值。`formatPageNumber` 对 >20 的值回退到数字字符串，这是预期行为。

---

### 1.15 `src/modules/pdf-tools/composables/pdfPreviewCoordinates.js`

**行数：** 98

**功能概述：** PDF 预览坐标转换工具。将 PDF 点坐标和毫米单位转换为 CSS 百分比定位。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `ptToPercent(pt, dimensionPt)` | 点→百分比 |
| `mmToPercent(mm, dimensionPt)` | 毫米→百分比 |
| `textOverlayStyle(kind, pageInfo, config)` | 计算文本叠加层 CSS 样式 |
| `bboxOverlayStyle(bbox)` | 计算 bbox 叠加层 CSS 样式 |
| `cleanupZoneStyle(heightMm, pageInfo)` | 计算清理区域样式 |

**问题：** 无明显问题。

---

### 1.16 `src/modules/pdf-tools/composables/splitFileName.js`

**行数：** 89

**功能概述：** 拆分文件名格式化工具。支持日期 token（`[YYYYMMDD]`、`[YYYY-MM-DD]`）、序号 token（`[序号]`、`[中文序号]`、`[#]`、`[##]`）。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `todayCompact(date)` | 获取 YYYYMMDD 格式日期 |
| `formatSplitFileName({base, index, prefix, suffix, ...})` | 格式化拆分文件名 |
| `expandSplitNameTokens(value, index, dateValue)` | 展开名称中的 token |
| `formatDateToken(pattern, value)` | 按模式格式化日期 |

**问题：** 无明显问题。

---

### 1.17 `src/modules/pdf-tools/composables/usePdfSplitRanges.js`

**行数：** 47

**功能概述：** 拆分范围校验。检查页段名称、起止页有效性、重叠、间隙。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `splitRangeWarnings(ranges, totalPages)` | 返回所有校验警告 |

**问题：** 无明显问题。逻辑简洁清晰。

---

### 1.18 `src/modules/pdf-tools/index.js`

**行数：** 32

**功能概述：** pdf-tools 模块注册文件。定义模块 ID、路由、菜单项、首页卡片。

**导出：** 默认导出模块配置对象。

**问题：** 无。

---

### 1.19 `src/modules/evidence-pdf/views/EvidencePdfView.vue`

**行数：** 235

**功能概述：** 证据处理主视图。提供三个 Tab：分项证据处理、合并证据处理、证据扫描。复用 `EvidencePdfWorkbench` 组件。

**关键功能：**
- `scanEvidence()`：扫描证据文件夹
- `buildEvidence()`：生成合并 PDF
- `reorderGroupFiles()`：排序组内文件

**依赖：**
- `EvidencePdfWorkbench`（核心工作台）
- `ToolWorkspaceShell`、`FileQueuePanel`

**问题：**
- **L93-101 `scanEvidence`：** 扫描失败时只显示错误消息，不清空 `evidenceGroups`。如果之前有成功扫描的结果，旧数据会保留。

---

### 1.20 `src/modules/evidence-pdf/index.js`

**行数：** 32

**功能概述：** evidence-pdf 模块注册文件。定义模块 ID、路由 `/evidence`、菜单项。

**问题：** 无。

---

## 2. Rust 后端文件逐文件分析

### 2.1 `src-tauri/src/commands/pdf.rs`

**行数：** 304

**功能概述：** Tauri 命令层，暴露所有 PDF 功能给前端。作为 JS↔Rust 桥接层，反序列化参数并委托给 `pdf` 模块。

**导出函数（25 个 Tauri 命令）：**
| 命令 | 作用 |
|---|---|
| `check_qpdf` | 检查 qpdf 安装状态 |
| `inspect_pdf` | 检查加密和页数 |
| `unlock_pdf` | 解锁加密 PDF |
| `merge_pdfs` | 合并 PDF |
| `split_pdf` | 拆分 PDF |
| `extract_pdf_pages` | 提取页面 |
| `compress_pdf` | 压缩 PDF |
| `split_merged_evidence_pdf` | 拆分合并证据 PDF |
| `scan_evidence_folder` | 扫描证据文件夹 |
| `build_evidence_group_pdfs` | 构建分组 PDF |
| `merge_evidence_pdfs` | 合并证据 PDF |
| `overlay_pdf_text` | 单文件页眉页脚叠加 |
| `batch_overlay_pdf_text` | 批量页眉页脚叠加 |
| `apply_evidence_pdf_rules` | 执行完整证据处理流水线 |
| `preview_pdf_header_footer` | 预览页眉页脚 |
| `detect_pdf_header_footer` | 检测页眉页脚 |
| `inspect_merged_evidence_pdf` | 合并 PDF 页段识别 |
| `delete_pdf_annotations` | 删除批注 |
| `delete_pdf_header_footer_artifacts` | 删除页眉页脚标准结构 |
| `render_pdf_preview` | 渲染页面预览 |
| `get_pdf_page_count` | 获取页数 |
| `detect_anti_copy` | 检测防复制 |
| `apply_anti_copy` | 添加防复制 |
| `remove_anti_copy` | 移除防复制 |
| `has_pdf_bookmarks` / `remove_pdf_bookmarks` | 书签管理 |

**依赖：** `crate::pdf::*`、`crate::external::*`、`serde`、`tauri`

**问题：**
- **L162-254：** 反序列化→重新序列化→再反序列化模式。args 先 `serde_json::from_value` 为类型化结构体，再 `serde_json::to_value` 转回 Value，后端函数再反序列化。浪费 CPU。
- **L269-273 `apply_anti_copy`：** 未知 method 字符串静默回退到 `CmapScramble`，应报错。

---

### 2.2 `src-tauri/src/pdf/mod.rs`

**行数：** 15

**功能概述：** 模块声明文件。声明 14 个子模块。

**问题：**
- 所有子模块都声明为 `pub mod`，部分内部模块（如 `normalize`、`page_info`、`content_text`）应为 `pub(crate)`。

---

### 2.3 `src-tauri/src/pdf/detection.rs`

**行数：** 2646

**功能概述：** 最大的文件。通过解析 `pdftotext -bbox` XML 输出和 PDF 内容流来检测页眉页脚。支持合并 PDF 的页段自动识别（基于页眉文本变化和页码序列不连续性）。支持中文页码、罗马数字、带圈数字、分数格式。

**关键导出：**
| 函数/结构体 | 作用 |
|---|---|
| `detect(args)` | 主检测入口 |
| `suggest_split_ranges(args)` | 合并 PDF 页段建议 |
| `DetectionResult` | 检测结果（每页检测、候选列表、artifact 摘要） |
| `SplitSuggestionResult` | 页段建议结果 |
| `HeaderFooterCandidate` | 页眉页脚候选 |

**依赖：** `lopdf`、`regex`、`poppler`（pdftotext）、`artifacts`、`qpdf`

**问题：**
- **L478-523 `parse_pdftotext_bbox`：** 用正则解析 XML，属性顺序必须固定。如果 `pdftotext` 输出格式变化会静默失败。
- **L615-649 `group_words_into_lines`：** y 轴固定 3.0pt 桶大小，无法配置。
- **L939-943：** 只处理 UTF-16 BE BOM，UTF-16 LE 会回退到 `String::from_utf8` 产生乱码。
- **L1801-1803 `parse_f32`：** 解析失败返回 `0.0`，静默吞掉错误。

---

### 2.4 `src-tauri/src/pdf/evidence_session.rs`

**行数：** 414

**功能概述：** 证据 PDF 处理流水线。编排批注删除、页眉页脚叠加、可选合并、书签应用。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `apply_rules(args)` | 执行完整证据处理流水线 |

**依赖：** `annotations`、`header_footer`、`qpdf`

**问题：**
- **L295-322 `collect_merge_bookmarks`：** `items.iter().zip(results.iter())` 假设 items 和 results 长度一致且顺序对应。如果处理失败导致 results 缺少元素，zip 会静默截断，书签页码偏移可能错位。

---

### 2.5 `src-tauri/src/pdf/evidence.rs`

**行数：** 1081

**功能概述：** 证据文件夹扫描、分组 PDF 构建（含 Word→PDF 转换）、最终合并。支持 macOS（AppleScript）、Windows（PowerShell COM）、Linux（LibreOffice）三种 Word 转换路径。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `scan_folder(root)` | 扫描证据文件夹 |
| `build_group_pdfs(args, state)` | 构建分组 PDF |
| `merge_all(args)` | 合并所有分组 PDF |

**问题：**
- **L19-83 `run_process_with_interactive_timeout`：** 使用 `try_wait` + `sleep(1s)` 忙等循环，UI 取消响应延迟最多 1 秒。
- **L727-747 `merge_pdfs_with_qpdf`：** stderr 被丢弃到 `/dev/null`，合并失败时无诊断信息。
- **L905-998 `build_overlay_ops`：** `_width_pt` 参数未使用，覆盖层始终从 x=36pt 开始。
- **L1036-1056 `safe_file_stem`：** 不支持 CJK Extension A/B 字符。
- **L251 `fnv1a_hash`：** 与 `header_footer.rs` 重复定义。

---

### 2.6 `src-tauri/src/pdf/header_footer.rs`

**行数：** 2395

**功能概述：** 核心页眉页脚叠加引擎。构建嵌入 CJK 字体的覆盖 PDF（通过 `allsorts` 子集化）、处理 artifact 和纯文本清理、书签管理、预览集成。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `overlay_text(args)` | 单文件页眉页脚叠加 |
| `batch_overlay(args)` | 批量叠加 |
| `preview_overlay(args)` | 预览渲染 |
| `apply_bookmarks(output, bookmarks, remove_existing)` | 写入书签 |
| `has_pdf_bookmarks(path)` / `remove_pdf_bookmarks(path)` | 书签检查/删除 |

**依赖：** `allsorts`（字体子集化）、`lopdf`、`printpdf`、`annotations`、`artifacts`、`content_text`、`normalize`、`page_info`、`preview`、`qpdf`

**问题：**
- **L237-244：** 生产代码中使用 `eprintln!` 调试输出。
- **L324-372 `apply_bookmark`：** 先写临时文件再复制回原文件，非原子操作。崩溃时可能丢失原文件。
- **L726-752 `unique_output_path`：** 10,000 个后缀用尽后静默返回已存在的路径，可能覆盖。
- **L1544-1559 `fnv1a_hash`：** 与 `evidence.rs` 重复。
- **L1570-1575 `requires_embedded_font`：** 对 Latin-1 字符（如重音字母）也触发嵌入字体，过于保守。
- **L1620-1701 `font_paths_for_family`：** 字体路径硬编码，启动时不验证。
- **L1738-1750 `format_page_number`：** "dingbat" 样式的 code 计算正确但代码难以理解。

---

### 2.7 `src-tauri/src/pdf/preview.rs`

**行数：** 152

**功能概述：** 使用 `pdftoppm`（poppler）渲染 PDF 页面为 PNG 图片，返回 base64 data URL。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `render_preview(args)` | 渲染预览页面 |
| `render_pdf_page_to_png(input, page, dpi)` | 底层渲染函数 |

**问题：**
- **L51-55：** PNG 文件被读取两次（`image::open` 获取尺寸 + `fs::read` 获取字节），可优化为一次读取。
- **L54：** 临时文件在结果检查前被删除，调试困难。

---

### 2.8 `src-tauri/src/pdf/artifacts.rs`

**行数：** 1430

**功能概述：** 处理 PDF 标准页眉页脚 artifact（标记内容 `BDC`/`EMC` + `Artifact` 标签）。支持检查、删除、就地编辑。支持嵌套 Form XObject。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `delete_header_footer_artifacts(args)` | 删除 artifact |
| `edit_header_footer_artifacts_to_temp(input, plan)` | 编辑 artifact |
| `inspect_meaningful_header_footer_artifacts(input, max_pages)` | 检查 artifact |
| `decode_pdf_string(object)` | 解码 PDF 字符串 |
| `page_xobjects(doc, page_id)` | 获取页面 XObjects |

**问题：**
- **L284-348 `inspect_referenced_form_artifacts`：** 递归深度限制 8，好的防御性编程。
- **L350-368 `HeaderFooterArtifactEditResult`：** `changed_count()` 可能重复计数同时编辑和删除的页面。
- **L694-708 `replacement_object_like`：** 非 ASCII 替换文本在 Latin 编码 artifact 中会静默失败。
- **L982-1012：** `temp_named_path` 等工具函数与 `header_footer.rs`、`annotations.rs` 重复。

---

### 2.9 `src-tauri/src/pdf/annotations.rs`

**行数：** 239

**功能概述：** 删除 PDF 批注（高亮、评论、墨迹等）。按 `/Subtype` 过滤，保留 Link/Popup/Widget。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `delete_annotations(args)` | 删除批注（命令入口） |
| `delete_annotations_file(input, output, kinds)` | 底层删除 |
| `delete_annotations_to_temp(input_path, kinds)` | 删除到临时文件 |

**问题：**
- **L174-213：** `temp_named_path` 等工具函数第三次重复。
- **L52-95：** 整个 PDF 加载到内存，大文件可能占用大量内存。

---

### 2.10 `src-tauri/src/pdf/split.rs`

**行数：** 313

**功能概述：** 拆分合并 PDF 为多个小 PDF。按页段调用 qpdf 提取，可选删除页眉页脚。

**关键导出：**
| 函数/结构体 | 作用 |
|---|---|
| `split_merged(args)` | 主拆分入口 |
| `SplitMergedArgs` | 拆分参数 |
| `SplitMergedResult` | 拆分结果 |

**问题：**
- **L160-163：** `temp_named_path` 第四次重复。PID+毫秒时间戳碰撞风险。
- **L204-218 `safe_file_stem`：** 文件名安全字符过滤不完整（如 `-` 开头、null 字节）。

---

### 2.11 `src-tauri/src/pdf/anti_ocr.rs`

**行数：** 453

**功能概述：** PDF 防复制保护。通过修改字体 ToUnicode CMap 表实现。支持三种方法：CMap 篡改、CMap 移除、文本叠加（实际也是 CMap 篡改）。将原始 CMap 备份到 PDF Info 字典中以便恢复。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `detect_anti_copy(input)` | 检测防复制保护 |
| `apply_anti_copy(input, output, method)` | 添加防复制 |
| `remove_anti_copy(input, output)` | 移除防复制 |

**问题：**
- **L125-141：** `TextOverlay` 方法实际执行的是 CMap 篡改，与 `CmapScramble` 行为相同。命名误导。
- **L166：** 使用 `format!("{:?}", font_id)` 作为备份 key，依赖 `ObjectId` 的 Debug 格式，不稳定。
- **L446-452 `scramble_hex`：** 篡改映射空间仅 4096 个值（0xE000-0xF000），可暴力逆向。
- **L224：** `serde_json::to_string(backup).unwrap_or_default()` 静默吞掉序列化错误，备份可能丢失。

---

### 2.12 `src-tauri/src/pdf/overlay.rs`

**行数：** 9

**功能概述：** 兼容性 facade 模块。重新导出 `header_footer` 和 `preview` 的符号。

**问题：**
- 技术债务，迁移完成后应删除。

---

### 2.13 `src-tauri/src/pdf/normalize.rs`

**行数：** 294

**功能概述：** 将 PDF 页面规范化为 A4 尺寸。计算仿射变换矩阵，缩放/居中/旋转页面内容。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `normalize_pdf_to_a4(input, dpi, orientation)` | A4 规范化入口 |

**问题：**
- **L9：** `_dpi` 参数接受但从未使用（前缀 `_`），误导性 API。
- **L46-53：** `"preserve"` 和 `_` 分支有相同条件，其中一个为死代码。
- **L200-207：** `temp_named_path` 再次重复。
- **L27-36：** qpdf 优化失败时静默忽略错误。

---

### 2.14 `src-tauri/src/pdf/content_text.rs`

**行数：** 999

**功能概述：** 通过分析 PDF 内容流删除纯文本页眉页脚。支持 `{page}`/`{total}` 占位符匹配和 bbox 匹配。处理嵌套 Form XObject。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `delete_plain_header_footer_to_temp(input_path, plan)` | 删除纯文本页眉页脚 |

**问题：**
- **L365：** `update_text_state_after_show` 是空函数（no-op），死代码。
- **L403-411：** 只处理 UTF-16 BE BOM，PDFDocEncoding 回退可能产生乱码。
- **L467-488：** 每次调用都编译正则表达式，应预编译。
- **L556-563：** `temp_named_path` 再次重复。

---

### 2.15 `src-tauri/src/pdf/page_info.rs`

**行数：** 226

**功能概述：** 通过 `qpdf --json` 提取 PDF 页面尺寸信息。处理页面旋转（90/270 度交换宽高）。

**关键导出：**
| 符号 | 作用 |
|---|---|
| `A4_WIDTH_PT` / `A4_HEIGHT_PT` | A4 尺寸常量 |
| `PageSize` | 页面尺寸结构体 |
| `get_page_infos(input)` | 获取所有页面尺寸 |

**问题：**
- **L70-76：** 同时检查 camelCase 和 PascalCase 的 `cropBox`/`mediaBox`，qpdf JSON 用 camelCase，大写变体可能是死代码。

---

### 2.16 `src-tauri/src/pdf/qpdf.rs`

**行数：** 391

**功能概述：** qpdf CLI 工具的高层封装。提供检查、解锁、合并、压缩、提取、拆分等操作。

**关键导出：**
| 函数 | 作用 |
|---|---|
| `inspect(path)` | 检查加密和页数 |
| `unlock(input)` | 解锁 PDF |
| `merge(inputs, output)` | 合并 PDF |
| `compress(input, output_dir)` | 压缩 PDF |
| `extract_pages(input, pages, output_dir)` | 提取页面 |
| `page_count(input)` | 获取页数 |

**问题：**
- **L21：** 退出码 3（警告）视为成功，警告信息被静默忽略。
- **L87：** `--password=` 传空密码，不支持有密码的 PDF 解锁。
- **L110：** 合并时总是添加优化参数，无法做"原始合并"。
- **L230-241 `unique_available_path`：** 10,000 后缀用尽后返回已存在路径。

---

### 2.17 `src-tauri/src/external/qpdf.rs`

**行数：** 109

**功能概述：** qpdf 外部工具实现。处理二进制发现（管理安装、Homebrew、PATH）、版本探测、自动安装。

**问题：**
- **L95-104：** `binary_name` 的 `_ => "qpdf.exe"` 分支为死代码。
- **L79：** 已知路径仅限 macOS，Linux 无回退路径。

---

### 2.18 `src-tauri/src/external/poppler.rs`

**行数：** 129

**功能概述：** Poppler 工具套件实现（`pdftoppm` + `pdftotext`）。处理二进制发现和安装。

**问题：**
- **L14-21：** 已知路径仅限 macOS。
- **L114-124：** Windows 上未知二进制名不会自动加 `.exe` 后缀。

---

## 3. PDF 工具模块完整数据流

### 3.1 普通 PDF 工具（解锁/合并/提取/压缩/拆分/防复制）

```
用户操作
  ↓
PdfToolsView.vue（Tab 选择对应功能）
  ↓
tauriBridge.js（tauriCallSafe）
  ↓
commands/pdf.rs（Tauri 命令层）
  ↓
pdf/qpdf.rs（qpdf CLI 封装）     pdf/anti_ocr.rs（CMap 操作）
  ↓                                ↓
qpdf 二进制                      lopdf 库直接操作
  ↓
输出文件
```

### 3.2 证据 PDF 处理（evidence-pdf）

```
用户导入 PDF 文件
  ↓
EvidencePdfWorkbench.vue
  ├─ loadEvidenceFiles(paths) → createEvidenceFile(path) → overlayFiles[]
  ├─ refreshOverlayPageCounts() → tauriCallSafe('get_pdf_page_count')
  └─ detectAllHeaderFooter()
       ↓
  useEvidencePdfDetection.js
       ↓ tauriCallSafe('detect_pdf_header_footer')
  commands/pdf.rs → detection.rs
       ├─ pdftotext -bbox → XML 解析 → 文本行分组 → 候选评分
       ├─ artifacts.rs → 标准结构 artifact 检查
       └─ 返回 DetectionResult
       ↓
  applyDetectionResultToFile() → file.existingElements[]

用户配置规则（页眉/页脚/页码/A4/批注/书签）
  ↓
currentRules computed（汇总所有配置）
  ↓
用户点击"执行"
  ↓
applyHeaderFooter()
  ↓ buildEvidencePdfRulePayload(overlayRows, rules, outputDir)
  ↓
tauriCallSafe('apply_evidence_pdf_rules', { args: payload })
  ↓
commands/pdf.rs → evidence_session.rs::apply_rules()
  ├─ annotations.rs::delete_annotations_to_temp() — 删除批注
  ├─ normalize.rs::normalize_pdf_to_a4() — A4 规范化
  ├─ content_text.rs::delete_plain_header_footer_to_temp() — 纯文本清理
  ├─ artifacts.rs::edit_header_footer_artifacts_to_temp() — artifact 编辑
  ├─ header_footer.rs::batch_overlay() — 批量叠加
  │    ├─ build_overlay_pdf() — 构建覆盖 PDF（含字体子集化）
  │    ├─ qpdf overlay 合并
  │    └─ apply_bookmarks() — 写入书签
  ├─ qpdf::merge() — 合并输出（如需要）
  └─ 返回结果
```

### 3.3 合并 PDF 拆分流程

```
用户导入合并 PDF
  ↓
importMergedPdfAsEvidence()
  ↓ tauriCallSafe('inspect_merged_evidence_pdf')
  ↓
commands/pdf.rs → detection.rs::suggest_split_ranges()
  ├─ 检测前 300 页的页眉变化
  ├─ 识别页码序列不连续点
  └─ 返回 SplitSuggestionResult（页段列表）
  ↓
mergedImportPlan（用户确认/调整页段）
  ↓
executeMergedImportPlan()
  ↓ tauriCallSafe('split_merged_evidence_pdf')
  ↓
commands/pdf.rs → split.rs::split_merged()
  ├─ 对每个页段调用 qpdf 提取
  ├─ 可选：header_footer.rs::overlay_text() 清理页眉页脚
  └─ 返回拆分结果
  ↓
overlayFiles[] 更新为拆分后的文件列表
  ↓
detectAllHeaderFooter() — 重新检测
```

---

## 4. evidence-pdf 会话管理机制

### 4.1 会话状态模型

evidence-pdf 不使用后端会话（无 session ID、无持久化），所有状态在前端管理：

```
EvidencePdfWorkbench.vue（主状态容器）
  │
  ├─ overlayFiles: ref([]) — 证据文件列表
  │    └─ 每个文件 50+ 字段（路径、页数、页眉页脚文本、检测结果、编辑状态...）
  │
  ├─ mergedImportPlan: ref(null) — 合并导入计划
  │    └─ { inputPath, outputDir, totalPages, items[], warnings[] }
  │
  ├─ currentRules: computed — 当前处理规则（50+ 字段）
  │    └─ { normalizeA4, headerMode, footerEnabled, pageNumberSequence, ... }
  │
  ├─ 分组管理
  │    ├─ headerGroups / footerTextGroups / pageNumberGroups — 每文件多组配置
  │    ├─ globalHeaderGroup / globalFooterTextGroup / globalPageNumberGroup — 全局共享组
  │    └─ globalApplyEnabled — 全局/每文件切换
  │
  ├─ 预览状态
  │    ├─ previewPage / previewReloadKey
  │    ├─ previewData（页面尺寸信息）
  │    └─ truePreview（真实渲染结果）
  │
  └─ 编辑状态
       ├─ editingContentRowId / editingContentRowValue
       └─ editUndoStack / editRedoStack
```

### 4.2 全局/每文件配置切换

`globalApplyEnabled` 控制所有文件共享一组配置还是每个文件独立配置：
- **全局模式（默认）：** `globalHeaderGroup`、`globalFooterTextGroup`、`globalPageNumberGroup` 被所有文件共享。
- **每文件模式：** 每个 `overlayFiles[i]` 有自己的 `headerGroups[]`、`footerTextGroups[]`、`pageNumberGroups[]`。

通过 `computed({ get, set })` 实现透明切换，`HeaderFooterRuleFields` 组件无需感知模式差异。

### 4.3 参数跟随机制

当切换到新文件时（`watch(selectedOverlayFile)`），如果新文件的组参数仍为默认值，会自动从第一个文件（`overlayFiles[0]`）同步样式参数（align、fontSize、fontFamily 等）。这确保了用户在第一个文件设置的样式能自动应用到后续文件。

### 4.4 Undo/Redo 机制

两个层级的 undo/redo：
1. **文本编辑级：** `editUndoStack`/`editRedoStack`（最多 30 步），记录 group text 和 element editedText 的快照。
2. **样式参数级：** `HeaderFooterRuleFields` 内部使用 `useHistory` composable，对 header/footerText/pageNumber 三组样式参数分别维护 undo/redo 栈。

---

## 5. 所有发现的 Bug 汇总

### 严重 Bug

| # | 文件 | 行号 | 描述 |
|---|---|---|---|
| 1 | `evidence_session.rs` | 295-322 | `items.iter().zip(results.iter())` 假设 items 和 results 长度一致。处理失败导致 results 缺少元素时，zip 静默截断，书签页码偏移错位。 |
| 2 | `header_footer.rs` | 324-372 | 书签写入先写临时文件再复制回原文件，非原子操作。进程崩溃时可能丢失原 PDF。 |
| 3 | `header_footer.rs` | 726-752 | `unique_output_path` 10,000 后缀用尽后返回已存在路径，可能导致数据覆盖。 |
| 4 | `qpdf.rs` | 87 | `--password=` 传空密码，不支持有密码的 PDF 解锁，且无错误提示说明。 |

### 中等问题

| # | 文件 | 行号 | 描述 |
|---|---|---|---|
| 5 | `detection.rs` | 478-523 | 用正则解析 XML（`pdftotext -bbox`），属性顺序必须固定。格式变化会静默失败。 |
| 6 | `content_text.rs` | 467-488 | 每次调用都编译正则表达式，大 PDF 性能差。应预编译。 |
| 7 | `anti_ocr.rs` | 125-141 | `TextOverlay` 方法实际执行 CMap 篡改，与 `CmapScramble` 相同。命名误导。 |
| 8 | `anti_ocr.rs` | 166 | 备份 key 使用 `format!("{:?}", font_id)`，依赖 `ObjectId` Debug 格式，lopdf 版本更新可能导致备份不可恢复。 |
| 9 | `anti_ocr.rs` | 446-452 | 篡改映射空间仅 4096 个值，可暴力逆向。 |
| 10 | `content_text.rs` | 403-411 | 只处理 UTF-16 BE BOM，UTF-16 LE 和 PDFDocEncoding 会产生乱码。 |
| 11 | `detection.rs` | 939-943 | 同上，只处理 UTF-16 BE。 |
| 12 | `evidence.rs` | 727-747 | qpdf 合并 stderr 丢弃到 `/dev/null`，失败时无诊断信息。 |
| 13 | `normalize.rs` | 9 | `_dpi` 参数接受但从未使用，误导性 API。 |
| 14 | `PdfToolsView.vue` | 269-273 | `apply_anti_copy` 未知 method 静默回退到 `CmapScramble`。 |
| 15 | `EvidencePdfWorkbench.vue` | 2576 | `reorderOverlayFiles` 中 `findIndex` 返回 -1 时 `Math.max(0, -1) = 0`，可能意外选中第一个文件。 |

### 代码异味 / 技术债务

| # | 文件 | 描述 |
|---|---|---|
| 16 | `commands/pdf.rs` L162-254 | 反序列化→重新序列化→再反序列化模式，浪费 CPU。 |
| 17 | `header_footer.rs`、`artifacts.rs`、`annotations.rs`、`split.rs`、`normalize.rs`、`content_text.rs`、`preview.rs` | `temp_named_path` 函数在 7 个文件中重复。应提取到共享工具模块。 |
| 18 | `header_footer.rs` L1544 + `evidence.rs` L251 | `fnv1a_hash` 重复定义。 |
| 19 | `header_footer.rs` L237-244 | 生产代码使用 `eprintln!` 调试输出。 |
| 20 | `overlay.rs` | 9 行的 facade 模块，技术债务。 |
| 21 | `content_text.rs` L365 | `update_text_state_after_show` 空函数，死代码。 |
| 22 | `useEvidencePdfSession.js` L125-188 | `createEvidenceFile` 返回 50+ 字段的扁平对象。 |
| 23 | `useEvidencePdfSession.js` L469-642 | `buildHeaderFooterItems` 170+ 行，职责过多。 |
| 24 | `EvidencePdfWorkbench.vue` | 组件 2600+ 行，100+ 个 ref/computed，复杂度过高。 |
| 25 | `ExistingPdfElementsDialog.vue` L112-119 | `window.addEventListener` 在 keep-alive 场景下可能泄漏。 |
| 26 | `ExistingPdfElementsDialog.vue` L297 | 直接修改 prop 数据（通过对象引用），违反单向数据流。 |
| 27 | `HeaderFooterRuleFields.vue` L347-353 | 自定义 debounce 无 cancel 机制。 |
| 28 | 所有 `temp_named_path` | PID+毫秒时间戳碰撞风险，并发场景下不安全。 |
| 29 | `mod.rs` | 所有子模块声明为 `pub mod`，部分应为 `pub(crate)`。 |
| 30 | `detection.rs` L1801-1803 | `parse_f32` 失败返回 `0.0`，静默吞掉错误。 |
| 31 | `EvidencePdfView.vue` L93-101 | 扫描失败不清空旧结果。 |
| 32 | `EvidencePdfWorkbench.vue` L933 | `quickCleanupPipeline` 使用普通变量控制异步流程。 |

---

*报告生成完毕。共审阅 38 个文件，发现 4 个严重 Bug、11 个中等问题、17 个代码异味/技术债务。*
