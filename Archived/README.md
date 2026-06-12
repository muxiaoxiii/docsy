# Docsy

轻量、高效、可爱的文档处理工具箱。Tauri 2 + Vue 3 + Rust。

## 功能状态

| 功能 | 状态 | 说明 |
|------|------|------|
| 文档生成 | ✅ 可用 | 字段表单 + docx 输出 + 历史记录 |
| 模板编辑 | ✅ 可用 | source map + 固有字编辑 + 字段标记 + .docsytpl 保存 |
| 模板管理 | ✅ 可用 | 字段配置 + 字典 + 归档/恢复 + Excel 导入导出 |
| PDF 解锁 | ✅ 可用 | 依赖 qpdf |
| PDF 证据整理 | ✅ 可用 | 扫描 → 分组合成 → 整理身份 → 页眉页脚 → 合并 |
| 图片排版 | ✅ 可用 | A4 布局 + fit/fill/original 缩放 + docx/pdf 输出 |
| 视频抽帧 | ✅ 可用 | 依赖 ffmpeg + 时间戳水印 |
| 基础格式工具栏 | ⏳ 未开始 | 字体/字号/加粗/斜体/下划线（固有字编辑） |
| 预置模板机制 | ⏳ 未开始 | 首次启动自动初始化 + 恢复出厂 |
| 批量生成 | ⏳ 未开始 | Excel/CSV → 多份文档 |

## 开发依赖

- Rust 1.80+
- Node.js 20+ + npm
- tauri-cli v2：`cargo install tauri-cli --version "^2" --locked`
- qpdf（运行时）：`brew install qpdf`（macOS）或从 [GitHub releases](https://github.com/qpdf/qpdf/releases) 下载（Windows）
- ffmpeg（可选，视频抽帧）：`brew install ffmpeg`
- LibreOffice（可选，DOC/DOCX 转 PDF）

## 本地启动

```bash
npm install
cargo tauri dev
```

首次启动会触发 Rust 依赖下载与编译，需要几分钟。

## 测试

```bash
# Rust 测试（45 个）
cd src-tauri && cargo test --quiet

# 前端测试（34 个，Vitest）
npm test

# 前端构建
npx vite build
```

## 目录结构

```text
Docsy/
├── README.md
├── Docsy软件设计文档.md          # 产品全貌、架构设计、接手约定
├── docs/
│   ├── project-health-report.md  # 项目健康报告（问题清单 + 修复方向）
│   ├── pdf-evidence-workflow.md  # PDF 证据整理设计与实现
│   ├── docx-renderer-research-officecli.md  # docx 渲染研究
│   ├── docx-research.md          # docx XML 结构笔记
│   ├── template-editor-refactor.md  # [归档] 编辑器重构记录
│   ├── template-presets.md       # [归档] 预置模板方案
│   ├── logging-observability.md  # 日志规范
│   └── docsy-module-status-and-improvement-plan.md  # [归档] GPT 模块分析
├── templates/                    # 开发用 docx 样本
├── src/                          # 前端
│   ├── App.vue                   # 一级菜单 + keep-alive 路由
│   ├── views/
│   │   ├── HomeView.vue          # 首页
│   │   ├── LetterView.vue        # 模板生成页
│   │   ├── TemplateView.vue      # 模板编辑器壳
│   │   ├── ManageView.vue        # 模板管理
│   │   ├── RecordsView.vue       # 记录中心
│   │   ├── PdfToolsView.vue      # PDF 工具壳
│   │   ├── PdfUnlock.vue         # PDF 解锁
│   │   ├── PdfEvidenceView.vue   # PDF 证据整理
│   │   ├── ImagePaddlerView.vue  # 图片排版
│   │   ├── VideoExtractView.vue  # 视频抽帧
│   │   └── SettingsView.vue      # 设置
│   ├── features/template-editor/ # 模板编辑器核心
│   │   ├── composables/          # session、preview、marks、textEdit、save
│   │   ├── services/             # API、mapper、预览服务
│   │   ├── components/           # Toolbar、PreviewPane、MarkAside、MarkPopover
│   │   └── utils/                # textRange、docxModel、htmlEscape
│   ├── components/               # FieldText/Date/Select/Party/Reference/List、DictEditor
│   └── services/appLogger.js     # 前端日志
└── src-tauri/                    # Rust 后端
    ├── src/
    │   ├── lib.rs                # Tauri 命令注册（58 个命令）
    │   ├── template_builder.rs   # 模板制作/编辑
    │   ├── docx/
    │   │   ├── model.rs          # DocxDocumentModel + source map
    │   │   ├── render.rs         # 字段替换渲染
    │   │   ├── utils.rs          # XML 工具函数
    │   │   └── package.rs        # OPC 包读写
    │   ├── pdf/
    │   │   ├── qpdf.rs           # qpdf 调用（解锁/合并/overlay）
    │   │   ├── evidence.rs       # 证据整理
    │   │   └── overlay.rs        # 页眉页脚 overlay 引擎
    │   ├── ffmpeg/               # 视频抽帧
    │   ├── image_paddler.rs      # 图片排版
    │   ├── templates.rs          # 模板配置 + 归档
    │   ├── history.rs            # 生成记录 + 设置
    │   ├── dict_xlsx.rs          # 字典 Excel
    │   └── app_log.rs            # 日志系统
    └── templates/                # 内置资源（letter.docx、fields、dictionaries）
```

## 接手指南

1. 先读本文档了解全貌
2. 读 `docs/project-health-report.md` 了解问题和方向
3. 按需查阅 `Docsy软件设计文档.md` 的具体章节
4. 模板编辑器相关改动参考 `docs/template-editor-refactor.md`
5. PDF 证据整理参考 `docs/pdf-evidence-workflow.md`

## 关键原则

- `.docsytpl` 是模板当前真源（不是 HTML，不是 plainText）
- 字段不能在管理页被物理删坏，真正取消字段必须回到模板编辑器
- 所有文档处理必须另存，不覆盖原件
- 法律文档日志必须脱敏（不记录完整正文/base64/敏感字段）
- 每次改动必须有测试或冒烟验证
