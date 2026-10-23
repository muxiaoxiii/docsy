# Docsy 官网（GitHub Pages）

Docsy 官方主页部署在 **https://docsy.muxiaoxi.top**，基于 **GitHub Pages** 托管与自动发布。

---

## 一、自动化发布流程

当向仓库推送版本标签（例如 `v1.1.2`）时：
1. **构建安装包**（`.github/workflows/build-desktop.yml`）：
   - 自动编译构建 macOS（Apple Silicon）、Windows x64 与 Windows x86 安装包；
   - 自动在 GitHub Releases 发布新版本并上传安装包及 SHA256 校验和。
2. **部署官网**（`.github/workflows/deploy-pages.yml`）：
   - 发布安装包后，通过 `workflow_dispatch` 明确触发 `main` 分支官网流程；不依赖旧默认分支的 `workflow_run` 配置；
   - 修改 `main` 上的 `site/**`、更新日志或官网流程，也会独立触发部署，无需重新打包桌面应用；
   - 从 GitHub Releases API 读取最新的已发布版本与下载链接；
   - 运行 `site/scripts/build-data.py` 刷新 `releases.json`、`changelog.json` 和相关下载信息；
   - 下载区与回退数据只使用已发布版本；更新日志与截图分别刷新，单项加载失败不会阻塞其他内容；
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

## 四、素材与校验

截图取自当前版本的浏览器界面预览，使用示例文字，不包含用户文件。更新 `assets/screenshots/` 时，同步核对 `data/screenshots.json` 与静态 HTML 的说明。官网保留已有 Doclet 形象。

部署前运行 `python3 -m unittest discover -s site/scripts` 与 `node --test site/scripts/test-site-runtime.mjs`，并检查首页、功能详情、组件说明及下载链接。
