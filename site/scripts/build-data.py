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

    log(
        f"完成：{len(releases)} 个版本，{sum(len(r['assets']) for r in releases)} 个安装包，"
        f"{len(changelog_entries)} 条更新日志"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
