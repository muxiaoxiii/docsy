# MDG-016 研究总结 — 交给 Claude 深度代码审计

## 背景

这是一个 Tauri 桌面应用（Docsy），用于中国律师处理证据 PDF。页眉页脚模块有多个 bug 和功能缺陷。我们已经做了初步研究，但需要 Claude 完整阅读代码后给出最终结论。

---

## 已确认的 Bug

### Bug 1：文件列表与实际渲染数据不一致

**现象**：固定文本 `证据[#]` 渲染正确，但文件列表显示文件名。

**初步分析**：
- 渲染用 `buildHeaderTextForGroup(file, index, group, rules)`，读 `group.mode`
- 文件列表用 `buildHeaderText(file, index, rules)`，读 `rules.headerMode`
- 两套函数使用不同参数，导致数据不一致

**需要 Claude 确认**：
1. 完整追踪 `buildHeaderText` 和 `buildHeaderTextForGroup` 的调用链
2. 确认 `rules.headerMode` 与 `group.mode` 的同步机制
3. 找到所有数据不一致的点

### Bug 2：按证据列表名称使用文件名

**现象**：选「按证据列表名称」，本应生成「证据1」「证据2」，实际显示文件名。

**初步分析**：
- `per_file` 模式 → `headerBaseTextForGroup()` → `file.header ?? stripPdf(file.name)`
- `file.header` 在 `startHeaderEdit` 中初始化，此时 `headerMode` 可能还没切换到 `per_file`
- 所以 `file.header` 被设为文件名

**需要 Claude 确认**：
1. `file.header` 的完整生命周期（初始化、更新、使用）
2. `startHeaderEdit` 被调用的时机与 `headerMode` 切换的时序

### Bug 3：序列始终从 1 开始

**根因**：`formatSequenceToken` 硬编码 `Math.max(1, index + 1)`。

**需要 Claude 确认**：
1. 所有调用 `formatSequenceToken` 的地方
2. `expandSplitNameTokens` 的完整逻辑

### Bug 4：删除页眉页脚失败但继续处理

**这是最关键的 bug，需要 Claude 深度审计。**

**现象**：用户标记删除现有页眉页脚，系统提示「已请求删除现有页眉页脚，但没有找到可安全删除的匹配内容；原文未被遮盖或改写」，继续处理，新页眉叠加在旧页眉上。

**初步分析链路**：

```
前端检测: pdftotext -bbox → 找到文本+坐标 → 显示给用户
用户确认: 标记 decision: 'delete'
↓
后端处理:
  步骤1: edit_or_delete_standard_artifacts_if_requested()
    → 检查 header_enabled
    → header_enabled = hasArtifactDecision(file, ['header'], 'delete') || existingHeaderReplacement
    → hasArtifactDecision 检查 source === 'artifact'
    → 如果元素 source 不是 'artifact' → header_enabled = false → 跳过 Artifact 删除
  
  步骤2: delete_confirmed_plain_text_header_footer_if_requested()
    → 构建 PlainTextCleanupPlan
    → 调用 content_text::delete_plain_header_footer_to_temp()
    → 遍历页面内容流操作符
    → 对每个文本操作符，检查 is_in_header_zone && matches_any_target
    → matches_any_target 有两条路径：
        a. target_matches: 文本内容匹配
        b. target_bbox_matches: bbox 坐标匹配
    → 但如果文本在 Form XObject 中，只遍历顶层内容流找不到
  
  步骤3: semantic_removed == 0 → 生成 warning，继续处理
```

**关键发现**：

1. **Artifact 路径**（`edit_header_footer_artifacts_file`）：
   - 遍历页面内容流，找 `BDC Artifact /Subtype /Header` 标记
   - 会递归进入 Form XObject（`edit_referenced_form_artifacts`）
   - 但在 Form XObject 中也只找 Artifact 标记

2. **Plain text 路径**（`content_text.rs` `filter_page_operations`）：
   - 遍历页面内容流，找文本操作符（`Tj`/`'`/`"`）
   - **不递归进入 Form XObject**
   - 用 `is_in_header_zone` + `matches_any_target` 判断

3. **pdftotext 检测**：
   - 从渲染层面提取文本，能看到 Form XObject 中的文本
   - 返回文本内容 + bbox 坐标

**推测的失败场景**：
- 页眉文本在 Form XObject 中，没有 Artifact 标记
- pdftotext 能检测到（渲染层面）
- Artifact 路径找不到（没有 Artifact 标记）
- Plain text 路径找不到（不进入 Form XObject）
- 结果：`semantic_removed == 0`

**需要 Claude 深度审计**：
1. 完整阅读 `content_text.rs` 的 `filter_page_operations`，确认是否真的不进入 Form XObject
2. 完整阅读 `artifacts.rs` 的 `edit_referenced_form_artifacts`，确认在 Form XObject 中的搜索逻辑
3. 找出所有可能的失败路径
4. 确认是否有其他原因导致删除失败（坐标转换、zone 设置等）
5. 提出修复方案

---

## 功能需求

### 需求 1：检测与删除统一

**目标**：检测到的文本一定能删除。

**当前状态**：
- Artifact 检测和删除用同一套代码（lopdf 内容流解析），一致性有保证
- pdftotext 检测和删除用不同技术路径，一致性没有保证

**初步方案**：
- 自动检测只做 Artifact 检测（快速，一致性有保证）
- 深度检测手动触发（pdftotext，可能有不一致）
- 但用户质疑：既然有 bbox，为什么不能保证删除？

**需要 Claude 评估**：
1. 基于 bbox 的删除是否足够可靠？
2. 是否需要在 plain text 路径中增加 Form XObject 递归？
3. 是否有其他方案保证检测到就能删除？

### 需求 2：按证据列表名称 UI 重构

**方案**：
- 「按证据列表名称」模式增加专用设置区
- 前缀：下拉（证据/对比文件/附件/自定义）
- 序号格式：数字/数字01/数字001/中文数字/大写中文
- 起始编号：默认 1
- 编号步长：默认 1
- 不依赖 `file.header`，直接由设置计算

**需要 Claude 确认**：
1. `per_file` 模式的所有调用点
2. 需要修改哪些函数

### 需求 3：占位符增强

**方案**：支持 `[#, 起点]` 和 `[#，起点，步长]` 格式。

**需要 Claude 确认**：
1. `expandSplitNameTokens` 的完整逻辑
2. `formatSequenceToken` 的所有调用点

### 需求 4：序列格式选项

**方案**：与模板模块对齐，支持多种页码格式。

**需要 Claude 确认**：
1. 模块中已有的页码格式定义
2. `pdfPageNumberRules.js` 的完整逻辑

### 需求 5：奇偶页支持

**方案**：新增 `first_page_different` 和 `odd_even_different` 字段。

**需要 Claude 确认**：
1. `expand_config_placeholders` 的完整逻辑
2. 当前 OverlayTextConfig 的所有字段

### 需求 6：分段→显示范围

**方案**：标签从「分段」改为「显示范围」。

### 需求 7：大文件检测优化

**方案**：超过阈值时提示用户选择是否跳过检测。

### 需求 8：合并完成醒目提示

**方案**：用 `ElNotification` 替代 `ElMessage.success`。

---

## 代码结构概览

### 前端
- `HeaderFooterRuleFields.vue`：页眉页脚配置 UI
- `useEvidencePdfSession.js`：核心业务逻辑（文本生成、参数构建）
- `useEvidencePdfDetection.js`：现有页眉页脚检测
- `useEvidencePdfExistingEditing.js`：现有元素编辑 UI
- `splitFileName.js`：占位符展开（`[#]`、`[序号]` 等）
- `pdfPageNumberRules.js`：页码格式规则

### 后端
- `header_footer.rs`：页眉页脚处理主入口（`overlay_text` → `process_job`）
- `artifacts.rs`：PDF Artifact 检测和删除（标准页眉页脚）
- `content_text.rs`：普通文本检测和删除（非 Artifact 页眉页脚）
- `detection.rs`：页眉页脚检测（pdftotext + Artifact）
- `overlay.rs`：PDF 叠加

---

## Claude 的任务

1. **完整阅读上述所有代码文件**
2. **确认或修正我们对 4 个 Bug 的分析**
3. **深度审计 Bug 4 的失败路径**，特别是：
   - `content_text.rs` 的 `filter_page_operations` 是否支持 Form XObject
   - 如果不支持，修复方案是什么
   - 基于 bbox 的删除是否可行
4. **评估所有需求的技术可行性**
5. **给出最终结论和修复建议**
