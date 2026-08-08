# MDG-016: 证据处理模块页眉页脚全面优化 — 详细设计文档

## 状态：✅ 已完成
## 优先级：P0
## 创建日期：2026-08-08

---

## 一、问题清单与根因分析

### Bug 1：数据不一致 — 文件列表与实际渲染不匹配

**现象**：固定文本 `证据[#]` 渲染正确，但文件列表显示文件名。

**根因追踪**：
```
渲染路径: buildHeaderTextForGroup(file, index, group, rules)
  → 读 group.mode → 正确

文件列表路径: displayRowHeader() → rowHeaderPreview() → buildHeaderText()
  → 读 rules.headerMode → 可能未同步 → 错误
```

**更深层问题**：文件列表没有做到「单一事实来源」。用户的工作流程是：
1. 确认现有页眉页脚（检测 + 确认删除/编辑）
2. 设置新的页眉页脚页码
3. 在文件列表中最终确认每个文件的结果

文件列表必须**完全真实地反映**每个文件最终会输出什么。包括：
- 新增的页眉页脚页码是什么
- 现有的页眉页脚是否能删除/编辑
- 如果不能删除/编辑，要明确提示

**修复方案**：
1. 文件列表显示统一使用 `buildHeaderTextForGroup`（与渲染一致）
2. 确保 `rules.headerMode` 与当前选中组的 `mode` 始终同步
3. 文件列表中增加状态列，显示现有页眉页脚的删除/编辑可行性

### Bug 2：按证据列表名称使用文件名

**现象**：选「按证据列表名称」，本应生成「证据1」「证据2」，实际显示文件名。

**根因追踪**：
```
per_file 模式 → headerBaseTextForGroup() → file.header ?? stripPdf(file.name)
↓
file.header 在 startHeaderEdit() 中初始化:
  row.header = rowHeaderPreview(row, index) || stripPdf(row.name)
↓
rowHeaderPreview() → buildHeaderText() → headerBaseText()
↓
此时 headerMode 可能不是 'per_file'，返回文件名
↓
file.header 被设为文件名
```

**修复方案（UI 重构）**：不依赖 `file.header`，改为 UI 选项驱动：
- 「按证据列表名称」模式下，增加专用设置区：
  - **前缀**：小列表，默认「证据」，可选「对比文件」等，也支持自定义输入
  - **序号格式**：数字 / 数字01 / 数字001 / 中文数字 / 大写中文
  - **起始编号**：默认 1
  - **编号步长**：默认 1
- 选到这个模式时，显示这些特殊设置
- 文本直接由设置计算，不依赖 `file.header`

### Bug 3：序列始终从 1 开始

**根因**：
```javascript
function formatSequenceToken(token, index = 0) {
  const value = String(Math.max(1, Number(index || 0) + 1))
  return value.padStart(token.length, '0')
}
```
硬编码 `Math.max(1, index + 1)`，没有起点和步长参数。

**修复方案**：
```javascript
function formatSequenceToken(token, index = 0, start = 1, step = 1) {
  const value = Math.max(0, start + index * step)
  return String(value).padStart(token.length, '0')
}
```

### Bug 4：删除页眉页脚失败但继续处理

**现象**：用户标记删除现有页眉页脚，系统提示「未找到可安全删除的匹配内容」，继续处理，新页眉叠加在旧页眉上。

**根因追踪**：
```
检测阶段: pdftotext -bbox → 找到文本 → 显示给用户
删除阶段:
  路径1: Artifact 路径 → 找 BDC Artifact /Subtype /Header → 非标准页眉找不到
  路径2: 普通文本路径 → 找 Tj/'/" 操作符 → Form XObject/注释找不到
结果: changed_count() == 0 → warning，继续处理
```

**核心问题**：检测用 pdftotext（渲染层面），删除用内容流解析（结构层面），技术路径不一致。

**修复方案（统一技术路径）**：
见下方「三、检测与删除统一方案」。

---

## 二、竞品调研结论

调研了 Acrobat、Foxit、WPS、PDF Expert、Smallpdf 五款编辑器。

### 行业标准功能
| 功能 | Acrobat | Foxit | WPS | PDF Expert | Smallpdf |
|------|---------|-------|-----|------------|----------|
| 6 区域布局 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 奇偶页不同 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 首页不同 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 页码格式 | 阿拉伯/罗马/字母 | 同左 | 同左+一二三/壹贰叁 | 仅阿拉伯 | 仅阿拉伯 |
| 自定义起始号 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 自定义步长 | ❌ | ❌ | ❌ | ❌ | ❌ |
| 字体/字号/颜色 | ✅ | ✅ | ✅ | 有限 | ❌ |
| 实时预览 | ✅ | ✅ | ✅ | ✅WYSIWYG | ❌ |
| 批量处理 | ✅ | ✅ | 有限 | ❌ | ❌ |
| 页码范围 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 变量宏 | 日期/页码/文件名 | 同左 | 同左 | 仅页码 | 仅页码 |
| 模板保存 | ✅ | ✅ | ❌ | ❌ | ❌ |
| Bates编号 | ❌ | ✅ | ❌ | ❌ | ❌ |

### Acrobat 设计要点
- 6 区域布局（左/中/右 × 页眉/页脚）
- 变量宏：`%PageNumber%`、`%PageCount%`、`%Date%`、`%FileName%`
- 奇偶页通过页码范围实现（跑两次）
- 预览：对话框内缩略图，实时更新
- 批量：Action Wizard

### WPS 设计要点
- 中文支持最好：一二三、壹贰叁
- 奇偶页/首页不同：复选框
- 分页式对话框（内容/页码/外观三个 tab）

### Foxit 设计要点
- 功能最接近 Acrobat
- Bates 编号（法律相关）
- 模板保存/加载

### Docsy 差异化优势（竞品都没有）
1. 证据编号（`证据1`、`证据2`...）
2. 自定义步长（`[#，6，2]` → 6, 8, 10...）
3. 证据感知页眉（自动按证据列表生成）
4. 自定义变量（`{案件名称}`、`{当事人}`）

### 技术参考
- PDF spec (ISO 32000) 只支持阿拉伯/罗马/字母页码标签
- 中文数字必须预渲染为文本
- 中国法律文书惯例：仿宋 10-10.5pt

---

## 三、检测与删除统一方案

### 当前问题

检测和删除使用不同的技术路径：
- 检测：pdftotext（外部工具，渲染层面）+ Artifact 检测（lopdf，结构层面）
- 删除：Artifact 路径（lopdf）+ 普通文本路径（lopdf）

pdftotext 能检测到的文本，lopdf 不一定能删除。

### 新方案：分层检测

```
检测策略分层：
├── 自动检测（快速）：只做 Artifact 检测
│   ├── 用 lopdf 加载 PDF，遍历内容流
│   ├── 找 BDC Artifact /Subtype /Header 或 /Footer 标记
│   ├── 纯内存操作，速度快（几百页秒级）
│   ├── 检测到的内容一定能删除/编辑（同一套技术路径）
│   └── 结果：确定性高，检测到 = 能操作
│
└── 手动触发（深度）：pdftotext + Artifact 组合检测
    ├── 调用 pdftotext -bbox 提取文本和位置
    ├── 识别非标准页眉页脚（普通文本、Form XObject）
    ├── 速度取决于页数，大文件较慢
    └── 结果：检测到不一定能操作，需要用户确认
```

### 自动检测流程

```
文件加载 → 自动触发 Artifact 检测（快速）
├── 发现标准 Artifact 页眉页脚
│   ├── 显示在文件列表中
│   ├── 标记「可删除」「可编辑」
│   └── 用户确认后，删除/编辑一定成功
├── 未发现标准 Artifact
│   ├── 显示「未发现标准页眉页脚」
│   └── 提示「需要深度检测？」
└── 整个过程秒级完成
```

### 手动深度检测流程

```
用户点击「深度检测」→ 触发 pdftotext + Artifact 组合检测
├── 大文件提示：「xxx.pdf 共 500 页，深度检测可能需要较长时间，是否继续？」
├── 检测完成
│   ├── 发现非标准页眉页脚
│   │   ├── 显示在文件列表中
│   │   ├── 标记「可能无法删除」（因为不是标准 Artifact）
│   │   └── 用户确认时提示「此页眉非标准格式，删除可能失败」
│   └── 未发现
│       └── 显示「未发现页眉页脚」
└── 用户决定是否继续
```

### 统一技术路径的关键

**Artifact 检测和删除用同一套代码**：
- 检测：`inspect_meaningful_header_footer_artifacts()` → 遍历内容流，找 Artifact 标记
- 删除：`edit_header_footer_artifacts_file()` → 遍历内容流，删除 Artifact 标记

两者都用 `lopdf` 解析 PDF 内容流，都找 `BDC Artifact /Subtype /Header` 或 `/Footer`。
所以 Artifact 检测到的内容，删除一定能成功。

**非标准页眉页脚的处理**：
- pdftotext 检测到的非标准页眉页脚，删除可能失败
- 在文件列表中标记「可能无法删除」
- 用户确认时明确告知风险
- 如果删除失败，提供「覆盖」选项（用新页眉覆盖旧页眉的位置）

---

## 四、设计方案

### 4.1 文件列表重构（Bug 1 修复 + 信息增强）

**目标**：文件列表是单一事实来源，完全真实反映每个文件的最终输出。

**文件列表新增列**：
| 列 | 内容 | 说明 |
|----|------|------|
| 文件名 | 原始文件名 | |
| 页数 | 文件页数 | |
| 页眉 | 最终页眉文本 | 统一用 buildHeaderTextForGroup 计算 |
| 页脚 | 最终页脚文本 | 同上 |
| 页码 | 最终页码格式 | |
| 现有页眉 | 现有页眉状态 | 「可删除」「可能无法删除」「无」 |
| 现有页脚 | 现有页脚状态 | 同上 |
| 状态 | 处理状态 | 「待处理」「已确认」「有警告」 |

**数据一致性保证**：
- 所有文本计算统一使用 `buildHeaderTextForGroup`（与渲染一致）
- `rules.headerMode` 与当前选中组的 `mode` 始终同步
- 修改任何设置后，文件列表自动刷新

### 4.2 页眉来源重新设计（Bug 2 修复）

**删除「不插入页眉」选项**（用开关替代）。

**保留三个选项**：
```
按证据列表名称 → 自动「证据1」「证据2」...
文件名         → 去掉扩展名的文件名
固定文本       → 用户输入，支持占位符
```

**「按证据列表名称」专用设置区**（选到此模式时显示）：

```
┌─ 按证据列表名称 ─────────────────────────┐
│                                          │
│  前缀：[证据 ▾]  （下拉：证据/对比文件/自定义）│
│  序号格式：[数字 ▾]                       │
│    ├── 数字：1, 2, 3...                  │
│    ├── 数字01：01, 02, 03...             │
│    ├── 数字001：001, 002, 003...         │
│    ├── 中文数字：一、二、三...             │
│    └── 大写中文：壹、贰、叁...            │
│  起始编号：[1]                            │
│  编号步长：[1]                            │
│                                          │
│  预览：证据1, 证据2, 证据3...             │
└──────────────────────────────────────────┘
```

**前缀下拉选项**：
- 证据（默认）
- 对比文件
- 附件
- 自定义（显示输入框）

**实现**：
- 不依赖 `file.header`，直接由前缀 + 序号格式 + 起始编号 + 步长计算
- `headerBaseTextForGroup` 中 `per_file` 模式：
  ```javascript
  if (group.mode === 'per_file') {
    const prefix = group.seqPrefix || '证据'
    const format = group.seqFormat || 'arabic'
    const start = group.seqStart || 1
    const step = group.seqStep || 1
    const num = start + index * step
    return prefix + formatNumber(num, format)
  }
  ```

### 4.3 占位符系统增强（Bug 3 修复）

**当前占位符**：
```
[#] / [##] / [###]    → 序号（1, 2, 3... / 01, 02, 03... / 001, 002, 003...）
[序号]                 → 1, 2, 3...
[中文序号]             → 一、二、三...
[文件名] / [name]      → 文件名
[日期]                 → YYYYMMDD
```

**新增占位符**：
```
[#, 6]                 → 从 6 开始
[#, 6, 2]              → 从 6 开始，步长 2
[#, 0]                 → 从 0 开始
[壹贰叁]               → 大写中文数字
```

**实现**（splitFileName.js）：
```javascript
export function expandSplitNameTokens(value, index = 0, dateValue = '', options = {}) {
  const { start = 1, step = 1 } = options
  return String(value || '').replace(/\[([^\]]+)\]/g, (match, token) => {
    // 解析 [#, 起点, 步长] 格式
    if (/^#+(,\s*\d+(,\s*\d+)?)?$/.test(token)) {
      const parts = token.split(',').map(s => s.trim())
      const padLen = parts[0].length
      const tokenStart = parts.length > 1 ? parseInt(parts[1]) : start
      const tokenStep = parts.length > 2 ? parseInt(parts[2]) : step
      return formatSequenceToken('#'.repeat(padLen), index, tokenStart, tokenStep)
    }
    if (token === '序号') return String(start + index * step)
    if (token === '中文序号') return toChineseNumber(start + index * step)
    if (token === '壹贰叁') return toChineseFormalNumber(start + index * step)
    // ... 其他占位符不变
  })
}

function formatSequenceToken(token, index = 0, start = 1, step = 1) {
  const value = Math.max(0, start + index * step)
  return String(value).padStart(token.length, '0')
}
```

### 4.4 序列格式选项

**页码格式下拉菜单**：
```javascript
export const PAGE_NUMBER_STYLES = [
  { value: 'arabic', label: '数字', sample: '1, 2, 3' },
  { value: 'arabic-padded-2', label: '数字01', sample: '01, 02, 03' },
  { value: 'arabic-padded-3', label: '数字001', sample: '001, 002, 003' },
  { value: 'chinese', label: '中文数字', sample: '一, 二, 三' },
  { value: 'chinese-formal', label: '大写中文', sample: '壹, 贰, 叁' },
  { value: 'roman-upper', label: '罗马大写', sample: 'I, II, III' },
  { value: 'roman-lower', label: '罗马小写', sample: 'i, ii, iii' },
  { value: 'circled', label: '带圈数字', sample: '①, ②, ③（1-20）' },
  { value: 'dingbat', label: '实心带圈', sample: '❶, ❷, ❸（1-20）' },
  { value: 'evidence', label: '证据编号', sample: '证据1, 证据2, 证据3' },
]
```

**后端 OverlayTextConfig 新增字段**：
```rust
#[serde(default = "default_number_start")]
number_start: u32,  // 默认 1
#[serde(default = "default_number_step")]
number_step: u32,   // 默认 1
```

### 4.5 奇偶页支持

**UI**：
```html
<el-checkbox v-model="firstPageDifferent">首页不同</el-checkbox>
<el-checkbox v-model="oddEvenDifferent">奇偶页不同</el-checkbox>
```

**后端 OverlayTextConfig 新增字段**：
```rust
#[serde(default)]
first_page_different: bool,
#[serde(default)]
first_page_text: String,
#[serde(default)]
odd_even_different: bool,
#[serde(default)]
even_page_text: String,
```

**expand_config_placeholders 扩展**：
```rust
fn expand_config_placeholders(config: &OverlayTextConfig, current_page: u32, total_pages: u32) -> String {
  // 首页不同
  if config.first_page_different && current_page == 1 {
    return expand_text(&config.first_page_text, current_page, total_pages, config)
  }
  // 奇偶页不同
  if config.odd_even_different && current_page % 2 == 0 {
    return expand_text(&config.even_page_text, current_page, total_pages, config)
  }
  // 默认
  expand_text(&config.text, current_page, total_pages, config)
}
```

### 4.6 分段→显示范围

标签从「分段」改为「显示范围」。含义：此页眉组只在指定页码范围内显示。

### 4.7 大文件检测优化

见上方「三、检测与删除统一方案」。核心：
- 自动检测只做 Artifact 检测（快速）
- 深度检测手动触发（大文件有提示）

### 4.8 删除预检（Bug 4 修复）

见上方「三、检测与删除统一方案」。核心：
- Artifact 检测到的内容一定能删除
- 非标准页眉标记「可能无法删除」
- 用户确认时明确告知风险

### 4.9 合并完成醒目提示

用 `ElNotification` 替代 `ElMessage.success`，`duration: 0`。

---

## 五、实施计划

### Phase 1：Bug 修复 + 检测统一（P0）
1. 统一检测和删除技术路径（Artifact 检测 = 能删除）
2. 自动检测只做 Artifact 检测（快速）
3. 深度检测手动触发
4. 文件列表统一使用 buildHeaderTextForGroup
5. 修复 file.header 初始化逻辑
6. 修复 formatSequenceToken 支持起点/步长

### Phase 2：UI 重构（P0）
7. 「按证据列表名称」专用设置区（前缀/序号格式/起始/步长）
8. 文件列表增加现有页眉页脚状态列
9. 删除「不添加页眉」选项

### Phase 3：高级功能（P1）
10. 奇偶页支持
11. 首页不同支持
12. 显示范围（原分段）
13. 合并完成醒目提示

### Phase 4：差异化功能（P2）
14. 证据编号宏增强
15. 用户自定义变量
16. Bates 编号
17. 模板保存/加载

---

## 六、技术参考

### PDF 规范
- ISO 32000-2:2020 Section 12.4.2 — Page Label Dictionary
- 页码标签只支持：阿拉伯(/D)、罗马大写(/R)、罗马小写(/r)、字母大写(/A)、字母小写(/a)
- 中文数字必须预渲染为文本

### 字体栈
- 仿宋 (FangSong) — 正文默认（GB 标准）
- 楷体 (KaiTi) — 正式文件
- 宋体 (SimSun) — 页眉页脚默认
- 黑体 (SimHei) — 标题强调
- 微软雅黑 (Microsoft YaHei) — 现代无衬线

### 中国法律文书格式
- GB/T 9704-2012 党政机关公文格式
- 页眉：仿宋或宋体，10-10.5pt
- 页码：居中，阿拉伯数字
