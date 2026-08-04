# Docsy PDF OCR 模块设计（扫描件 → 可搜索 PDF）

更新时间：2026-08-04（v0.9.2）

> 前置调研：`~/research/pdf-ocr-rust/REPORT.md`（PaddleOCR Rust 生态与内存、MinerU 本地化可行性、反 OCR 机制，36 个来源，位于仓库外）。

## 目标

把扫描件/图片型 PDF 转为**文本型（可搜索、可复制、可选中）PDF**，全部本地离线处理，不上传任何文件。与证据处理链路衔接：扫描件证据在合并输出前补一层 OCR 文本层，提升可检索性。

第一阶段**只做"全文 OCR 文本层"**，不做版面分析、表格/公式识别、阅读顺序重排——那是 MinerU VLM 路线的领域，本模块保留后续升级空间。

核心流程：

```text
选择 PDF
→ 检查是否纯扫描件（已有文本层则提示/跳过）
→ 逐页栅格化（Poppler pdftoppm）
→ 图像预处理（灰度/可选二值化）
→ PP-OCRv6 det/cls/rec（ONNX + onnxruntime-rs）
→ 聚合识别文本与包围盒坐标
→ harumi 写入不可见文本层（Tr 3 + ToUnicode CMap）
→ 另存副本输出（不覆盖原文件）
```

## 架构边界

- **不引入 Python 运行时**：不走 OCRmyPDF/MinerU 子进程路线（内存需求 RAM 16GB+、分发体积大）。
- **不重写解析算法**：MinerU 的 PDF 解析本就委托 MuPDF/PDFium，模型是 ONNX 格式可被 `ort` crate 直接加载；本模块复用其 ONNX 模型 + Rust 自写轻量管线（调研结论，见 REPORT §三）。
- **复用现有能力**：Poppler pdftoppm（`pdf/preview.rs` 已有单页渲染）、外部工具托管下载（`external/managed.rs`）、文件队列/工作台组件（`ToolWorkspaceShell`/`FileQueuePanel`）、tauriBridge 错误与动画事件。
- 输出遵循项目惯例：**一律另存副本，不覆盖原文件**。

## 技术选型（路线 A：纯 Rust 拼装）

| 组件 | 选型 | 理由 | 许可证 |
| --- | --- | --- | --- |
| 页面栅格化 | Poppler `pdftoppm`（已有） | 证据链路已集成，按需渲染、不整份读入内存 | 系统工具 |
| OCR 引擎 | **PP-OCRv6 ONNX**（RapidAI 统一转换，ModelScope 托管） + **paddle-ocr-rs**（ort crate） | 中文精度高；纯 CPU 零显存；Apache-2.0；v6 tiny 仅 1.5M 参数 | Apache-2.0 |
| 文本层回写 | **harumi** `add_invisible_text` | 纯 Rust、零 C 依赖、自动 CJK 字体子集化 + CID/ToUnicode CMap，支持 Tauri | MIT OR Apache-2.0 |
| 模型分发 | 设置页托管下载（复用 `external/managed.rs`） | 与 qpdf/poppler/ffmpeg 一致的用户路径 | - |

备选（不推荐为首选）：

- `ocrs`（纯 Rust OCR，rten 推理）：英文好、中文一般，适合无中文场景。
- `leptess`（Tesseract 绑定）：v0.14.0 自 2023-02 未更新，需随应用分发系统动态库与 chi_sim traineddata。
- OCRmyPDF 子进程：事实标准但依赖 Python + Tesseract，仅适合快速验证。
- MinerU VLM 路线：需要表格/公式/版面时再评估（显存 8GB 起）。

## 内存与性能预算

- 模型权重：PP-OCRv6 tiny/small/medium 参数 1.5M / 7.7M / 34.5M（ONNX 体积约 6MB / 30MB / 140MB，估算）[1]。
- 进程峰值目标 **< 500MB**：参考社区实测（Mac Mini M4，MNN 后端）完整管线峰值 388–422MB——模型权重仅占 20–25MB，大头是整页图像处理 [2]。**按页流式处理（渲染→OCR→释放）**，2000 页大 PDF 峰值保持稳定。
- 显存：0（纯 CPU 推理）。
- 单页 CPU 延迟目标 < 2s：参考 v6 相对 v5 的 CPU 端到端 5.2× 加速、Apple M4 上 6.1×（tiny）[1]。
- 并发：默认 1 个推理会话、单页串行；不引入多线程推理，控制峰值。

> 注：ort 路径尚无公开内存基准（调研 Open Question 1），原型阶段需实测确认。

## 代码结构

### 后端 `src-tauri/src/ocr/`

| 文件 | 职责 |
| --- | --- |
| `mod.rs` | 命令入口与数据结构（OcrJob / OcrResult / OcrPageResult） |
| `pipeline.rs` | 页面级管线：渲染 → 预处理 → det/cls/rec → 文本块聚合 |
| `models.rs` | ONNX 模型加载与会话管理（ort），模型文件校验/版本 |
| `textlayer.rs` | harumi 文本层回写封装（坐标 → 不可见文本） |
| `progress.rs` | 进度上报与取消（接线 `SubprocessRegistry`，修复现有取消失效问题） |

命令域 `commands/ocr.rs` 注册进 `commands/mod.rs`，建议命令：

- `check_ocr_models`：检测模型是否已下载（引导到设置页）
- `ocr_pdf_to_searchable`：单文件/批量执行，返回每页结果与输出路径
- `cancel_operation` 复用（配合 progress.rs 接线）

### 前端 `src/modules/pdf-ocr/`

- `index.js`：模块注册（路由/菜单/首页卡片），复用 moduleRegistry 自动发现
- `OcrWorkbench.vue`：工作台（文件队列、模型状态、参数、进度、结果列表、打开输出）
- `composables/useOcrProgress.js`：进度轮询/事件订阅
- 复用 `ToolWorkspaceShell`、`FileQueuePanel`（可排序）、`tauriCallSafe`

### 设置页

- 「OCR 模型」分组：PP-OCRv6 det/cls/rec ONNX 下载、版本、清除（复用工具清单与托管安装机制）
- 诊断信息：模型加载状态、最近一次 OCR 统计

## 关键设计决策

1. **文本层只写不可见文本（渲染模式 3），不改原图**：视觉零变化，可搜索可复制；失败页保留原样并标记，不中断整批。
2. **"已有文本"检测**：处理前用结构特征（Tr 3 / 透明度 / 零字号 / 几何位置）判断是否已含文本层或正文文本；已含文本则提示并跳过，提供"强制栅格化后重 OCR"选项（对应 OCRmyPDF `--force-ocr` 语义）[3]。注意：**仅凭提取文本判断会被 Unicode 混淆/隐藏文本层绕过**，需结合渲染状态特征（调研结论，CrackedPDFs F1=0.960 vs 纯文本检测失败）[4]。
3. **进度与取消**：OCR 任务注册进 `SubprocessRegistry`（本次审查 H1 发现的取消失效问题一并修复），前端取消真正终止推理会话。
4. **模型缺失降级**：未下载模型时给出明确引导，不静默失败。
5. **输出命名**：`原文件名_ocr.pdf` 放同目录，冲突时 `unique_*_path` 去重（沿用项目惯例）。

## 与现有模块的关系

- **evidence-pdf**：证据合并输出前，可选对扫描件执行 OCR 生成文本层，提升最终 PDF 可检索性；`detect_pdf_header_footer` 对"页面只有图片"的扫描件可标记为"建议 OCR"（与证据设计文档 161 行"OCR 作为未来可选能力"衔接）。
- **pdf-tools**：拆分/提取结果页可单选"OCR 转文本型"。
- **settings**：模型托管与诊断。

## 反 OCR 输入的处理（第二阶段可选）

调研结论（REPORT §四）为后续功能埋点：

- **隐藏文本层检测**：基于结构特征检测 PDF 是否含不可见文本层，供"是否纯扫描件"判断与证据真实性提示使用。
- **对抗输入提示**：OCR 结果与可见层显著不一致（被 Unicode 混淆/对抗扰动）时提示用户，避免把被篡改文本当作正文。
- **不做**：生成"防 OCR"输出。文本层欺骗只影响阅读器元数据、骗不过 OCR；真正的 OCR 对抗需要图像空间扰动，与"生成可搜索 PDF"的目标互斥，且存在合规风险，不纳入本模块。

## 验收标准

1. 中文扫描件 → 生成副本：在 Foxit/Chrome/系统预览中**搜索命中、可复制、视觉与原件一致**。
2. 2000 页扫描 PDF：峰值 RSS < 600MB，处理中可取消（取消真正生效）。
3. 纯扫描件、已含文本层 PDF、加密 PDF 三种输入行为正确（提示/跳过/解锁引导）。
4. 模型缺失时引导到设置页下载，下载完成后可一键重试。
5. 单页 OCR 失败不中断整批，结果列表明确标记失败页。

## 风险与未决问题

| 风险 | 说明 | 缓解 |
| --- | --- | --- |
| ort 内存无官方基准 | 峰值内存为 MNN 后端实测推断 | 原型阶段实测 RSS 增量 |
| PP-OCRv6 ONNX 可用性 | RapidAI 已提供 v6 转换，中文实测精度待验证 | 原型验证 + tiny/small/medium 三档对比 |
| harumi 中文兼容性 | 中文不可见文本层在主流阅读器搜索/复制兼容性未实测 | 原型验证（验收标准 1） |
| 模型分发体积 | medium 档 ONNX 约 140MB（估算） | 默认 tiny/small，设置页可选 |
| 法律文书特殊版式 | 印章/手写/骑缝章区域 OCR 噪声 | 检测结果保留原图，仅叠加文本层，不破坏原件 |

## 参考

- [1] PaddleOCR 官方仓库（PP-OCRv6 发布说明）— https://github.com/PaddlePaddle/PaddleOCR （2026-06-11）
- [2] rust-paddle-ocr 模型版本与内存对比（社区实测）— https://deepwiki.com/zibo-chen/rust-paddle-ocr/5.1-model-versions-and-comparison
- [3] OCRmyPDF errors 文档（--force-ocr / --redo-ocr）— https://ocrmypdf.readthedocs.io/en/latest/errors.html
- [4] CrackedPDFs 基准（隐藏文本层需结构特征检测）— https://arxiv.org/abs/2607.19396
- 完整调研：`~/research/pdf-ocr-rust/REPORT.md`（36 个来源，含 harumi / paddle-ocr-rs / MinerU 架构细节）
