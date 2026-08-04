# Docsy

轻量本地文档处理工具箱。基于 **Tauri 2 + Vue 3 + Rust**。

面向法律文书场景，提供模板生成、PDF 证据处理、图片排版和视频抽帧等能力。所有处理在本地完成，不上传任何文件。

## 功能模块

### 文书模板

用 Word 标黄可变文字，Docsy 自动扫描字段并生成可重复填写的模板。

- **制作模板**：在 Word 中把需要替换的文字标黄 → 导入 Docsy → 确认字段类型和名称 → 保存为 `.docsytpl`
- **填写模板**：打开 `.docsytpl`，按字段类型填写（文本、日期、下拉选择、当事人列表、引用、勾选组）
- **批量填写**：导出字段表为 Excel → 批量填写 → 校验导入 → 一次生成多个 Word 文件
- **历史建议**：自动记录填写历史，跨模板按通用字段名推荐常用值
- **8 种字段类型**：text / date / select / party_list / reference / checkbox / radio_group / checkbox_group
- **空值规则**：字段为空时自动删除关联的前缀、后缀文字，避免生成病句
- **quick-xml 引擎**：结构化 XML 树解析，精确坐标定位，格式完整保留

### 证据处理

面向法律证据材料的 PDF 整理工具。

- **证据文件列表**：导入独立 PDF 或合并 PDF，识别或设置每个文件的证据身份
- **页眉页脚检测**：三层检测（Artifact 标记 / 内容文本 / 视觉区域），自动识别现有页眉、页脚和页码
- **原有内容处理**：按文件列出检测结果，支持逐项或批量选择删除、忽略、编辑
- **新页眉页脚**：页眉（证据身份）、页脚文字、页码三套独立设置
- **页码样式**：阿拉伯数字、中文数字、大小写罗马数字、带圈数字、实心带圈数字（❶❷❸）
- **页码格式**：`{page}`、`{total}`、`{range}` 以及自定义前后符号（如 `-{page}-`）
- **连续编号**：全部文件连续编号或每个文件单独编号；支持分段例外
- **合并输出**：按证据列表顺序合并为一个 PDF，页码自动连续计算
- **拆分输入**：导入合并 PDF，按页眉变化或手动页段拆分为多个证据文件
- **PDF 预览**：PDF.js 真实页面预览，支持翻页、跳转和坐标定位

### PDF 工具

基础 PDF 处理功能。

| 工具 | 说明 |
| --- | --- |
| **解锁** | 移除 PDF 密码保护，批量检测加密状态，原文件旁生成已解锁副本 |
| **合并** | 按列表顺序合并多个 PDF，支持拖拽调整顺序 |
| **提取页面** | 从 PDF 中挑选指定页导出（如 `3,7,12-15`） |
| **压缩** | 使用 qpdf 重新压缩流、整理对象并移除未引用资源 |
| **拆分** | 翻页预览，按页码范围生成多个独立 PDF，支持拆分后删除页眉页脚区域 |

### 图片排版

批量图片排版为 A4 文档。

- 文件夹扫描与前缀分组
- fit / fill / original 三种缩放模式
- PDF 和 DOCX 双格式输出
- 拖拽排序、分页预览
- 输出顺序与预览一致

### 视频抽帧

从视频中按时间范围或帧率导出帧画面。

- 视频信息读取（时长、分辨率、编码）
- 按时间范围或固定帧率抽帧
- 时间戳水印叠加
- FFmpeg 检测与自动安装

### 设置

应用配置与诊断。

| 功能 | 说明 |
| --- | --- |
| **外部工具** | 检测 qpdf / Poppler / FFmpeg / Word / WPS / LibreOffice 状态，支持下载安装到 Docsy 工具目录 |
| **主菜单** | 模块排序与显示/隐藏 |
| **模板回收站** | 删除的模板可恢复或彻底删除，支持数据迁移 |
| **应用设置** | LibreOffice 路径、工具清单地址 |
| **诊断信息** | 版本号、操作系统、外部工具版本、日志查看 |

### 首页

快捷入口卡片、版本信息、Doclet 宠物工作动画（长耗时操作自动出现）。

## 技术架构

```
前端: Vue 3 + Element Plus + Pinia + Vue Router + pdfjs-dist
后端: Rust (Tauri 2) + quick-xml + lopdf + allsorts + rusqlite
外部: qpdf + Poppler (pdftoppm/pdftotext) + FFmpeg
```

### 模块注册

前端通过 `src/modules/<name>/index.js` 自动注册路由、菜单和首页卡片。新增模块只需创建自包含目录，无需修改全局配置。

### 后端命令

Tauri 命令集中在 `src-tauri/src/commands/mod.rs` 注册，共 6 个命令域：

| 域 | 文件 | 职责 |
| --- | --- | --- |
| `pdf` | `commands/pdf.rs` | PDF 解锁/合并/拆分/提取/压缩/证据处理/页眉页脚/预览 |
| `template` | `commands/template.rs` | 模板检查/保存/渲染/库管理/批量填写/历史 |
| `image_paddler` | `commands/image_paddler.rs` | 图片文件夹分析与排版生成 |
| `video` | `commands/video.rs` | FFmpeg 检测/视频信息/抽帧/帧列表 |
| `settings` | `commands/settings.rs` | 应用设置/模块注册/外部工具检测与安装 |
| `system` | `commands/system.rs` | 文件打开/URL/日志/诊断/字体/超时响应 |

### 模板引擎 (quick-xml)

```
src-tauri/src/docx_template/
├── engine.rs    # 命令入口：inspect / save / render
├── ooxml.rs     # quick-xml 树解析与序列化
├── scan.rs      # 标黄扫描 + 文本框递归 + 格式检测
├── save.rs      # 坐标定位 + sdt 包裹 + 同 run 多字段拆分
├── render.rs    # 值替换 + 格式保留 + 表格行复制 + 前后缀擦除
├── index.rs     # 稳定文本索引（段落/run/text 坐标）
├── package.rs   # docx/docsytpl zip 读写 + 大小限制
├── batch.rs     # 批量填写：Excel 导出/校验/批量生成
└── mod.rs       # 数据结构 + 验证器 + 模板库管理
```

### PDF 处理

```
src-tauri/src/pdf/
├── evidence_session.rs   # 证据列表、页码范围、输出规则
├── page_info.rs          # 页面尺寸、旋转、页数读取
├── preview.rs            # 后端单页渲染（pdftoppm）
├── header_footer.rs      # 页眉页脚读取/检测/插入/删除
├── artifacts.rs          # 标准 Pagination Artifact 检测与写入
├── detection.rs          # 三层检测引擎（Artifact/内容文本/视觉区域）
├── annotations.rs        # 批注读取和删除
├── content_text.rs       # 内容流文本提取
├── normalize.rs          # A4 规范化
├── split.rs              # 合并 PDF 拆分
├── overlay.rs            # qpdf overlay 合成
├── qpdf.rs               # qpdf 低层封装
└── evidence.rs           # 证据文件夹扫描与合并
```

### Doclet 工作动画

右下角宠物动画，长耗时操作自动出现（350ms 防闪烁延迟）。支持手动触发：

```js
import { showLoading, hideLoading } from '@/core/tauriBridge.js'
const id = showLoading('正在导出…')
await doWork()
hideLoading(id)
```

## 开发

### 环境

- Rust 1.80+
- Node.js 20+
- `cargo install tauri-cli --version "^2" --locked`

### 启动

```bash
npm install
npm run tauri dev
```

### 验证

```bash
npm test                    # 前端 vitest (70 tests, 13 files)
npm run lint                # ESLint
cargo test --manifest-path src-tauri/Cargo.toml   # Rust tests (141 tests)
cargo check --manifest-path src-tauri/Cargo.toml  # Rust type check
```

### 外部工具

| 工具 | 用途 | 安装方式 |
| --- | --- | --- |
| qpdf | PDF 合并/拆分/叠加/结构处理 | 设置页下载或系统安装 |
| Poppler | PDF 文本检测和页眉页脚区域识别 | 设置页下载或系统安装 |
| FFmpeg | 视频信息读取和抽帧 | 设置页下载或系统安装 |
| Microsoft Word | Word 转 PDF 首选引擎 | 系统安装 |
| WPS Writer | Windows 下 Word 不可用时的备选 | 系统安装 |
| LibreOffice | Word 转 PDF 备用引擎 | 系统安装 |

设置页可一键检测所有工具状态，qpdf / Poppler / FFmpeg 支持下载安装到 Docsy 工具目录。

## 目录结构

```
Docsy/
├── src/                          # 前端源码 (~20,400 行)
│   ├── App.vue                   # 主布局（侧边栏 + 路由视图 + Doclet 动画）
│   ├── core/                     # 核心工具
│   │   ├── tauriBridge.js        # Tauri IPC 封装 + 动画事件 + 错误处理
│   │   ├── loading.js            # 手动加载动画 API
│   │   ├── moduleRegistry.js     # 模块自动收集与路由/菜单/卡片生成
│   │   ├── filePath.js           # 文件路径工具
│   │   ├── pdfUtils.js           # PDF 页码解析与范围编辑
│   │   ├── numberFormat.js       # 数字格式化
│   │   ├── unitConversion.js     # 单位转换（mm/pt/px）
│   │   └── composables/          # 共享 composables
│   │       ├── usePointerReorder.js    # Pointer Events 拖拽排序
│   │       └── useWindowFileDrop.js    # 窗口文件拖放
│   ├── modules/                  # 功能模块（自包含）
│   │   ├── home/                 # 首页
│   │   ├── template/             # 文书模板（制作/填写/批量/历史）
│   │   ├── pdf-tools/            # PDF 工具 + 证据工作台
│   │   ├── evidence-pdf/         # 证据 PDF 生成入口
│   │   ├── image-paddler/        # 图片排版
│   │   ├── video-extract/        # 视频抽帧
│   │   └── settings/             # 设置
│   ├── shared/components/        # 共享组件
│   │   ├── ToolWorkspaceShell.vue      # 工具工作区布局壳
│   │   ├── FileQueuePanel.vue          # 文件队列面板（可排序）
│   │   ├── ReorderableImageGrid.vue    # 可排序图片网格
│   │   ├── ImagePreviewGrid.vue        # 图片预览网格
│   │   └── DocletWorkingPet.vue        # Doclet 宠物动画
│   ├── router/                   # Vue Router
│   ├── stores/                   # Pinia store
│   ├── services/                 # 日志 / 开发追踪
│   └── styles.css                # 全局样式 + Element Plus 主题覆盖
├── src-tauri/                    # Rust 后端 (~20,600 行)
│   └── src/
│       ├── docx_template/        # quick-xml 模板引擎
│       ├── commands/             # Tauri 命令注册
│       ├── external/             # 外部工具管理（qpdf/poppler/ffmpeg/word/wps/libreoffice）
│       ├── ffmpeg/               # FFmpeg 封装（检测/探测/抽帧）
│       ├── pdf/                  # PDF 处理（检测/页眉/拆分/合并/预览/规范化）
│       ├── services/             # 数据目录 / 模块注册
│       ├── image_paddler.rs      # 图片排版引擎
│       ├── sort_utils.rs         # 自然排序
│       ├── template_history.rs   # 模板填写历史 (SQLite)
│       └── app_log.rs            # 应用日志
├── .github/workflows/            # CI/CD (macOS + Windows)
├── docs/                         # 当前设计文档
│   ├── architecture.md           # 架构总览
│   ├── template-system-design.md # 模板系统设计
│   ├── pdf-evidence-processing-design.md  # PDF 证据处理设计
│   └── pdf-workbench-0.8.6-design.md      # PDF 工作台设计
└── Archived/                     # 归档（旧代码 + 旧文档）
```

## 构建

```bash
# macOS
npm run tauri:build:mac

# Windows（交叉编译）
npm run tauri:build:windows
```

CI 在 `git push --tags v*` 时自动构建 macOS (.dmg) 和 Windows (.exe) 包。

## 许可

MIT
