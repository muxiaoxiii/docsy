# Docsy 代码与算法审查报告

- **版本**: 0.9.1（`package.json` / `src-tauri/Cargo.toml`）
- **审查日期**: 2026-08-04
- **审查方式**: 只读代码审查（前端 src/ ~20,400 行、Rust src-tauri/src/ ~20,600 行、docs/ 4 份设计文档、CHANGELOG），未运行任何测试、编译或构建
- **审查目标**: 判断代码与算法是否实现设计目的（对照 docs/ 设计文档与 README）

> 说明：本报告中的高/中危条目均已逐一回读源码验证过 file:line；低危条目以「按报告核对」标注的为已在审查中抽验，其余来自全量通读。所有行号为审查当日源码。

---

## 1. 总体结论

项目整体质量较高：**核心算法基本都实现了设计目的，且多数实现比设计文档所描述的更完整**。模板引擎（quick-xml 树操作 + w:tag 稳定定位 + sdt 包裹 + 空值擦除）、PDF 证据处理（三层检测、Artifact 语义删除、全局连续页码、A4 规范化保留文本方案）、图片排版（坐标计算经核验正确）、批量填写、工具托管等均已按设计落地，未见用户输入触发的 panic 路径、未见命令注入（外部命令全部参数数组方式）、前端无 v-html/内联 HTML（无 XSS 面）、zip 读取有大小预检与 `take(limit+1)` 兜底。

存在 3 个需要正视的问题：

1. **取消功能整体失效**（前端「取消」按钮、CHANGELOG 声称的「超 30s 取消」均无效）——`SubprocessRegistry` 从未有进程注册，`cancel_operation` 恒返回 false；
2. **模板扫描/保存的 run 索引契约存在不一致**——段落中出现「有 `w:t` 元素但无文本」的 run 时，保存阶段坐标整体偏移，生成文档错位（数据错误）；
3. **「清除现有页眉页脚」对未检测内容实际无效**，且设计文档声称的「区域覆盖兜底」并不存在——行为是保守的（避免误删），但文档与实现矛盾，用户预期可能落空。

文档层面：`pdf-evidence-processing-design.md` 有相当多「尚未实现」清单已过时（实际已实现），A4 规范化方案描述与实际实现相反；`template-system-design.md` 缺失 `reference` 字段类型；README「7 种字段类型」与列出的 8 个名字不符。

---

## 2. 各模块设计目的达成度

| 模块 | 达成度 | 说明 |
| --- | --- | --- |
| 文书模板引擎 | 高 | 扫描/保存/渲染闭环完整，w:tag 稳定定位、同 run 多字段拆分、前后缀擦除、表格行复制均按设计实现；见 4.1-H2 的索引一致性风险 |
| PDF 证据处理 | 较高 | 三层检测、Artifact 语义删除、连续页码、A4 规范化、拆分/合并全部落地；「区域覆盖兜底」未实现（见 4.1-H3） |
| PDF 基础工具 | 高 | 解锁/合并/提取/压缩/拆分均基于 qpdf，行为与设计一致 |
| 图片排版 | 高 | fit/fill/original 三模式、A4 排版坐标（含 `cell_y_mm` 底边计算）经核验正确，非镜像 |
| 视频抽帧 | 高 | FFmpeg 检测/探测/抽帧/水印完整；文件名时间戳为估算（见 4.2-M8） |
| 设置/工具管理 | 较高 | 外部工具检测/托管安装、菜单排序、回收站、诊断齐全 |
| Doclet 动画 | 中 | 动画与 350ms 防闪烁正常；但配套的「取消」机制未接线（见 4.1-H1） |

### 2.1 已确认正确的核心算法（亮点）

- **标黄扫描**：`markRefs` 按 Unicode 字符序切分（防中文被 UTF-8 字节切坏）；递归进入文本框 `txbxContent` 与嵌套 `w:sdt`；单 run 多 `w:t` 合并为一个可选中节点（scan.rs:91-96 注释明确了该契约）。
- **模板保存**：`w:tag` 用字段 id 包裹 `<w:sdt>`（不按段落/run 序号定位）；部分选中按字符范围切分为「前缀 run + sdt + 后缀 run」；同 run 多字段按 start 排序逐个切分并跳过产物保持后续坐标（save.rs:349-361）。
- **渲染**：按 tag 查字段；值写入 sdtContent 首个 `w:r` 的 `w:t` 并保留全部 `rPr`；勾选/单选/多选只改 marker 内 `w:t` 不动 `w:rPr`（兼容 Wingdings/Symbol）；空值按 optionalRule 逆序收集兄弟文本节点擦除前缀/后缀（`被告甲公司，第三人[字段]之间纠纷` → `被告甲公司之间纠纷`）；party_list 在正文按顿号连接、表格行整行复制。
- **页码全局连续编号**：按最终顺序全局计算（证据 2 首页为 `13/200` 而非 `1/8`），与设计文档一致；`{page}/{total}/{range}` 与自定义符号展开正确。
- **Artifact 删除**：BDC/EMC 配平区间整体删除、支持内联属性字典与命名属性引用、Form XObject 嵌套（深度上限 8）；**无 Subtype 的 BMC 不处理**（防误删正文）——符合「风险分级」设计。
- **A4 规范化**：采用内容流复合矩阵缩放（`q…Q` 包裹 + 重设 MediaBox/CropBox + 移除 Rotate），**保留文本可选中性**，比设计文档所述方案更优（文档滞后，见 §6）。
- **批量填写**：xlsx 第 0 行（隐藏行）存 `tpl_id/field_id/type` 元数据、按元数据/表头匹配字段、逐行校验日期与必填、`{N}`/`{fieldName}` 占位符文件名、逐行渲染。
- **健壮性**：生产路径无用户输入触发的 `unwrap`（仅编译期常量正则）；外部命令全部 `Command::arg` 数组传参（无 shell 注入）；`fs::copy`/输出名多走 `unique_*_path`。

---

## 3. 缺陷清单

严重度定义：**高** = 功能错误 / 输出数据错误 / 崩溃；**中** = 边界场景未处理、结果与预期不一致；**低** = 代码质量、能力缺口、死代码。

### 3.1 高

**H1. 取消功能整体失效（前端取消按钮、CHANGELOG「超 30s 取消」均为 no-op）**
- `src-tauri/src/lib.rs:20-82` 定义 `SubprocessRegistry`（`register/unregister/cancel`），但全工程对 `register`/`unregister` **零调用点**（已 grep 证实，仅 lib.rs:156 构造并 `manage`）；`commands/system.rs:138-143` 的 `cancel_operation` 因此恒返回 `false`。
- 前端 `src/App.vue:157-175` 的取消按钮只清掉动画，后台子进程继续运行；`src/modules/pdf-tools/views/PdfToolsView.vue` 等处的「取消」同样无效。
- 后果：长任务（合并/批量/拆入）无法中途取消，与 CHANGELOG 0.9.1「超 30s 取消」描述不符。
- 修复方向：在 `qpdf/ffmpeg` 子进程启动处注册 pid 到 registry，`cancel` 发送终止信号；或移除前端取消入口改为真实进度上报。

**H2. 模板扫描/保存的 run 索引契约不一致，含空 `w:t` 的段落会导致字段错位**
- `scan.rs:95-116`：仅当 `collect_direct_run_text` 非空时才递增 `run_idx`（即「无文本 run 不占坐标」）。
- `save.rs:309` 与 `371`：用 `run_has_text`（save.rs:624-635）判断——只要存在 `w:t` 元素就计数（**与文本是否为空无关**）。
- 差异：段落中若存在 `<w:t></w:t>` 的空 run（WPS/部分工具生成的 docx 可能出现），scan 跳过它、save 计数它，导致该 run 之后**所有** run 坐标整体偏移 1。`coord_map` 键不匹配 → 字段被包裹到错误的 run 或找不到 → 标黄原文残留为正文（黄被剥离但文本未替换），生成的 docx 文本错位。
- 前端 `TemplateView.vue:2101-2108` 的 `shouldMergeMark` 假设 run 索引连续（`next.runIndex === current.runIndex + markRefs.length`），受同一问题影响。
- 修复方向：统一判定函数——两侧都用「存在非空 `w:t` 文本」或都按元素存在性计数，并补一个「段落含空 run」的往返测试。

**H3. 「清除现有页眉页脚」对未检测内容无效；设计文档声称的「区域覆盖兜底」未实现**
- `src-tauri/src/pdf/header_footer.rs:341-351`：清理请求（`cleanup.header_enabled/footer_enabled` 等）但语义删除计数为 0 时，只 push 一条警告「原文未被遮盖或改写」，不做白底覆盖。
- 设计文档 `docs/pdf-evidence-processing-design.md:533`、`:553` 声称「若没有可删的标准标记，仍继续使用区域覆盖兜底」「非标准内容会区域覆盖兜底」——**与代码矛盾**（前端 `EvidencePdfWorkbench.vue:968` 的提示文案反而如实说明「不会使用白色遮盖」，说明前端是知情的，文档过时）。
- 后果：用户勾选「清除页眉区域」时，若旧页眉既非标准 Artifact 又未被检测确认，则**完全不会删除**，仅后端一条警告。行为保守（不会误删正文）但预期可能落空，且文档误导。
- 修复方向：二选一——补白底覆盖（风险：覆盖正文区域）；或改文档并让前端更醒目地提示「未确认内容不会被清除」。

### 3.2 中

**M1. 多行字段值换行丢失**
- `render.rs:640-697`（`render_into_existing_runs`）：含 `\n` 的值原样写入单个 `w:t`，OOXML 中字面换行在 Word 里渲染为空格（应使用 `w:br`）。
- 后果：地址、备注等多行文本的折行丢失。
- 修复方向：按 `\n` 拆分，在 `w:t` 之间插入 `w:br`（注意跨 run 保留 rPr）。

**M2. A4 规范化假设 MediaBox 原点为 0；`_dpi` 参数被忽略**
- `normalize.rs:65-86`：`fit_matrix` 的居中平移以 (0,0) 为原点计算；非零原点页面（如 `[20 30 w h]`）缩放居中后会整体偏移甚至裁切。
- `normalize.rs:9`：`_dpi` 形参从未使用（重渲染路径未落地）。
- 修复方向：按页面 MediaBox 的实际 `llx/lly` 修正平移量。

**M3. 内容流重写后无结构校验（设计文档 116 行要求配套 `qpdf --check`）**
- `content_text.rs:125-131`、`artifacts.rs:450-458`：整段 `Content::encode()` 重写后直接保存，未跑 `qpdf --check`；`qpdf.rs` 中不存在 check 辅助函数，`--check` 仅测试用（header_footer.rs:2010）。
- 后果：遇到图形状态不平衡/未知操作符的 PDF，重写后可能产出损坏文件且无反馈。
- 修复方向：重写路径统一走「qpdf 校验 + 失败回退原文件」封装。

**M4. 页数刷新竞态：新导入文件可能保持 `pages:0`**
- `EvidencePdfWorkbench.vue:1375-1396`：`refreshOverlayPageCounts` 用 `if (checkingOverlayPages.value) return` 直接短路；`selectOverlayFiles:1330-1342` 先替换 `overlayFiles` 再 `await refreshOverlayPageCounts()`。
- 后果：上一次刷新未结束时新文件 `pages:0`，后续页眉检测、总页数、页码预览全部按 0/1 页处理，可能生成错误页段。
- 修复方向：改为「最后发起者」token 或串行队列，而不是直接短路。

**M5. 页眉检测防重入守卫吞掉新文件的检测**
- `useEvidencePdfDetection.js:25`：`detectingAllHeaderFooter.value` 为 true 时直接 return；`selectOverlayFiles` 中紧随的 `detectAllHeaderFooter({silent:true})` 可能被跳过。
- 后果：新文件未做旧页眉页脚检测就进入处理流程（与 M4 叠加时更明显）。

**M6. 中文数字两套实现且 ≥100 退化**
- `src/core/numberFormat.js:3-14`：仅支持 1~99，≥100 直接返回阿拉伯数字。
- `src/modules/pdf-tools/composables/pdfPageNumberRules.js:139-157`：另一套实现，支持到 9999。
- 后果：文档页数 ≥100 时「中文页码/中文序号」风格退化，且两个模块行为不一致。
- 修复方向：统一收敛到一套（如 pdfPageNumberRules 的实现），core 版本删除或复用。

**M7. 页码样式实时预览与实际输出不一致**
- `useEvidencePdfPreview.js:63-73`：`previewFooterText` 用 `expandPlaceholders`（只替换 `{page}` 数字），**不按 `pageNumberStyle` 转换**。
- 汇总「页码样例」`EvidencePdfWorkbench.vue:943-955` 与表格列 `:1733-1740` 用 `renderPageNumberTemplate`（按样式转换）。
- 后果：中文/罗马数字样式下，overlay 预览与实际输出页码不同。

**M8. 抽帧文件名时间戳为估算值**
- `ffmpeg/extract.rs:306-311`：文件名时间按 `timeline_offset_seconds + idx / fps` 推算，未取实际帧 PTS；`-ss` 输入定位会吸附关键帧并丢帧，导致文件名时间与水印时间漂移。
- 修复方向：用 `-vf` 侧 filter（`select`/`showinfo` 或 `metadata` 水印）或 `-frame_pts` 命名，取真实 PTS。

**M9. 表格行内 party_list 多槽位渲染为空**
- `render.rs:184-230`（`try_expand_table_row`）：复制行时只为**第一个**找到的 party_list 字段插入单项值（`{text,suffix}`）；该字段在同一行有其他 `mark_refs` 槽时，`rendered_base_text`（render.rs:317-345）按 `items.len()==1`、slot>0 取不到值 → 空。
- 后果：同一表格行中同一当事人字段出现多次（如原被告在不同单元格引用同一字段）时，复制行仅第 0 槽有值。
- 修复方向：行复制时把「字段值 = 单项 + 槽位序号」一并传入并按槽位展开。

**M10. 转换超时等待线程无上限轮询；ConversionState 为全局单例**
- `lib.rs:120-142`：`wait_for_user_response` 无限循环（500ms 轮询），前端若不响应（窗口关闭/异常）该线程永久阻塞。
- `ConversionState`（lib.rs:94-101）全局单例：两个长任务并发时会互相覆盖 `timed_out/response`。
- 修复方向：轮询加超时上限；超时状态改为按任务 id 隔离或加锁。

**M11. 处理后文件状态语义问题（两处）**
- `EvidencePdfWorkbench.vue:1484-1486`（页眉页码替换流程）：成功后 `file.path = success.outputPath`，**原路径被覆盖丢失**；再次「生成处理后文件」会基于上一次产物继续处理，状态语义易误读（输出名冲突由后端 `unique_output_path` 兜底，但不透明）。
- `EvidencePdfWorkbench.vue:1541-1557`（证据处理流程）：只更新 `file.outputPath`，不更新 `file.pages`；A4 规范化改变页数后 `totalOverlayPages` 仍按旧值。

### 3.3 低

- **L1** `ooxml.rs:47-48`：`std::str::from_utf8` 直接拒绝非 UTF-8 XML part（OOXML 规范允许 UTF-16），quick-xml 的 encoding 特性从未生效；遇 UTF-16 part 直接报错。
- **L2** `detection.rs:881`：独立页码正则 `^\d{1,4}$`，5 位以上页码不被识别（同文件 1109 行另有 `\d{1,6}` 规则，两处不一致）。
- **L3** `scan.rs:66-72`（仅递归 `w:sdt`）/ `save.rs:300-305`（非 `w:r` 直接跳过）：`w:hyperlink`/`w:smartTag` 内 run 两侧均不可见——链接内标黄无法建字段（行为一致，属能力缺口）。
- **L4** `batch.rs:408-410`：注释「0-based row indices relative to data rows starting at row 2」措辞与实现易混淆（前端按 `e.row+1` 展示正确，仅注释误导）。
- **L5** `evidence.rs:743-764`（`apply_identity_rename`）：`fs::copy` 静默覆盖 `_renamed/` 下同名文件；`start + i as u32` 理论溢出（实际不可达）。
- **L6** `sort_utils.rs:3-44`：非 ASCII 按字节序比较，且全链路无 NFC/NFD 归一化——macOS 上相同中文名的不同规范形式排序/匹配可能不一致。
- **L7** `header_footer.rs:743-864`：每次构建 overlay 都重建内置字体对象与 CJK 子集；allsorts 取 `.ttc` 首 face，失败仅告警降级。
- **L8** `detection.rs:15`：拆分分析硬限 600 页，超过只提示不分析。
- **L9** `batch.rs:331-354`：`is_valid_date_text` 每次调用重建正则，轻微性能损耗。
- **L10** 死代码：`src/stores/app.js`（`useAppStore`）、`src/services/devTracker.js`（`installDevTracker`，main.js 未调用）、`moduleRegistry.js:47-53`（`getModule`/`getModuleSettings`）、Rust `get_module_registry` 命令（`commands/mod.rs:54`，前端从不调用）。
- **L11** `PdfToolsView.vue:456-468`：合并输出恒为 `<dir>/merged.pdf`，同名文件**直接覆盖**无确认/去重。
- **L12** `TemplateView.vue:3776`：`typeof selected === 'string' ? selected : selected` 恒等空操作。
- **L13** `PdfJsPreview`（pdfjs-dist 回退分支 `loadDocument`）整文件读入内存，超大 PDF 首帧渲染慢（仅自动回退路径触发）。
- **L14** `ReorderableImageGrid.vue:15-35`：拖动中分页不变且 `elementFromPoint` 只命中当前页，**跨页拖拽不可用**（只能靠上下按钮）；`ImagePaddlerView.vue:429-437` 的 `reorderLayoutImages` 基于已按 Z/N 重排的序列再 `moveItem`，非 Z 模式下视觉顺序与数组语义易混淆。
- **L15** `usePointerReorder.js:22-24`：合成事件 `pointerId` 为 null 时 `setPointerCapture(null)` 可能抛 `NotFoundError`（未 try/catch，与 `finish` 的释放不同）。
- **L16** `VideoExtractView.vue:315-321`：`e.dataTransfer.files[0].path` 在 macOS WKWebView 不可用（Tauri v2），该 drop 分支基本失效；窗口级 `useWindowFileDrop` 可兜底，但可能双触发重复 probe。
- **L17** 两套「转换超时」与「取消」机制（`respond_conversion_timeout` / `cancel_operation`）各自独立且都未真正取消子进程，职责边界不清。

---

## 4. 前端 ↔ Rust 对接核对

- `commands/mod.rs:20-90` 注册的 49 个命令与前端调用逐一对应，**全部匹配**；参数经 Tauri 默认 `rename_all` 转换（`templatePath→template_path`、`operationId→operation_id` 等）。
- `run_image_paddler` 单参 `args`、`split_merged_evidence_pdf`/`apply_evidence_pdf_rules`/`preview_pdf_header_footer`/`render_pdf_preview` 等 `serde_json::Value` 命令前端统一 `{args:{...}}` 展开，一致。
- 响应字段（`SplitOutput`/`HeaderFooterResult`/`PreviewResult`/`BatchValidationResult`/`BatchRenderResult`/`ToolStatus` 等）camelCase 序列化与前端读取一致。
- **唯一不匹配点**：`cancel_operation`——签名正确但后端无注册逻辑（H1），表现为「取消无效」。

---

## 5. 文档问题清单（建议更新）

1. `pdf-evidence-processing-design.md:533/553` 声称「区域覆盖兜底」——未实现（H3）；`:211` 说「当前稳定实现可以先覆盖区域」亦与实际不符。
2. `pdf-evidence-processing-design.md:489` 声称「尚未实现对象层删除和内容流安全编辑」——实际 artifacts.rs/content_text.rs 已实现，文档过时。
3. A4 规范化：文档（`pdf-evidence-processing-design.md:269-274`、`:490`）声称当前是「重渲染（pdftoppm 重建）」且「保留文本方案未实现」——实际实现是内容流矩阵缩放（保留文本），方向相反，文档需改写。
4. `pdf-evidence-processing-design.md` 中「尚未完成收敛」清单（07-13 版）多项已实现（Artifact 删除、确认窗口、页码分类），建议清理。
5. `template-system-design.md` 字段类型未含 `reference`，但实现（`mod.rs:452`）与前端均支持——设计文档缺失该类型定义。
6. README「7 种字段类型」与列出的 8 个名字（含 `reference`）不符。
7. README 页码样式漏掉实心圈 `❶`（0.8.6 引入）及「键帽样式不暴露」限制。
8. CHANGELOG 0.9.1 声称的「超 30s 取消」未实现（H1）。
9. `docs/architecture.md` 版本号停留在 v0.9.0，未更新 0.9.1。

---

## 6. 后续修复建议（按优先级，本次未实施）

| 优先级 | 条目 | 影响面 |
| --- | --- | --- |
| P0 | H1 取消机制接线（或移除前端取消入口） | 所有长任务 UX |
| P0 | H2 run 索引判定统一 + 空 run 往返测试 | 模板保存正确性 |
| P1 | H3 补白底覆盖或修订文档与前端提示 | 页眉页脚清除预期 |
| P1 | M6/M7 中文数字与页码预览统一 | 页码样式一致性 |
| P1 | M4/M5 前端竞态守卫改为串行/最后发起者 | 证据导入正确性 |
| P2 | M1 换行渲染、M2 原点偏移、M3 qpdf --check 兜底 | 输出质量 |
| P2 | M8 抽帧真实 PTS、M9 party_list 多槽位 | 数据正确性 |
| P3 | L1-L17 低危项按需清理（死代码、注释、归一化等） | 维护性 |

---

*报告完。全部结论基于审查日源码与文档，未运行任何测试。*
