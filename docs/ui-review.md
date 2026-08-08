# Docsy UI/UX 审查报告

> 审查日期：2026-08-07  
> 审查范围：`src/` 目录下全部 Vue 组件、视图、共享组件及全局样式  
> 技术栈：Tauri 2 + Vue 3 (Composition API) + Element Plus + CSS Custom Properties

---

## 目录

1. [设计系统总览](#1-设计系统总览)
2. [应用壳层 (App.vue)](#2-应用壳层)
3. [模板模块 (Template)](#3-模板模块)
4. [PDF 工具模块 (PDF Tools)](#4-pdf-工具模块)
5. [证据 PDF 模块 (Evidence PDF)](#5-证据-pdf-模块)
6. [视频抽帧模块 (Video Extract)](#6-视频抽帧模块)
7. [图片排版模块 (Image Paddler)](#7-图片排版模块)
8. [设置模块 (Settings)](#8-设置模块)
9. [关于页面 (About)](#9-关于页面)
10. [共享组件](#10-共享组件)
11. [全局可用性问题与改进方向](#11-全局可用性问题与改进方向)
12. [优先级改进路线图](#12-优先级改进路线图)

---

## 1. 设计系统总览

### CSS 变量体系 (`styles.css`)

Docsy 使用一套完整的暖色调 CSS 自定义属性体系，覆盖背景、文字、边框、语义色等：

| 变量 | 用途 | 色值 |
|------|------|------|
| `--docsy-canvas` | 主背景 | `#f6efe6` |
| `--docsy-surface` | 面板背景 | `#fcf5ea` |
| `--docsy-surface-elevated` | 卡片/弹窗背景 | `#fff9f0` |
| `--docsy-surface-muted` | 次级背景 | `#f3eadf` |
| `--docsy-sidebar` | 侧边栏背景 | `#f1e8dc` |
| `--docsy-primary` | 主色 | `#3f7167` |
| `--docsy-accent` | 强调色 | `#c77847` |
| `--docsy-text-strong` | 主文字 | `#26332f` |
| `--docsy-text` | 正文文字 | `#46514d` |
| `--docsy-text-muted` | 次要文字 | `#7a817d` |
| `--docsy-danger` | 危险/删除 | `#b5524b` |

**优点**：
- 全局变量覆盖了 Element Plus 的 `--el-*` 变量，确保第三方组件与自定义样式一致
- 暖色调设计有辨识度，区别于常见的冷色系工具软件

**问题**：
- 未定义暗色模式变量，所有色彩硬编码为浅色
- 部分组件使用了未定义的变量名（如 `--docsy-sidebar-hover`、`--docsy-border-strong` 已定义，但某些组件直接用硬编码 `rgba()` 值）
- `--docsy-shadow` 全局定义但使用不一致，部分组件用 `rgba()` 硬编码

---

## 2. 应用壳层 (App.vue)

### 页面功能
提供全局布局框架：左侧边栏导航 + 顶部标题栏 + 主内容区 + 底部浮动操作面板（Doclet 宠物动画）。

### 用户交互流程
1. 用户通过左侧 `el-menu` 切换模块路由
2. 点击品牌区域回到首页
3. 底部有「关于」和「设置」图标按钮
4. 长时间操作时右下角显示 Doclet 动画 + 取消按钮

### 组件层级结构
```
App.vue
├── el-container (app-container)
│   ├── el-aside (220px 侧边栏)
│   │   ├── .brand (Logo + 名称，点击回首页)
│   │   ├── el-menu (动态菜单项)
│   │   └── .sidebar-footer (关于 + 设置)
│   └── el-container
│       ├── el-header (页面标题)
│       ├── el-main → <router-view />
│       └── Transition → DocletWorkingPet (浮动操作面板)
```

### 样式/布局特征
- 固定 `100vh` 布局，`overflow: hidden`
- 侧边栏固定 220px 宽度
- 菜单项圆角 6px，高 40px，有 hover/active 状态
- Doclet 面板使用 `position: fixed`，右下角浮动
- 操作面板显示延迟 350ms（防止闪烁），取消按钮 30s 后才出现

### 可用性问题

1. **键盘导航缺失**：菜单项使用 `el-menu` 的 `@select`，但没有可见的 focus ring 或键盘快捷键提示
2. **品牌区域歧义**：`.brand` 有 `cursor: pointer`，暗示可点击，但没有 tooltip 说明功能（"返回首页"）
3. **操作取消反馈**：取消操作后 `pendingOperations.clear()` 直接清空所有操作，不会告知用户哪些操作被取消了
4. **窗口事件堆积**：`onMounted` 注册了 3 个 `window.addEventListener`，如果用户在页面间频繁切换但组件未正确卸载，可能导致事件重复注册
5. **响应式缺失**：没有响应式布局，220px 侧边栏在小屏幕会挤压内容区
6. **页面标题重复**：顶部 header 只显示页面标题文字，无其他操作，占据了 60px 的垂直空间

---

## 3. 模板模块 (Template)

### 3.1 TemplateView.vue

#### 页面功能
模板管理的核心视图，包含四个标签页：
- **制作模板**：从 Word 标黄导入 → 定义字段类型 → 保存模板
- **填写模板**：从模板库选择 → 填写表单 → 生成 Word
- **填写历史**：查看历史记录 → 快速填入
- **设置**：分隔符、回收站、数据库管理、导入导出

#### 用户交互流程
```
选择 Word → 扫描标黄 → 确认字段(类型/名称/关系) → 保存模板
                                                      ↓
选择模板 → 填写字段(自动完成/引用/历史建议) → 生成 Word
                                                      ↓
                                              保存到填写历史
```

#### 组件层级结构
```
TemplateView.vue (2900 行，核心控制器)
├── el-tabs (4 tabs)
│   ├── TemplateBuildTab (1464 行)
│   ├── TemplateRenderTab (1145 行)
│   ├── TemplateHistoryTab (218 行)
│   └── TemplateSettingsTab (344 行)
├── el-dialog (拆分标黄片段)
└── el-dialog (批量保存记录)
```

#### 样式/布局特征
- 使用 Element Plus 的顶部 tabs 切换
- 所有子标签页共用同一个数据层（TemplateView 是状态所有者）
- 大量 `v-model` 双向绑定从父传子（30+ 个 prop 传递到 TemplateBuildTab）

#### 可用性问题

1. **God Component 反模式**：TemplateView.vue 长达 2900 行，包含所有业务逻辑。虽然已拆分为子组件，但子组件只是「展示层」，所有事件处理仍在父组件。这导致：
   - 代码难以维护
   - 组件间数据流不透明
   - Vue DevTools 中事件链难以追踪

2. **Props 爆炸**：TemplateBuildTab 接收 20+ 个 props + 20+ 个 emits，TemplateRenderTab 类似。这是典型的 prop drilling 问题，应使用 provide/inject 或 Pinia store。

3. **无初始状态引导**：新用户打开模板模块，看到 4 个标签页但不知道从哪开始。没有入门引导或空状态提示。

4. **撤销机制脆弱**：`undoStack` 是简单的数组快照，每次 pushUndoSnapshot 会 `JSON.parse(JSON.stringify(...))` 整个 fieldRows，性能风险高。

5. **标签页切换无确认**：在「制作模板」标签有未保存修改时，切换到其他标签不会提示保存。

### 3.2 TemplateBuildTab.vue

#### 页面功能
制作模板的工作区：导入 Word、字段列表编辑、字段设置弹窗。

#### 用户交互流程
1. 点击「选择 Word」→ 导入并扫描标黄
2. 在表格中确认每个字段的类型、名称、必填等
3. 多选后可合并为字段、设为前缀/后缀
4. 点击「⋯」打开字段详细设置
5. 保存模板

#### 样式/布局特征
- 面板式布局（`.panel` + `.panel-header`）
- 字段表格使用 `el-table`，列包括：拆分、标黄文字、类型、字段、必填、关系、设置
- 字段设置使用 `el-popover`（click 触发），宽度 420px
- 选择工具栏在表格上方，有字段名/显示名输入 + 类型选择 + 操作按钮

#### 可用性问题

1. **表格列过窄**：在 1280px 屏幕上，8 列表格的「标黄文字」和「字段」列会被挤压到极窄，依赖 `show-overflow-tooltip` 但用户需 hover 才能看到完整内容
2. **字段设置 popover 位置**：使用 `placement="left-start"`，当表格在屏幕右侧时可能溢出
3. **隐式操作**：字段名输入时会自动检测重复并转为引用（`onFieldNameInput`），有 `ElMessage.success` 提示但用户可能没注意到
4. **参考建议提醒**：使用 `el-badge` 的 `value="!"` 提示，但 badge 位于 `⋯` 按钮上，用户可能不知道需要点击查看
5. **无拖拽排序**：字段顺序由 Word 文档中高亮出现的顺序决定，不能手动重排

### 3.3 TemplateRenderTab.vue

#### 页面功能
模板填写工作区：模板库网格、字段表单、填写预览。

#### 用户交互流程
1. 从模板库选择模板（卡片网格）
2. 逐字段填写（自动完成、历史建议、引用选择）
3. 可预览生成效果
4. 点击「生成 Word」

#### 样式/布局特征
- 模板库使用 CSS Grid 卡片布局
- 字段表单使用卡片式分组（`.fill-field-card`）
- 字段类型自动切换不同的输入控件（日期、单选、多选、列表、引用等）
- 历史建议以 `el-tag` 显示在字段下方

#### 可用性问题

1. **模板库卡片信息密度过低**：每个卡片只显示名称、字段数、更新时间，缺少模板预览或描述
2. **字段卡片缺乏视觉分层**：所有字段卡片外观相同，没有区分可编辑 / 只读 / 引用 / 重复
3. **批量填写入口隐蔽**：批量功能藏在 dropdown 里，用户可能不知道可以批量
4. **填充预览位置**：预览面板在字段列表下方，长表单时需要滚动很远才能看到
5. **字段搜索无高亮匹配**：搜索框过滤字段但不高亮匹配文字

### 3.4 TemplateHistoryTab.vue

#### 页面功能
查看每次生成文书的完整表单记录，可快速填入。

#### 可用性问题
- **无搜索/过滤**：历史记录按模板分组，但无法按日期或关键词搜索
- **无删除单条记录**：只能在设置页清理全部历史
- **分页使用简单的展开/收起**：每组默认显示 20 条，没有真正的分页控件

### 3.5 TemplateSettingsTab.vue

#### 页面功能
模块设置（分隔符）、模板导入/导出、回收站、数据库管理。

#### 可用性问题
- **回收站和数据库的区别不明确**：用户可能混淆「模板回收站」和「模板数据库」
- **合并对话框无预览**：合并操作前无法预览将合并哪些字段数据

---

## 4. PDF 工具模块 (PDF Tools)

### PdfToolsView.vue

#### 页面功能
PDF 工具集，包含 6 个子工具：
- 解锁（移除密码）
- 合并（多 PDF 合一）
- 提取页面
- 压缩
- 拆分（带页面预览）
- 防复制（CMap 篡改/移除）

#### 用户交互流程
```
选择工具标签 → 选择/拖入 PDF 文件 → 配置参数 → 执行操作 → 查看结果
```

#### 组件层级结构
```
PdfToolsView.vue (1154 行)
├── el-tabs (tab-position="left", 6 tabs)
│   ├── 解锁 → ToolWorkspaceShell + FileQueuePanel
│   ├── 合并 → ToolWorkspaceShell + FileQueuePanel (sortable)
│   ├── 提取页面 → ToolWorkspaceShell + el-input
│   ├── 压缩 → ToolWorkspaceShell
│   ├── 拆分 → ToolWorkspaceShell + PdfJsPreview + el-table
│   └── 防复制 → ToolWorkspaceShell + FileQueuePanel
```

#### 样式/布局特征
- 左侧标签导航（`tab-position="left"`），宽度 124px（全局 CSS 控制）
- 每个工具使用 `ToolWorkspaceShell` 统一框架（标题 + 描述 + 工具栏 + 内容 + 操作按钮）
- 拆分工具是左右分栏：左侧表格 + 右侧 PDF 预览
- 支持文件拖放（`useWindowFileDrop` composable）

#### 可用性问题

1. **拆分页面复杂度过高**：拆分工具有页段表格、页面预览、页眉页脚清理选项、拖拽重排等，所有内容在一个标签页内，信息密度过大
2. **PDF 预览尺寸固定**：`PdfJsPreview` 的 `page-preview` 固定 `max-width: 620px`，在高分屏或大窗口上利用率低
3. **文件队列状态不持久**：切换标签页后，之前添加的文件仍在（`lazy` 加载），但没有「清空所有」的快捷方式
4. **合并操作无进度**：合并多个大文件时只有 `merging` loading 状态，没有进度百分比
5. **防复制方法选择器位置**：方法选择（CMap 篡改/移除）放在 actions 区域，用户可能在执行前没注意到
6. **提取页面的路径显示**：`extractFile` 和 `extractOutputDir` 用 `div.path-line` 显示，缺少标签说明

---

## 5. 证据 PDF 模块 (Evidence PDF)

### EvidencePdfView.vue

#### 页面功能
证据处理的入口视图，三个标签：
- 分项证据处理（多个 PDF → 叠加页眉页脚 → 合并）
- 合并证据处理（一个大 PDF → 拆分 → 分别处理）
- 证据扫描（扫描文件夹自动整理）

#### 组件层级结构
```
EvidencePdfView.vue (234 行)
├── el-tabs (tab-position="left", 3 tabs)
│   ├── 分项 → EvidencePdfWorkbench (workflow="merge")
│   ├── 合并 → EvidencePdfWorkbench (workflow="split")
│   └── 扫描 → ToolWorkspaceShell + FileQueuePanel
```

#### 可用性问题
- **标签命名不直观**：「分项证据处理」和「合并证据处理」对新用户来说含义不清
- **扫描功能独立**：证据扫描是单独的标签页，与前两个工作台没有交互

### EvidencePdfWorkbench.vue

#### 页面功能
证据 PDF 处理的核心工作台，3634 行。功能极其丰富：
- 文件导入（单文件/合并 PDF）
- 页眉页脚检测与确认
- A4 规范化
- 新页眉/页脚/页码插入（支持多组规则）
- PDF 书签
- 输出模式选择（单文件 / 合并 / 两者）
- 合并 PDF 的页段拆分
- 原有元素确认与编辑

#### 用户交互流程
```
导入 PDF → 检测原有页眉页脚 → 确认/删除/编辑原有元素
    → 配置新页眉页脚页码 → 配置输出选项 → 生成
```

#### 样式/布局特征
- 单列滚动布局（非分栏）
- 使用 `.rule-block` 分组配置项
- `.rule-grid` 网格布局放置表单项
- 文件表格使用 `el-table` 带展开行（显示文件内检测到的元素）
- 大量内联 `.local-processing` 状态区域

#### 可用性问题

1. **单文件超长**：3634 行，是整个项目最复杂的单个组件
2. **状态区域碎片化**：至少 4 种不同的 `local-processing` 状态（分析合并 PDF、拆分、处理证据、检测页眉页脚），分散在模板各处
3. **配置项过多**：从页眉文字到字体大小、边距、偏移、颜色，以及多组规则，用户容易迷失
4. **原元素确认流程复杂**：有「一键确认」「一键清除」「删除现有」「恢复删除标记」四个按钮，加上 summary pill 点击跳转到对话框，交互路径长
5. **输出模式切换无即时预览**：切换输出模式（单文件/合并/两者）后，下方的输出路径会变化但没有明显的视觉反馈
6. **规则字段的 v-model 绑定链过长**：HeaderFooterRuleFields 接收 30+ 个 v-model props

---

## 6. 视频抽帧模块 (Video Extract)

### VideoExtractView.vue

#### 页面功能
视频帧提取工具：选择视频 → 配置抽帧参数 → 执行 → 查看结果网格。

#### 用户交互流程
```
检查 FFmpeg → 选择/拖入视频 → 查看视频信息 → 配置抽帧参数
    → 开始抽帧 → 查看结果图片网格 → 可拖拽重排
```

#### 组件层级结构
```
VideoExtractView.vue (691 行)
├── .extract-layout (左右分栏)
│   ├── .extract-settings (左 380px)
│   │   ├── FFmpeg 状态
│   │   ├── 文件选择（拖放区域）
│   │   ├── 视频信息
│   │   ├── 抽帧设置（el-form）
│   │   └── 开始按钮
│   └── .extract-results (右侧)
│       ├── 结果标题
│       ├── ReorderableImageGrid
│       └── el-empty
```

#### 样式/布局特征
- 左右分栏布局：左侧 380px 设置面板，右侧自适应结果区
- 使用 `section-block` 分段，每段有标题 + 内容 + 底部分割线
- 拖放区域有 hover/active 状态反馈
- 响应式断点 1180px：分栏变单列

#### 可用性问题

1. **FFmpeg 安装状态混在配置面板中**：FFmpeg 检测/安装占据了设置面板的顶部空间，对已安装用户是浪费
2. **抽帧进度缺失**：`extracting` 只有 boolean 状态，大视频抽帧可能耗时很长，没有进度条或预估时间
3. **时间范围输入格式自由**：`startTime`/`endTime` 接受 "01:30:00" 或秒数，但 placeholder 只提示了一种格式
4. **质量滑块意义不明**：`quality` 滑块 1-100，但没有说明是 JPEG 质量还是其他指标
5. **结果图片无批量操作**：只能拖拽重排，不能批量删除或选择导出
6. **清除视频后结果不清除**：`clearVideo()` 会清空 `resultImages`，但用户可能期望保留之前的结果

---

## 7. 图片排版模块 (Image Paddler)

### ImagePaddlerView.vue

#### 页面功能
将图片批量排版为 A4 文档（PDF/DOCX），支持多种布局、自定义文件名规则、实时预览。

#### 用户交互流程
```
选择文件夹(可多选) → 自动分析 → 调整排版参数 → 实时预览 → 生成文档
```

#### 组件层级结构
```
ImagePaddlerView.vue (1076 行)
├── .paddler-layout (左右分栏)
│   ├── .settings-panel (左 360px)
│   │   ├── 标题
│   │   ├── el-form (文件夹/格式/布局/缩放/方向/边距/文件名/排列/边框)
│   │   └── 生成按钮
│   └── .result-panel (右侧)
│       ├── 生成结果
│       ├── 分析摘要 (el-descriptions)
│       ├── 推荐参数栏
│       ├── 第一页预览 (page-preview)
│       ├── 文件分组
│       └── 图片列表 (ReorderableImageGrid)
```

#### 样式/布局特征
- 左右分栏：左侧 360px 设置面板，右侧自适应结果/预览
- 预览使用真实 A4 比例缩放（`aspectRatio: 210/297`）
- 支持缩放预览（50%-180%）
- 文件名规则使用动态网格布局

#### 可用性问题

1. **设置面板信息密度过高**：16 个表单项挤在 360px 宽度内，滚动频繁
2. **文件名规则编辑器复杂**：5 种规则类型（删除/替换/前缀/后缀/保留成分），每种有不同的输入组合，`filename-rule-keep` 的 grid 有 7 列
3. **预览性能**：`preloadVisibleImages` 对当前页所有图片调用 `read_image_data_url`，大量高分辨率图片可能卡顿
4. **分析结果更新不即时**：`scheduleAnalyze` 有 80ms 防抖，但用户调整布局参数后不会自动重新分析
5. **推荐参数栏信息不足**：只显示 `analysis.recommended.reason` 文字，没有解释为什么推荐这个布局
6. **多文件夹模式**：支持选择多个文件夹，但 UI 上只用 `folders.join('；')` 显示路径，长路径会很难看

---

## 8. 设置模块 (Settings)

### SettingsView.vue

#### 页面功能
应用设置页面：外部工具状态、主菜单排序、应用设置、诊断信息。

#### 组件层级结构
```
SettingsView.vue (676 行)
├── h2 "设置"
├── el-card (外部工具状态)
│   └── .tool-list → .tool-item × 6
├── el-card (主菜单)
│   └── .menu-order-list → .menu-order-item × N
├── el-card (应用设置)
│   └── el-form (LibreOffice 路径, 工具清单地址)
└── el-card (诊断信息)
    └── el-descriptions + diag-actions
```

#### 样式/布局特征
- 单列卡片布局，`max-width: 1040px`，居中
- 响应式断点 760px：卡片头部和工具项改为纵向排列
- 工具项使用 `tool-item` 分组，每个有状态标签 + 详情 + 操作按钮

#### 可用性问题

1. **工具状态一次性全部检测**：`onMounted` 时 `checkTools()` 对 6 个工具并行检测，可能导致多个加载状态同时出现
2. **工具操作入口过多**：每个工具有「检测此工具」「下载安装到 Docsy」「本地 zip 安装」「下载页」「VC++ 运行库」等多个按钮，没有分层组织
3. **菜单排序无拖拽**：只有「上移」「下移」按钮，没有拖拽排序
4. **保存设置无自动保存**：修改后需要手动点击「保存设置」，没有 dirty 状态提示
5. **诊断信息可读性差**：版本号、系统信息等直接文字展示，没有格式化或可复制按钮
6. **LibreOffice 路径输入无验证**：输入后没有路径有效性检测

---

## 9. 关于页面 (About)

### AboutView.vue

#### 页面功能
品牌展示页：Logo、版本号、GitHub 链接、小红书二维码、版权信息。

#### 样式/布局特征
- 居中单列布局，max-width 400px
- 柔和的卡片式设计（无边框，靠间距分隔）
- GitHub 链接使用自定义按钮样式（非 Element Plus 组件）

#### 可用性问题
1. **版本号硬编码回退**：`import.meta.env.PACKAGE_VERSION || '0.9.6'`，如果构建时未注入版本号会显示错误版本
2. **GitHub 链接使用 `@tauri-apps/plugin-shell`**：而不是其他视图中使用的 `openExternalUrl`，不一致
3. **二维码无替代文本方案**：alt 文本为"小红书二维码"，对纯文本用户无帮助

---

## 10. 共享组件

### 10.1 DocletWorkingPet.vue

#### 页面功能
操作进行中的动画宠物，使用 spritesheet 帧动画。

#### 特征
- 正确使用 `role="status"` 和 `aria-live="polite"` 提升无障碍性
- `prefers-reduced-motion` 媒体查询禁用动画
- 固定 156px 高度，可能在小屏幕上占据过多空间

#### 可用性问题
- 消息区域 `max-width: 180px`，长消息会被截断
- 没有进度条或百分比，只有已用时显示

### 10.2 ReorderableImageGrid.vue

#### 页面功能
可重排的图片网格组件，支持分页、缩放、预览。

#### 特征
- 分页加载（24/48/96 张每页）
- 拖拽手柄 + 上下移动按钮双重排序方式
- 使用 `usePointerReorder` composable 处理拖拽
- 懒加载图片（3 个并发 worker）

#### 可用性问题
1. **预览对话框无键盘关闭**：`el-dialog` 虽然有 ESC 关闭，但无快捷键打开大图
2. **拖拽反馈不明显**：`is-reorder-dragging` 只改变透明度，`is-reorder-before/after` 只有 3px 边框阴影
3. **分页状态不持久**：切换组件重新挂载后，页码重置为 1
4. **图片预加载并发限制**：固定 3 个 worker，大量图片时可能感觉慢

### 10.3 FileQueuePanel.vue

#### 页面功能
文件队列列表组件，支持拖拽排序、删除、清空。

#### 特征
- 支持 sortable/removable/clearable 配置
- 使用 `usePointerReorder` 拖拽
- 通过 slot 支持自定义 meta 和 actions

#### 可用性问题
1. **无批量选择**：只能逐个删除，没有多选删除
2. **无文件图标**：所有文件看起来一样，没有文件类型图标区分
3. **空状态文案单一**：只有一个 `emptyText` prop，无法自定义空状态的图标或操作按钮

### 10.4 ToolWorkspaceShell.vue

#### 页面功能
工具页面的标准布局壳层（标题 + 描述 + 工具栏 + 内容 + 操作按钮）。

#### 特征
- 使用 flexbox 纵向布局
- `min-height: 0` 确保 flex 子项正确收缩
- 响应式断点 760px：header 改为纵向排列
- padding 22px 24px 24px

#### 可用性问题
- **无面包屑导航**：用户在左侧标签切换后，可能不知道自己在哪个工具
- **header-actions 插槽使用率低**：只有少数视图使用了 `header-actions` 插槽

### 10.5 UndoRedoButtons.vue

#### 页面功能
撤销/重做按钮对，支持 compact 模式。

#### 可用性问题
- **尺寸逻辑冗余**：`:size="compact ? 'small' : 'small'"` 两种情况都是 'small'，compact prop 实际上只影响 `margin-left` 和图标大小
- **无键盘快捷键处理**：tooltip 显示了 `Ctrl+Z` / `Ctrl+Shift+Z`，但组件本身不处理键盘事件（由使用者负责）

### 10.6 TextPlacementFields.vue

#### 页面功能
页眉/页脚/页码的通用排版设置字段（位置、字号、字体、边距、偏移、颜色）。

#### 可用性问题
- **HTML 结构问题**：模板第一行 `>` 在 `<div>` 后有悬空的 `>` 字符（第 4 行），虽不影响渲染但不符合语义
- **颜色选择器无预设**：`el-color-picker` 默认全光谱，但 PDF 处理通常只需要黑色/深灰
- **字体选择无预览**：8 种字体用下拉列表展示，没有字体名预览效果

### 10.7 HeaderFooterRuleFields.vue

#### 页面功能
页眉、页脚文字和页码的完整配置组件，支持多组规则、每组独立设置。

#### 可用性问题
1. **Props 数量爆炸**：接收 40+ 个 props，是整个项目最"重"的 prop 接口
2. **多组切换 UI 不明显**：当有多个组时，使用简单的列表（`.header-group-item`），没有标签页或更明显的视觉分组
3. **模板标记帮助文本重复**：页眉和页脚的模板标记帮助完全相同，在 tooltip 中重复定义
4. **页码预览位置**：预览在配置项列表中间，用户可能不知道滚动到哪里看

### 10.8 ExistingPdfElementsDialog.vue

#### 页面功能
确认原有 PDF 页眉页脚元素的对话框，支持批量选择/决策/编辑。

#### 特征
- 支持 Shift+click 范围选择
- 支持文件过滤、排序
- 低置信度行有特殊样式标识
- 决策类型：保留/忽略/标记删除/标记编辑

#### 可用性问题
1. **决策标签不一致**：dialog 标题是"确认原有页眉、页脚和页码"，但操作有 4 种（保留/忽略/删除/编辑），「忽略识别」和「保留」的区别不直观
2. **同序列选择逻辑复杂**：`selectBySequence` 对页码和页眉页脚有不同的选择逻辑，用户可能不知道这个功能
3. **预览会关闭对话框**：`previewRow` 先关闭对话框再 emit preview 事件，返回后需要重新打开

### 10.9 PageNumberRuleDialog.vue

#### 页面功能
页码分段与例外规则编辑对话框。

#### 可用性问题
- **列过多**：8 列（范围依据/起始页/结束页/处理/样式/格式/位置/编号偏移）在窄屏幕上挤压
- **规则优先级不直观**：提示"后面的规则覆盖前面的规则"，但没有拖拽排序或优先级数字

---

## 11. 全局可用性问题与改进方向

### 11.1 无障碍性 (Accessibility)

| 问题 | 影响 | 建议 |
|------|------|------|
| 大量使用原生 `<button>` 和 `<div>` 而非语义化 HTML | 屏幕阅读器无法正确导航 | 为交互元素添加 `role` 和 `aria-*` 属性 |
| 颜色对比度不足 | `--docsy-text-muted` (#7a817d) 在 `--docsy-surface-muted` (#f3eadf) 上的对比度约 3.2:1，低于 WCAG AA 的 4.5:1 | 调暗文字颜色或调亮背景 |
| 只有 DocletWorkingPet 使用了 `aria-live` | 其他操作反馈（如 `ElMessage`）对屏幕阅读器不可见 | 为关键操作结果添加 `aria-live` 区域 |
| 键盘导航不完整 | 表格行、图片卡片等自定义交互元素无键盘焦点管理 | 添加 `tabindex` 和 `keydown` 处理 |
| `prefers-reduced-motion` 只在 DocletWorkingPet 中使用 | 其他动画（transition、loading spinner）不受约束 | 全局添加 `prefers-reduced-motion` 媒体查询 |

### 11.2 一致性问题

| 问题 | 位置 | 建议 |
|------|------|------|
| 文件打开方式不一致 | AboutView 用 `@tauri-apps/plugin-shell` 的 `open`，其他用 `tauriBridge` 的 `openExternalUrl` | 统一使用 `openExternalUrl` |
| 空状态展示不一致 | 有的用 `el-empty`，有的用 `div.queue-empty`，有的用 `p` 文字 | 统一使用 `el-empty` 或自定义空状态组件 |
| 加载状态不一致 | 有的用 `el-button :loading`，有的用 `local-processing` 自定义区域，有的用 `el-icon.is-loading` | 定义统一的 loading 状态组件 |
| 错误展示不一致 | 有的用 `ElMessage.error`，有的用 `el-alert`，有的用 `preview-error` 自定义 | 区分 toast（瞬时错误）和 inline（持久错误）两种模式 |
| 面板样式不一致 | Template 子组件用 `.panel` + `.panel-header`，PDF 工具用 `ToolWorkspaceShell` | 统一面板容器组件 |
| 路径显示不一致 | 有的用 `div.path-line`，有的用 `div.path-text`，有的用 `span.path-label` | 统一路径显示组件 |

### 11.3 缺失的反馈/加载状态

| 场景 | 当前状态 | 建议 |
|------|----------|------|
| 模板扫描 | 只有 `scanning` boolean | 显示已扫描片段数 |
| PDF 合并 | 只有 `merging` boolean | 显示文件进度 (2/5) |
| 视频抽帧 | 只有 `extracting` boolean | 显示帧数进度或百分比 |
| 图片分析 | 只有 `analyzing` boolean + "正在分析..." 文字 | 使用进度条 |
| 大文件导入 | 转换超时有对话框，但加载中无反馈 | 添加骨架屏或进度提示 |
| 设置保存 | 成功用 `ElMessage.success`，但无 dirty 状态 | 添加未保存提示 |

### 11.4 错误 UI 缺口

| 场景 | 当前处理 | 建议 |
|------|----------|------|
| 网络错误（工具下载） | `ElMessage.error` | 添加重试按钮和离线提示 |
| 文件损坏 | `ElMessage.error` | 提供更详细的错误描述和修复建议 |
| 磁盘空间不足 | 后端报错，前端无特殊处理 | 在执行前检查磁盘空间 |
| 并发操作冲突 | 无处理 | 添加操作锁或队列提示 |
| 部分失败（批量操作） | 简单统计成功/失败数 | 显示失败文件列表和重试选项 |

### 11.5 桌面应用 UX 最佳实践差距

| 实践 | 当前状态 | 建议 |
|------|----------|------|
| **全局搜索/命令面板** | 无 | 添加 Cmd+K 快速搜索模板、历史、功能 |
| **快捷键体系** | 仅撤销/重做有提示 | 定义全局快捷键（保存、新建、切换标签等） |
| **拖放支持** | 部分视图支持 | 扩展到所有文件输入场景 |
| **窗口状态记忆** | 无 | 记住窗口大小、上次打开的标签页 |
| **自动保存** | 无 | 模板草稿自动保存 |
| **多窗口/标签** | 无 | 支持多个文档同时编辑 |
| **通知系统** | 只有 ElMessage toast | 长任务完成后发送系统通知 |
| **右键菜单** | 无 | 表格行、图片卡片添加上下文菜单 |
| **深色模式** | 无 | 定义暗色变量方案 |

---

## 12. 优先级改进路线图

### P0 — 核心体验修复

1. **模板模块状态管理重构**：将 TemplateView 的 2900 行逻辑拆分到 Pinia store + composables，降低子组件 prop 数量
2. **EvidencePdfWorkbench 拆分**：将 3634 行拆分为多个子组件（文件导入区、原有元素区、新规则区、输出区）
3. **全局错误边界**：添加 Vue `onErrorCaptured` 全局错误处理，防止白屏
4. **关键操作进度反馈**：PDF 合并、视频抽帧等长时间操作添加真实进度

### P1 — 一致性提升

5. **统一路径显示组件**：创建 `FilePathDisplay.vue`，统一路径截断、复制、打开行为
6. **统一空状态组件**：扩展现有 `el-empty` 用法，或创建增强版空状态组件
7. **统一操作反馈模式**：定义 toast（成功/失败瞬时）+ inline alert（持久错误）+ progress（长时间操作）规范
8. **键盘快捷键体系**：使用 `useMagicKeys` 或类似方案，定义全局快捷键

### P2 — 体验增强

9. **深色模式**：在 `:root` 添加 `[data-theme="dark"]` 变量覆盖
10. **拖拽排序增强**：为 Settings 菜单排序、字段顺序等添加拖拽支持
11. **全局搜索**：Cmd+K 命令面板
12. **自动保存**：模板草稿、设置修改自动保存
13. **系统通知**：长任务完成后发送 Tauri 系统通知

### P3 — 无障碍与国际化

14. **WCAG AA 对比度修复**：调整 `--docsy-text-muted` 色值
15. **键盘导航完整化**：所有交互元素添加 focus 管理
16. **ARIA 标注**：为自定义组件添加语义化属性
17. **i18n 基础设施**：当前硬编码中文，未来可接入 vue-i18n

---

## 附录：组件文件大小统计

| 文件 | 行数 | 大小 | 复杂度评级 |
|------|------|------|-----------|
| EvidencePdfWorkbench.vue | 3634 | 123KB | 🔴 极高 |
| TemplateView.vue | 2900 | 103KB | 🔴 极高 |
| TemplateBuildTab.vue | 1464 | 46KB | 🟠 高 |
| PdfToolsView.vue | 1154 | 38KB | 🟠 高 |
| TemplateRenderTab.vue | 1145 | 37KB | 🟠 高 |
| ImagePaddlerView.vue | 1076 | 33KB | 🟡 中 |
| HeaderFooterRuleFields.vue | 877 | 31KB | 🟡 中 |
| VideoExtractView.vue | 691 | 19KB | 🟢 正常 |
| SettingsView.vue | 676 | 19KB | 🟢 正常 |
| ExistingPdfElementsDialog.vue | 351 | 14KB | 🟢 正常 |
| TemplateSettingsTab.vue | 344 | 11KB | 🟢 正常 |
| PdfJsPreview.vue | 289 | 7KB | 🟢 正常 |
| ReorderableImageGrid.vue | 346 | 11KB | 🟢 正常 |
| EvidencePdfView.vue | 234 | 6KB | 🟢 正常 |
| FileQueuePanel.vue | 203 | 5KB | 🟢 正常 |
| TemplateHistoryTab.vue | 218 | 6KB | 🟢 正常 |
| ToolWorkspaceShell.vue | 116 | 2KB | 🟢 简单 |
| DocletWorkingPet.vue | 140 | 3KB | 🟢 简单 |
| AboutView.vue | 157 | 4KB | 🟢 简单 |
| TextPlacementFields.vue | 64 | 3KB | 🟢 简单 |
| PageNumberRuleDialog.vue | 111 | 5KB | 🟢 简单 |
| UndoRedoButtons.vue | 50 | 1KB | 🟢 简单 |
| reorderableItems.js | 8 | <1KB | 🟢 简单 |

> **建议**：超过 500 行的组件应优先考虑拆分；超过 1000 行的组件是必须拆分的候选项。
