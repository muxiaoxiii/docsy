# 媒体处理模块审阅（image-paddler + video-extract）

> 审阅日期：2026-08-08
> 模块规模：image-paddler 前端 1107 行 + 后端 1587 行；video-extract 前端 722 行 + 后端 578 行

---

## 1. image-paddler 模块

### 1.1 概述

图片排版到 A4 PDF/DOCX，支持 2×3 等网格布局、文件名显示、边框、自然排序分组。面向律师整理证据图片场景。

### 1.2 前端结构

```
src/modules/image-paddler/
├── views/
│   └── ImagePaddlerView.vue   (1076 行 ⚠️ 无 components/composables 拆分)
└── index.js
```

**问题**：所有状态、计算属性、文件名规则引擎、布局算法、拖拽处理均堆在单文件 `<script setup>` 中（257-762 行）。与 pdf-tools（6 components + 14 composables）形成鲜明对比。

**建议拆分**：
- `composables/useFilenameRules.js` — 文件名规则引擎
- `composables/useImageLayout.js` — 布局算法
- `components/PagePreview.vue` — A4 预览
- `components/FilenameRuleEditor.vue` — 文件名规则编辑器

### 1.3 后端核心逻辑

`image_paddler.rs`（1587 行）：

| 功能 | 函数 | 行号 |
|------|------|------|
| 排版 | `parse_layout` / `run_images` | :139-170 / :516-622 |
| PDF 生成 | `generate_pdf` | :913-1082 |
| DOCX 生成 | `generate_docx` | :1126-1287 |
| 排序 | `cell_order` / `reorder_images` | :638-664 / :624-636 |
| 分组 | `build_groups` | :268-284 |
| CJK 字体 | 字体回退加载 | :1084-1119 |

**注意**：本模块**无水印功能**，仅有边框（`border_enabled`/`border_color`）与文件名显示。

### 1.4 问题

**P1 — 单文件过大**：1587 行应拆为 `analyze.rs`/`pdf.rs`/`docx.rs`/`filename.rs`/`layout.rs`/`font.rs`。

**P1 — 参数过多**：`generate_pdf` 与 `generate_docx` 各有 17 个参数（:913-931/:1126-1145），`#[allow(clippy::too_many_arguments)]` 是坏味道。

**建议**：提取 `LayoutContext` 结构体。

**P2 — 颜色映射重复**：`border_rgb`/`docx_border_color`/前端 `borderColorCss`（ImagePaddlerView.vue:598-610）三处重复。

**P2 — 测试覆盖良好**：14 个测试（:1330-1587），覆盖排序、分组、布局、文件名。

### 1.5 UI/UX

左 360px 设置栏 + 右结果栏。

**亮点**：
- 80ms 防抖分析（:639-646）
- `analysisRequestId` 防竞态（:390-393）
- A4 比例预览（:334-339）
- 推荐参数一键应用（:690-699）
- 拖拽多文件夹（:714-729）

**不足**：
- 无分页预览（仅第一页）
- 无生成进度（仅 `:loading`）
- 未使用 `ToolWorkspaceShell`/`FileQueuePanel` 共享组件

---

## 2. video-extract 模块

### 2.1 概述

视频抽帧为图片序列，支持 fps 滤镜、时间戳水印（drawtext）。

### 2.2 前端结构

```
src/modules/video-extract/
├── views/
│   └── VideoExtractView.vue   (691 行 ⚠️ 同样无拆分)
└── index.js
```

### 2.3 后端结构

```
src-tauri/src/ffmpeg/
├── mod.rs       (3 行)
├── probe.rs     (110 行 — ffprobe 封装)
├── detect.rs    (48 行 — drawtext 能力检测)
└── extract.rs   (417 行 — 抽帧核心)
```

后端拆分良好，与 image-paddler 形成对比。

### 2.4 核心逻辑

| 功能 | 函数 | 说明 |
|------|------|------|
| 探测 | `probe_video` (probe.rs:7) | ffprobe，45s 超时 |
| drawtext 检测 | `has_drawtext` (detect.rs:33) | 解析 `ffmpeg -filters` |
| 抽帧 | `extract` (extract.rs:8) | fps filter + 可选 drawtext，5 分钟空闲超时 |
| 时间戳水印 | `drawtext_filter` (extract.rs:229) | `%{pts:hms}` 帧时间戳，4 位置/5 颜色 |
| 文件重命名 | `rename_extracted_frames` (extract.rs:283) | `.docsy_tmp_xxx` → `前缀_HH_MM_SS_mmm_frame_序号` |

### 2.5 问题

**P1 — 无进度反馈**：ffmpeg 输出未解析 `frame=` 进度行，前端仅 spinner。

**P1 — 无取消机制**：5 分钟空闲超时是唯一的终止方式，用户无法主动取消。

**P2 — 死代码**：`detect.rs` 的 `list_system_fonts`（:4-31）未被 extract.rs 使用。

**P2 — drawtext 字体未指定**：`drawtext_filter` 未指定 `fontfile`，跨平台可能回退失败。

**P2 — 双重拖放绑定**：`useWindowFileDrop`（:315-321）与原生 `@drop`（:455-463）双重绑定，逻辑重复。

### 2.6 UI/UX

左 380px + 右结果栏。

**亮点**：
- ffmpeg 状态检测与一键安装（:270-289）
- drop-zone 拖放（:36-56）
- 视频信息卡片（:59-83）
- 水印不可用时 el-alert 警告（:146-153）

**不足**：
- 抽帧时仅 spinner 无百分比
- 结果仅静态网格，无"打开输出目录"按钮（image-paddler 有 `openGeneratedOutput`）
- 未使用 `ToolWorkspaceShell`/`FileQueuePanel` 共享组件

---

## 3. 共性问题

### 3.1 文件选择器未统一

三者均直接 `import { open } from '@tauri-apps/plugin-dialog'` 散落各 View：

| 模块 | 调用位置 | 方式 |
|------|---------|------|
| image-paddler | ImagePaddlerView.vue:380 | `open({ directory: true, multiple: true })` |
| video-extract | VideoExtractView.vue:298 | `selectFile` + `selectOutputDir` |
| pdf-tools | PdfToolsView.vue | 11 处 `open` 调用 |

**建议**：提取 `core/fileDialog.js` 统一目录/文件/多选与路径归一。

### 3.2 进度反馈粗粒度

三者均经 `tauriBridge.js` 的 `tauriCallSafe` → `emitOperationEvent('start'/'finish')` 触发全局 Doclet 动画，**仅 start/finish 两态**，无百分比/帧数进度。

### 3.3 UI 风格与 pdf-tools 不一致

| 模块 | 布局 | 共享组件使用 |
|------|------|-------------|
| image-paddler | 自定义 360px+1fr 双栏 | ❌ 未用 ToolWorkspaceShell |
| video-extract | 自定义 380px+1fr 双栏 | ❌ 未用 ToolWorkspaceShell |
| pdf-tools | ToolWorkspaceShell + el-tabs | ✅ |

三者均用 `var(--docsy-*)` CSS 变量（颜色一致），但**布局结构不同**。两个媒体模块未使用 `ToolWorkspaceShell`/`FileQueuePanel` 等共享组件，导致与 pdf-tools 视觉与交互范式割裂。

**建议**：媒体模块迁移至 `ToolWorkspaceShell`，统一交互范式。

---

## 4. 改进建议

| 优先级 | 模块 | 问题 | 建议 | 工作量 |
|--------|------|------|------|--------|
| **P1** | image-paddler | 单文件 1076 行 | 拆出 composables/components | 1-2 天 |
| **P1** | image-paddler | 后端 17 参数函数 | 提取 LayoutContext | 0.5 天 |
| **P1** | video-extract | 无进度反馈 | 解析 ffmpeg frame= 进度 | 1 天 |
| **P1** | video-extract | 无取消机制 | 接入 OperationManager | 0.5 天 |
| **P2** | 两者 | 文件选择器未统一 | 提取 core/fileDialog.js | 0.5 天 |
| **P2** | 两者 | 未用 ToolWorkspaceShell | 迁移到共享外壳 | 1 天 |
| **P2** | image-paddler | 后端单文件 1587 行 | 拆为 6 个子模块 | 1 天 |
| **P2** | image-paddler | 颜色映射三处重复 | 统一 | 0.3 天 |
| **P2** | video-extract | drawtext 未指定 fontfile | 加字体路径 | 0.3 天 |
| **P2** | video-extract | 双重拖放绑定 | 去重 | 0.2 天 |
| **P3** | image-paddler | 无分页预览 | 支持多页预览 | 0.5 天 |
| **P3** | video-extract | 无打开输出目录按钮 | 补充 | 0.1 天 |
