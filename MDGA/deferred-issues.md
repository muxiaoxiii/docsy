# MDGA — 待修改事项记录

> 本文档记录 0.9.7-beta9 全量代码审查中发现、但**暂未在本版本修复**的问题，按优先级排序，供后续迭代参考。
> 审查时间：2026-08-10；审查范围：最近提交 `5d033b4` / `6ba0223` + Rust 后端 + 前端整体架构。
> 已在 beta9 修复的问题见 `CHANGELOG.md`，不在此列出。

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

### 5. 重复工具代码 / 临时文件泄漏（P2）
- `unique_output_path` 有 6 份：`pdf/mod.rs:130`（pub）、`pdf/qpdf.rs:338`、`pdf/qpdf.rs:355`、`pdf/header_footer.rs:782`、`docx_template/batch.rs:811`、`image_paddler.rs:873`。
- `TempPathGuard` 有 3 份：`docx_template/engine.rs:294`、`pdf/header_footer.rs:1951`、`external/managed.rs:340`。而多数临时文件点（`annotations.rs:100`、`normalize.rs:25-27`、`artifacts.rs`、`content_text.rs:84`、`header_footer.rs:367/419/507`、`split.rs:163`）仍是手动 `let _ = remove_file`，错误/取消路径会泄漏临时文件到系统 temp 目录（PDF 内容可能敏感）。

建议：建 crate 级 `util/fs.rs` 统一收敛 + 全面改用 RAII guard。

### 6. 双 PDF 引擎并存（P3）
lopdf 与 qpdf 子进程 JSON 混用，同一条流水线反复横跳：`normalize.rs` 先用 qpdf 取页尺寸、再用 lopdf 改内容、再调 qpdf 优化。`qpdf_stream.rs:389` 还维护了 qpdf JSON → lopdf Object 的对象模型桥接，是长期脆弱点。短期可接受，建议收敛"读取"路径到一侧。

### 7. 业务逻辑泄露到命令层（P3）
- `commands/template.rs:9-41`：`MANIFEST_CACHE`（带 mtime 失效的模板清单缓存）是业务缓存，应下沉到 `docx_template`。
- `commands/pdf.rs:288-291`：method 字符串 → 枚举的映射留在命令层（轻微）。

### 8. 安全细节（P3）
- `read_image_data_url`（`commands/system.rs:119-122`）接受前端任意路径读取本地文件并返回 base64，是一个任意文件读原语（限于 image crate 可解码格式）。建议加目录白名单或走 tauri fs plugin 的 scope。
- 临时文件权限未收紧（系统 temp 目录默认权限），处理敏感 PDF 时建议 0600。

### 9. 小问题（P4）
- `pdf/overlay.rs` 是 8 行兼容 facade，迁移完成后应删除。
- `pdf/mod.rs` 里 `annotations`/`evidence`/`split` 等全是 `pub`，实际只需 `pub(crate)`。
- `glyph_names.rs` 4627 行是生成的字形表，建议文件头标注 "generated" 或拆到 `data/`。
- `error.rs` 的枚举缺 `#[non_exhaustive]`。

---

## 二、PDF 文本处理（本次修复的遗留缺口）

### 10. 合成预定义 CMap 不支持混合宽度编码（轻微）
`cmap.rs` `compose_encoding_and_unicode`：codespace 合成 `[0;n]–[FF;n]`（n=最大码长），而 `decode` 要求码长等于 codespace 长度，混合宽度编码（如 `90ms-RKSJ-H` 的 1 字节半角片假名）里的短码永远匹配不到 → 整串解码失败退 bbox。属覆盖缺口而非错误。同函数内 `cid_to_unicode.decode(&cid_bytes)?` 任一 CID 缺失就放弃整个字体映射，偏严。

### 11. `glyph_to_char` 对多单元 uni 名处理不符 AGL 规范（轻微）
`glyph_names.rs:4574-4584`：`uni4E2D4E2E` 这类连字名只取前 4 位返回 `'中'`，按规范应整体映射或返回 None（返回 `char` 的签名本身表达不了连字）。

### 12. 归一化覆盖缺口（轻微）
- `normalize_for_match` 的全角映射只覆盖 `FF01–FF5E`（全角 ASCII）；半角片假名（`FF61–FF9F`）不归一化，除非输入碰巧含 RTL 展示形式触发了 NFKC。
- 遗留 CJK 编码打分（`artifacts.rs::legacy_cjk_candidate_score`）是启发式：常用字表只覆盖高频文书用字，生僻内容或纯繁体古文仍可能误判编码；韩语常用音节表同理。如出现误判案例，优先扩充对应常用字表。

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

### 14. 错误展示不统一（P3）
封装层（`src/core/tauriBridge.js`）本身做得好，但展示层一半走 `userFacingError()`（全项目仅 21 处），一半直接 `ElMessage.error(result.error || '...')` 把 Rust/anyhow 原始错误串弹给用户（如 `VideoExtractView.vue:280,288,333`、`PdfToolsView.vue:652,709,738,885`）。建议约定：凡 `res.error` 上屏前必须过 `userFacingError`。
另：`tauriBridge.js:4-26` 的 `operationLabels` 是硬编码命令→文案映射，新增 Rust 命令需记得回前端补文案。

### 15. 目录/命名一致性（P4）
- `src/components/UndoRedoButtons.vue` 孤零零一个文件，与 `src/shared/components/`（7 个组件）职责重叠，两处"共享组件"目录应合并。
- `src/shared/pdf-tools/` 名不副实：里面主要是 evidence-pdf 的会话逻辑，建议改名或按真实归属拆分。
- 模块内部布局不统一：home 是扁平结构，其余模块用 `views/` 子目录。

### 16. 工程化小项（P4）
- `eslint.config.js:15-27` 手维护浏览器 globals 白名单，缺 `fetch`、`FileReader`、`Blob`、`FormData`、`AbortController` 等；建议引入 `globals` 包的 `globals.browser`。
- `vite ^5.2.0` 偏旧（Vite 6/7 已发布），与 `@vitejs/plugin-vue ^5` 自洽，升级可作技术债记录。
- 事件总线靠裸字符串 CustomEvent（`docsy-operation-*`、`docsy-template-library-changed`），事件名散落多处，建议集中定义事件名常量。
- 前端测试覆盖率低：~29.9k 行源码仅 5 个测试文件，template 模块（最复杂）零测试。
