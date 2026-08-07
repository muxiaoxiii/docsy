# Docsy 设计文档 vs 实际实现 对照分析

> 生成时间：2026-08-07
> 代码版本：v0.9.6（package.json / Cargo.toml）
> 对照范围：9 份设计文档 + README + CHANGELOG vs src/ (~20,400 行) + src-tauri/src/ (~20,600 行)

---

## 总览

| 类别 | 数量 |
| --- | --- |
| 设计文档描述但代码未实现的功能 (A) | 12 |
| 代码实现但设计文档未提及的功能 (B) | 8 |
| 架构差异 (C) | 7 |
| 设计文档过时/不准确的描述 (D) | 14 |
| 实现决策变更但文档未更新 (E) | 6 |

---

## 1. architecture.md（架构总览）

### D-1: 版本号过时
- **文档声称**：`更新时间：2026-08-04（v0.9.2）`
- **实际版本**：v0.9.6
- **影响**：文档落后 4 个小版本

### D-2: 命令数量不准确
- **文档声称**：`共 58 个命令`；pdf 域 20 个、template 域 16 个
- **实际**：共约 75 个命令；pdf 域 26 个、template 域 25 个
- **原因**：v0.9.3–0.9.6 新增了 anti_copy（3 个）、bookmarks（2 个）、更多 template 命令等

### B-1: anti_ocr / anti_copy 模块未在架构文档中提及
- **代码**：`src-tauri/src/pdf/anti_ocr.rs`（452 行），提供 `detect_anti_copy` / `apply_anti_copy` / `remove_anti_copy` 三个命令，前端 `PdfToolsView.vue` 有「防复制」页签
- **架构文档**：未提及该模块

### B-2: bookmarks 功能未在架构文档中提及
- **代码**：`has_pdf_bookmarks` / `remove_pdf_bookmarks` 命令，EvidencePdfWorkbench 中使用
- **架构文档**：未提及

### C-1: 模块列表与实际目录不完全匹配
- **文档列出**：6 个模块（home / evidence-pdf / pdf-tools / image-paddler / video-extract / template / settings）
- **实际目录**：7 个模块（与文档一致），但 architecture.md 未提及 `evidence-pdf` 模块有独立的 `EvidencePdfView.vue`（分项证据处理 / 合并证据处理 / 证据扫描三页签）

### D-3: 测试数量过时
- **文档声称**：`70 tests, 13 files`（前端）/ `141 tests`（Rust）
- **实际**：需要重新验证（0.9.3–0.9.6 可能有变化）

---

## 2. template-system-design.md（模板系统设计）

### D-4: 字段类型列表不完整
- **文档列出**（当前实现范围段落）：`text / date / select / party_list / reference / checkbox / radio_group / checkbox_group`（8 种）
- **实际**：实现也支持这 8 种，与文档一致
- **但**：文档正文中的字段模型示例和详细描述缺少 `reference` 类型的完整说明（仅在"当前实现范围"中列出名称），而代码中 `reference` 是重要类型

### A-1: 公共规则库尚未完整实现
- **文档描述**：公共规则库包含"法院名称模式、常见法院名种子、案由体系、案号格式、日期格式等公开规则"
- **实际**：`src/modules/template/rules/` 下有 `publicRules.js`、`courtNames.js`、`causeActions2025.js`，已实现法院名称和案由识别，但**案号格式、日期格式的自动推断**在前端代码中仅部分实现

### A-2: 多模板材料包未实现
- **文档描述**（"多模板材料包"章节）：一次生成所函、送达地址确认书、授权委托书等多个模板
- **实际**：无材料包相关的命令、组件或 UI

### A-3: 下划线占位规则未完全实现
- **文档描述**：`下划线占位不作为通用规则`，但模板制作时应检测下划线辅助标记
- **实际**：模板系统仅检测黄色高亮，下划线不作为字段识别信号

### A-4: 拆分操作（一个 mark 内按字符范围拆成多行）UI 未完全实现
- **文档描述**：确认界面支持"拆分"操作，一个黄色 mark 内可按字符范围拆成多行
- **实际**：后端 `save.rs` 支持按 `start/end` 偏移切分 run，但前端确认界面的拆分交互能力有限

### E-1: 模板编辑器从"所见即所得"简化为"标黄扫描 + 确认"
- **旧设计**（Docsy软件设计文档 §5.2.2）：模板编辑器应支持固有字编辑、基础格式工具栏、取消标记后恢复编辑等
- **实际**：模板制作采用"Word 中标黄 → 导入 Docsy → 确认字段"流程，不在 Docsy 内编辑 Word 原文
- **文档已更新**：template-system-design.md 已反映此变更，但 Archived/Docsy软件设计文档.md 仍是旧设计

### C-2: .docsytpl 格式简化
- **旧设计**（Docsy软件设计文档 §13.2）：包含 `manifest.json + template.docx + fields.json + dictionaries.json + preview.png`
- **实际**：仅包含 `manifest.json + template.docx`（字段定义在 manifest.json 中，无独立 fields.json）
- **template-system-design.md 已更新**：正确描述为 `manifest.json + template.docx`

---

## 3. pdf-evidence-processing-design.md（PDF 证据处理设计）

### D-5: "区域覆盖兜底"描述与实际不符
- **文档声称**（L533/L553）："若没有可删的标准标记，仍继续使用区域覆盖兜底"
- **实际**：代码中当语义删除计数为 0 时，仅 push 警告消息，**不做白底覆盖**
- **状态**：CHANGELOG 0.9.2 称已修正文档，但部分措辞可能仍有残留

### D-6: A4 规范化方案描述方向相反
- **文档声称**（L269-274/L490）：当前是"重渲染（pdftoppm 重建）"，"保留文本方案未实现"
- **实际**：实现的是**内容流矩阵缩放**（保留文本可选中性），比文档所述方案更优
- **代码**：`normalize.rs` 使用 `q…Q` 包裹 + 重设 MediaBox/CropBox

### D-7: "尚未实现"清单已过时
- **文档 L489** 声称"尚未实现对象层删除和内容流安全编辑"
- **实际**：`artifacts.rs`（标准 Artifact 删除）、`content_text.rs`（内容流文本提取）均已实现
- **文档 L567-578** 的"尚未完成收敛"清单中多项已实现（如 Artifact 删除、确认窗口、页码分类）

### A-5: 后端模块结构与设计不完全一致
- **设计文档建议**：`evidence_session.rs / page_info.rs / preview.rs / header_footer.rs / annotations.rs / normalize.rs / split.rs / merge.rs / qpdf.rs / evidence.rs`
- **实际**：缺少独立的 `merge.rs`（合并在 `header_footer.rs` 和 `evidence.rs` 中），额外存在 `anti_ocr.rs / artifacts.rs / content_text.rs / detection.rs / overlay.rs`

### A-6: 设计的 Tauri 命令未完全实现
- **设计文档建议**的业务命令：`inspect_evidence_pdfs / render_pdf_page_preview / detect_pdf_header_footer / apply_evidence_pdf_rules / merge_evidence_pdfs / inspect_merged_evidence_pdf / split_merged_evidence_pdf / delete_pdf_annotations / normalize_pdf_pages`
- **实际**：大部分已实现，但 `normalize_pdf_pages` 作为独立命令不存在（A4 规范化在 `apply_evidence_pdf_rules` 内部执行）

### E-2: 预览实现策略从 PDF.js 优先改为后端渲染优先
- **设计文档**：PDF.js 负责前端真实页面预览和交互坐标
- **实际**：`PdfJsPreview.vue` 默认使用后端 `pdftoppm` 单页渲染，PDF.js 为备用引擎（CHANGELOG 0.7-2026 记录此变更）
- **文档已部分更新**：pdf-evidence-processing-design.md L540 提及此变更

---

## 4. pdf-ocr-module-design.md（PDF OCR 模块设计）

### A-7: 整个 OCR 模块未实现
- **文档描述**：完整的 OCR 管线——PP-OCRv6 ONNX 推理、LiteParse 布局重建、三输出（可搜索 PDF / Markdown / JSON）、harumi 文本层回写
- **实际**：`src-tauri/src/ocr/` 目录不存在；`src/modules/pdf-ocr/` 目录不存在；无 `parse_pdf` / `check_ocr_models` 命令
- **评估**：该文档为 v0.9.3 的前瞻性设计，尚未进入实现阶段

### E-3: 文档中"与现有模块的关系"描述为已衔接
- **文档声称**：`detect_pdf_header_footer` 对"页面只有图片"的扫描件可标记为"建议 OCR"
- **实际**：代码中无此标记逻辑，OCR 模块完全不存在

---

## 5. pdf-workbench-0.8.6-design.md（PDF 工作台设计）

### D-8: 统一对象模型未完全采用
- **文档设计**：`ExistingPdfElement` 统一模型（kind / detectedText / editedText / decision 等）
- **实际**：`existingPdfElements.js` 中使用类似的结构，但字段命名和层级与设计不完全一致

### D-9: 页码样式列表不完整
- **文档列出**：阿拉伯数字 / 中文数字 / 大写罗马 / 小写罗马 / 带圈数字 / 实心圈数字
- **实际**：实现一致，但文档 L137 说"键帽数字不暴露"，此限制是否仍在代码中需验证

### A-8: `PageNumberOverride` 的 `exclude` / `override` 分段逻辑
- **文档设计**：支持按页码范围排除或覆盖页码样式
- **实际**：`PageNumberRuleDialog.vue` 存在，但分段/例外的完整 UI 交互需要进一步验证

### D-10: Word 探测设计已实现但文档未反映最新修复
- **文档设计**：Windows 探测顺序（注册表 → COM → 安装目录 → where）、macOS 探测顺序（Applications → Launch Services → mdfind）
- **实际**：已实现且经过多次修复（CHANGELOG 0.8.4/0.9.0），但文档未更新修复细节

---

## 6. code-review-0.9.1.md（代码审查报告）

### D-11: 高危问题 H1/H2 已修复
- **H1（取消功能失效）**：CHANGELOG 0.9.2 声称已修复——SubprocessRegistry 现在注册 qpdf 子进程 PID
- **H2（run 索引不一致）**：CHANGELOG 0.9.2 声称已修复——save.rs 的 `run_has_text()` 匹配 scan.rs 行为
- **H3（区域覆盖兜底）**：CHANGELOG 0.9.2 声称已修正文档描述

### D-12: 中等问题 M6/M7 已修复
- **M6（中文数字两套实现）**：CHANGELOG 0.9.2 声称已统一到 `core/numberFormat.js`
- **M7（页码预览样式不一致）**：CHANGELOG 0.9.2 声称已修复

---

## 7. Archived/Docsy软件设计文档.md（旧设计文档）

> 此文档为 2026-06-03 版本，已被归档。以下列出与当前实现的主要差异。

### E-4: UI 布局从"标签页"改为"侧边栏菜单"
- **旧设计**：顶部导航栏 + 左侧标签页（所函生成 / 模板制作 / 模板管理 / PDF 工具 / 记录中心 / 设置）
- **实际**：左侧侧边栏菜单 + 路由视图，无顶部导航栏，无独立"记录中心"和"模板管理"标签页

### A-9: 记录中心未实现
- **旧设计**（§12.6）：独立的记录中心页面，支持搜索/筛选、详情查看、复用上次输入、重新生成
- **实际**：无独立记录中心模块；历史记录集成在模板填写页的侧边（`TemplateHistoryTab.vue`）

### A-10: 模板管理独立页面未实现
- **旧设计**（§12.4）：独立的模板管理页，列出所有模板，支持编辑、删除、导入导出、固定到标签页
- **实际**：模板管理集成在模板模块的 `TemplateSettingsTab.vue` 中，无独立页面

### A-11: pinned_to_tab（固定到标签页）功能未实现
- **旧设计**：模板可"固定到标签页"获得独立入口
- **实际**：代码中无 `pinned_to_tab` / `pinnedToTab` 相关字段或逻辑

### A-12: 配置导入导出（.docsybundle）未实现
- **旧设计**（§11.3）：导出为 `.docsybundle` zip 包，支持模板、字段历史、当事人主档、字典的导入导出
- **实际**：CHANGELOG 0.5.3 提到过配置导入导出，但当前代码中无 `.docsybundle` 相关逻辑

### C-3: SQLite 数据库 schema 大幅简化
- **旧设计**（§11.2）：5 张表（templates / field_history / parties / generation_records / dictionaries）
- **实际**：`template_history.rs` 使用 SQLite，但表结构简化为 `generation_runs` + `field_history`，无独立的 `templates` / `parties` / `dictionaries` 表

### C-4: 模板字段模型从"丰富属性"简化为"标黄确认"
- **旧设计**（§5.6）：字段有 `key / label / type / required / multiple / min / default_role / subject_type_options / style / options / remember_history` 等丰富属性
- **实际**：字段模型更简洁——`id / name / label / semantic_key / type / required / optional_rule / mark_refs`，样式继承自 Word 原文 run 属性

### E-5: "所函生成"从独立标签页变为模板模块的一个功能
- **旧设计**：所函生成是独立的内置标签页
- **实际**：所有模板（包括所函）统一在 `template` 模块中处理

### C-5: PDF 工具从"子菜单布局"改为"页签布局"
- **旧设计**（§12.5）：左侧子菜单（解锁/合并/拆分/压缩/Word→PDF）+ 中间主操作区
- **实际**：`PdfToolsView.vue` 使用 Element Plus 的 `el-tabs`（顶部页签），包含证据处理、解锁、合并、拆分、防复制等页签

### E-6: PDF 处理从 qpdf/lopdf 混合改为 qpdf 主导 + lopdf 辅助
- **旧设计**（§2.2）：建议使用 `pdfium-render` 或 `lopdf`
- **实际**：qpdf 作为主要 PDF 处理引擎（通过子进程调用），lopdf 用于对象层操作（批注删除、Artifact 检测），无 pdfium

---

## 8. README.md

### D-13: 命令数量描述不准确
- **README 声称**：`共 6 个命令域`
- **实际**：正确（pdf / template / image_paddler / video / settings / system），但每个域的命令数未列出

### B-3: README 未提及 anti-copy（防复制）功能
- **实际**：PdfToolsView 有完整的"防复制"页签（detect / apply / remove），README 未提及

### B-4: README 未提及 bookmarks（书签）功能
- **实际**：证据处理支持检测和移除 PDF 书签，README 未提及

### B-5: README 未提及 evidence-pdf 独立模块
- **README 目录结构**：列出了 `evidence-pdf/` 但未详细说明其功能
- **实际**：`EvidencePdfView.vue` 提供分项证据处理、合并证据处理、证据扫描三个独立页签

---

## 9. CHANGELOG.md

### D-14: v0.9.0 声称"7 种字段类型"
- **CHANGELOG 0.9.0**：`7 种字段类型：text / date / select / party_list / reference / checkbox / radio_group / checkbox_group`
- **实际**：列出的是 8 种，不是 7 种
- **CHANGELOG 0.9.2 已修正**：`README：修正字段类型数量（7→8）`

### B-6: v0.9.3–0.9.6 的新增功能未反映在设计文档中
- 检测对话框排序/范围选择（0.9.6）
- 模板语法提示 tooltip（0.9.6）
- 两阶段检测（0.9.6）
- 一键清除页眉页脚（0.9.2）
- 关于页面（0.9.2）
- 页码格式预设（0.9.2）
- 防复制功能（0.9.3+，具体版本未在 CHANGELOG 中明确记录）

---

## 10. 跨文档综合差异

### C-6: 后端实际命令数远超设计文档声称

| 域 | architecture.md 声称 | 实际 | 差异 |
| --- | --- | --- | --- |
| pdf | 20 | 26 | +6（anti_copy×3, bookmarks×2, get_pdf_page_count） |
| template | 16 | 25 | +9 |
| settings | 8 | 8 | 0 |
| system | 8 | 11 | +3 |
| video | 4 | 4 | 0 |
| image_paddler | 2 | 2 | 0 |
| **合计** | **58** | **76** | **+18** |

### C-7: 前端实际模块数与设计文档对齐但细节有差异

| 模块 | 设计文档描述 | 实际 |
| --- | --- | --- |
| home | 首页 | ✅ 一致 |
| template | 文书模板 | ✅ 一致，但无独立"模板管理"/"记录中心" |
| pdf-tools | PDF 工具 | ✅ 一致，新增"防复制"页签 |
| evidence-pdf | 证据处理入口 | ✅ 一致 |
| image-paddler | 图片排版 | ✅ 一致 |
| video-extract | 视频抽帧 | ✅ 一致 |
| settings | 设置与诊断 | ✅ 一致，新增"关于"页面 |

### B-7: 代码中存在但设计文档未提及的后端文件

| 文件 | 功能 | 设计文档状态 |
| --- | --- | --- |
| `anti_ocr.rs` | 防复制保护检测/应用/移除 | 未提及 |
| `content_text.rs` | 内容流文本提取 | pdf-evidence-design 提及但未独立描述 |
| `overlay.rs` | 兼容层（旧 overlay 命令） | 提及为兼容层 |

### B-8: 代码中存在但设计文档未提及的前端功能

| 功能 | 位置 | 设计文档状态 |
| --- | --- | --- |
| 关于页面 | `AboutView.vue` | 未提及 |
| 页码格式预设（5 种快捷样式） | `PageNumberRuleDialog.vue` | 未提及 |
| 一键清除页眉页脚流水线 | `EvidencePdfWorkbench.vue` | 未提及 |
| 检测对话框排序 + Shift 范围选择 | `ExistingPdfElementsDialog.vue` | 未提及 |
| 模板语法提示 tooltip | `HeaderFooterRuleFields.vue` | 未提及 |
| 模板模块 Undo/Redo | `UndoRedoButtons.vue` | 未提及 |

---

## 建议优先更新的文档

1. **architecture.md**：更新版本号到 v0.9.6，修正命令数量，补充 anti_ocr / bookmarks 模块
2. **pdf-evidence-processing-design.md**：清理"尚未实现"清单，修正 A4 规范化描述，补充防复制/书签功能
3. **README.md**：补充防复制、书签、关于页面等功能描述
4. **pdf-ocr-module-design.md**：标注为"前瞻性设计，尚未实现"
5. **Archived/Docsy软件设计文档.md**：已归档，无需更新，但应明确标注为"已被 template-system-design.md 替代"

---

*本文档基于 2026-08-07 的代码和文档静态分析生成，未运行测试。*
