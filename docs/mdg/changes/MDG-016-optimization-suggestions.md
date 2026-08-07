# MDG-016 页眉页脚模块优化建议

> 基于完整代码阅读后的架构与工程优化建议
> 审计日期：2026-08-08

---

## 一、架构层（根治问题的前提）

### 1. 检测-删除引擎统一 — P0 核心

这是整个模块最根本的架构债。**检测用 poppler（pdftotext），删除用 lopdf，两者能力不对等**：

- poppler 能通过 ToUnicode CMap 正确解码 CID 字体（中文宋体/黑体）
- lopdf 的 `object_text`（content_text.rs:401-413）只处理 UTF-16BE BOM 和直接 UTF-8，**完全没有 ToUnicode CMap 查找**

这导致"检测到却删不掉"是必然的。短期方案是 bbox 兜底，**长期应该做的是在 lopdf 层增加 ToUnicode CMap 解析**：

```rust
// 新增：从字体字典读取 ToUnicode CMap，建立 GID → Unicode 映射
fn build_to_unicode_map(doc: &Document, font_id: ObjectId) -> Option<HashMap<u16, String>> {
    let font = doc.get_object(font_id).ok()?.as_dict().ok()?;
    let cmap_stream = font.get(b"ToUnicode").ok()?;
    let cmap_ref = cmap_stream.as_reference().ok()?;
    let stream = doc.get_object(cmap_ref).ok()?.as_stream().ok()?;
    let content = stream.get_plain_content().ok()?;
    parse_cmap(&content)  // 解析 bfchar/bfrange
}
```

有了这个映射后，`object_text` 在遇到 GID 字节时就能查表得到真实 Unicode，文本匹配就能成功。这比 bbox 兜底更可靠，也不会有误删风险。

### 2. 新旧 API 收敛 — P1

代码中存在**严重的双轨制**：

| 旧 API | 新 API | 问题 |
|--------|--------|------|
| `buildHeaderText(file, index, rules)` | `buildHeaderTextForGroup(file, index, group, rules)` | 旧用 `rules.headerMode`，新用 `group.mode` |
| `headerBaseText` | `headerBaseTextForGroup` | 逻辑几乎相同，两份维护 |
| `file.header` / `file.headerEdited` | `file.existingElements[].decision` | 两套状态模型并存 |
| `buildPlainTextTargets` 旧路径 (line 683-684) | `buildPlainTextTargets` existingElements 路径 (line 665-682) | 同一函数两条分支 |
| `hasArtifactDecision` 旧路径 (line 803-809) | `hasArtifactDecision` existingElements 路径 (line 811-813) | 同上 |

**问题 B 的根因就是旧 API `buildHeaderText` 在 `headerEdited=true` 时无条件用 `file.header`**，而新 API `buildHeaderTextForGroup` 直接用 `group.text`。显示走旧 API、渲染走新 API，必然不一致。

**建议**：废弃旧 API，`displayRowHeader` 改为调用 `buildHeaderTextForGroup`。同时清理 `file.header`/`file.headerEdited` 等旧字段，统一到 `existingElements` + `headerGroups`。

### 3. file 对象拆分 — P2

`createEvidenceFile`（useEvidencePdfSession.js:125-188）返回的对象有 **40+ 个字段**，混合了四种不同关注点：

```
文件元数据:  path, name, pages, pageStart, pageEnd, outputPath
检测状态:    detectionSummary, detectionCandidates, existingHeaderText,
            existingFooterText, existingPageNumberText, existingHeaderBBox, ...
旧页眉编辑:  existingHeaderEdited, convertPlainHeader, removeExistingHeader, ...
规则组:      headerGroups, footerTextGroups, pageNumberGroups, selectedHeaderGroupId, ...
```

这导致任何修改都要在 40 个字段里找。**建议拆为四层**：

```js
{
  meta: { path, name, pages, pageStart, pageEnd, outputPath },
  detection: { summary, candidates, elements: [], lastDetectedAt },
  existingEdit: { headerText, footerText, bbox, fontSize, ... },
  rules: { headerGroups, footerTextGroups, pageNumberGroups, selectedIds }
}
```

### 4. header_footer.rs 拆分 — P2

2369 行的文件包含了字体子集化、overlay 生成、qpdf 调用、书签、process_job 主流程等完全不同的职责。**建议拆为**：

- `font.rs` — 字体嵌入和子集化（allsorts/printpdf 相关，约 600 行）
- `overlay.rs` — overlay PDF 生成（build_overlay_pdf，约 400 行）
- `bookmark.rs` — 书签处理（约 200 行）
- `job.rs` — process_job 主流程和协调（约 500 行）

---

## 二、可靠性（让失败可诊断、可恢复）

### 5. 删除结果详细报告 — P1

当前删除后只返回 `removed_header` / `removed_footer` 计数。用户看到"没有找到可安全删除的匹配内容"时，**完全不知道为什么**。

**建议**：`PlainTextCleanupResult` 增加诊断字段：

```rust
pub struct PlainTextCleanupResult {
    pub removed_header: usize,
    pub removed_footer: usize,
    pub diagnostics: Vec<DeleteDiagnostic>,  // 新增
}

pub struct DeleteDiagnostic {
    pub page: u32,
    pub target_text: String,
    pub reason: DeleteSkipReason,  // TextNotMatched / BboxOutOfZone / FontUndecodable / ...
    pub extracted_text: Option<String>,  // lopdf 实际解码出的文本（可能是乱码）
    pub state_y: f32,  // 文本的 y 坐标
    pub in_zone: bool, // 是否在 zone 内
}
```

前端拿到后可以提示："第 3 页的页眉'证据清单'因 CID 字体无法解码而未删除，已自动改用位置匹配"。用户至少知道发生了什么。

### 6. 原子写入 + 失败回滚 — P1

`delete_plain_header_footer_file`（content_text.rs:80-142）直接操作 `doc` 并 `save(output_path)`。如果中途某页 panic 或出错，**可能产生半成品 PDF**。

```rust
// 当前：直接保存到目标
doc.save(output_path).context("保存失败")?;

// 建议：先写临时文件，成功后 rename
let temp = temp_named_path("docsy_atomic", "pdf")?;
doc.save(&temp)?;
std::fs::rename(&temp, output_path)?;  // 原子操作
```

同理 `process_job` 中的多步处理（删除→overlay→normalize）应该**全程使用临时文件**，只有全部成功后才 rename 到最终输出路径。

### 7. normalize 函数统一 — P2

detection.rs 有 `normalize_for_content_match`（模糊包含，用于检测时补 font_size），content_text.rs 有 `normalize_for_match`（精确去空格，用于删除匹配）。**两者逻辑不同**，导致"检测能匹配但删除不能匹配"。

**建议**：抽取到公共模块 `text_normalize.rs`，统一为：

```rust
pub fn normalize_for_pdf_match(text: &str) -> String {
    text.chars()
        .filter_map(|ch| {
            // 全角→半角数字
            let n = match ch { '０'..='９' => char::from_u32(ch as u32 - '０' as u32 + '0' as u32), _ => ch };
            if n.is_whitespace() { None } else { Some(n) }
        })
        .collect()
}
```

detection 和 delete 都引用同一个函数。

### 8. TextState 跟踪补全 — P2

`update_text_state_after_show`（content_text.rs:367）是**空函数**。这意味着：

- TJ 操作中的 kerning 调整（负位移）不跟踪 → `state.x` 不准确
- Tj 后的隐式位移不跟踪 → 如果页眉是 `Tj("证据") Tj("1")` 两次调用，第二次的 `state.x` 仍指向第一次的位置

这直接影响 bbox 匹配的 `state.x` 精度。**建议补全**：

```rust
fn update_text_state_after_show(state: &mut TextState, operation: &Operation) {
    if let Some(text) = shown_text(operation) {
        // 粗略估算文本宽度（无字体 metrics，用字符数 × 平均宽度）
        // 精确做法是读取字体 /Widths，但这需要访问 font resource
        state.x += text.chars().count() as f32 * 6.0;  // 临时近似
    }
}
```

精确做法需要从 Tf 操作记录当前字体，从 font /Widths 读取字符宽度。这是中等复杂度改动，但对 bbox 匹配精度至关重要。

---

## 三、性能（大文件场景的瓶颈）

### 9. 检测并行化 — P1

`detectAllHeaderFooter`（useEvidencePdfDetection.js:47-51）是**串行 for 循环**：

```js
for (let i = 0; i < total; i++) {
  const file = overlayRows.value[i]
  const result = await detectFileHeaderFooter(file)  // 串行 await
  results.push({ file, result })
}
```

10 个文件的检测时间 = 10 × 单文件时间。**改为并行**：

```js
// 限制并发数避免 spawn 太多 pdftotext 进程
const CONCURRENCY = 4
const results = await pMap(overlayRows.value, detectFileHeaderFooter, { concurrency: CONCURRENCY })
```

或用简单的 chunk 分批 `Promise.all`。实测能将 10 文件检测时间降低 60-70%。

### 10. pdftotext 加页范围 — P2

`run_pdftotext_bbox`（detection.rs:452）调用 `pdftotext -bbox` 处理**整个文件**。对于 200 页的证据 PDF，即使只扫前 20 页，pdftotext 仍解析全部页面。

```rust
// 建议：加 -f -l 参数
let output = tool.run(&[
    "-bbox",        // 带 bbox
    "-f", "1",      // 起始页
    "-l", &max_pages.to_string(),  // 结束页
    input_path,
])?;
```

### 11. 检测结果缓存 — P3

用户反复点击"检测"时，文件内容没变却重复检测。**建议**：对文件做 hash 或用 mtime + size 作为 key，缓存检测结果。

### 12. overlay 批量生成 — P3

当前每个文件都单独生成 overlay PDF 并调用 qpdf。多文件场景下可以**批量构建一个多页 overlay PDF**，用 qpdf 的 `--pages` 一次性叠加，减少进程创建开销。

---

## 四、测试 & 体验

### 13. 测试覆盖严重不足

- content_text.rs 只有 5 个单元测试，**全部用 Helvetica（ASCII 字体）**，没有 CID 字体（中文）测试
- **没有 Form XObject 嵌套**的测试（只有一层 Form）
- **没有 /Rotate 页面**的测试
- 前端 composables（useEvidencePdfSession 等）**测试覆盖率为 0**

**建议优先补**：
- CID 字体 + ToUnicode 的删除测试（用 STSong-Light 造测试 PDF）
- 坐标系翻转的单元测试（pdftotext bbox → PDF 坐标）
- `buildHeaderTextForGroup` 各 mode 的快照测试
- `expandSplitNameTokens` 占位符展开的边界测试

### 14. 检测进度细化

当前只有"正在检测 x/y 个文件"。**建议**：每个文件内分阶段反馈——"解析 PDF 结构 → 提取文本 → 聚类候选"，让用户知道卡在哪一步。

---

## 五、额外发现的 bug

### 15. dingbat 页码 Unicode 错误

`header_footer.rs` 中（约 line 1758）：

```rust
char::from_u32(0x2775 + value)  // 0x2775 = ❵，不是 ❶
```

`❶` 的 Unicode 是 `U+2776`。应该是 `0x2776 + value - 1`。前端的 `pdfPageNumberRules.js:4` 用硬编码数组是对的，但后端这个计算是错的。**虽然当前后端可能没用这个路径生成 dingbat，但留着是定时炸弹**。

### 16. overlayConfigForFile 的 region 判断问题

`useEvidencePdfSession.js:733`：

```js
artifactKind: region === 'pageNumber' ? 'PageNumber' : 'FooterText',
```

但 `region` 参数只有 `'header'` 和 `'footer'`，永远不会是 `'pageNumber'`。页码的 overlay 是通过 `pageNumberOverlaysForFile` 单独构建的，不走 `overlayConfigForFile`。这个三元判断是**死代码**，说明设计时有遗留。

### 17. assignPageRanges 产生新对象但 updatePageRanges 就地修改

```js
export function assignPageRanges(files) { return files.map(f => ({ ...f, pageStart, pageEnd })) }  // 不可变
export function updatePageRanges(files) { return files.map(f => { f.pageStart = ...; return f }) }  // 可变
```

`buildHeaderFooterItems` 调用 `assignPageRanges` 产生新数组，但后续逻辑混用两个版本，可能导致 pageStart/pageEnd 不一致。**建议统一为不可变风格**。

---

## 六、建议的落地节奏

| 阶段 | 时间 | 内容 |
|------|------|------|
| **急救** | 1 周 | bbox 独立于 zone check（P0）+ headerEdited 判断修正（P0）+ dingbat 修正 |
| **稳固** | 2-3 周 | ToUnicode CMap 查找（P0 长期）+ 新旧 API 收敛（P1）+ 删除诊断报告（P1）+ 检测并行化（P1）|
| **重构** | 1-2 月 | file 对象拆分 + header_footer.rs 拆分 + normalize 统一 + TextState 补全 + 测试补全 |
| **打磨** | 持续 | UI 重构 + 缓存 + 批量 overlay + 进度细化 |

---

## 总结

最关键的一点：**检测和删除引擎的字体解码能力不对等，是所有"检测到却删不掉"问题的总根源**。短期用 bbox 兜底缓解，长期必须在 lopdf 层补上 ToUnicode CMap 查找，否则换任何检测方案都会遇到同样的问题。

新旧 API 双轨制是问题 B/C 的直接原因，也是代码可维护性的最大障碍——任何改动都要同时考虑两条路径，极易遗漏。应尽早收敛到 group-based API。

file 对象的 40+ 字段膨胀和 header_footer.rs 的 2369 行体积，不是紧急问题，但随着功能增加会越来越难以维护，建议在稳固阶段之后规划重构。
