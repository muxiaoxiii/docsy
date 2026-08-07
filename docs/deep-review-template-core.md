# Docsy Template 模块与 Core 层深度审阅

> 审阅日期：2026-08-07
> 分支：codex/template-quickxml-0.8
> 审阅范围：41 个文件（前端 + Rust 后端）

---

## 目录

1. [逐文件分析](#1-逐文件分析)
2. [Template 模块完整数据流](#2-template-模块完整数据流)
3. [Core 层架构分析](#3-core-层架构分析)
4. [所有发现的 Bug 和代码问题](#4-所有发现的-bug-和代码问题)

---

## 1. 逐文件分析

### 1.1 Core 层

#### `src/core/tauriBridge.js` (153 行)

**功能概述**：Tauri IPC 调用的统一封装层，提供带/不带动画、安全/抛异常等多种调用方式。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `emitOperationEvent(type, command, operationId)` | 发射 CustomEvent 驱动 Doclet 动画 |
| `tauriCall(command, args)` | 标准调用，抛异常，触发动画 |
| `tauriCallQuiet(command, args)` | 静默调用，返回 `{ok, data/error}`，不触发动画 |
| `tauriCallSafe(command, args)` | 安全调用，返回 `{ok, data/error}`，触发动画 |
| `openPath(path)` | 打开文件/目录 |
| `openExternalUrl(url)` | 打开外部链接 |
| `getPdfPageCount(input)` | 获取 PDF 页数 |
| `userFacingError(error, fallback, maxLength)` | 错误信息用户友好化 |
| `showLoading, hideLoading` | 从 `loading.js` 重导出 |

**依赖**：`@tauri-apps/api/core`, `../services/appLogger.js`, `./loading.js`

**问题**：
- **L54**：`void logError(...)` 中 `void` 关键字用于忽略 Promise 是有意为之（fire-and-forget），但 `tauriCallQuiet` (L62-70) 没有记录错误日志，静默失败可能难以排查。

---

#### `src/core/moduleRegistry.js` (82 行)

**功能概述**：模块自动发现与注册。通过 `import.meta.glob` 扫描 `modules/*/index.js`，排序后提供路由、菜单、首页卡片。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `moduleRegistry` (const) | 按 order 排序的模块列表 |
| `getRoutes()` | 汇总所有模块路由 |
| `getMenuItems(settings)` | 根据 settings 生成菜单项（可见性+排序） |
| `getHomeCards(settings)` | 生成首页卡片 |
| `getMenuModules()` | 返回模块基础信息 |
| `defaultMenuOrder()` | 默认菜单顺序 |

**依赖**：`@element-plus/icons-vue`

**问题**：无显著问题。设计清晰。

---

#### `src/core/filePath.js` (27 行)

**功能概述**：跨平台文件路径工具函数。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `fileName(path)` | 提取文件名 |
| `parentDir(path)` | 提取父目录 |
| `stripExtension(name, pattern)` | 去除扩展名 |
| `stripPdf(name)` | 去除 .pdf 扩展名 |
| `getExtension(path)` | 获取小写扩展名 |

**依赖**：无

**问题**：无。纯函数，简洁正确。

---

#### `src/core/loading.js` (38 行)

**功能概述**：手动加载动画触发器，与 tauriBridge 的自动动画共用同一 CustomEvent 管道。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `showLoading(label)` | 发射 start 事件，返回 id |
| `hideLoading(id)` | 发射 finish 事件 |

**依赖**：无

**问题**：无。

---

#### `src/core/numberFormat.js` (31 行)

**功能概述**：数字转中文（0-9999）。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `toChineseNumber(value)` | 数字转中文，如 10→十，2026→二零二六 |

**依赖**：无

**问题**：无。逻辑正确，"一十"→"十" 的简化处理得当。

---

#### `src/core/pdfUtils.js` (85 行)

**功能概述**：PDF 页面范围操作工具集。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `pageCount(range)` | 计算页数 |
| `navigatePage(current, delta, max)` | 翻页 |
| `setRangeStart/End(range, page)` | 设置范围起止 |
| `buildRangeAfter(items, index, max, options)` | 构建新范围 |
| `insertRangeAfter(items, index, max, options)` | 插入新范围 |
| `removeRangeAt(items, index, currentIndex)` | 删除范围 |
| `parsePageSelection(input, maxPage)` | 解析页码选择字符串 |

**依赖**：无

**问题**：
- **L12-15** `setRangeStart` 和 **L19-23** `setRangeEnd` 直接 mutate 传入的 `range` 对象，调用方需注意副作用。在 Vue reactive 场景中这是有意为之，但纯函数接口通常应返回新对象。

---

#### `src/core/unitConversion.js` (7 行)

**功能概述**：mm ↔ pt 单位转换。

**导出函数**：`mmToPt(mm)`, `ptToMm(pt)`

**依赖**：无。无问题。

---

#### `src/core/composables/useHistory.js` (74 行)

**功能概述**：通用 undo/redo 历史栈 composable。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `useHistory({snapshot, restore, maxSteps, onChange})` | 返回 `{push, undo, redo, canUndo, canRedo, clear, length}` |
| `forwardRef(getter, setter)` | 便捷构造 snapshot/restore 对 |

**依赖**：`vue`

**问题**：
- **L20-28** `push()` 截断 redo 分支时使用 `stack.value.slice(0, index.value + 1)`，每次 push 都创建新数组。在高频操作场景下可能有性能影响，但 maxSteps=20 限制了数组大小，实际影响不大。

---

#### `src/core/composables/usePointerReorder.js` (72 行)

**功能概述**：基于 Pointer Events 的拖拽排序。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `reorderTargetIndex(from, target, placement, itemCount)` | 计算目标索引 |
| `usePointerReorder({itemCount, onReorder, itemAttribute})` | 返回拖拽状态和控制函数 |

**依赖**：`vue`

**问题**：无显著问题。pointer capture 处理得当，有 try-catch 保护 releasePointerCapture。

---

#### `src/core/composables/useWindowFileDrop.js` (31 行)

**功能概述**：窗口级文件拖放处理。

**导出函数**：`useWindowFileDrop({onEnter, onLeave, onDrop, onError})`

**依赖**：`vue`, `@tauri-apps/api/webview`

**问题**：
- **L12**：`onEnter` 在 `type === 'over'` 时也被调用。这是有意的（"over" 事件持续触发），但语义上 `onEnter` 命名不够精确，更适合叫 `onDragActivity` 或类似名称。

---

### 1.2 Template 模块 — Composables

#### `src/modules/template/composables/fieldRowUtils.js` (1199 行)

**功能概述**：字段行操作的纯工具函数集合。这是 template 模块最核心的工具库，包含类型系统、预览构建、引用解析、日期格式化等。

**关键导出**（约 60+ 个函数）：
- 类型系统：`FIELD_TYPE_GROUPS`, `typeGroupOf`, `typeGroupSubOptions`, `typeActualOf`, `typeLabel`
- 行操作：`rowUsage`, `isMarkerType`, `isConnectorRow`, `isGeneratedFieldName`
- 标记引用：`markRefsForTextRange`, `refsForRowTextRange`, `markSegmentsFromRefs`
- 预览：`previewRangesByRun`, `previewRunSegment`, `previewSourceLabel`, `previewReplacementText`
- 引用：`referenceSourceKey`, `parseReferenceSourceKey`, `normalizedReferenceSource`, `referenceSourceLabel`
- 日期：`parseDateParts`, `formatDateValue`
- 列表：`splitPartyLabelText`, `partyItemsToValues`, `parsePartyItem`
- 历史：`groupHistoryRuns`, `historyRunSummary`

**依赖**：`../rules/publicRules.js`

**问题**：
- **文件过大**（1199 行）：承担了太多职责。建议拆分为 `typeUtils.js`, `previewUtils.js`, `referenceUtils.js`, `dateUtils.js` 等。
- **L176-204** `splitPartyLabelSegments` 与 `TemplateView.vue` 中的同名函数（L1299-1324）存在**重复实现**。TemplateView.vue 中的版本是运行时使用的，fieldRowUtils.js 中的版本是从中提取的。两个版本逻辑相同但维护两份代码是隐患。

---

#### `src/modules/template/composables/useFieldNormalization.js` (810 行)

**功能概述**：字段行标准化、构建、验证的完整 pipeline。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `normalizeFieldRows(rows, documentRuns)` | 标准化 pipeline（拆分→合并→排序） |
| `expandDuplicateMarkRefs(rows, marks)` | 扩展同文本标记引用 |
| `inferFieldFromText(text, context, checkboxLike, index)` | 从文本推断字段类型 |
| `markToRow(mark, index)` | 标记转字段行 |
| `autoMergeMarks(rawMarks)` | 自动合并相邻标记 |
| `buildFields(fieldRows)` | 构建 manifest 字段列表 |
| `validateFieldRowsBeforeSave(fieldRows, marks)` | 保存前验证 |

**依赖**：`fieldRowUtils.js`, `publicRules.js`

**问题**：
- **L39-52** `normalizeFieldRows` 的链式调用很深（8 层嵌套），可读性不佳。建议提取为 pipeline 变量。
- **L536-646** `buildFields` 函数约 110 行，职责较重但逻辑清晰。

---

#### `src/modules/template/composables/usePreviewSelection.js` (422 行)

**功能概述**：预览面板中的文本选择和标记管理。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `usePreviewSelection(documentRuns, documentText, fieldRows, previewSampleValues)` | 返回预览选择状态和操作函数 |
| `buildTemplatePreview(runs, fallbackText, rows, sampleValues)` | 构建模板预览数据 |
| `rememberSourcePreviewSelection()` | 记住当前选区 |
| `collectSourcePreviewSelection()` | 收集选区信息 |
| `nextReferenceFieldName()` | 生成下一个引用字段名 |

**依赖**：`vue`, `fieldRowUtils.js`, `useFieldNormalization.js`

**问题**：
- **L29-30** `lastPreviewAddKey` 和 `lastPreviewAddAt` 是模块级变量（非 ref），在 composable 多次调用时共享状态。这是防抖逻辑的一部分，但如果是 singleton composable 则没问题。实际上 `usePreviewSelection` 在 `TemplateView.vue` 中只调用一次，所以不是 bug。
- **L285-293** `triggerPreviewSelectionAdd` 与 `TemplateView.vue` 中的同名函数（L1115-1127）存在**逻辑重复**。TemplateView.vue 版本有额外的 try-catch 和错误提示，usePreviewSelection 版本返回 success/hasPayload 让调用方处理。两套逻辑需要保持同步。

---

#### `src/modules/template/composables/useTemplateSettings.js` (141 行)

**功能概述**：模板设置管理（分隔符、回收站、数据库、历史清理）。

**导出函数**：
| 签名 | 作用 |
|------|------|
| `useTemplateSettings(loadHistoryContext, loadTemplateHistoryRuns)` | 返回设置状态和操作函数 |

**依赖**：`vue`, `element-plus`, `tauriBridge.js`

**问题**：
- **L9**：`window.localStorage.getItem` 在 SSR 环境下会报错，但 Tauri 桌面应用不走 SSR，所以实际无问题。

---

#### `src/modules/template/composables/useBatchFill.js` (296 行)

**功能概述**：批量填写的导出/导入/渲染流程。

**导出函数**：`useBatchFill(...)` 返回批量处理状态和操作。

**依赖**：`vue`, `element-plus`, `@tauri-apps/plugin-dialog`, `filePath.js`, `tauriBridge.js`, `fieldRowUtils.js`

**问题**：
- **L57-68**：`importAndBatchRender` 中先校验再弹出输出目录选择，用户体验合理。但校验通过后如果用户取消目录选择（L89-91），`batchProcessing` 已被设为 true 又被设回 false，中间没有清理操作，没问题但流程略显冗余。

---

### 1.3 Template 模块 — 规则

#### `src/modules/template/rules/publicRules.js` (319 行)

**功能概述**：法律文书模板的公共推断规则——法院名、案号、日期、案由、诉讼阶段等。

**关键导出**：
- `PUBLIC_ROLE_PREFIX_RULES`：角色前缀规则（原告、被告等）
- `PUBLIC_SUFFIX_RULES`：后缀规则（律师、代理人等）
- `PUBLIC_CAUSE_ACTIONS`：案由列表
- `PUBLIC_LITIGATION_STAGES`：诉讼阶段
- `inferTemplateField({text, context, checkboxLike, index})`：核心推断函数
- 各种 `looksLike*` 判断函数

**依赖**：`causeActions2025.js`, `courtNames.js`

**问题**：
- **L186-219** `inferTemplateField` 的推断优先级合理：日期 > 法院 > 案号 > 案由 > 诉讼阶段 > 律所 > 后缀身份 > 当事人角色 > 上下文角色 > 默认文本。
- **L235-255** `inferPartyRole` 在上下文中查找最近的角色前缀，`valueIndex - roleIndex <= 120` 的 120 字符窗口是经验值，可能在超长段落中误判。

---

#### `src/modules/template/rules/courtNames.js` (554 行)

**功能概述**：全国法院名称数据（550+ 条）。

**导出**：`PUBLIC_COURT_NAMES` 数组。

**问题**：无。纯数据文件。

---

#### `src/modules/template/rules/causeActions2025.js` (983 行)

**功能概述**：2025 年民事案件案由规定数据（980+ 条）。

**导出**：`PUBLIC_CAUSE_ACTIONS_2025` 数组。

**问题**：无。纯数据文件。

---

### 1.4 Template 模块 — 组件

#### `src/modules/template/views/TemplateView.vue` (2901 行)

**功能概述**：模板模块的主视图容器，管理四个 Tab 页（制作、填写、历史、设置），是整个模块的状态中心。

**核心状态**：
- `sourceDocx`, `marks`, `documentText`, `documentRuns` — 源 Word 文档状态
- `fieldRows`, `selectedRows` — 字段行状态
- `templatePath`, `templateManifest` — 模板包状态
- `formValues`, `referenceSelections`, `structureOverrides`, `typeOverrides` — 填写表单状态
- `historyContext`, `historyRuns` — 历史数据
- `previewSampleValues`, `sourcePreviewSelection` — 预览状态

**依赖**：大量（4 个子组件 + 4 个 composable + core 工具 + rules）

**问题**：
- **文件过大**（2901 行）：作为状态中心不可避免地庞大，但许多函数可以提取到 composable 中。
- **L173** `import { invoke } from '@tauri-apps/api/core'`：直接导入 `invoke` 而非通过 `tauriBridge.js`。在 `runDiagnostic` (L1424) 中用于 `get_log_file_path`，绕过了统一的错误处理和动画触发。
- **L175** `import { writeTextFile } from '@tauri-apps/plugin-fs'`：同上，直接使用底层 API。
- **L266-271** `watch(buildTabRef, ...)` 使用 `{ immediate: true }` 监听组件 ref，这是将子组件暴露的 DOM ref 同步到 composable 的标准做法，但依赖子组件 `defineExpose` 的稳定性。
- **L2143-2150** `JSON.stringify(formValues)` 作为 watch source：每次 formValues 的任何变化都会触发完整的 JSON 序列化，在字段较多时可能有性能影响。
- **L2158-2171** 引用字段自动解析也用 `JSON.stringify(formValues)` 触发，与 fill preview 的 watcher 可能产生级联更新。
- **L2708-2713** `ensureExtension` 函数在 TemplateView.vue 中定义但与 `fieldRowUtils.js` 中的同名函数（L225-231）**完全重复**。

---

#### `src/modules/template/components/TemplateBuildTab.vue` (1464 行)

**功能概述**：制作模板 Tab 页。展示字段表格、预览面板、全文面板。

**Props**：25 个 props 从父组件传入。
**Emits**：30+ 个事件向父组件发射。

**依赖**：`fieldRowUtils.js`（大量导入）

**问题**：
- **Props 过多**（25 个）：这是 Vue 中"prop drilling"的典型表现。考虑使用 provide/inject 或将更多状态下沉到 composable。
- **L642-826** script 部分主要是一些 computed 和 wrapper 函数，将 `fieldRowUtils.js` 的纯函数适配为接受 `props.fieldRows` 的版本。这种适配层增加了间接性但保持了工具函数的可测试性。

---

#### `src/modules/template/components/TemplateRenderTab.vue` (1145 行)

**功能概述**：填写模板 Tab 页。展示模板库卡片、填写表单、预览面板。

**Props**：16 个。
**Emits**：22 个。

**依赖**：`fieldRowUtils.js`

**问题**：
- **L441-444** `fieldFormKey` 函数与 TemplateView.vue 中的同名函数**重复定义**。
- **L446-453** `effectiveFieldType` 函数与 TemplateView.vue 中的同名函数（L425-427）逻辑略有不同——RenderTab 版本额外处理了 `fillAllPositions` follower 的情况。这种**微妙差异**是维护隐患。

---

#### `src/modules/template/components/TemplateSettingsTab.vue` (345 行)

**功能概述**：设置 Tab 页。管理分隔符、回收站、模板数据库、导入导出。

**Props**：10 个。**Emits**：12 个。

**依赖**：`tauriBridge.js`, `fieldRowUtils.js`

**问题**：无显著问题。组件职责清晰。

---

#### `src/modules/template/components/TemplateHistoryTab.vue` (219 行)

**功能概述**：填写历史 Tab 页。按模板分组展示历史记录。

**Props**：3 个。**Emits**：6 个。

**依赖**：`filePath.js`, `fieldRowUtils.js`

**问题**：无显著问题。简洁清晰。

---

#### `src/modules/template/index.js` (31 行)

**功能概述**：模块注册入口。声明 id、路由、菜单项、首页卡片。

**问题**：无。

---

### 1.5 基础设施文件

#### `src/main.js` (17 行)

**功能概述**：Vue 应用入口。安装 Pinia、Router、ElementPlus（中文 locale）。

**问题**：无。

---

#### `src/App.vue` (376 行)

**功能概述**：根组件。左侧边栏导航 + 主内容区 + Doclet 工作动画面板。

**依赖**：`moduleRegistry.js`, `tauriBridge.js`, `@tauri-apps/api/event`

**问题**：
- **L196-211**：`listen('docsy-conversion-timeout', ...)` 在 `onMounted` 中注册，返回的 unlisten 函数存储在模块级变量。如果组件被多次挂载/卸载（热更新场景），可能产生重复监听。但 Tauri 桌面应用中 App.vue 只挂载一次，实际无问题。

---

#### `src/router/index.js` (18 行)

**功能概述**：路由配置，从 moduleRegistry 动态获取路由。

**问题**：无。

---

#### `src/stores/app.js` (31 行)

**功能概述**：Pinia store，管理全局设置。

**问题**：无。

---

#### `src/services/appLogger.js` (75 行)

**功能概述**：前端日志服务，通过 Tauri IPC 写入后端日志。

**导出函数**：`logDebug`, `logInfo`, `logWarn`, `logError`

**问题**：
- **L8** `looksSensitiveKey` 的正则 `/base64|docx|bytes|content|html|xml|text/i` 会将几乎所有包含 "text" 的 key 标记为敏感，可能导致有用的调试信息被截断。

---

#### `src/services/devTracker.js` (155 行)

**功能概述**：开发模式下的用户操作追踪器。记录点击、输入、选择等操作。

**导出函数**：`installDevTracker`, `getDevSessionLog`, `exportDevLog`, `clearDevLog`

**问题**：
- **L15** `_sessionId` 使用 `Math.random()`，在需要确定性的场景下不可靠，但仅用于开发追踪所以可以接受。

---

### 1.6 Rust 后端

#### `src-tauri/src/commands/template.rs` (333 行)

**功能概述**：模板相关的所有 Tauri command 定义。是前端与 Rust 后端的桥梁。

**关键 commands**（20+ 个）：
- 模板 CRUD：`inspect_docx_template`, `save_docx_template`, `save_docx_template_to_library`
- 模板库：`list_template_library`, `list_template_trash`, `move_template_to_trash`, `restore_template_from_trash`, `permanently_delete_template`
- 历史：`get_template_history_context`, `list_template_generation_runs`, `clear_template_history`, `seed_template_history`
- 批量：`export_template_fields_xlsx`, `validate_batch_import`, `batch_render_from_xlsx`, `save_batch_history_rows`
- 设置：`save_template_field_settings`
- 导入导出：`import_template_to_library`, `export_templates`

**问题**：
- **L9-34** `MANIFEST_CACHE` 使用 `Mutex<Option<...>>` 全局缓存。`cached_manifest` 函数在每次调用时获取 mutex 锁，但在高并发场景下（虽然 Tauri 单窗口不太可能）锁竞争可能成为瓶颈。当前实现通过 mtime 检查保证缓存有效性，设计合理。
- **L277-291** `save_batch_history_rows` 对每一行都调用 `inspect_template_package`，如果批量行数多且模板路径相同，会重复解析同一个包。应该缓存 manifest。

---

#### `src-tauri/src/docx_template/engine.rs` (815 行)

**功能概述**：模板引擎核心。包含 inspect、save、render 的顶层流程。

**关键函数**：
- `inspect_docx(path)` — 读取 docx 提取标记
- `save_docx(args)` — 保存模板包
- `render_docx(args, source)` — 渲染模板生成 docx
- `scan_package_to_runs_and_marks(pkg)` — 扫描包提取 runs 和 marks

**依赖**：`scan.rs`, `save.rs`, `render.rs`, `package.rs`

**问题**：
- **L275-289** `convert_doc_to_docx` 使用 `office_oxide` 库转换 .doc→.docx，临时文件通过 `TempPathGuard` 的 Drop trait 自动清理。设计良好。
- **L253-272** `ensure_template_package_safe` 检查不支持的敏感内容（批注、宏、签名等），安全策略合理。

---

#### `src-tauri/src/docx_template/scan.rs` (325 行)

**功能概述**：XML 扫描器。解析 Word XML 构建文本索引。

**关键函数**：
- `scan_document_index(part_name, tree)` — 扫描单个 XML part
- `scan_package_index_to_document_index(parts)` — 扫描所有 parts 构建统一索引

**问题**：
- **L72-80** 对 `w:sdt`, `w:sdtContent`, `w:hyperlink` 递归扫描，确保这些容器内的 run 也被正确索引。这与 `save.rs` 中的坐标匹配逻辑保持一致，是正确的设计。

---

#### `src-tauri/src/docx_template/save.rs` (850 行)

**功能概述**：模板保存。将字段标记包装为 `<w:sdt>` 内容控件。

**关键函数**：
- `build_template_docx(package_xml, fields, index)` — 构建模板 docx
- `validate_coordinate_targets(fields, index)` — 验证坐标目标
- `wrap_runs_by_coordinates(...)` — 按坐标包装 run

**问题**：
- **L397-407** `cursor.1`（run index）只在 `had_text` 为 true 时递增。这确保了空 `<w:t></w:t>` 元素不会偏移坐标，与 scan.rs 的行为保持一致。
- **L330-332** 对 `w:sdt`, `w:sdtContent`, `w:hyperlink` 的递归处理与 scan.rs 完全对称，保证坐标一致性。

---

#### `src-tauri/src/docx_template/render.rs` (1459 行)

**功能概述**：模板渲染。用实际值替换 `<w:sdt>` 内容控件。

**关键函数**：
- `render_docx(package_xml, manifest, values, overrides, separator)` — 渲染入口
- `build_tag_map(manifest)` — 构建 tag→field 映射
- `render_tree(node, tag_map, values, overrides, separator)` — 递归渲染

**问题**：
- **L93-95** 从右向左遍历 children（`while i > 0; i -= 1`），这样 splice 操作不会影响未处理的索引。这是处理可变数组遍历+删除的标准模式。
- **L193-236** `try_expand_table_row` 实现表格行复制（party_list 在表格中时，每个当事人复制一行）。使用 `children[idx].clone()` 而非 serialize→parse 往返，性能更好。

---

#### `src-tauri/src/docx_template/batch.rs` (777 行)

**功能概述**：批量填写——导出 xlsx、校验导入、批量渲染。

**关键函数**：
- `export_fields_xlsx(manifest, defaults, path)` — 导出字段表
- `validate_imported_xlsx(manifest, path)` — 校验导入
- `batch_render(manifest, xlsx, dir, ...)` — 批量渲染

**问题**：
- **L584** `engine::render_docx(args, "")` 传入空 source 字符串，跳过历史记录。这是有意的——批量渲染的历史由用户手动选择保存。
- **L586-588** 批量渲染的 `build_row_values` 在成功和错误路径都被调用（L570 和 L589），成功路径调用了两次。L589 的调用是为了构造 `BatchRenderRow` 供历史保存，但值已经在 L570 构建过一次。可以缓存避免重复构建。

---

#### `src-tauri/src/docx_template/index.rs` (143 行)

**功能概述**：文本索引数据结构。`TextNodeRef`, `TextIndex`, `DocumentIndex`。

**问题**：无。数据结构设计清晰。

---

#### `src-tauri/src/docx_template/ooxml.rs` (244 行)

**功能概述**：自定义 XML 解析器/序列化器（基于 quick-xml）。

**关键类型**：`XmlNode` (Element | Text), `XmlTree`

**问题**：
- **L23-44** 编码检测：拒绝 UTF-16BE/LE，剥离 UTF-8 BOM，检查 XML 声明中的 encoding。覆盖了常见的编码问题场景。
- **L76** `reader.config_mut().trim_text(false)` 保留空白，这对 Word 文档的 `xml:space="preserve"` 至关重要。

---

#### `src-tauri/src/docx_template/package.rs` (179 行)

**功能概述**：docx/docsytpl 包的读写。

**关键函数**：
- `read_docx_package(path)` / `write_docx_package(path, pkg)`
- `read_docsytpl_package(path)` / `write_docsytpl_package(path, manifest, pkg)`

**问题**：
- **L148-158** `atomic_temp_path` 使用纳秒时间戳作为 nonce，理论上在极端并发下可能冲突。但 Tauri 单窗口场景下不会有问题。
- **L63-78** `write_docsytpl_package` 使用原子写入（先写临时文件再 rename），防止写入中断导致文件损坏。设计良好。
- **L82-129** `read_zip_package` 有完善的大小限制检查（总大小、XML 大小、单条目大小、条目数量），防止 zip bomb 攻击。

---

#### `src-tauri/src/template_history.rs` (948 行)

**功能概述**：模板历史数据库（SQLite）。记录每次填写/生成的完整字段值，提供智能建议。

**关键函数**：
- `record_history_run(...)` — 记录生成历史
- `history_context(manifest, values, full_refresh)` — 获取建议上下文
- `list_generation_runs(limit)` — 列出历史记录
- `clear_history()` — 清空历史
- `merge_template_field_history(source, target)` — 合并跨模板历史

**问题**：
- **L326-343** `list_generation_runs` 中的孤儿检测：如果模板文件不存在于磁盘，自动标记为 trashed。这是合理的清理策略，但在网络驱动器或临时不可用的外部存储上可能误判。
- **L466-476** `open_db` 使用 WAL 模式 + `busy_timeout(5s)`，支持并发读取。设计合理。
- **L594-612** `ensure_template_meta` 使用 `INSERT ... ON CONFLICT DO UPDATE`（upsert），避免了之前的竞态条件（注释中提到了旧的 UPDATE-then-INSERT 方案的问题）。

---

#### `src-tauri/src/services/history.rs` (40 行)

**功能概述**：实际上是 `services/history.rs` 但内容是 `AppSettings` 的读写。文件名与内容不匹配。

**导出函数**：`get_settings()`, `save_settings(settings)`

**问题**：
- **文件名误导**：`services/history.rs` 实际处理的是 `AppSettings`，不是 history。应该重命名为 `settings.rs` 或类似名称。

---

## 2. Template 模块完整数据流

### 2.1 制作模板流程

```
用户选择 Word 文件
    │
    ▼
selectSourceDocx() → inspectSourceDocx()
    │
    ▼
tauriCallSafe('inspect_docx_template', {path})
    │  [Rust] engine::inspect_docx()
    │    → package::read_docx_package()     读取 ZIP
    │    → scan_package_to_runs_and_marks()  扫描 XML 构建 runs/marks
    │      → scan::scan_document_index()     逐 paragraph/run 索引
    │    → 返回 {documentText, documentRuns, marks}
    │
    ▼
前端接收 marks → normalizeFieldRows(autoMergeMarks(marks).map(markToRow))
    │
    │  normalizeFieldRows pipeline:
    │    1. autoSplitLeadingConnectorRows  拆分前导连接符
    │    2. autoSplitKnownSuffixRows       拆分已知后缀
    │    3. autoSplitLegalCompoundRows     拆分法律复合文本（案号+案由）
    │    4. dropGeneratedConnectorRows     删除生成的连接符行
    │    5. autoAssignStructureTargets     自动绑定前缀/后缀目标
    │    6. autoSplitPartyListRows         拆分当事人列表
    │    7. refreshPartyItemsForRows       刷新列表项
    │    8. reorderRowsByDocumentPosition  按文档位置排序
    │
    ▼
用户编辑字段行（改名、改类型、合并、拆分...）
    │
    ▼
saveTemplate()
    │  → validateFieldRowsBeforeSave()  验证所有标黄文本已处理
    │  → buildFields(fieldRows)         构建 manifest fields
    │
    ▼
tauriCallSafe('save_docx_template_to_library', {args})
    │  [Rust] engine::save_docx()
    │    → package::read_docx_package() / read_docsytpl_package()
    │    → scan_package_to_runs_and_marks()  重新扫描获取最新坐标
    │    → prune_stray_punctuation_refs()    清理意外标点引用
    │    → validate_manifest()               验证 manifest
    │    → save::build_template_docx()       构建模板 XML
    │      → validate_coordinate_targets()   验证坐标目标
    │      → strip_all_yellow_highlights()   清除所有黄色高亮
    │      → wrap_runs_by_coordinates()      按坐标包装 <w:sdt>
    │    → package::write_docsytpl_package() 写入 .docsytpl ZIP 包
    │
    ▼
返回 {outputPath, manifest}
    │
    ▼
前端切换到填写页 → openTemplatePackage(path)
```

### 2.2 填写模板流程

```
用户选择/打开模板
    │
    ▼
openTemplatePackage(path)
    │  → tauriCallSafe('inspect_docsytpl', {path})
    │    [Rust] inspect_template_package()
    │      → read_docsytpl_package()  读取 ZIP + manifest.json
    │  → templateManifest = result.data
    │  → resetFormValues(fields)      初始化表单值
    │  → loadHistoryContext()          加载历史建议
    │    → tauriCallSafe('get_template_history_context', ...)
    │      [Rust] history_context()
    │        → query_field_suggestions()      字段名建议
    │        → query_semantic_suggestions()   通用字段名建议
    │        → query_association_suggestions() 关联建议
    │
    ▼
用户填写表单 → formValues reactive 对象更新
    │
    │  引用字段自动解析 (watch formValues → resolveAllReferenceFields)
    │  历史建议更新 (scheduleHistoryRefresh → loadHistoryContext)
    │  填充预览更新 (watch formValues → buildFillPreview)
    │
    ▼
renderTemplate()
    │  → normalizeValues()            标准化表单值
    │    → 遍历 renderableTemplateFields
    │    → party_list → partyItemsToValues()
    │    → date → formatDateValue(value, format)
    │    → reference → resolveReferenceForRender()
    │    → 生成 {fieldId: value, fieldName: value, semanticKey: value, ...}
    │  → normalizeStructureOverrides() 标准化结构覆盖
    │
    ▼
tauriCallSafe('render_docx_template', {args})
    │  [Rust] engine::render_docx()
    │    → package::read_docsytpl_package()  读取模板包
    │    → render::render_docx()             渲染 XML
    │      → build_tag_map()                 构建 tag→field 映射
    │      → render_tree()                   递归渲染
    │        → 遇到 <w:sdt> → find_sdt_tag() → 查找 tag
    │        → __delete_* → 删除节点
    │        → 普通字段 → replace_sdt_content() → 替换文本
    │        → party_list → try_expand_table_row() → 表格行复制
    │        → 空值 → strip_prefix/suffix_before/after() → 清理周围文字
    │      → 输出渲染后的 XML bytes
    │    → package::write_docx_package()    写入输出 docx
    │    → record_history_run()             记录历史（best-effort）
    │
    ▼
返回 outputPath → openPath(outputPath) 打开生成的 Word
```

### 2.3 批量填写流程

```
导出字段表:
  exportBatchTemplate()
    → tauriCallSafe('export_template_fields_xlsx', ...)
    [Rust] batch::export_fields_xlsx()
      → 生成 xlsx: Row0=元数据, Row1=表头, Row2=示例行(否), Row4+=空行

导入并生成:
  importAndBatchRender()
    → tauriCallSafe('validate_batch_import', ...)
    [Rust] batch::validate_imported_xlsx()  → 校验结果
    → showValidationDialog()                → 用户确认
    → 选择输出目录
    → tauriCallSafe('batch_render_from_xlsx', ...)
    [Rust] batch::batch_render()
      → 逐行: build_row_values() → engine::render_docx(args, "")
      → 返回 {success, failed, rows}
    → 用户可选择保存历史 → openBatchSaveDialog()
    → tauriCallSafe('save_batch_history_rows', ...)
```

---

## 3. Core 层架构分析

### 3.1 分层结构

```
┌─────────────────────────────────────────────┐
│  Views (TemplateView.vue)                    │  状态中心 + 事件分发
├─────────────────────────────────────────────┤
│  Components (Build/Render/Settings/History)  │  纯 UI 展示 + emit
├─────────────────────────────────────────────┤
│  Composables                                 │  业务逻辑 + 状态管理
│  (useFieldNormalization, usePreviewSelection,│
│   useBatchFill, useTemplateSettings)         │
├─────────────────────────────────────────────┤
│  fieldRowUtils.js                            │  纯工具函数（无副作用）
├─────────────────────────────────────────────┤
│  rules/ (publicRules, courtNames, causeActions) │  推断规则 + 数据
├─────────────────────────────────────────────┤
│  Core (tauriBridge, filePath, loading, ...)  │  基础设施
├─────────────────────────────────────────────┤
│  Services (appLogger, devTracker)            │  横切关注点
├─────────────────────────────────────────────┤
│  Tauri IPC → Rust Backend                    │  文件 I/O + XML 处理
└─────────────────────────────────────────────┘
```

### 3.2 核心设计模式

1. **事件驱动的加载动画**：`tauriBridge.js` 发射 CustomEvent → `App.vue` 监听并显示 Doclet 动画。解耦了业务逻辑和 UI 反馈。

2. **纯函数工具层**：`fieldRowUtils.js` 的所有函数都是纯函数，接受 `rows` 作为参数而非访问闭包。这使得它们可以被 composable 和组件共同使用，且易于测试。

3. **Coordinate-based XML 模板**：前端的 `markId` 格式为 `{part}-p{paragraph}-r{run}`，Rust 后端的 scan 和 save 都使用相同的坐标系统遍历 XML 树。这种设计避免了基于文本匹配的歧义。

4. **Manifest-driven rendering**：模板的所有字段信息存储在 `manifest.json` 中，渲染时通过 `tag` 映射找到对应的 `<w:sdt>` 内容控件。

5. **History-driven suggestions**：SQLite 数据库记录每次填写的完整值，通过字段名、通用字段名、关联查询提供三层建议。

### 3.3 核心层的优势

- **tauriBridge.js** 的三层调用模式（tauriCall/tauriCallQuiet/tauriCallSafe）覆盖了不同场景的需求
- **useHistory** composable 提供了通用的 undo/redo 能力
- **usePointerReorder** 自定义拖拽排序，不依赖第三方库
- **moduleRegistry** 的自动发现机制使新模块只需创建 `index.js` 即可注册

### 3.4 核心层的改进空间

- **useHistory** 在 template 模块中未被使用（TemplateView.vue 自行实现了 `pushUndoSnapshot`/`undoLastAction`）。建议统一使用 useHistory。
- **loading.js** 与 tauriBridge.js 的关系可以通过合并简化——将 `showLoading`/`hideLoading` 直接放在 tauriBridge.js 中。

---

## 4. 所有发现的 Bug 和代码问题

### 🔴 严重问题

| # | 文件 | 行号 | 问题 | 影响 |
|---|------|------|------|------|
| 1 | `batch.rs` | 586-589 | `build_row_values` 在成功渲染路径被调用两次（L570 和 L589），第二次调用是多余的。L589 构造 `BatchRenderRow` 时应直接使用 L570 的结果。 | 性能浪费，每行批量渲染多一次字段值构建 |

### 🟡 中等问题

| # | 文件 | 行号 | 问题 | 影响 |
|---|------|------|------|------|
| 2 | `commands/template.rs` | 277-291 | `save_batch_history_rows` 对每行都调用 `inspect_template_package`，相同模板路径会重复解析。 | 批量保存历史时性能差 |
| 3 | `services/history.rs` | 全文件 | 文件名是 `history.rs` 但内容是 `AppSettings` 读写。 | 代码组织混乱，难以维护 |
| 4 | `tauriBridge.js` | 62-70 | `tauriCallQuiet` 不记录错误日志，静默失败。 | 生产环境排查困难 |
| 5 | `TemplateView.vue` | 173 | 直接导入 `invoke` 绕过 tauriBridge 统一错误处理和动画。 | 不一致的错误处理 |
| 6 | `TemplateView.vue` | 2143-2150 | `JSON.stringify(formValues)` 作为 watch source，每次变化都完整序列化。 | 字段多时可能有性能影响 |
| 7 | `TemplateView.vue` | 2708-2713 | `ensureExtension` 与 `fieldRowUtils.js` 中同名函数完全重复。 | 维护隐患 |
| 8 | `fieldRowUtils.js` | 176-204 | `splitPartyLabelSegments` 与 TemplateView.vue 中的同名函数重复实现。 | 维护隐患 |
| 9 | `TemplateRenderTab.vue` | 446-453 | `effectiveFieldType` 与 TemplateView.vue 版本逻辑微妙不同。 | 可能导致不一致行为 |
| 10 | `appLogger.js` | 8 | `looksSensitiveKey` 正则过于宽泛，"text" 几乎匹配所有 key。 | 调试信息被过度截断 |

### 🟢 轻微问题 / 代码异味

| # | 文件 | 行号 | 问题 | 建议 |
|---|------|------|------|------|
| 11 | `TemplateView.vue` | 全文件 | 2901 行，职责过重。 | 提取更多 composable |
| 12 | `fieldRowUtils.js` | 全文件 | 1199 行，60+ 导出函数。 | 按职责拆分文件 |
| 13 | `TemplateBuildTab.vue` | 680-705 | 25 个 props，30+ emits。 | 考虑 provide/inject |
| 14 | `useWindowFileDrop.js` | 12 | `onEnter` 在 `over` 事件时也被调用，命名不精确。 | 重命名为 `onDragActivity` |
| 15 | `pdfUtils.js` | 12-23 | `setRangeStart/End` 直接 mutate 传入对象。 | 在纯函数场景中考虑返回新对象 |
| 16 | `usePreviewSelection.js` | 285-293 | `triggerPreviewSelectionAdd` 与 TemplateView.vue 版本逻辑重复。 | 统一到 composable 中 |
| 17 | `TemplateView.vue` | 1094-1107 | 自行实现 undo 而非使用 `useHistory` composable。 | 统一使用 useHistory |
| 18 | `batch.rs` | 563 | `skip_rows.contains(&row_idx)` 对每行线性搜索。 | 使用 HashSet 提升查找效率 |
| 19 | `template_history.rs` | 326-343 | 孤儿检测修改了数据库（标记 trashed），但这是只读查询的副作用。 | 考虑分离清理逻辑 |

### ✅ 设计亮点

1. **Coordinate-based mark ID** (`{part}-p{paragraph}-r{run}`)：scan 和 save 使用相同的坐标遍历逻辑，保证了一致性。
2. **Atomic file writing**：`package.rs` 使用临时文件 + rename 的原子写入，防止文件损坏。
3. **Size limits**：`package.rs` 对 ZIP 包的各种大小都有限制，防止 zip bomb。
4. **WAL mode SQLite**：`template_history.rs` 使用 WAL 模式支持并发读取。
5. **Manifest caching**：`commands/template.rs` 的 manifest 缓存避免了重复解析。
6. **Prune stray punctuation refs**：`engine.rs` 自动清理意外标点引用，提升了模板质量。
7. **Party list table row replication**：`render.rs` 的表格行复制支持复杂的当事人列表渲染。
8. **Cross-template suggestion**：`template_history.rs` 的建议系统跨模板共享相同字段名的历史数据。
