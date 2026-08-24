#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Docsy 下载站数据生成脚本。

从 GitHub Releases API 数据 + CHANGELOG.md 生成站点数据文件
（releases.json / changelog.json），并补齐缺失的安装包文件。

GitHub Actions 与 NAS 定时任务共用本脚本，保证两份数据格式一致。

用法:
    build-data.py --api-json <releases.json> --assets-dir <dir> \
        --changelog <CHANGELOG.md> --out <data_dir> [--proxy <prefix>]

参数:
    --api-json     GitHub Releases API 返回的 JSON（gh 或 curl 均可）
    --assets-dir   安装包存放目录（缺失文件会被下载到这里）
    --changelog    CHANGELOG.md 路径
    --out          输出目录（写入 releases.json 与 changelog.json）
    --proxy        可选：GitHub 下载代理前缀（如 https://ghproxy.net/），
                   仅影响下载，不影响 API 数据本身
"""

import argparse
import hashlib
import json
import re
import sys
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO = "muxiaoxiii/docsy"
GITHUB_RELEASES_URL = f"https://github.com/{REPO}/releases"
API_JSON_VERSION = 2


def log(message: str) -> None:
    print(f"[build-data] {message}", file=sys.stderr)


def download_file(url: str, target: Path, proxy: str = "") -> bool:
    """下载 url 到 target；返回是否真正下载了文件。"""
    if proxy:
        url = proxy + url
    log(f"下载 {url}")
    request = urllib.request.Request(
        url, headers={"User-Agent": "docsy-mirror-sync/1.0"}
    )
    with urllib.request.urlopen(request, timeout=120) as response, open(
        target, "wb"
    ) as out:
        out.write(response.read())
    return True


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 256), b""):
            digest.update(chunk)
    return digest.hexdigest()


def platform_of(name: str) -> str:
    """根据文件名判断平台与架构。"""
    lower = name.lower()
    if lower.endswith(".dmg"):
        if "intel" in lower or "x86_64" in lower:
            return "macos-intel"
        return "macos"
    if lower.endswith(".exe"):
        return "windows"
    if lower.endswith(".deb"):
        return "linux-deb"
    if lower.endswith(".appimage"):
        return "linux-appimage"
    return "other"


def parse_changelog(text: str):
    """解析 Keep a Changelog 格式，返回条目列表。

    支持 ## [版本] - 日期 / ### 小节 / - 条目 三层结构。
    """
    entries = []
    current = None
    section = None
    for raw in text.splitlines():
        line = raw.rstrip()
        version_match = re.match(r"^##\s+\[([^\]]+)\]\s*-\s*([\d-]+)", line)
        if version_match:
            if current:
                entries.append(current)
            current = {
                "version": version_match.group(1).strip(),
                "date": version_match.group(2).strip(),
                "sections": [],
            }
            section = None
            continue
        if current is None:
            continue
        section_match = re.match(r"^###\s+(.+)$", line)
        if section_match:
            section = {"title": section_match.group(1).strip(), "items": []}
            current["sections"].append(section)
            continue
        item_match = re.match(r"^-\s+(.+)$", line)
        if item_match and section is not None:
            section["items"].append(item_match.group(1).strip())
    if current:
        entries.append(current)
    return entries


def build_releases(api_json, assets_dir: Path, proxy: str) -> list:
    """从 API 数据构建 releases 列表；同时补下载缺失的安装包。"""
    releases = []
    for release in api_json:
        tag = release.get("tag_name", "")
        version = tag.lstrip("v") if tag else (release.get("name") or "")
        published = release.get("published_at", "")
        assets = []
        for asset in release.get("assets", []):
            name = asset.get("name", "")
            if not name or not name.lower().startswith("docsy"):
                continue
            size = int(asset.get("size") or 0)
            local = assets_dir / name
            if not local.exists() or local.stat().st_size != size:
                # 只有最新版本自动补下载，旧版本交给手动/全量同步
                is_latest = not releases
                if is_latest and asset.get("browser_download_url"):
                    download_file(asset["browser_download_url"], local, proxy)
            if local.exists():
                assets.append(
                    {
                        "platform": platform_of(name),
                        "name": name,
                        "size": local.stat().st_size,
                        "sha256": sha256_of(local),
                        "url": f"/downloads/{name}",
                        "github_url": asset.get(
                            "browser_download_url",
                            f"{GITHUB_RELEASES_URL}/download/{tag}/{name}",
                        ),
                    }
                )
        if assets:
            releases.append(
                {
                    "version": version,
                    "tag": tag,
                    "published_at": published,
                    "assets": assets,
                }
            )
    return releases


# ============================================================
# 外部工具数据（与 app 内 src-tauri/src/external/managed.rs 保持一致）
# 版本/校验值来自 app 内置工具清单；官方地址来自各项目官方 GitHub Releases。
# ============================================================

# ghproxy 系列镜像前缀（国内加速 GitHub 下载，与 app 内置 mirrors 一致）
GH_MIRROR_PREFIXES = [
    "https://gh-proxy.com/",
    "https://ghfast.top/",
    "https://gh-proxy.net/",
]

# 各工具在 docsy 中的用途（与设置页 tools 列表一致）
TOOL_PURPOSE = {
    "qpdf": "PDF 合并、拆分、叠加和结构处理",
    "poppler": "PDF 预览渲染和页眉页脚文本检测",
    "ffmpeg": "视频信息读取、抽帧和时间戳水印",
}

# macOS 无官方预编译包，官方推荐 Homebrew
MACOS_BREW = {
    "qpdf": {
        "command": "brew install qpdf",
        "url": "https://formulae.brew.sh/formula/qpdf",
        "note": "推荐方式：Docsy 会自动检测系统已安装的 qpdf",
    },
    "poppler": {
        "command": "brew install poppler",
        "url": "https://formulae.brew.sh/formula/poppler",
        "note": "推荐方式：Docsy 会自动检测系统已安装的 Poppler",
    },
    "ffmpeg": {
        "command": "brew install ffmpeg",
        "url": "https://formulae.brew.sh/formula/ffmpeg",
        "note": "时间戳水印需要 drawtext 滤镜，Homebrew 官方 ffmpeg 已包含；Docsy 会自动检测",
    },
}

# Windows 官方包（与 managed.rs embedded_windows_package_spec 一致）
WINDOWS_PACKAGE = {
    "qpdf": {
        "version": "12.3.2",
        "file": "qpdf-12.3.2-msvc64.zip",
        "size": 24536601,
        "sha256": "8941870a604e7c87ed24566b038d46c24ce76616254d2383c578f60c0677f202",
        "url": "https://github.com/qpdf/qpdf/releases/download/v12.3.2/qpdf-12.3.2-msvc64.zip",
        "page": "https://github.com/qpdf/qpdf/releases",
        "note": "官方 msvc64 压缩包；也可以在 Docsy 设置页直接「下载安装到 Docsy」自动安装",
    },
    "poppler": {
        "version": "26.02.0-0",
        "file": "Release-26.02.0-0.zip",
        "size": 16138240,
        "sha256": "993e4a94376ed712fafc7058d724ea0b943d118bbd2305cd9ed55174eb85cda5",
        "url": "https://github.com/oschwartz10612/poppler-windows/releases/download/v26.02.0-0/Release-26.02.0-0.zip",
        "page": "https://github.com/oschwartz10612/poppler-windows/releases",
        "note": "Poppler 社区 Windows 打包（官方推荐）；也可以让 Docsy 自动安装",
    },
    "ffmpeg": {
        "version": "9.0（master 滚动版）",
        "file": "ffmpeg-master-latest-win64-gpl.zip",
        "size": 170641785,
        "sha256": "",
        "url": "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
        "page": "https://github.com/BtbN/FFmpeg-Builds/releases",
        "note": "BtbN 官方构建（GPL 版，含 drawtext 滤镜）；滚动更新无固定校验值，也可以让 Docsy 自动安装",
    },
}

TOOL_ORDER = ["qpdf", "poppler", "ffmpeg"]


def build_tools() -> dict:
    """生成外部工具数据：工具 × 系统 → 版本/官方地址/镜像/说明。"""
    tools = {}
    for name in TOOL_ORDER:
        brew = MACOS_BREW[name]
        pkg = WINDOWS_PACKAGE[name]
        entries = [
            {
                "platform": "macos",
                "os": "macOS（Apple 芯片 / Intel）",
                "type": "brew",
                "version": "Homebrew 最新版",
                "command": brew["command"],
                "url": brew["url"],
                "size": None,
                "sha256": "",
                "mirror_url": "",
                "note": brew["note"],
            },
            {
                "platform": "windows",
                "os": "Windows 10 / 11（64 位）",
                "type": "zip",
                "version": pkg["version"],
                "command": "",
                "url": pkg["url"],
                "size": pkg["size"],
                "sha256": pkg["sha256"],
                "mirror_url": GH_MIRROR_PREFIXES[0] + pkg["url"],
                "note": pkg["note"],
            },
        ]
        tools[name] = {
            "label": name,
            "purpose": TOOL_PURPOSE[name],
            "page": pkg["page"],
            "entries": entries,
        }
    return tools


def main() -> int:
    parser = argparse.ArgumentParser(description="生成 Docsy 下载站数据")
    parser.add_argument("--api-json", required=True, help="GitHub Releases API JSON 文件")
    parser.add_argument("--assets-dir", required=True, help="安装包目录")
    parser.add_argument("--changelog", required=True, help="CHANGELOG.md 路径")
    parser.add_argument("--out", required=True, help="输出目录")
    parser.add_argument("--proxy", default="", help="GitHub 下载代理前缀（可选）")
    args = parser.parse_args()

    assets_dir = Path(args.assets_dir)
    out_dir = Path(args.out)
    assets_dir.mkdir(parents=True, exist_ok=True)
    out_dir.mkdir(parents=True, exist_ok=True)

    with open(args.api_json, encoding="utf-8") as f:
        api_json = json.load(f)
    if isinstance(api_json, dict):
        api_json = api_json.get("releases", [api_json])

    releases = build_releases(api_json, assets_dir, args.proxy)

    with open(args.changelog, encoding="utf-8") as f:
        changelog_entries = parse_changelog(f.read())

    releases_data = {
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "github_url": GITHUB_RELEASES_URL,
        "latest": releases[0]["version"] if releases else None,
        "releases": releases,
    }
    (out_dir / "releases.json").write_text(
        json.dumps(releases_data, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    (out_dir / "changelog.json").write_text(
        json.dumps(changelog_entries, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    tools_data = {
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "github_url": GITHUB_RELEASES_URL,
        "tools": build_tools(),
    }
    (out_dir / "tools.json").write_text(
        json.dumps(tools_data, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    log(
        f"完成：{len(releases)} 个版本，{sum(len(r['assets']) for r in releases)} 个安装包，"
        f"{len(changelog_entries)} 条更新日志，{len(tools_data['tools'])} 个外部工具"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
