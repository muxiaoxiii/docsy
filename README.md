# Docsy

轻量的本地文档处理工具箱。当前版本已移除模板生成、内置模板、模板编辑、模板管理和字典推荐链路，后续模板模块从零重新设计。

## 当前保留模块

| 模块 | 说明 |
| --- | --- |
| PDF 工具 | PDF 解锁、合并、拆分、证据整理、页眉页脚叠加 |
| 图片排版 | 图片批量排版为 A4 文档 |
| 视频抽帧 | 按时间或频率导出视频帧 |
| 设置 | 外部工具检测、LibreOffice 路径、日志与诊断 |

## 已移除模块

- 模板生成 / 文档生成表单
- 内置所函模板
- 模板制作 / 模板编辑器
- 模板管理
- 字典、字段历史、当事人主档
- 模板配置包导入导出
- 模板生成记录中心

## 开发依赖

- Rust 1.80+
- Node.js 20+ + npm
- Tauri CLI v2：`cargo install tauri-cli --version "^2" --locked`
- qpdf / Poppler / FFmpeg：运行时可在“设置 -> 外部工具状态”中下载安装到 Docsy 工具目录；系统已安装时会优先复用
- LibreOffice 可选，用于文档转换能力；因体积和安装器差异较大，保留系统安装和路径配置方式

## 本地启动

```bash
npm install
npm run tauri dev
```

## 验证

```bash
npm test
npx vite build
cd src-tauri && cargo test --quiet
```

## 目录结构

```text
Docsy/
├── src/
│   ├── App.vue
│   ├── core/
│   ├── modules/
│   │   ├── home/
│   │   ├── pdf-tools/
│   │   ├── image-paddler/
│   │   ├── video-extract/
│   │   └── settings/
│   ├── router/
│   └── services/
├── src-tauri/
│   └── src/
│       ├── commands/
│       ├── external/
│       ├── ffmpeg/
│       ├── pdf/
│       └── services/
└── docs/
```

## 后续模板重写原则

新模板模块不要复用已删除的旧路径和旧命令名。重新设计时应先定义统一的数据模型和模块边界，再实现生成、编辑、管理三类体验。
