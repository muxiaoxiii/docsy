# MDG-016: 证据处理模块页眉页脚全面优化 — 详细设计文档

## 状态：🟡 设计完成，待实施
## 优先级：P0
## 创建日期：2026-08-08

---

## 一、问题清单与根因分析

### Bug 1：固定文本 `证据[#]` 渲染为证据列表名称

**现象**：页眉来源选「固定文本」，输入 `证据[#]`，渲染出来的是「证据1」，但文件列表中显示的仍是文件名。

**根因追踪**：
```
用户输入: 证据[#]（custom 模式）
↓
headerBaseTextForGroup() → group.text = "证据[#]"
↓
decorateHeaderTextForGroup() → expandSplitNameTokens("证据[#]", index, dateValue)
↓
formatSequenceToken("#", 0) → "1"
↓
结果: "证据1"  ← 正确

但文件列表显示:
displayRowHeader() → rowHeaderPreview() → buildHeaderText() → headerBaseText()
↓
headerBaseText() 中 rules.headerMode 可能不是 'custom'，而是 'per_file' 或其他
↓
返回 file.header ?? stripPdf(file.name)  ← 文件名
```

**核心问题**：`buildHeaderText`（文件列表用）和 `buildHeaderTextForGroup`（渲染用）使用不同的参数。
- `buildHeaderText` 用 `rules.headerMode`（全局模式）
- `buildHeaderTextForGroup` 用 `group.mode`（组模式）
- 当用户在组级别切换模式时，`rules.headerMode` 可能没有同步更新

**修复方案**：
1. 文件列表显示应该统一使用 `buildHeaderTextForGroup`（与渲染一致）
2. 或者确保 `rules.headerMode` 与当前选中组的 `mode` 同步

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

**核心问题**：`file.header` 的初始化依赖当前 `headerMode`，但 `headerMode` 可能还没切换到 `per_file`。

**修复方案**：
1. `file.header` 初始化时，如果 `headerMode === 'per_file'`，强制生成 `证据${index + 1}`
2. 或者 `per_file` 模式下不依赖 `file.header`，直接计算

### Bug 3：序列始终从 1 开始

**根因**：
```javascript
// splitFileName.js
function formatSequenceToken(token, index = 0) {
  const value = String(Math.max(1, Number(index || 0) + 1))
  return value.padStart(token.length, '0')
}
```
硬编码 `Math.max(1, index + 1)`，没有起点和步长参数。

**修复方案**：
```javascript
function formatSequenceToken(token, index = 0, start = 1, step = 1) {
  const value = start + index * step
  return String(Math.max(0, value)).padStart(token.length, '0')
}
```
支持 `[#, 6]`（从 6 开始）和 `[#，6，2]`（从 6 开始，步长 2）。

### Bug 4：删除页眉页脚失败但继续处理

**现象**：用户标记删除现有页眉页脚，系统提示「已请求删除现有页眉页脚，但没有找到可安全删除的匹配内容」，但继续处理，最终新页眉叠加在旧页眉上。

**根因追踪**：
```
前端检测: pdftotext -bbox → 找到页眉区域文本 → 显示给用户
用户确认: 标记删除
↓
后端处理:
  1. edit_or_delete_standard_artifacts_if_requested()
     → artifacts::edit_header_footer_artifacts_to_temp()
     → 在 PDF 内容流中找 BDC Artifact /Subtype /Header 标记
     → 如果页眉不是用标准 Artifact 标记的 → changed_count() == 0
  2. delete_confirmed_plain_text_header_footer_if_requested()
     → content_text::delete_plain_header_footer_to_temp()
     → 在 PDF 内容流中找文本操作符 Tj/'/"
     → 如果页眉是 Form XObject 或注释 → 找不到文本操作符
  3. semantic_removed == 0 → 生成 warning，但继续处理
```

**核心问题**：检测用 `pdftotext`（外部工具，能识别任何可见文本），删除用 PDF 内容流解析（只能删除 Artifact 标记或直接文本操作符）。两种技术路径不一致。

**用户的正确观点**：检测到了就应该能删。如果检测找到了文本，说明它在 PDF 中是可见的，应该可以删除。

**修复方案**：
1. **预检机制**：处理前对每个有删除标记的文件，先尝试删除到临时文件，检查是否成功
2. **如果删除失败**：弹窗告知用户「无法安全删除以下文件的页眉页脚：xxx。是否继续添加新页眉？」
3. **用户选择**：
   - 「继续」→ 只添加新页眉（可能叠加在旧页眉上）
   - 「取消」→ 中止处理
4. **长期方案**：统一检测和删除的技术路径，确保检测到的内容一定能删除

---

## 二、竞品调研摘要

### 功能对比（5 款 PDF 编辑器）

| 功能 | Acrobat | Foxit | WPS | PDF Expert | Smallpdf |
|------|---------|-------|-----|------------|----------|
| 6 区域布局 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 奇偶页不同 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 首页不同 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 页码格式 | 阿拉伯/罗马/字母 | 同左 + 中文 | 同左 + 一二三/壹贰叁 | 仅阿拉伯 | 仅阿拉伯 |
| 自定义起始号 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 自定义步长 | ❌ | ❌ | ❌ | ❌ | ❌ |
| 字体/字号/颜色 | ✅ | ✅ | ✅ | 有限 | ❌ |
| 实时预览 | ✅ | ✅ | ✅ | ✅ WYSIWYG | ❌ |
| 批量处理 | ✅ | ✅ | 有限 | ❌ | ❌ |
| 页码范围 | ✅ | ✅ | ✅ | ❌ | ❌ |
| 变量宏 | 日期/页码/文件名 | 同左 | 同左 | 仅页码 | 仅页码 |
| 模板保存 | ✅ | ✅ | ❌ | ❌ | ❌ |
| Bates 编号 | ❌ | ✅ | ❌ | ❌ | ❌ |

### Docsy 差异化优势（竞品没有的）
1. **证据编号**：`证据1`、`证据2`...（法律证据专用）
2. **自定义步长**：`[#，6，2]` → 6, 8, 10...
3. **中文序号**：一、二、三（WPS 有，其他没有）
4. **证据感知页眉**：自动按证据列表生成页眉
5. **自定义变量**：`{案件名称}`、`{当事人}`

### 技术参考
- PDF spec (ISO 32000) 只支持阿拉伯/罗马/字母页码标签
- 中文数字必须预渲染为文本（不能用 PDF Page Labels）
- 中国法律文书惯例：仿宋 10-10.5pt
- 推荐实现：content stream 注入 + Form XObject 预览

---

## 三、设计方案

### 3.1 页眉来源重新设计

**当前选项**（HeaderFooterRuleFields.vue:57-62）：
```javascript
none     → 不插入页眉
filename → 文件名
per_file → 按证据列表名称
custom   → 固定文本
```

**新设计**：
```javascript
per_file → 按证据列表名称    → 自动「证据1」「证据2」...
filename → 文件名           → 去掉扩展名的文件名
custom   → 固定文本         → 用户输入，支持占位符
// 删除 none（用开关替代）
```

**关键改动**：
- 删除「不插入页眉」选项（用 headerInsertEnabled 开关替代）
- 「按证据列表名称」严格生成 `证据 + 序号`，不 fallback 到文件名
- 「固定文本」支持增强占位符系统

**代码改动**：
- `HeaderFooterRuleFields.vue`：删除 `none` 选项
- `useEvidencePdfSession.js`：
  - `headerBaseTextForGroup()` 中 `per_file` 模式直接返回 `证据${index + 1}`，不查 `file.header`
  - `headerBaseText()` 同步修改
  - `displayRowHeader()` 统一使用 `buildHeaderTextForGroup`

### 3.2 占位符系统增强

**当前占位符**（splitFileName.js `expandSplitNameTokens`）：
```
[#] / [##] / [###]    → 序号（1, 2, 3... / 01, 02, 03... / 001, 002, 003...）
[序号]                 → 1, 2, 3...
[中文序号]             → 一、二、三...
[文件名] / [name]      → 文件名
[日期]                 → YYYYMMDD
[YYYY-MM-DD]           → 自定义日期
```

**新增占位符**：
```
[#, 起点]              → 从指定数字开始（如 [#, 6] → 6, 7, 8...）
[#, 起点, 步长]        → 指定步长（如 [#, 6, 2] → 6, 8, 10...）
[#, 0]                 → 从 0 开始
[壹贰叁]               → 大写中文数字（壹、贰、叁...）
```

**代码改动**（splitFileName.js `expandSplitNameTokens`）：
```javascript
export function expandSplitNameTokens(value, index = 0, dateValue = '', options = {}) {
  const { start = 1, step = 1 } = options
  return String(value || '').replace(/\[([^\]]+)\]/g, (match, token) => {
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
    // ... 其他占位符
  })
}

function formatSequenceToken(token, index = 0, start = 1, step = 1) {
  const value = Math.max(0, start + index * step)
  return String(value).padStart(token.length, '0')
}
```

### 3.3 序列格式选项（与模板模块对齐）

**页码格式下拉菜单**：
```javascript
// pdfPageNumberRules.js PAGE_NUMBER_STYLES 扩展
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

**起点和步长 UI**：
```html
<div class="rule-item">
  <label>起始编号</label>
  <el-input-number v-model="numberStart" :min="0" :max="9999" />
</div>
<div class="rule-item">
  <label>编号步长</label>
  <el-input-number v-model="numberStep" :min="1" :max="100" />
</div>
```

**后端扩展**（OverlayTextConfig）：
```rust
#[serde(default = "default_number_start")]
number_start: u32,  // 默认 1
#[serde(default = "default_number_step")]
number_step: u32,   // 默认 1
```

### 3.4 奇偶页支持

**UI 设计**（参考 WPS）：
```html
<div class="rule-item">
  <el-checkbox v-model="firstPageDifferent">首页不同</el-checkbox>
</div>
<div class="rule-item">
  <el-checkbox v-model="oddEvenDifferent">奇偶页不同</el-checkbox>
</div>
```

**数据结构扩展**（OverlayTextConfig）：
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

**后端实现**（expand_config_placeholders）：
```rust
fn expand_config_placeholders(config: &OverlayTextConfig, current_page: u32, total_pages: u32) -> String {
  // 首页不同
  if config.first_page_different && current_page == 1 {
    return config.first_page_text.clone()
      .replace("{page}", &format_page_number(1, &config.number_style))
      .replace("{total}", &format_page_number(total_pages, &config.number_style))
  }
  // 奇偶页不同
  if config.odd_even_different && current_page % 2 == 0 {
    return config.even_page_text.clone()
      .replace("{page}", &format_page_number(current_page, &config.number_style))
      .replace("{total}", &format_page_number(total_pages, &config.number_style))
  }
  // 默认
  config.text.clone()
    .replace("{page}", &format_page_number(current_page, &config.number_style))
    .replace("{total}", &format_page_number(total_pages, &config.number_style))
}
```

### 3.5 分段功能重新设计

**当前问题**：标签「分段」容易误导，用户理解为「某个页码的起点和终点」。

**新设计**：
- 标签改为「显示范围」
- 含义：此页眉组只在指定页码范围内显示
- 默认：全部页面
- 支持多组页眉，每组有自己的显示范围

**UI 改动**：
```html
<div class="rule-item page-range-row">
  <label>显示范围</label>
  <div class="page-range-inputs">
    <el-input-number v-model="headerPageStartModel" :min="1" placeholder="起始页" />
    <span class="range-sep">–</span>
    <el-input-number v-model="headerPageEndModel" :min="0" placeholder="结束页（0=全部）" />
  </div>
</div>
```

### 3.6 大文件检测优化

**方案**：
1. 检测前检查文件页数
2. 超过阈值（默认 100 页）时弹窗：
   ```
   文件 xxx.pdf 共 500 页，检测现有页眉页脚可能需要较长时间。
   
   ○ 跳过检测，直接添加新页眉页脚
   ○ 执行检测（预计需要 30 秒）
   ```
3. 用户选择「跳过检测」→ 清除删除标记，只执行添加
4. 用户选择「执行检测」→ 正常流程

**代码改动**（useEvidencePdfDetection.js）：
```javascript
async function detectAllHeaderFooter(options = {}) {
  // ... 现有代码 ...
  for (let i = 0; i < total; i++) {
    const file = overlayRows.value[i]
    // 大文件预检
    if (file.pages > LARGE_FILE_THRESHOLD) {
      const action = await ElMessageBox.confirm(
        `文件 ${file.name} 共 ${file.pages} 页，检测现有页眉页脚可能需要较长时间。`,
        '大文件检测',
        {
          confirmButtonText: '执行检测',
          cancelButtonText: '跳过检测',
          distinguishCancelAndClose: true,
        }
      ).catch(() => 'skip')
      if (action === 'skip') {
        file.statusText = '跳过检测'
        file.statusType = 'info'
        continue
      }
    }
    // ... 正常检测 ...
  }
}
```

### 3.7 删除预检（Bug 4 修复）

**新流程**：
```
用户标记删除 → 处理前预检 → 检测到可删除内容？
  ├─ 是 → 正常删除 + 添加新页眉
  └─ 否 → 弹窗：「未找到可删除的页眉页脚，是否继续添加？」
           ├─ 继续 → 只添加新页眉（不删除）
           └─ 取消 → 中止处理
```

**实现方案**：
1. 前端：在 `buildHeaderFooterItems` 中，对有删除标记的文件，先调用后端预检接口
2. 后端：新增 `preview_header_footer_deletion` 命令，只检测不修改
3. 如果预检发现无法删除，前端弹窗确认

**长期方案**：统一检测和删除的技术路径
- 检测时记录文本在 PDF 内容流中的位置（操作符索引）
- 删除时直接使用记录的位置，而不是重新查找
- 这样检测到的内容一定能删除

### 3.8 合并完成醒目提示（需求 7）

**当前**：`ElMessage.success`（3 秒 toast）

**新设计**：`ElNotification`
```javascript
ElNotification({
  title: '✅ 证据 PDF 处理完成',
  message: `已处理 ${successCount} 个 PDF${mergeText}${warningText}`,
  type: 'success',
  duration: 0,  // 不自动关闭
  dangerouslyUseHTMLString: true,
})
```

---

## 四、实施计划

### Phase 1：Bug 修复（P0）
1. 修复 `headerMode` 与 group `mode` 不一致问题
2. 修复 `file.header` 初始化逻辑
3. 修复 `formatSequenceToken` 支持起点/步长
4. 删除预检机制

### Phase 2：核心增强（P0）
5. 占位符系统增强（`[#，起点，步长]`）
6. 序列格式选项（与模板模块对齐）
7. 删除「不添加页眉」选项

### Phase 3：高级功能（P1）
8. 奇偶页支持
9. 首页不同支持
10. 分段功能重新设计
11. 大文件检测优化
12. 合并完成醒目提示

### Phase 4：差异化功能（P2）
13. 证据编号宏
14. 用户自定义变量
15. Bates 编号支持
16. 模板保存/加载

---

## 五、技术参考

### PDF 规范
- ISO 32000-2:2020 Section 12.4.2 — Page Label Dictionary
- 页码标签只支持：阿拉伯(/D)、罗马大写(/R)、罗马小写(/r)、字母大写(/A)、字母小写(/a)
- 中文数字必须预渲染为文本

### 字体栈
```
中文法律文书：
├── 仿宋 (FangSong) — 正文默认（GB 标准）
├── 楷体 (KaiTi) — 正式文件
├── 宋体 (SimSun) — 页眉页脚默认
├── 黑体 (SimHei) — 标题强调
└── 微软雅黑 (Microsoft YaHei) — 现代无衬线
```

### 中国法律文书格式参考
- GB/T 9704-2012 党政机关公文格式
- 页眉：仿宋或宋体，10-10.5pt
- 页码：居中，阿拉伯数字

---

## 六、Claude 调研补充

### Acrobat 页眉页脚设计要点
- 6 区域布局（左/中/右 × 页眉/页脚）
- 变量宏系统：`%PageNumber%`、`%PageCount%`、`%Date%`、`%FileName%`
- 页码格式：阿拉伯、罗马、字母
- 自定义起始号：支持
- 奇偶页：通过页码范围实现（如 "1,3,5,7..." 和 "2,4,6,8..."）
- 首页不同：通过排除页码 1 实现
- 预览：对话框内缩略图，实时更新
- 批量处理：Action Wizard

### WPS 页眉页脚设计要点
- 最佳中文支持：一二三、壹贰叁
- 奇偶页不同：复选框
- 首页不同：复选框
- 页码格式：阿拉伯、罗马、字母、中文
- 预览：对话框内预览

### Foxit 页眉页脚设计要点
- 接近 Acrobat 的功能集
- Bates 编号支持（法律相关）
- 模板保存/加载
- 批量处理

### 技术实现建议
- PDF 规范不支持中文页码标签，必须预渲染为文本
- 推荐：content stream 注入（最终输出）+ Form XObject（预览/撤销）
- 中国法律文书惯例：仿宋 10-10.5pt
