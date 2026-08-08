# Docsy PDF 解析/OCR 模块设计（扫描件 → 文本型 PDF + Markdown/JSON 知识库输出）

更新时间：2026-08-04（v0.9.3 修订：纳入 LiteParse 借鉴与三输出形态）

> 前置调研：`~/research/pdf-ocr-rust/REPORT.md`（PaddleOCR Rust 生态与内存、MinerU 本地化可行性、反 OCR 机制，36 个来源，位于仓库外）。

## 目标

把扫描件/图片型 PDF 转为**文本型（可搜索、可复制、可选中）PDF**，同时提供 **Markdown / JSON 两种解析输出**供 AI 处理与知识库读取，全部本地离线处理，不上传任何文件。

一个入口，三种产物：

```text
输入 PDF ──→ ① 带文本层的 PDF（另存副本，可搜索复制）
          ──→ ② Markdown（结构化：标题/表格/列表，喂 AI/RAG/知识库）
          ──→ ③ JSON（文本 + 词级 bbox + 元数据，喂程序/向量化）
```

与证据处理链路衔接：扫描件证据在合并输出前补一层 OCR 文本层，提升可检索性；解析产物可供 Docsy 与 Casy 的文档知识库使用。

核心流程：

```text
选择 PDF
→ 复杂度检测（is-complex 语义：文本层有无/稀疏/乱码 → 路由）
→ 文本型 PDF：pdftotext -bbox 直接提取（秒级，跳过 OCR）
→ 扫描件：逐页栅格化（Poppler pdftoppm）→ 预处理 → PP-OCRv6 det/cls/rec
→ 词块聚合（文本 + bbox）
→ 网格投影布局重建（移植 LiteParse PROJ 算法，阅读顺序/行列）
→ 分叉输出：
    ├─ Markdown（移植 LiteParse 渲染启发式：标题/表格/列表）
    ├─ JSON（文本 + bbox + 页元数据）
    └─ harumi 写入不可见文本层（Tr 3 + ToUnicode CMap）→ 可搜索 PDF 副本
```

不做版面分析/表格/公式识别/手写识别的高级结构化（MinerU VLM 路线领域，本模块保留后续升级空间；LiteParse 的 Markdown 重建是启发式规则，复杂版面质量有限，符合本模块"知识库易读"的定位）。

## 架构边界

- **不引入 Python 运行时**：不走 OCRmyPDF/MinerU 子进程路线（内存需求 RAM 16GB+、分发体积大）。
- **不重写解析算法**：MinerU 的 PDF 解析本就委托 MuPDF/PDFium，模型是 ONNX 格式可被 `ort` crate 直接加载；本模块复用其 ONNX 模型 + Rust 自写轻量管线（调研结论，见 REPORT §三）。
- **复用现有能力**：Poppler pdftoppm（`pdf/preview.rs` 已有单页渲染）、外部工具托管下载（`external/managed.rs`）、文件队列/工作台组件（`ToolWorkspaceShell`/`FileQueuePanel`）、tauriBridge 错误与动画事件。
- 输出遵循项目惯例：**一律另存副本，不覆盖原文件**。

## 技术选型（纯 Rust 拼装 + 算法借鉴，不引入 PDFium）

| 组件 | 选型 | 理由 | 许可证 |
| --- | --- | --- | --- |
| 复杂度检测 | 自研（文本层有无/稀疏/乱码判断，对齐 LiteParse `is-complex` 语义） | 几十行逻辑，先路由再处理，文本型 PDF 秒级直出 | - |
| 页面栅格化 | Poppler `pdftoppm`（已有） | 证据链路已集成，按需渲染、不整份读入内存 | 系统工具 |
| 文本型直出 | Poppler `pdftotext -bbox`（已有） | 词级坐标已实证可用（页眉 bbox 提取先例）；布局重建数据源 | 系统工具 |
| OCR 引擎 | **PP-OCRv6 ONNX**（RapidAI 统一转换，ModelScope 托管） + **paddle-ocr-rs**（ort crate） | 中文精度高；纯 CPU 零显存；Apache-2.0；v6 tiny 仅 1.5M 参数 | Apache-2.0 |
| 布局重建 | **移植 LiteParse 网格投影（PROJ）算法** | 词块投影 → 阅读顺序/行列重建，扫描件→Markdown 的核心；纯算法可移植 | Apache-2.0（移植保留版权头） |
| Markdown 输出 | **移植 LiteParse 渲染启发式**（标题/表格/列表/图片/链接识别规则，可裁剪） | 知识库/AI 友好格式，规则可裁剪到法律文书场景 | Apache-2.0（移植保留版权头） |
| 文本层回写 | **harumi** `add_invisible_text` | 纯 Rust、零 C 依赖、自动 CJK 字体子集化 + CID/ToUnicode CMap，支持 Tauri | MIT OR Apache-2.0 |
| 模型分发 | 设置页托管下载（复用 `external/managed.rs`） | 与 qpdf/poppler/ffmpeg 一致的用户路径 | - |

### PDFium 评估结论（不引入）

LiteParse 的 PDF 层用 PDFium（Chrome 内置引擎，BSD-3-Clause；`pdfium-render` crate 0.9.3、183 万下载、活跃）。**Docsy 不引入**，理由：

- 渲染质量（Chrome 级）Docsy 已有 Poppler pdftoppm 覆盖，OCR 输入图质量足够；
- 空间文本数据 Docsy 已有 `pdftotext -bbox` 词级坐标（实证过），布局重建数据源不缺；
- PDFium 是外部 C 动态库（macOS 约 15-25MB），与 v0.9 已定"去外部二进制 + 纯 Rust 化"方向（hayro 渲染 + pdf-syntax/lopdf 解析）相悖；v0.9 选型时 PDFium 即被列为备选退路未采纳；
- LiteParse 用 PDFium 是为跨语言分发统一，Docsy 是单一桌面应用且已有多平台外部工具链，不需要为此买单。
- 长期纯 Rust 化时布局数据从 hayro/pdf-syntax 补齐，本模块 PDF 层接口（渲染位图 + 词级 bbox）保持不变。

备选（不推荐为首选）：

- `ocrs`（纯 Rust OCR，rten 推理）：英文好、中文一般，适合无中文场景。
- `leptess`（Tesseract 绑定）：v0.14.0 自 2023-02 未更新，需随应用分发系统动态库与 chi_sim traineddata；中文质量差于 PP-OCR。
- OCRmyPDF 子进程：事实标准但依赖 Python + Tesseract，仅适合快速验证。
- MinerU VLM 路线：需要表格/公式/版面时再评估（显存 8GB 起）。
- LiteParse 整体集成（crate/CLI）：bundled Tesseract 体积大、V2 API 未稳定，仅借鉴其算法。

## 内存与性能预算

- 模型权重：PP-OCRv6 tiny/small/medium 参数 1.5M / 7.7M / 34.5M（ONNX 体积约 6MB / 30MB / 140MB，估算）[1]。
- 进程峰值目标 **< 500MB**：参考社区实测（Mac Mini M4，MNN 后端）完整管线峰值 388–422MB——模型权重仅占 20–25MB，大头是整页图像处理 [2]。**按页流式处理（渲染→OCR→释放）**，2000 页大 PDF 峰值保持稳定。
- 显存：0（纯 CPU 推理）。
- 单页 CPU 延迟目标 < 2s：参考 v6 相对 v5 的 CPU 端到端 5.2× 加速、Apple M4 上 6.1×（tiny）[1]。
- 并发：默认 1 个推理会话、单页串行；不引入多线程推理，控制峰值。
- 布局重建与 Markdown 渲染为纯 CPU 启发式，单页 < 50ms，可忽略。

> 注：ort 路径尚无公开内存基准（调研 Open Question 1），原型阶段需实测确认。

## 代码结构

### 后端 `src-tauri/src/ocr/`

| 文件 | 职责 |
| --- | --- |
| `mod.rs` | 命令入口与数据结构（OcrJob / OcrResult / OcrPageResult / ParseOutput） |
| `routing.rs` | 复杂度检测（文本型 vs 扫描件路由，对齐 LiteParse `is-complex` 语义） |
| `pipeline.rs` | 页面级管线：渲染 → 预处理 → det/cls/rec → 词块聚合 |
| `models.rs` | ONNX 模型加载与会话管理（ort），模型文件校验/版本 |
| `layout.rs` | **布局重建（移植 LiteParse 网格投影 PROJ）**：词块 → 阅读顺序/行列；**保留 Apache-2.0 版权头** |
| `markdown.rs` | **Markdown 渲染（移植 LiteParse 启发式）**：标题/表格/列表/图片/链接；**保留 Apache-2.0 版权头** |
| `export.rs` | JSON 输出（文本 + bbox + 页元数据）与 Markdown 组装 |
| `textlayer.rs` | harumi 文本层回写封装（坐标 → 不可见文本） |
| `progress.rs` | 进度上报与取消（接线 `SubprocessRegistry`，修复现有取消失效问题） |

命令域 `commands/ocr.rs` 注册进 `commands/mod.rs`，建议命令：

- `check_ocr_models`：检测模型是否已下载（引导到设置页）
- `parse_pdf`：单文件/批量执行，返回三件套路径（可搜索 PDF 副本 / md / json）
- `cancel_operation` 复用（配合 progress.rs 接线）

### 前端 `src/modules/pdf-ocr/`

- `index.js`：模块注册（路由/菜单/首页卡片），复用 moduleRegistry 自动发现
- `OcrWorkbench.vue`：工作台（文件队列、模型状态、参数、进度、结果列表、打开 md/json/PDF）
- `composables/useOcrProgress.js`：进度轮询/事件订阅
- 复用 `ToolWorkspaceShell`、`FileQueuePanel`（可排序）、`tauriCallSafe`

### 设置页

- 「OCR 模型」分组：PP-OCRv6 det/cls/rec ONNX 下载、版本、清除（复用工具清单与托管安装机制）
- 诊断信息：模型加载状态、最近一次解析统计

## 关键设计决策

1. **文本层只写不可见文本（渲染模式 3），不改原图**：视觉零变化，可搜索可复制；失败页保留原样并标记，不中断整批。
2. **"已有文本"检测（路由）**：处理前用结构特征（文本层有无 / 稀疏 / 乱码 / Tr 3 / 透明度 / 零字号 / 几何位置）判断是否需 OCR——文本型 PDF 直接 `pdftotext -bbox` 秒级直出，扫描件走 OCR 管线（对齐 OCRmyPDF `--force-ocr` 语义提供"强制栅格化重 OCR"选项）。注意：**仅凭提取文本判断会被 Unicode 混淆/隐藏文本层绕过**，需结合渲染状态特征（调研结论，CrackedPDFs F1=0.960 vs 纯文本检测失败）[4]。
3. **三输出统一产物**：一次解析同时产出 md / json / 带文本层 PDF 副本；md 与 json 共用布局重建结果（词块 + 阅读顺序），文本层回写共用 OCR 词块坐标——不重复计算。
4. **算法移植合规**：layout.rs 与 markdown.rs 移植自 LiteParse（Apache-2.0），文件头部保留版权声明与来源链接；仅移植纯算法，不引入其 PDFium/Tesseract 依赖。
5. **进度与取消**：OCR 任务注册进 `SubprocessRegistry`（本次审查 H1 发现的取消失效问题一并修复），前端取消真正终止推理会话。
6. **模型缺失降级**：未下载模型时给出明确引导，不静默失败。
7. **输出命名**：`原文件名_ocr.pdf` / `原文件名.md` / `原文件名.json` 放同目录，冲突时 `unique_*_path` 去重（沿用项目惯例）。

## 与现有模块的关系

- **evidence-pdf**：证据合并输出前，可选对扫描件执行 OCR 生成文本层，提升最终 PDF 可检索性；`detect_pdf_header_footer` 对"页面只有图片"的扫描件可标记为"建议 OCR"（与证据设计文档 161 行"OCR 作为未来可选能力"衔接）。
- **pdf-tools**：拆分/提取结果页可单选"OCR 转文本型"；解析产物（md/json）可作为文档知识库输入。
- **Casy**：复用同一后端命令（`parse_pdf`）与模型资产，把案件文档/证据解析为 md/json 供其知识库与 Skill 工作流读取；本地离线，无跨进程依赖。
- **settings**：模型托管与诊断。

## 反 OCR 输入的处理（第二阶段可选）

调研结论（REPORT §四）为后续功能埋点：

- **隐藏文本层检测**：基于结构特征检测 PDF 是否含不可见文本层，供"是否纯扫描件"判断与证据真实性提示使用。
- **对抗输入提示**：OCR 结果与可见层显著不一致（被 Unicode 混淆/对抗扰动）时提示用户，避免把被篡改文本当作正文。
- **不做**：生成"防 OCR"输出。文本层欺骗只影响阅读器元数据、骗不过 OCR；真正的 OCR 对抗需要图像空间扰动，与"生成可搜索 PDF"的目标互斥，且存在合规风险，不纳入本模块。

## 验收标准

1. 中文扫描件 → 生成副本：在 Foxit/Chrome/系统预览中**搜索命中、可复制、视觉与原件一致**。
2. Markdown 输出：单栏法律文书（正文/段落/简单列表）重建为可读 md，无乱序；JSON 含词级 bbox 与页元数据。
3. 文本型 PDF（含文本层）：`parse_pdf` 秒级直出 md/json，不触发 OCR。
4. 2000 页扫描 PDF：峰值 RSS < 600MB，处理中可取消（取消真正生效）。
5. 纯扫描件、已含文本层 PDF、加密 PDF 三种输入行为正确（提示/跳过/解锁引导）。
6. 模型缺失时引导到设置页下载，下载完成后可一键重试。
7. 单页 OCR 失败不中断整批，结果列表明确标记失败页；md/json/PDF 三产物一致（同一解析结果）。

## 风险与未决问题

| 风险 | 说明 | 缓解 |
| --- | --- | --- |
| ort 内存无官方基准 | 峰值内存为 MNN 后端实测推断 | 原型阶段实测 RSS 增量 |
| PP-OCRv6 ONNX 可用性 | RapidAI 已提供 v6 转换，中文实测精度待验证 | 原型验证 + tiny/small/medium 三档对比 |
| harumi 中文兼容性 | 中文不可见文本层在主流阅读器搜索/复制兼容性未实测 | 原型验证（验收标准 1） |
| 布局重建质量 | LiteParse PROJ 移植对双栏/表格/手写版面重建有限（启发式上限） | 法律文书多单栏正文；复杂版面保留"原始词序 + 提示"降级 |
| Markdown 启发式 | 表格/复杂列表识别规则可能误判 | 规则可裁剪；JSON 输出保留 bbox 可纠错 |
| 模型分发体积 | medium 档 ONNX 约 140MB（估算） | 默认 tiny/small，设置页可选 |
| 法律文书特殊版式 | 印章/手写/骑缝章区域 OCR 噪声 | 检测结果保留原图，仅叠加文本层，不破坏原件 |

## 参考

- [1] PaddleOCR 官方仓库（PP-OCRv6 发布说明）— https://github.com/PaddlePaddle/PaddleOCR （2026-06-11）
- [2] rust-paddle-ocr 模型版本与内存对比（社区实测）— https://deepwiki.com/zibo-chen/rust-paddle-ocr/5.1-model-versions-and-comparison
- [3] OCRmyPDF errors 文档（--force-ocr / --redo-ocr）— https://ocrmypdf.readthedocs.io/en/latest/errors.html
- [4] CrackedPDFs 基准（隐藏文本层需结构特征检测）— https://arxiv.org/abs/2607.19396
- [5] run-llama/liteparse（布局重建 PROJ + Markdown 启发式的移植来源，Apache-2.0）— https://github.com/run-llama/liteparse
- [6] pdfium-render crate（PDFium 绑定评估后不引入）— https://crates.io/crates/pdfium-render
- 完整调研：`~/research/pdf-ocr-rust/REPORT.md`（36 个来源，含 harumi / paddle-ocr-rs / MinerU 架构细节）
