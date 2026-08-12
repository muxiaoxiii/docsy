# PDF 转 Markdown 设计

## 目标

`MD转换` 处理 Markdown 与 Office 的双向转换，也负责 PDF 到 Markdown：

- 可复制文本 PDF 快速、本地、无模型地转换；
- 扫描件、表格、公式、多栏和印章等复杂页面可选用本地 AI 解析；
- 所有内容只在本机处理，不上传证据文件；
- 大型证据包按页段执行、持续报告进度，并允许用户主动取消；
- Markdown、图片和中间结果具有明确的输出目录，不覆盖原文件。

PDF 转 Markdown **不进入证据处理、页眉页脚、A4 规范化或合并拆分链路**。这些链路必须保持确定性、低内存和可审计；文档理解结果只服务于阅读、检索、摘录和后续 Office 转换。

## 分层

### 1. 文本层提取（内置，默认）

使用现有 Poppler 的 `pdftotext -layout` 提取 PDF 已有文本层，按页段输出为 Markdown。

- 不 OCR，不重采样，不改变原 PDF；
- 速度快，内存只保存当前任务的文本输出；
- 保留页码分隔，方便核对原始页；
- 文本层为空时明确提示改用 AI 解析，而不是伪造文本；
- 阅读顺序、表格单元格和图片语义只能尽力保留，结果会标注为“文本层提取”。

### 2. AI 文档解析（可选、托管、本地）

首个引擎定为 **Docsy Paddle 文档解析包**，以 PaddleOCR / PP-StructureV3 的 Apache-2.0 源码为基础裁剪和改造，封装为独立 worker。

它负责：

- OCR、版面区块和阅读顺序；
- 表格转 HTML/Markdown、公式转 LaTex；
- 图片资产导出及相对路径引用；
- 页级 JSON 清单、置信度和警告，供 Docsy 展示和重跑失败页。

主安装包不携带 Python、模型或 GPU 依赖。用户在需要时从设置或 MD 转换页下载对应平台的 AI 包；包安装到 `Docsy/tools/pdf-md-paddle`，可离线使用、可移除、可手动导入。模型下载、解压和运行均在后台任务中进行，不阻塞窗口。

### 3. MinerU（后续可选引擎）

MinerU 适用于非常复杂的论文、跨页表格、公式和图表。它与 Paddle 使用同一 worker 协议，但不作为首发引擎：当前上游采用基于 Apache-2.0 且附加条件的 MinerU Open Source License，接入前必须单独完成许可证、模型分发和各平台运行时审查。

不允许把 MinerU 作为云端 API 的静默回退；只有用户明确安装并选择本地 MinerU 包时才调用。

## Worker 协议

Docsy 不直接把 Python 包、模型细节或厂商 CLI 散落在 Rust 业务层。每个 AI 引擎实现同一个行式 JSON 协议：

```text
stdin  : {"protocol":1,"inputPath":"...","outputDir":"...","pages":{"start":1,"end":50},"options":{"tables":true,"formulas":true,"images":true}}
stdout : {"event":"progress","page":12,"totalPages":50,"stage":"layout"}
stdout : {"event":"result","markdownPath":"...","assetsDir":"...","manifestPath":"...","warnings":[]}
stderr : 诊断日志（只写入应用日志，不作为普通用户提示全文展示）
```

Rust 侧职责：校验输入、创建唯一输出目录、启动隐藏子进程、注册取消 PID、解析进度、限制日志长度、检查结果清单，并将路径返回前端。worker 侧职责：逐页处理、可恢复输出、模型推理和格式转换。

结果清单为 `result.json`：

```json
{
  "protocol": 1,
  "engine": "paddle",
  "engine_version": "...",
  "input": "source.pdf",
  "pages": [{"page": 1, "status": "done", "text_layer": false, "warnings": []}],
  "markdown_path": "document.md",
  "assets_dir": "document_assets",
  "warnings": []
}
```

## UI 与任务行为

PDF 被拖入 `MD转换` 后显示两种方法：

| 方法 | 适用文件 | 依赖 | 默认 |
| --- | --- | --- | --- |
| 文本层提取 | 可复制文字的 PDF | Poppler | 是 |
| AI 文档解析 | 扫描件、表格、公式、复杂排版 | 可选 Paddle 包 | 否 |

任务卡显示输入类型、页段、当前阶段、已完成页数、警告和输出文件夹。大文件没有固定超时；只有用户点击取消才停止。失败页可按页段重新执行，不重新扫描已完成页面。

AI 输出采用单独目录：`<文件名>_docsy_md/`，其中包含 `.md`、`assets/`、`result.json` 和每页中间结果；文本层输出默认仍放在源文件同目录，并自动避让重名。

## 质量与安全边界

- 先检测文本层；有可靠文本层时不为“更漂亮”而无提示 OCR，避免识别错误改写证据文字；
- AI 输出始终标识为识别结果，不能作为 PDF 原文替代品；
- 禁止上传文件、禁止隐式联网调用云 API；
- worker 下载包必须使用 HTTPS、SHA-256、流式解压、单文件和总解压限额；
- 每页结果持久化，禁止将完整 2,000 页 PDF 的页面图、模型输入或 Markdown 同时放入内存；
- 主程序只认协议清单与相对路径，禁止 worker 返回任意路径覆盖源文件。

## 实施顺序

1. ✅ 内置文本层 PDF→MD：类型识别、页段、输出、测试（已完成，见下节）；
2. 建立 `pdf_md` worker 协议、操作进度与取消桥接；
3. 将裁剪后的 Paddle 适配器以 `vendor/docsy-paddle-md` 维护，制作 macOS / Windows 可验证工具包；
4. 接入 Paddle 包安装、状态、手动导入和 AI 模式；
5. 以同一协议评估 MinerU 包，并在许可证和平台验证完成后作为高级引擎提供。

## 实施要点（第 1 步已完成）

- 命令：Tauri 命令 `convert_pdf_text_layer`，参数 `input` / `outputDir` / `startPage` / `endPage`
  （Rust 侧 snake_case，Tauri 自动对接 camelCase；页段为 1-based 闭区间，留空 = 全文）。
- 文本层提取引擎：Poppler `pdftotext -layout -enc UTF-8`，页段经 `-f N` / `-l M` 传给
  pdftotext 自身按页段执行；子进程经 `SubprocessRegistry` 注册（op_id 形如
  `pdf_text_layer:<pid>`），前端 `cancel_operation` 可终止。
- 空页约定：按 `\x0c` 分页保留所有页，无文本层的页在 Markdown 中写占位提示并计入
  `empty_pages`（原始页码）；返回 `warning` 中含“第 N 页无文本层，可能为扫描件，
  建议后续用 AI 文档解析包处理”。仅当范围内全部页为空时报错。
- 分页标记：`## 第 N 页` 标题，有页段时 N 保持原始页码（start_page 偏移）。
- 输出命名避让：`unique_output_path`，默认源文件同目录、同 stem 换 `.md`，撞名自动加序号。
- 取消与操作管理：命令走 `run_managed`（`OperationManager` CancellationToken），
  子进程运行前检查 `token.is_cancelled()`。
- Office→MD 已接入格式（经 vendored AnyDoc）：doc / docx / docm / xls / xlsx / xlsm /
  xlsb / ppt / pptx / pptm / pps / ppsx / ppsm / pot / potx / potm / odt / ods / odp /
  rtf / csv / epub。其中 docx 优先走自研 `docx_to_md`（AnyDoc 兜底）；`.doc` 维持双引擎
  （默认 `extract` = AnyDoc 直读，文字/标题/表格/列表保真度实测良好；可选 `word` =
  本机 Word/WPS 中转），由前端弹窗选择。
- AI 文档解析选项：当前 UI 仅有禁用占位提示，尚未接入 worker。
