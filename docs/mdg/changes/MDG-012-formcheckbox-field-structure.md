# MDG-012: Word 域（Field）启发式检测与保护

**优先级**: P0
**状态**: ✅ 已完成
**创建日期**: 2026-08-07
**分支**: codex/template-quickxml-0.8

---

## 设计理念

**启发式检测，非强制处理**。

FORMCHECKBOX 不是新的字段类型，而是已有 checkbox 类型的**特殊来源**。跟 ☑☐ 等勾选符号一样，自动推断为 checkbox 字段，用户可以在 UI 里看到并修改。

区别在于：普通勾选符号（☑）是纯文本，需要包裹 SDT 才能替换。而 FORMCHECKBOX 有完整的域结构（fldChar + ffData + instrText），渲染时应该**保留域结构**，只替换结果文本和更新 checked 状态。

---

## Word 域机制概要

### 域结构

```xml
<w:r><w:fldChar w:fldCharType="begin">      ← 域开始
  <w:ffData>                                  ← 域属性（表单域特有）
    <w:name w:val="选中1"/>
    <w:checkBox><w:checked w:val="0"/></w:checkBox>
  </w:ffData>
</w:fldChar></w:r>
<w:r><w:instrText>FORMCHECKBOX</w:instrText></w:r>  ← 域代码
<w:r><w:fldChar w:fldCharType="separate"/></w:r>    ← 分隔符
<w:r><w:t>一般代理</w:t></w:r>                       ← 域结果
<w:r><w:fldChar w:fldCharType="end"/></w:r>         ← 域结束
```

### 域类型（非穷尽）

| 类型 | 说明 | 典型 instrText |
|------|------|----------------|
| FORMCHECKBOX | 表单勾选框 | `FORMCHECKBOX` |
| FORMTEXT | 表单文本框 | `FORMTEXT` |
| FORMDROPDOWN | 表单下拉框 | `FORMDROPDOWN` |
| HYPERLINK | 超链接 | `HYPERLINK "url"` |
| TOC | 目录 | `TOC \o "1-3"` |
| REF | 交叉引用 | `REF bookmark` |
| DATE | 日期 | `DATE \@ "yyyy-MM-dd"` |
| PAGE | 页码 | `PAGE` |

域可嵌套，域代码可跨多个 run。

---

## 根因分析

当前 docx_template 模块对 Word 域**零感知**。

### 三层断裂

1. **scan.rs — 碰巧正确**：`collect_direct_run_text` 只取 `w:t`，`w:instrText` 不算文本，域代码区的 run 碰巧不被索引
2. **save.rs — 根因**：`wrap_paragraph_runs` 不识别域结构，把所有 `w:r` 一视同仁。虽然域代码区 run 不在 coord_map 里，但遍历逻辑仍然执行
3. **render.rs — 放大器**：`render_into_existing_runs` 给没有 `w:t` 的 run 注入了新的 `<w:t>`，导致 fldChar run 里出现字段文本

---

## 修复方案：启发式检测

### 原则

1. **不引入新字段类型**：FORMCHECKBOX → 复用 checkbox 类型
2. **自动推断**：检测到 FORMCHECKBOX → 自动设为 checkbox，跟 ☑☐ 一样
3. **保留域结构**：保存模板时，域代码区的 run 不包裹 SDT
4. **用户可覆盖**：UI 里可以看到并修改类型、标签等

### Phase 1：域边界感知（scan.rs）

**目标**：扫描时识别域结构，标记来自 FORMCHECKBOX 的文本 run。

**方案**：在 `scan_paragraph_runs` 中添加域深度跟踪。

```rust
fn scan_paragraph_runs(children, index, paragraph_idx, run_idx) {
    // Pre-scan: 建立域边界表
    // 遍历 children，记录每个 run 是否在域代码区、是否是 FORMCHECKBOX
    let field_map = build_field_map(children);

    for (i, child) in children.iter().enumerate() {
        if let XmlNode::Element { name, children, .. } = child {
            if name != "w:r" {
                if name == "w:sdt" || name == "w:sdtContent" || name == "w:hyperlink" {
                    scan_paragraph_runs(children, index, paragraph_idx, run_idx);
                }
                continue;
            }

            // 检查域边界表
            if field_map.is_in_field_code(i) {
                continue; // 域代码区的 run 跳过
            }

            let is_form_checkbox = field_map.is_form_checkbox_result(i);
            let form_checked = field_map.form_checked_state(i);

            // 正常扫描逻辑...
            let text = collect_direct_run_text(children);
            if !text.is_empty() {
                let checkbox_like = is_checkbox_text(&text) || is_form_checkbox;
                // ... index.add_text_node(...)
            }
        }
    }
}
```

**TextNodeRef 扩展**：

```rust
pub struct TextNodeRef {
    // ... existing fields ...
    pub checkbox_like: bool,        // 已有：是否像勾选框
    pub form_field: bool,           // 新增：是否来自 Word 域
    pub form_checked: Option<bool>, // 新增：域的默认 checked 状态
}
```

### Phase 2：域结构保护（save.rs）

**目标**：保存模板时，域代码区的 run 不包裹 SDT。

**方案**：`wrap_paragraph_runs` 同样添加域深度跟踪。

```rust
fn wrap_paragraph_runs(children, part, cursor, coord_map) {
    let mut field_depth = 0;
    let mut in_field_code = false;

    for child in children.iter_mut() {
        // 检测 fldChar 类型
        if let Some(fld_type) = get_fldchar_type(child) {
            match fld_type {
                "begin" => { field_depth += 1; in_field_code = true; }
                "separate" => { in_field_code = false; }
                "end" => { field_depth = field_depth.saturating_sub(1); }
                _ => {}
            }
            if field_depth > 0 {
                continue; // 域结构内的 run 跳过坐标处理
            }
        }

        if field_depth > 0 && in_field_code {
            continue; // 域代码区跳过
        }

        // 正常的 wrap 逻辑...
        // 但注意：域结果区的文本 run 仍然需要包裹 SDT
    }
}
```

**关键**：域结果区的文本 run（如 "一般代理"）仍然包裹 SDT，这样渲染时可以替换文本。但 fldChar、instrText 等 run 不被包裹。

### Phase 3：渲染保护（render.rs）

**目标**：渲染时保留域结构，只替换结果文本。

**方案**：`render_into_existing_runs` 不给没有 `w:t` 的 run 注入文本。

```rust
fn render_into_existing_runs(children: &[XmlNode], text: &str) -> XmlNode {
    // ... existing logic ...
    for child in children {
        if let XmlNode::Element { name, children: cc, .. } = child {
            if name == "w:r" {
                let mut new_children = cc.clone();
                // ... existing w:t replacement logic ...
                
                // 防御性修复：如果 run 没有 w:t，不注入新的 w:t
                // if first { new_children.push(...); }  ← 删除这个分支
            }
        }
    }
}
```

同时，对于 FORMCHECKBOX 类型的字段，渲染时还需要更新 `w:ffData/w:checked` 状态。这需要在渲染路径中识别域结构。

### Phase 4：前端自动推断

**现状已支持**：`inferTemplateField({ checkboxLike: true })` → type = "checkbox"。

**新增**：来自 FORMCHECKBOX 的 mark 自动设 `checkboxLike = true`。前端无需改动。

**UI 增强**（可选）：在 TemplateBuildTab 中显示 "来源: Word 表单域" 标签，让用户知道这是自动检测的。

---

## 实施顺序

| Phase | 内容 | 改动范围 | 复杂度 |
|-------|------|----------|--------|
| 1 | scan.rs 域边界感知 + TextNodeRef 扩展 | scan.rs, mod.rs | 🟡 中 |
| 2 | save.rs 域深度跟踪 | save.rs | 🟡 中 |
| 3 | render.rs 防御性修复 + checked 状态更新 | render.rs | 🟢 低 |
| 4 | 前端自动推断（已有基础） | 无需改动 | 🟢 低 |

Phase 1-3 是必须的，Phase 4 已有基础。

---

## 验证方法

1. 用包含 FORMCHECKBOX 的 Word 文件创建模板
2. 检查 .docsytpl 内的 document.xml：
   - 域代码区的 run 没有被包裹 SDT ✓
   - 域结果区的文本 run 被包裹 SDT ✓
   - fldChar/instrText 结构完整 ✓
3. 批量生成文件：
   - "一般代理"/"特殊代理" 只出现一次 ✓
   - 勾选状态正确（checked/unchecked）✓
4. 测试其他域类型（HYPERLINK 等不受影响）
5. 测试普通勾选符号（☑☐）仍然正常工作

## 测试文件

- 正确文件：`/Users/only/Documents/Workspace/浦项项目/【无效阶段文件】/444 【6号专利新无效】/无效授权委托书/无效授权委托书-4W122723-6号专利-吕晗-20260806.docx`
- 生成文件：同目录 `folder/无效授权委托书-新的-*.docx`
- 模板：`~/Library/Application Support/Docsy/templates/无效授权委托书-新的.docsytpl`

## 参考

- OOXML 规范：ECMA-376 Part 1, §17.16 (Field Definitions)
- Word 表单域：§17.16.24 (Simple Field), §17.16.4 (Field Code)
- `w:fldChar` 结构：§17.3.1 (Field Character)
- 现有勾选框检测：`scan.rs:is_checkbox_text`, `publicRules.js:inferTemplateField`
