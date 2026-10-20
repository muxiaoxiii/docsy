# Docsy PDF 引擎研究：PdfCraft、craft-pdf、PDF Oxide 与 Hayro

研究日期：2026-10-19（Asia/Shanghai）。
用途：供 Gemini 实施、Codex 后续复核。实施任务见 [实施与验收计划](pdf-engine-implementation-plan.md)。

后续研究：[内置 PDF 引擎替代能力与接入要求](pdf-native-first-code-review-2026-10-19.md)，补充压平、加密/修复、资源剪枝、PDF.js 与文本提取的源码和运行对照。

## 1. 决策

**PdfCraft 已具备值得接入验证的原生 Rust PDF 对象层与页面组织能力；本轮应建立候选后端并进行对照验收，不应直接删除 qpdf、Poppler 或切换生产默认值。**

这不是“新项目都不如 qpdf”的结论。PdfCraft 的书签、附件、表单关联、独立标记流和碎片图片优化均有可学习的具体实现。另一方面，源码和小样本实测已经发现其默认行为与 Docsy 不等价，适配工作不能省略。

建议顺序：

1. 采用 PdfCraft 的对象层与 organize 模块做隔离候选后端，验证页面组织与书签策略。
2. 在现有中文字体与坐标实现上借鉴可识别、可更新、可移除的标记流。
3. 单独验证碎片图片合并与保守压缩，不重写现有全部压缩模块。
4. PDF Oxide 作为文字/坐标/Markdown 提取候选；Hayro/craft-pdf 作为预览候选，分别评估。
5. 只有具体能力通过验收后才改变相应默认路由。不要将一次集成扩大成 UI、Office 转换或整个 PDF 工具箱重写。

## 2. 证据等级与研究边界

本文区分：

- **实测**：本机实际编译、运行、读回或渲染的结果。
- **源码确认**：检查实现和调用关系，未必完成 Docsy 接入或真实文件验收。
- **上游说明**：README、路线图和已知缺口中的作者陈述。
- **建议**：本研究提出的 Docsy 设计，不是上游现成能力。

本轮覆盖 Docsy 会使用的解析/写入、合并提取、页面几何、书签关联、页眉水印、图片优化、渲染/提取、依赖与任务边界。没有逐行审计上游全部代码，也没有完成密码算法审计、签名验证、脱敏安全审计、Windows 实机验收、超大文件压测或生产默认后端迁移。

没有将“能打开”“qpdf --check 成功”“上游测试通过”解释为完整保真或安全证明。

## 3. 固定研究基线

| 项目 | 本轮固定提交 | 检查深度 |
| --- | --- | --- |
| Docsy | 51931b2676b8c0debb1eb06b49ced81350506eb7 + 当时工作区修改 | 现有调用链和相关实现 |
| storytold/pdfcraft | 4db6ac4ae2448df6389d8bb0bc54db6ba175721c | 相关源码、定向测试、合并/提取实验 |
| storytold/craft-libs | ae2f796b673f29cf3161f36eb6a8933f0a3fe028 | craft-pdf、craft-atomic 源码/接口和依赖；未执行其独立测试 |
| yfedoseev/pdf_oxide | ad49c4cb3638dc950a29350ef882161372fde473 | Cargo 特性、读取/提取/编辑源码；未在本轮编译或跑样本 |
| LaurenzV/hayro | 未固定提交 | 上游 README 与 PdfCraft 固定依赖/补丁；不构成独立接入验证 |

PdfCraft 本轮包版本为 0.9.0，工作区要求 Rust 1.95。PDF Oxide 为 0.3.78，声明 Rust 1.88。Docsy 本机检查时 rustc 为 1.99.0；这不代表所有发布 CI 都满足要求。

Docsy 工作区已有 Word→PDF 等未提交修改；研究期间相关文件还发生了更新。**这些不是本研究写入的代码，不能重置、覆盖，也不能沿用聊天中旧行号作为当前修复状态。** 机器可读证据记录在 [evidence.json](research/pdf-engine-2026-10-19/evidence.json)。

## 4. Docsy 当前并不是只使用 qpdf

| 能力 | 当前入口/实现 | 实施时必须保留的语义 |
| --- | --- | --- |
| 合并、提取、无损收尾 | src-tauri/src/pdf/qpdf.rs | 默认合并面向打印，压平注释外观；双面模式对奇数页文件补同尺寸空白页 |
| Office 混排 | commands/pdf.rs、pdf/word_convert.rs、pdf/evidence.rs | Office 转换独立于 PDF 引擎；不要用本研究替换 Word/WPS/LibreOffice/doc2x |
| 页尺寸/旋转 | pdf/page_info.rs、artifact_geometry.rs | MediaBox/CropBox、继承、非零原点和旋转的现有约定 |
| 页眉页脚 | pdf/header_footer.rs、overlay_pdf.rs、overlay_font.rs | 中文字体子集、Unicode 映射、页码规则；qpdf overlay 的旋转补偿不能重复施加 |
| 原有元素检测与删除 | pdf/artifacts.rs、qpdf_stream.rs、content_text.rs | 标准 Artifact、Form XObject、共享资源和文本定位，不能只靠矩形遮盖 |
| A4 规范化 | pdf/normalize.rs | 视觉尺寸、旋转和安全边距；不是把页面栅格化 |
| 图片压缩 | pdf/compress.rs | 已有 CTM/有效 DPI 分析、条件降采样、特殊图像跳过策略 |
| 大文件 | pdf/chunked.rs | 300 页或 100 MiB 起分段、每段 80 页；特定复杂目录结构拒绝不安全分段 |
| 防复制/恢复 | pdf/anti_ocr.rs | Info 中 DocsyAntiCopyBackup 及字体内 DocsyOriginalCMap；旧恢复格式的重写限制 |
| 原生预览 | pdf/preview.rs | pdftoppm；必要时通过 qpdf 修复后重试 |
| 前端预览 | shared/pdf-tools/components/PdfJsPreview.vue | 另有 PDF.js 路径；保留坐标与切页取消语义 |
| 文本层→MD | markdown/pdf_to_md.rs | pdftotext 两遍流式处理、页段、原页号、取消、无文本页提示 |

因此，换掉 merge 函数不等于不再需要 qpdf。要完全移除 qpdf，仍需替代对象索引、流读取/回写、书签、叠加、修复和分段等调用。

### 对之前建议的修正

Docsy 已有图片有效 DPI 计算和条件重编码。PdfCraft 的有效 DPI 分析是对照对象，不能作为 Docsy 缺失功能重复建设。新增价值应具体落在多层 Form 的测量边界、共享图像多次使用、碎片图片合并及遇到不确定情况时的处理策略。

## 5. PdfCraft 模块评估

源码链接以下均锁定到本轮提交，避免 main 漂移。

### 5.1 cos：对象层，值得作为候选

[README](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/cos/README.md)

源码确认：

- PDF 对象图为编辑模型，解析缓存及原始字节共享，修改通过 copy-on-write 保存。
- 提供 write_incremental 与 write_full；前者追加修订，后者重新写可达对象。
- 提供交叉引用修复日志、密码错误与加密相关接口。
- 对象惰性解析不等于文件流式读取：当前输入仍整体缓存在内存，写入返回完整字节数组。

Docsy 适配判断：

- 比调用多次 qpdf JSON 有机会减少进程/JSON 开销，但本轮没有性能证据。
- 不可据此撤掉大文件限制或取消隔离。
- 增量写入保留原字节，不等于修改后的文档仍满足签名权限、DocMDP 或验证器要求。
- 低层 API 不负责原子发布、输出避名、应用权限策略及所有文档级数据的复制。

### 5.2 organize：本轮最直接的候选

[页面导入/合并/提取源码](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/organize/src/import.rs)

源码确认：

- 页面及所引用资源深拷贝并重编号；重写注释的 /P 和内部跳转。
- 处理同名字段冲突，注册 /AcroForm 和可选内容图层。
- combine 将源书签嵌套到文件级书签下，带入文档附件，去重部分资源。
- Combiner 可逐份添加输入并释放源文档；目标文档和最终写出仍需占用内存，不是全链路流式。
- 源结构树未被复制，导入时移除相应结构链接；被排除页的跳转有删除/失去目的地等特定行为。
- extract_pages 与 combine 的文档级保留策略不同：不能拿合并的能力推断提取一定保留书签、附件和任意 Info。

**实测差异见第 7 节：直接提取会丢自定义 Info；这对 Docsy 恢复信息是阻止默认切换的兼容缺口。**

### 5.3 edit/content：学习标记与最小编辑，不替换中文字体实现

[edit](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/edit/src/lib.rs)；
[content](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/content/README.md)

源码确认：

- 页眉、水印、背景使用独立内容流，包含 /Artifact 和 /PCMark 分类标记。
- 通过明确标记更新/删除自己生成的内容；追加前处理原内容图形状态隔离。
- content 提供算子、源码跨度和序列化，保留内联图像作为独立处理对象。
- 本轮检查的页眉文字路径使用 Helvetica/WinAnsi，first_undrawable 会拒绝无法表示的字符。

Docsy 应借鉴“独立流 + 所属标记 + 安全定位”的方法，在现有 CJK 字体嵌入和共享页面几何上实现自己的版本化标记。不能识别不到旧标记就自动删除疑似页眉，也不能把对历史文件的识别功能缩减为只认新标记。

### 5.4 optimize：可选择移植的算法，不使用整套默认预设

[优化器](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/optimize/README.md)；
[设置定义](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/optimize/src/lib.rs)

值得对照：

- 碎片条带图像合并；仅满足方向、颜色、位深、相邻关系等条件才合并。
- 递归测量 Form 中图片用途，不能测量时的保守策略。
- 透明蒙版与图像同步重采样；收益不足时保留原图。
- 空间占用报告、分阶段进度及中止回调。

不直接采用：

- Standard 默认 150 ppi、Mobile 96 ppi 不适合作为 Docsy 证据材料默认参数。
- 不将 Remove Hidden Information 或 sanitize 作为压缩收尾。
- 不自动删除附件、批注、书签、表单、标签或自定义恢复数据以换取更小体积。
- JPEG 重编码属于有损步骤，不能放入“无损优化”入口。

### 5.5 render、OCR、签名、脱敏：独立范围

[架构](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/docs/architecture.md)；
[已知缺口](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/docs/gaps.md)

- 当前渲染基于本地补丁版 Hayro；部分检查使用 lopdf。不能说是所有层都完全独立实现。
- 不在第一轮引入完整 engine。它会拉入 OCR、JS/XFA、签名、UI 周边等明显超出页面组织需求的依赖。
- 脱敏不等同于当前页眉删除：不可用遮盖矩形宣称完成信息删除；必须覆盖旧修订、ActualText、Alt、附件和共享对象等泄漏通道。
- 签名、PDF/A、OCR 的宣传能力不是 Docsy 的验收结论。本轮不纳入实施范围。

## 6. craft-pdf、craft-atomic、PDF Oxide 与 Hayro

### craft-pdf

[接口与边界](https://github.com/storytold/craft-libs/blob/ae2f796b673f29cf3161f36eb6a8933f0a3fe028/crates/pdf/README.md)

- read / interpret / raster / write 可分别启用，默认不启用。
- 读取/渲染来自固定补丁版 Hayro；写入封装 Krilla。
- 不提供完整的通用文档编辑模型；COS 编辑、缓存、调度及字体政策仍由应用负责。
- 接入者须在依赖根同步固定必要 patch；依赖仓库内部的 Cargo patch 不会自动传递。
- 诊断、解压上限、取消有明确边界；成功打开不是对象全部保留证明。

结论：适合后续 preview PoC；不是把 qpdf 换成一个 craft-pdf 依赖就能完成迁移。

### craft-atomic

[原子写入](https://github.com/storytold/craft-libs/blob/ae2f796b673f29cf3161f36eb6a8933f0a3fe028/crates/atomic/README.md)

值得学习同目录 staging、排他新建、失败注入和 Windows 临时占用重试。注意：

- 原子替换与“不覆盖已存在文件”是不同接口契约。
- 新建输出应使用排他发布；绝不能把 atomic_write 当作默认防覆盖。
- 进程被强杀后 staging 仍可能残留；需要应用恢复策略。
- 不为这一项收益直接替换 Docsy 所有文件工具。本轮已有相关修复，实施前重新读现状。

### PDF Oxide

[固定源码](https://github.com/yfedoseev/pdf_oxide/tree/ad49c4cb3638dc950a29350ef882161372fde473)；
[Cargo 特性](https://github.com/yfedoseev/pdf_oxide/blob/ad49c4cb3638dc950a29350ef882161372fde473/Cargo.toml)

重点入口：

- src/document.rs：extract_text、extract_spans、to_markdown。
- src/editor/document_editor.rs：merge_from_bytes、extract_pages_to_bytes、保存与对象导入。
- rendering 使用 tiny-skia 等可选依赖；table-ml 会带入 PDFium；OCR 另需推理运行时/模型。
- 默认特性、渲染、OCR、表格模型要分别统计，不能把“Rust 核心”写成所有能力零依赖。

本轮仅源码确认。优先用于独立的文字/坐标/Markdown 质量比较，不将作者发布的平均耗时或通过率移植成 Docsy 指标。其页级深拷贝 API 不能自动证明书签、附件、表单、恢复数据完整。

### Hayro

[上游](https://github.com/LaurenzV/hayro)

专注解释与渲染。上游 README 明确仍处于开发中，存在字体、渲染特性与性能边界。PdfCraft 本轮实际固定的是带本地补丁的 0.7 系列；不能把上游另一版本的表现等同于该补丁版。

结论：preview 候选，不能单独替代 qpdf 的编辑/重写语义。本轮未独立编译或渲染 Hayro，候选输出的视觉检查使用 Poppler。

## 7. 本机验证结果

### 7.1 上游定向测试

命令：

~~~sh
cargo test --locked -p pdfcraft-cos -p pdfcraft-organize \
  -p pdfcraft-edit -p pdfcraft-optimize --lib
cargo test --locked -p pdfcraft-optimize --test optimize
~~~

第一条命令结果：

| 包 | 通过 | 失败 | 忽略 |
| --- | ---: | ---: | ---: |
| pdfcraft-cos | 63 | 0 | 0 |
| pdfcraft-organize | 134 | 0 | 0 |
| pdfcraft-edit | 185 | 0 | 0 |
| pdfcraft-optimize 的 lib 目标 | 0 | 0 | 0 |

optimize 的 0 项不能算算法测试通过；另外运行的 integration test 为 **22 通过、0 失败、0 忽略**。合计本轮定向运行 404 项测试通过。原始测试摘要同时保存在 evidence.json；这不是上游整个工作区的全量测试。

研究副本首次稀疏检出缺少 assets/fonts，导致字体 include_bytes 编译失败；补齐该目录后重跑通过。这是本次检出范围问题，不作为上游编译缺陷。

### 7.2 独立合并/提取探针

使用临时 example 调用实际 cos/organize API，不修改 Docsy 源码。测试输入为自行生成的两份合计 3 页 PDF，含中文、90° 旋转、同名表单字段、书签、附件、自定义 Info 与页内跳转。

对照命令为 qpdf --empty --pages input-A.pdf input-B.pdf -- qpdf-merged.pdf。
**这是不压平的结构合并对照，不是 Docsy 当前打印合并的等价测试。**

| 项目 | 实测结果 |
| --- | --- |
| PdfCraft combine | 3 页，页序正确；两个 common_name 字段分别为 value-A/value-B，其中一个改名 common_name-1 |
| 合并书签/附件 | 文件级书签下保留源书签；A.txt/B.txt 均存在 |
| PdfCraft extract [1,0] | 2 页，倒序正确，旋转保留 |
| qpdf --check | 两份输入和三份输出均 exit 0 |
| Poppler 渲染 | 5 份 PDF、11 页均成功；已目视检查总览 |
| 图像比较 | 8 组输入/输出页面在 72 DPI 下逐像素一致 |
| 自定义 Info | combine 与 extract 输出均未保留 DocsyReviewToken |
| extract 文档级内容 | 本样例源书签和附件未带入；字段值仍在 |
| qpdf 结构对照 | 字段冲突也得到处理；该命令未带入源书签/附件 |

图像一致只证明这些样例在同一 Poppler/分辨率下的可见页面一致，不证明所有元数据、无障碍结构或所有渲染器一致，也不构成速度测试。

**重要发现：** DocsyAntiCopyBackup 位于 Info。探针中的自定义 Info 丢失，加上 extract_pages 只复制已知 Info 键的源码，足以说明不能把该 API 不加适配地用于 Docsy 的恢复链。是否可复制还需按备份版本和字体身份重新验证，不能盲拷贝旧对象编号。

完整字段、几何、文本、哈希和检查结果见 [probe-results.json](research/pdf-engine-2026-10-19/probe-results.json)；[渲染总览](research/pdf-engine-2026-10-19/contact-sheet.png)。

### 7.3 未验证事项

- Docsy 原生 WebView 全流程、Windows x64/x86、安装包体积和 CI 新环境构建。
- 实际签章外观压平、无 AP 注释、共享外观、签名验证与 DocMDP。
- 300/1,000/2,000 页和 100 MiB 以上文件的峰值内存、取消延迟与损坏文件。
- 本地真实中文卷宗与外部读者（Acrobat/WPS 等）的跨渲染器比较。
- PDF Oxide 与 craft-pdf 的实际性能和包体，不宣称已优于当前工具。

## 8. 交付判断

| 能力 | 判断 |
| --- | --- |
| cos + organize 的候选后端 | 可以进入实现；默认仍使用现有后端 |
| 自动保留源书签/附件的增强合并 | 有源码和小样本依据；需要确定输出政策并补测试 |
| 打印合并直接替换 | 暂不通过：未完成压平外观、恢复信息与大文件等价验收 |
| 标记流更新/移除 | 建议在现有中文绘制链上实现 |
| 碎片图片优化 | 独立、显式有损路径评估；保留无损入口边界 |
| 全面切换原生预览/文本提取 | 后续独立 PoC，不能随页面组织一起默认上线 |
| 删除 qpdf/Poppler | 不在本阶段范围 |

本研究允许 Gemini 按实施文档开始代码工作；不授权覆盖当前修改、删除旧后端、擅自上传文书、发布版本或把待验证项改为通过。Codex 后续以实际 diff、可复现输出和验收矩阵复核。
