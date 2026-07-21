# Docsy

轻量本地文档处理工具箱。基于 **Tauri 2 + Vue 3 + Rust**。

当前开发分支：`codex/template-quickxml-0.8`（0.8.1）

## 功能模块

| 模块 | 说明 |
| --- | --- |
| **模板系统** | Word 模板制作 / 字段填写 / 生成文书，基于 quick-xml 引擎解析 OOXML |
| **PDF 工具** | 解锁、合并、拆分、页眉页脚叠加、页面提取、压缩 |
| **证据 PDF** | 批量页眉页脚检测与标注、合并证据导入拆分、页码自动分配 |
| **图片排版** | 批量图片排版为 A4 文档 |
| **视频抽帧** | 按时间范围或帧率导出视频帧 |
| **设置** | 外部工具管理（qpdf/poppler/ffmpeg）、模板回收站、菜单排序 |

## 技术架构

```
前端: Vue 3 + Element Plus + Pinia + Vue Router + pdfjs-dist
后端: Rust (Tauri 2) + quick-xml + lopdf + allsorts + rusqlite
```

### 模板引擎 (0.8.x)

0.8 版本将模板引擎从正则表达式改写为 **quick-xml 结构化 XML 树**：

```
src-tauri/src/docx_template/
├── engine.rs    # 命令入口：inspect / save / render
├── ooxml.rs     # quick-xml 树解析与序列化
├── scan.rs      # 标黄扫描 + 文本框递归 + 格式检测
├── save.rs      # 坐标定位 + sdt 包裹 + 同 run 多字段拆分
├── render.rs    # 值替换 + 格式保留 + 表格行复制 + 前后缀擦除
├── index.rs     # 稳定文本索引（段落/run/text 坐标）
├── package.rs   # docx/docsytpl zip 读写 + 大小限制
├── table.rs     # 表格行复制 + 多字段检测
└── migration.rs # manifest 版本迁移
```

**关键改进**（相对 0.7.x 正则引擎）：
- 坐标管线：scan 和 save 共用 paragraph/run 索引，消除错位
- 非黄色高亮完整保留，不再被保存模板时误删
- 既有 Word 内容控件主动检测并拒绝
- 拆分字段（同 run 子串）按 start/end 偏移精确定位
- 文本框中标黄可被扫描到
- 格式保留：渲染时保留原 run 的 `w:rPr`（字体/加粗/斜体/符号字体）

### Doclet 工作动画

右下角宠物动画，长耗时操作自动出现（350ms 防闪烁延迟），支持手动触发的 `showLoading()`/`hideLoading()` API：

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
npm test                    # 前端 vitest (55 tests)
npm run lint                # ESLint
cargo test --manifest-path src-tauri/Cargo.toml   # Rust tests (154 tests)
cargo check --manifest-path src-tauri/Cargo.toml  # Rust type check
```

### 外部工具

qpdf / Poppler / FFmpeg 可在设置页下载安装到 Docsy 工具目录，或使用系统已安装版本。LibreOffice 可选（用于旧版 .doc 转换）。

## 目录结构

```
Docsy/
├── src/
│   ├── App.vue
│   ├── core/               # 核心工具
│   │   ├── tauriBridge.js  # Tauri IPC + 动画事件
│   │   ├── loading.js      # 手动加载动画 API
│   │   ├── moduleRegistry.js
│   │   ├── filePath.js / numberFormat.js / pdfUtils.js / unitConversion.js
│   ├── modules/
│   │   ├── home/           # 首页
│   │   ├── template/       # 模板系统（制作/填写/历史）
│   │   ├── pdf-tools/      # PDF 工具 + 证据工作台
│   │   ├── evidence-pdf/   # 证据 PDF 生成
│   │   ├── image-paddler/  # 图片排版
│   │   ├── video-extract/  # 视频抽帧
│   │   └── settings/       # 设置
│   ├── router/             # 路由
│   ├── stores/             # Pinia store
│   ├── services/           # 日志 / 开发追踪
│   └── shared/components/  # 共享组件（宠物动画等）
├── src-tauri/
│   └── src/
│       ├── docx_template/  # quick-xml 模板引擎
│       ├── commands/       # Tauri 命令注册
│       ├── external/       # 外部工具管理
│       ├── ffmpeg/         # 视频处理
│       ├── pdf/            # PDF 处理
│       ├── services/       # 数据目录 / 模块注册
│       └── template_history.rs  # 模板填写历史 (SQLite)
├── .github/workflows/      # CI/CD (macOS + Windows)
└── docs/                   # 设计文档 / Code Review 报告
```

## 构建

```bash
# macOS
npm run tauri:build:mac

# Windows（交叉编译）
npm run tauri:build:windows
```

CI 在 `git push --tags v*` 时自动构建 macOS (.dmg) 和 Windows (.exe) 包。
