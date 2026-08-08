# MDG-014: 统一文档预览模块 + 模板编辑 UI 优化

**优先级**: P1
**状态**: ✅ 已完成
**创建日期**: 2026-08-07
**分支**: codex/template-quickxml-0.8

---

## 问题描述

### 补选提取不到文本

模板制作页的预览区域，用户选中文本后点击底部图例按钮（文本/日期/列表等）添加字段，有时提取不到选择的文本。

**原因分析**：底部图例按钮原来只是图例（纯展示），后来顺便加了"从选区添加字段"功能。但选择逻辑依赖 `data-run-id` 属性，而渲染预览的 `<button>` 元素没有这个属性。用户可能在渲染预览（右侧）选中文本，但 `resolvePreviewSelection` 只能解析源预览（左侧）的选区。

### 图例按钮冗余

底部的类型图例（文本/日期/列表/引用/勾选/前缀/后缀/保留原文）本来是展示用，后来加了点击添加功能。但：
- 有些类型不需要从选区添加（如前缀/后缀/保留原文）
- 操作入口分散（图例、表格内下拉、顶部工具栏）
- 视觉上占空间但使用频率低

### 预览模块缺乏通用性

当前预览逻辑分散在：
- `TemplateBuildTab.vue`：模板制作预览（segment 渲染）
- `TemplateRenderTab.vue`：填写预览（纯文本 `<pre>`）
- `usePreviewSelection.js`：选区解析

三处各自实现，没有统一接口。

---

## 设计方案：统一文档预览模块

### 核心思路

**一次解析，三种渲染模式**。一个通用的 `DocumentPreview` 组件，接受 runs + overlay 数据，输出 HTML 渲染。

```
Word 文件 → 解析为 Run[] (带格式) → DocumentPreview 组件
  mode=original:  纯渲染，保留粗体/斜体/下划线/字体
  mode=fields:    原文 + 字段位置用 [字段名] 高亮覆盖
  mode=fill:      原文 + 字段值替换高亮覆盖
```

### 数据结构

```typescript
interface PreviewRun {
  id: string              // run ID (对应 Word 的 w:r)
  text: string            // 文本内容
  paragraphIndex: number  // 段落索引
  bold?: boolean
  italic?: boolean
  underline?: boolean
  fontHint?: string       // eastAsia / ascii
}

interface PreviewOverlay {
  runId: string           // 对应 PreviewRun.id
  start: number           // 字符起始位置
  end: number             // 字符结束位置
  label: string           // 显示文本（字段名 或 填写值）
  type?: string           // 字段类型（用于颜色区分）
  filled?: boolean        // 是否已填写（用于填写模式）
  clickable?: boolean     // 是否可点击
}
```

### 组件接口

```vue
<DocumentPreview
  :runs="documentRuns"
  :overlays="fieldOverlays"
  mode="fields"           // original | fields | fill
  :formatting="true"      // 是否保留格式
  @select="onTextSelect"
  @click-overlay="onOverlayClick"
/>
```

### 三种模式

#### 模式1：原文预览（mode=original）

```html
<!-- 纯渲染，每个 run 一个 span -->
<span style="font-weight:bold">代理师</span>
<span style="text-decoration:underline">     吕晗</span>
代理权限为：
<span>一般代理</span>
```

- 保留粗体/斜体/下划线
- 不显示字段标记
- 可用于查看原始 Word 内容

#### 模式2：字段标注（mode=fields）

```html
<!-- 原文 + 字段位置用高亮覆盖 -->
代理师 <span class="overlay field-text">[代理人]</span>
代理权限为：
<span class="overlay field-checkbox">[一般代理]</span>
<span class="overlay field-checkbox">[特殊代理]</span>
```

- 未标记区域显示原文
- 字段位置用 `[字段名]` 替换，背景色区分类型
- 点击字段 → 聚焦到表格对应行
- 选中原文 → 弹出"添加为字段"菜单

#### 模式3：填写预览（mode=fill）

```html
<!-- 原文 + 字段值替换 -->
代理师 <span class="overlay filled">吕晗</span>
代理权限为：
<span class="overlay filled">☑一般代理</span>
<span class="overlay unfilled">[特殊代理]</span>
```

- 已填字段：值替换原文，绿色背景
- 未填字段：`[字段名]` 灰色背景
- 勾选框：☑/☐ 替换
- 实时更新（formValues 变化时重渲染）

### 选区修复

统一预览组件内置选区解析：

```typescript
function resolveSelection(range: Range): SelectionResult {
  // 遍历所有 overlay span，找到与选区相交的
  // 从 overlay 的 runId + start/end 反推原始位置
  // 返回 { text, refs, context }
}
```

所有 overlay span 都带 `data-run-id` 和 `data-start` 属性，选区解析统一处理。

### 底部图例精简

移除底部图例的 8 个类型按钮，改为：

**一个"添加为字段"按钮**：
- 默认类型：文本
- 自动推断：`is_checkbox_text(text)` → 勾选，`looksLikeDateText(text)` → 日期
- 推断依据复用已有的 `inferTemplateField` 逻辑
- 用户添加后可在表格中修改类型

**图例只保留纯展示**（颜色说明），不带交互功能。

---

## 通用性

`DocumentPreview` 组件不依赖模板模块的数据结构，只接受 `runs[]` + `overlays[]`。

其他可复用的场景：
- **证据 PDF 预览**：runs = PDF 文本块，overlays = 标注/高亮
- **文书对比**：两组 runs 并排，overlays = 差异标记
- **模板库预览**：只读模式，不需要选区功能

---

## 实施顺序

| Phase | 内容 | 改动范围 | 复杂度 |
|-------|------|----------|--------|
| 1 | 提取 DocumentPreview 组件 | 新组件 + TemplateBuildTab 重构 | 🟡 中 |
| 2 | 填写预览接入 DocumentPreview | TemplateRenderTab | 🟢 低 |
| 3 | 选区解析统一 | usePreviewSelection 重构 | 🟡 中 |
| 4 | 底部图例精简 | TemplateBuildTab | 🟢 低 |

Phase 1 是核心，完成后 Phase 2-4 自然跟进。

---

## 验证方法

1. 导入一个有粗体/斜体/下划线的 Word 文件
2. 原文预览：格式保留正确
3. 字段标注模式：字段位置高亮，点击聚焦到表格
4. 填写模式：填入值后实时更新，已填/未填颜色区分
5. 选区：在任意预览区域选中文本 → 点击"添加为字段" → 正确提取
6. 通用性：在非模板模块中也能使用 DocumentPreview

## 参考

- Google Docs 内联评论（overlay 在文档上）
- VS Code diff 视图（并排 + overlay）
- Notion 预览块（inline formatting）
