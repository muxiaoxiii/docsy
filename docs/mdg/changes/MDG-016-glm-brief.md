# MDG-016: 页眉页脚模块深度审计请求

## 给审计者的说明

你是一个独立审计者。你的任务是：
1. 阅读下面的所有代码和分析
2. 独立验证我们的结论是否正确
3. 找出我们可能遗漏的问题
4. 提出你自己的修复方案
5. 对我们的方案给出评价

---

## 一、项目背景

这是一个 Tauri 桌面应用（Docsy），用于中国律师处理证据 PDF。核心功能是给 PDF 添加页眉页脚页码，以及删除/替换现有的页眉页脚。

### 技术栈
- 前端：Vue 3 + Element Plus
- 后端：Rust (Tauri 2)
- PDF 处理：lopdf (Rust 库)
- 外部工具：pdftotext (poppler-utils)

### 模块架构
```
用户操作流程：
1. 加载证据 PDF 文件
2. 系统自动检测现有页眉页脚（pdftotext + Artifact 检测）
3. 用户确认：删除/编辑/保留现有页眉页脚
4. 用户设置新的页眉页脚页码
5. 点击生成 → 后端处理 → 输出最终 PDF

数据流：
前端配置 → buildHeaderFooterItems() → HeaderFooterJob JSON → 后端 overlay_text() → process_job()
```

---

## 二、已确认的 4 个 Bug

### Bug 1：文件列表与实际渲染数据不一致

**现象**：固定文本 `证据[#]` 渲染正确，但文件列表显示文件名。

**原因**：两套函数使用不同数据源。

| 路径 | 函数 | 数据源 |
|------|------|--------|
| 渲染 | `buildHeaderTextForGroup(file, index, group, rules)` | `group.mode` |
| 文件列表显示 | `buildHeaderText(file, index, rules)` | `rules.headerMode` |

当 `rules.headerMode` 与 `group.mode` 不同步时，显示错误。

### Bug 2：按证据列表名称使用文件名

**现象**：选「按证据列表名称」，本应生成「证据1」「证据2」，实际显示文件名。

**原因**：`file.header` 在 `startHeaderEdit` 中初始化时，`headerMode` 可能还没切换到 `per_file`，所以被设为文件名。之后切到 `per_file` 模式时，`file.header` 已经是文件名了。

**代码链路**：
```
startHeaderEdit (useEvidencePdfExistingEditing.js:34):
  row.header = rowHeaderPreview(row, index) || stripPdf(row.name)
  ↑ 此时 headerMode 可能不是 'per_file'，rowHeaderPreview 返回文件名
  ↑ file.header 被设为文件名

headerBaseTextForGroup (useEvidencePdfSession.js:356):
  per_file → file.header ?? stripPdf(file.name)
  ↑ 读到的是文件名
```

### Bug 3：序列始终从 1 开始

**原因**：`formatSequenceToken` 硬编码 `Math.max(1, index + 1)`，没有起点参数。

### Bug 4：删除页眉页脚失败但继续处理（最关键）

**现象**：用户标记删除现有页眉页脚，系统提示「已请求删除现有页眉页脚，但没有找到可安全删除的匹配内容；原文未被遮盖或改写」，继续处理，新页眉叠加在旧页眉上。

**这是本次审计的核心问题。详见第三节。**

---

## 三、Bug 4 深度分析

### 3.1 检测阶段（前端 → 后端）

检测有两种方式：

**方式 1：pdftotext 检测**（外部工具）
- 调用 `pdftotext -bbox`，从渲染层面提取文本和坐标
- 能看到所有可见文本，包括 Form XObject 中的文本
- 返回：文本内容 + bbox（边界框坐标）

**方式 2：Artifact 检测**（lopdf 内存解析）
- 遍历 PDF 内容流，找 `BDC Artifact /Subtype /Header` 或 `/Footer` 标记
- 只能找到标准 Artifact 标记的页眉页脚
- 速度快（纯内存操作）

检测结果存储在 `file.existingElements` 中，每个元素有：
```javascript
{
  kind: 'header' | 'footerText' | 'pageNumber',
  detectedText: '页眉文本',
  normalizedText: '页眉文本',
  bbox: { x0, y0, x1, y1, page, width, height },
  source: 'artifact' | 'content-text',
  decision: 'keep' | 'delete' | 'edit'
}
```

### 3.2 删除阶段（后端）

用户标记 `decision: 'delete'` 后，前端构建参数传给后端：

```javascript
cleanup: {
  headerEnabled: hasArtifactDecision(file, ['header'], 'delete'),  // 只检查 artifact 元素
  forceDeleteHeader: hasArtifactDecision(file, ['header'], 'delete'),
  plainHeaderTargets: buildPlainTextTargets(file, 'header'),  // 非 artifact 元素
  // ...
}
```

后端处理分两步：

**步骤 1：Artifact 路径**（`edit_or_delete_standard_artifacts_if_requested`）
- 条件：`header_enabled == true`（只有 artifact 元素标记删除时才为 true）
- 操作：在 PDF 内容流中找 `BDC Artifact /Subtype /Header` 标记，删除或替换
- 会递归进入 Form XObject

**步骤 2：Plain text 路径**（`delete_confirmed_plain_text_header_footer_if_requested`）
- 条件：`plain_header_targets` 非空（非 artifact 元素标记删除时有内容）
- 操作：遍历内容流，找匹配的文本操作符删除
- **也会递归进入 Form XObject**（通过 `filter_referenced_form_text`）

### 3.3 失败路径分析

**路径 A：Zone check 作为 gate 阻止 bbox 匹配**（最可能的原因）

```rust
// content_text.rs:300-305
if is_in_header_zone(state.y, plan)        // ← 先检查 zone
    && matches_any_target(text, targets, state)  // ← zone 不通过，这里永远不会执行
```

`is_in_header_zone` 检查文本的 y 坐标是否在页眉区域内。如果不通过，`matches_any_target`（包括 bbox 匹配）**永远不会被调用**。

**路径 B：CID 字体编码导致文本匹配失败**

PDF 中的中文文本通常用 CID 字体（字形索引），`String::from_utf8_lossy` 解码出来是乱码，与 pdftotext 提取的文本不匹配。

**路径 C：Form XObject 坐标空间不匹配**（关键问题）

当 `filter_page_operations` 处理 Form XObject 的内容流时：
- `state.y` 是**表单本地坐标**
- `plan.page_box` 是**页面坐标**（从 MediaBox/CropBox 计算）
- `target_bbox_matches` 中的 bbox 是**页面坐标**（从 pdftotext 获取）

如果表单的 BBox ≠ 页面 MediaBox，坐标就不匹配。

**具体例子**：
```
页面 MediaBox: [0, 0, 595, 842]
表单 BBox: [0, 0, 595, 80]（只覆盖页眉区域）
表单 Matrix: [1, 0, 0, 1, 0, 762]（把表单放在页面顶部）

文本在表单中的位置：y = 70（表单本地坐标）
实际页面位置：y = 70 + 762 = 832（页面坐标）

pdftotext 检测到的 bbox：y0=10, y1=30（页面坐标，top-down）
转换为 PDF 坐标：pdf_y0 = 842 - 30 = 812, pdf_y1 = 842 - 10 = 832

target_bbox_matches 比较：
  state.y = 70（表单本地）
  bbox 范围 = [812, 832]（页面坐标）
  70 不在 [812, 832] 内 → 匹配失败
```

**路径 D：没有构建 plain text targets**

如果 `existingElements` 为空或 `decision` 不是 `'delete'`，`buildPlainTextTargets` 返回空数组。

### 3.4 现有测试的局限性

测试 `deletes_confirmed_plain_header_inside_form_xobject`（content_text.rs:760）使用表单 BBox `[0, 0, 595, 842]`——与页面 MediaBox 完全相同。所以表单本地坐标 = 页面坐标，测试通过。但这个测试**没有覆盖**表单 BBox ≠ 页面 MediaBox 的真实场景。

---

## 四、我们提出的修复方案

### 方案 1：绕过 zone check（当前提案）

```rust
fn matches_any_target(text: &str, targets: &[&PlainTextTarget], state: &TextState, in_zone: bool) -> bool {
    targets.iter().any(|target| {
        if target_matches(text, target) && in_zone { return true; }
        if target.bbox.is_some() && target_bbox_matches(state, target) { return true; }
        false
    })
}
```

**评价**：
- ✅ 修复了 zone gate 问题
- ❌ 不修复 Form XObject 坐标空间问题
- ❌ 对表单 BBox ≠ 页面 MediaBox 的情况仍然失败

### 方案 2：Form 坐标变换 + 绕过 zone check（推荐）

在 `filter_referenced_form_text` 中：
1. 读取表单的 BBox 和 Matrix
2. 计算坐标变换（至少 y 平移：`y_page = d * y_form + f`）
3. 调整 `plan.page_box` 或传递 y 偏移量给 `filter_page_operations`
4. 在 zone/bbox 检查前，把 `state.y` 从表单坐标转换为页面坐标

**评价**：
- ✅ 修复 Form XObject 坐标空间问题
- ✅ 结合方案 1，修复 zone gate 问题
- ⚠️ 中等复杂度

### 方案 3：白底矩形覆盖（遮盖）

在页面级别插入白色矩形覆盖旧文本。不需要进入 Form XObject，不需要坐标变换。

**评价**：
- ✅ 技术上最可靠
- ❌ 用户不接受（旧文本数据仍在 PDF 中）
- ❌ 如果页面背景不是白色，会有明显白块

---

## 五、你需要做的

1. **阅读代码文件**（见下方文件清单），独立验证我们的分析
2. **回答以下问题**：
   - Bug 4 的根因分析是否正确？有没有遗漏的失败路径？
   - 方案 2（Form 坐标变换）是否可行？有没有更简单的实现？
   - 有没有我们没想到的方案？
   - 现有代码中有没有其他潜在问题？
3. **提出你的修复方案**，给出具体代码修改建议

---

## 六、代码文件清单

请按顺序阅读：

### 后端（Rust）
1. **`src-tauri/src/pdf/content_text.rs`**（991 行）
   - 核心函数：`delete_plain_header_footer_to_temp`、`filter_page_operations`、`filter_referenced_form_text`、`matches_any_target`、`target_bbox_matches`、`is_in_header_zone`
   - 这是 Bug 4 的主要代码所在

2. **`src-tauri/src/pdf/artifacts.rs`**（1390 行）
   - 核心函数：`inspect_meaningful_header_footer_artifacts`、`edit_header_footer_artifacts_file`、`edit_referenced_form_artifacts`、`target_artifact_region`
   - Artifact 检测和删除

3. **`src-tauri/src/pdf/header_footer.rs`**（2369 行）
   - 核心函数：`overlay_text`、`process_job`、`edit_or_delete_standard_artifacts_if_requested`、`delete_confirmed_plain_text_header_footer_if_requested`、`standard_artifact_processing_warnings`
   - 主入口和流程控制

4. **`src-tauri/src/pdf/detection.rs`**（2645 行）
   - 核心函数：`detect`、`run_pdftotext_bbox`、`parse_pdftotext_bbox`
   - 检测逻辑

### 前端（JavaScript/Vue）
5. **`src/modules/pdf-tools/composables/useEvidencePdfSession.js`**（1042 行）
   - 核心函数：`buildHeaderFooterItems`、`buildHeaderTextForGroup`、`buildPlainTextTargets`、`hasArtifactDecision`、`overlayConfigForFile`
   - 参数构建和文本生成

6. **`src/modules/pdf-tools/composables/useEvidencePdfDetection.js`**（502 行）
   - 核心函数：`detectAllHeaderFooter`、`applyDetectionResultToFile`
   - 检测流程

7. **`src/modules/pdf-tools/composables/splitFileName.js`**（131 行）
   - 核心函数：`expandSplitNameTokens`、`formatSequenceToken`
   - 占位符展开

8. **`src/modules/pdf-tools/composables/pdfPageNumberRules.js`**（141 行）
   - `PAGE_NUMBER_STYLES`、`renderPageNumberTemplate`
   - 页码格式

9. **`src/modules/pdf-tools/composables/useEvidencePdfExistingEditing.js`**（273 行）
   - `startHeaderEdit`、`rowHeaderPreview`
   - 现有元素编辑

10. **`src/modules/pdf-tools/components/HeaderFooterRuleFields.vue`**（877 行）
    - 页眉页脚配置 UI

---

## 七、代码文件路径

所有文件的绝对路径：

```
/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/content_text.rs
/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/artifacts.rs
/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/header_footer.rs
/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/detection.rs
/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/useEvidencePdfSession.js
/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/useEvidencePdfDetection.js
/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/splitFileName.js
/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/pdfPageNumberRules.js
/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/composables/useEvidencePdfExistingEditing.js
/Users/only/Documents/PythonProgram/Docsy/src/modules/pdf-tools/components/HeaderFooterRuleFields.vue
```

---

## 八、之前的审计报告

Claude 的两轮审计报告：
- 第一轮：`/Users/only/Documents/PythonProgram/Docsy/docs/mdg/changes/MDG-016-audit-report.md`
- 第二轮：关于 Form XObject 坐标空间问题的分析（见上文第三节）

你可以参考这些报告，但请独立思考，不要被它们的结论影响。

---

## 九、补充：关键代码片段

### content_text.rs 中的 filter_page_operations（Bug 4 核心）

```rust
fn filter_page_operations(
    operations: &[Operation],
    plan: &PagePlainTextPlan,
) -> (Vec<Operation>, PlainTextCleanupResult) {
    let mut output = Vec::with_capacity(operations.len());
    let mut result = PlainTextCleanupResult::default();
    let mut state = TextState::default();

    for operation in operations {
        update_text_state_before_show(&mut state, operation);
        let shown_text = shown_text(operation);
        let mut remove_region = None;
        if let Some(text) = shown_text.as_deref() {
            if is_in_header_zone(state.y, plan)           // ← zone gate
                && matches_any_target(text, &plan.header_targets, &state)  // ← 包含 bbox 匹配
            {
                remove_region = Some(TextRegion::Header);
            } else if is_in_footer_zone(state.y, plan)
                && matches_any_target(text, &plan.footer_targets, &state)
            {
                remove_region = Some(TextRegion::Footer);
            }
        }
        match remove_region {
            Some(TextRegion::Header) => result.removed_header += 1,
            Some(TextRegion::Footer) => result.removed_footer += 1,
            None => output.push(operation.clone()),
        }
        update_text_state_after_show(&mut state, operation);
    }
    (output, result)
}
```

### content_text.rs 中的 filter_referenced_form_text

```rust
fn filter_referenced_form_text(
    doc: &mut Document,
    operations: &[Operation],
    xobjects: &Dictionary,
    plan: &PagePlainTextPlan,
    visited: &mut BTreeSet<ObjectId>,
) -> Result<PlainTextCleanupResult> {
    // ... 遍历 Do 操作符，找到 Form XObject
    // 解码表单内容流
    // 调用 filter_page_operations(&content.operations, plan)  ← 传入相同的 plan！
    // 没有坐标变换！
    // 递归处理嵌套的 XObject
}
```

### content_text.rs 中的 target_bbox_matches

```rust
fn target_bbox_matches(state: &TextState, target: &PlainTextTarget) -> bool {
    let Some(bbox) = target.bbox else { return false; };
    if bbox.width <= 0.0 || bbox.height <= 0.0 { return false; }
    let x_padding = 18.0;
    let y_padding = 18.0;
    let pdf_y0 = bbox.height - bbox.y1;  // pdftotext top-down → PDF bottom-up
    let pdf_y1 = bbox.height - bbox.y0;
    state.x >= bbox.x0 - x_padding
        && state.x <= bbox.x1 + x_padding
        && state.y >= pdf_y0 - y_padding
        && state.y <= pdf_y1 + y_padding
}
```

### content_text.rs 中的 is_in_header_zone

```rust
fn is_in_header_zone(y: f32, plan: &PagePlainTextPlan) -> bool {
    y >= plan.page_box.max_y - plan.header_zone_pt
        && y <= plan.page_box.max_y + 24.0
        && plan.page_box.width > 0.0
}
```

### 前端 buildPlainTextTargets

```javascript
function buildPlainTextTargets(file, region) {
  if (Array.isArray(file.existingElements) && file.existingElements.length) {
    const kinds = region === 'header' ? ['header'] : ['footerText', 'pageNumber']
    return file.existingElements
      .filter(element =>
        kinds.includes(element.kind) &&
        element.source !== 'artifact' &&
        ['delete', 'edit'].includes(element.decision),
      )
      .map(element => ({
        text: element.detectedText,
        normalizedText: element.normalizedText || element.detectedText,
        pageStart: element.pageStart || 1,
        pageEnd: element.pageEnd || file.pages || 1,
        bbox: element.bbox || null,
      }))
      .filter(target => target.text)
  }
  // fallback...
}
```

---

## 十、我们的疑问清单

请逐一回答：

1. **Bug 4 根因**：我们的分析（zone gate + CID 编码 + Form 坐标空间）是否正确？有没有其他失败路径？

2. **方案 2 可行性**：Form 坐标变换是否可行？最简单的实现方式是什么？

3. **有没有更好的方案**？比如：
   - 能不能在检测阶段就记录文本在内容流中的位置（操作符索引），删除时直接用？
   - 能不能在 Form XObject 处理时，先合并到页面内容流再统一处理？
   - 能不能用 PDF 的 Structured Content 或 Tagged PDF 来定位？

4. **现有代码的其他问题**：
   - `content_text.rs` 中有没有其他 bug？
   - `artifacts.rs` 中有没有其他 bug？
   - 前端参数构建有没有其他问题？

5. **对 8 个需求的评估**：每个需求的技术可行性和实现难度？

6. **你的修复方案**：给出具体的代码修改建议。
