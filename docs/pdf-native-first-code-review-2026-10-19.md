# Docsy 内置 PDF 引擎替代能力与接入要求

日期：2026-10-19。用途：复核 Gemini 的“内置优先、扩展按需安装”结论，并为后续实施提供代码依据。

**现有项目可以推进内置优先。页面组织、压平、密码处理、修复、书签和资源清理均有可复用的 Rust 实现；目前的业务入口尚未完成迁移。不能用“只改默认后端”完成交付，也不能用未经统计的 85%、90%、100% 描述替代覆盖率。**

## 1. 基线与验证范围

- Docsy HEAD：`8746760240ce801156620d8d840d7bf825d9d2f8`，另有当前未提交的 1.1.1 候选代码；本文不将候选代码等同于已发布版本。
- PdfCraft：`4db6ac4ae2448df6389d8bb0bc54db6ba175721c`。
- craft-libs：`ae2f796b673f29cf3161f36eb6a8933f0a3fe028`。
- PDF Oxide：`ad49c4cb3638dc950a29350ef882161372fde473`，版本 0.3.78；独立提取探针关闭默认特性。
- 使用 Docsy 本地安装的 PDF.js 运行预览与文本探针；Node Canvas 仅作测试载体，不是建议在桌面安装包中引入 Node。

本轮新增 16 份合成输入：10 类注释/Widget、4 类加密文件、2 类交叉引用损坏。对压平输出重新解析，并用 Poppler 渲染源文件、原生输出和 qpdf 输出，逐行查看对照图。密码处理、修复、清理与书签的原生探针另在 `PATH=/nonexistent` 下运行，结果与普通环境一致。

没有修改应用业务代码或用户 PDF。尚未完成真实中文法律文书样本集、多平台安装包、复杂字体/表单、超大文件内存和吞吐量验收。源码快照与结果见 [机器可读证据](research/native-first-2026-10-19/results.json)。

## 2. 对 Gemini 结论的逐项判定

| 结论 | 判定 | 代码依据与修正 |
| --- | --- | --- |
| 页面合并、提取有原生替代基础 | 成立 | `organize::Combiner`、`extract_pages`、`split` 已存在；Docsy 已有候选适配。 |
| 1.1.1 已经使普通用户完全不需要 qpdf | 不成立 | `select_backend` 仍默认 Qpdf；业务合并创建 Print 策略，候选策略对所有 Print 请求回退；其他业务模块也直接调用 qpdf。 |
| 原生速度比子进程快数倍 | 无本项目基准支持 | 减少进程启动开销不等于端到端更快。需要同文件、同输出语义、同优化配置的耗时与 RSS 对照。 |
| qpdf 叠加不可逆，原生打标保证不伤正文 | 过度概括 | 可撤销性取决于标记所有权、内容流组织与删除实现；不能由引擎名称推出。当前 marks 尚未接入生产页眉页脚链，而且之前发现的包装流误认领缺陷仍在源码中。 |
| 页数只读 Trailer/Catalog 即可 | 不准确 | 实际需要遍历/验证页树，不能无条件信任 `/Count`；继承、坏引用和环要单独处理。 |
| 孤儿对象清理可直接等同 qpdf 优化 | 部分成立 | `write_full` 清除不可达对象，但仍被资源字典引用的未使用对象需要先剪枝。本轮发现上游剪枝的一个边界缺口，见第 5 节。 |
| 密码处理只能替代 85% | 比例无依据 | 当前 cos/crypt 已有标准加密算法与接口；本轮验证 RC4、AES-128、AES-256 和空用户密码 AES-256。应用适配仍缺密码入口与权限策略。 |
| 没有可复用的纯 Rust 压平实现 | 错误 | PdfCraft 上游 `pdfcraft-edit::flatten` 已实现；PDF Oxide 编辑器也有压平 API。存在实现不代表已达到 Docsy 的输出契约。 |
| PDF.js 能承担界面预览 | 成立，但限于对应入口 | 现有组件已有回退。后端页眉页脚真实预览、文本检测和 PDF 转 Markdown 不会因此自动脱离 Poppler。 |
| 当前 extract_poc 可直接作为 bbox 提取备选 | 不成立 | 连续 Tj 的游标、字体解码、估算宽度、继承页面尺寸均有可复现差异，见第 7 节。 |
| “普通文件占 90%” | 无统计依据 | 应按能力判断文件能否处理，不按未经统计的“普通/特殊”标签决定。 |

## 3. 压平已有实现，需要补输出政策

### 3.1 已经可以复用的代码

[上游 flatten.rs](https://github.com/storytold/pdfcraft/blob/4db6ac4ae2448df6389d8bb0bc54db6ba175721c/crates/edit/src/flatten.rs) 提供：

```rust
pub fn flatten(
    doc: &mut Document,
    pages: &[usize],
    comments: bool,
    fields: bool,
) -> Result<usize, EditError>
```

其实现包含 `/AP /N` 外观读取、`/AS` 状态选择、外观 `/BBox` 与 `/Matrix` 到注释 `/Rect` 的映射、XObject 命名避冲突、向内容流写入 `q ... cm /Name Do Q`，以及从字段树清理已压平 Widget。无法绘制的部分注释会保留，返回值是绘制数量，不是“压平已完整完成”的证明。

本轮运行上游压平相关单元测试：**9 passed，0 failed**。普通 Widget、复选框 `/AS`、旋转外观的合成对照中，原生输出均绘制了外观并移除了 Widget，qpdf 对照也有相同的红色图形和 Widget 数量。不能再把这项能力写成需要从零等待生态补齐。

Docsy 当前 Cargo 只直接引入 `cos` 和 `organize`，没有 `edit`。完整 `edit` 还依赖 annot/fonts/model/content 等模块。可评估有选择地引入，或按许可提取最小压平适配；不要为此直接引入整个 PdfCraft engine/UI/OCR。

### 3.2 实测差异

这里比较的是 Docsy 当前实际使用的 `qpdf --flatten-annotations=all`。它不同于 `--flatten-annotations=print`，不能只因业务策略叫 Print 就混用两者的规则。

| 合成输入 | PdfCraft 原生结果 | qpdf all 结果 | 接入要求 |
| --- | --- | --- | --- |
| 普通 Widget，有有效 AP | 绘制 1，Widget 归零 | Widget 归零，图形保留 | 可以作为基本实现起点。 |
| 复选框 AS=Yes | 选择红色 Yes 外观，Widget 归零 | 同样保留红色图形 | 状态选择可复用。 |
| Widget 无 AP | 绘制 0，保留 Widget | 保留 Widget | 两者都不能保证任意输入完成静态化；需要明确拒绝、补生成或提示。 |
| NoView + Print，F=36 | 跳过，Widget 仍为 1，渲染无红图形 | Widget 归零，渲染有 6000 个红像素 | 不能直接套上游偏屏幕可见性的过滤规则。 |
| Invisible + Print，F=5 | 写入外观，6000 个红像素 | 不写入外观，0 个红像素 | 明确 Invisible/Hidden/NoView/Print 的处理规则。 |
| Hidden + Print，F=6 | 不绘制，但保留 Widget/Fields | 不绘制，Widget 被移除 | 视觉一致不代表输出已无交互数据。 |
| NeedAppearances=true，字段值与 AP 不一致 | 直接固化已有 AP，Widget 归零 | 保留 Widget，不固化该 AP | 预检过期外观，避免把旧显示永久写进页面。 |
| Watermark 注释，有 AP | 不属于其 comment/field 选择范围，保留注释 | 写入外观并移除注释 | 上游通用名称 flatten 不代表覆盖 qpdf all 的全部注释类型。 |
| 零宽 BBox | 不绘制，Widget 被移除 | 不绘制，Widget 被移除 | 不把 drawn=0 单独解释为失败或成功；核对输入可见性与残留结构。 |

红像素只是这些确定性矩形样本的比较指标，不是完整文档保真评分。完整数据、命令退出码和渲染图保存在 `release-candidate/native-first-2026-10-19/`。

### 3.3 Gemini 的具体实施任务

1. 在适配层明确压平范围和可见性政策，保持当前 all 行为，或将产品行为变化单独说明。
2. 先识别外观缺失、无效、过期、未支持的注释类型，再决定处理。不要仅凭 `has_form_fields` 判断打印能力，页面上的孤立 Widget/Stamp 也要检查。
3. 执行后检查目标 Widget、字段树及其他应静态化对象；有残留时返回可区分的未支持结果，不能用一个绘制数量宣告完成。
4. 不把 Link 等可保留交互对象与必须压平的 Widget 混为一类。
5. 保留签章图像不等于保留数字签名有效性。合并/提取/全量重写后的签名提示与校验须独立处理；qpdf 也不能保证原证书验证结果不变。

## 4. 加密与修复可以现在接入

当前引入的 `cos::Document` 已有 `open_with_password`、`unlock`、`security`、`remove_encryption` 和加密写出接口。`crypt` 包处理标准 RC4/AES 算法，不限于弱加密。当前 Docsy 候选适配只调用无密码打开接口，且对加密输入回退，这属于应用限制。

本轮由 qpdf 独立生成加密文件，再由原生探针读取：

- RC4 40 位、AES-128、AES-256：正确用户/所有者密码均成功打开，去加密保存后可无密码重开；无密码和错误密码被拒绝。
- 空用户密码 AES-256：不提供密码即可打开，仍保留加密状态；原生去加密保存后无加密状态。它说明“可打开但限制复制”不等于弱加密。
- 共 16 次密码组合检查，8 次预期成功、8 次预期拒绝；算法间实际样本覆盖仅限上述四种生成配置。
- 上游 cos 加密集成测试：**11 passed，0 failed**。
- `startxref=0` 和缺失 xref/startxref 两个损坏样本均由原生重建索引，返回 1 页并提供修复日志；输出可重新解析和渲染。

应补的应用工作是密码输入与生命周期、权限政策、可区分的密码/解析/修复错误、修复警告，以及重写后几何和资源读回校验。损坏修复可能遇到多个可解释对象版本，不能把修复成功自动当作内容正确。

## 5. 清理、书签和分卷的替代路径

`cos::write_full` 已能清理不可达对象。本轮添加不可达 sentinel 和 Catalog 可达 sentinel，保存并重开后确认前者消失、后者保留。判断在解析后的对象图上完成，避免把对象流压缩造成的“明文搜不到”误判为对象被删除。

但垃圾清理有两层：

1. 引用图上不可达：完整重写可以清理。
2. 页面资源字典仍引用、实际内容流不使用：必须先删资源引用，再做完整重写。

`organize::extract_pages` 已调用 `prune_unused_xobjects`，会递归分析部分 Form 用途，并保护不确定图形资源；当前它不是任意 PDF 资源类别的通用清理器。

**本轮新增发现：页面完全没有 Do 算子时，未使用的 XObject 仍可能留下。** `prune.rs::Pruner::content` 只在遇到 Do 时向 `used` 注册资源键，最终清理遍历的又只有 `used`，所以“零次使用”不会进入清理。合成有效 Form XObject 实测为：

```text
full_save_keeps_unused=true
extract_without_do_keeps_unused=true
extract_with_do_prunes_unused=true
```

适配前应修正或补偿这一边界，并检查注释 AP、Type3、Pattern、软蒙版等间接用途，避免为减小体积误删可见资源。不能用简单扫描页面 Do 的实现替代所有资源可达性判断。

书签方面，`organize` 已提供读取、增删、改名、移动和目标页设置。本轮添加中文标题“证据一”、完整保存、重开，确认标题与目标第 0 页保持。业务需要进一步验证命名目的地、层级、页序变换和排除页链接。

分卷、证据拆分可基于页面组织接口迁移，但仍须保留当前目录结构限制、分卷命名、取消、临时文件发布、防复制恢复信息及大文件预算。原生 cos 当前仍整体持有输入字节，完整保存返回整段字节；不能把“按对象惰性解析”当成有界内存的文件流式处理。

## 6. PDF.js 能先接管界面预览，但要覆盖入口与坐标

### 6.1 现有回退已经静默发生

`PdfJsPreview.vue` 的 auto 路由先调用后端，失败后进入 PDF.js。执行组件中实际的路由函数、将后端设为缺工具错误后得到：

```text
auto:    backend -> pdfjs，errorText 为空，没有 error 事件
pdfjs:   pdfjs，errorText 为空
backend: backend，出现 error 事件
```

`tauriCallSafe` 会记录日志/操作事件，但不会在组件回退成功时自行弹出安装框。因此可以优化默认路线和失败开销，不能把“auto 缺 Poppler 总会弹窗”写成现有事实。

PDF.js 已用于证据工作台、PDF 工具预览和下一页缩略图。本轮在不调用 Poppler 的 Node Canvas 探针中渲染了普通 Widget、旋转/裁剪/UserUnit 和无 AP 样本，并提取了三个文本样本。此结果证明引擎在这些样本上可独立工作，不代表 Tauri WebView 真机和全部 PDF 均已通过。

### 6.2 容易遗漏的入口

- `header_footer::preview_overlay` 先生成真实处理结果，再直接调用 Rust `preview::render_preview`；前端组件切 PDF.js 不会改变这条服务端生成 PNG 的路线。
- `get_pdf_page_count` 命令仍直接调用 qpdf。即使画面能由 PDF.js 绘制，文件导入、提取页数和证据页段分析仍可能先失败。
- 检测使用 `pdftotext -bbox`；PDF 文本层转 Markdown 则在 `markdown/pdf_to_md.rs` 使用可取消的 pdftotext 输出。这两条链不会随预览迁移。
- 页面信息已有 lopdf 后备路径，优先顺序可以调整；但要统一几何契约，不能只调换函数次序。

### 6.3 几何与离线资产

当前 PDF.js 分支直接把 `getViewport({scale:1})` 的宽高作为 widthPt/heightPt。对本轮 CropBox `[20,30,380,370]`、Rotate=90、UserUnit=2 的样本，PDF.js 返回 **680×720**。当前 Rust PageSize 解析链未计入 UserUnit，会按裁剪框与旋转得到 **340×360**。切换优先级会暴露两条路线的单位差异；须统一页面单位、裁剪原点、旋转、毫米换算与导出定位。

当前 getDocument 仅提供 data/enableXfa；源码中未显式配置 cMapUrl、standardFontDataUrl、wasmUrl。对离线中文、非嵌入字体和特殊图片解码，应检查实际构建产物及资源解析路径，再宣称免下载。字体资源与解码器能否缺省工作需要样本证明，不能由“存在 pdfjs-dist 依赖”推出。

若需要纯 Rust 后端渲染，[craft-pdf raster](https://github.com/storytold/craft-libs/blob/ae2f796b673f29cf3161f36eb6a8933f0a3fe028/crates/pdf/src/raster.rs) 已封装 Hayro，提供尺寸/像素限制、取消与诊断。其 MAX_SIDE=8192、MAX_PIXELS=64000000 只约束输出分配，未约束所有后端临时内存；诊断为空也不是完整保真证明。本轮仅源码核对此方案，未完成渲染接入验收。

## 7. 文本与坐标应采用成熟提取链，不能直接提升当前 PoC

当前 `extract_poc.rs` 用 UTF-8 lossy 解码 PDF 字符串，以字号×0.6×字数估宽，没有完整字体度量与游标推进。读取 ToUnicode CMap 只解决字符映射的一部分，不能替代文字布局解释器。现有 `cmap.rs` 的资源读取也依赖 QpdfObjectIndex，不能仅复用模块名字就消除 qpdf。

本轮重新编译当前 PoC 源码，并用同一份输入对照 PDF.js 与 PDF Oxide：

| 输入 | 当前 PoC | PDF.js | PDF Oxide |
| --- | --- | --- | --- |
| `(A) Tj (B) Tj`，Helvetica 12pt | A、B 起点均为 x=30 | AB 总宽约 16.008 | 字符起点约 30、38.004，总宽 16.008 |
| WinAnsi 字节 E9 | 页面纯文本为 é，但 span 是替换字符 | é，宽 6.672 | é，宽 6.6 |
| 继承 MediaBox 400×600 | 返回默认 595×842 | 读取 400×600 | 读取有效页面框，并给出文本 span |

PDF Oxide 的 WinAnsi 宽度与 PDF.js 仍有差异，三个样本不能证明完整词框等价。应优先验证其 `extract_spans` / `extract_chars`，统一为 Docsy WordBox，再运行既有页眉页脚/页段建议逻辑；不要直接换掉业务规则，更不要把字符串长度估算包装为精确 bbox。

**依赖集成陷阱已复现并解决：** 将上述固定 PDF Oxide 源码作为独立 path dependency，首次解析到 office_oxide 0.1.12，编译在 `src/converters/pdf_to_ir.rs` 失败：`DocumentIR` 初始化缺少 `defined_names`。对照其上游 Cargo.lock 将 office_oxide 固定为 **0.1.9** 后构建成功，三个提取样本均执行完成。没有修改 PDF Oxide 源码；接入 Docsy 时须锁定经过验证的依赖组合，不能只固定 Git 提交。

该提取探针关闭默认特性，不能从中推断 PDF Oxide 的加密、OCR、渲染或完整编辑路径已经通过。中文 CMap、竖排、Form 内文字、嵌入字体、ActualText、旋转与裁剪仍需实测。

## 8. 现有业务调用需要按操作迁移

| 业务链 | 现有依赖位置 | 原生替代工作 |
| --- | --- | --- |
| 合并/提取 | commands/pdf.rs、backend | 接入压平、密码及输出政策，再调整默认与回退。 |
| 证据生成/汇总/会话 | evidence.rs、evidence_session.rs | 改用统一门面，保持恢复信息与分组输出语义。 |
| 分卷、拆分 | chunked.rs、split.rs | 替换页段组织、对象检查与空白页内容流读取，保持资源清理与内存限制。 |
| 页眉页脚、Bates、旧元素清理 | header_footer.rs、artifacts.rs、content_text.rs、qpdf_stream.rs | 替换 overlay 与流索引/回写；保留中文字体、共享资源克隆和既有删除规则。marks PoC 不能直接代表这些功能已迁移。 |
| 页数、页尺寸、签章检查、书签 | commands/pdf.rs、page_info.rs、signatures.rs、bookmarks.rs | 用原生字典和页树读取，并统一几何、标志及目标页策略。 |
| 压缩与无损优化 | compress.rs、qpdf.rs | 分清图像重编码与对象/资源清理，保持所有用途的图像质量约束。 |
| 预览 | PdfJsPreview.vue、preview.rs、header_footer.rs | 界面 PDF.js 优先；真实处理后预览另接最终文件或原生 renderer。 |
| 文本检测、PDF 转 Markdown | detection.rs、markdown/pdf_to_md.rs | 引入经过验收的提取适配，保留原页号、分页、扫描页提示和取消。 |

首次向导默认勾选 qpdf/Poppler/FFmpeg，但提供跳过；“强制安装”不是准确的产品描述。迁移后应按可用功能调整向导与设置说明，避免原生已可用仍引导全量下载。

## 9. 建议给 Gemini 的实施顺序与验收

1. **原生基础读取与界面预览。** 先迁页数、页信息、书签/签章对象检查，统一几何；PDF.js 离线资源打包与现有 auto 路线一起验证。完成后，干净安装环境应可导入、计数和预览。
2. **打印外观压平。** 复用现成算法并补第 3 节的政策/残留检查。原生后端随后承担 Print 合并；不能通过把业务改成 Structural 绕过打印契约。
3. **证据链、打标、流读写与拆分迁移。** 将旁路 qpdf 调用接入门面，修复 marks 所有权、压缩 DPI 和资源剪枝等已知缺口，保留中文字体与恢复信息。
4. **文本提取适配。** 固定已验证的 PDF Oxide 依赖组合，或验证其他完整提取器。以 WordBox 的坐标、编码和页号契约接既有检测/Markdown 流程。
5. **默认启用与扩展回退。** 内置能力已验证时直接执行；仅对明确的能力不支持或修复限制提示扩展。取消、权限/磁盘错误、输出发布失败不得换引擎重做。任何回退都保持同一输出政策。

最终验收至少包含：

- 在未安装 qpdf/Poppler 的安装包环境中，从 UI 完成导入、计数、预览、合并、提取、打标、证据拆分、检测和文本导出；按已完成的功能明确列出可用范围。
- 允许使用 qpdf/Poppler 作开发期对照工具；这与应用运行时需要它们是不同问题。
- 有效 AP、缺失/过期 AP、可见性标志、孤立 Widget、签章外观、继承几何、中文字体均有输出读回与渲染核对。
- 没有外置工具时，普通输入不会因隐藏的页数/书签/真实预览调用提前失败。
- 不支持的文件返回具体原因；有扩展时回退可观测，没有扩展时提示针对当前文件，原文件与已有输出保持完整。
- 失败、取消、低磁盘空间与大文件处理均不能返回半成品；内存和耗时以实测报告表达，不给未经测量的比例或倍数。

## 10. 复现与交付物

探针源码在 [research/native-first-2026-10-19](research/native-first-2026-10-19/)。核心原生探针依赖完整上游的 cos/organize/edit；单独创建测试 crate 后，将 `native_probe.rs` 作为 main.rs。提取探针复制当前 `extract_poc.rs` 到 extraction_probe.rs 同目录，并加入 lopdf/serde/serde_json。PDF Oxide 探针使用独立 crate，关闭默认特性，并按上游锁文件固定 office_oxide=0.1.9。具体锁文件随证据保存。

合成输入与渲染测量：

```sh
python3 docs/research/native-first-2026-10-19/probe.py generate release-candidate/native-first-2026-10-19
# 执行 native_probe，输出 native/ 文件后再测量
python3 docs/research/native-first-2026-10-19/probe.py measure release-candidate/native-first-2026-10-19
node docs/research/native-first-2026-10-19/pdfjs_probe.mjs release-candidate/native-first-2026-10-19
node docs/research/native-first-2026-10-19/preview_route_probe.mjs release-candidate/native-first-2026-10-19/preview-route.json
```

生成/测量脚本需要测试环境有 qpdf、Poppler、pypdf 和 Pillow；它们不是原生执行探针的依赖。PDF.js/PoC 对照的三个文字样本来自前次审查保留的合成证据目录，路径在脚本和结果中明确记录。结果不是全量应用回归或发布验收。

## 11. 预览选型与 Word 文件处理状态补充

### 11.1 预览方案

当前建议是以现有 PDF.js 完善默认界面阅读器：共享文档实例与相邻页缓存、按屏幕像素密度渲染、限制并发与内存、保持旧帧至新帧完成、补文字选择层，并统一裁剪原点/旋转/UserUnit。真实导出预览应读取最终处理后的 PDF，避免只显示参数模拟层。

| 方案 | 对 Docsy 的作用 | 当前决策 |
| --- | --- | --- |
| PDF.js | 已接入；界面阅读、选择/搜索等能力可通过 Display/Viewer 层继续建设 | 优先完善默认路线；离线资源与安装包 WebView 仍需验收。 |
| PDFium + pdfium-render | 可在后端绘制页面并提取文本；可以随安装包携带原生库，或静态链接 | 若需要统一后端 renderer，值得做兼容性/性能对照；当前项目未接入，不能宣称已优于现有输出。 |
| Hayro + craft-pdf | 符合纯 Rust 方向，已有尺寸限制、取消、警告封装 | 作为原生渲染候选；上游仍自称 experimental，列有非嵌入 CID 字体、knockout 等限制，中文证据样本尤其需要验收。 |

PDFium 随包携带可以免用户另行安装，但仍是 C++ 引擎；`pdfium-render` 本身不包含 Pdfium 二进制。线程安全包装通过互斥锁串行调用，不能把开启 thread_safe 解释为获得并行加速。[PDFium 包装器说明](https://github.com/ajrcarey/pdfium-render)。

[Hayro 上游说明](https://github.com/LaurenzV/hayro)没有承诺全面兼容或更高性能；[PDF.js 分层说明](https://mozilla.github.io/pdf.js/getting_started/)则明确区分渲染 API 与完整阅读器 UI。当前 Canvas 组件的体验不足，不足以证明必须更换解析引擎。

### 11.2 doc 和 docx 处理并不等于原生 Word 排版

本次状态核验执行了以下当前源码测试，共 **20 passed，0 failed**：

| 测试范围 | 数量 | 能证明什么 |
| --- | --- | --- |
| doc2docx::tests | 5 | 含一次实际 sidecar 转换；已确认 sidecar 和 sample.doc 均存在，未走缺失时的跳过分支。 |
| docx_template::engine::tests | 3 | 模板检查、保存、生成及表格当事人列表复制的 OOXML 流程。 |
| markdown::docx_to_md::tests | 7 | 标题、行内格式、列表、表格/合并单元格、文本框及非法输入处理。 |
| pdf::word_convert::tests | 5 | 文件识别、PDF 透传、输出冲突和失败清理；这些测试没有启动 Office，不能算 Word 转 PDF 真机验收。 |

现有实现：

- `.docx/.docm` 的 Markdown 提取和模板处理已有内置路径；原包读取/OOXML 修改与保留页面版式的 PDF 排版是不同能力。
- `.doc` 默认使用随包 doc2x 转为临时 docx，再做模板/提取；Markdown 另有 AnyDoc 直读后备。它不是纯 Rust MS-DOC 解析重写，也不代表直接写回旧版 `.doc`。
- Word 转 PDF 在 macOS 尝试 Microsoft Word；Windows 尝试 Word、WPS；之后尝试 LibreOffice。`.doc` 直接转换失败时，还会经 doc2x 转 docx 后再次尝试这些引擎。
- doc2x 只解决格式中转，没有提供独立的 Word 页面排版/PDF 导出。没有 Word/WPS/LibreOffice 时，当前流程不能仅靠 doc2x 完成 Word 转 PDF。
- 1.0.10 发布证据中保留了一次 macOS Word 转 PDF 成功记录，PDF 结构检查返回兼容警告。当前 word_convert.rs 的 SHA-256 与该发布快照一致。本轮未重新启动 Office；现有记录不足以证明所有 doc/docx、Windows WPS 和 Linux LibreOffice 均已完成实机验收。
- Wordcraft 独立排版引擎尚未接入当前转换链。因此“取消 qpdf/Poppler”与“取消 Word/WPS/LibreOffice”需要分开实施和验收。

本次测试原始输出保存在本节配套证据目录中的 `word-doc2x-tests.log`、`word-template-tests.log`、`word-docx-md-tests.log`，机器可读结果见 `word-status.json`。

### 11.3 后续 Office 转换器与真实字段文件实测

同日后续已实际运行本机 Microsoft Word、office2pdf v0.9.1 和 MiniPdf Rust 发布构建，对照两份带表单域文件及其新增页眉副本。结果更新见 [Office 原生转换器与依赖升级评审](office-converters-and-upgrade-review-2026-10-19.md)。因此上节“本轮未重新启动 Office”仅描述该节先前测试批次；后续已完成 macOS Word 导出对照，Windows/Linux 和安装包 UI 仍未验收。

office2pdf 是现成的纯 Rust OOXML 转 PDF 候选，说明生态里已有可接入方案；当前 Docsy 生产转换链尚未接入。真实样本证明两种原生引擎仍有表单丢失、页眉与分页问题，不能据合成样本成功直接替换系统排版器。
