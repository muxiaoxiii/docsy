# Docsy 网站 Docker 部署快速指南

## 🚀 快速开始

### 1. 准备工作

确保你已经：
- 安装了 Docker 和 Docker Compose
- 有一个域名（如 `docsy.example.com`）
- 域名已解析到你的服务器IP

### 2. 部署步骤

```bash
# 1. 进入site目录
cd /path/to/docsy/site

# 2. 启动Docker服务
docker-compose up -d

# 3. 检查服务状态
docker-compose ps

# 4. 查看日志
docker-compose logs -f
```

### 3. 访问网站

- HTTP: `http://你的域名`
- HTTPS: `https://你的域名`（Caddy会自动申请SSL证书）

## 📋 配置说明

### Docker Compose 配置 (`docker-compose.yml`)

```yaml
services:
  docsy-site:
    image: caddy:2-alpine
    container_name: docsy-site
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - ./public:/srv/www:ro
      - caddy_data:/data
      - caddy_config:/config
    environment:
      - ACME_AGREE=true

volumes:
  caddy_data:
  caddy_config:
```

### Caddy 配置 (`Caddyfile`)

```caddy
docsy.example.com {
    root * /srv/www
    encode gzip
    file_server
    
    handle /downloads/* {
        header Content-Disposition "attachment"
        file_server
    }
    
    header {
        -Server
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
    }
    
    @data path /data/*
    header @data Cache-Control "no-store"
    
    log {
        output stdout
        format console
    }
}
```

## 🔧 常用命令

### 服务管理

```bash
# 启动服务
docker-compose up -d

# 停止服务
docker-compose down

# 重启服务
docker-compose restart

# 查看日志
docker-compose logs -f

# 进入容器
docker exec -it docsy-site sh
```

### 更新网站内容

```bash
# 1. 更新public目录中的文件
# 2. 重启服务（Caddy会自动重新加载配置）
docker-compose restart
```

### 备份与恢复

```bash
# 备份数据卷
docker run --rm -v docsy-site_caddy_data:/data -v $(pwd):/backup alpine tar czf /backup/caddy_data_backup.tar.gz /data

# 恢复数据卷
docker run --rm -v docsy-site_caddy_data:/data -v $(pwd):/backup alpine tar xzf /backup/caddy_data_backup.tar.gz -C /
```

## 🛠️ 故障排除

### 1. SSL证书申请失败

**问题**: Caddy无法申请SSL证书

**解决方案**:
- 确保域名A记录正确解析到服务器IP
- 确保80和443端口已开放
- 检查防火墙设置

### 2. 网站无法访问

**问题**: 浏览器无法打开网站

**解决方案**:
```bash
# 检查容器状态
docker-compose ps

# 检查端口监听
netstat -tlnp | grep -E ':80|:443'

# 检查Caddy配置
docker exec docsy-site caddy validate --config /etc/caddy/Caddyfile
```

### 3. 下载文件404错误

**问题**: 下载链接返回404

**解决方案**:
- 确保`public/downloads/`目录包含对应文件
- 运行同步脚本: `./scripts/sync-releases.sh`

## 📦 目录结构

```
site/
├── Caddyfile                # Caddy服务器配置
├── docker-compose.yml       # Docker Compose配置
├── public/                  # 网站静态文件
│   ├── index.html           # 主页
│   ├── features.html        # 功能详情页
│   ├── css/                 # 样式文件
│   ├── js/                  # 脚本文件
│   ├── assets/              # 图片等资源
│   ├── data/                # 数据文件
│   └── downloads/           # 下载文件
├── scripts/                 # 同步脚本
└── README.md                # 项目说明
```

## 🔄 自动同步

### GitHub Actions 自动推送

每次发布新版本时，GitHub Actions会自动同步到NAS。

### NAS定时拉取

```bash
# 添加定时任务（每小时检查一次）
10 * * * * cd /path/to/docsy-site && ./scripts/sync-releases.sh >> sync.log 2>&1
```

## 🌐 多环境部署

### 极空间NAS

1. 打开Docker应用
2. 新建项目，粘贴`docker-compose.yml`内容
3. 启动项目

### 群晖NAS

1. 打开Container Manager
2. 新建项目，导入`docker-compose.yml`
3. 启动项目

### 云服务器

1. 安装Docker和Docker Compose
2. 上传site目录
3. 执行`docker-compose up -d`

## 📊 监控与维护

### 查看资源使用

```bash
# 查看容器资源使用
docker stats docsy-site

# 查看磁盘使用
docker system df
```

### 日志管理

```bash
# 查看实时日志
docker-compose logs -f

# 查看最近100行日志
docker-compose logs --tail=100
```

### 性能优化

1. **启用Gzip压缩**: 已在Caddyfile中配置
2. **缓存策略**: 静态资源缓存，数据文件不缓存
3. **安全头**: 已配置基本安全响应头

## 🆘 获取帮助

- GitHub Issues: https://github.com/muxiaoxiii/docsy/issues
- 查看日志: `docker-compose logs -f`
- 检查配置: `docker exec docsy-site caddy validate --config /etc/caddy/Caddyfile`

## 📝 更新记录

- **2024-01-20**: 初始版本，支持基本部署
- **2024-01-21**: 添加功能详情页面，优化截图展示