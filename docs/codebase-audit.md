# Docsy 代码库全面审计报告

> 审计日期: 2026-08-07  
> 项目路径: `/Users/only/Documents/PythonProgram/Docsy`  
> 技术栈: Tauri 2 + Rust (后端) + Vue 3 + Element Plus (前端)  
> 前端文件: 80+ | Rust 文件: 52

---

## 一、架构总览 / Architecture Overview

Docsy 是一个 Tauri 桌面应用，面向法律/知识产权从业者，提供 Word 模板填充、PDF 工具、证据整理、视频抽帧和图片排版功能。

### 整体数据流 / Overall Data Flow

```
┌─────────────────────────────────────────────────────────┐
│  Vue 3 Frontend (src/)                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ Template │  │ PDF Tools│  │ Evidence │  ...           │
│  │ Module   │  │ Module   │  │ PDF      │               │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘               │
│       │              │              │                    │
│  ┌────▼──────────────▼──────────────▼─────┐             │
│  │  Core Layer (src/core/)                │             │
│  │  tauriBridge.js → invoke()             │             │
│  │  moduleRegistry.js → route/menu        │             │
│  │  composables/ → reusable logic         │             │
│  └────────────────┬───────────────────────┘             │
└───────────────────┼─────────────────────────────────────┘
                    │ Tauri IPC (invoke / emit)
┌───────────────────┼─────────────────────────────────────┐
│                   │  Rust Backend (src-tauri/src/)       │
│  ┌────────────────▼───────────────────────┐             │
│  │  commands/ → Tauri command handlers     │             │
│  │  ├ template.rs  (模板 CRUD + 渲染)      │             │
│  │  ├ pdf.rs       (PDF 操作)              │             │
│  │  ├ settings.rs  (设置管理)              │             │
│  │  ├ system.rs    (系统工具)              │             │
│  │  ├ video.rs     (视频抽帧)              │             │
│  │  └ image_paddler.rs (图片排版)          │             │
│  └─────────────────────────────────────────┘             │
│  ┌─────────────────────────────────────────┐             │
│  │  docx_template/ → Word OOXML 引擎       │             │
│  │  pdf/           → PDF 处理引擎          │             │
│  │  external/      → 外部工具封装          │             │
│  │  ffmpeg/        → FFmpeg 封装           │             │
│  │  services/      → 持久化服务            │             │
│  └─────────────────────────────────────────┘             │
└──────────────────────────────────────────────────────────┘
```

---

## 二、前端模块分析 / Frontend Module Analysis

### 2.1 核心层 (src/core/)

#### 功能概述
前端核心基础设施，提供 Tauri IPC 桥接、模块注册、可复用组合式函数和工具函数。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `tauriBridge.js` | Tauri IPC 统一封装：`tauriCall`、`tauriCallSafe`、`tauriCallQuiet`，操作动画事件分发 |
| `moduleRegistry.js` | 模块自动发现（`import.meta.glob`），路由/菜单/首页卡片生成 |
| `loading.js` | 手动加载动画触发器（`showLoading`/`hideLoading`） |
| `pdfUtils.js` | PDF 页码操作工具函数（范围计算、页面选择解析） |
| `numberFormat.js` | 数字格式化 |
| `unitConversion.js` | 单位转换（mm ↔ pt） |
| `filePath.js` | 文件路径工具 |
| `composables/useHistory.js` | 撤销/重做栈 |
| `composables/usePointerReorder.js` | 拖拽排序 |
| `composables/useWindowFileDrop.js` | 窗口文件拖放 |

#### 数据流
```
User Action → Component → tauriBridge.tauriCallSafe(cmd, args)
  → emitOperationEvent('start') → DocletWorkingPet动画
  → invoke(cmd, args) → Rust Command Handler
  → emitOperationEvent('finish') → 动画结束
  → Result 返回 → Component 更新UI
```

#### 关键函数
- `tauriCallSafe(command, args)`: 安全 IPC 调用，捕获错误返回 `{ok, data/error}`
- `userFacingError(error)`: 将后端错误转为用户友好消息
- `emitOperationEvent(type, command, id)`: 操作动画事件分发

---

### 2.2 模板模块 (modules/template/)

#### 功能概述
Word 文档模板的创建、编辑、填充和批量生成。用户在 Word 中用黄色高亮标记字段位置，Docsy 读取后将其转为可填充的表单。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `index.js` | 模块注册：路由、菜单项、首页卡片 |
| `views/TemplateView.vue` | 主视图：Build/Render/Settings/History 四标签页 |
| `components/TemplateBuildTab.vue` | 模板构建：字段行编辑、标记映射 |
| `components/TemplateRenderTab.vue` | 模板填充：字段表单、历史建议、预览 |
| `components/TemplateSettingsTab.vue` | 模板字段设置（引用源、日期格式） |
| `components/TemplateHistoryTab.vue` | 生成历史列表 |
| `composables/useFieldNormalization.js` | 字段值规范化 |
| `composables/fieldRowUtils.js` | 字段行操作工具 |
| `composables/useBatchFill.js` | 批量填充逻辑 |
| `composables/usePreviewSelection.js` | 预览选区管理 |
| `composables/useTemplateSettings.js` | 模板设置持久化 |
| `rules/causeActions2025.js` | 案由行动态规则 |
| `rules/courtNames.js` | 法院名称规则 |
| `rules/publicRules.js` | 公共规则（页码格式等） |

#### 数据流
```
1. 构建模板:
   Word文件 → inspect_docx_template → {marks, runs, text}
   → 用户选择标记 → 组装 fields → save_docx_template
   → engine::save_docx → scan + save → .docsytpl 包

2. 填充模板:
   .docsytpl → inspect_docsytpl → manifest (fields)
   → 用户填写表单值 → render_docx_template
   → engine::render_docx → 替换 <w:sdt> → 输出 .docx

3. 批量生成:
   模板 fields → export_template_fields_xlsx → .xlsx 模板
   → 用户编辑 xlsx → validate_batch_import → 校验
   → batch_render_from_xlsx → 逐行渲染 → 多个 .docx
```

#### 依赖关系
- **后端**: `commands/template.rs` → `docx_template/*`
- **核心层**: `tauriBridge.js`, `moduleRegistry.js`
- **历史**: `template_history.rs` (SQLite)

---

### 2.3 PDF 工具模块 (modules/pdf-tools/)

#### 功能概述
PDF 操作工具集：解锁、合并、拆分、页眉页脚检测/添加/删除、批注管理、防复制保护、书签管理。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `index.js` | 模块注册 |
| `views/PdfToolsView.vue` | PDF 工具主视图（解锁/合并/拆分/压缩/提取） |
| `views/EvidencePdfWorkbench.vue` | 证据 PDF 工作台 |
| `components/PdfJsPreview.vue` | PDF.js 预览组件 |
| `components/HeaderFooterRuleFields.vue` | 页眉页脚规则编辑 |
| `components/PageNumberRuleDialog.vue` | 页码规则对话框 |
| `components/TextPlacementFields.vue` | 文字定位字段 |
| `components/ExistingPdfElementsDialog.vue` | 已有 PDF 元素对话框 |
| `composables/useEvidencePdfSession.js` | 证据 PDF 会话管理 |
| `composables/useEvidencePdfDetection.js` | 页眉页脚检测 |
| `composables/useEvidencePdfPreview.js` | 证据 PDF 预览 |
| `composables/useEvidencePdfExistingEditing.js` | 已有元素编辑 |
| `composables/useEvidencePdfMergedImport.js` | 合并导入 |
| `composables/pdfPageNumberRules.js` | 页码规则引擎 |
| `composables/pdfPreviewCoordinates.js` | 预览坐标转换 |
| `composables/existingPdfElements.js` | PDF 元素提取 |
| `composables/splitFileName.js` | 拆分文件命名 |
| `composables/usePdfSplitRanges.js` | 拆分范围管理 |

#### 数据流
```
1. PDF 解锁:
   用户选择PDF → inspect_pdf → {encrypted, pages}
   → unlock_pdf → qpdf --decrypt → _unlocked.pdf

2. 证据整理:
   扫描文件夹 → scan_evidence_folder → 分组
   → build_evidence_group_pdfs → Word→PDF转换 + 合并
   → apply_evidence_pdf_rules → 页眉页脚 + 批注 + 合并

3. 页眉页脚检测:
   PDF → detect_pdf_header_footer → pdftotext -bbox
   → 解析XML → 聚合候选 → 输出检测结果

4. 页眉页脚添加:
   用户配置规则 → preview_pdf_header_footer → 预览
   → overlay_pdf_text → lopdf 写入 → 输出PDF
```

#### 依赖关系
- **后端**: `commands/pdf.rs` → `pdf/*` (15个子模块)
- **外部工具**: qpdf, poppler (pdftotext/pdftoppm)
- **核心层**: `pdfUtils.js`, `tauriBridge.js`

---

### 2.4 证据 PDF 模块 (modules/evidence-pdf/)

#### 功能概述
证据 PDF 的独立入口，提供证据文件夹扫描、分组、合并、页眉页脚处理的端到端工作流。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `index.js` | 模块注册 |
| `views/EvidencePdfView.vue` | 证据 PDF 视图（入口页面） |

#### 依赖关系
- 大量复用 `pdf-tools` 模块的 composables
- 后端复用 `pdf/evidence.rs`, `pdf/evidence_session.rs`

---

### 2.5 视频抽帧模块 (modules/video-extract/)

#### 功能概述
从视频文件中按时间点或频率抽取帧图片。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `index.js` | 模块注册 |
| `views/VideoExtractView.vue` | 视频抽帧视图 |

#### 数据流
```
视频文件 → check_ffmpeg → probe_video → 视频信息
→ 用户配置抽帧参数 → extract_frames → ffmpeg 抽帧
→ list_output_frames → 显示结果
```

#### 依赖关系
- **后端**: `commands/video.rs` → `ffmpeg/*`
- **外部工具**: ffmpeg, ffprobe

---

### 2.6 图片排版模块 (modules/image-paddler/)

#### 功能概述
将多张图片批量排版为 A4 PDF/DOCX 文档，支持多种布局、自动推荐设置。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `index.js` | 模块注册 |
| `views/ImagePaddlerView.vue` | 图片排版视图 |

#### 数据流
```
图片文件夹 → analyze_image_paddler_folder → 图片信息 + 推荐设置
→ 用户配置布局 → run_image_paddler → 生成 PDF/DOCX
```

#### 依赖关系
- **后端**: `commands/image_paddler.rs` → `image_paddler.rs`
- **依赖**: `image` crate (图片处理)

---

### 2.7 设置模块 (modules/settings/)

#### 功能概述
应用设置管理：菜单可见性、外部工具路径、工具安装。

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `index.js` | 模块注册 |
| `views/SettingsView.vue` | 设置页面 |
| `views/AboutView.vue` | 关于页面 |

#### 依赖关系
- **后端**: `commands/settings.rs` → `services/history.rs`, `external/managed.rs`

---

### 2.8 共享组件 (src/shared/)

#### 核心文件清单
| 文件 | 职责 |
|------|------|
| `components/DocletWorkingPet.vue` | 加载动画宠物 |
| `components/FileQueuePanel.vue` | 文件队列面板 |
| `components/ImagePreviewGrid.vue` | 图片预览网格 |
| `components/ReorderableImageGrid.vue` | 可排序图片网格 |
| `components/ToolWorkspaceShell.vue` | 工具工作区外壳 |
| `components/reorderableItems.js` | 排序逻辑 |

---

## 三、Rust 后端模块分析 / Backend Module Analysis

### 3.1 入口与状态管理 (lib.rs, main.rs)

#### 功能概述
应用初始化、全局状态管理、子进程注册。

#### 核心结构
- `SubprocessRegistry`: 子进程 PID 注册表，支持取消操作
- `ConversionState`: Word 转换超时状态（原子变量 + 轮询等待）
- `APP_HANDLE`: 全局 Tauri AppHandle（用于非命令上下文发射事件）
- `SUBPROCESS_REGISTRY`: 全局子进程注册表

#### 关键函数
- `run()`: 应用入口，初始化日志、注册插件、设置状态、启动 Tauri
- `SubprocessRegistry::spawn_and_wait()`: 可取消的子进程执行
- `ConversionState::wait_for_user_response()`: 超时后等待用户决策

---

### 3.2 命令层 (commands/)

#### 功能概述
Tauri 命令处理器，前端 IPC 的入口点。每个命令都是 `#[tauri::command]` 标注的异步函数。

#### 文件清单
| 文件 | 命令数 | 职责 |
|------|--------|------|
| `template.rs` | 25 | 模板 CRUD、渲染、历史、批量 |
| `pdf.rs` | 24 | PDF 操作（解锁/合并/拆分/页眉页脚/批注/防复制/书签） |
| `settings.rs` | 7 | 设置读写、外部工具管理 |
| `system.rs` | 12 | 系统操作（打开路径/URL、日志、诊断、字体列表、取消） |
| `video.rs` | 4 | 视频操作（检测/探测/抽帧/列出） |
| `image_paddler.rs` | 2 | 图片排版（分析/执行） |

#### 统一模式
所有命令使用 `run_blocking()` 将阻塞操作移至线程池：
```rust
pub async fn some_command(args: SomeArgs) -> Result<SomeResult, String> {
    run_blocking(move || some_backend_function(&args)).await
}
```

---

### 3.3 Word 模板引擎 (docx_template/)

#### 功能概述
完整的 OOXML (Word) 模板引擎：读取、扫描、保存、渲染。

#### 文件清单
| 文件 | 职责 |
|------|------|
| `mod.rs` | 类型定义、验证、库管理、工具函数 |
| `engine.rs` | 顶层编排：inspect_docx、save_docx、render_docx |
| `scan.rs` | XML 扫描：构建 TextIndex（段落、run、高亮检测） |
| `save.rs` | 模板保存：将字段映射写入 <w:sdt> 内容控件 |
| `render.rs` | 模板渲染：替换 <w:sdt> 为字段值 |
| `index.rs` | 文本索引数据结构 |
| `ooxml.rs` | XML 解析/序列化（quick-xml） |
| `package.rs` | ZIP 包读写（docx/docsytpl） |
| `batch.rs` | 批量操作：xlsx 导出、验证、批量渲染 |

#### 数据流
```
1. inspect_docx:
   .docx → package::read_docx_package → HashMap<name, bytes>
   → scan::scan_package_index_to_document_index → TextIndex
   → 提取 runs + marks + text → TemplateInspection

2. save_docx:
   .docx + fields → engine::save_docx
   → scan_package_to_runs_and_marks → 验证坐标
   → save::build_template_docx → wrap_runs_by_coordinates → <w:sdt>
   → package::write_docsytpl_package → .docsytpl (manifest + XML)

3. render_docx:
   .docsytpl + values → engine::render_docx
   → render::render_docx → 遍历 <w:sdt> → 替换内容
   → package::write_docx_package → .docx
```

#### 关键设计
- **坐标系统**: 使用 `(part, paragraph_index, run_index)` 三元组标识标记位置
- **黄色高亮**: 用户在 Word 中用黄色高亮标记字段，保存时清除高亮
- **内容控件**: 保存时将标记文本包裹在 `<w:sdt>` + `<w:tag>` 中
- **安全限制**: ZIP 条目数 ≤ 4096, 单文件 ≤ 512MB, XML ≤ 128MB

---

### 3.4 PDF 处理引擎 (pdf/)

#### 功能概述
PDF 操作的完整引擎，基于 lopdf (纯 Rust) + qpdf/poppler (外部工具)。

#### 文件清单
| 文件 | 职责 |
|------|------|
| `qpdf.rs` | qpdf 封装：inspect/unlock/merge/split/compress/extract |
| `header_footer.rs` | 页眉页脚处理：overlay_text、batch_overlay、书签 |
| `detection.rs` | 页眉页脚检测：pdftotext bbox 分析、候选聚合 |
| `evidence_session.rs` | 证据会话：apply_rules（批注删除 + 页眉页脚 + 合并） |
| `evidence.rs` | 证据管理：scan_folder、build_group_pdfs、merge_all |
| `split.rs` | PDF 拆分：split_merged（按页段拆分 + 可选清理） |
| `annotations.rs` | 批注管理：delete_annotations |
| `artifacts.rs` | 标准页眉页脚：inspect/delete/edit form artifacts |
| `anti_ocr.rs` | 防复制保护：CMap 篡改/移除/文字覆盖 |
| `preview.rs` | PDF 预览：render_preview (pdftoppm) |
| `page_info.rs` | 页面信息：get_page_infos (qpdf --json) |
| `content_text.rs` | 内容文本清理：delete_plain_header_footer |
| `normalize.rs` | A4 规范化：normalize_pdf_to_a4 |
| `overlay.rs` | 兼容层（re-export header_footer + preview） |

#### 关键设计
- **qpdf 集成**: 通过 `run_cancellable()` 执行，支持取消
- **lopdf 直接操作**: 批注、防复制、书签等直接操作 PDF 对象
- **pdftotext bbox**: 页眉页脚检测使用 poppler 的 bbox XML 输出
- **表单 XObject**: artifacts.rs 支持嵌套表单 XObject 中的页眉页脚

---

### 3.5 外部工具封装 (external/)

#### 功能概述
统一的外部工具管理框架：检测、安装、路径解析、命令执行。

#### 文件清单
| 文件 | 职责 |
|------|------|
| `mod.rs` | ExternalTool trait、工具检查/安装分发、命令执行工具 |
| `qpdf.rs` | qpdf 工具 |
| `ffmpeg.rs` | ffmpeg 工具 |
| `poppler.rs` | poppler 工具 (pdftotext/pdftoppm) |
| `libreoffice.rs` | LibreOffice 工具 |
| `word.rs` | Microsoft Word 工具 |
| `wps.rs` | WPS Writer 工具 |
| `managed.rs` | 托管工具安装/管理 |

#### ExternalTool trait
```rust
pub trait ExternalTool: Send + Sync {
    fn check(&self) -> ToolStatus;
    fn try_install(&self) -> anyhow::Result<String>;
    fn binary_path(&self) -> anyhow::Result<PathBuf>;
}
```

#### 命令执行
- `hidden_command()`: Windows 下隐藏控制台窗口
- `command_output_with_timeout()`: 带超时的命令执行
- `command_output_with_idle_timeout()`: 空闲超时（流式读取 stdout/stderr）

---

### 3.6 FFmpeg 封装 (ffmpeg/)

#### 文件清单
| 文件 | 职责 |
|------|------|
| `probe.rs` | 视频信息探测 |
| `extract.rs` | 帧提取 |
| `detect.rs` | 系统字体列表、drawtext 检测 |

---

### 3.7 服务层 (services/)

#### 文件清单
| 文件 | 职责 |
|------|------|
| `history.rs` | 应用设置持久化 (`settings.json`) |
| `module_registry.rs` | 模块描述符（静态注册） |

---

### 3.8 模板历史 (template_history.rs)

#### 功能概述
SQLite 数据库，记录模板填充历史，提供字段值建议。

#### 核心表结构
- `generation_runs`: 每次生成的记录（模板ID、输出路径、字段值JSON、来源）
- `field_history`: 每个字段的值历史（用于建议）
- `template_meta`: 模板元数据（名称、路径、回收站状态）

#### 关键功能
- `record_history_run()`: 记录一次生成
- `history_context()`: 获取历史上下文（last_values + field_suggestions + semantic_suggestions + association_suggestions）
- `list_generation_runs()`: 列出生成历史
- `merge_template_field_history()`: 跨模板字段历史合并
- `migrate_template_data_to_common()`: 删除模板时迁移历史到公共池

#### 设计特点
- WAL 模式：支持并发读取
- 自动清理：模板文件不存在时自动标记为 trashed
- 建议去重：基于 display_value 的 DISTINCT 查询

---

### 3.9 图片排版引擎 (image_paddler.rs)

#### 功能概述
图片扫描、分组、排版为 A4 PDF/DOCX。

#### 核心功能
- `analyze()`: 扫描图片、分组、推荐设置
- `run()`: 执行排版（支持 per_folder 输出模式）
- 布局解析：1x1, 2x1, 2x2, custom
- 自动推荐：根据图片比例和分辨率推荐布局
- 文件名处理：规则引擎（保留编号/时间/文本，替换/删除）

---

### 3.10 工具模块

| 文件 | 职责 |
|------|------|
| `sort_utils.rs` | 自然排序（中文+数字） |
| `app_log.rs` | 日志系统（JSON 格式、14天轮转、panic hook） |

---

## 四、模块间依赖关系图

```
┌──────────────────────────────────────────────────────────────┐
│                    Frontend (Vue 3)                           │
│                                                              │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────┐ ┌────────┐ │
│  │template │ │pdf-tools │ │evidence  │ │video │ │image   │ │
│  │         │ │          │ │-pdf      │ │-extr │ │-paddler│ │
│  └────┬────┘ └────┬─────┘ └────┬─────┘ └──┬───┘ └───┬────┘ │
│       │           │             │           │         │      │
│  ┌────▼───────────▼─────────────▼───────────▼─────────▼────┐ │
│  │              core/ (tauriBridge, moduleRegistry)         │ │
│  └────────────────────────┬────────────────────────────────┘ │
└───────────────────────────┼──────────────────────────────────┘
                            │ Tauri IPC
┌───────────────────────────┼──────────────────────────────────┐
│  ┌────────────────────────▼────────────────────────────────┐ │
│  │              commands/ (command handlers)                │ │
│  └──┬──────────┬──────────┬──────────┬──────────┬──────────┘ │
│     │          │          │          │          │             │
│  ┌──▼───┐  ┌──▼───┐  ┌───▼──┐  ┌───▼──┐  ┌───▼───┐         │
│  │docx_ │  │ pdf/ │  │exter-│  │ffmpeg│  │image_ │         │
│  │templ-│  │      │  │nal/  │  │      │  │paddler│         │
│  │ate/  │  │      │  │      │  │      │  │       │         │
│  └──┬───┘  └──┬───┘  └──┬───┘  └──┬───┘  └───────┘         │
│     │         │         │         │                          │
│  ┌──▼─────────▼─────────▼─────────▼──────────────────────┐  │
│  │           services/ + template_history                  │  │
│  │           (SQLite, settings.json)                       │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

---

## 五、外部依赖

### Rust 后端
| Crate | 用途 |
|-------|------|
| `tauri` | 桌面框架 |
| `lopdf` | PDF 读写（纯 Rust） |
| `printpdf` | PDF 字体/CMAP 生成 |
| `allsorts` | 字体子集化 |
| `quick-xml` | OOXML XML 解析/序列化 |
| `zip` | ZIP 包读写 |
| `calamine` | Excel 读取 |
| `rust_xlsxwriter` | Excel 写入 |
| `rusqlite` | SQLite 数据库 |
| `image` | 图片处理 |
| `regex` | 正则表达式 |
| `serde`/`serde_json` | 序列化 |
| `anyhow`/`thiserror` | 错误处理 |
| `reqwest` | HTTP（URL 校验） |
| `base64` | Base64 编码 |
| `chrono` | 时间处理 |
| `dirs` | 系统目录 |
| `office_oxide` | .doc → .docx 转换 |

### 前端
| 包 | 用途 |
|---|------|
| `vue` 3 | UI 框架 |
| `vue-router` | 路由 |
| `pinia` | 状态管理 |
| `element-plus` | UI 组件库 |
| `@tauri-apps/api` | Tauri IPC |
| `pdfjs-dist` | PDF 预览 |

### 外部工具
| 工具 | 用途 |
|------|------|
| `qpdf` | PDF 操作（解锁/合并/拆分/压缩） |
| `poppler` | PDF 文本提取/渲染 |
| `ffmpeg`/`ffprobe` | 视频处理 |
| `LibreOffice` | Word→PDF 转换（后备） |
| `Microsoft Word` | Word→PDF 转换（首选，macOS/Windows） |
| `WPS Writer` | Word→PDF 转换（Windows 后备） |

---

## 六、数据存储

| 存储 | 位置 | 格式 |
|------|------|------|
| 模板库 | `~/Library/Application Support/Docsy/templates/` | `.docsytpl` (ZIP) |
| 模板回收站 | `templates/_trash/` | `.docsytpl` |
| 历史数据库 | `~/Library/Application Support/Docsy/template_history.sqlite3` | SQLite3 |
| 应用设置 | `~/Library/Application Support/Docsy/settings.json` | JSON |
| 日志 | `~/Library/Application Support/Docsy/logs/docsy-YYYYMMDD.log` | NDJSON |
| 托管工具 | `~/Library/Application Support/Docsy/tools/` | 二进制 |

---

## 七、测试覆盖

### 前端测试
- `useHistory.test.js` — 撤销/重做
- `usePointerReorder.test.js` — 拖拽排序
- `tauriBridge.test.js` — IPC 桥接
- `moduleRegistry.test.js` — 模块注册
- `pdfUtils.test.js` — PDF 工具函数
- `useEvidencePdfSession.test.js` — 证据会话
- `useEvidencePdfDetection.test.js` — 页眉页脚检测
- `pdfPageNumberRules.test.js` — 页码规则
- `pdfPreviewCoordinates.test.js` — 预览坐标
- `existingPdfElements.test.js` — PDF 元素
- `splitFileName.test.js` — 拆分命名
- `usePdfSplitRanges.test.js` — 拆分范围
- `reorderableItems.test.js` — 排序
- `publicRules.test.js` — 公共规则

### Rust 测试
- `qpdf.rs` — 加密检测、退出码解析
- `detection.rs` — 页眉页脚检测（bbox 解析、候选聚合、拆分建议）
- `artifacts.rs` — 标准页眉页脚（表单 XObject、实际文本提取）
- `content_text.rs` — 普通文本清理
- `normalize.rs` — A4 规范化
- `split.rs` — 拆分验证
- `annotations.rs` — 批注删除
- `scan.rs` — XML 扫描
- `engine.rs` — 端到端（inspect → save → render）
- `page_info.rs` — 页面尺寸解析
- `external/mod.rs` — 命令失败详情
- `sort_utils.rs` — 自然排序
- `index.rs` — 文本索引

---

*审计完成。详细 Bug 报告请参阅 `bug-report.md`。*
