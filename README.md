# Docsy

轻量、高效、可爱的文档处理工具箱。Tauri + Vue + Rust。

## 当前状态

Docsy 已具备文档模板生成、模板制作/编辑、模板管理、记录中心、PDF 解锁、图片排版、视频抽帧和基础设置能力。

详细设计和接手信息以 `Docsy软件设计文档.md` 为准；模板编辑器重构执行方案见 `docs/template-editor-refactor.md`。

## 当前核心开发焦点

主线：**文档 → 提取模板 → 套用模板 → 编辑模板/字段字典/模板设置**。

关键原则：
- 模板身份分两层：用户可见的稳定 `templateId`（如 `letter`）+ 内部内容版本 `versionId`
- 模板制作和模板编辑使用同一个 `TemplateEditor` 内核；区别只在初始化来源和字段呈现方式
- 内置/用户模板走同一套字段 schema、字典、渲染逻辑；编辑后的当前版本必须在生成、管理、字典、历史中一致生效
- `.docsytpl` 保存 `builder_state.json` 保留制作经验，旧模板回退到占位符反推；内置模板编辑后以用户当前版本覆盖出厂版本但保留恢复能力
- 连续空格人名以 docx XML 字符位置为准，不依赖 HTML 折叠空白

近期 Roadmap：
1. 模板闭环统一（制作/编辑/生成/设置围绕同一份字段 schema）
2. 模板编辑器升级（字段字显示为 `<<标签>>`，固有字已支持纯文字编辑；后续补基础字体/字号/加粗/斜体/下划线工具栏）
3. 定位底座升级（预览节点带 docx 源位置，解决重复文本/连续空格/跨 run）
4. 经验复用系统化（历史记录 → 字段经验库 → 一键重新生成）
5. 内置/用户模板对等（抽象通用模板能力，不再为单份模板写特殊逻辑）
6. 回归测试 + 生成文件可用性校验

## 开发依赖

- Rust（1.80+）
- Node.js（20+）+ npm
- qpdf（PDF 解锁运行时依赖）
  - macOS：`brew install qpdf`
  - Windows：从 https://github.com/qpdf/qpdf/releases 下载，把 `qpdf.exe` 放到打包产物同目录
- tauri-cli v2：`cargo install tauri-cli --version "^2" --locked`

## 本地启动

```bash
npm install
cargo tauri dev
```

首次启动会触发 Rust 依赖下载与编译，需要几分钟。

## 目录结构

```
Docsy/
├── Docsy软件设计文档.md      # 产品、架构、进度与接手约定
├── README.md
├── package.json
├── vite.config.js
├── index.html
├── docs/
│   ├── docx-research.md      # docx 格式研究笔记
│   └── template-editor-refactor.md # 模板编辑器重构执行文档
├── templates/                 # 内置 Word 模板（源文件）
├── src/                       # 前端
│   ├── main.js
│   ├── App.vue                # 一级菜单 + keep-alive 路由
│   ├── views/
│   │   ├── HomeView.vue       # 首页（快捷入口 + 最近模板/记录）
│   │   ├── LetterView.vue     # 模板生成页（通用，接 prop templateId）
│   │   ├── TemplateView.vue   # 模板制作/编辑器（薄壳，转发到 features）
│   │   ├── ManageView.vue     # 模板管理
│   │   ├── RecordsView.vue    # 记录中心
│   │   ├── ImagePaddlerView.vue # 图片排版
│   │   ├── VideoExtractView.vue # 视频抽帧
│   │   ├── PdfToolsView.vue   # PDF 工具壳
│   │   ├── PdfUnlock.vue      # PDF 解锁
│   │   ├── SettingsView.vue   # 设置页
│   │   └── PlaceholderView.vue
│   ├── features/
│   │   └── template-editor/   # 模板编辑器核心
│   │       ├── TemplateEditorView.vue
│   │       ├── components/    # Toolbar、PreviewPane、MarkAside、MarkPopover
│   │       ├── composables/   # session、preview、marks、textEdit
│   │       ├── services/      # API、mapper、预览服务
│   │       └── utils/         # 文本范围、HTML 转义
│   └── components/
│       ├── FieldText.vue / FieldDate.vue / FieldSelect.vue
│       ├── FieldParty.vue / FieldReference.vue / FieldList.vue
│       └── DictEditor.vue
└── src-tauri/                 # Rust 后端
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── templates/              # 内置资源（include_bytes!）
    └── src/
        ├── main.rs
        ├── lib.rs              # Tauri 命令注册（30+ 命令）
        ├── docx/               # docx 渲染核心
        │   ├── mod.rs
        │   ├── render.rs       # 字段替换渲染
        │   └── utils.rs        # 公共工具函数（xml_escape、flatten_nested_paragraphs）
        ├── pdf/                # qpdf 调用
        ├── ffmpeg/             # 视频抽帧模块
        │   ├── mod.rs
        │   ├── detect.rs       # FFmpeg 检测与下载
        │   ├── probe.rs        # 视频信息读取
        │   └── extract.rs      # 抽帧执行
        ├── image_paddler.rs    # 图片排版成 docx/pdf
        ├── templates.rs        # 模板配置存储 + 归档
        ├── template_builder.rs # 模板制作/编辑
        ├── dict_xlsx.rs        # 字典 Excel 导入/导出
        └── history.rs          # 生成记录 + 应用设置
```

## 接手指南

先读 `Docsy软件设计文档.md` 的附录 B；如果要改模板制作/编辑模块，再读 `docs/template-editor-refactor.md`。
