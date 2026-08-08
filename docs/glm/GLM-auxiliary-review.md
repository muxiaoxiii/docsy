# 辅助模块审阅（settings + evidence-pdf + home）

> 审阅日期：2026-08-08
> 模块规模：settings 前端 862 行 + 后端 61 行；evidence-pdf 前端 265 行；home 前端 186 行

---

## 1. settings 模块

### 1.1 概述

应用设置页面，包含外部工具状态、主菜单排序、应用设置、诊断信息四个分组。

### 1.2 前端结构

```
src/modules/settings/
├── views/
│   ├── SettingsView.vue   (676 行 ⚠️ 巨型单文件)
│   └── AboutView.vue      (157 行)
└── index.js
```

无 `components/` 或 `composables/` 子目录，工具检测、菜单排序等逻辑全堆在 `SettingsView.vue` 中。

### 1.3 设置项与持久化

四个 `el-card` 分组：
- **外部工具状态**：qpdf/poppler/ffmpeg/word/wps/libreoffice 六工具
- **主菜单**：`menu_visibility`、`menu_order`
- **应用设置**：`libreoffice_path`、`tool_manifest_url`
- **诊断信息**：日志路径、数据库路径

持久化到 `data_dir/Docsy/settings.json`（`history.rs:24-32`），结构定义在 `history.rs:7-12`。后端 `settings.rs` 仅 61 行，是纯转发层。

### 1.4 工具配置

`tools` reactive 数组（`SettingsView.vue:167-253`）定义六工具，支持：
- 检测（`check_external_tool` 命令）
- 在线安装（GitHub releases + SHA256）
- 本地 zip 安装
- 清除托管

状态机用 `probeState`（pending/checking/available/unavailable/error）管理，设计合理。

### 1.5 问题

**P1 — 无表单验证**：`libreoffice_path` 和 `tool_manifest_url` 是纯文本输入，保存时不校验路径合法性或 URL 格式。

**P1 — 错误反馈不一致**：`loadSettings`（:276）、`loadDiagnostic`（:375）、`loadManagedToolsDir`（:368）在失败时**静默忽略**（无 `ElMessage` 提示），而 `saveSettings` 有提示。

**P2 — 版本号硬编码**：`AboutView.vue:39` 版本号 fallback 硬编码 `'0.9.6'`，易过期。

**P2 — 死 CSS**：`SettingsView.vue:625-635` 的 `.bundle-actions` 和 `.export-options` CSS 类模板中从未使用。

**P3 — 单文件偏大**：676 行应拆出 `useToolDetection.js`、`useMenuSettings.js` 等 composables。

### 1.6 UI/UX

布局清晰，响应式适配（:637-675）。但与 home 的卡片风格不一致：settings 用 `el-card shadow="never"`，home 用 `el-card shadow="hover"`。

---

## 2. evidence-pdf 模块

### 2.1 概述

证据 PDF 处理入口，265 行，本质是 pdf-tools 的薄壳 + 证据扫描独立功能。

### 2.2 前端结构

```
src/modules/evidence-pdf/
├── views/
│   └── EvidencePdfView.vue   (234 行含 CSS)
└── index.js
```

三个 Tab：
- 分项证据处理 → `<EvidencePdfWorkbench workflow="split" />`
- 合并证据处理 → `<EvidencePdfWorkbench workflow="merge" />`
- 证据扫描 → 独立逻辑（文件夹扫描 → Word 转 PDF → 合并）

### 2.3 耦合问题

`EvidencePdfView.vue:68` 跨模块 import `../../pdf-tools/views/EvidencePdfWorkbench.vue`（3635 行大组件）。

**关键发现**：`EvidencePdfWorkbench` **仅被 evidence-pdf 使用**，pdf-tools 自身的 `PdfToolsView.vue` 从不引用它。这是明显的架构错位——组件放在了错误的模块里。

### 2.4 建议

**不应合并整个模块**（证据扫描是独立功能），但应将 `EvidencePdfWorkbench.vue` 从 `pdf-tools/views/` 迁移到 `evidence-pdf/views/` 或 `evidence-pdf/components/`，消除跨模块依赖。

### 2.5 其他问题

**P2 — 死 CSS**：`EvidencePdfView.vue:188-199` 的 `.group-files` 和 `.group-file` CSS 类模板中从未使用（实际用 `FileQueuePanel` 组件渲染），是重构残留。

---

## 3. home 模块

### 3.1 概述

应用首页，hero 区 + 功能卡片网格。

### 3.2 前端结构

```
src/modules/home/
├── views/
│   └── HomeView.vue   (186 行)
└── index.js
```

### 3.3 homeCards 机制

`HomeView.vue:44` 通过 `getHomeCards(settings)` 聚合。`moduleRegistry.js:35-45` 遍历所有模块，过滤可见性后 flatMap 各模块的 `homeCards` 数组，注入 `icon` 和 `moduleId`。卡片点击跳转 `router.push({ name: card.route })`。

### 3.4 homeCard 定义

各模块在 `index.js` 中定义，格式统一（`title`/`description`/`route`/`icon`）：

| 模块 | 卡片 | icon |
|------|------|------|
| evidence-pdf | 证据处理 | — |
| pdf-tools | PDF 工具 | — |
| image-paddler | 图片排版 | — |
| video-extract | 视频抽帧 | — |
| template | 模板系统 | — |
| settings | （无） | — |
| home | （无） | — |

格式一致性好。settings 和 home 自身 `homeCards: []`。

### 3.5 问题

**P3 — 缺 order 字段**：`home/index.js` 缺 `order` 字段（其他模块都有），但首页无需排序，影响不大。

**P3 — 卡片交互风格**：home 用 `el-card shadow="hover"`，与 settings 的 `shadow="never"` 不一致。

---

## 4. 共性问题

### 4.1 UI 风格

三模块均使用 `--docsy-*` CSS 变量体系，风格基本一致。但：
- settings 用 `el-card shadow="never"`
- home 用 `el-card shadow="hover"`
- evidence-pdf 用 `ToolWorkspaceShell` 封装

卡片交互风格不完全统一。

### 4.2 死代码

| 模块 | 位置 | 内容 |
|------|------|------|
| settings | SettingsView.vue:625-635 | `.bundle-actions` / `.export-options` CSS |
| evidence-pdf | EvidencePdfView.vue:188-199 | `.group-files` / `.group-file` CSS |

均为重构残留，应清理。

### 4.3 错误处理

统一使用 `tauriCallSafe` 返回 `{ok, data/error}` 模式。但 settings 模块内**读取类函数静默失败、写入类函数才提示**，不一致。`EvidencePdfView` 和 `HomeView` 的错误处理较统一（失败时 `ElMessage.error`）。

---

## 5. 改进建议

| 优先级 | 模块 | 问题 | 建议 | 工作量 |
|--------|------|------|------|--------|
| **P1** | evidence-pdf | EvidencePdfWorkbench 放错模块 | 迁移到 evidence-pdf/ | 0.5 天 |
| **P1** | settings | 无表单验证 | 增加路径/URL 校验 | 0.3 天 |
| **P1** | settings | 读取静默失败 | 统一错误提示 | 0.2 天 |
| **P2** | settings | 版本号硬编码 | 从 package.json 或后端获取 | 0.2 天 |
| **P2** | 两者 | 死 CSS | 清理 | 0.1 天 |
| **P2** | settings | 单文件偏大 | 拆出 composables | 1 天 |
| **P3** | home | 缺 order 字段 | 补充 | 0.05 天 |
| **P3** | 两者 | 卡片 shadow 不一致 | 统一 | 0.1 天 |
