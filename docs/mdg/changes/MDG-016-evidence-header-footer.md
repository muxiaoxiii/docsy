# MDG-016: 证据处理模块页眉页脚全面优化

## 状态：✅ 已完成
## 优先级：P0
## 创建日期：2026-08-08
## 审计来源：Claude 审计 + GLM 5.2 审计 + 交叉审计 + 优化建议

---

## 一、问题清单

### Bug 1：文件列表与实际渲染数据不一致
- **现象**：固定文本 `证据[#]` 渲染正确，但文件列表显示文件名
- **根因**：渲染用 `buildHeaderTextForGroup`（读 `group.mode`），文件列表用 `buildHeaderText`（读 `rules.headerMode`），两套函数不同步
- **深层原因**：新旧 API 双轨制——`buildHeaderText`（旧，用 `rules.headerMode`）和 `buildHeaderTextForGroup`（新，用 `group.mode`）并存
- **修复**：废弃旧 API，`displayRowHeader` 改为调用 `buildHeaderTextForGroup`

### Bug 2：按证据列表名称使用文件名
- **现象**：选「按证据列表名称」，期望「证据1」「证据2」，实际显示文件名
- **根因**：`headerBaseTextForGroup` 中 `per_file` 模式返回 `file.header ?? stripPdf(file.name)`，而 `file.header` 在 `startHeaderEdit` 中初始化时 `headerMode` 可能还没切换到 `per_file`
- **修复**：`per_file` fallback 改为 `证据${index + 1}`

### Bug 3：序列始终从 1 开始
- **根因**：`formatSequenceToken` 硬编码 `Math.max(1, index + 1)`
- **修复**：支持 `[#, 起点]` 和 `[#，起点，步长]` 格式

### Bug 4：删除页眉页脚失败但继续处理（最关键）
- **现象**：标记删除后提示「没有找到可安全删除的匹配内容」，新页眉叠加在旧页眉上
- **根因**（三方审计共识）：
  1. **CID 字体编码**（核心根因）：pdftotext 通过 ToUnicode CMap 解码中文，lopdf 的 `object_text` 只处理 UTF-16BE/UTF-8，CID 字体解码出乱码
  2. **Zone check 作为 gate**：`is_in_header_zone` 是前置条件，不通过则 `matches_any_target`（含 bbox 匹配）不执行
  3. **Form XObject 坐标空间**：`filter_referenced_form_text` 传入页面级 `plan`，zone check 用页面坐标检查表单局部坐标
  4. **bbox.height 与 page_box 可能不一致**：pdftotext 页面高度与 lopdf 的 CropBox/MediaBox 可能不同
- **修复**：见下方「三、修复方案」

### Bug 5：新旧 API 双轨制
- **问题**：`buildHeaderText`（旧）与 `buildHeaderTextForGroup`（新）并存，显示走旧 API、渲染走新 API
- **修复**：统一到 group-based API，废弃旧 API

---

## 二、功能需求

### 需求 1：检测与删除统一
- **短期**：bbox 匹配独立于 zone check + 坐标统一
- **长期**：在 lopdf 层增加 ToUnicode CMap 查找

### 需求 2：按证据列表名称 UI 重构
- 前缀：下拉（证据/对比文件/附件/自定义）
- 序号格式：数字/数字01/数字001/中文数字/大写中文
- 起始编号：默认 1
- 编号步长：默认 1

### 需求 3：占位符增强
- `[#, 6]` → 从 6 开始
- `[#，6，2]` → 从 6 开始，步长 2

### 需求 4：序列格式选项
- 与模板模块对齐，支持多种页码格式

### 需求 5：奇偶页支持
- 首页不同 + 奇偶页不同

### 需求 6：分段→显示范围
- 标签改名

### 需求 7：大文件检测优化
- pdftotext 加 `-f` `-l` 参数
- 检测并行化（`Promise.all` + 并发限制）
- 超过阈值时提示用户

### 需求 8：合并完成醒目提示
- 用 `ElNotification` 替代 `ElMessage.success`

---

## 三、修复方案（审计共识）

### Phase 1：急救（1 周）

#### 1.1 bbox 匹配独立于 zone check + 坐标统一（P0）
**文件**：`content_text.rs`

**改动 1**：`filter_page_operations` 中 bbox 匹配独立于 zone check
```rust
if let Some(text) = shown_text.as_deref() {
    // 文本匹配：必须在 zone 内
    if is_in_header_zone(state.y, plan)
        && matches_any_target_text(text, &plan.header_targets)
    {
        remove_region = Some(TextRegion::Header);
    } else if is_in_footer_zone(state.y, plan)
        && matches_any_target_text(text, &plan.footer_targets)
    {
        remove_region = Some(TextRegion::Footer);
    }
}
// bbox 匹配：独立于 zone check
if remove_region.is_none() {
    if target_bbox_matches_any(&state, &plan.header_targets) {
        remove_region = Some(TextRegion::Header);
    } else if target_bbox_matches_any(&state, &plan.footer_targets) {
        remove_region = Some(TextRegion::Footer);
    }
}
```

**改动 2**：确保 bbox 坐标使用 lopdf 的页面尺寸
- 在构建 `PlainTextTarget` 时，用 lopdf 的 `page_box.max_y` 替代 pdftotext 的 `bbox.height`

#### 1.2 Bug B 修复（P0）
**文件**：`useEvidencePdfSession.js`

`buildHeaderText` 中 `headerEdited` 仅在 `per_file` 模式生效：
```js
export function buildHeaderText(file, index, rules) {
  if (rules.headerMode === 'per_file' && file?.headerEdited) {
    return decorateHeaderText(file.header ?? '', file, index, rules)
  }
  if (rules.headerMode === 'none') return ''
  const base = headerBaseText(file, index, rules)
  return decorateHeaderText(base, file, index, rules)
}
```

#### 1.3 删除结果详细报告（P1）
**文件**：`content_text.rs`

`PlainTextCleanupResult` 增加诊断字段：
```rust
pub struct PlainTextCleanupResult {
    pub removed_header: usize,
    pub removed_footer: usize,
    pub diagnostics: Vec<DeleteDiagnostic>,
}

pub struct DeleteDiagnostic {
    pub page: u32,
    pub target_text: String,
    pub reason: DeleteSkipReason,  // TextNotMatched / BboxOutOfZone / FontUndecodable
    pub extracted_text: Option<String>,
    pub state_y: f32,
    pub in_zone: bool,
}
```

### Phase 2：稳固（2-3 周）

#### 2.1 Form XObject 坐标变换（P1）
**文件**：`content_text.rs`

`filter_referenced_form_text` 中读取表单的 BBox 和 Matrix，构建调整后的 plan：
```rust
let form_bbox = stream_dict.get(b"BBox")...;
let form_matrix = stream_dict.get(b"Matrix")...;
// 为表单构建调整后的 page_box
let form_plan = PagePlainTextPlan {
    page_box: PageBox {
        width: fx1 - fx0,
        min_y: fy0,
        max_y: fy1,
    },
    ..plan.clone()
};
filter_page_operations(&content.operations, &form_plan)
```

#### 2.2 Bug C 修复（P1）
**文件**：`useEvidencePdfSession.js`

`headerBaseTextForGroup` 中 `per_file` fallback 改为 `证据${index + 1}`

#### 2.3 文本拼接匹配（P1）
**文件**：`content_text.rs`

在 `filter_page_operations` 中维护 BT/ET 块内的文本缓冲区，支持 split Tj 匹配

#### 2.4 新旧 API 收敛（P1）
**文件**：`useEvidencePdfSession.js`、`useEvidencePdfExistingEditing.js`

- 废弃 `buildHeaderText`，`displayRowHeader` 改用 `buildHeaderTextForGroup`
- 废弃 `headerBaseText`，统一到 `headerBaseTextForGroup`
- 清理 `file.header` / `file.headerEdited` 旧字段

#### 2.5 检测并行化（P1）
**文件**：`useEvidencePdfDetection.js`

```js
const CONCURRENCY = 4
const results = await pMap(overlayRows.value, detectFileHeaderFooter, { concurrency: CONCURRENCY })
```

#### 2.6 原子写入 + 失败回滚（P1）
**文件**：`content_text.rs`、`header_footer.rs`

先写临时文件，成功后 rename：
```rust
let temp = temp_named_path("docsy_atomic", "pdf")?;
doc.save(&temp)?;
std::fs::rename(&temp, output_path)?;
```

### Phase 3：重构（1-2 月）

#### 3.1 ToUnicode CMap 查找（P0 长期）
**文件**：`content_text.rs`（新增 `text_normalize.rs`）

从字体字典读取 ToUnicode CMap，建立 GID → Unicode 映射：
```rust
fn build_to_unicode_map(doc: &Document, font_id: ObjectId) -> Option<HashMap<u16, String>> {
    let font = doc.get_object(font_id).ok()?.as_dict().ok()?;
    let cmap_stream = font.get(b"ToUnicode").ok()?;
    let cmap_ref = cmap_stream.as_reference().ok()?;
    let stream = doc.get_object(cmap_ref).ok()?.as_stream().ok()?;
    let content = stream.get_plain_content().ok()?;
    parse_cmap(&content)
}
```

#### 3.2 normalize 函数统一（P2）
抽取到公共模块 `text_normalize.rs`

#### 3.3 TextState 跟踪补全（P2）
实现 `update_text_state_after_show`，跟踪 TJ kerning 和隐式位移

#### 3.4 pdftotext 页范围优化（P2）
`run_pdftotext_bbox` 加 `-f` `-l` 参数

#### 3.5 file 对象拆分（P2）
拆为四层：`meta`、`detection`、`existingEdit`、`rules`

#### 3.6 header_footer.rs 拆分（P2）
拆为：`font.rs`、`overlay.rs`、`bookmark.rs`、`job.rs`

#### 3.7 测试补全
- CID 字体 + ToUnicode 删除测试
- 坐标系翻转单元测试
- `buildHeaderTextForGroup` 各 mode 快照测试
- `expandSplitNameTokens` 边界测试

### Phase 4：打磨（持续）

#### 4.1 检测结果缓存（P3）
文件 hash/mtime+size 作为 key

#### 4.2 overlay 批量生成（P3）
多文件场景批量构建 overlay PDF

#### 4.3 检测进度细化（P3）
每个文件内分阶段反馈

---

## 四、UI 改进建议

### 4.1 页眉来源卡片化
将「页眉来源」改为卡片选择：
- 不插入（开关）
- 文件名
- 证据序号（含前缀/序号格式/起始/步长专用设置区）
- 固定文本

### 4.2 Tab 分层（参考 WPS）
- 内容 Tab：来源、文本、前缀后缀、显示范围
- 外观 Tab：字体、字号、颜色、边距、对齐
- 高级 Tab：奇偶页、首页不同、模板保存

### 4.3 其他
- 「分段」改为「显示范围」
- 模板标记帮助改为输入框下方常驻快捷按钮
- 页码预览显示多页（第 1 页、中间页、最后页）

---

## 五、额外发现

### 5.1 overlayConfigForFile 的 region 判断是死代码
`useEvidencePdfSession.js:733` 中 `region === 'pageNumber'` 永远不成立（页码通过 `pageNumberOverlaysForFile` 单独构建）

### 5.2 assignPageRanges vs updatePageRanges 不可变/可变混用
`buildHeaderFooterItems` 调用 `assignPageRanges` 产生新数组，但后续逻辑混用两个版本

### 5.3 TJ 操作内部位置不跟踪
`TextState` 不跟踪 TJ 内部的字符位置移动，影响 bbox 匹配精度

---

## 六、文件清单

| 文件 | 行数 | 涉及修改 |
|------|------|---------|
| `src-tauri/src/pdf/content_text.rs` | 991 | Bug4 修复、Form 坐标变换、TextState、normalize |
| `src-tauri/src/pdf/header_footer.rs` | 2369 | 诊断报告、原子写入 |
| `src-tauri/src/pdf/artifacts.rs` | 1390 | （参考） |
| `src-tauri/src/pdf/detection.rs` | 2645 | pdftotext 页范围、normalize 统一 |
| `src/modules/pdf-tools/composables/useEvidencePdfSession.js` | 1042 | Bug B/C、API 收敛 |
| `src/modules/pdf-tools/composables/useEvidencePdfDetection.js` | 502 | 检测并行化 |
| `src/modules/pdf-tools/composables/splitFileName.js` | 131 | Bug3、占位符增强 |
| `src/modules/pdf-tools/composables/pdfPageNumberRules.js` | 141 | （参考） |
| `src/modules/pdf-tools/composables/useEvidencePdfExistingEditing.js` | 273 | API 收敛 |
| `src/modules/pdf-tools/components/HeaderFooterRuleFields.vue` | 877 | UI 重构 |

---

## 七、审计文档索引

| 文档 | 来源 | 路径 |
|------|------|------|
| Claude 审计报告 | Claude | `MDG-016-claude-brief.md` |
| GLM 审计报告 | GLM 5.2 | `MDG-016-audit-report.md` |
| 交叉审计 | Claude 审 GLM | `MDG-016-cross-audit-review.md` |
| 优化建议 | GLM 5.2 | `MDG-016-optimization-suggestions.md` |
| 本变更单 | 合并 | `MDG-016-evidence-header-footer.md` |
