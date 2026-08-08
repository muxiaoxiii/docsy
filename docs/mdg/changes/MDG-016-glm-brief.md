# MDG-016: 页眉页脚模块深度审计请求

## 你的任务

你是一个独立审计者。请你：

1. **先完整阅读所有代码文件**（见下方清单），建立对这个模块的全面理解
2. **然后基于你的理解**，回答下方的问题清单
3. **不要受我们已有分析的影响**，你可以得出完全不同的结论
4. **如果你发现问题，给出具体的修复建议**（最好附代码）

---

## 一、项目背景

Tauri 桌面应用（Docsy），用于中国律师处理证据 PDF。核心功能：
- 给 PDF 添加页眉、页脚、页码
- 检测并删除/替换现有的页眉页脚页码
- 支持多文件批量处理（证据列表）

技术栈：Vue 3 + Element Plus 前端，Rust (Tauri 2) 后端，lopdf 库处理 PDF，pdftotext (poppler) 做文本提取。

用户操作流程：
1. 加载多个证据 PDF 文件
2. 系统自动检测每个文件的现有页眉页脚
3. 用户确认：对检测到的页眉页脚，选择删除、编辑或保留
4. 用户设置新的页眉页脚页码（来源、格式、字体、位置等）
5. 点击生成 → 后端处理每个文件 → 输出最终 PDF

---

## 二、用户报告的问题

用户在实际使用中遇到了以下问题：

### 问题 A：删除失败
用户标记删除现有页眉页脚后点击生成，系统提示「已请求删除现有页眉页脚，但没有找到可安全删除的匹配内容；原文未被遮盖或改写」。最终输出的 PDF 中，旧页眉页脚仍在，新页眉页脚叠加在上面。

用户确认：检测阶段确实检测到了文本，也正确标记了删除，但删除没有生效。

### 问题 B：文件列表显示不一致
页眉来源选「固定文本」，输入 `证据[#]`，最终渲染出来的是「证据1」（正确），但文件列表中显示的是文件名（错误）。

### 问题 C：按证据列表名称模式用文件名
选「按证据列表名称」模式，期望生成「证据1」「证据2」，但实际显示文件名。

### 问题 D：序列从 1 开始
序号始终从 1 开始，无法设置起点。

### 问题 E：其他
- 合并完成后没有明显提示
- 大文件检测很慢
- UI 有点乱

---

## 三、代码文件清单

请按以下顺序阅读所有文件。每个文件都标注了行数和职责。

### 后端（Rust）— 按依赖顺序阅读

1. **`/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/content_text.rs`**（991 行）
   - 普通文本检测和删除
   - 包含 `filter_page_operations`、`filter_referenced_form_text`、`matches_any_target`、`target_bbox_matches`、`is_in_header_zone`
   - **这是问题 A 最可能的代码位置**

2. **`/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/artifacts.rs`**（1390 行）
   - PDF Artifact 标记的检测和删除
   - 包含 `inspect_meaningful_header_footer_artifacts`、`edit_header_footer_artifacts_file`、`edit_referenced_form_artifacts`
   - 与 content_text.rs 是平行的两条处理路径

3. **`/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/header_footer.rs`**（2369 行）
   - 主入口 `overlay_text` → `process_job`
   - 协调 artifact 路径和 plain text 路径
   - 包含警告生成逻辑

4. **`/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/detection.rs`**（2645 行）
   - 检测逻辑：pdftotext 调用 + Artifact 检测
   - 检测结果的解析和结构化

### 前端（JavaScript/Vue）— 按数据流顺序阅读

5. **`/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/useEvidencePdfDetection.js`**（502 行）
   - 检测流程控制
   - 检测结果存储到 `file.existingElements`

6. **`/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/useEvidencePdfExistingEditing.js`**（273 行）
   - 用户对检测结果的编辑（删除/编辑/保留）
   - 包含 `startHeaderEdit`、`rowHeaderPreview`

7. **`/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/useEvidencePdfSession.js`**（1042 行）
   - 核心业务逻辑
   - 包含 `buildHeaderFooterItems`（构建后端参数）、`buildHeaderTextForGroup`（渲染文本）、`buildPlainTextTargets`（构建删除目标）

8. **`/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/splitFileName.js`**（131 行）
   - 占位符展开（`[#]`、`[序号]`、`[文件名]` 等）

9. **`/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/pdfPageNumberRules.js`**（141 行）
   - 页码格式定义

10. **`/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/components/HeaderFooterRuleFields.vue`**（877 行）
    - 页眉页脚配置 UI

---

## 四、问题清单

请基于你对代码的理解，逐一回答以下问题。

### 关于问题 A（删除失败）

1. **完整的数据流是什么？** 从前端检测到用户标记删除，到后端执行删除，请追踪完整的数据流，标注每一步的代码位置。

2. **删除有哪些可能失败的路径？** 请逐一列出，标注代码位置和失败条件。

3. **我们之前猜测了三个可能的原因**：
   - Zone check 作为 gate 阻止了 bbox 匹配
   - CID 字体编码导致文本匹配失败
   - Form XObject 坐标空间不匹配
   你的看法是什么？这些是真正的原因吗？还有没有其他原因？

4. **修复方案**：我们提出了三个方案：
   - 方案 1：绕过 zone check，让 bbox 匹配独立于 zone 检查
   - 方案 2：在 Form XObject 处理时做坐标变换（读取表单的 BBox 和 Matrix，把本地坐标转为页面坐标）
   - 方案 3：白底矩形覆盖（用户不接受）
   你的评价是什么？有没有更好的方案？

5. **如果让你修复这个问题，你会怎么改？** 请给出具体的代码修改建议（哪个文件、哪个函数、怎么改）。

### 关于问题 B 和 C（数据不一致）

6. **问题 B 的根因是什么？** 请追踪「固定文本」模式下，渲染路径和文件列表显示路径的代码差异。

7. **问题 C 的根因是什么？** 请追踪「按证据列表名称」模式下，`file.header` 的初始化时机和使用方式。

8. **修复建议**：你认为应该怎么修？

### 关于问题 D（序列起点）

9. **当前的序列生成逻辑是什么？** 请追踪 `[#]`、`[序号]` 等占位符的展开逻辑。

10. **如何支持自定义起点和步长？** 请给出修改建议。

### 模块整体评估

11. **代码质量**：这个模块的代码质量如何？有没有架构上的问题？

12. **其他潜在问题**：除了上述 4 个 bug，你在阅读代码时还发现了什么问题？

13. **UI 问题**：阅读 `HeaderFooterRuleFields.vue`，你觉得当前 UI 有什么问题？如何改进？

14. **对 8 个需求的评估**：
    - 检测与删除统一（保证检测到就能删除）
    - 按证据列表名称 UI 重构（前缀/序号格式/起始/步长的专用设置区）
    - 占位符增强（`[#, 起点, 步长]`）
    - 序列格式选项（中文数字、大写中文、罗马数字等）
    - 奇偶页支持
    - 分段→显示范围
    - 大文件检测优化
    - 合并完成醒目提示
    每个需求的技术可行性和实现难度？

---

## 五、输出格式

请按以下结构输出你的报告：

```
## 1. 模块概述
（你对这个模块的整体理解）

## 2. 问题 A 分析与修复
（数据流追踪、失败路径、修复方案）

## 3. 问题 B/C 分析与修复

## 4. 问题 D 分析与修复

## 5. 其他发现
（你在代码中发现的其他问题）

## 6. UI 改进建议

## 7. 需求评估

## 8. 总结与建议
（优先级排序的修复建议）
```
