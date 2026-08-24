# Docsy 下载站（软件主页）

Docsy 的软件主页与下载镜像站，部署在 **https://docsy.muxiaoxi.top**（Caddy 自动 HTTPS）。
解决国内用户从 GitHub 下载安装包困难的问题。

- 页面：
  - `index.html` 软件主页（介绍 + 下载 + 软件截图 + 更新日志）
  - `tools.html` 外部工具下载页（qpdf / Poppler / FFmpeg 的系统-版本对照 + 官方地址 + 风险提示）
- 部署：极空间（或任意支持 Docker Compose 的 NAS）上跑一个 Caddy 容器
- 同步：GitHub Actions 发布时推送（主通道）+ NAS 定时任务拉取（兜底）

```
site/
├── public/                  # 静态站点（部署后挂载给 Caddy）
│   ├── index.html           # 软件主页
│   ├── tools.html           # 外部工具下载页
│   ├── css/ js/ assets/     # 样式、脚本、图片（logo/吉祥物/图标）
│   ├── data/                # releases.json / changelog.json / tools.json（脚本生成）
│   │                        # screenshots.json（软件截图清单，手动维护）
│   ├── assets/screenshots/  # 软件截图图片（放入后登记到 screenshots.json）
│   └── downloads/           # 安装包（同步脚本拉取，不入仓库）
├── Caddyfile                # Caddy 配置（域名 docsy.muxiaoxi.top）
├── docker-compose.yml       # 一键部署
└── scripts/
    ├── build-data.py        # 生成数据 + 补下载安装包（CI 与 NAS 共用）
    └── sync-releases.sh     # NAS 定时任务兜底同步
```

---

## 一、部署到极空间

### 1. 准备目录

把本目录上传到 NAS，例如 `/volume1/docker/docsy-site`（极空间路径一般是
`/tmp/zspace/...` 或你挂载的存储卷，按实际为准）。目录里必须有：

```
docsy-site/
├── Caddyfile
├── docker-compose.yml
└── public/          （含 downloads/ 安装包与 data/ 数据）
```

> 仓库里的 `site/public/downloads/` 已包含最新版安装包（beta26）。
> 部署后也可以手动运行同步脚本补齐/更新，见下文"四、同步安装包"。

### 2. 确认域名

`Caddyfile` 里已配置域名 **docsy.muxiaoxi.top**，无需修改（如果换域名，改第一行即可）。
把该域名的 **A 记录** 解析到 NAS 的公网 IP（已完成域名解析这一步）。

### 3. 创建容器项目

在极空间的 Docker 应用中：

1. 打开 **容器 / 项目（Compose）**，新建项目
2. 项目名称填 `docsy-site`，把 `docker-compose.yml` 的内容粘贴进去
3. 确认工作目录指向 `docsy-site` 文件夹
4. 启动项目

Caddy 会：

- 监听 80 / 443 端口
- 首次启动自动向 Let's Encrypt 申请域名证书并续期
- 把 `public/` 作为站点根目录提供服务

> **端口要求**：NAS 的 80 / 443 端口需要已做端口转发（如果你用 80/443 之外的
> 端口，把 docker-compose.yml 里的端口映射改掉，但域名访问就要带端口了）。
>
> **国内部署提示**：如果域名在国内备案，且 NAS 在公网 80/443 提供服务，需要完成
> ICP 备案。域名在国外注册、或通过 CDN 回源可不备案，请按你的实际情况处理。

### 4. 验证

浏览器打开 `https://你的域名`，能看到主页即可。

---

## 二、GitHub Actions 自动推送（主通道）

每次发布新版本，GitHub Actions 会自动把安装包和数据同步到 NAS，无需手动操作。

1. **NAS 开启 SSH**，确认能 `ssh` 登录
2. 生成一对密钥（如 `ssh-keygen -t ed25519`），把**公钥**加到 NAS 的
   `~/.ssh/authorized_keys`
3. 在 GitHub 仓库 **Settings → Secrets and variables → Actions** 添加：

   | Secret | 说明 | 示例 |
   |---|---|---|
   | `NAS_HOST` | NAS 的 SSH 地址 | `docsy.example.com` 或 DDNS 域名 |
   | `NAS_PORT` | SSH 端口 | `22` |
   | `NAS_USER` | SSH 用户名 | `root` 或你的用户名 |
   | `NAS_SSH_KEY` | SSH 私钥全文（含 BEGIN/END 行） | |
   | `NAS_PATH` | 同步目标目录（**需提前创建**） | `/volume1/docker/docsy-site/public` |

4. 工作流文件 `.github/workflows/sync-site-to-nas.yml` 已就绪：
   - 每次 `release published` 自动触发
   - 也可以在 Actions 页面手动 Run workflow

> 注意：NAS 需要能被 GitHub Actions 的服务器访问（公网 SSH 入站）。
> 家宽没有公网入站或 SSH 端口未转发时，此通道无法使用，改用下面的定时拉取方案。

---

## 三、NAS 定时任务兜底（推荐同时配置）

不依赖公网入站，NAS 主动从 GitHub 拉取。

1. 测试一次：

   ```bash
   cd /volume1/docker/docsy-site
   ./scripts/sync-releases.sh
   ```

2. 添加 crontab（极空间可以在 Docker 容器里挂一个 cron，或 NAS 宿主
   上执行；每小时检查一次）：

   ```cron
   10 * * * * cd /volume1/docker/docsy-site && ./scripts/sync-releases.sh >> sync.log 2>&1
   ```

3. 国内 NAS 直连 GitHub 慢时，设置代理前缀（仅影响安装包下载）：

   ```bash
   GH_PROXY=https://ghproxy.net/ ./scripts/sync-releases.sh
   ```

---

## 四、数据与文件说明

- `public/data/releases.json`：版本列表、安装包文件名/大小/SHA256/下载地址，
  页面据此渲染下载按钮与历史版本
- `public/data/changelog.json`：由 CHANGELOG.md 解析生成，页面渲染更新日志
- `public/data/tools.json`：外部工具（qpdf/Poppler/FFmpeg）的系统-版本对照与
  官方下载地址，`tools.html` 据此渲染；数据与 app 内置工具清单
  （`src-tauri/src/external/managed.rs`）保持一致
- `public/downloads/`：安装包本体；文件名以 `Docsy_` 开头
- 两个同步通道（Actions 推送 / NAS 拉取）都调用同一个 `build-data.py`，
  重复文件按"同名 + 同大小"跳过，不会重复下载

手动刷新数据（不上传安装包时）：

```bash
python3 site/scripts/build-data.py \
  --api-json <(curl -s "https://api.github.com/repos/muxiaoxiii/docsy/releases?per_page=12") \
  --assets-dir site/public/downloads \
  --changelog CHANGELOG.md \
  --out site/public/data
```

### 添加软件截图

1. 把截图（建议 1200px 左右宽、PNG/WebP）放入 `public/assets/screenshots/`
2. 在 `public/data/screenshots.json` 的 `items` 里登记：
   ```json
   { "items": [
     { "src": "assets/screenshots/home.png", "alt": "首页" },
     { "src": "assets/screenshots/evidence.png", "alt": "证据处理" }
   ] }
   ```
3. 主页「软件截图」区会自动显示；`items` 为空时该区自动隐藏

---

## 五、维护

- **更新页面内容**：修改 `public/index.html` / `css/style.css` / `js/main.js`，
  重新 scp/rsync 到 NAS 即可（Caddy 静态站点无需重启容器）
- **版本信息过期**：运行同步脚本刷新 `data/`
- **证书续期**：Caddy 自动处理，无需干预
- **回滚**：历史安装包保留在 `downloads/`，页面"历史版本"可直接下载

## 六、常见问题

- **证书申请失败**：确认域名 A 记录指向 NAS、80/443 端口可达
- **下载 404**：`downloads/` 里没有对应文件。先跑同步脚本，或检查
  `releases.json` 里该版本的 `url`
- **页面显示旧版本**：数据文件被浏览器缓存。Caddy 已对 `/data/*` 设
  `Cache-Control: no-store`，强制刷新一次即可
- **极空间上容器起不来**：确认 compose 里 `./public` 相对路径与实际目录一致
