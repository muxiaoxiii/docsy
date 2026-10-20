# Docsy 1.1.1 候选版 PDF 原生优先（Native-First）实施与验收报告（2026-10-19）

> 依据文档：[Gemini 实施与验收计划](pdf-engine-implementation-plan.md)、[代码研究与实施要求](pdf-native-first-code-review-2026-10-19.md) 及 [Office 转换引擎对照报告](office-converters-and-upgrade-review-2026-10-19.md)
> 阶段状态：**阶段 0 ~ 4 Review 契约（全部 P1/P2）及 Native-First 改造全部落实并通过验证**
> Word 转换策略：**`src-tauri/src/pdf/word_convert.rs` 保持系统 Word / WPS / LibreOffice 保真转换管线不变（未修改）**
> 版本一致性：**`1.1.1` 全仓版本检查通过 (`node scripts/check-release-version.mjs`)**

---

## 一、1.1.1 候选版核心架构决策与边界确认

1. **DOCX / DOC 转 PDF 保持现有系统办公引擎管线不变**：
   - 根据 Codex 对复杂法律文书（新增页眉 DOCX / DOC、含 `FORMCHECKBOX` 复选框与 `FORMTEXT` 表单文字）的实测对照，`office2pdf v0.9.1` 与 `MiniPdf 0.7.0` 均出现新增页眉与正文重叠/缺失、复选框消失、表单文字丢失及页数偏差（5 页变 6/8 页、10 页变 14 页）。
   - 因此，[`src-tauri/src/pdf/word_convert.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/word_convert.rs) **未作任何改动**，继续优先使用系统 Microsoft Word / WPS / LibreOffice 保真转换。

2. **PDF 原生优先（Native-First）+ 外置 `qpdf` 受控兜底**：
   - **普通合并与页面提取默认走原生 `PdfCraftBackend`**：[`select_backend`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/mod.rs) 已切换为默认返回 `BackendType::PdfCraft`，普通合并（`merge_pdfs`）与页面提取（`extract_pdf_pages`）直接走纯 Rust 原生路径；仅在命中 `DocsyAntiCopyBackup` 打印合并、`NeedAppearances=true` 过期表单外观或需用户密码等场景时，由 `dispatch_merge` / `dispatch_extract` 自动回退 `QpdfBackend`（也可通过环境变量 `DOCSY_PDF_BACKEND=qpdf` 强制使用 `qpdf`）。
   - **页数与页面几何信息读取**：[`page_info.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/page_info.rs) 与 [`qpdf.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/qpdf.rs) 的 `get_page_infos`、`page_count` 改为 `pdfcraft-cos` $\rightarrow$ `lopdf` $\rightarrow$ `qpdf` 顺序；全路径统一乘上继承的 `/UserUnit`（PDF 1.6+，默认 `1.0`），与前端 PDF.js `getViewport({ scale: 1 })` 几何尺寸严格一致。
   - **直接移除 PDF 编辑/权限密码**：[`qpdf::is_encrypted`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/qpdf.rs) 与 [`qpdf::unlock`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/qpdf.rs) 优先通过 `pdfcraft-cos` 原生识别并解开空用户密码的 RC4 / AES-128 / AES-256 权限加密，调用 `doc.remove_encryption()` 剥离 `/Encrypt` 字典后无损重写；遇到需用户打开密码或未知加密算法时回退 `qpdf` 或明确报错。
   - **数字签名扫描与书签增删查**：[`signatures::inspect_signatures`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/signatures.rs) 与 [`bookmarks.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/bookmarks.rs)（`has_pdf_bookmarks`、`apply_bookmarks`、`remove_pdf_bookmarks`）均切换为 `pdfcraft` 原生优先、失败回退 `qpdf`，在 `PATH=/nonexistent` 无外部工具环境下可直接运行。
   - **原生打印压平（电子签章/表单/注释烘焙）**：新增 [`src-tauri/src/pdf/backend/flatten.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/flatten.rs)，在 `MergePolicy::Print` 下将电子印章、表单 `/Widget`、带 `/AS` 状态的复选框、旋转 `/Matrix` 外观、`/Watermark` 及 `NoView` (`F=36`) 注释的 `/AP /N` 外观流烘焙进页面 `/Contents` 并清理 `/Annots` 与 `/AcroForm /Fields`；对 `Hidden` (`F=6`) / `Invisible` (`F=5`) 及零尺寸 `/BBox` 不绘制外观并安全移除注释壳；对 `NeedAppearances=true`（过期外观）或缺少 `/AP /N` 的可见表单 Widget 主动返回 `BackendError::Unsupported`，由 `dispatch_merge` 受控回退至 `qpdf`。

---

## 二、Review 问题与代码研究缺口逐项对账

| 类别与问题 | 根因与风险 | 修复措施与落地代码位置 | 验证结果 |
|---|---|---|---|
| **[压平] 原生打印压平与电子签章保留** | 上游 `pdfcraft-edit` 跳过 `NoView`、未覆盖 `/Watermark`、对 `Hidden`/`Invisible`/`zero_bbox` 残留注释壳，且对 `NeedAppearances=true` 会烘焙过期外观。 | 新建 [`src/pdf/backend/flatten.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/flatten.rs) 并接入 [`PdfCraftBackend::merge`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/pdfcraft_backend.rs)：支持 `NoView` 与 `/Watermark` 压平，清理 `Hidden`/`Invisible`/`zero_bbox` 注释与字段，对 `NeedAppearances=true` 或缺失 `/AP /N` 的可见 Widget 返回 `Unsupported` 触发回退。 | `test_native_print_flatten_cases_and_merge` 与 `review_print_policy_flattens_form_appearance` **全部通过** |
| **[资源清理] 无 `Do` 算子页面残留未用 XObject** | `pdfcraft-organize/src/prune.rs` 仅在遇到 `Do` 算子时才向 `self.used` 注册 `(page, b"XObject")`，导致页面无 `Do` 时未用 Form/Image XObject 不被清理。 | 在 [`vendor/pdfcraft/crates/organize/src/prune.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/vendor/pdfcraft/crates/organize/src/prune.rs) 扫描前预初始化 `XObject`/`Font`/`Shading` 空集合，同时递归保护 `/Pattern`、`/ExtGState`、`/Shading` 与 `/Annots`。 | `test_prune_unused_xobject_when_page_has_no_do_operator` **通过** |
| **[几何] `UserUnit` 预览尺寸不一致** | 后端 `page_info.rs` 未乘 `/UserUnit`，与前端 PDF.js 尺寸不一致。 | 在 [`page_info.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/page_info.rs) 三条解析路径与 `pdfcraft-organize` `INHERITABLE` 中补齐 `UserUnit` 缩放。 | `test_user_unit_page_info_consistency` (`680x720`) **通过** |
| **[解析] `stream` 后水平空白报错** | `pdfcraft-cos` 仅允许 `stream\r\n` 或 `stream\n`，遇到 `stream \n` 报错。 | 在 [`vendor/pdfcraft/crates/cos/src/parser.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/vendor/pdfcraft/crates/cos/src/parser.rs) 放宽 `stream` 后水平空格/制表符。 | 回归测试 **通过** |
| **[标记] `/DocsyWrapper` 误删普通正文** | `is_wrapper_stream` 仅凭 `/DocsyWrapper` 键名判断，未校验所有权真值与内容。 | 在 [`src/pdf/backend/marks.rs`](file:///Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/marks.rs) 要求 `is_docsy_stream == true`、角色为 `/Push` 或 `/Pop` 且流内容精确等于 `q\n` / `\nQ\n`。 | `test_body_stream_with_docsy_wrapper_key_is_preserved` **通过** |
| **[阶段 0–4] 历史 P1/P2 验收契约** | 防删除、防覆盖、防复制恢复链、继承几何、共享大图防过度降采样、提取 PoC 真实坐标与报错。 | 全部保持绿灯，无任何回归。 | 阶段 0–2 探针 **10/10 通过**，mimo 阶段 3–4 探针 **22/22 通过** |

---

## 三、测试套件与自动化验证数据汇总

| 测试项 / 套件 | 执行命令 | 结果数据 | 状态 |
|---|---|---|---|
| **版本一致性检查** | `node scripts/check-release-version.mjs` | **Release versions agree: 1.1.1** | **PASSED** |
| **Docsy Rust 单元测试** | `cargo test --lib` | **435 passed, 0 failed, 15 ignored** | **PASSED** |
| **无外置工具 (`PATH=/nonexistent`) 验证** | `PATH=/nonexistent ~/.cargo/bin/cargo test --lib ...` | **书签、签名、页信息、原生压平、资源清理全部通过** | **PASSED** |
| **mimo 阶段 3–4 验收套件** | `cargo test --manifest-path .../docsy-phase34-review.../Cargo.toml` | **22 passed, 0 failed, 2 ignored** | **PASSED** |
| **Reviewer 阶段 0–2 复现套件** | `cargo test --lib review_probe` | **10 passed, 0 failed** | **PASSED** |
| **前端自动化套件** | `npm run test` | **39 个文件，330 项测试全部通过** | **PASSED** |
| **前端代码规范** | `npm run lint` | **0 errors, 0 warnings** | **PASSED** |
| **差异格式检查** | `git diff --check` | **0 格式错误** | **PASSED** |
