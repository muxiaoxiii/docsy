# Docsy 架构审阅

> 审阅日期：2026-08-08
> 分支：codex/template-quickxml-0.8（MDG 工作分支）
> 审阅范围：前端 7 模块 + 后端 11 模块，约 40000 行代码
> 前置条件：用户已完成一轮重构（MDG-001 至 MDG-016）

---

## 1. 总体评价

Docsy 是一个 Tauri 2 桌面应用，面向中国律师处理证据 PDF 和 Word 模板。整体架构**清晰合理**——前端按功能模块划分、后端按领域分层、外部工具统一封装。经过 MDG-001 至 016 的重构，代码质量已有显著提升（cancel_operation 通用化、Mutex 安全模式、重复代码消除等）。

但仍存在以下**架构级问题**，按严重程度排列：

| # | 问题 | 严重度 | 影响 |
|---|------|--------|------|
| 1 | evidence-pdf 模块直接 import pdf-tools 视图，模块边界破裂 | 高 | 耦合扩散 |
| 2 | 错误处理全链路 String 化，前端无法区分错误类型 | 高 | 用户体验 |
| 3 | 两套取消机制并存（SubprocessRegistry + OperationManager） | 中 | 技术债 |
| 4 | App.vue 与 useAppStore settings 逻辑重复 | 中 | 状态不一致风险 |
| 5 | 日志系统自造轮子，无缓冲无级别过滤 | 中 | 性能/可观测性 |
| 6 | 文件选择器、确认对话框等通用交互未统一封装 | 中 | UI 不一致 |
| 7 | pdf/ 模块 11000 行，detection.rs 2645 行过大 | 低 | 可维护性 |

---

## 2. 前端架构

### 2.1 模块注册 — 设计良好

`src/core/moduleRegistry.js` 使用 Vite `import.meta.glob` 自动发现 7 个模块，按 `order` 排序。每个模块 `index.js` 导出统一形状（`id/name/icon/routes/menuItems/homeCards/order`），路由懒加载。**约定优于配置，新增模块零改 core**，这是项目最亮眼的架构设计。

### 2.2 模块间依赖 — 一处破裂

7 个模块之间基本无横向 import，全部向上依赖 `core/` 和 `shared/`，无循环依赖。

**唯一例外**：`src/modules/evidence-pdf/views/EvidencePdfView.vue:68` 直接 import `pdf-tools/views/EvidencePdfWorkbench.vue`。evidence-pdf 模块只有 265 行，本质上是一个薄壳，真正的逻辑在 pdf-tools 的 123KB 视图文件里。这导致：

- evidence-pdf 无法独立移除或替换
- pdf-tools 的变更会直接影响 evidence-pdf
- 模块边界名存实亡

**建议**：要么将 `EvidencePdfWorkbench` 下沉到 `shared/`，要么合并两个模块。考虑到 evidence-pdf 只有 265 行且职责就是"证据 PDF 工作台"，建议合并。

### 2.3 状态管理 — 双轨并行

项目安装了 Pinia，但只有一个 store（`src/stores/app.js`，管 settings）。而 `App.vue:76-106` 并未使用该 store，而是自己维护 `const settings = ref({...})` 并直接调 `tauriCallSafe('get_app_settings')`。

跨模块状态共享通过两种机制：
- `core/composables/`（useHistory / usePointerReorder / useWindowFileDrop）
- `window` CustomEvent（`docsy-operation-start/finish`、`docsy-settings-updated`）

**store 与事件总线两套机制并行**，settings 有两个来源，存在不一致风险。

**建议**：`App.vue` 改用 `useAppStore`，统一 settings 来源。事件总线保留用于操作动画，但状态性数据应走 store。

### 2.4 Tauri 桥接 — 一处违规

`tauriBridge.js` 提供三档封装（`tauriCall`/`tauriCallQuiet`/`tauriCallSafe`），统一日志和错误归因。但 `src/modules/template/views/TemplateView.vue:176` 直接 `import { invoke } from '@tauri-apps/api/core'`，绕过桥接，丢失日志和错误归因。

**建议**：改走 `tauriCallSafe`。可加 ESLint 规则禁止 `@tauri-apps/api/core` 的直接 import（`src/services/appLogger.js` 例外，属日志底层）。

### 2.5 组件复用 — 缺失统一封装

`src/components/UndoRedoButtons.vue` 孤立且未被引用。真正的共享组件在 `src/shared/components/`（DocletWorkingPet、FileQueuePanel、ReorderableImageGrid、FilenameTokenInput 等）。

**重复点**：
- 文件选择：各模块直接调 `@tauri-apps/plugin-dialog` 的 `open`，无统一封装
- 确认对话框：分散调用 `ElMessageBox.confirm`
- el-dialog：pdf-tools 下 3 个、EvidencePdfWorkbench 内嵌 1 个，各自实现

**建议**：
1. 抽出 `useFileDialog` composable
2. 抽出 `useConfirmDialog` composable
3. 统一 dialog 样式（宽度、按钮顺序、关闭行为）
4. 清理 `src/components/UndoRedoButtons.vue` 或将其移入 `shared/components/`

### 2.6 core 层 — 职责清晰

`src/core/` 下 7 个文件职责清晰、无重复：
- `moduleRegistry.js` — 模块发现
- `tauriBridge.js` — IPC 封装
- `filePath.js` — 路径字符串
- `numberFormat.js` — 中文数字
- `unitConversion.js` — mm/pt 转换
- `pdfUtils.js` — PDF 页码区间
- `loading.js` — 手动 loading 事件

唯一可议：`loading.js` 与 `tauriBridge.js` 共用同一套 `docsy-operation-*` CustomEvent，耦合在事件协议上。但 `tauriBridge.js:157` 已 re-export 便于单点引入，可接受。

---

## 3. 后端架构

### 3.1 命令注册 — 规范良好

`commands/mod.rs:62` 通过 `generate_handler!` 注册约 84 个命令，分 6 组（pdf/template/system/settings/video/image_paddler）。命名规范是 `动词_名词` 或 `动词_名词_目的`（如 `split_merged_evidence_pdf`）。参数/返回值经 serde 序列化，统一 `#[serde(rename_all = "camelCase")]`。

`mod.rs` 还提供两个执行封装：
- `run_blocking` — 快速命令，`spawn_blocking`
- `run_managed` — 长任务，自动注册 CancellationToken

### 3.2 错误处理 — 全链路 String 化

后端内部用 `anyhow::Result`，命令层通过 `run_blocking`/`run_managed` 把 `anyhow::Error` 转 `String` 返回前端。**没有自定义 Error 枚举**。

这导致：
- 前端只能拿到字符串，无法区分"文件不存在""权限不足""工具缺失""格式错误"等
- 前端只能做字符串匹配判断错误类型，脆弱且不可维护
- 错误信息中英文混合，格式不统一

**建议**：引入 `thiserror` 定义领域错误枚举：

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DocsyError {
    #[error("文件不存在: {path}")]
    FileNotFound { path: String },
    #[error("外部工具缺失: {tool}")]
    ToolMissing { tool: String, installUrl: Option<String> },
    #[error("PDF 处理失败: {reason}")]
    PdfFailed { reason: String, detail: Option<String> },
    #[error("操作已取消")]
    Cancelled,
    // ...
}
```

前端拿到结构化错误后可以做精准提示（"qpdf 未安装，点击下载" vs "文件损坏"）。

### 3.3 取消机制 — 双轨并存

两套取消机制：
- `SubprocessRegistry`（`OnceLock<Arc<...>>`）— 旧路径，按 PID kill 子进程
- `OperationManager`（Arc，manage 进 Tauri State）— 新路径，CancellationToken 协作式取消

注释明说"新命令用 run_managed，旧命令不改"。这是明确的技术债——两套机制行为不同（kill vs 协作），维护者需要判断每个命令走哪条路。

**建议**：逐步迁移旧命令到 `run_managed`，最终废弃 `SubprocessRegistry`。优先迁移有子进程调用的命令（qpdf/ffmpeg/pdftotext）。

### 3.4 日志系统 — 自造轮子

`app_log.rs` 自实现 JSON Lines 日志，每次写日志重新打开文件（append），**无缓冲、无级别过滤**。前端日志经 `write_frontend_log` 命令回传。

优点：零依赖、JSON 格式、自动清理 14 天、panic hook。
缺点：无缓冲（每次 fsync）、无级别过滤（debug 日志也会写）、无日志轮转（单文件可能很大）、非 `log`/`tracing` 生态（无法与第三方库日志统一）。

**建议**：中期迁移到 `tracing` + `tracing-appender`，获得异步写入、日志轮转、级别过滤、span 上下文。短期至少加缓冲和级别过滤。

### 3.5 外部工具 — 封装良好，路径硬编码

`external/` 封装 qpdf/ffmpeg/poppler/libreoffice/word/wps 六个工具，统一 `ExternalTool` trait（`check`/`try_install`/`binary_path`）。`managed.rs` 是下载安装器——GitHub releases + SHA256 强制校验。

`binary_path()` 三级查找：托管目录 → Homebrew 已知路径 → `which`/`where`。

**问题**：Homebrew 路径硬编码（`/opt/homebrew/bin`、`/usr/local/bin`），不支持环境变量或配置项。`libreoffice_path` 已可配，其他工具应跟进。

**问题**：下载源默认 GitHub releases，对国内用户不友好。已支持 `tool_manifest_url` 设置项覆盖，但默认值需优化（可考虑国内镜像）。

### 3.6 后端模块规模

| 模块 | 行数 | 评估 |
|------|------|------|
| pdf/ | 11013 | ⚠️ 过大，detection.rs 2645 行 + header_footer.rs 2369 行需拆分 |
| docx_template/ | 5659 | 偏大但子模块划分合理（8 个子模块） |
| external/ | 1994 | 合理 |
| image_paddler.rs | 1587 | 单文件偏大，可拆为 paddler_core + layout + watermark |
| template_history.rs | 949 | 合理 |
| commands/ | 1132 | 合理，分 6 组 |
| ffmpeg/ | 578 | 合理 |
| operations.rs | 317 | 合理 |

**pdf/ 模块**有 14 个子模块，但 detection.rs（2645 行）和 header_footer.rs（2369 行）各自过大。detection.rs 已在 MDG-016 中提出拆分建议。header_footer.rs 包含字体嵌入、overlay 生成、qpdf 调用、书签、process_job 等不同职责，应拆为 font.rs / overlay.rs / bookmark.rs / job.rs。

### 3.7 并发状态

四套全局状态：
- `APP_HANDLE`（OnceLock）— 全局 AppHandle
- `SUBPROCESS_REGISTRY`（OnceLock<Arc>）— 旧取消机制
- `OperationManager`（Arc，Tauri State）— 新取消机制
- `ConversionState`（Arc，Tauri State）— COM 转换超时

并发用 `std::sync::Mutex`。MDG-002 已修复 Mutex::unwrap 的 panic 风险。两套取消机制并存是主要技术债（见 3.3）。

---

## 4. 跨层面问题

### 4.1 前后端数据契约 — 无 schema

84 个 Tauri 命令的参数和返回值没有统一的 schema 定义。前端各 composable 各自定义 JS 对象结构，后端各 struct 各自定义 Rust 结构，两者靠 `camelCase` serde 约定对应。

**风险**：字段重命名、类型变更时无编译期检查，只能靠运行时发现。

**建议**：长期考虑用 TypeScript 类型定义 + Rust struct 的自动生成（如 `ts-rs` crate），或至少在 `docs/` 下维护一份命令 schema 文档。

### 4.2 外部工具版本依赖 — 无版本锁定

qpdf/ffmpeg/poppler 的版本由 managed 安装器的 `tools-manifest.json` 决定，但没有在运行时校验最低版本。如果用户用了系统自带旧版本工具，可能出现功能异常但无提示。

**建议**：`ExternalTool::check()` 应返回版本号，与最低要求版本对比。

### 4.3 临时文件管理 — 无统一清理

各模块各自创建临时文件（`temp_named_path`），处理完成后手动 `remove_file`。如果进程异常退出，临时文件会残留。

**建议**：注册应用退出时的清理钩子，或在启动时清理 `_docsy_*` 前缀的临时文件。

---

## 5. 架构改进建议（按优先级）

### P0 — 影响用户体验和代码安全

1. **错误类型结构化**：引入 `DocsyError` 枚举，前端能区分错误类型做精准提示
2. **合并 evidence-pdf 到 pdf-tools**：消除模块边界破裂

### P1 — 影响可维护性

3. **统一 settings 来源**：App.vue 改用 useAppStore
4. **禁止裸 invoke**：TemplateView.vue 改走 tauriBridge + ESLint 规则
5. **统一文件选择器/确认对话框**：抽出 composable
6. **取消机制统一**：逐步迁移到 OperationManager，废弃 SubprocessRegistry

### P2 — 影响长期演进

7. **日志迁移 tracing**：获得异步、轮转、级别过滤
8. **pdf/ 模块拆分**：detection.rs 和 header_footer.rs 拆小
9. **路径配置化**：外部工具路径支持环境变量
10. **临时文件清理**：统一清理机制

### P3 — 锦上添花

11. **命令 schema 文档**：维护前后端数据契约
12. **工具版本校验**：运行时检查最低版本
13. **image_paddler.rs 拆分**：单文件 1587 行偏大

---

## 6. 模块审阅计划

后续逐个模块审阅，按以下顺序（规模和风险排序）：

| 顺序 | 模块 | 前端行数 | 后端行数 | 审阅重点 |
|------|------|---------|---------|---------|
| 1 | template | 10718 | 6608 | 模板系统核心、Word XML 处理、批量渲染 |
| 2 | pdf-tools（非页眉页脚） | 10718 | 11013 | 证据分组/合并、anti-OCR、预览、normalize |
| 3 | image-paddler | 1107 | 1587 | 图片排版、水印、分组排序 |
| 4 | video-extract | 722 | 578 | 视频抽帧、ffprobe、drawtext |
| 5 | settings | 862 | 61 | 设置持久化、工具配置 |
| 6 | evidence-pdf + home | 451 | — | 合并评估、首页卡片 |

每个模块产出一份审阅文档，记录在 `docs/glm/` 目录，命名格式 `GLM-{模块名}-review.md`。

UI 一致性审阅作为横切关注点，在各模块审阅中同步进行，最后汇总为 `GLM-ui-consistency-review.md`。

---

## 7. 总结

Docsy 的架构基础是好的——模块自动注册、core 层职责清晰、外部工具有统一封装、Tauri 桥接有三档封装。经过 MDG-001 至 016 的重构，代码质量已显著提升。

当前最需要解决的架构问题是**错误处理全链路 String 化**和**evidence-pdf 模块边界破裂**。前者影响用户体验（无法精准提示），后者影响代码可维护性（耦合扩散）。这两个问题应该在逐个模块审阅之前优先解决，因为它们是横切层面的，会反复出现在每个模块的审阅中。
