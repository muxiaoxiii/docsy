# MDGA — 待修改事项记录

> 本文档记录 0.9.7-beta9 全量代码审查中发现、但**暂未修复**的问题，按优先级排序，供后续迭代参考。
> 审查时间：2026-08-10；审查范围：最近提交 `5d033b4` / `6ba0223` + Rust 后端 + 前端整体架构。
> 已在各版本修复的问题见 `CHANGELOG.md`；beta12 完成的条目在下方以 ✅ 标注存档。

---

## 一、Rust 后端架构（重构项，工作量大）

### 1. `header_footer.rs` 上帝模块（P1）
2609 行、约 69 个函数，混合至少 4 个独立关注点：

- 书签增删查（`header_footer.rs:401/489/506`）——与页眉页脚无关，却被命令层当书签模块用（`commands/pdf.rs:314-325`）
- 字体子集嵌入（`header_footer.rs:1255-1795`，约 500 行，含平台字体路径探测 `font_paths_for_family:1713`）
- overlay PDF 构建与文本算子生成（`1058-1186`, `1515-1662`）
- 批量编排与预览（`252-390`）

建议：拆出 `bookmarks.rs`、`overlay_font.rs`，主模块只留编排逻辑。

### 2. 后端内部 JSON-as-IPC，类型边界失效（P1）
- `commands/pdf.rs` 先定义类型化参数结构体，随即 `serde_json::to_value(args)` 抹掉类型（`commands/pdf.rs:165,180,186,...`）；业务层入口签名全是 `pub fn xxx(args: &serde_json::Value)`（`header_footer.rs:241/305/353`、`detection.rs:216/329`、`split.rs:68`、`evidence.rs:360`、`annotations.rs:29`、`preview.rs:42`），内部再反序列化一次——双重序列化 + 编译期契约丢失。
- 后端模块之间也用 JSON 互相调用：`evidence_session.rs:39-43` 手工拼 `json!({"items": ...})` 调 `header_footer::batch_overlay`，返回值再用 `get("results")` 抠出来（`evidence_session.rs:54-65`）。

建议：业务函数直接收类型化 struct（Tauri 命令层本就能自动反序列化参数），JSON 中转完全多余。这是全项目最值得重构的一点。

### 3. `DocsyError` 形同虚设（P2）
`error.rs` 只有 3 个变体，仅 commands 层 4 个文件引用；绝大多数用法是手工 `DocsyError::Unknown { message }`，与 `From<anyhow::Error>` 默认转换结果完全一样。前端拿到的 `kind` 几乎恒为 `"unknown"`，错误分类价值为零。
决断：要么在业务层用 thiserror 真正分类（加密 PDF、页码越界、工具缺失等），要么删掉这个类型直接用 String。

### 4. 三套取消机制并存且覆盖不一（P2）
- `SubprocessRegistry`（PID kill，`lib.rs:22-122`）
- `OperationManager`（CancellationToken，`operations.rs:68`）
- `ConversionState`（Condvar 交互超时，`lib.rs:143-198`）

问题：
- `qpdf.rs:8-18` 的 `run_cancellable` 接了 SubprocessRegistry，但 `page_info.rs:22-26` 直接 `.output()`，页数查询不可取消——同类操作取消能力不一致。
- `SubprocessRegistry::cancel`（`lib.rs:49-74`）按 PID kill：子进程已退出后 PID 可能被系统复用，kill 会误伤无关进程；且注释写 "then SIGKILL if needed" 但 Unix 分支只发了 SIGTERM，没有兜底。
- `evidence.rs:14` 反向依赖 `crate::ConversionState` 和 AppHandle（`evidence.rs:22-27`），pdf 业务层耦合 lib.rs 的 UI 全局状态，破坏分层。应改为注入 trait/回调。

### 6. 双 PDF 引擎并存（P3）
lopdf 与 qpdf 子进程 JSON 混用，同一条流水线反复横跳：`normalize.rs` 先用 qpdf 取页尺寸、再用 lopdf 改内容、再调 qpdf 优化。`qpdf_stream.rs:389` 还维护了 qpdf JSON → lopdf Object 的对象模型桥接，是长期脆弱点。短期可接受，建议收敛"读取"路径到一侧。

### 8. 安全细节（P3，beta12 部分处理）
- `read_image_data_url`（`commands/system.rs`）：beta12 已补 gif/ico 解码与 50MB 上限，但仍接受前端任意路径读取本地文件并返回 base64，是一个任意文件读原语（限于 image crate 可解码格式）。建议加目录白名单或走 tauri fs plugin 的 scope。
- 临时文件权限：beta12 已在 `util/fs.rs` 提供 0600 的 `set_private_permissions`（`#[cfg(unix)]`），但各临时文件点尚未全面接线。

---

## 二、PDF 文本处理（遗留缺口）

### 10. 合成预定义 CMap 不支持混合宽度编码（轻微）
`cmap.rs` `compose_encoding_and_unicode`：codespace 合成 `[0;n]–[FF;n]`（n=最大码长），而 `decode` 要求码长等于 codespace 长度，混合宽度编码（如 `90ms-RKSJ-H` 的 1 字节半角片假名）里的短码永远匹配不到 → 整串解码失败退 bbox。属覆盖缺口而非错误（beta12 已加注释说明）。同函数内 `cid_to_unicode.decode(&cid_bytes)?` 任一 CID 缺失就放弃整个字体映射，偏严。

### 12. 归一化覆盖缺口（轻微，beta12 部分处理）
- ~~半角片假名（`FF61–FF9F`）不归一化~~ ✅ beta12 已在 `text_utils` 对其做 NFKC 归一化。
- 遗留 CJK 编码打分（`artifacts.rs::legacy_cjk_candidate_score`）是启发式：常用字表只覆盖高频文书用字，生僻内容或纯繁体古文仍可能误判编码；韩语常用音节表同理。如出现误判案例，优先扩充对应常用字表。

### 12c. 普通文本删除路径不组合 CTM（已知限制）
`content_text.rs` 的 bbox 兜底匹配在嵌套 Form（带 `cm` 变换）内坐标不组合，导致无法解码的文本在深层 Form 中既匹配不了文本也对不上 bbox。beta11 通过 artifact `/Contents` 通道覆盖了 iText 印章类文件；非 artifact 的深层嵌套文本仍是保守跳过（告警不改动原文）。如需支持，要在 `QpdfStreamUsageSeed` 里累积 CTM。

---

## 三、前端

### 13. 巨型文件需要拆分（P1）
| 文件 | 行数 | 问题 |
|---|---|---|
| `src/modules/evidence-pdf/components/EvidencePdfWorkbench.vue` | 3810 | god component：页眉页脚规则、覆盖层排序、合并导入、预览全塞一个组件 |
| `src/modules/template/composables/useTemplateState.js` | 2579 | 巨型 composable，实质承担了 store 角色，应按域拆分（填充预览/历史/库管理） |
| `src/modules/pdf-tools/views/PdfToolsView.vue` | 1182 | 一个 view 塞了 5+ 个子功能，30+ 个 `ref`，建议按 tab 拆子组件 |
| `src/modules/template/components/TemplateBuildTab.vue` (1517) / `TemplateRenderTab.vue` (1186) / `fieldRowUtils.js` (1198) | — | template 模块是复杂度高发区 |

`src/shared/pdf-tools/composables/useEvidencePdfSession.js`（1083 行）也偏大，但已有测试覆盖，优先级低。

另（beta12 批次 4 新发现的孤儿文件，全仓库无引用，建议删除）：
- `src/modules/evidence-pdf/composables/useContentRowEditing.js`（内含一份带旧 bug 的 `displayContentRowText` 副本）
- `src/modules/evidence-pdf/components/EvidenceOverlayTable.vue`

### 15. 目录/命名一致性（P4，beta12 部分处理）
- ~~`src/components/UndoRedoButtons.vue` 孤零零一个文件~~ ✅ beta12 已移入 `src/shared/components/`。
- `src/shared/pdf-tools/` 名不副实：里面主要是 evidence-pdf 的会话逻辑，建议改名或按真实归属拆分。
- 模块内部布局不统一：home 是扁平结构，其余模块用 `views/` 子目录。

### 16. 工程化小项（P4，beta12 部分处理）
- ~~eslint globals 白名单缺 `fetch` 等~~ ✅ beta12 已补 5 个；仍可考虑引入 `globals` 包的 `globals.browser` 彻底替代手维护。
- `vite ^5.2.0` 偏旧（Vite 6/7 已发布），与 `@vitejs/plugin-vue ^5` 自洽，升级可作技术债记录。
- 事件总线靠裸字符串 CustomEvent（`docsy-operation-*`、`docsy-template-library-changed`），事件名散落多处，建议集中定义事件名常量。
- 前端测试覆盖率低：~29.9k 行源码仅 15 个测试文件；template 模块本轮已补 3 个测试文件（21 条用例），其余模块仍待补。
- `tauriBridge.js:4-26` 的 `operationLabels` 是硬编码命令→文案映射，新增 Rust 命令需记得回前端补文案。
- **前端预览与后端渲染坐标不一致**（beta13 后调查确认）：`pdfPreviewCoordinates.js` 的页眉页脚预览用固定 36pt 边距（后端用 marginMm）、固定像素字号（后端绝对 pt）、无数据时回退 A4 尺寸（异形页百分比定位失真）。预览无法反映横向溢出/收拢。要对齐需把预览换算改为与 `header_footer.rs` 同一套参数。
- **旋转页（Rotate=90/270）overlay 坐标可能错位**：`page_info.rs` 把宽高互换后写 overlay 页，但 base 页 Rotate 标志仍在，viewer 二次旋转；A4 规范化会顺带消除旋转所以规范化路径不受影响。未遇到实际案例，暂记。

---

## 四、Markdown ↔ docx（Unreleased 新增功能的已知边界）

- **版式保真是 Markdown 的格式天花板**：合并单元格、图片精确位置/大小/文字环绕、文本框、分栏等版式信息经过 Markdown 必然丢失（pandoc 同理）。如用户场景要求版式不丢，应改走 docx↔HTML 路线，需另行设计。
- **undoc 备选评估**：`undoc` crate（仅 .docx/.xlsx/.pptx → Markdown/文本/JSON，单向，不支持 legacy .doc）docx→md 方向覆盖比自研略广（脚注/尾注、页眉页脚、合并单元格列对齐、下划线、文本框、xlsx/pptx 提取）；2026-08 评估时 v0.8.0 发版仅数天，暂不引入。若实际文档撞上上述边缘 case，可引入或照其思路补齐自研实现。
- **.doc 高保真**：已实现 Word/WPS 自动化引擎（Windows COM `Word.Application`/`KWPS.Application` SaveAs 16；macOS AppleScript Word `format document default`），作为 .doc 弹窗的可选项。已知限制：macOS 只走 Word（WPS for Mac 无 AppleScript 支持，与证据模块 doc→pdf 一致）；Word/WPS 首次启动可能较慢（120s 超时）。LibreOffice 方案因体积被用户否决，不再考虑。docx_template/engine.rs 的 .doc 路径仍是 office_oxide 纯文本保真度，如模板模块涉及带表格/图片的 .doc 可复用同一 word 引擎。
- docx→md 引用块无法还原为 `>`（写入侧只做缩进）；有序列表起始号反向一律从 1 重编；单元格内嵌套表格压平；远程/缺失图片输出占位文本。
- MD→docx 的样式目前固定（Times New Roman/宋体 docDefaults），如用户需要模板化样式（红头文件格式等）再立项。
- README.md:68 仍有「## 🔧 PDF 工具」章节标题，下次更新 README 时同步为「文档工具」。

---

## 五、模板模块（本轮深度审查后仍未修项）

> 审查/修复时间：2026-08-11；已修条目见 CHANGELOG [Unreleased]「模板模块深度修复」。以下为审查中发现、本轮未处理或未能彻底处理的问题。

### 前端
- **自动推断的同名合并需手动改名**：构建模板时两处同名字段（如两个"日期"）不做自动序号区分，需用户手动改名后才能各填各值。
- **`renderableTemplateFields` 同名异类型去重分支语义混乱**：同名但类型不同的字段去重逻辑边界不清晰，遇到时需先梳理再动。
- **`buildFillPreview` 读 formValues 快照**：固定来源的 reference 字段在填充预览里显示占位符而非解析值（纯显示问题，实际渲染正确）。
- **旧模板存量自引用 reference 字段不迁移**：历史模板里已存的自引用字段实时解析后能取到值，但 UI 显示只读，不做数据迁移。
- **批量路径未走 historyValues**：批量填充时历史记录值来源与单条渲染路径不一致。
- 引用 slot key 不一致（`id` vs `id#0`）；DocumentPreview 不分段；`useBatchFill.js:139` 批量保存对话框死状态；`TemplateRenderTab.vue:481` 文件名预览恒占位；预览 watch 覆盖面不足；`getPartyListRows` 临时行丢失；长度单位码元/码点混用（`useTemplateState.js:1834`）；日期 iso 留空输出 "-  -"（`fieldRowUtils.js:759`）；`inspectSourceDocx`/`editTemplateFromLibrary` 无请求序号、存在竞态。

### 后端
- 日期校验过松（`batch.rs:391`）；ooxml 解析静默丢属性/CDATA（`ooxml.rs:121/163`）；同一表格行出现第二个 party_list 时退化处理（`render.rs:219`）；分隔符剥离只认整节点（`render.rs:609`）；validate 的 total_rows 计数偏大（`batch.rs:364`）；docProps 剔除可能产生悬空关系（存疑）；`to_xml` 不写 XML 声明（存疑）。

---

## 存档：beta12 已完成条目
- ✅ **5. 重复工具代码 / 临时文件泄漏**：新建 `util/fs.rs` 收敛 `same_path`/`temp_named_path`/`safe_file_stem`/`unique_output_path`/`TempPathGuard`/`set_private_permissions`；顺带修复 header_footer 旧版撞名超限会覆盖原文件的隐患。
- ✅ **7. 业务逻辑泄露到命令层**：`MANIFEST_CACHE` 下沉 `docx_template/mod.rs`；`AntiCopyMethod::parse` 下移。
- ✅ **9. 小问题**：`pdf/overlay.rs` facade 已删除；`pdf/mod.rs` 15 个子模块改 `pub(crate)`；`glyph_names.rs` 标注 GENERATED；`error.rs` 加 `#[non_exhaustive]`；`glyph_to_char` 多码点 uni 名返回 None（条目 11）。
- ✅ **12a. 预览页脚短路**：`previewFooterText` 去掉 `existingFooterText` 短路，与 `buildHeaderFooterItems` 同源。
- ✅ **12b. `useHeaderFooterRules.js` 死代码**：已删除。
- ✅ **14. 错误展示不统一**：38 处后端 error 统一过 `userFacingError`。
- ✅ **单独编号页码列表显示 bug**（用户报告）：`buildFileContentRows` 页码行携带规则覆盖后的 `effectiveGroup.sequence`，`displayContentRowText` 优先使用，列表与实际/预览渲染一致。
- ✅ **per_file 页眉序号起点设置**（用户报告）：`perFileSeqStart` 字段贯通预览与实际生成，UI 在 per_file 模式提供"序号起始"输入（0–9999）。
