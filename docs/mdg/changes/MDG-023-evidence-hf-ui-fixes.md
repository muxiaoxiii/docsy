# MDG-023: 证据处理页眉页脚 UI 全面修复

## 状态：🟢 已完成
## 优先级：P0
## 关联：用户反馈 2026-08-08

---

## 问题清单

| # | 问题 | 严重程度 | 状态 |
|---|------|----------|------|
| 1 | 标记删除后执行删除，页眉未被删除 | P0 | 待修 |
| 2 | 显示范围功能重新设计 | P0 | 待改 |
| 3 | 固定文本模式失效 | P0 | 待修 |
| 4 | 按证据列表名称模式失效 | P0 | 待修 |
| 5 | 恢复删除标记按钮来历不明 | P1 | 待删 |
| 6 | quickCleanup 使用硬编码 zone 高度 | P1 | 待修 |
| 7 | 一键确认跳过低置信度 | P1 | 待改 |
| 8 | showTotal 不同步模板 | P1 | 待修 |
| 9 | 页码列表显示不正确 | P1 | 待修 |
| 10 | 按钮重组 | P2 | 待改 |
| 11 | 页眉来源选项 | P2 | 待改 |
| 12 | 导入流程优化 | P2 | 待改 |

---

## 问题 1：标记删除后执行删除，页眉未被删除（P0）

### 根因

`EvidencePdfWorkbench.vue:1418-1421`：
```javascript
const autoCleanupHeaderEnabled = computed(() =>
  overlayFiles.value.some(
    (file) => file.existingHeaderArtifact && file.existingHeaderEdited && !file.removeExistingHeader,
  ),
)
```

条件要求 `existingHeaderArtifact` 为 `true`，但纯文本页眉的 `existingHeaderArtifact` 是 `false`，导致 cleanup 从未启用。

### 用户说明

用户确认后，不管是不是 artifact 都可以删，只要位置匹配正确。

### 修复方案

```javascript
const autoCleanupHeaderEnabled = computed(() =>
  overlayFiles.value.some((file) => file.removeExistingHeader),
)
const autoCleanupFooterEnabled = computed(() =>
  overlayFiles.value.some((file) => file.removeExistingFooter || file.removeExistingPageNumber),
)
```

---

## 问题 2：显示范围功能重新设计（P0）

### 根因

父组件 `EvidencePdfWorkbench.vue` 缺失 `v-model:header-page-start` 等 4 个绑定。

### 设计方案

```
显示范围: [✓] 全部页面
         取消勾选后显示:
         起始页 [1] – 结束页 [  ]
         （结束页留空=全部页面；结束页不能超过本文件页数）
```

- 加一个 checkbox "全部页面"，默认勾选
- 取消勾选后显示起始页和结束页输入框
- 结束页留空 = 全部页面（对应 pageEnd=0）
- 结束页上限 = 当前文件页数（通过 file.pages 限制 max）
- 保持后端兼容（pageEnd=0 仍然是全部）

### 修复方案

1. **EvidencePdfWorkbench.vue**：添加 computed 属性 + v-model 绑定
2. **HeaderFooterRuleFields.vue**：加 checkbox，结束页 max 绑定文件页数

---

## 问题 3：固定文本模式失效（P0）

### 根因

`buildFileContentRows`（useEvidencePdfSession.js:255-256）传原始 `selectedGroup` 给 `contentRowText`，未合并 `rules.headerMode`。渲染路径正确（`buildHeaderText` 合并了），列表路径错误。

### 用户说明

应该包含完整的输入匹配模式。

### 修复方案

```javascript
// 修改前
const text = contentRowText(file, index, kind, selectedGroup, rules)

// 修改后
let effectiveGroup = selectedGroup
if (kind === 'header' && rules.headerMode !== undefined) {
  effectiveGroup = { ...selectedGroup, mode: rules.headerMode }
} else if (kind === 'pageNumber' && rules.pageNumberSequence) {
  effectiveGroup = { ...selectedGroup, sequence: rules.pageNumberSequence }
}
const text = contentRowText(file, index, kind, effectiveGroup, rules)
```

同样修复 line 280 的 extra groups。

---

## 问题 4：按证据列表名称模式失效（P0）

### 根因

同问题 3。

### 用户说明

选中时出现额外设置项（"证据"文本输入框、序号类型选择）。

### 修复方案

在 `HeaderFooterRuleFields.vue` 中，当 `headerMode === 'per_file'` 时显示额外设置项：
- "证据"文本输入框（默认"证据"，可改为"对比文件"等）
- 序号类型选择（数字/中文）

---

## 问题 5：恢复删除标记按钮来历不明（P1）

### 用户说明

用户不理解这个功能从哪来的。这个按钮应该删除，不在新的 5 个按钮中。

### 修复方案

删除"恢复删除标记"按钮（EvidencePdfWorkbench.vue:93-95）及其对应的 `restoreExistingHeaderFooterMarks` 函数。

---

## 问题 6：quickCleanup 使用硬编码 zone 高度（P1）

### 根因

`EvidencePdfWorkbench.vue:2305-2318`：
```javascript
cleanupHeaderHeightMm: 18,   // ← 硬编码
cleanupFooterHeightMm: 18,   // ← 硬编码
```

### 修复方案

改为使用 `cleanupHeaderHeightMm.value` 和 `cleanupFooterHeightMm.value`。

---

## 问题 7：一键确认跳过低置信度（P1）

### 用户说明

拆成一键保留和一键忽略，对所有检测到的成分生效（包括低置信度）。

### 修复方案

- `keepAllExistingElements`：将所有未确认项标记为 'keep'（不过滤 lowConfidence）
- `ignoreAllExistingElements`：将所有未确认项标记为 'ignore'（不过滤 lowConfidence）

---

## 问题 8：showTotal 不同步模板（P1）

### 用户说明

页码编辑框里的 total 没有跟随总页码开关，还显示，但实际上已经失效了。

### 修复方案

在 `HeaderFooterRuleFields.vue` 中添加 watcher：
```javascript
let lastTemplateWithTotal = ''
watch(() => props.pageNumberShowTotal, (showTotal) => {
  const tpl = props.pageNumberTemplate || '{page}/{total}'
  if (!showTotal && tpl.includes('{total}')) {
    lastTemplateWithTotal = tpl
    const cleaned = tpl
      .replaceAll('{total}', '')
      .replaceAll('//', '/')
      .replace(/\/+$/, '')
      .replace(/^\//, '')
    pageNumberTemplateModel.value = cleaned || '{page}'
  } else if (showTotal && !tpl.includes('{total}')) {
    pageNumberTemplateModel.value = lastTemplateWithTotal || '{page}/{total}'
  }
})
```

---

## 问题 9：页码列表显示不正确（P1）

### 用户说明

选择"每个文件单独编号"时，文件列表中显示的页码还是连续编页的那个数字。应该改成：括号外面写真实页码，括号里写总页数。例如：`1 (35)`、`1 (35)`、`1 (35)`（每个文件都是第 1 页，总页数 35）。

这是只有单个文件分别编页码的时候才生效的设定。

### 修复方案

修改 `pageRangeText` 或相关函数，在 per-file 模式下显示格式改为 `页码（总页数）`。

---

## 问题 10：按钮重组（P2）

### 用户说明

**5 个按钮**：
1. **一键保留**：将所有未确认项标记为 'keep'（包括低置信度）— 从一键确认拆出来
2. **一键忽略**：将所有未确认项标记为 'ignore'（包括低置信度）— 从一键确认拆出来
3. **标记删除**：将所有确认项标记为 'delete'，等后面处理文件时一起处理 — 原"删除现有"
4. **立即删除**：弹窗确认后立即执行删除 — 原"一键清除页眉页脚"
5. **深度检测**：手动触发完整检测

**删除的按钮**：
- 一键确认 → 拆成一键保留/一键忽略
- 恢复删除标记 → 删除

**立即删除的弹窗流程**：
1. 如果用户没有进行过页眉页脚页码的确认，按钮不生效
2. 如果没有确认过，弹出确认窗口让用户先确认
3. 如果已经确认了，弹窗请用户选择：标记删除 / 立即删除 / 取消
4. 立即删除：删除确认后的页眉页脚页码
5. 标记删除：在后面处理文件时一起处理

---

## 问题 11：页眉来源选项（P2）

### 用户说明

只有文件名/固定文本/按证据列表名称三项。目前问题是三个选项只有一个有效。这三个选项暂时不要增加，与问题 3 和 4 相关。

### 修复方案

精简选项为三个，并确保三个选项都能正常工作。删除"不插入页眉"选项。

---

## 问题 12：导入流程优化（P2）

### 用户说明

**深度检测**其实就是正常的检测，相比快速检测，多了针对页眉页脚位置的、不在 artifacts 流里面的文本。这个操作会慢一点，如果导入 1000 页或几十个文件，会拖慢速度。

因此把导入之后的流程拆分了：
- **快速检测**：只做标准页眉检测（artifacts）和页数检测，这两项放到一起
- **深度检测**：手动触发，包含非 artifact 文本检测

**合并展示进度**：每个文件的检测都会触发 doclet 动画，需要合并展示进度。

**首页特殊匹配**：对 PDF 文件首页做特殊匹配，如果页眉或页脚位置有"证据X"或"对比文件X"的序列，不需要重复（两页及以上）也作为候选页眉页脚给用户确认。

### 修复方案

1. 快速检测只做 artifacts 检测 + 页数检测，合并进度展示
2. 深度检测手动触发，包含文本检测
3. 首页特殊规则：证据X/对比文件X 序列直接作为候选

---

## 测试计划

- [ ] 标记删除 → 处理 → 检查 _cleaned 文件夹页眉是否被删除
- [ ] 固定文本 → 输入自定义文本 → 列表和渲染都显示自定义文本
- [ ] 按证据列表名称 → 列表显示"证据1, 证据2..."
- [ ] 显示总页数开关关掉后，格式框更新
- [ ] 一键保留/一键忽略功能正确
- [ ] 立即删除弹窗流程正确
- [ ] 页码 per-file 模式显示格式正确
- [ ] 首页特殊匹配证据X/对比文件X

## 变更日志

- 2026-08-08 23:35 — 根据用户详细反馈重写变更单
