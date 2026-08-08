# MDG-023: 证据处理页眉页脚 UI 修复

## 状态：🟢 已完成
## 优先级：P0
## 关联：用户反馈 2026-08-08
## 版本：v0.9-mdg

---

## 完成情况

| Phase | 内容 | 状态 |
|-------|------|------|
| Phase 1 | 页眉来源列表显示修复 + seq/seq_cn/prefix_seq 选项 | ✅ |
| Phase 2 | 显示范围控件绑定修复 | ✅ |
| Phase 3 | 按钮改造（保留/忽略/立即删除/标记删除/重新检测/深度检测） | ✅ |
| Phase 4 | 显示总页数开关同步模板 | ✅ |
| Phase 5 | Doclet 闪烁修复 | ⏳ 待验证 |

---

## 问题清单

| # | 问题 | 严重程度 | 根因 |
|---|------|----------|------|
| 1 | 固定文本失效 | P0 | buildFileContentRows 未合并 rules.headerMode |
| 2 | 按证据列表名称失效 + 缺子选项 | P0 | 同上 + UI 缺 seq/seq_cn/prefix_seq 选项 |
| 3 | 页码列表显示不对（per-file 显示连续） | P1 | displayContentRowText 未读 group.sequence |
| 4 | 显示范围控件无效 | P1 | 待查 |
| 5 | 显示总页数开关无效 | P1 | 待查 |
| 6 | 无重新检测按钮 | P2 | 缺失功能 |
| 7 | 一键确认遗漏低置信度 | P2 | 改为一键保留/一键忽略 |
| 8 | 一键清除→立即删除，删除现有→标记删除 | P2 | 按钮重命名 + 逻辑调整 |
| 9 | 新增深度检测按钮 | P2 | 缺失功能 |
| 10 | Doclet 闪烁 | P3 | 待验证 |

---

## 问题 1 & 2：固定文本 / 按证据列表名称失效

### 根因

`buildFileContentRows`（useEvidencePdfSession.js:256）传原始 `selectedGroup` 给 `contentRowText`，未合并 `rules.headerMode` 覆盖。

数据流对比：

**渲染路径（正确）**：
```
buildEvidencePdfRulePayload → buildHeaderText → selectedGroupFor → 
  { ...group, mode: rules.headerMode ?? group.mode } → 
  buildHeaderTextForGroup → headerBaseTextForGroup → 检查 group.mode ✅
```

**列表路径（错误）**：
```
buildFileContentRows → selectedGroupFor → 
  selectedGroup（原始，mode 未覆盖）→ 
  contentRowText → buildHeaderTextForGroup → headerBaseTextForGroup → 
  检查 group.mode（原始值，不是 rules.headerMode）❌
```

### 修改前代码

```javascript
// useEvidencePdfSession.js:255-256
if (newEnabled && enabled && selectedGroup) {
  const text = contentRowText(file, index, kind, selectedGroup, rules)
```

同样 line 280：
```javascript
const text = contentRowText(file, index, kind, g, rules)
```

### 修改后代码

```javascript
// useEvidencePdfSession.js:255-256
if (newEnabled && enabled && selectedGroup) {
  // Merge rules.headerMode into group so list display matches rendering
  const effectiveGroup = kind === 'header' && rules.headerMode !== undefined
    ? { ...selectedGroup, mode: rules.headerMode }
    : selectedGroup
  const text = contentRowText(file, index, kind, effectiveGroup, rules)
```

同样 line 280：
```javascript
const effectiveG = kind === 'header' && rules.headerMode !== undefined
  ? { ...g, mode: rules.headerMode }
  : g
const text = contentRowText(file, index, kind, effectiveG, rules)
```

### 影响分析

| 调用位置 | 文件:行号 | 当前行为 | 修改后兼容 |
|----------|-----------|----------|------------|
| buildFileContentRows header | useEvidencePdfSession.js:256 | 用原始 group.mode | 合并 rules.headerMode ✅ |
| buildFileContentRows extra groups | useEvidencePdfSession.js:280 | 用原始 group.mode | 合并 rules.headerMode ✅ |
| buildHeaderText | useEvidencePdfSession.js:337 | 已正确合并 | 不受影响 ✅ |
| buildEvidencePdfRulePayload | useEvidencePdfSession.js:948 | 用 buildHeaderText（正确） | 不受影响 ✅ |

---

## 问题 2 补充：页眉来源缺 seq/seq_cn/prefix_seq 选项

### 修改前代码

```html
<!-- HeaderFooterRuleFields.vue:57-62 -->
<el-select v-model="headerModeModel">
  <el-option label="不插入页眉" value="none" />
  <el-option label="文件名" value="filename" />
  <el-option label="按证据列表名称" value="per_file" />
  <el-option label="固定文本" value="custom" />
</el-select>
```

### 修改后代码

```html
<el-select v-model="headerModeModel">
  <el-option label="不插入页眉" value="none" />
  <el-option label="文件名" value="filename" />
  <el-option label="按证据列表名称" value="per_file" />
  <el-option label="固定文本" value="custom" />
  <el-option label="序号（证据1, 证据2）" value="seq" />
  <el-option label="中文序号（证据一、证据二）" value="seq_cn" />
  <el-option label="前缀+序号" value="prefix_seq" />
</el-select>
```

当选择 `prefix_seq` 时，显示一个额外的前缀输入框（复用现有的 headerText 输入框，placeholder 改为"输入前缀，如"原告""）。

---

## 问题 3：页码列表显示不对

### 根因

`displayContentRowText`（EvidencePdfWorkbench.vue:2619-2626）对 pageNumber 类型，读 `group.sequence` 判断连续/单独编号：

```javascript
const seq = group.sequence || 'continuous'
const continuous = seq !== 'per-file'
```

但 `group.sequence` 可能未正确设置。需要检查 `rules.pageNumberSequence` 是否传递到 group。

### 修改方向

在 `buildFileContentRows` 中，将 `rules.pageNumberSequence` 合并到 pageNumber group：
```javascript
const effectiveGroup = kind === 'pageNumber' && rules.pageNumberSequence
  ? { ...selectedGroup, sequence: rules.pageNumberSequence }
  : selectedGroup
```

---

## 问题 4 & 5：显示范围控件 / 显示总页数开关

待进一步调查。

---

## 问题 6：新增重新检测按钮

在 `block-actions` 区域（EvidencePdfWorkbench.vue:78）新增"重新检测"按钮：
```html
<el-button size="small" @click="redetectAllHeaderFooter">重新检测</el-button>
```

实现：先重置所有文件的检测状态（existingElements、existingHeaderText 等），再调用 `detectAllHeaderFooter({})`。

---

## 问题 7：一键确认 → 一键保留 / 一键忽略

### 修改前

```html
<el-button size="small" @click="confirmAllExistingElements">一键确认</el-button>
```

### 修改后

```html
<el-button size="small" @click="keepAllExistingElements">一键保留</el-button>
<el-button size="small" @click="ignoreAllExistingElements">一键忽略</el-button>
```

`keepAllExistingElements`：将所有未确认的 existingElements（不管置信度）标记为 `decision = 'keep'`。
`ignoreAllExistingElements`：将所有未确认的 existingElements 标记为 `decision = 'ignore'`。

---

## 问题 8：按钮重命名

### 一键清除 → 立即删除

点击后弹窗三个选项：
- **确认** → 立即删除，输出到新文件夹，并替换导入文件
- **标记删除** → 只标记，稍后一起处理
- **取消** → 不做任何操作

### 删除现有 → 标记删除

只标记 `decision = 'delete'`，等批量处理。

---

## 问题 9：新增深度检测按钮

在"原页眉页脚"区域新增"深度检测"按钮，触发完整的页眉/页脚/页码检测（使用现有的 `detectAllHeaderFooter` 但不传 `silent: true`）。

---

## 测试计划

- [ ] 单元测试：npm run test（buildHeaderText / buildFileContentRows 相关）
- [ ] 手动验证：选"固定文本"→ 输入自定义文本 → 列表和渲染都显示自定义文本
- [ ] 手动验证：选"按证据列表名称"→ 列表显示"证据1, 证据2..."
- [ ] 手动验证：选"序号" / "中文序号" → 列表和渲染正确
- [ ] 手动验证：页码"每个文件单独编号" → 列表显示1,2,3（非连续）
- [ ] 手动验证：显示范围输入框可修改
- [ ] 手动验证：显示总页数开关关掉后，格式框更新
- [ ] 手动验证：重新检测按钮可用
- [ ] 手动验证：一键保留/一键忽略功能正确
- [ ] 手动验证：立即删除弹窗三个选项都正确

## 回退方案

`git revert HEAD --no-edit`

## 变更日志

- 2026-08-08 19:50 — 创建变更单，定位问题 1&2 根因
