# UI 一致性与美观性专项审阅

> 审阅日期：2026-08-08
> 审阅范围：全项目 7 个前端模块 + shared/components + styles.css
> 审阅维度：设计系统、共享组件、布局一致性、交互模式、Element Plus 规范、视觉风格、响应式

---

## 1. 设计系统基础

### 1.1 CSS 变量体系 — 基础扎实

全部集中在 `src/styles.css` 的 `:root` 块，约 20 个 `--docsy-*` 变量：

| 类别 | 变量 |
|------|------|
| 背景 | `--docsy-canvas` / `surface` / `surface-elevated` / `surface-muted` / `sidebar` |
| 主色 | `--docsy-primary` / `-hover` / `-soft` |
| 强调 | `--docsy-accent` / `-soft` |
| 语义 | `--docsy-success` / `warning` / `danger` |
| 文本 | `--docsy-text-strong` / `-muted` |
| 边框 | `--docsy-border-subtle` / `-strong` |
| 阴影 | `--docsy-shadow` |

通过 `--el-color-primary: var(--docsy-primary)` 等映射桥接到 Element Plus。**基础设计良好**。

### 1.2 变量使用违规

| 文件 | 行号 | 问题 |
|------|------|------|
| DocumentPreview.vue | :88-89 | 用了不存在的 `--docsy-surface-base`/`--docsy-border-light`，写死 fallback |
| DocumentPreview.vue | :82 | 硬编码 `'SimSun','宋体'` 字体 |
| FilenameTokenInput.vue | :273-278 | 硬编码 `#f3e8ff`、`#6b2fa0` 等十六进制色 |
| TemplateBuildTab.vue | :1182-1230 | preview-token 硬编码 `#d1fae5` 等 |

**建议**：把硬编码色迁入 `--docsy-*` 变量体系。

---

## 2. 共享组件清单

`src/shared/components/` 下 8 个组件：

| 组件 | 职责 | 使用模块 |
|------|------|---------|
| ToolWorkspaceShell | 工具页骨架（header/toolbar/content/actions 四插槽） | pdf-tools、evidence-pdf |
| FileQueuePanel | 文件队列（排序/删除/清空） | pdf-tools、evidence-pdf |
| ImagePreviewGrid | 图片分页预览网格 | ❌ **无人引用，冗余** |
| ReorderableImageGrid | 可拖拽排序图片网格 | image-paddler、video-extract |
| FilenameTokenInput | 文件名 token 编辑器 | template |
| DocumentPreview | Word run 渲染 + overlay | template |
| DocletWorkingPet | 全局工作动画 | App.vue 全局 |
| reorderableItems.js | moveItem 工具函数 | — |

**问题**：`ImagePreviewGrid` 与 `ReorderableImageGrid` 功能高度重叠，前者无人引用。image-paddler 和 video-extract 既不用 `ToolWorkspaceShell` 也不用 `FileQueuePanel`。

**建议**：清理 `ImagePreviewGrid`；媒体模块迁移到 `ToolWorkspaceShell`。

---

## 3. 布局一致性 — 三派分裂

### 3.1 三派布局

| 派系 | 模块 | 布局 | 左栏宽度 |
|------|------|------|---------|
| **ToolWorkspaceShell 派** | pdf-tools、evidence-pdf | `el-tabs tab-position="left"` + `ToolWorkspaceShell` | — |
| **双栏派** | image-paddler | `grid-template-columns: 360px minmax(0,1fr)` | 360px |
| **双栏派** | video-extract | `grid-template-columns: 380px minmax(0,1fr)` | 380px |
| **卡片堆叠派** | settings | `el-card shadow="never"` 分组 | — |
| **卡片堆叠派** | template | 自定义 `.panel`（带 box-shadow） | — |
| **卡片堆叠派** | home | `el-card shadow="hover"` | — |

### 3.2 问题

1. **双栏宽度不一**：image-paddler 360px vs video-extract 380px，无统一约定
2. **卡片阴影不统一**：settings `never`、home `hover`、template 自定义
3. **template 不用 el-card**：用自定义 `.panel`（TemplateBuildTab.vue:876），绕过 Element Plus
4. **媒体模块脱离共享外壳**：image-paddler/video-extract 不用 `ToolWorkspaceShell`

**建议**：
- 统一双栏宽度（建议 360px）
- 统一卡片阴影（建议 `hover`）
- template 改用 `el-card`
- 媒体模块迁移到 `ToolWorkspaceShell`

---

## 4. 交互模式一致性

### 4.1 文件选择 — 未统一

各模块直接 `import { open } from '@tauri-apps/plugin-dialog'`，参数风格各异：
- image-paddler：`open({ directory: true, multiple: true })`
- video-extract：单文件 + filters
- pdf-tools：11 处 `open` 调用

**建议**：提取 `core/fileDialog.js`。

### 4.2 进度反馈 — 三种 spinner 并存

| 实现 | 位置 | 说明 |
|------|------|------|
| 全局 DocletWorkingPet | App.vue | 操作级动画 |
| 中央 el-icon Loading | VideoExtractView.vue:202 | 模块本地 |
| 自定义 .processing-spinner | EvidencePdfWorkbench.vue:23 | 模块本地 |

**建议**：统一到 DocletWorkingPet + 按钮loading，移除本地 spinner。

### 4.3 错误提示 — 基本一致

- `ElMessage`：瞬时反馈，全项目统一
- `el-alert`：持续性警告（split warnings、drawtext、conversion failures），用法一致
- 确认对话框：统一 `ElMessageBox.confirm/prompt/alert`，参数风格一致

### 4.4 拖放 — 双轨问题

pdf-tools、image-paddler、video-extract、EvidencePdfWorkbench 都用 `useWindowFileDrop` composable，但 video-extract 还叠加了局部 `@dragover/@drop` 的 `.drop-zone`（VideoExtractView.vue:37-42），双轨实现。

pdf-tools 用全屏 `.pdf-drop-overlay`，video-extract 用局部 drop-zone，**视觉表现不一**。

**建议**：统一拖放视觉（建议全屏 overlay）。

---

## 5. Element Plus 使用规范

### 5.1 el-button size 不统一

| 模块 | size |
|------|------|
| pdf-tools | 默认 |
| image-paddler | 表单内全 small |
| video-extract | 抽帧按钮 large（:180） |
| settings | small |

**建议**：统一为 `small`（工具型应用标准）。

### 5.2 el-dialog width 单位混用

| 位置 | width |
|------|-------|
| TemplateView:138 | `"520px"` |
| TemplateView:146 | `"min(820px, 94vw)"` |
| ImagePreviewGrid:42 | `"80%"` |

**建议**：统一为 `"min(Npx, 90vw)"` 格式。

### 5.3 el-card shadow 不统一

见 3.1 表格。

---

## 6. 视觉风格

### 6.1 间距

| 模块 | padding |
|------|---------|
| ToolWorkspaceShell | `22px 24px 24px` |
| settings / template | `20px 24px 32px` |
| image-paddler | `20px` |

**建议**：统一为 `20px 24px`。

### 6.2 图标

统一使用 `@element-plus/icons-vue`，一致。

### 6.3 字体

全局 `styles.css:64` 定义系统字体栈，但 `DocumentPreview.vue:82` 单独硬编码 `'SimSun','宋体'`。

---

## 7. 响应式设计

两套断点并存：
- 双栏模块：`@media (max-width: 1180px)` 转单栏
- 卡片模块：`@media (max-width: 760px)`

**评估**：逻辑自洽（双栏需要更宽的断点），但缺少统一约定文档。

---

## 8. UI 改进建议汇总

| 优先级 | 问题 | 建议 | 工作量 |
|--------|------|------|--------|
| **P1** | 三派布局分裂 | 媒体模块迁移到 ToolWorkspaceShell | 2 天 |
| **P1** | 双栏宽度不一 | 统一 360px | 0.2 天 |
| **P1** | 三种 spinner 并存 | 统一到 DocletWorkingPet + button loading | 0.5 天 |
| **P1** | 硬编码颜色 | 迁入 --docsy-* 变量 | 0.5 天 |
| **P2** | el-button size 不统一 | 统一 small | 0.3 天 |
| **P2** | el-card shadow 不统一 | 统一 hover | 0.2 天 |
| **P2** | el-dialog width 单位混用 | 统一 min(Npx, 90vw) | 0.2 天 |
| **P2** | ImagePreviewGrid 冗余 | 删除 | 0.1 天 |
| **P2** | video-extract 拖放双轨 | 去重 | 0.2 天 |
| **P2** | 文件选择器未统一 | 提取 core/fileDialog.js | 0.5 天 |
| **P3** | 间距不完全一致 | 统一 20px 24px | 0.2 天 |
| **P3** | DocumentPreview 硬编码字体 | 移除 | 0.1 天 |
| **P3** | template 不用 el-card | 改用 el-card | 0.3 天 |
