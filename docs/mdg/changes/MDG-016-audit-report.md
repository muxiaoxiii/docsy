# MDG-016 页眉页脚模块深度审计报告

> 审计范围：4 个 Rust 后端文件 + 6 个前端文件，共约 9400 行代码
> 审计日期：2026-08-08

---

## 1. 模块概述

Docsy 的页眉页脚模块是一个**双路径检测 + 双路径删除**的架构：

### 检测路径（detection.rs）
```
detect_pdf_header_footer
  ├─ artifacts::inspect_meaningful_header_footer_artifacts  ← Artifact 标记检测
  │    └─ 扫描 BDC/EMC marked content，识别 /Artifact /Subtype /Header|/Footer
  └─ run_pdftotext_bbox → parse_pdftotext_bbox              ← pdftotext 文本检测
       └─ 调用 poppler pdftotext -bbox，正则解析 XML 得到 WordBox
            └─ build_page_detections → build_candidates     ← 按 zone 聚类
```

### 删除路径（header_footer.rs → process_job）
```
process_job
  ├─ edit_or_delete_standard_artifacts_if_requested         ← Artifact 路径
  │    └─ artifacts::edit_header_footer_artifacts_to_temp
  │         └─ edit_target_artifact_ranges（BDC/EMC 原位编辑/删除）
  ├─ delete_confirmed_plain_text_header_footer_if_requested ← 普通文本路径
  │    └─ content_text::delete_plain_header_footer_to_temp
  │         └─ filter_page_operations（zone check + 文本匹配）
  └─ build_overlay_pdf → qpdf overlay                        ← 叠加新页眉页脚
```

### 核心数据流
前端检测 → `file.existingElements`（含 decision: keep/edit/delete/ignore）→ `buildHeaderFooterItems` 构建后端参数 → `cleanup.plainHeaderTargets` / `cleanup.forceDeleteHeader` → 后端执行删除 → qpdf overlay 叠加新内容。

**架构上的根本问题：检测和删除用了两套完全不同的 PDF 解析引擎**——检测用 poppler（pdftotext），删除用 lopdf。两者的字体解码能力、坐标系、文本提取方式完全不同，这直接导致了问题 A。

---

## 2. 问题 A 分析与修复

### 2.1 完整数据流追踪

```
1. 前端检测
   useEvidencePdfDetection.js: detectFileHeaderFooter(file)
   → tauriCallQuiet('detect_pdf_header_footer', { inputPath, maxPages, headerZoneMm, footerZoneMm })
   → 后端 detection.rs: detect_pdf_header_footer()

2. 后端检测
   detection.rs:
   ├─ artifacts::inspect_meaningful_header_footer_artifacts()  → artifact 候选
   └─ run_pdftotext_bbox() → parse_pdftotext_bbox()
        → build_page_detections() → build_candidates()
        → 返回 headerCandidates / footerCandidates（source = "content-text" 或 "artifact"）

3. 前端存储检测结果
   useEvidencePdfDetection.js: applyDetectionResultToFile(file, data)
   → file.existingHeaderText = header?.text           // pdftotext 提取的真实文本
   → file.existingHeaderBBox = header?.bbox            // pdftotext 坐标（原点左上，y 向下）
   → file.existingHeaderArtifact = (header?.source === 'artifact')
   → file.existingElements = mergeExistingElements(...)

4. 用户标记删除
   useEvidencePdfExistingEditing.js: finishExistingHeaderEdit(row) 或直接设置
   → row.removeExistingHeader = true
   → element.decision = 'delete'

5. 前端构建后端参数
   useEvidencePdfSession.js: buildHeaderFooterItems()
   → buildPlainTextTargets(file, 'header')
     → file.existingElements.filter(element => element.decision === 'delete')
     → 返回 [{ text, normalizedText, pageStart, pageEnd, bbox }]
   → cleanup.plainHeaderTargets = [...]
   → cleanup.forceDeleteHeader = hasArtifactDecision(file, ['header'], 'delete')

6. 后端执行删除
   header_footer.rs: process_job()
   ├─ edit_or_delete_standard_artifacts_if_requested()  ← artifact 路径
   │    → artifacts::edit_header_footer_artifacts_to_temp()
   └─ delete_confirmed_plain_text_header_footer_if_requested()  ← 普通文本路径
        → content_text::delete_plain_header_footer_to_temp()
        → filter_page_operations()
             → is_in_header_zone(state.y, plan)      ← zone check（PDF 原生坐标）
             → matches_any_target(text, targets, &state)
                  → target_matches(text, target)      ← 文本精确匹配
                  → target_bbox_matches(state, target) ← bbox 位置兜底
```

### 2.2 删除失败的可能路径

| # | 失败路径 | 代码位置 | 失败条件 |
|---|---------|---------|---------|
| 1 | **CID 字体文本不匹配** | content_text.rs:401 `object_text` + 427 `matches_any_target` | pdftotext 提取真实中文，lopdf 解码出 GID 乱码字节，`normalize_for_match` 后不相等 |
| 2 | **bbox 坐标系不匹配** | content_text.rs:449 `target_bbox_matches` | detection bbox 用 pdftotext 坐标（原点左上，y 向下），删除用 PDF 原生坐标（原点左下，y 向上），坐标系方向相反 |
| 3 | **zone check 阻止 bbox 匹配** | content_text.rs:300-308 | `matches_any_target` 在 `is_in_header_zone` 通过后才被调用，zone check 是 gate；如果文本不可解码（GID 乱码），text 匹配失败后转向 bbox 匹配，但 bbox 匹配用的是 `state`（PDF 坐标），而 target.bbox 是 pdftotext 坐标，坐标系不一致导致 bbox 匹配也失败 |
| 4 | **Form XObject 坐标空间** | content_text.rs:197 `filter_page_operations` | Form XObject 内的文本坐标是表单局部坐标，与页面坐标不同，但 `filter_referenced_form_text` 传入的 `plan` 使用页面级 `page_box`，zone check 用错误的坐标系 |
| 5 | **target_bbox_matches 的 y 翻转错误** | content_text.rs:461-462 | `pdf_y0 = bbox.height - bbox.y1`，这是正确的翻转，但 `state.y` 是 PDF 原生坐标，而 `bbox.height` 来自 pdftotext 的页面高度。如果页面有旋转（/Rotate），两者坐标系完全不同 |
| 6 | **plainHeaderTargets 为空** | useEvidencePdfSession.js:664-685 `buildPlainTextTargets` | 如果 `file.existingElements` 不为空但没有任何元素的 decision === 'delete'，返回空数组 |
| 7 | **artifact 路径和 plain text 路径互斥判断错误** | useEvidencePdfSession.js:802-814 `hasArtifactDecision` | 如果 existingElements 为空，回退到旧的 `file.existingHeaderArtifact && file.removeExistingHeader` 判断；如果 existingElements 不为空但元素 source 不是 'artifact'，返回 false，forceDeleteHeader 为 false |

### 2.3 对三个猜测原因的评价

| 猜测 | 评价 | 是否真正原因 |
|------|------|-------------|
| **Zone check 作为 gate 阻止 bbox 匹配** | ✅ **正确，但不是根因**。zone check 确实是 gate（content_text.rs:300-308），但即使绕过 zone check，bbox 匹配也会因坐标系不一致而失败。zone check 只是让问题更隐蔽。 | 部分原因 |
| **CID 字体编码导致文本匹配失败** | ✅ **这是最核心的根因**。pdftotext 通过 ToUnicode CMap 正确解码中文，lopdf 的 `object_text`（content_text.rs:401-413）仅处理 UTF-16BE BOM 和直接 UTF-8，完全没有 ToUnicode CMap 查找。对于 CID 字体（宋体、黑体等），PDF 内容流中的 Tj 操作数是 GID 字节序列，lopdf 解码出的是乱码，与 pdftotext 提取的真实文本永远不相等。 | **核心根因** |
| **Form XObject 坐标空间不匹配** | ✅ **正确，是另一个独立原因**。Form XObject 内的坐标是表单局部坐标，`filter_referenced_form_text`（content_text.rs:144-232）传入的 `plan.page_box` 是页面级 MediaBox，zone check（`is_in_header_zone`）用页面级坐标系检查表单局部坐标，必然失败。 | 独立原因 |

**还有没有其他原因？** 有：

- **bbox 坐标系方向相反**：pdftotext `-bbox` 的原点在左上、y 向下；PDF 原生坐标系原点在左下、y 向上。`target_bbox_matches` 中的 `pdf_y0 = bbox.height - bbox.y1` 尝试翻转，但这是基于"pdftotext 的 y0/y1 是从顶部计算"的假设。如果页面有 /Rotate，或者 CropBox 与 MediaBox 不一致，翻转完全错误。
- **文本被拆分到多个 Tj/TJ 操作**：`shown_text`（content_text.rs:377-399）逐操作提取文本。如果页眉文本 "证据1" 被编码为 `Tj("证据") Tj("1")` 两个操作，每个操作的 shown_text 分别是 "证据" 和 "1"，与 target.text "证据1" 都不匹配。detection.rs 的 pdftotext 会把它们合并为一个 word。

### 2.4 对三个修复方案的评价

| 方案 | 评价 |
|------|------|
| **方案 1：绕过 zone check** | ❌ **治标不治本**。绕过 zone check 后，bbox 匹配仍因坐标系不一致而失败。而且绕过 zone check 有误删风险——正文区域可能有与页眉相同的文本（如引用页眉内容）。zone check 是必要的安全门，不应移除。 |
| **方案 2：Form XObject 坐标变换** | ✅ **正确方向**，但只解决问题的一部分（Form XObject 内的坐标空间）。需要读取表单的 BBox 和 Matrix，把局部坐标转为页面坐标再做 zone check。但这不能解决 CID 字体编码问题。 |
| **方案 3：白底矩形覆盖** | 用户不接受，跳过。 |

**更好的方案**：见下方 2.5。

### 2.5 我的修复建议

核心思路：**让删除路径使用与检测路径相同的文本提取能力**，而非依赖 lopdf 的有限解码。

#### 修复 1：bbox 匹配独立于 zone check（低风险，立即修复）

将 `filter_page_operations` 中的逻辑从"zone check 作为 gate"改为"文本匹配 OR bbox 匹配，zone check 仅作为文本匹配的附加条件"：

```rust
// content_text.rs: filter_page_operations (约 287-318 行)
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
            // 文本匹配：必须在 zone 内 + 文本匹配
            if is_in_header_zone(state.y, plan)
                && matches_any_target_text(text, &plan.header_targets, &state)
            {
                remove_region = Some(TextRegion::Header);
            } else if is_in_footer_zone(state.y, plan)
                && matches_any_target_text(text, &plan.footer_targets, &state)
            {
                remove_region = Some(TextRegion::Footer);
            }
        }
        // bbox 匹配：独立于 zone check，不要求在 zone 内
        // 这处理 CID 字体无法解码但检测时有 bbox 的情况
        if remove_region.is_none() {
            if target_bbox_matches_any(&state, &plan.header_targets) {
                remove_region = Some(TextRegion::Header);
            } else if target_bbox_matches_any(&state, &plan.footer_targets) {
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

// 新增辅助函数
fn target_bbox_matches_any(state: &TextState, targets: &[&PlainTextTarget]) -> bool {
    targets.iter().any(|target| target_bbox_matches(state, target))
}

fn matches_any_target_text(text: &str, targets: &[&PlainTextTarget], _state: &TextState) -> bool {
    targets.iter().any(|target| target_matches(text, target))
}
```

#### 修复 2：bbox 坐标系统一（关键修复）

`target_bbox_matches` 中的坐标翻转逻辑有误。detection 返回的 bbox 使用 pdftotext 坐标（原点左上，y 向下），需要正确转换为 PDF 坐标（原点左下，y 向上）：

```rust
// content_text.rs: target_bbox_matches (约 449-467 行)
fn target_bbox_matches(state: &TextState, target: &PlainTextTarget) -> bool {
    let Some(bbox) = target.bbox else {
        return false;
    };
    if bbox.width <= 0.0 || bbox.height <= 0.0 {
        return false;
    }
    if bbox.page > 0 && (bbox.page < target.page_start || bbox.page > target.page_end) {
        return false;
    }
    let x_padding = 18.0;
    let y_padding = 18.0;
    // pdftotext 坐标：原点左上，y 向下
    // PDF 坐标：原点左下，y 向上
    // 转换：pdf_y = page_height - pdftotext_y
    let pdf_y0 = bbox.height - bbox.y1;  // bbox 下边界在 PDF 坐标中是上方
    let pdf_y1 = bbox.height - bbox.y0;  // bbox 上边界在 PDF 坐标中是下方
    // state.y 是 PDF 原生坐标（原点左下）
    state.x >= bbox.x0 - x_padding
        && state.x <= bbox.x1 + x_padding
        && state.y >= pdf_y0 - y_padding
        && state.y <= pdf_y1 + y_padding
}
```

**注意**：当前代码的翻转公式其实是对的（`pdf_y0 = bbox.height - bbox.y1`），但问题在于 `bbox.height` 来自 pdftotext 的页面高度，可能与 lopdf 解析的 `page_box.max_y` 不同（尤其有 CropBox 或 /Rotate 时）。需要确保 `bbox.height` 与 `page_box` 使用相同的页面尺寸。

#### 修复 3：Form XObject 坐标变换（中等风险，需测试）

```rust
// content_text.rs: filter_referenced_form_text (约 144-232 行)
fn filter_referenced_form_text(
    doc: &mut Document,
    operations: &[Operation],
    xobjects: &Dictionary,
    plan: &PagePlainTextPlan,
    visited: &mut HashSet<ObjectId>,
) -> Result<PlainTextCleanupResult> {
    let mut result = PlainTextCleanupResult::default();
    for operation in operations.iter().filter(|op| op.operator == "Do") {
        // ... 现有的对象查找逻辑 ...
        
        // 读取 Form XObject 的 BBox 和 Matrix
        let form_bbox = stream_dict.get(b"BBox").ok().and_then(|v| {
            // 解析 [x0 y0 x1 y1]
            v.as_array().ok().and_then(|arr| {
                if arr.len() == 4 {
                    Some((
                        object_number(&arr[0])? as f32,
                        object_number(&arr[1])? as f32,
                        object_number(&arr[2])? as f32,
                        object_number(&arr[3])? as f32,
                    ))
                } else {
                    None
                }
            })
        });
        let form_matrix = stream_dict.get(b"Matrix").ok().and_then(|v| {
            // 解析 6 元素矩阵 [a b c d e f]
            v.as_array().ok().and_then(|arr| {
                if arr.len() == 6 {
                    Some([
                        object_number(&arr[0])? as f32,
                        object_number(&arr[1])? as f32,
                        object_number(&arr[2])? as f32,
                        object_number(&arr[3])? as f32,
                        object_number(&arr[4])? as f32,
                        object_number(&arr[5])? as f32,
                    ])
                } else {
                    None
                }
            })
        });

        // 为 Form XObject 构建调整后的 plan
        let form_plan = if let Some((fx0, fy0, fx1, fy1)) = form_bbox {
            // Form 的 BBox 定义了局部坐标系
            // 默认 Matrix 是 [1 0 0 1 0 0]（无变换）
            let (mx, my) = form_matrix
                .map(|m| (m[4], m[5]))
                .unwrap_or((0.0, 0.0));
            PagePlainTextPlan {
                header_targets: plan.header_targets.clone(),
                footer_targets: plan.footer_targets.clone(),
                header_zone_pt: plan.header_zone_pt,
                footer_zone_pt: plan.footer_zone_pt,
                // Form 的坐标系：BBox 的 y 范围是 [fy0, fy1]
                // 需要把页面级 zone 映射到 Form 局部坐标
                page_box: PageBox {
                    width: fx1 - fx0,
                    min_y: fy0,
                    max_y: fy1,
                },
            }
        } else {
            // 没有 BBox，回退到页面级 plan（旧行为）
            plan.clone()
        };

        let (filtered, form_result) = filter_page_operations(&content.operations, &form_plan);
        // ... 后续逻辑不变 ...
    }
    Ok(result)
}
```

**注意**：`PagePlainTextPlan` 目前没有 derive Clone，需要加上。`Vec<&PlainTextTarget>` 也需要改为 `Vec<PlainTextTarget>` 或用其他方式处理生命周期。

#### 修复 4：文本拼接匹配（低风险，立即修复）

处理页眉文本被拆分到多个 Tj 操作的情况：

```rust
// content_text.rs: filter_page_operations
// 在循环外维护一个 "当前 BT/ET 块内的拼接文本"
fn filter_page_operations(
    operations: &[Operation],
    plan: &PagePlainTextPlan,
) -> (Vec<Operation>, PlainTextCleanupResult) {
    let mut output = Vec::with_capacity(operations.len());
    let mut result = PlainTextCleanupResult::default();
    let mut state = TextState::default();
    let mut text_block_buffer = String::new();  // 当前文本块拼接
    let mut text_block_ops: Vec<usize> = Vec::new();  // 当前文本块的操作索引

    for (op_index, operation) in operations.iter().enumerate() {
        update_text_state_before_show(&mut state, operation);
        
        if operation.operator == "BT" {
            text_block_buffer.clear();
            text_block_ops.clear();
        }
        
        let shown = shown_text(operation);
        if let Some(text) = &shown {
            text_block_buffer.push_str(text);
            text_block_ops.push(op_index);
        }
        
        // 在 ET 或文本块结束时检查拼接文本
        if operation.operator == "ET" && !text_block_buffer.is_empty() {
            let combined = normalize_for_match(&text_block_buffer);
            // 检查拼接文本是否匹配任何 target
            for target in &plan.header_targets {
                let target_text = normalize_for_match(&target.text);
                if combined == target_text {
                    // 标记整个文本块的所有操作为删除
                    // （实现略：需要记录哪些操作要删除）
                }
            }
            text_block_buffer.clear();
            text_block_ops.clear();
        }
        
        // ... 原有的单操作匹配逻辑 ...
    }
    (output, result)
}
```

### 2.6 修复优先级

1. **P0 - 立即修复**：bbox 匹配独立于 zone check（修复 1）+ 确保坐标翻转正确（修复 2）
2. **P1 - 短期修复**：Form XObject 坐标变换（修复 3）
3. **P2 - 中期改进**：文本拼接匹配（修复 4）
4. **P3 - 长期架构改进**：在 lopdf 层面增加 ToUnicode CMap 查找，或改用 pdfium/poppler 做删除

---

## 3. 问题 B/C 分析与修复

### 3.1 问题 B 根因：渲染路径与列表显示路径的代码差异

**渲染路径**（`buildHeaderFooterItems` → 后端实际写入）：
```js
// useEvidencePdfSession.js:506-508
const header =
  headerInsertEnabled && headerGroup && headerGroup.enabled !== false && headerModeValue !== 'none'
    ? buildHeaderTextForGroup(file, index, { ...headerGroup, mode: headerModeValue }, rules)
    : ''
```
`buildHeaderTextForGroup` → `headerBaseTextForGroup` → `mode === 'custom'` 返回 `group.text`（即 `证据[#]`）→ `decorateHeaderTextForGroup` 展开 `[#]` → 得到 `证据1`。✅ 正确。

**文件列表显示路径**：
```js
// useEvidencePdfExistingEditing.js:216-222
function displayRowHeader(row, index) {
  if (!insertHeaderFooterEnabled.value) return ''
  if (workflowMode.value === 'split' && headerMode.value === 'per_file') {
    return row?.header ?? rowHeaderPreview(row, index)
  }
  return rowHeaderPreview(row, index)
}
```
`rowHeaderPreview` → `buildHeaderText(row, index, currentRules.value)`：
```js
// useEvidencePdfSession.js:326-333
export function buildHeaderText(file, index, rules) {
  if (file?.headerEdited) {
    return decorateHeaderText(file.header ?? '', file, index, rules)  // ← 问题在这
  }
  if (rules.headerMode === 'none') return ''
  const base = headerBaseText(file, index, rules)
  return decorateHeaderText(base, file, index, rules)
}
```

**根因**：如果 `file.headerEdited === true`（用户曾编辑过页眉），`buildHeaderText` 使用 `file.header`（在 `createEvidenceFile` 中初始化为 `stripPdf(name)`，即文件名），而不是 `rules.headerText`（固定文本 `证据[#]`）。

触发条件：用户在「按证据列表名称」模式下编辑过某文件的页眉（`startHeaderEdit` → `finishHeaderEdit` 设置 `headerEdited = true`），然后切换到「固定文本」模式。此时 `headerEdited` 仍为 true，`buildHeaderText` 使用旧的 `file.header`（文件名），而渲染路径 `buildHeaderTextForGroup` 直接使用 `group.text`。

### 3.2 问题 C 根因：per_file 模式的语义错配

```js
// useEvidencePdfSession.js:355-362
function headerBaseTextForGroup(file, index, group, _rules) {
  if (group.mode === 'per_file') return file.header ?? stripPdf(file.name)  // ← 返回文件名
  if (group.mode === 'seq') return `证据${index + 1}`                        // ← 返回证据序号
  // ...
}
```

UI 中「按证据列表名称」映射到 `per_file` 模式，但 `per_file` 返回 `file.header`（默认是 `stripPdf(name)` = 文件名）。用户期望的是「证据1」「证据2」，这对应的是 `seq` 模式。

**根因**：UI 标签「按证据列表名称」与代码语义错配。`per_file` 的含义是"使用每个文件自己的 header 字段"，而非"按证据列表顺序生成序号"。

### 3.3 修复建议

#### 修复问题 B

`buildHeaderText` 在 `headerEdited` 为 true 时不应忽略 `rules.headerMode`。只有当 `headerMode === 'per_file'` 时才应使用 `file.header`：

```js
// useEvidencePdfSession.js:326-333
export function buildHeaderText(file, index, rules) {
  // 只有 per_file 模式才尊重 file.headerEdited 和 file.header
  if (rules.headerMode === 'per_file' && file?.headerEdited) {
    return decorateHeaderText(file.header ?? '', file, index, rules)
  }
  if (rules.headerMode === 'none') return ''
  const base = headerBaseText(file, index, rules)
  return decorateHeaderText(base, file, index, rules)
}
```

#### 修复问题 C

有两种方案：

**方案 A（推荐）**：修改 `per_file` 模式的行为，让它使用证据列表序号而非文件名：

```js
function headerBaseTextForGroup(file, index, group, _rules) {
  if (group.mode === 'per_file') return file.header || `证据${index + 1}`  // 优先用自定义，否则序号
  // ...
}
```

**方案 B**：修改 UI 标签，把「按证据列表名称」改名为「使用文件名」，新增一个「证据序号」选项映射到 `seq`。

建议方案 A，因为「按证据列表名称」的语义应该是"按照证据列表的顺序生成名称"。

---

## 4. 问题 D 分析与修复

### 4.1 当前序列生成逻辑

```js
// splitFileName.js:34-44
export function expandSplitNameTokens(value, index = 0, dateValue = '') {
  return String(value || '').replace(/\[([^\]]+)\]/g, (match, token) => {
    if (/^#+$/.test(token)) return formatSequenceToken(token, index)  // [#] → 01, [##] → 01
    if (token === '序号') return String(index + 1)                     // [序号] → 1
    if (token === '中文序号') return toChineseNumber(index + 1)         // [中文序号] → 一
    // ...
  })
}

// splitFileName.js:46-49
export function formatSequenceToken(token, index = 0) {
  const value = String(Math.max(1, Number(index || 0) + 1))  // ← 始终从 1 开始
  return value.padStart(token.length, '0')
}
```

序列始终从 `index + 1` 开始，`index` 是文件在列表中的位置（0-based），无法自定义起点和步长。

### 4.2 修改建议：支持自定义起点和步长

#### 方案 1：扩展占位符语法 `[#, 起点, 步长]`

```js
// splitFileName.js
export function expandSplitNameTokens(value, index = 0, dateValue = '', options = {}) {
  const start = Number(options.sequenceStart ?? 1)
  const step = Number(options.sequenceStep ?? 1)
  const seq = start + index * step  // 实际序号

  return String(value || '').replace(/\[([^\]]+)\]/g, (match, token) => {
    // 支持 [#,3,2] 格式：起点 3，步长 2
    const parts = token.split(',').map(s => s.trim())
    if (parts[0].match(/^#+$/)) {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      const tokenSeq = tokenStart + index * tokenStep
      return formatSequenceToken(parts[0], 0, tokenSeq)
    }
    if (parts[0] === '序号') {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      return String(tokenStart + index * tokenStep)
    }
    if (parts[0] === '中文序号') {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      return toChineseNumber(tokenStart + index * tokenStep)
    }
    // ... 日期等其他 token ...
    return match
  })
}

export function formatSequenceToken(token, index = 0, overrideValue = null) {
  const value = overrideValue != null
    ? String(overrideValue)
    : String(Math.max(1, Number(index || 0) + 1))
  return value.padStart(token.length, '0')
}
```

#### 方案 2：全局起点/步长设置

在 `HeaderFooterRuleFields.vue` 的页眉来源区域增加「起始序号」和「步长」输入框，将值存入 `rules.sequenceStart` 和 `rules.sequenceStep`，然后传递给 `expandSplitNameTokens`。

建议同时实现两种方案：方案 2 提供全局默认值，方案 1 提供每个占位符的精细控制。

---

## 5. 其他发现

### 5.1 代码质量问题

1. **新旧 API 并存**：`buildHeaderText`（旧 API，用 `rules.headerMode`）和 `buildHeaderTextForGroup`（新 API，用 `group.mode`）并存，容易混淆。`displayRowHeader` 用旧 API，`buildHeaderFooterItems` 用新 API，导致显示和渲染不一致。**建议**：统一到 group-based API。

2. **`headerBaseText` 与 `headerBaseTextForGroup` 重复**：两个函数逻辑几乎相同，一个用 `rules.headerMode`，一个用 `group.mode`。**建议**：合并为一个函数，从 group 取 mode。

3. **`createEvidenceFile` 中 `header: stripPdf(name)` 的副作用**：文件创建时 `header` 被初始化为文件名，这在 `per_file` 模式下作为默认值。但如果用户切换模式，这个初始值会干扰其他模式的显示（问题 B 的根因）。**建议**：`header` 初始化为 `null`，在 `per_file` 模式下用 `file.header ?? stripPdf(file.name)` 作为 fallback。

4. **`content_text.rs` 中 `update_text_state_after_show` 是空函数**（line 367）：这是一个 TODO 或遗漏。PDF 文本状态在 Tj/TJ 后可能需要更新（如 TJ 中的 kerning 调整会影响 x 坐标），当前被忽略，可能导致 bbox 匹配时 state.x 不准确。

5. **detection.rs 和 content_text.rs 的 `normalize_for_match` 实现不一致**：detection.rs 有 `normalize_for_content_match`（模糊包含），content_text.rs 有 `normalize_for_match`（精确去空格），两者逻辑不同，导致"检测能匹配但删除不能匹配"。

### 5.2 潜在 Bug

1. **页码 dingbat 格式错误**：`header_footer.rs:1758` 中 `char::from_u32(0x2775 + value)`，0x2775 是 `❵`，不是 `❶`。`❶` 的 Unicode 是 0x2776。应该是 `0x2776 + value - 1`。但前端的 `pdfPageNumberRules.js:4` 中 DINGBAT 数组用的是硬编码，是正确的。

2. **`pageNumberOverlaysForFile` 中的 signature 合并逻辑**（pdfPageNumberRules.js:54-83）：当连续页的签名相同且 number 连续时合并。但如果 numberOffset 在中途变化（通过 override），可能导致合并不当，进而少生成 overlay。

3. **`overlayConfigForFile` 中 `useDetectedPlacement` 的 region 判断**（useEvidencePdfSession.js:712-717）：`region === 'pageNumber'` 时 `isHeader = false`，但 base 对象中 `artifactKind: region === 'pageNumber' ? 'PageNumber' : 'FooterText'`。当 region 是 'pageNumber' 时，base 的其他字段用的是 footer 配色，但 marginMm 用的是 footer 的。如果页码实际在页眉区域，margin 计算会错误。

4. **`filter_page_operations` 的 TextState 只跟踪单个文本位置**：对于 TJ 操作中的多个字符串（带 kerning 调整），state.x 只记录最后一个 Td/Tm 的位置，不跟踪 TJ 内部的字符位置移动。如果页眉文本是 TJ 中的第二个字符串，state.x 可能指向第一个字符串的位置，导致 bbox 匹配错误。

### 5.3 性能问题

1. **大文件检测慢**：`DETECTION_SCAN_MAX_PAGES = 20`，对于超过 20 页的文件只扫描前 20 页。但 pdftotext 仍然需要处理整个文件（因为 `-bbox` 模式没有页范围参数）。**建议**：改用 `pdftotext -bbox -f 1 -l 20` 限制页范围。

2. **检测串行执行**：`detectAllHeaderFooter` 中 for 循环串行检测每个文件。**建议**：用 `Promise.all` 或 Worker 并行检测。

---

## 6. UI 改进建议

阅读 `HeaderFooterRuleFields.vue` 后发现以下 UI 问题：

### 6.1 信息架构混乱

当前 UI 是线性的：页眉文字 → 页脚文字 → 页码，每个 section 内有大量配置项。问题是：
- 「页眉来源」下拉有 4 个选项（不插入/文件名/按证据列表名称/固定文本），但「按证据列表名称」的语义不清晰
- 页眉前缀/后缀与页眉来源的关系不明确——前缀/后缀是加在什么基础上的？
- 页码的「分段与例外」是一个按钮，打开后是另一个弹窗，信息层级不一致

**建议**：
- 将「页眉来源」改为 3 个互斥的卡片选择：不插入 / 文件名 / 证据序号（含序号格式、起点、步长）/ 固定文本
- 前缀/后缀只在「文件名」和「证据序号」模式下显示，固定文本模式下隐藏（因为固定文本本身就可以包含完整内容）
- 页码的分段与例外改为内联展开，而非弹窗

### 6.2 「分段」标签含义不清

```html
<label>分段</label>
<el-input-number v-model="headerPageStartModel" ... placeholder="起始页" />
<span class="range-sep">–</span>
<el-input-number v-model="headerPageEndModel" ... placeholder="0=全部" />
```

「分段」这个词暗示"把文档分成几段"，但实际功能是"此页眉只在指定页码范围内显示"。**建议**：改为「显示范围」或「页码范围」。

### 6.3 模板标记帮助不够醒目

模板标记的帮助信息在 tooltip 中（点击 InfoFilled 图标），用户很难发现。**建议**：在输入框下方常驻显示常用标记的快捷按钮（如 `[序号]` `[文件名]` `[日期]`），点击即插入。

### 6.4 页码预览只有一行

```html
<span class="page-number-preview">{{ pageNumberPreviewText }}</span>
```
只显示第 1 页的预览。**建议**：显示第 1 页、中间页、最后页的预览，帮助用户确认格式。

### 6.5 缺少合并完成提示

用户报告"合并完成后没有明显提示"。**建议**：合并完成后显示一个全屏 overlay 或居中弹窗，带"打开文件夹"和"打开文件"按钮。

---

## 7. 需求评估

| 需求 | 技术可行性 | 实现难度 | 说明 |
|------|-----------|---------|------|
| **检测与删除统一** | ⭐⭐⭐⭐ | 高 | 核心需求。需要在 lopdf 层增加 ToUnicode CMap 查找，或改用 pdfium 做删除。短期可用 bbox 匹配独立于 zone check 来缓解。 |
| **按证据列表名称 UI 重构** | ⭐⭐⭐⭐⭐ | 低 | 纯前端改动。重构 HeaderFooterRuleFields.vue 的页眉来源区域，增加前缀/序号格式/起始/步长专用设置区。 |
| **占位符增强 `[#, 起点, 步长]`** | ⭐⭐⭐⭐⭐ | 低 | 修改 `expandSplitNameTokens` 和 `formatSequenceToken`，解析逗号分隔的参数。纯前端。 |
| **序列格式选项** | ⭐⭐⭐⭐⭐ | 低 | 已有 `toChineseNumber`、`toRoman`，只需在 UI 增加选项并传入 `formatSequenceToken`。 |
| **奇偶页支持** | ⭐⭐⭐⭐ | 中 | 需要在 `overlay_applies_to_page`（Rust）和 `overlayAppliesToPage`（JS）中增加奇偶页判断。`build_overlay_pdf` 的 candidate 筛选也需要支持。 |
| **分段→显示范围** | ⭐⭐⭐⭐⭐ | 低 | 纯 UI 改名 + 微调。已有 pageStart/pageEnd 逻辑，只需改标签。 |
| **大文件检测优化** | ⭐⭐⭐⭐ | 中 | pdftotext 加 `-f` `-l` 参数限制页范围；检测并行化用 Worker。 |
| **合并完成醒目提示** | ⭐⭐⭐⭐⭐ | 低 | 纯前端，合并完成后显示 overlay 弹窗。 |

---

## 8. 总结与建议

### 8.1 修复优先级

| 优先级 | 问题 | 修复方案 | 预计工作量 |
|--------|------|---------|-----------|
| **P0** | 问题 A：CID 字体删除失败 | bbox 匹配独立于 zone check + 坐标系统一 | 2-3 天 |
| **P0** | 问题 B：列表显示不一致 | `buildHeaderText` 中 `headerEdited` 仅在 per_file 模式生效 | 0.5 天 |
| **P1** | 问题 C：per_file 语义错配 | 修改 `headerBaseTextForGroup` 的 per_file 行为 | 0.5 天 |
| **P1** | 问题 D：序列起点 | 扩展 `expandSplitNameTokens` 支持起点/步长 | 1 天 |
| **P1** | Form XObject 坐标变换 | `filter_referenced_form_text` 中读取 BBox/Matrix | 2 天 |
| **P2** | dingbat 页码格式错误 | 修正 `0x2775` → `0x2776 + value - 1` | 0.1 天 |
| **P2** | 文本拼接匹配 | `filter_page_operations` 中维护文本块缓冲区 | 1-2 天 |
| **P2** | 新旧 API 统一 | 废弃 `buildHeaderText`，统一到 `buildHeaderTextForGroup` | 1 天 |
| **P3** | 大文件检测优化 | pdftotext 加页范围参数 + 并行检测 | 1-2 天 |
| **P3** | UI 重构 | 页眉来源卡片化 + 显示范围改名 + 模板快捷按钮 | 2-3 天 |

### 8.2 架构改进建议

**短期（1-2 周）**：
1. 修复 bbox 匹配逻辑，使其独立于 zone check
2. 统一坐标系（pdftotext → PDF 原生坐标的转换）
3. 修复前端的 headerEdited 判断逻辑
4. 支持序列起点和步长

**中期（1-2 月）**：
1. 在 lopdf 层面增加 ToUnicode CMap 查找，使文本匹配不再依赖 bbox 兜底
2. Form XObject 坐标变换
3. 检测并行化
4. UI 重构

**长期（3-6 月）**：
1. 考虑用 pdfium（Google Chrome 的 PDF 引擎）替换 lopdf 做删除操作，统一检测和删除的 PDF 解析能力
2. 或者将检测和删除都改为基于 lopdf（去掉 pdftotext 依赖），但需要在 lopdf 层实现完整的字体解码

### 8.3 核心结论

**问题 A 的根因是检测和删除使用了两个不同的 PDF 解析引擎，且两者的字体解码能力和坐标系不一致**。pdftotext（poppler）能通过 ToUnicode CMap 正确解码中文，lopdf 只能处理 UTF-16BE 和直接 UTF-8，导致 CID 字体的文本匹配必然失败。bbox 兜底匹配也因坐标系方向相反而不可靠。

短期最有效的修复是**让 bbox 匹配独立于 zone check，并确保坐标翻转正确**。这能让"检测到 bbox 但文本不可解码"的情况也能被删除。长期来看，应在 lopdf 层增加 ToUnicode CMap 查找，或改用更强大的 PDF 引擎。

**问题 B/C 的根因是新旧 API 并存且 `headerEdited` 标志的判断不够精确**。`buildHeaderText`（旧 API）在 `headerEdited` 为 true 时无条件使用 `file.header`，忽略了当前 `headerMode` 是否为 `per_file`。修复方法是只在 `per_file` 模式下尊重 `headerEdited`。

**问题 D 是功能缺失**，当前序列始终从 1 开始。修复方法是扩展 `expandSplitNameTokens` 支持起点和步长参数。
