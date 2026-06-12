# Docsy

轻量、高效的本地文档处理工具箱。Tauri 2 + Vue 3 + Rust。

> 当前版本：v0.5.1 — 架构重构中

## 功能模块

| 模块 | 说明 | 状态 |
|------|------|------|
| 文档生成 | 基于模板的 Word/PDF 文档生成 | 开发中 |
| 模板编辑 | 可视化制作 .docsytpl 模板 | 开发中 |
| 模板管理 | 模板 CRUD、字典、归档 | 开发中 |
| PDF 工具 | 解锁、合并、拆分、证据整理 | 迁移中 |
| 图片排版 | 图片批量排版为 A4 文档 | 迁移中 |
| 视频抽帧 | 按时间/频率导出视频帧 | 迁移中 |

## 开发依赖

- Rust 1.80+
- Node.js 20+ + npm
- tauri-cli v2: `cargo install tauri-cli --version "^2" --locked`
- qpdf (运行时): `brew install qpdf`
- ffmpeg (可选): `brew install ffmpeg`

## 本地启动

```bash
npm install
cargo tauri dev
```

## 测试

```bash
cd src-tauri && cargo test --quiet
npm test
npx vite build
```

## 目录结构

```
Docsy/
├── src/                          # 前端 (Vue 3)
│   ├── App.vue                   # 壳：布局 + router-view
│   ├── router/                   # vue-router
│   ├── core/                     # 模块注册中心 + tauriBridge
│   ├── stores/                   # Pinia stores
│   ├── modules/                  # 自包含功能模块
│   │   ├── home/
│   │   ├── doc-gen/              # 文档生成
│   │   ├── template-editor/      # 模板编辑
│   │   ├── template-mgmt/        # 模板管理
│   │   ├── pdf-tools/            # PDF 工具
│   │   ├── image-paddler/        # 图片排版
│   │   ├── video-extract/        # 视频抽帧
│   │   └── settings/             # 设置
│   ├── components/               # 跨模块共享组件
│   └── services/                 # 日志等服务
├── src-tauri/                    # 后端 (Rust)
│   └── src/
│       ├── lib.rs                # 入口
│       ├── commands/             # Tauri 命令层（薄壳）
│       ├── services/             # 业务逻辑层
│       ├── docx/                 # docx 底层操作
│       ├── pdf/                  # PDF 操作
│       ├── ffmpeg/               # FFmpeg 集成
│       └── external/             # 外部工具统一抽象
├── Archived/                     # v0.4 及更早版本归档
└── CHANGELOG.md
```

## 版本历史

- **v0.5.1** (2026-06-12): 架构重构启动，模块注册系统，vue-router + Pinia
- **v0.1.0**: 初始版本

详见 [CHANGELOG.md](./CHANGELOG.md)
