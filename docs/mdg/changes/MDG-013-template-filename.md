# MDG-013: 模板驱动文件名生成

**优先级**: P1
**状态**: 🟡 进行中
**创建日期**: 2026-08-07
**分支**: codex/template-quickxml-0.8

---

## 设计方案

### 数据结构

manifest.json 新增 `filenameTemplate` 字段：

```json
{
  "filenameTemplate": {
    "tokens": [
      { "type": "field", "value": "无效案号" },
      { "type": "literal", "value": "-" },
      { "type": "field", "value": "代理人" },
      { "type": "literal", "value": "-" },
      { "type": "preset", "value": "日期" }
    ],
    "separator": "-"
  }
}
```

渲染时展开：`4W122723-吕晗-20260807.docx`

单文件默认：`模板名-日期`
批量默认：`模板名-序号-日期`

### Token 类型

| 类型 | 值 | 渲染 | 来源 |
|------|-----|------|------|
| field | 字段名 | 字段值 | 模板字段列表 |
| preset | 序号 | 1, 2, 3... | 批量时自增 |
| preset | 中文序号 | 一, 二, 三... | 批量时自增 |
| preset | 日期 | 20260807 | 当天日期 |
| literal | 任意文本 | 原样 | 用户输入 |
| literal | 分隔符 (-, _, 空格) | 原样 | 用户选择 |

### 可复用组件

**核心逻辑**：复用 `src/modules/pdf-tools/composables/splitFileName.js`
- `formatSplitFileName()` — 拼接
- `expandSplitNameTokens()` — token 展开
- `todayCompact()` — 日期

**UI 组件**：新建 `FilenameTokenInput.vue`（shared），token 气泡可拖拽排列

### UI 位置

模板制作页（TemplateBuildTab）：在"确认字段"标题下方加一个折叠区域"文件名规则"

模板填写页（TemplateRenderTab）：在"生成 Word"按钮旁显示预览文件名

### 文件名非法字符

自动替换 `/\:*?"<>|` 为 `_`

### 冲突处理

同名文件自动加后缀 `-1`, `-2` 等（复用 MDG-006 的路径碰撞检测）

---

## 实施

| Phase | 内容 | 状态 |
|-------|------|------|
| 1 | FilenameTokenInput 组件 + splitFileName.js | ✅ |
| 2 | TemplateBuildTab 集成 + tooltip | ✅ |
| 3 | TemplateView 状态管理 + 默认 token | ✅ |
| 4 | 序号/日期格式选项、布局调整、长度校验 | ✅ |
| 4b | 3栏布局、预览色块、模板名按钮 | ✅ |
| 5 | 模板填写页集成 + manifest 持久化 | ✅ |
| 6 | batch.rs 使用 filenameTemplate 生成文件名 | ✅ |
