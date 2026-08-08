# MDG-016 现有代码深度分析

## 一、架构概览

### 数据流
```
前端配置 → buildHeaderFooterItems() → HeaderFooterJob JSON → 后端 overlay_text() → process_job()
```

### 核心模块
| 模块 | 文件 | 职责 |
|------|------|------|
| 配置 UI | HeaderFooterRuleFields.vue | 页眉页脚配置界面 |
| 文本生成 | useEvidencePdfSession.js | 占位符替换、文本组装 |
| 占位符 | splitFileName.js | `[#]`、`[序号]`、`[日期]` 等展开 |
| 页码规则 | pdfPageNumberRules.js | 页码格式、连续/分文件、overrides |
| 检测 | useEvidencePdfDetection.js | 现有页眉页脚自动检测 |
| 后端处理 | header_footer.rs | PDF 叠加、清理、书签 |
| 工件检测 | artifacts.rs | PDF Artifact 结构检测 |
| 诊断 | detection.rs | pdftotext + bbox 分析 |

---

## 二、页眉来源模式分析

### 前端定义 (HeaderFooterRuleFields.vue:57-62)
```javascript
none     → 不插入页眉
filename → 文件名
per_file → 按证据列表名称
custom   → 固定文本
```

### 文本生成逻辑 (useEvidencePdfSession.js)

**headerBaseTextForGroup(file, index, group, rules)**:
```javascript
per_file  → file.header ?? stripPdf(file.name)  // ⚠️ 问题1
custom    → group.text || ''                      // ⚠️ 问题2
seq       → `证据${index + 1}`                    // ⚠️ 问题3
seq_cn    → `证据${toChineseNumber(index + 1)}`   // ⚠️ 问题3
prefix_seq → `${group.text || ''}证据${index + 1}` // ⚠️ 问题3
fallback  → stripPdf(file.name)
```

---

## 三、Bug 精确根因

### Bug 1：固定文本 `证据[#]` 渲染为证据列表名称

**调用链**:
1. 用户输入 `证据[#]` 作为 custom 文本
2. `headerBaseTextForGroup` 返回 `group.text` = `证据[#]`
3. `decorateHeaderTextForGroup` 调用 `expandSplitNameTokens(证据[#], index, dateValue)`
4. `expandSplitNameTokens` 中 `[#]` 被 `formatSequenceToken` 替换为 `index + 1`
5. 结果 = `证据1`

**但问题在**：文件列表中显示的页眉调用的是 `displayRowHeader` → `rowHeaderPreview` → `buildHeaderText` → `headerBaseText`
- 如果 `headerMode` 是 `custom`，返回 `rules.headerText`
- 如果 `headerMode` 是 `per_file`，返回 `file.header ?? stripPdf(file.name)`

**根因**：`headerMode` 和 group 的 `mode` 可能不一致。UI 选择的是 group mode，但 `rules.headerMode` 可能还是旧值。

### Bug 2：按证据列表名称使用文件名

**调用链**:
1. `per_file` 模式 → `file.header ?? stripPdf(file.name)`
2. `file.header` 在 `startHeaderEdit` 中设置为 `rowHeaderPreview(row, index) || stripPdf(row.name)`
3. `rowHeaderPreview` 调用 `buildHeaderText` → `headerBaseText`
4. 如果此时 `headerMode` 不是 `per_file`，`headerBaseText` 返回文件名
5. `file.header` 被设为文件名

**根因**：`file.header` 的初始化依赖当前 `headerMode`，但 `headerMode` 可能还没切换到 `per_file`。

### Bug 3：序列始终从 1 开始

**根因**：`formatSequenceToken` 硬编码 `Math.max(1, index + 1)`
```javascript
function formatSequenceToken(token, index = 0) {
  const value = String(Math.max(1, Number(index || 0) + 1))
  return value.padStart(token.length, '0')
}
```
没有起点和步长参数。

---

## 四、分段功能分析

### 前端 UI
- 两个输入框：`pageStart` 和 `pageEnd`
- 标签是「分段」
- 默认值：pageStart=1, pageEnd=0

### 数据流
- `pageStart`/`pageEnd` 是 `OverlayTextConfig` 的字段
- 后端 `expand_config_placeholders` 使用这些字段限制页码范围
- 每个 header group 有自己的 pageStart/pageEnd

### 评估
- **当前意义**：限制某个页眉组只在特定页码范围显示
- **问题**：用户理解为「分段 = 某个页码的起点和终点」，但实际上这是页眉的显示范围
- **结论**：功能本身有用，但标签「分段」容易误导。应改为「显示范围」或「页码范围」。

---

## 五、检测性能分析

### 检测流程
1. 前端 `detectAllHeaderFooter` 遍历所有文件
2. 每个文件调用 `detectFileHeaderFooter` → `detect_pdf_header_footer`
3. 后端 `detect` 函数：
   - `inspect_meaningful_header_footer_artifacts`：遍历 max_pages 页，解码每页内容流
   - `run_pdftotext_bbox`：调用 pdftotext 提取文本
   - 解析 XML、分析候选、评分排序

### 性能瓶颈
- `DETECTION_SCAN_MAX_PAGES = 20`：前端限制每个文件最多检测 20 页
- 但 `inspect_meaningful_header_footer_artifacts` 需要 `Document::load` 加载整个 PDF
- 对于 300+ 页的 PDF，即使只检测 20 页，加载 PDF 本身就很慢
- `pdftotext -bbox` 对大文件也较慢

### 优化方案
- 超过阈值页数时，提示用户是否跳过检测
- 检测和处理分离：检测用轻量级方式（只读前几页），处理时再加载完整 PDF

---

## 六、奇偶页现状

- **完全没有奇偶页支持**
- 后端 `OverlayTextConfig` 没有奇偶页字段
- 前端没有奇偶页配置 UI
- `pageNumberOverlaysForFile` 中的 `overrides` 可以按页码范围设置不同规则，但不是奇偶页

---

## 七、与模板模块的差距

| 功能 | 模板模块 | 页眉页脚模块 |
|------|----------|--------------|
| 序号格式 | 序号/序号01/序号001/中文序号 | 只有 `[#]`/`[##]`/`[###]` |
| 序号起点 | 不支持 | 不支持 |
| 序号步长 | 不支持 | 不支持 |
| 占位符 | 字段/预设/文本 | 只有简单文本替换 |
| 文件名模板 | FilenameTokenInput 组件 | 无 |
| 预览 | DocumentPreview 实时预览 | 后端 preview_overlay 但前端不完整 |
