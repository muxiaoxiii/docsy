# Docsy 官网（GitHub Pages）

Docsy 官方主页部署在 **https://docsy.muxiaoxi.top**，基于 **GitHub Pages** 托管与自动发布。

---

## 一、自动化发布流程

当向仓库推送版本标签（例如 `v1.0.7`）时：
1. **构建安装包**（`.github/workflows/build-desktop.yml`）：
   - 自动编译构建 macOS（Apple Silicon）、Windows x64 与 Windows x86 安装包；
   - 自动在 GitHub Releases 发布新版本并上传安装包及 SHA256 校验和。
2. **部署官网**（`.github/workflows/deploy-pages.yml`）：
   - 在桌面包发布完成后自动触发；
   - 从 GitHub Releases API 读取最新的已发布版本与下载链接；
   - 运行 `site/scripts/build-data.py` 刷新 `releases.json`、`changelog.json` 和相关下载信息；
   - 自动部署发布到 GitHub Pages（`docsy.muxiaoxi.top`）。

---

## 二、目录结构

```
site/
├── public/                  # 静态站点根目录
│   ├── index.html           # 软件主页（功能展示、下载入口、更新日志）
│   ├── features.html        # 功能特性详情页
│   ├── tools.html           # 外部工具对照与下载指南（qpdf / Poppler / FFmpeg）
│   ├── css/ js/ assets/     # 样式表、脚本逻辑与视觉素材（Logo、吉祥物、UI 截图）
│   └── data/                # 由脚本生成的 releases.json / changelog.json / tools.json
└── scripts/
    └── build-data.py        # 提取 Releases 与 CHANGELOG.md 数据供前端加载
```

---

## 三、手动部署与测试

如需在本地调试或手动触发部署：
1. 在 GitHub Actions 页面直接点击 **Deploy Docsy Site to GitHub Pages** -> **Run workflow** 即可全量刷新并重新上线。
2. 本地调试静态页面只需通过任意 HTTP 服务访问 `site/public/` 目录即可：
   ```bash
   npx serve site/public
   ```
