# PDF 底层库选型与纯 Rust 化实施方案

更新时间：2026-07-13

本文档供 Codex 实施参考，与 `pdf-evidence-processing-design.md` 配合使用。

## 目标

- 去掉 qpdf、pdftoppm、pdftotext 等外部二进制依赖
- 纯 Rust 化 PDF 处理栈
- 愿意重度投入实现

## 推荐方案：lopdf 核心 + hayro 渲染 + pdf-syntax 解析

### 整体架构

```
当前:  printpdf(生成) + qpdf CLI(写入) + pdftoppm(渲染) + pdftotext(文本提取)
目标:  printpdf(生成) + lopdf(读写/合并/拆分) + hayro(渲染) + pdf-syntax(轻量解析)
```

### 各库职责划分

| 库 | 替代什么 | 职责 |
|---|---------|------|
| **lopdf** | qpdf CLI | 合并、拆分、解锁、overlay 合成、页面信息提取 |
| **hayro** | pdftoppm | PDF 页面渲染为位图（预览、A4 归一化） |
| **pdf-syntax** | qpdf --json / --show-pages | 轻量只读解析、Artifact 检测、内容流扫描 |
| **printpdf** | 保持不变 | 生成 overlay PDF、A4 归一化 PDF |
| ~~pdftotext~~ | pdf-syntax 自实现 | 文本+bounding box 提取（见下文） |

### 库版本与下载量参考

| 库 | 版本 | 下载量/周 | 纯 Rust | 用途 |
|---|------|----------|---------|------|
| lopdf | 0.44.0 | 514K | ✅ | 读写/合并/拆分/overlay |
| hayro | 0.7.1 | 70K | ✅ | PDF 渲染为位图 |
| pdf-syntax | 0.5.6 | 14 | ✅ | 只读解析/内容流扫描 |
| printpdf | 0.9.1 | — | ✅ | 生成 overlay PDF（已有） |
| pdfium-render | 0.9.2 | 46K | ❌ (C++) | 备选方案 |
| qpdf (crate) | 0.3.5 | 285 | ❌ (C++) | 当前 CLI 的 Rust 绑定（不采用） |

---

### pdf-syntax 的角色详解

pdf-syntax (v0.5.6) 是一个**纯只读**的 PDF 解析库，fork 自 hayro-syntax，由 PDFluent 商业 SDK 维护。它是整个方案中**最轻量**的组件（~15K SLoC），但承担关键的读取职责。

**能做什么**:
- 解析 xref 表（所有格式，包括 xref stream）
- 解析所有 PDF 对象类型（包括 object stream）
- 遍历页面及其内容流的**类型化操作符**（typed operations）
- 解码 PDF 流（FlateDecode 等）

**不能做什么**:
- 不能写入/修改/合并/拆分 PDF（明确声明只读）
- 不支持加密 PDF（尚未实现）
- 不提供高级功能（字体解析、颜色空间等）

**在 Docsy 中的具体职责**:

| 当前实现 | pdf-syntax 替代 |
|---------|----------------|
| `qpdf --json <file>` 提取页面尺寸 | `Pdf::new()` → `pages()` → `media_box()` |
| `qpdf --show-pages --json` 获取每页信息 | `page.typed_operations()` 遍历内容流 |
| `qpdf --qdf` 生成临时文件扫描 Artifact | 直接在内存中解析内容流，查找 BDC/EMC 标记 |
| `pdftotext -bbox` 提取文本位置 | 解析 Tj/TJ/Tm/Td 操作符 + 矩阵运算得到 bbox |

**关键优势**: pdf-syntax 直接在内存中解析，不需要像当前 qpdf 那样生成临时文件。对于 Artifact 检测和文本提取，它是纯 Rust 方案中最轻量的选择。

**注意**: pdf-syntax 是 hayro 生态的底层组件。hayro（渲染器）和 hayro-write（页面重写）都依赖它。引入 pdf-syntax 不会与 hayro 冲突，反而为未来扩展打下基础。

### 关键问题：pdftotext 替代

pdftotext 用于 `detection.rs` 中提取文本+bbox 进行页眉页脚检测。替代方案：

1. **pdf-syntax**: 可以解析内容流获取文本操作符（Tj/TJ），但不直接提供 bbox
2. **hayro-interpret**: 渲染管线中有文本定位能力，可以提取文本位置
3. **自实现**: 解析内容流 + 计算文本矩阵变换 → 得到 bbox

**建议**: 用 pdf-syntax 解析内容流中的文本操作符，配合 CTM（当前变换矩阵）计算文本位置。这是最轻量的方案。

---

## 分阶段实施计划

### Phase 1: lopdf 替换 qpdf 的合并/拆分/解锁（核心）

**涉及文件**:
- `src-tauri/src/pdf/qpdf.rs` — 主要重写
- `src-tauri/src/pdf/evidence.rs` — 合并调用点
- `src-tauri/Cargo.toml` — 添加 lopdf 依赖

**合并实现**:

lopdf 合并需要手动操作，但逻辑已验证（lopdf 官方示例）：
1. 创建新 Document
2. 对每个源文档执行 `renumber_objects_with(max_id)` 避免 ID 冲突
3. 收集所有 Page 对象，设置统一的 Parent
4. 构建新的 Pages 字典（Kids + Count）
5. 构建新的 Catalog
6. 保存

**拆分实现**:
1. 加载源 PDF
2. 对每一页，提取 Page 对象及其依赖（XObject、Font、Content Stream）
3. 构建只包含目标页的新 Document
4. 保存为独立文件

**解锁实现**:
lopdf 支持空密码自动解密（v0.44.0）。对于需要密码的 PDF：
- `doc.authenticate_password(password)`
- 重新保存为无加密 PDF

**页面信息提取**:
```rust
let doc = Document::load(path)?;
let pages = doc.get_pages();
for (_, page_id) in pages {
    let page = doc.get_object(page_id)?;
    let mediabox = page.as_dict()?.get(b"MediaBox")?;
    // 解析 [llx lly urx ury]
}
```

### Phase 2: lopdf 实现 overlay 合成（最复杂）

**涉及文件**:
- `src-tauri/src/pdf/header_footer.rs` — 主要重写
- `src-tauri/src/pdf/evidence.rs` — overlay 调用点

**实现策略**:

qpdf 的 `--overlay` 做的事情是：将 overlay PDF 的每一页内容流叠加到目标 PDF 的对应页上。

用 lopdf 实现需要：

1. **解析 overlay PDF 的内容流**
   - 读取 overlay 每页的 Contents 流
   - 解码（可能被 FlateDecode 压缩）

2. **解析目标 PDF 的内容流**
   - 读取目标每页的 Contents 流

3. **合并内容流**
   - 将 overlay 的内容流操作追加到目标页内容流末尾
   - 关键：需要用 `q` (save state) 和 `Q` (restore state) 包裹 overlay 内容

4. **处理资源冲突**
   - overlay PDF 引用的字体（如 /F1）可能与目标 PDF 冲突
   - 需要重命名 overlay 的资源引用（/F1 → /DocsyOverlayF1 等）
   - 将 overlay 的 Resource 字典合并到目标页的 Resource 字典

5. **保存结果**

**风险点**:
- 内容流操作符解析需要正确处理所有 PDF 操作符
- 资源重命名需要同时修改内容流中的引用
- 某些 PDF 的内容流可能有嵌套的 Form XObject 引用

**缓解措施**:
- 对所有 PDF 都采用纯内容流合并策略
- 需要实现完整的 PDF 操作符解析和资源冲突处理

### Phase 3: hayro 替换 pdftoppm

**涉及文件**:
- `src-tauri/src/pdf/normalize.rs` — A4 归一化渲染
- `src-tauri/src/pdf/preview.rs` — 预览渲染
- `src-tauri/Cargo.toml` — 添加 hayro 依赖

**实现**:
```rust
use hayro::{Pdf, RenderConfig};
use std::sync::Arc;

let data = std::fs::read(path)?;
let pdf = Pdf::new(Arc::new(data))?;
let page = &pdf.pages()[page_index];

let config = RenderConfig::new()
    .set_target_dpi(200);

let bitmap = page.render_with_config(&config);
// bitmap 包含 RGBA 像素数据，可以直接用 image crate 处理
```

**优势**:
- 纯 Rust，无外部依赖
- 可以替代 pdftoppm 的所有调用
- 直接得到像素数据，不需要临时 PNG 文件

### Phase 4: pdf-syntax 替换 qpdf 的读取功能

**涉及文件**:
- `src-tauri/src/pdf/page_info.rs` — 页面尺寸提取
- `src-tauri/src/pdf/detection.rs` — Artifact 检测
- `src-tauri/Cargo.toml` — 添加 pdf-syntax 依赖

**页面尺寸提取**:
```rust
use pdf_syntax::Pdf;

let data = std::fs::read(path)?;
let pdf = Pdf::new(Arc::new(data))?;
for page in pdf.pages().iter() {
    let mediabox = page.media_box();
    // [llx, lly, urx, ury]
}
```

**Artifact 检测**:
pdf-syntax 可以遍历内容流操作符，查找 `/Artifact` 标记：
```rust
for op in page.typed_operations() {
    // 查找 BDC 操作符中的 Artifact 标记
    // /Artifact << /Type /Pagination /Subtype /Header >> BDC
}
```

**替代 qdf 输出**:
当前用 `qpdf --qdf` 生成线性化副本来扫描 Artifact。用 pdf-syntax 可以直接在内存中解析，不需要临时文件。

### Phase 5: pdftotext 替代（文本+bbox 提取）

**涉及文件**:
- `src-tauri/src/pdf/detection.rs` — 文本层检测

**方案**: 用 pdf-syntax 解析内容流中的文本操作符

```rust
for op in page.typed_operations() {
    match op {
        // Td/TD: 文本位置
        // Tj/TJ/'/\": 文本内容
        // Tm: 文本矩阵（包含位置和缩放）
        // BT/ET: 文本块边界
    }
}
```

**需要实现**:
- 跟踪当前变换矩阵（CTM）和文本矩阵（Tm）
- 计算每个文本操作符在页面坐标系中的位置
- 提取文本内容

**复杂度**: 中等。PDF 文本定位涉及矩阵运算，但 Docsy 的 detection 只需要判断文本是否在页面顶部/底部 12% 区域，精度要求不高。

### Phase 6: 清理外部工具依赖

**移除**:
- `src-tauri/src/external/qpdf.rs` — QpdfTool
- qpdf 二进制分发逻辑

**保留**:
- LibreOffice（DOC/DOCX 转 PDF，无纯 Rust 替代）

**Cargo.toml 变更**:
```toml
# 新增
lopdf = "0.44"
hayro = "0.7"
pdf-syntax = "0.5"

# 保留
printpdf = "0.9.1"
image = "0.25.5"

# 可移除（如果 Phase 5 完成）
# base64, regex 仍可能在其他地方使用
```

---

## 风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| overlay 合成的资源冲突 | 高 | 纯内容流合并，需要完整的资源重命名和冲突处理 |
| lopdf 合并对复杂 PDF 的兼容性 | 中 | lopdf 社区大、测试多；但需要针对证据场景的边界测试 |
| hayro 渲染质量 vs pdftoppm | 中 | hayro 是最成熟的纯 Rust 渲染器，但可能有边缘 case |
| 性能对比 | 低 | 纯 Rust 可能比子进程调用更快，但大文件的内存使用需要关注 |
| 编译时间增加 | 低 | lopdf + hayro + pdf-syntax 总计约 60K SLoC，可接受 |

## 建议的实施顺序

1. **Phase 1** (合并/拆分/解锁) — 最基础，先验证 lopdf 的可行性
2. **Phase 3** (hayro 渲染) — 独立模块，风险低
3. **Phase 2** (overlay 合成) — 最复杂，放在后面有更多上下文
4. **Phase 4** (pdf-syntax 读取) — 优化性，可以渐进替换
5. **Phase 5** (pdftotext 替代) — 与 Phase 4 一起做，完全去掉 Poppler
6. **Phase 6** (清理) — 所有功能验证后最后清理

## 验证方案

每个 Phase 完成后：
1. 用真实证据 PDF 测试合并/拆分/overlay
2. 对比纯 Rust 输出与 qpdf 输出的二进制差异（页面渲染对比）
3. 测试加密 PDF 的解锁
4. 测试 CJK 文本的页眉页脚
5. 测试 A4 归一化后的渲染质量

## 已确认的决策

1. **overlay 合成**: 采用纯内容流合并策略，对所有 PDF（包括外部来源）都尝试解析内容流合并。需要实现完整的 PDF 操作符解析和资源冲突处理。

2. **pdftotext 替代**: 一起替换，在 Phase 5 中完全去掉 Poppler 依赖。

3. **pdfium-render 退路**: 在架构设计中保留 pdfium-render 作为备选方案。如果 lopdf 的 overlay/合并遇到无法解决的兼容性问题，可以切换到 pdfium-render（静态链接）。具体做法：抽象一个 `PdfEngine` trait，lopdf 和 pdfium-render 各自实现，通过配置切换。

## 备选方案：pdfium-render 退路设计

如果 lopdf 在 overlay 合成上遇到不可解决的兼容性问题，可以切换到 pdfium-render：

```rust
trait PdfEngine {
    fn merge(&self, inputs: &[PathBuf], output: &Path) -> Result<()>;
    fn split(&self, input: &Path, output_dir: &Path) -> Result<Vec<PathBuf>>;
    fn overlay(&self, base: &Path, overlay: &Path, output: &Path) -> Result<()>;
    fn unlock(&self, input: &Path, password: &str, output: &Path) -> Result<()>;
    fn page_info(&self, input: &Path) -> Result<Vec<PageInfo>>;
}

struct LopdfEngine;      // 纯 Rust 实现
struct PdfiumEngine;     // pdfium-render 实现（需要 Pdfium 二进制）
```

pdfium-render 的优势是功能最完整，overlay/合并/拆分都有成熟 API。代价是需要 Pdfium 二进制（约 10-15MB）。可以通过 `bblanchon/pdfium-binaries` 获取预编译版本，静态链接到 Tauri 应用中。

这个退路可以在 Phase 2 的 overlay 实现遇到问题时再考虑引入。
