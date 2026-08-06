实现证据处理模块的检测改进方案。

## 改动 1：Rust 端 — 单页文件过滤 content-text 非页码候选

文件：src-tauri/src/pdf/detection.rs

在 `build_candidates` 函数中，生成候选后增加过滤：
- 当 `pages_analyzed <= 1`（单页文件）且候选来源是 `content-text`（非 artifact）
  且候选不含 `page-number` label 时，过滤掉该候选
- 页码候选保留（因为可能跨文件编写）
- Artifact 来源的候选保留（标准 PDF 结构，可信度高）

## 改动 2：前端 — 低置信度候选处理

文件：src/modules/pdf-tools/composables/existingPdfElements.js

在 `detectedElementFromCandidate` 中：
- 传递 `confidence` 字段（已有）
- 新增 `lowConfidence` 标记：当 `confidence < 0.3` 或（单页文件 + content-text 来源）时为 true

文件：src/modules/pdf-tools/views/EvidencePdfWorkbench.vue

在 `existingElementRows` computed 中：
- 低置信度行的 `element.decision` 默认不设为自动确认，保持 null（待用户决定）

文件：src/modules/pdf-tools/components/ExistingPdfElementsDialog.vue

- 低置信度行排序到列表末尾（排序优先级：已决策 > 正常 > 低置信度）
- 低置信度行用灰色样式（opacity: 0.6 或灰色背景）
- 低置信度行默认不勾选

## 改动 3：前端 — 跨文件页码序列验证

文件：src/modules/pdf-tools/views/EvidencePdfWorkbench.vue

在 `existingElementRows` computed 中，对单页文件的页码候选做跨文件验证：
- 收集所有文件的页码候选及其 detectedText（页码值）
- 如果某文件的页码值能跟其他文件的页码形成连续序列（如文件1有"1"，文件2有"2"），保留为正常置信度
- 如果是孤立页码值（不跟任何其他文件连续），标记为低置信度

## 改动 4：检测进度动画 — 改为全局任务级

文件：src/modules/pdf-tools/views/EvidencePdfWorkbench.vue

当前：每个文件检测时弹独立的动画/进度指示
改为：
- 检测任务整体有一个进度条/动画，显示"正在检测 X/N 个文件..."
- 不要每个文件弹独立的动画
- 检测完成后统一显示结果

找到检测相关的动画/进度代码，合并为一个全局进度指示器。

## 验证
cargo test + npm test + npm run build 全过。
