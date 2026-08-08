# pdf-tools 模块审阅（非页眉页脚部分）

> 审阅日期：2026-08-08
> 审阅范围：pdf-tools 模块中排除页眉页脚子模块（已在 MDG-016 审阅）的其他子功能
> 前端 10718 行 + 后端 11013 行（排除 header_footer.rs/artifacts.rs/content_text.rs/detection.rs）

---

## 1. 模块概述

pdf-tools 是 Docsy 最大的功能模块，除了页眉页脚（MDG-016 已审）外，还包含：PDF 解锁/合并/拆分/压缩/提取页面/防复制检测、证据 PDF 分组/合并/会话管理、A4 规范化、预览渲染、批注处理。

### 前端结构
```
src/modules/pdf-tools/
├── views/
│   ├── PdfToolsView.vue           (1154 行 — 6 Tab 工具集)
│   └── EvidencePdfWorkbench.vue   (3635 行 ⚠️ 证据工作台)
├── composables/（排除已审的 useEvidencePdf*）
│   ├── pdfPageNumberRules.js      (141 行 — 页码样式)
│   ├── splitFileName.js           (131 行 — token 展开)
│   ├── pdfPreviewCoordinates.js   (mm/pt/percent 换算)
│   ├── existingPdfElements.js     (已存在元素归并)
│   └── usePdfSplitRanges.js       (页段校验)
├── components/
│   ├── HeaderFooterRuleFields.vue (877 行 — 已审)
│   ├── TextPlacementFields.vue    (64 行)
│   ├── PageNumberRuleDialog.vue   (111 行)
│   ├── ExistingPdfElementsDialog.vue (351 行)
│   └── PdfJsPreview.vue           (289 行 — pdfjs + pdftoppm 双通道)
└── index.js
```

### 后端结构（排除已审 4 文件）
```
src-tauri/src/pdf/
├── evidence.rs          (1073 行 — 证据分组/合并)
├── evidence_session.rs  (406 行 — 会话编排)
├── anti_ocr.rs          (452 行 — 防复制检测/去除)
├── qpdf.rs              (390 行 — qpdf 封装)
├── split.rs             (305 行 — PDF 拆分)
├── normalize.rs         (285 行 — A4 规范化)
├── page_info.rs         (225 行 — 页面信息)
├── preview.rs           (143 行 — 预览渲染)
├── annotations.rs       (199 行 — 批注处理)
├── overlay.rs           (8 行 — 兼容门面)
└── mod.rs               (公共函数)
```

---

## 2. 核心子功能分析

### 2.1 证据 PDF 处理（evidence.rs + evidence_session.rs）

**完整流程**：
```
scan_folder (evidence.rs:187)
  → 遍历子目录（跳过 _ / . 前缀和符号链接）
  → collect_supported_files 递归
  → 自然排序 → 按子目录分组
build_group_pdfs (evidence.rs:267)
  → 每组：PDF 直收，Word 经 Word/WPS/LibreOffice 串行转换
  → merge_pdfs_with_qpdf 输出 {group}-{hash}.pdf
merge_all (evidence.rs:361)
  → 可选 apply_identity_rename + apply_overlay_batch
  → qpdf 合并
apply_rules (evidence_session.rs:9)
  → annotations::delete_annotations_to_temp
  → header_footer::batch_overlay
  → apply_merge_if_requested（页码偏移累计书签）
```

**问题**：
- `build_group_pdfs` 串行处理，Word 转换 60s/个交互超时，大目录耗时长
- `run_process_with_interactive_timeout`（evidence.rs:21）用 `sleep(1s)` 轮询 `try_wait`，浪费 CPU
- `apply_overlay_batch`（evidence.rs:778）每文件单独 spawn qpdf，N 文件 = 2N 次进程启动
- 批注删除产生临时文件再传给 batch_overlay，多一次磁盘往返

### 2.2 防复制检测与去除（anti_ocr.rs）

**原理**：PDF 文字选择依赖字体的 ToUnicode CMap 反查字符。破坏 CMap 让复制得到乱码，视觉不变。

| 操作 | 函数 | 逻辑 |
|------|------|------|
| 检测 | `detect_anti_copy` (:41) | 遍历字体，查 MARKER 或 PUA(E000-F8FF) 映射占比 >1/3 |
| 添加 | `apply_anti_copy` (:82) | 先 `build_backup` 序列化原 CMap 到 Info 字典，再 `CmapScramble` 用 hash 替换为 PUA |
| 移除 | `remove_anti_copy` (:150) | 从 Info 取备份还原，无备份则删已损坏 CMap |

**问题**：
- `AntiCopyMethod::TextOverlay`（:125）分支与 `CmapScramble` 完全相同，注释称 overlay 在 JS/Python 层做但实际未实现 — **死代码**
- `get_font_name`（:306）、`AntiCopyMethod::label`（:22）标 `#[allow(dead_code)]`
- 前端 `PdfToolsView.vue:282` 防复制下拉只暴露 2 种方法，TextOverlay 永不可选
- 三入口每次全量 `Document::load`/`save`，大 PDF 内存峰值高
- `build_backup` 把所有 CMap 以字符串塞进 Info，超大 PDF 易膨胀

### 2.3 qpdf 封装（qpdf.rs）

封装了 inspect/unlock/merge/optimize/compress/extract_pages/split/page_count。

**亮点**：
- `run_cancellable`（:8）优先走 `SubprocessRegistry.spawn_and_wait`（可取消）
- `status_is_success` 接受 exit 0/3（qpdf warning）
- `page_count`（:292）失败回退 `--json` 解析，再回退 `page_info::get_page_infos`

**问题**：仍使用旧版 SubprocessRegistry 而非 OperationManager。

### 2.4 PDF 拆分（split.rs）

按页段 `--pages n-m` 拆分，可选页眉页脚清理，含 gap/overlap 校验。

### 2.5 A4 规范化（normalize.rs）

组合 fit/forced-orientation/unrotate 三个矩阵，写入 `cm` 操作。

### 2.6 预览渲染（preview.rs）

`pdftoppm` 渲染单页 PNG → base64；失败时 qpdf 修复后重试。每页落 PNG → `image::open` 再 base64，**未走流式；多页预览无缓存**。

---

## 3. 问题清单

### P1 — 重复代码

**路径工具函数重复定义**：
| 函数 | 位置 1 | 位置 2 | 位置 3 |
|------|--------|--------|--------|
| `safe_file_stem` | evidence.rs:1029 | split.rs:206 | — |
| `unique_output_path` | qpdf.rs:318 | split.rs:222 | header_footer.rs:738 |
| `unique_temp_pdf` | evidence.rs:1051 | mod.rs::temp_named_path | — |

**建议**：统一到 `mod.rs`。

### P1 — 旧路径残留

`evidence.rs` 的 `OverlayConfig`/`apply_overlay_batch`/`apply_overlay_single`/`ensure_legacy_overlay_text_supported`（:997 拒绝非 ASCII）是被新 `header_footer` 取代的旧实现，仍被 `merge_all` 调用。

**建议**：迁移到新路径或删除。

### P1 — 前端臃肿

`EvidencePdfWorkbench.vue` 3635 行、`PdfToolsView.vue` 1154 行单文件过大，状态散落。

### P2 — 并发不一致

`inspectUnlockFiles`（PdfToolsView.vue:368）用 4 worker 并发，`inspectAntiOcrFiles`（:502）却 `for-of await` 串行。

**建议**：anti-ocr 检测改并发并加进度事件。

### P2 — 超时不一致

前端 `Promise.race` + `setTimeout` 10s/30s（:379/:557）与后端 `run_process_with_interactive_timeout` 60s（evidence.rs:21）语义不一致。

### P2 — 死代码

`AntiCopyMethod::TextOverlay` 分支未实现但保留；前端永不暴露。**建议移除或补全**。

### P3 — 性能

- `apply_overlay_batch` 每文件单独 spawn qpdf，应批量处理
- `preview.rs` 多页预览无缓存
- `evidence_session::apply_rules` 批注删除产生临时文件多一次磁盘往返

---

## 4. UI/UX 问题

| 问题 | 位置 | 建议 |
|------|------|------|
| 文件拖放 + 预览逻辑各自实现未复用 | PdfToolsView / EvidencePdfWorkbench | 抽取共享 composable |
| 状态展示不统一：unlock/anti-ocr 用标签，extract/compress 仅 ElMessage | PdfToolsView.vue | 统一队列化反馈 |
| 拆分 tab 仅单文件预览，多文件需切模块 | PdfToolsView.vue | 支持批量拆分或明确引导 |
| PdfJsPreview 在两视图分别 import | PdfToolsView:313 / workbench | 提升到 shared |
| inspectAntiOcrFiles 串行，30s 超时易误判 | PdfToolsView.vue:502 | 改并发 + 进度事件 |

---

## 5. 改进建议

| 优先级 | 问题 | 建议 | 工作量 |
|--------|------|------|--------|
| **P1** | 路径函数重复 | 统一到 mod.rs | 0.5 天 |
| **P1** | 旧 overlay 路径残留 | 迁移或删除 | 1 天 |
| **P1** | EvidencePdfWorkbench 超载 | 拆分 | 2-3 天 |
| **P2** | anti-ocr 检测串行 | 改并发 + 进度 | 1 天 |
| **P2** | TextOverlay 死代码 | 移除或补全 | 0.3 天 |
| **P2** | 超时不一致 | 统一前后端超时语义 | 0.5 天 |
| **P3** | overlay 批量 spawn | 批量 qpdf 调用 | 1 天 |
| **P3** | 预览无缓存 | 加 LRU 缓存 | 0.5 天 |
