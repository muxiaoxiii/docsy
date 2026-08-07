# Docsy Bug 报告 / Bug Report

> 审计日期: 2026-08-07  
> 项目路径: `/Users/only/Documents/PythonProgram/Docsy`

---

## 严重程度说明
- 🔴 **P0 (Critical)**: 可能导致崩溃、数据丢失或安全问题
- 🟠 **P1 (High)**: 功能性 Bug，影响用户体验
- 🟡 **P2 (Medium)**: 代码异味、潜在问题
- 🟢 **P3 (Low)**: 改进建议、风格问题

---

## 🔴 P0 — Critical

### BUG-001: `cancel_operation` 忽略 `operation_id`，杀死所有子进程
**文件**: `src-tauri/src/commands/system.rs:138-145`
```rust
pub fn cancel_operation(
    registry: tauri::State<'_, std::sync::Arc<crate::SubprocessRegistry>>,
    _operation_id: String,  // ← 参数被忽略！
) -> Result<bool, String> {
    registry.cancel_all();  // ← 杀死所有进程
    Ok(true)
}
```
**问题**: 前端传入特定 `operation_id` 想取消单个操作，但后端忽略了该参数，调用 `cancel_all()` 杀死所有注册的子进程。如果用户同时有两个操作运行（如后台批量渲染 + 前台预览），取消一个会杀死两个。
**影响**: 非预期地中断其他正在运行的操作。
**修复建议**: 使用 `registry.cancel(&operation_id)` 替代 `cancel_all()`。

---

### BUG-002: `MANIFEST_CACHE` 使用 `Mutex::unwrap()` 可能 panic
**文件**: `src-tauri/src/commands/template.rs:16,32`
```rust
if let Some((cached_path, cached_mtime, manifest)) = MANIFEST_CACHE.lock().unwrap().as_ref() {
    // ...
}
*MANIFEST_CACHE.lock().unwrap() = Some((path.to_string(), mtime, manifest.clone()));
```
**问题**: 两处 `.unwrap()` 在 Mutex 中毒（poison）时会 panic，导致整个 Tauri 命令崩溃。虽然 Mutex 中毒在实际中较少见（通常发生在另一个线程 panic 时），但对桌面应用来说应该优雅处理。
**影响**: 极端情况下命令 panic。
**修复建议**: 使用 `.lock().unwrap_or_else(|e| e.into_inner())` 或返回错误。

---

### BUG-003: `ConversionState::wait_for_user_response` 使用忙等待轮询
**文件**: `src-tauri/src/lib.rs:158-180`
```rust
pub fn wait_for_user_response(&self, app: &tauri::AppHandle) -> bool {
    // ...
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        match self.response.load(Ordering::SeqCst) {
            1 => { return true; }
            2 => { return false; }
            _ => continue,
        }
    }
}
```
**问题**: 使用 `thread::sleep` 轮询等待用户响应。这会阻塞调用线程（虽然是 `spawn_blocking` 线程池），且 500ms 轮询间隔意味着用户响应后最多有 500ms 延迟。
**影响**: 资源浪费、响应延迟。
**修复建议**: 使用 `std::sync::Condvar` 或 `tokio::sync::Notify` 替代忙等待。

---

## 🟠 P1 — High

### BUG-004: `same_path` 函数在多个模块中重复定义，行为不一致
**文件**:
- `pdf/annotations.rs:174-176` — 使用 `comparable_path` (normalize + fallback)
- `pdf/artifacts.rs:340-345` — 使用 `canonicalize` + fallback `==`
- `pdf/evidence_session.rs:340-344` — 使用 `canonicalize` + fallback `==`

**问题**: 三个模块各自实现了 `same_path` 逻辑，行为不一致。`annotations.rs` 的版本更健壮（规范化路径组件），而 `artifacts.rs` 和 `evidence_session.rs` 在 `canonicalize` 失败时直接用原始路径比较，可能在相对路径场景下误判。
**影响**: 路径比较结果可能因模块不同而不同。
**修复建议**: 提取为公共函数放在 `mod.rs` 或工具模块中。

---

### BUG-005: `temp_named_path` 函数在 4 个模块中重复定义
**文件**:
- `pdf/preview.rs:144-151`
- `pdf/annotations.rs:206-213`
- `pdf/split.rs:231-238`
- `pdf/normalize.rs:200-207`

**问题**: 四处独立实现了几乎相同的临时文件路径生成逻辑。虽然功能正确，但违反 DRY 原则，且签名略有不同（有的带 `extension` 参数，有的不带）。
**影响**: 维护负担，修改时容易遗漏。
**修复建议**: 提取为 `pdf/mod.rs` 中的公共函数。

---

### BUG-006: `header_footer.rs` 中遗留 debug `eprintln!`
**文件**: `src-tauri/src/pdf/header_footer.rs:238-244`
```rust
// Debug: log bookmark data for the first item
if let Some(first) = items.first() {
    let bm = first.get("bookmarks");
    let bm_rm = first.get("bookmarkRemoveExisting");
    eprintln!(
        "[batch_overlay] first item bookmarks={:?}, bookmarkRemoveExisting={:?}",
        bm, bm_rm
    );
}
```
**问题**: 生产代码中遗留了调试输出到 stderr。在 Tauri 应用中 stderr 输出会被写入日志文件，造成噪音。
**影响**: 日志污染。
**修复建议**: 删除或改为 `log::debug!` / `tracing::debug!`。

---

### BUG-007: `preview_image_data_url` 中 `mime` 变量被计算但从未使用
**文件**: `src-tauri/src/commands/system.rs:67-88`
```rust
let mime = match ext.as_str() {
    "jpg" | "jpeg" => "image/jpeg",
    // ...
};
// ... 生成 JPEG 缩略图 ...
let _ = mime;  // ← 显式忽略
Ok(format!("data:image/jpeg;base64,{encoded}"))
```
**问题**: `mime` 变量被计算后显式忽略（`let _ = mime`），硬编码返回 `image/jpeg`。原始格式信息丢失，虽然功能上缩略图统一为 JPEG 是合理的，但代码意图不明确。
**影响**: 代码异味。
**修复建议**: 删除 `mime` 变量和匹配逻辑，直接返回 `image/jpeg`。

---

### BUG-008: `evidence.rs` 中 `fnv1a_hash` 重复定义
**文件**:
- `docx_template/mod.rs:610-617` — 模块级 `pub(super) fn fnv1a_hash`
- `pdf/evidence.rs:140-147` — 本地 `fn fnv1a_hash`

**问题**: 两处独立实现了相同的 FNV-1a 哈希函数。
**影响**: 维护负担。
**修复建议**: 提取为公共工具函数。

---

### BUG-009: `apply_anti_copy` 中 `TextOverlay` 方法实际执行的是 `CmapScramble`
**文件**: `src-tauri/src/pdf/anti_ocr.rs:125-141`
```rust
AntiCopyMethod::TextOverlay => {
    // TextOverlay: CMap removal + overlay is done in JS/Python layer
    // because lopdf doesn't easily support adding content streams
    for font_id in page_fonts(&doc, *page_id) {
        // ... 执行的是 scramble 而非 overlay ...
        let scrambled = scramble_cmap(&cmap_text);
        set_tounicode_cmap(&mut doc, font_id, scrambled.as_bytes());
    }
}
```
**问题**: `TextOverlay` 方法的注释说"CMap removal + overlay is done in JS/Python layer"，但实际代码执行的是 `CmapScramble` 逻辑（篡改 CMap），而非真正的文字覆盖。这意味着 `TextOverlay` 和 `CmapScramble` 产生相同的结果。
**影响**: 用户选择"文字覆盖"防复制方法时，实际得到的是 CMap 篡改，可能不如预期有效。
**修复建议**: 要么实现真正的文字覆盖，要么在 UI 中移除该选项或明确说明实际行为。

---

### BUG-010: `batch_overlay` 中首次 item 的 bookmark 调试输出对所有请求执行
**文件**: `src-tauri/src/pdf/header_footer.rs:238-244`
**问题**: 每次调用 `batch_overlay` 都会检查第一个 item 并输出调试信息，即使这不是调试模式。
**影响**: 性能微损 + 日志噪音（同 BUG-006）。

---

## 🟡 P2 — Medium

### BUG-011: `list_template_library_items` 中读取 manifest 失败时静默跳过
**文件**: `src-tauri/src/docx_template/mod.rs:383-385`
```rust
let Ok(manifest) = read_template_manifest(&path) else {
    continue;  // ← 静默跳过损坏的模板
};
```
**问题**: 如果 `.docsytpl` 文件损坏（ZIP 损坏、manifest 缺失等），该模板会被静默跳过，用户无法知道为什么模板不显示。
**影响**: 用户困惑（"我的模板去哪了？"）。
**修复建议**: 收集警告信息，在返回结果中附加。

---

### BUG-012: `sanitize_manifest_private_labels` 可能将合法标签替换为字段 ID
**文件**: `src-tauri/src/docx_template/mod.rs:299-321`
```rust
if generated_field_name(&field.name) || marked.contains(field.label.trim()) {
    field.label = fallback.to_string();
}
```
**问题**: 如果用户在 Word 中标记的文本恰好与某个字段的 label 相同（如标记"原告"，字段 label 也是"原告"），该字段的 label 会被替换为 name/id，即使这是用户有意设置的。
**影响**: 字段标签被意外重置。
**修复建议**: 添加更精确的判断条件，例如只在 label 等于标记文本且 name 也是生成名称时才替换。

---

### BUG-013: `import_template_to_library` 不检查目标是否已存在
**文件**: `src-tauri/src/commands/template.rs:295-306`
```rust
pub async fn import_template_to_library(source_path: String) -> Result<String, String> {
    run_blocking(move || {
        let dest = crate::docx_template::template_library_dir().join(file_name);
        std::fs::copy(source, &dest)?;  // ← 覆盖已有文件
        Ok(dest.display().to_string())
    }).await
}
```
**问题**: 如果目标路径已存在同名模板，会被静默覆盖，丢失用户的字段配置和历史数据。
**影响**: 数据丢失。
**修复建议**: 使用 `unique_docx_output_path` 或检查冲突后提示用户。

---

### BUG-014: `preview_overlay` 中临时文件可能在错误路径下残留
**文件**: `src-tauri/src/pdf/header_footer.rs:291-311`
```rust
let preview_output = temp_named_path("docsy_hf_preview", "pdf");
job.output_path = preview_output.to_string_lossy().to_string();
// ... 执行 ...
let _ = fs::remove_file(&preview_output);  // ← 忽略删除失败
cleanup_temp(annotation_temp);
```
**问题**: 如果 `process_job` 或 `render_preview` panic（虽然 unlikely），临时文件不会被清理。使用 RAII guard（如 `TempPathGuard`）会更安全。
**影响**: 磁盘空间泄漏（极端情况）。
**修复建议**: 使用 `TempPathGuard` 模式（已在 `engine.rs` 中使用）。

---

### BUG-015: `ScrambleCmap` 中 PUA 范围判断可能误判
**文件**: `src-tauri/src/pdf/anti_ocr.rs:358-378`
```rust
fn has_pua_mappings(cmap_text: &str) -> bool {
    // ...
    total_mappings > 5 && pua_hits > total_mappings / 3
}
```
**问题**: PUA 检测使用简单的阈值（>5 个映射且 >1/3 命中 PUA），但 CMap 中的十六进制解析是基于 `<` 和 `>` 的简单分割，可能在包含多行映射或复杂格式的 CMap 中误判。
**影响**: 防复制检测可能误报或漏报。

---

### BUG-016: `scan_document_index` 中文本框内的段落可能被重复计算
**文件**: `src-tauri/src/docx_template/scan.rs:39-56`
```rust
fn scan_element_children_recursive(children: &[XmlNode], index: &mut TextIndex, _path: &mut Vec<String>) {
    for node in children {
        if let XmlNode::Element { name, children, .. } = node {
            if name == "w:p" {
                index.add_paragraph();
                // ... 扫描段落 run ...
            }
            // Always recurse into all children
            scan_element_children_recursive(children, index, _path);
        }
    }
}
```
**问题**: 当遇到 `w:p` 时，先添加段落并扫描 run，然后递归进入 children。如果 `w:p` 的 children 中还嵌套了 `w:p`（如文本框），内层段落会被正确处理。但如果 XML 结构是 `w:body > w:p > w:r > w:drawing > w:txbxContent > w:p`，外层段落的 run 已经被扫描，内层段落也会被扫描——这是正确的行为。经验证代码逻辑正确，此条降级为非 Bug。
**影响**: 无（代码正确但逻辑复杂，值得注释说明）。

---

### BUG-017: 前端 `cancelCurrentOperation` 清除所有 pending operations
**文件**: `src/App.vue:167-185`
```javascript
async function cancelCurrentOperation() {
    const firstId = Array.from(pendingOperations.keys()).at(0)
    if (firstId) {
        await tauriCallSafe('cancel_operation', { operationId: firstId })
    }
    pendingOperations.clear()  // ← 清除所有
    // ...
}
```
**问题**: 与 BUG-001 联动。前端取消时传入了正确的 `operationId`，但后端杀死所有进程；前端也清除所有 pending 状态。即使后端修复了 BUG-001，前端的 `pendingOperations.clear()` 仍然会清除所有操作状态。
**影响**: 取消一个操作会导致所有操作的 UI 状态丢失。
**修复建议**: 只删除被取消的 operationId。

---

### BUG-018: `import_template_to_library` 不处理文件名冲突
**文件**: `src-tauri/src/commands/template.rs:295-306`
**问题**: 同 BUG-013。导入模板时如果库中已有同名文件，直接覆盖。

---

## 🟢 P3 — Low / Improvements

### STYLE-001: `ConversionState` 使用 `SeqCst` 内存序
**文件**: `src-tauri/src/lib.rs:132-154`
**问题**: 所有原子操作都使用 `Ordering::SeqCst`，对于简单的标志位读写，`Ordering::Relaxed` 或 `Ordering::Acquire`/`Release` 通常足够且性能更好。
**影响**: 性能微损（可忽略）。

---

### STYLE-002: `docx_template/mod.rs` 过大（644 行）
**问题**: 该文件包含类型定义、验证、库管理、工具函数等多个职责。
**建议**: 将类型定义拆分到 `types.rs`，验证逻辑拆分到 `validate.rs`。

---

### STYLE-003: `detection.rs` 过大（2645 行）
**问题**: 单文件包含检测、建议、候选聚合等多个复杂逻辑。
**建议**: 拆分为 `detect/mod.rs`, `detect/candidates.rs`, `detect/split_suggestions.rs`。

---

### STYLE-004: `header_footer.rs` 过大（2394 行）
**问题**: 包含 overlay、cleanup、bookmark、preview 等多个子功能。
**建议**: 拆分为 `overlay.rs`, `cleanup.rs`, `bookmark.rs`。

---

### STYLE-005: `image_paddler.rs` 过大（1587 行）
**问题**: 包含分析、排版、文件名处理等多个子功能。
**建议**: 拆分为 `analyze.rs`, `layout.rs`, `filename.rs`。

---

### STYLE-006: `template_history.rs` 过大（947 行）
**问题**: 包含数据库操作、历史查询、建议生成等多个子功能。
**建议**: 拆分为 `db.rs`, `suggestions.rs`, `runs.rs`。

---

### STYLE-007: 多处使用 `serde_json::Value` 作为函数参数
**文件**: `pdf/detection.rs`, `pdf/evidence_session.rs`, `pdf/header_footer.rs` 等
**问题**: 许多函数接受 `&serde_json::Value` 而非具体的结构体类型，然后在函数内部反序列化。这绕过了 Rust 的类型系统。
**建议**: 使用具体的 Args 结构体作为参数类型。

---

### STYLE-008: `module_registry.rs` (Rust) 的 `all_descriptors` 未被使用
**文件**: `src-tauri/src/services/module_registry.rs:3`
```rust
#[allow(dead_code)]
pub fn all_descriptors() -> Vec<serde_json::Value> {
```
**问题**: 该函数被标记为 `#[allow(dead_code)]`，说明它当前没有被调用。前端的模块注册是通过 `moduleRegistry.js` 的 `import.meta.glob` 实现的，与 Rust 端的描述符无关。
**建议**: 如果不需要从 Rust 端查询模块信息，可以删除此文件。

---

### STYLE-009: `#[allow(dead_code)]` 在 `index.rs` 中多处使用
**文件**: `src-tauri/src/docx_template/index.rs`
```rust
#[allow(dead_code)] // retained for diagnostics
pub part_name: String,

#[allow(dead_code)]
pub fn total_text_nodes(&self) -> usize { ... }

#[allow(dead_code)]
pub fn iter_nodes(&self) -> impl Iterator<Item = (&str, &TextNodeRef)> { ... }

#[allow(dead_code)]
pub fn iter_highlighted(&self) -> impl Iterator<Item = (&str, &TextNodeRef)> { ... }
```
**问题**: 多个成员和方法被标记为 dead code 但保留用于"诊断"。
**建议**: 如果确实需要，可以使用 `#[cfg(test)]` 或 feature flag 而非全局 `allow`。

---

### STYLE-010: `preview_image_data_url` 中硬编码最大预览尺寸
**文件**: `src-tauri/src/commands/system.rs:58-59`
```rust
const MAX_PREVIEW_EDGE: u32 = 1600;
const MAX_SOURCE_PIXELS: u64 = 64_000_000;
```
**问题**: 魔法数字，没有配置选项。
**建议**: 可以从设置中读取或作为参数传入。

---

### STYLE-011: 前端 `operationLabels` 硬编码在 `tauriBridge.js`
**文件**: `src/core/tauriBridge.js:4-26`
**问题**: 所有操作标签硬编码在桥接层，新增命令需要修改此处。
**建议**: 可以从模块注册中动态获取，或使用 i18n。

---

## 潜在问题 / Potential Issues

### POTENTIAL-001: ZIP 解压炸弹防护
**位置**: `docx_template/package.rs`
**评估**: 已有良好防护 — `MAX_ZIP_ENTRIES` (4096)、`MAX_PACKAGE_UNCOMPRESSED_BYTES` (1GB)、`MAX_XML_ENTRY_BYTES` (128MB)、TOCTOU-safe bounded read。✅

### POTENTIAL-002: SQL 注入
**位置**: `template_history.rs`
**评估**: 所有 SQL 使用参数化查询 (`params![]`)，无字符串拼接 SQL。✅
**例外**: `query_run_field_summaries_for_runs` 使用 `format!` 构建 `IN (...)` 占位符，但值是 `?` 占位符而非用户输入，安全。✅

### POTENTIAL-003: 路径遍历
**位置**: `import_template_to_library`, `export_templates`
**评估**: 使用 `file_name()` 提取文件名（不含路径分隔符），安全。✅

### POTENTIAL-004: 子进程命令注入
**位置**: `external/*.rs`, `pdf/qpdf.rs`
**评估**: 使用 `Command::new().arg()` 而非 shell 拼接，安全。✅

### POTENTIAL-005: `list_generation_runs` 中的孤儿清理可能影响性能
**位置**: `template_history.rs:327-343`
```rust
for run in all_runs {
    if !run.template_path.is_empty() && !std::path::Path::new(&run.template_path).exists() {
        orphaned_ids.push(run.template_id.clone());
    }
}
```
**问题**: 每次列出历史时都遍历所有记录并检查文件是否存在。如果历史记录很多且文件在远程/慢速磁盘上，可能造成延迟。
**建议**: 使用 lazy 清理或定时清理。

---

## 总结

| 严重程度 | 数量 | 说明 |
|----------|------|------|
| 🔴 P0 Critical | 3 | cancel_all、Mutex unwrap、忙等待 |
| 🟠 P1 High | 7 | same_path 重复、debug 输出、TextOverlay 误导、模板覆盖 |
| 🟡 P2 Medium | 8 | 静默跳过、标签误替换、临时文件泄漏 |
| 🟢 P3 Low | 11 | 代码风格、文件过大、魔数字 |
| ✅ 安全 | 5 | ZIP 防护、SQL 注入、路径遍历、命令注入均安全 |

**整体评估**: 代码质量较高，架构清晰，模块化良好。主要问题集中在：
1. 代码重复（`same_path`, `temp_named_path`, `fnv1a_hash`）
2. 部分文件过大（>1000 行）
3. 少数功能性 Bug（cancel_operation、模板覆盖）
4. 错误处理可以更一致（Mutex unwrap、静默跳过）
