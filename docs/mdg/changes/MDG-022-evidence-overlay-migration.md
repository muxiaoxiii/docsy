# MDG-022: evidence.rs 遗留 overlay 代码迁移到 header_footer 模块

## 状态：🟢 已完成
## 优先级：P2
## 来源：evidence.rs 中 5 处 GLM-P2 TODO

---

## 一、变更内容

### 1. 公开 OverlayTextConfig

`header_footer.rs` 中 `OverlayTextConfig` 及其所有字段改为 `pub`，允许其他模块使用。

### 2. 替换 apply_overlay_batch

旧实现（~230 行）基于 `printpdf` 手动构建 PDF 操作，仅支持 Helvetica（ASCII-only）。

新实现（~90 行）构造 `serde_json::Value`（camelCase 格式），调用 `header_footer::batch_overlay`，自动获得：
- CJK 嵌入字体支持
- 标准化页眉页脚处理流程
- 统一的 artifact 标记

### 3. 字段映射

| 旧 OverlayConfig | 新 HeaderFooterJob JSON |
|---|---|
| `header.content == "filename"` | `header.text = file_stem`, `region = "header"` |
| `header.content == "custom"` | `header.text = custom_text`, `region = "header"` |
| `header.content == "sequence"` | `header.text = seq`, `region = "header"`, `numberStyle = "arabic"` |
| `footer.content == "page_total"` | `footer.text = "{page} / {total}"`, `region = "footer"` |
| `font_size` | `fontSize` |
| `y_offset` | `marginMm`（pt × 25.4 / 72.0） |

### 4. 删除的遗留代码

| 函数 | 行数 | 说明 |
|------|------|------|
| `apply_overlay_single` | 72 行 | 单文件 overlay（printpdf + qpdf） |
| `build_overlay_ops` | 93 行 | PDF 操作构建 |
| `ensure_legacy_overlay_text_supported` | 9 行 | ASCII 检查（CJK 已由嵌入字体支持） |
| `create_overlay_pdf_multi` | 11 行 | PDF 创建辅助 |
| `qpdf_all_page_dimensions` | 10 行 | 页尺寸查询（header_footer 内部处理） |
| `unique_temp_pdf` | 8 行 | 临时文件辅助 |
| **合计删除** | **~203 行** | |

### 5. 保留的结构

`OverlayConfig` / `HeaderConfig` / `FooterConfig` 保留用于前端 JSON 反序列化（行 393），前端格式不变。

---

## 二、影响分析

- **前端**：无变化，overlay JSON 格式不变
- **输出**：输出文件路径格式不变（`_overlaid/` 子目录）
- **CJK 支持**：自动获得，不再报"旧版证据叠加不支持中文"错误
- **净效果**：删除 ~110 行，新增 ~90 行

---

## 三、验证

- [x] cargo test 160/160 passed
- [x] npm test 82/82 passed
- [x] cargo check 编译通过
