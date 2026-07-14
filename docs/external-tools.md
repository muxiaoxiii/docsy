# 外部工具安装与分发

Docsy 的主程序保持轻量。PDF 和视频相关能力按需调用外部命令行工具，用户可以在“设置 -> 外部工具状态”里检查和安装。

## 工具目录

托管安装的工具放在系统应用数据目录下：

- macOS: `~/Library/Application Support/Docsy/tools/`
- Windows: `%APPDATA%\Docsy\tools\`

检测顺序是：

1. Docsy 托管工具目录。
2. macOS 常见系统路径，例如 `/opt/homebrew/bin` 和 `/usr/local/bin`。
3. 系统 `PATH`，macOS 使用 `which`，Windows 使用 `where`。

## 自动安装范围

当前可自动下载安装到 Docsy 工具目录的工具：

- `qpdf`：PDF 解锁、合并、拆分、overlay 和结构检查。
- `poppler`：`pdftoppm`、`pdftotext`，用于 PDF 预览渲染和文本检测。
- `ffmpeg`：视频探测、抽帧和时间戳水印。

`LibreOffice` 不纳入托管安装。它体积大、平台安装器和系统集成差异明显，Docsy 只提供检测、官方下载入口和路径配置。

## 下载清单

安装器优先读取远程清单：

```text
https://github.com/muxiaoxiii/docsy/releases/download/toolchain-v1/tools-manifest.json
```

开发和内测时可以用环境变量覆盖：

```text
DOCSY_TOOL_MANIFEST_URL
```

清单格式：

```json
{
  "tools": {
    "qpdf": {
      "windows-x86_64": {
        "version": "12.3.2",
        "url": "https://example.com/qpdf.zip",
        "sha256": "...",
        "binaries": ["qpdf.exe"]
      }
    }
  }
}
```

如果清单不可用，Windows x64 会回退到公开 zip 包：

- qpdf: qpdf 官方 `msvc64.zip`
- FFmpeg: Gyan.dev `ffmpeg-release-essentials.zip`
- Poppler: `oschwartz10612/poppler-windows` release zip

macOS 没有同等稳定、轻量、覆盖 qpdf/Poppler 的官方静态 zip 组合，因此 macOS 自动安装依赖 Docsy 发布的 `toolchain-v1` 工具包。工具包发布前，系统已安装版本仍可被检测和复用。

## Windows 打包

Windows 桌面安装包应在 Windows runner 或 Windows 机器上构建。macOS 本机可以检查大部分 Rust/Vue 代码，但不能可靠完成 `x86_64-pc-windows-msvc` 的原生依赖编译和 Tauri 打包。

仓库已提供 `.github/workflows/build-desktop.yml`，tag 或手动触发后会分别产出 macOS 和 Windows 桌面包。
