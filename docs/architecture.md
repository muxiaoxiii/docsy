# Docsy 架构

更新时间：2026-08-03（v0.9.0）

## 模块边界

Docsy 由 6 个功能模块组成，通过统一的模块注册系统自动发现：

```
src/modules/
├── home/           # 首页（快捷入口 + 版本信息）
├── evidence-pdf/   # 证据处理入口（→ EvidencePdfView）
├── pdf-tools/      # PDF 工具（解锁/合并/提取/压缩/拆分 + 证据工作台）
├── image-paddler/  # 图片排版
├── video-extract/  # 视频抽帧
├── template/       # 文书模板（制作/填写/批量/历史）
└── settings/       # 设置与诊断
```

每个模块通过 `index.js` 注册路由、菜单项和首页卡片。新增模块只需创建自包含目录。

## 前端结构

| 层 | 路径 | 职责 |
| --- | --- | --- |
| 入口 | `App.vue` | 主布局（侧边栏 + 路由视图 + Doclet 动画） |
| 路由 | `router/index.js` | 从 moduleRegistry 动态生成 |
| 状态 | `stores/app.js` | Pinia store（应用设置） |
| 核心 | `core/` | IPC 封装、模块注册、路径工具、PDF 工具函数 |
| 共享组件 | `shared/components/` | ToolWorkspaceShell / FileQueuePanel / ReorderableImageGrid / DocletWorkingPet |
| 业务模块 | `modules/*/` | 各模块的视图、组件和 composables |

## 后端结构

Tauri 命令集中在 `src-tauri/src/commands/mod.rs` 注册（共 58 个命令）。

| 域 | 命令数 | 业务实现 |
| --- | --- | --- |
| `pdf` | 20 | `pdf/` 目录（detection/header_footer/evidence/split/overlay/...） |
| `template` | 16 | `docx_template/`（quick-xml 引擎）+ `template_history.rs`（SQLite） |
| `settings` | 8 | `services/history.rs` + `external/`（工具检测与安装） |
| `system` | 8 | `app_log.rs` + 系统交互 |
| `video` | 4 | `ffmpeg/`（检测/探测/抽帧） |
| `image_paddler` | 2 | `image_paddler.rs` |

## 模板系统设计

设计文档：`docs/template-system-design.md`

- Word 中黄色高亮作为字段制作入口
- 保存 `.docsytpl` zip 包（manifest.json + template.docx）
- 普通字段写入 `<w:sdt>` 内容控件，勾选字段写入 marker 控件
- quick-xml 结构化 XML 树引擎，坐标管线消除错位
- 批量填写：Excel 导出/校验/批量生成
- 7 种字段类型 + 4 种历史建议源

## PDF 证据处理设计

设计文档：`docs/pdf-evidence-processing-design.md`

- 核心对象：证据文件列表 + 页码范围 + 输出规则
- 三层检测：Artifact 标记 / 内容文本 / 视觉区域
- 页眉、页脚文字、页码三类独立对象
- qpdf 作为安全改写和结构校验底座
- PDF.js 负责前端真实页面预览
- Poppler 工具链作为后端渲染和文本检测兜底

## 外部工具抽象

`src-tauri/src/external/` 提供统一的外部工具管理：

| 工具 | 文件 | 用途 |
| --- | --- | --- |
| qpdf | `qpdf.rs` | PDF 结构处理、合并、拆分、overlay |
| Poppler | `poppler.rs` | PDF 文本提取、渲染 |
| FFmpeg | `ffmpeg.rs` | 视频处理 |
| Word | `word.rs` | Word → PDF 转换（macOS AppleScript / Windows COM） |
| WPS | `wps.rs` | WPS → PDF 转换（Windows COM） |
| LibreOffice | `libreoffice.rs` | DOC/DOCX → PDF 转换（备用） |
| 托管安装 | `managed.rs` | 工具下载、安装、更新 |

## 数据存储

| 数据 | 位置 | 格式 |
| --- | --- | --- |
| 应用设置 | `~/Library/Application Support/docsy/settings.json` | JSON |
| 模板库 | `~/Library/Application Support/docsy/templates/` | .docsytpl (zip) |
| 模板回收站 | `~/Library/Application Support/docsy/template-trash/` | .docsytpl (zip) |
| 模板历史 | `~/Library/Application Support/docsy/template_history.sqlite3` | SQLite |
| 应用日志 | `~/Library/Logs/docsy/` | 文本日志 |

## 测试

```bash
npm test                    # 前端 vitest (70 tests, 13 files)
cargo test --manifest-path src-tauri/Cargo.toml   # Rust tests (141 tests)
```

测试覆盖：docx 模板引擎 / PDF 处理 / 图片排版 / 自然排序 / 历史记录 / 页码检测 / 页码规则 / 拆分范围 / 预览坐标 / 模块注册 / 文件拖放 / Pointer Events 排序。
