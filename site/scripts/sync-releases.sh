#!/usr/bin/env bash
# Docsy 下载站 NAS 定时同步脚本（兜底方案）
#
# 从 GitHub Releases 拉取新版本安装包并刷新站点数据。
# 主同步通道是 GitHub Actions（发布时自动推送），本脚本用于 NAS 侧兜底，
# 两者都配置时以 Actions 为准，重复下载会自动跳过（同名同大小不覆盖）。
#
# 用法（在 site/ 目录下执行）：
#   ./scripts/sync-releases.sh
#
# 可选环境变量：
#   GH_PROXY   GitHub 下载代理前缀。国内 NAS 直连 GitHub 慢时可设为
#              例如 https://ghproxy.net/ （仅用于下载安装包，不影响 API）
#
# 定时任务示例（crontab，每小时检查一次）：
#   10 * * * * cd /volume1/docker/docsy-site && ./scripts/sync-releases.sh >> sync.log 2>&1

set -euo pipefail

REPO="muxiaoxiii/docsy"
BRANCH="v0.9-mdg"
SITE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PUBLIC_DIR="$SITE_DIR/public"
DOWNLOAD_DIR="$PUBLIC_DIR/downloads"
DATA_DIR="$PUBLIC_DIR/data"
API_URL="https://api.github.com/repos/$REPO/releases?per_page=12"
CHANGELOG_URL="https://raw.githubusercontent.com/$REPO/$BRANCH/CHANGELOG.md"
PROXY="${GH_PROXY:-}"

mkdir -p "$DOWNLOAD_DIR" "$DATA_DIR"

echo "[sync] $(date '+%Y-%m-%d %H:%M:%S') 开始同步 $REPO"

TMP_JSON="$(mktemp)"
trap 'rm -f "$TMP_JSON"' EXIT

if ! curl -fsSL --connect-timeout 20 "$API_URL" -o "$TMP_JSON"; then
  echo "[sync] 获取 GitHub Releases 列表失败（网络问题？可设置 GH_PROXY 代理）"
  exit 1
fi

# 更新日志直接从 GitHub 拉取，与代码库保持同步
TMP_CHANGELOG="$(mktemp)"
trap 'rm -f "$TMP_JSON" "$TMP_CHANGELOG"' EXIT
if ! curl -fsSL --connect-timeout 20 "$CHANGELOG_URL" -o "$TMP_CHANGELOG"; then
  echo "[sync] 获取 CHANGELOG.md 失败，跳过更新日志刷新"
  TMP_CHANGELOG="$SITE_DIR/../CHANGELOG.md"
  if [ ! -f "$TMP_CHANGELOG" ]; then
    echo "[sync] 本地也无 CHANGELOG.md，终止"
    exit 1
  fi
fi

python3 "$SITE_DIR/scripts/build-data.py" \
  --api-json "$TMP_JSON" \
  --assets-dir "$DOWNLOAD_DIR" \
  --changelog "$TMP_CHANGELOG" \
  --out "$DATA_DIR" \
  --proxy "$PROXY"

echo "[sync] 完成"
