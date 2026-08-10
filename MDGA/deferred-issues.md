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
- 前端测试覆盖率低：~29.9k 行源码仅 15 个测试文件，template 模块（最复杂）零测试。
- `tauriBridge.js:4-26` 的 `operationLabels` 是硬编码命令→文案映射，新增 Rust 命令需记得回前端补文案。

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
