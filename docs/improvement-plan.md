# Docsy v0.9.6 完善方案

> 生成日期：2026-08-07
> 基于：review-summary / bug-report / deep-review-* / ui-review / design-vs-implementation 共 7 份审阅报告
> 技术栈：Tauri 2 + Rust + Vue 3 + Element Plus

---

## 一、Bug 修复方案（按优先级）

### 🔴 P0 — 立即修复

---

#### P0-1: `cancel_operation` 忽略 `operation_id`，杀死所有子进程

**问题根因**：`src-tauri/src/commands/system.rs:137-145` 中 `_operation_id` 参数被忽略，直接调用 `registry.cancel_all()`。同时 `src/App.vue:167-185` 前端 `cancelCurrentOperation` 也调用 `pendingOperations.clear()` 清除所有操作状态。

**修复方案**：

Rust 端 `src-tauri/src/commands/system.rs`：
```rust
#[tauri::command]
pub fn cancel_operation(
    registry: tauri::State<'_, std::sync::Arc<crate::SubprocessRegistry>>,
    operation_id: String,
) -> Result<bool, String> {
    // 只取消指定操作，而非所有进程
    Ok(registry.cancel(&operation_id))
}
```

前端 `src/App.vue`：
```javascript
async function cancelCurrentOperation() {
    const firstId = Array.from(pendingOperations.keys()).at(0)
    if (firstId) {
        await tauriCallSafe('cancel_operation', { operationId: firstId })
        pendingOperations.delete(firstId)  // 只删除被取消的那一个
    }
    // 删除: pendingOperations.clear()
}
```

**影响范围**：所有使用 `SubprocessRegistry` 的后端操作（PDF 合并、批量渲染、视频抽帧等）。

**回归测试**：
1. 启动两个并发操作（如批量渲染 + PDF 预览），取消其中一个，验证另一个继续运行
2. 取消不存在的 operationId，验证返回 `false` 而非 panic
3. 快速连续取消同一操作，验证幂等性

---

#### P0-2: `MANIFEST_CACHE` 使用 `Mutex::unwrap()` 可能 panic

**问题根因**：`src-tauri/src/commands/template.rs:16,32` 两处 `MANIFEST_CACHE.lock().unwrap()` 在 Mutex 中毒时会 panic，导致整个 Tauri 命令崩溃。

**修复方案**：

`src-tauri/src/commands/template.rs`：
```rust
fn cached_manifest(
    path: &str,
) -> anyhow::Result<crate::docx_template::TemplateManifest> {
    // unwrap_or_else 恢复中毒的锁，而非 panic
    if let Some((cached_path, cached_mtime, manifest)) =
        MANIFEST_CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
    {
        if cached_path == path {
            if let Ok(meta) = std::fs::metadata(path) {
                if let Ok(mtime) = meta.modified() {
                    if mtime == *cached_mtime {
                        return Ok(manifest.clone());
                    }
                }
            }
        }
    }
    let manifest = crate::docx_template::inspect_template_package(path)?;
    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    *MANIFEST_CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
        Some((path.to_string(), mtime, manifest.clone()));
    Ok(manifest)
}
```

**影响范围**：模板缓存，低风险——只影响极端异常路径。

**回归测试**：
1. 模拟 Mutex 中毒场景（在其他线程 panic 后调用 `cached_manifest`）
2. 正常缓存命中/未命中路径不变

---

#### P0-3: `ConversionState::wait_for_user_response` 使用忙等待轮询

**问题根因**：`src-tauri/src/lib.rs:158-180` 使用 `thread::sleep(500ms)` 循环轮询用户响应，阻塞调用线程且有最高 500ms 延迟。

**修复方案**：

`src-tauri/src/lib.rs` — 将 `response` 从 `AtomicU8` 改为 `Condvar` + `Mutex<u8>`：
```rust
use std::sync::{Arc, Condvar, Mutex};

pub struct ConversionState {
    pub timed_out: AtomicBool,
    /// 0=waiting, 1=continue, 2=cancel. Protected by Condvar.
    pub response: Mutex<u8>,
    pub cv: Condvar,
    pub pid: AtomicU64,
}

impl ConversionState {
    pub fn new() -> Self {
        Self {
            timed_out: AtomicBool::new(false),
            response: Mutex::new(0),
            cv: Condvar::new(),
            pid: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.timed_out.store(false, Ordering::Relaxed);
        *self.response.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        self.pid.store(0, Ordering::Relaxed);
    }

    /// 前端通过 respond_conversion_timeout 设置响应
    pub fn set_response(&self, value: u8) {
        let mut guard = self.response.lock().unwrap_or_else(|e| e.into_inner());
        *guard = value;
        self.cv.notify_all();
    }

    /// 后端调用：信号超时并阻塞等待用户响应（毫秒级唤醒）
    pub fn wait_for_user_response(&self, app: &tauri::AppHandle) -> bool {
        self.timed_out.store(true, Ordering::Relaxed);
        *self.response.lock().unwrap_or_else(|e| e.into_inner()) = 0;

        let _ = app.emit("docsy-conversion-timeout", ());

        let mut guard = self.response.lock().unwrap_or_else(|e| e.into_inner());
        while *guard == 0 {
            guard = self.cv.wait_timeout(
                guard,
                std::time::Duration::from_secs(5),
            ).unwrap_or_else(|e| e.into_inner()).0;
        }
        let result = *guard == 1;
        self.timed_out.store(false, Ordering::Relaxed);
        result
    }
}
```

对应修改 `src-tauri/src/commands/system.rs` 中 `respond_conversion_timeout`：
```rust
pub fn respond_conversion_timeout(
    state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
    continue_waiting: bool,
) -> Result<(), String> {
    state.set_response(if continue_waiting { 1 } else { 2 });
    Ok(())
}
```

**影响范围**：COM 转换超时流程（Word→PDF）。低频路径。

**回归测试**：
1. 触发 COM 转换超时，前端弹出提示后点击"继续"/"取消"，验证响应延迟 < 100ms
2. 超时后关闭窗口，验证不会死锁（5s 超时唤醒兜底）

---

#### P0-4: 书签页码偏移错位（`collect_merge_bookmarks` zip 静默截断）

**问题根因**：`src-tauri/src/pdf/evidence_session.rs:295-322` 中 `items.iter().zip(results.iter())` 假设 items 和 results 长度一致。当某文件处理失败导致 results 缺少元素时，zip 静默截断，后续文件的书签页码偏移错位。

**修复方案**：

`src-tauri/src/pdf/evidence_session.rs`：
```rust
fn collect_merge_bookmarks(items: &[Value], results: &[Value]) -> Vec<header_footer::BookmarkConfig> {
    let mut bookmarks = Vec::new();
    let mut page_offset: u32 = 0;

    // 用索引遍历 items，通过 output_path 在 results 中查找匹配项
    for (idx, item) in items.iter().enumerate() {
        let item_output = item.get("output_path")
            .and_then(Value::as_str)
            .unwrap_or("");

        // 在 results 中查找匹配的输出
        let pages = results.iter()
            .find(|r| {
                r.get("outputPath").and_then(Value::as_str) == Some(item_output)
                    || r.get("index").and_then(Value::as_u64) == Some(idx as u64)
            })
            .and_then(|r| r.get("pages").and_then(Value::as_u64))
            .unwrap_or(0) as u32;

        if pages == 0 {
            // 此文件处理失败，跳过其书签但记录警告
            log::warn!("书签计算：第 {} 个文件处理结果缺失，跳过其书签", idx);
            continue;
        }

        if let Some(bms) = item.get("bookmarks").and_then(Value::as_array) {
            for bm_value in bms {
                if let Ok(bm) = serde_json::from_value::<header_footer::BookmarkConfig>(bm_value.clone()) {
                    if bm.enabled && !bm.label.is_empty() {
                        let mut adjusted = bm;
                        adjusted.page_index += page_offset;
                        bookmarks.push(adjusted);
                    }
                }
            }
        }

        page_offset += pages;
    }

    bookmarks
}
```

**影响范围**：合并证据 PDF 时的书签写入。影响数据准确性。

**回归测试**：
1. 构造 3 个文件，中间一个处理失败，验证第 3 个文件的书签页码偏移正确
2. 所有文件成功时行为与原来一致
3. 全部失败时返回空书签列表

---

#### P0-5: 书签写入非原子操作，崩溃可能丢失原 PDF

**问题根因**：`src-tauri/src/pdf/header_footer.rs:324-372` 中 `apply_bookmark` 先写临时文件再 `fs::copy` 回原文件。如果进程在 `fs::copy` 期间崩溃，原 PDF 可能被损坏。

**修复方案**：

`src-tauri/src/pdf/header_footer.rs` — 使用备份+原子 rename 策略：
```rust
fn apply_bookmark(output: &Path, config: &BookmarkConfig) -> Result<()> {
    if !config.enabled || config.label.is_empty() {
        return Ok(());
    }

    // 1. 创建原文件备份
    let backup = output.with_extension("pdf.bak");
    fs::copy(output, &backup).context("创建书签备份失败")?;

    // 2. 写入新内容到临时文件
    let temp = temp_named_path("docsy_bookmark", "pdf");
    let mut doc = Document::load(output).context("加载 PDF 以写入书签失败")?;
    // ... (原有书签构建逻辑不变) ...
    doc.save(&temp).context("保存书签 PDF 失败")?;

    // 3. 原子替换：先 rename 备份，再 rename 新文件
    // 如果在此处崩溃，backup 保留了原文件
    fs::rename(output, &backup).context("备份原 PDF 失败")?;
    if let Err(e) = fs::rename(&temp, output) {
        // 替换失败，尝试恢复备份
        let _ = fs::rename(&backup, output);
        return Err(anyhow::anyhow!("写入书签失败，已恢复原文件: {}", e));
    }

    // 4. 成功后清理备份
    let _ = fs::remove_file(&backup);
    Ok(())
}
```

对 `apply_bookmarks`（多书签版本）应用相同策略。

**影响范围**：所有书签写入操作。

**回归测试**：
1. 正常写入书签后原 PDF 完好
2. 模拟写入失败（如磁盘满），验证原 PDF 被恢复
3. 写入后备份文件被正确清理

---

#### P0-6: 输出路径碰撞可能覆盖数据

**问题根因**：`src-tauri/src/pdf/header_footer.rs:726-752` 中 `unique_output_path` 在 10,000 个后缀用尽后返回原始已存在路径，后续写入将覆盖已有数据。

**修复方案**：

```rust
fn unique_output_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("output");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for index in 1..10_000 {
        let name = if extension.is_empty() {
            format!("{stem}-{index}")
        } else {
            format!("{stem}-{index}.{extension}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    // 用尽后报错，而非静默覆盖
    anyhow::bail!(
        "输出路径 {} 的所有候选名已被占用（共尝试 10000 个变体）。请清理输出目录后重试。",
        path.display()
    )
}
```

同步修改所有调用处从 `fn unique_output_path(path: &Path) -> PathBuf` 改为 `fn unique_output_path(path: &Path) -> Result<PathBuf>` 并处理错误。

同步修复 `src-tauri/src/pdf/qpdf.rs:230-241` 中的 `unique_available_path`，同样的问题。

**影响范围**：所有 PDF 输出路径生成。

**回归测试**：
1. 输出目录中已有同名文件，验证生成 `-1` 后缀
2. 模拟 10,000 个文件已存在，验证返回错误而非覆盖

---

### 🟠 P1 — 本周修复

---

#### P1-1: 统一 `same_path` 为公共函数

**问题根因**：三处独立实现行为不一致：
- `pdf/annotations.rs:174-176` — `comparable_path`（normalize + fallback）
- `pdf/artifacts.rs:340-345` — `canonicalize` + fallback `==`
- `pdf/evidence_session.rs:340-344` — `canonicalize` + fallback `==`

**修复方案**：

在 `src-tauri/src/pdf/mod.rs` 中新增公共函数（采用最健壮的 `comparable_path` 方案）：
```rust
/// 跨平台路径比较，处理大小写不敏感（Windows）和 symlink 场景
pub fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    comparable_path(a) == comparable_path(b)
}

fn comparable_path(p: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(p)
        .unwrap_or_else(|_| p.to_path_buf())
        .components()
        .collect()
}
```

删除三处私有实现，改为 `use super::same_path;`。

---

#### P1-2: 统一 `temp_named_path` 为公共函数

**问题根因**：7 处重复定义（`header_footer.rs`、`annotations.rs`、`artifacts.rs`、`split.rs`、`normalize.rs`、`content_text.rs`、`preview.rs`），且 PID+毫秒时间戳碰撞风险。

**修复方案**：

在 `src-tauri/src/pdf/mod.rs` 中新增：
```rust
use std::sync::atomic::{AtomicU32, Ordering};

static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// 生成临时文件路径，使用进程ID+单调递增计数器+纳秒时间戳，避免碰撞
pub fn temp_named_path(prefix: &str, extension: &str) -> std::path::PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let name = format!("{prefix}_{pid}_{ts}_{counter}.{extension}");
    std::env::temp_dir().join(name)
}

/// RAII guard，Drop 时自动删除临时文件
pub struct TempPathGuard {
    path: Option<std::path::PathBuf>,
}

impl TempPathGuard {
    pub fn new(prefix: &str, extension: &str) -> Self {
        Self { path: Some(temp_named_path(prefix, extension)) }
    }

    pub fn path(&self) -> &std::path::Path {
        self.path.as_ref().unwrap()
    }

    /// 消费 guard 但不删除文件（用于 rename 场景）
    pub fn keep(mut self) -> std::path::PathBuf {
        self.path.take().unwrap()
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(&path);
        }
    }
}
```

删除 7 处私有实现，改为 `use super::{temp_named_path, TempPathGuard};`。

---

#### P1-3: 统一 `fnv1a_hash` 为公共函数

**问题根因**：`docx_template/mod.rs:610-617` 和 `pdf/evidence.rs:140-147` 重复定义。

**修复方案**：

在 `src-tauri/src/sort_utils.rs`（已有的工具模块）中新增：
```rust
/// FNV-1a 32-bit 哈希
pub fn fnv1a_hash(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}
```

删除两处私有实现，改为 `use crate::sort_utils::fnv1a_hash;`。

---

#### P1-4: 清理 `eprintln!` 调试输出

**问题根因**：`src-tauri/src/pdf/header_footer.rs:238-244` 生产代码遗留 `eprintln!`。

**修复方案**：

```rust
// 删除以下代码块（L238-244）：
// if let Some(first) = items.first() {
//     let bm = first.get("bookmarks");
//     let bm_rm = first.get("bookmarkRemoveExisting");
//     eprintln!("[batch_overlay] first item bookmarks={:?}, bookmarkRemoveExisting={:?}", bm, bm_rm);
// }
```

---

#### P1-5: 修正 `TextOverlay` 方法命名误导

**问题根因**：`src-tauri/src/pdf/anti_ocr.rs:125-141` 中 `AntiCopyMethod::TextOverlay` 实际执行 `CmapScramble` 逻辑。

**修复方案**：

方案 A（推荐）— 合并枚举并重命名：
```rust
// src-tauri/src/pdf/anti_ocr.rs
pub enum AntiCopyMethod {
    CmapScramble,  // 篡改 CMap 映射
    CmapRemove,    // 移除 ToUnicode CMap
    // 删除 TextOverlay，因为它和 CmapScramble 行为相同
}
```

对应修改前端 `PdfToolsView.vue` 中的防复制方法选择器，移除"文字覆盖"选项，或将其映射到 `CmapScramble` 并在 tooltip 中说明。

方案 B — 如果要保留 UI 选项名称，重命名为准确的描述：
```rust
pub enum AntiCopyMethod {
    CmapScramble,      // 基础混淆
    CmapRemove,        // 完全移除
    CmapScrambleAlt,   // 替代混淆（原 TextOverlay，相同行为但名称准确）
}
```

---

#### P1-6: 前后端默认值对齐（`margin_mm` 等）

**问题根因**：`src-tauri/src/image_paddler.rs:523` 中 `margin_mm` 默认 `15.0`，但前端推荐 `12.0`；`filename_without_ext` 后端默认 `false`，前端默认 `true`。

**修复方案**：

`src-tauri/src/image_paddler.rs`：
```rust
// L523: 修改默认值与前端一致
let margin_mm = args.margin_mm.unwrap_or(12.0);  // 原为 15.0

// L525: 修改默认值与前端一致
let filename_without_ext = args.filename_without_ext.unwrap_or(true);  // 原为 false
```

---

#### P1-7: 修复 `managed.rs` macOS/Linux SHA256 为空

**问题根因**：`src-tauri/src/external/managed.rs:212-229` 中 `embedded_package_spec` 非 Windows 平台返回空 SHA256，导致 `has_sha256` 检查失败，嵌入式配置永远不生效。

**修复方案**：

方案 A — 为 macOS/Linux 嵌入式配置提供 SHA256：
```rust
fn embedded_package_spec(name: &str, platform: &str) -> Option<ToolPackage> {
    if platform == "windows-x86_64" {
        return embedded_windows_package_spec(name);
    }

    let (version, sha256, binaries) = match (name, platform) {
        ("qpdf", "darwin-aarch64") => (
            "12.2.0",
            "", // TODO: 填入实际 SHA256
            required_binaries("qpdf").ok()?,
        ),
        ("qpdf", "darwin-x86_64") => (
            "12.2.0",
            "", // TODO: 填入实际 SHA256
            required_binaries("qpdf").ok()?,
        ),
        ("ffmpeg", "darwin-aarch64") => (
            "8.1",
            "", // TODO: 填入实际 SHA256
            required_binaries("ffmpeg").ok()?,
        ),
        // ... 其他工具/平台组合
        _ => return None,
    };
    Some(ToolPackage {
        version: version.to_string(),
        url: format!("{DEFAULT_RELEASE_BASE}/{name}-{version}-{platform}.zip"),
        sha256: sha256.to_string(),
        max_bytes: None,
        binaries,
    })
}
```

方案 B（最小改动）— 如果暂时无法获取 SHA256，修改 `load_package_spec` 中的检查逻辑允许嵌入式配置跳过 SHA256：
```rust
// 在 load_package_spec 中，对嵌入式配置不做 has_sha256 要求
if spec.sha256.is_empty() && !is_embedded {
    continue; // 仅跳过缺少 SHA256 的非嵌入式配置
}
```

---

#### P1-8: `apply_anti_copy` 未知 method 静默回退到 `CmapScramble`

**问题根因**：`src-tauri/src/commands/pdf.rs:269-273` 中未知 method 字符串静默回退。

**修复方案**：

```rust
let method = match args.method.as_deref() {
    Some("cmap_scramble") | None => AntiCopyMethod::CmapScramble,
    Some("cmap_remove") => AntiCopyMethod::CmapRemove,
    Some(unknown) => return Err(format!("未知的防复制方法: {unknown}")),
};
```

---

### 🟡 P2 — 本月修复

#### P2-1: `list_template_library_items` 静默跳过损坏模板
**文件**：`src-tauri/src/docx_template/mod.rs:383-385`
**方案**：收集警告列表，在返回结果 JSON 中附加 `warnings` 字段。

#### P2-2: `sanitize_manifest_private_labels` 可能误替换合法标签
**文件**：`src-tauri/src/docx_template/mod.rs:299-321`
**方案**：增加条件——只在 `label == marked_text && generated_field_name(&field.name)` 时才替换。

#### P2-3: `import_template_to_library` 不检查目标冲突
**文件**：`src-tauri/src/commands/template.rs:295-306`
**方案**：使用已有的 `unique_docx_output_path` 或在覆盖前弹出确认对话框（通过返回冲突信息让前端处理）。

#### P2-4: 正则解析 XML 脆弱性
**文件**：`src-tauri/src/pdf/detection.rs:478-523`
**方案**：将 `pdftotext -bbox` XML 输出用 `quick-xml` 解析，替代正则。可参考 `docx_template/ooxml.rs` 已有的 XML 解析模式。

#### P2-5: `content_text.rs` 每次调用编译正则
**文件**：`src-tauri/src/pdf/content_text.rs:467-488`
**方案**：使用 `once_cell::sync::Lazy` 或 `std::sync::OnceLock` 预编译：
```rust
use std::sync::OnceLock;
fn page_pattern() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\{page\}|\{total\}").unwrap())
}
```

#### P2-6: `scan_document_index` 中递归逻辑需要注释说明
**文件**：`src-tauri/src/docx_template/scan.rs:39-56`
**方案**：添加注释说明嵌套 `w:p` 的处理逻辑（已验证正确，只需文档化）。

---

## 二、架构改进方案

### 2.1 巨型组件拆分方案

#### 2.1.1 EvidencePdfWorkbench (3634 行) 拆分

**拆分目标**：将 3634 行拆分为 5-7 个子组件 + 2-3 个 composable。

**子组件拆分方案**：

| 新组件 | 行数估计 | 职责 |
|--------|---------|------|
| `EvidenceFileImporter.vue` | ~200 | 文件导入区：单文件/合并 PDF 导入按钮、拖放区、合并分析进度 |
| `ExistingElementsPanel.vue` | ~400 | 原有元素区：原页眉页脚检测结果、一键确认/清除、展开行编辑 |
| `HeaderFooterRulePanel.vue` | ~300 | 新规则区：页眉/页脚/页码配置（使用已有的 `HeaderFooterRuleFields`） |
| `A4NormalizePanel.vue` | ~80 | A4 规范化设置区块 |
| `BookmarkPanel.vue` | ~120 | 书签配置区块 |
| `OutputModePanel.vue` | ~150 | 输出模式选择和路径配置 |
| `EvidenceProcessingStatus.vue` | ~100 | 处理状态展示（分析中/拆分中/处理中） |

**Composable 拆分方案**：

| 新 Composable | 职责 |
|---------------|------|
| `useEvidenceFileManagement` | 从 `EvidencePdfWorkbench` 提取文件导入/删除/排序逻辑（~`loadEvidenceFiles`, `removeOverlayFile`, `reorderOverlayFiles`） |
| `useEvidenceOutputConfig` | 提取输出模式/路径/执行逻辑（~`applyHeaderFooter`, `currentRules`, `canApplyHeaderFooter`） |
| `useEvidenceEditState` | 提取编辑状态管理（~`editUndoStack`, `editRedoStack`, `pushEditUndo`, `editingContentRowId`） |

**状态提升/下沉策略**：

```
EvidencePdfView.vue (状态中心)
  ├── useEvidencePdfSession (已有，保持不变 — 数据层)
  ├── useEvidencePdfDetection (已有，保持不变 — 检测层)
  ├── useEvidenceFileManagement (新 — 文件管理)
  ├── useEvidenceOutputConfig (新 — 输出配置)
  ├── useEvidenceEditState (新 — 编辑状态)
  │
  ├── EvidenceFileImporter.vue ← useEvidenceFileManagement
  ├── ExistingElementsPanel.vue ← useEvidencePdfDetection + useEvidenceEditState
  ├── HeaderFooterRulePanel.vue ← useEvidencePdfPreview
  ├── OutputModePanel.vue ← useEvidenceOutputConfig
  └── EvidenceProcessingStatus.vue ← 纯 props 展示
```

**数据流重构**：
- 子组件通过 `defineModel`（Vue 3.4+）或 `v-model` 绑定核心状态
- 业务操作通过 composable 提供的函数调用，而非 emit 到父组件
- 消除 `HeaderFooterRuleFields` 的 30+ props 问题：将规则对象作为单个 prop 传入，内部使用 `computed` 派生

**拆分路径**：
1. 先提取 `useEvidenceFileManagement` composable
2. 提取 `EvidenceFileImporter` 子组件
3. 提取 `useEvidenceOutputConfig` composable
4. 提取 `OutputModePanel` 子组件
5. 逐步提取其余子组件
6. 每步完成后运行回归测试

---

#### 2.1.2 TemplateView (2900 行) 拆分

**问题分析**：TemplateView 是状态中心 + 事件分发器，子组件（BuildTab/RenderTab 等）只是展示层。25 个 props + 30+ emits 传递到 TemplateBuildTab。

**拆分方案**：

**方案：Pinia Store + Composable**

Step 1 — 创建 Pinia store `src/modules/template/stores/templateStore.js`：
```javascript
import { defineStore } from 'pinia'

export const useTemplateStore = defineStore('template', {
  state: () => ({
    // 源文档状态
    sourceDocx: null,
    marks: [],
    documentText: '',
    documentRuns: [],

    // 字段行状态
    fieldRows: [],
    selectedRows: [],

    // 模板包状态
    templatePath: null,
    templateManifest: null,
    templateName: '',

    // 填写表单状态
    formValues: {},
    referenceSelections: {},
    structureOverrides: {},
    typeOverrides: {},

    // UI 状态
    activeTab: 'build',
    scanning: false,
    saving: false,
    rendering: false,
    batchProcessing: false,
  }),

  getters: {
    renderableTemplateFields() { ... },
    filteredRenderableFields() { ... },
    canSaveTemplate() { ... },
  },

  actions: {
    async selectSourceDocx() { ... },
    async saveTemplate() { ... },
    async renderTemplate() { ... },
    // ... 从 TemplateView.vue 迁移
  },
})
```

Step 2 — 将 TemplateView.vue 瘦身为壳组件：
```vue
<template>
  <div class="template-view">
    <el-tabs v-model="store.activeTab" class="template-tabs">
      <el-tab-pane label="制作模板" name="build">
        <TemplateBuildTab />
      </el-tab-pane>
      <el-tab-pane label="填写模板" name="render">
        <TemplateRenderTab />
      </el-tab-pane>
      <el-tab-pane label="填写历史" name="history">
        <TemplateHistoryTab />
      </el-tab-pane>
      <el-tab-pane label="设置" name="settings">
        <TemplateSettingsTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { useTemplateStore } from '../stores/templateStore'
const store = useTemplateStore()
</script>
```

Step 3 — 子组件直接使用 store，无需 props/emits 传递：
```vue
<!-- TemplateBuildTab.vue -->
<script setup>
import { useTemplateStore } from '../stores/templateStore'
const store = useTemplateStore()
// 直接访问 store.fieldRows, store.marks 等
// 直接调用 store.saveTemplate(), store.undoLastAction() 等
</script>
```

**收益**：
- TemplateView.vue 从 2900 行降至 ~50 行
- TemplateBuildTab 的 props 从 25 个降至 0 个
- 消除 30+ 个 emit 事件
- Vue DevTools 中可追踪 store 状态变化
- 子组件可独立测试

---

### 2.2 重复代码消除方案

| 重复项 | 涉及文件 | 消除方案 | 已在 P1 中覆盖 |
|--------|---------|---------|---------------|
| `same_path` | annotations.rs / artifacts.rs / evidence_session.rs | 提取到 `pdf/mod.rs` | ✅ P1-1 |
| `temp_named_path` | 7 个 pdf 子模块 | 提取到 `pdf/mod.rs` + `TempPathGuard` | ✅ P1-2 |
| `fnv1a_hash` | docx_template/mod.rs / pdf/evidence.rs | 提取到 `sort_utils.rs` | ✅ P1-3 |
| `splitPartyLabelSegments` | fieldRowUtils.js:176-204 / TemplateView.vue:1299-1324 | 删除 TemplateView 版本，统一使用 fieldRowUtils 版本 | 新增 |
| `ensureExtension` | fieldRowUtils.js:225-231 / TemplateView.vue:2708-2713 | 删除 TemplateView 版本，统一使用 fieldRowUtils 版本 | 新增 |
| `triggerPreviewSelectionAdd` | usePreviewSelection.js:285-293 / TemplateView.vue:1115-1127 | 删除 TemplateView 版本，使用 composable 版本 | 新增 |
| `effectiveFieldType` | TemplateRenderTab.vue:446-453 / TemplateView.vue:425-427 | 统一到 fieldRowUtils.js，消除微妙差异 | 新增 |
| `fieldFormKey` | TemplateRenderTab.vue:441-444 / TemplateView.vue | 删除重复，提取到 fieldRowUtils.js | 新增 |
| `download_package` / `download_package_to_temp_file` | managed.rs:265-389 | 提取公共下载逻辑到 `download_to_writer` | 新增 |

**新增重复消除**：

`src/modules/template/composables/fieldRowUtils.js` 新增统一函数：
```javascript
// 统一 effectiveFieldType，合并两个版本的逻辑差异
export function effectiveFieldType(field, typeOverrides, formValues, fieldRows) {
  const override = typeOverrides?.[field.id]
  if (override) return override
  // 处理 fillAllPositions follower 场景（来自 RenderTab 版本的逻辑）
  if (field.followTarget && formValues?.[field.followTarget]) {
    const targetRow = fieldRows.find(r => r.id === field.followTarget)
    if (targetRow?.type === 'party_list') return 'party_list'
  }
  return field.type
}
```

---

### 2.3 前后端默认值统一方案

| 参数 | 前端默认值 | 后端默认值 | 文件位置 | 统一值 |
|------|-----------|-----------|---------|-------|
| `margin_mm` | 12.0 | 15.0 | image_paddler.rs:523 | **12.0** |
| `filename_without_ext` | true | false | image_paddler.rs:525 | **true** |
| `headerMode` | undefined (不插入) | 未定义 | 后端无独立默认值 | **前端控制** |

**统一策略**：以后端 `unwrap_or` 值与前端默认值一致为原则。前端推荐值来自用户体验分析，后端应跟随。

---

## 三、UI/UX 改进方案

### 3.1 WCAG 对比度修复

**当前问题**：`--docsy-text-muted` (#7a817d) 在 `--docsy-surface-muted` (#f3eadf) 上的对比度约 3.2:1，低于 WCAG AA 的 4.5:1。

**修复方案**：调整 `--docsy-text-muted` 色值。

`src/styles.css`：
```css
:root {
  /* 原值 #7a817d (对比度 3.2:1) → 新值 #5c6660 (对比度 5.1:1) */
  --docsy-text-muted: #5c6660;
}
```

对比度验证：
- `#5c6660` on `#f3eadf` → 5.1:1 ✅ (WCAG AA)
- `#5c6660` on `#fcf5ea` → 5.4:1 ✅ (WCAG AA)
- `#5c6660` on `#fff9f0` → 5.7:1 ✅ (WCAG AA)

---

### 3.2 Loading/Error 状态统一方案

**当前问题**：4+ 种不同实现方式：
1. `el-button :loading` — inline 按钮 loading
2. `.local-processing` — 自定义区域 + spinner
3. `el-icon.is-loading` — 旋转图标
4. `ElMessage.error` — toast 错误

**统一方案**：

**Loading 状态 — 三级规范**：

| 级别 | 场景 | 组件 | 文件 |
|------|------|------|------|
| L1: 按钮级 | 单个操作按钮 | `el-button :loading` | 保持不变 |
| L2: 区域级 | 页面某区域加载中 | 新组件 `LoadingOverlay.vue` | `src/shared/components/LoadingOverlay.vue` |
| L3: 页面级 | 整页加载 | 骨架屏 `el-skeleton` | 按需添加 |

新建 `src/shared/components/LoadingOverlay.vue`：
```vue
<template>
  <div class="loading-overlay" role="status" aria-live="polite">
    <span class="processing-spinner" />
    <div class="loading-content">
      <strong>{{ title }}</strong>
      <p v-if="description">{{ description }}</p>
    </div>
  </div>
</template>

<script setup>
defineProps({
  title: { type: String, required: true },
  description: { type: String, default: '' },
})
</script>
```

**Error 状态 — 两级规范**：

| 级别 | 场景 | 实现 |
|------|------|------|
| Toast 级 | 瞬时错误（操作失败、验证错误） | `ElMessage.error` 保持不变 |
| Inline 级 | 持久错误（连接断开、文件损坏） | 新组件 `InlineAlert.vue` |

新建 `src/shared/components/InlineAlert.vue`：
```vue
<template>
  <el-alert
    :title="title"
    :description="description"
    :type="type"
    show-icon
    :closable="closable"
  >
    <template v-if="$slots.action" #default>
      <slot name="action" />
    </template>
  </el-alert>
</template>
```

**替换计划**：

| 当前实现 | 替换为 |
|---------|--------|
| `EvidencePdfWorkbench.vue` 中 4 个 `.local-processing` 块 | `<LoadingOverlay :title="..." :description="..." />` |
| `PdfToolsView.vue` 中的 `merging` 状态 | `<LoadingOverlay>` + 进度百分比 |
| `VideoExtractView.vue` 中的 `extracting` 状态 | `<LoadingOverlay>` + 帧数进度 |

---

### 3.3 暗色模式方案

**CSS 变量扩展**：

在 `src/styles.css` 末尾添加：
```css
[data-theme="dark"] {
  --docsy-canvas: #1a1a1a;
  --docsy-surface: #242424;
  --docsy-surface-elevated: #2d2d2d;
  --docsy-surface-muted: #1f1f1f;
  --docsy-sidebar: #1e1e1e;
  --docsy-sidebar-hover: #2a2a2a;
  --docsy-border-subtle: #383838;
  --docsy-border-strong: #444444;
  --docsy-preview-paper: #f5f5f5;
  --docsy-text-strong: #e8e8e8;
  --docsy-text: #c8c8c8;
  --docsy-text-muted: #999999;
  --docsy-primary: #5a9e8f;
  --docsy-primary-hover: #4d8a7c;
  --docsy-primary-soft: #1e3a33;
  --docsy-accent: #d99a6b;
  --docsy-accent-soft: #3a2a1a;
  --docsy-success: #6aad7a;
  --docsy-warning: #d4a04a;
  --docsy-danger: #d46b64;
  --docsy-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);

  /* Element Plus 变量覆盖 */
  --el-color-primary: var(--docsy-primary);
  --el-text-color-primary: var(--docsy-text-strong);
  --el-text-color-regular: var(--docsy-text);
  --el-text-color-secondary: var(--docsy-text-muted);
  --el-bg-color: var(--docsy-surface-elevated);
  --el-bg-color-page: var(--docsy-canvas);
  --el-fill-color: #2a2a2a;
  --el-fill-color-light: var(--docsy-surface-muted);
  --el-fill-color-lighter: #333333;
  --el-fill-color-blank: var(--docsy-surface-elevated);
}
```

**主题切换实现**：

`src/core/theme.js`：
```javascript
import { ref, watch } from 'vue'

const THEME_KEY = 'docsy-theme'
const theme = ref(localStorage.getItem(THEME_KEY) || 'light')

export function useTheme() {
  function setTheme(value) {
    theme.value = value
    document.documentElement.setAttribute('data-theme', value)
    localStorage.setItem(THEME_KEY, value)
  }

  function toggleTheme() {
    setTheme(theme.value === 'light' ? 'dark' : 'light')
  }

  // 初始化
  document.documentElement.setAttribute('data-theme', theme.value)

  // 跟随系统偏好
  function followSystem() {
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    setTheme(mq.matches ? 'dark' : 'light')
    mq.addEventListener('change', (e) => setTheme(e.matches ? 'dark' : 'light'))
  }

  return { theme, setTheme, toggleTheme, followSystem }
}
```

**在 App.vue 侧边栏底部添加切换按钮**：
```vue
<template>
  <!-- sidebar-footer 区域 -->
  <div class="sidebar-footer">
    <el-button link @click="router.push('/about')">关于</el-button>
    <el-button link @click="router.push('/settings')">设置</el-button>
    <el-button link @click="toggleTheme">
      <el-icon><Moon v-if="theme === 'light'" /><Sunny v-else /></el-icon>
    </el-button>
  </div>
</template>
```

**硬编码色值排查**：

| 文件 | 问题 | 修复 |
|------|------|------|
| 多个组件使用 `rgba(54, 45, 36, ...)` 硬编码阴影 | 暗色模式下阴影不可见 | 改用 `var(--docsy-shadow)` |
| `.local-processing` 使用硬编码 `#fef9f2` | 暗色模式下对比度差 | 改用 `var(--docsy-surface-muted)` |
| `DocletWorkingPet.vue` spritesheet | 暗色模式下可能不清晰 | 添加 `filter: brightness(0.9)` |

---

### 3.4 键盘快捷键方案

**实现方式**：使用 VueUse 的 `useMagicKeys` 或自实现简单的键盘事件管理。

`src/core/shortcuts.js`：
```javascript
import { onMounted, onUnmounted } from 'vue'

const shortcuts = new Map()

export function registerShortcut(key, handler, description) {
  shortcuts.set(key, { handler, description })
}

export function unregisterShortcut(key) {
  shortcuts.delete(key)
}

export function useGlobalShortcuts() {
  function handleKeydown(e) {
    const key = buildKeyString(e)
    const shortcut = shortcuts.get(key)
    if (shortcut) {
      e.preventDefault()
      shortcut.handler()
    }
  }

  function buildKeyString(e) {
    const parts = []
    if (e.metaKey || e.ctrlKey) parts.push('mod')
    if (e.shiftKey) parts.push('shift')
    if (e.altKey) parts.push('alt')
    parts.push(e.key.toLowerCase())
    return parts.join('+')
  }

  onMounted(() => window.addEventListener('keydown', handleKeydown))
  onUnmounted(() => window.removeEventListener('keydown', handleKeydown))
}

export function getShortcutList() {
  return Array.from(shortcuts.entries()).map(([key, { description }]) => ({
    key,
    description,
  }))
}
```

**全局快捷键定义**：

| 快捷键 | 功能 | 范围 |
|--------|------|------|
| `Mod+S` | 保存模板 | Template 模块 |
| `Mod+Z` | 撤销 | 全局 |
| `Mod+Shift+Z` | 重做 | 全局 |
| `Mod+N` | 新建（选择新文件） | 各工具模块 |
| `Mod+1..4` | 切换 Tab 页 | 全局 |
| `Escape` | 关闭弹窗/取消操作 | 全局 |

---

## 四、文档更新方案

### 4.1 需要更新的文档

#### `docs/architecture.md`
- 更新版本号：`v0.9.2` → `v0.9.6`
- 更新命令数量：`58 个` → `76 个`（pdf: 20→26, template: 16→25, system: 8→11）
- 补充 `anti_ocr.rs` 模块描述（452 行，防复制保护检测/应用/移除）
- 补充 `bookmarks` 功能描述
- 更新模块结构图，添加 `overlay.rs`（兼容层）和 `content_text.rs`

#### `docs/pdf-evidence-processing-design.md`
- 修正 A4 规范化描述：从"重渲染"改为"内容流矩阵缩放"（保留文本可选中性）
- 清理"尚未实现"清单：`artifacts.rs`、`content_text.rs`、确认窗口、页码分类均已实现
- 补充防复制功能描述
- 补充书签功能描述
- 更新预览实现策略：后端 pdftoppm 优先，PDF.js 为备用

#### `docs/template-system-design.md`
- 补充 `reference` 类型的完整说明（当前仅在"当前实现范围"中列出名称）
- 补充引用字段自动解析机制描述
- 补充历史建议三层查询机制描述

#### `README.md`
- 补充防复制功能（detect/apply/remove）
- 补充书签功能（has/remove）
- 补充关于页面
- 更新字段类型数量（7→8）

#### `docs/CHANGELOG.md`
- 修正 v0.9.0 的"7 种字段类型"为"8 种"

---

### 4.2 需要新增的文档

| 文档 | 内容 | 优先级 |
|------|------|--------|
| `docs/module-anti-copy.md` | 防复制模块设计：三种方法（CmapScramble/CmapRemove）、备份机制、安全考量（映射空间 4096 值） | P2 |
| `docs/module-bookmarks.md` | 书签模块设计：写入机制、合并 PDF 偏移计算、与页码的关系 | P2 |
| `docs/external-tools.md` | 外部工具管理：查找优先级、SHA256 校验、托管安装流程、安全机制 | P2 |
| `docs/architecture-addendum.md` | v0.9.3–0.9.6 架构变更记录（相对于 architecture.md 的增量） | P1 |

---

### 4.3 需要删除/归档的过时文档

| 文档 | 操作 | 原因 |
|------|------|------|
| `docs/Archived/Docsy软件设计文档.md` | **添加归档标注** | 已被 `template-system-design.md` 替代，首页添加"⚠️ 此文档已归档，最新设计见 template-system-design.md" |
| `docs/pdf-ocr-module-design.md` | **添加状态标注** | 前瞻性设计，尚未进入实现阶段。首页添加"📋 状态：设计阶段，尚未实现" |

---

## 五、技术债务清理方案

### 5.1 大文件拆分路径

| 文件 | 当前行数 | 目标行数 | 拆分路径 |
|------|---------|---------|---------|
| `EvidencePdfWorkbench.vue` | 3634 | <500 | 按 2.1.1 方案拆分为 7 个子组件 + 3 个 composable |
| `TemplateView.vue` | 2900 | <100 | 按 2.1.2 方案迁移到 Pinia store |
| `detection.rs` | 2645 | <800 | 拆分为 `detection/mod.rs` + `detection/candidates.rs` + `detection/split_suggestions.rs` + `detection/xml_parser.rs` |
| `header_footer.rs` | 2394 | <800 | 拆分为 `header_footer/mod.rs` + `header_footer/overlay.rs` + `header_footer/bookmark.rs` + `header_footer/cleanup.rs` |
| `TemplateBuildTab.vue` | 1464 | <800 | 通过 Pinia store 消除 props/emits 适配层，减少 ~300 行 |
| `image_paddler.rs` | 1587 | <600 | 拆分为 `image_paddler/mod.rs` + `image_paddler/analyze.rs` + `image_paddler/layout.rs` + `image_paddler/filename.rs` |
| `fieldRowUtils.js` | 1199 | <400 | 拆分为 `typeUtils.js` + `previewUtils.js` + `referenceUtils.js` + `dateUtils.js` + `partyUtils.js` |
| `useFieldNormalization.js` | 810 | <400 | 拆分为 `normalizePipeline.js` + `fieldInference.js` + `fieldValidation.js` |
| `template_history.rs` | 948 | <400 | 拆分为 `template_history/mod.rs` + `template_history/db.rs` + `template_history/suggestions.rs` |
| `content_text.rs` | 999 | <500 | 拆分为 `content_text/mod.rs` + `content_text/parser.rs` + `content_text/deleter.rs` |
| `artifacts.rs` | 1430 | <500 | 拆分为 `artifacts/mod.rs` + `artifacts/inspect.rs` + `artifacts/delete.rs` + `artifacts/edit.rs` |
| `evidence.rs` | 1081 | <500 | 拆分为 `evidence/mod.rs` + `evidence/scan.rs` + `evidence/build.rs` + `evidence/merge.rs` |
| `useEvidencePdfSession.js` | 1043 | <500 | 拆分为 `evidenceFileModel.js` + `evidencePayloadBuilder.js` + `evidencePageRanges.js` |

**拆分优先级排序**（按收益/风险比）：
1. TemplateView → Pinia store（收益最高，消除全局状态中心）
2. fieldRowUtils.js → 5 个工具文件（纯函数，拆分风险最低）
3. EvidencePdfWorkbench → 子组件（复杂度最高，需谨慎逐步推进）
4. Rust 大文件拆分（按需进行，优先处理 detection.rs 和 header_footer.rs）

---

### 5.2 死代码清理清单

| 文件 | 位置 | 代码 | 操作 |
|------|------|------|------|
| `src-tauri/src/services/module_registry.rs` | 全文件 | `all_descriptors()` + `#[allow(dead_code)]` | 删除文件（前端模块注册由 `moduleRegistry.js` 驱动） |
| `src-tauri/src/pdf/overlay.rs` | 全文件 | 9 行 facade 模块 | 删除，将导入改为直接使用 `header_footer` 和 `preview` |
| `src-tauri/src/pdf/content_text.rs:365` | `update_text_state_after_show` | 空函数 no-op | 删除 |
| `src-tauri/src/commands/system.rs:67-88` | `mime` 变量 | `let _ = mime;` 显式忽略 | 删除 mime 匹配逻辑，直接返回 `image/jpeg` |
| `src-tauri/src/pdf/normalize.rs:9` | `_dpi` 参数 | 接受但从未使用 | 删除参数，修改调用方 |
| `src-tauri/src/pdf/normalize.rs:46-53` | `"preserve"` 和 `_` 分支 | 相同条件的死代码 | 删除 `"preserve"` 分支或合并 |
| `src-tauri/src/docx_template/index.rs` | 多处 `#[allow(dead_code)]` | `part_name`, `total_text_nodes`, `iter_nodes`, `iter_highlighted` | 评估是否需要：如仅用于诊断，改用 `#[cfg(debug_assertions)]` |
| `src-tauri/src/pdf/page_info.rs:70-76` | `CropBox`/`MediaBox` 大写变体 | qpdf JSON 使用 camelCase，大写变体是死代码 | 删除大写变体检查 |
| `src-tauri/src/external/external/qpdf.rs:95-104` | `_ => "qpdf.exe"` | 永远不会命中 | 删除死分支 |
| `src/modules/pdf-tools/views/EvidencePdfWorkbench.vue` | 多处 `useHistory` 未使用 | 引入但未实际使用 | 删除 import 或接入使用 |

---

### 5.3 魔法数字常量化清单

| 文件 | 行号 | 魔法数字 | 提取为 |
|------|------|---------|-------|
| `src-tauri/src/commands/system.rs:58-59` | `MAX_PREVIEW_EDGE: u32 = 1600` / `MAX_SOURCE_PIXELS: u64 = 64_000_000` | 已有常量定义 ✅ | — |
| `src-tauri/src/pdf/header_footer.rs` | `10_000` (unique_output_path 循环上限) | `const MAX_PATH_COLLISION_ATTEMPTS: usize = 10_000` |
| `src-tauri/src/pdf/detection.rs:615` | `3.0` (y 轴桶大小) | `const LINE_GROUPING_TOLERANCE_PT: f32 = 3.0` |
| `src-tauri/src/pdf/anti_ocr.rs:446` | `0xE000..0xF000` (PUA 范围) | `const PUA_RANGE_START: u32 = 0xE000` / `const PUA_RANGE_END: u32 = 0xF000` |
| `src/modules/pdf-tools/composables/useEvidencePdfDetection.js:39` | `300` (MERGED_IMPORT_AUTO_SCAN_PAGES) | 已有常量定义 ✅ | — |
| `src/modules/pdf-tools/composables/useEvidencePdfSession.js:770` | `0.58` / `0.72` (CJK/Latin ratio) | `const CJK_CHAR_WIDTH_RATIO = 0.58` / `const LATIN_CHAR_WIDTH_RATIO = 0.72` |
| `src/modules/template/composables/usePreviewSelection.js` | `350` (防抖间隔 ms) | `const PREVIEW_ADD_DEBOUNCE_MS = 350` |
| `src-tauri/src/image_paddler.rs:948` | `15.0` (DOCX_TRAILING_GAP_MM) | 已有常量定义 ✅ | — |
| `src-tauri/src/lib.rs:167` | `500` (轮询间隔 ms) | 修复 P0-3 后此数字消失 |
| `src/modules/pdf-tools/components/HeaderFooterRuleFields.vue:671` | `Date.now()` (group ID) | 改用 `crypto.randomUUID()` 或自增计数器 |
| `src/core/services/appLogger.js:8` | `/base64\|docx\|bytes\|content\|html\|xml\|text/i` | `const SENSITIVE_KEY_PATTERN = /base64\|docx\|bytes\|content\|html\|xml/i` （移除 `text` 以减少误截断） |
| `src-tauri/src/external/managed.rs` | 各工具的 `max_bytes` 大小限制 | 提取为各工具的常量：`const QPDF_MAX_DOWNLOAD_BYTES` / `const FFMPEG_MAX_DOWNLOAD_BYTES` 等 |

---

## 附录：实施路线图

### 第一周（P0 修复）
- [ ] P0-1: cancel_operation 按 ID 取消
- [ ] P0-2: Mutex unwrap 安全模式
- [ ] P0-3: Condvar 替代忙等待
- [ ] P0-4: 书签页码偏移修复
- [ ] P0-5: 书签写入原子操作
- [ ] P0-6: 输出路径碰撞报错

### 第二周（P1 修复 + 架构基础）
- [ ] P1-1 ~ P1-8: 所有 P1 Bug 修复
- [ ] 开始 fieldRowUtils.js 拆分（风险最低）
- [ ] 开始 TemplateView → Pinia store 迁移

### 第三周（架构改进 + UI 基础）
- [ ] 完成 TemplateView 拆分
- [ ] WCAG 对比度修复
- [ ] 创建 LoadingOverlay / InlineAlert 共享组件
- [ ] 开始 EvidencePdfWorkbench 拆分

### 第四周（UI/UX + 文档）
- [ ] 暗色模式实现
- [ ] 键盘快捷键体系
- [ ] 更新设计文档（architecture.md / pdf-evidence-design.md）
- [ ] 新增缺失文档（anti-copy / bookmarks / external-tools）
- [ ] 死代码清理

### 持续改进
- [ ] Rust 大文件拆分（按需）
- [ ] 魔法数字常量化（按需）
- [ ] P2/P3 Bug 修复
- [ ] 全局搜索 / 自动保存 / 系统通知

---

*本方案基于 2026-08-07 的审阅报告和代码分析生成。实施时应根据实际开发进度和用户反馈动态调整优先级。*
