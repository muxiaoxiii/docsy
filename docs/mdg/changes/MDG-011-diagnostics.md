# MDG-010: 全局诊断与探针系统

## 状态：🟡 设计中
## 优先级：P1
## 关联：MDG-001（OperationManager 事件接驳）

---

## 1. 问题诊断

### 1.1 现状

| 位置 | 诊断内容 | 方式 |
|------|----------|------|
| `app_log.rs` | panic hook + 前端日志写入 | 手动调用 |
| `TemplateView.vue:1380` | 模板模块状态快照 | 模块内嵌 JSON dump |
| `SettingsView.vue:376` | 工具版本 + 系统信息 | `get_diagnostic_info` 命令 |
| `HomeView.vue:52` | 同上 | 同上 |
| `commands/system.rs:24` | 前端错误转发 | `write_frontend_log` 命令 |

### 1.2 问题

1. **模块各自为政**：模板模块自己写诊断 JSON，如果 PDF 模块也要诊断，又得写一套
2. **没有结构化日志**：只有 error 级别，没有 debug/info/warn + 时间戳 + 模块名
3. **没有操作追踪**：一个 batch_overlay 操作处理了 50 个 PDF，中间第 37 个失败了，无法回溯前 36 个的处理过程
4. **没有性能指标**：操作耗时、文件大小、处理速度都没有记录
5. **没有崩溃报告**：panic hook 只写调用栈，不写系统状态（活跃操作、最近日志、内存使用）
6. **没有用户导出**：出问题了让用户手动去 ~/Library/Logs/docsy/ 找文件

### 1.3 与 MDG-001 的关系

MDG-001 的 OperationManager 已经埋了接驳点：
- `docsy-operation-started` / `docsy-operation-finished` 事件
- 每个操作有 operationId + command

诊断系统应该订阅这些事件，自动追踪每个操作的生命周期。

---

## 2. 设计目标

### 2.1 核心目标

**一个全局的、深入的诊断和探针系统**，替代所有模块内嵌的诊断代码。

### 2.2 具体目标

1. **结构化日志**：debug/info/warn/error + 时间戳 + 模块名 + 操作关联
2. **操作追踪**：通过 OperationManager 事件自动追踪每个操作的完整生命周期
3. **性能指标**：操作耗时、文件大小、处理速度
4. **状态快照**：任何模块可以注册自己的状态提供者（替代模板模块的内嵌 dump）
5. **崩溃报告**：panic 时自动收集活跃操作 + 最近日志 + 系统信息
6. **用户导出**：一键导出诊断报告（日志 + 状态快照 + 系统信息）

### 2.3 非目标

- 不做远程日志收集（本地桌面应用）
- 不做实时日志流（不需要 WebSocket 推送到调试界面）
- 不做日志搜索（日志文件够小，直接打开看）

---

## 3. 架构设计

### 3.1 整体架构

```
┌─ 前端 ──────────────────────────────────────────────────────┐
│  diagnostics.js（全局服务）                                   │
│    log.debug/info/warn/error(message, context)              │
│    snapshot(moduleName, data)  // 注册模块状态快照            │
│    exportReport()              // 一键导出诊断报告            │
│                                                              │
│  各模块：                                                    │
│    只调用 diagnostics.log.info('模板保存成功', { fields: 5 }) │
│    只调用 diagnostics.snapshot('template', () => ({ ... })) │
│    不再自己写 JSON 文件                                       │
├──────────────────────────────────────────────────────────────┤
│  tauriBridge.js：                                            │
│    tauriCall / tauriCallSafe 自动记录操作耗时                  │
│    emitOperationEvent 自动关联 operationId                   │
└──────────────────────────────────────────────────────────────┘
                    ↕ invoke + listen
┌─ Rust ──────────────────────────────────────────────────────┐
│  diagnostics.rs（全局诊断模块）                               │
│    Diagnostics { logger, metrics, snapshots }               │
│                                                              │
│  Logger：                                                    │
│    log(level, module, message, context)                     │
│    结构化写入日志文件（按日期轮转）                            │
│    关联 operationId（一个操作的所有日志可追溯）                │
│                                                              │
│  Metrics：                                                   │
│    record_operation(command, duration, success, file_size?) │
│    record_external_tool(tool, duration, exit_code?)         │
│                                                              │
│  OperationTracker：                                          │
│    订阅 OperationManager 的 begin/finish 事件               │
│    记录每个操作的开始/结束/耗时/取消                          │
│    活跃操作列表（崩溃报告用）                                 │
│                                                              │
│  CrashReporter：                                             │
│    panic hook 改进：收集活跃操作 + 最近日志 + 系统信息        │
│    自动生成 crash-report-YYYYMMDD-HHMMSS.json               │
│                                                              │
│  StateProviders：                                            │
│    各模块注册自己的状态提供者                                  │
│    export_report() 时收集所有提供者的快照                     │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 日志系统

```rust
// Rust 端
pub struct Logger {
    log_dir: PathBuf,
    current_file: Mutex<File>,
    max_file_size: u64,      // 默认 5MB
    max_files: usize,        // 默认 10 个轮转文件
}

pub struct LogEntry {
    timestamp: DateTime<Utc>,
    level: LogLevel,         // Debug, Info, Warn, Error
    module: String,          // "pdf.overlay", "template.build", etc.
    message: String,
    operation_id: Option<String>,  // 关联 OperationManager
    context: serde_json::Value,    // 任意附加数据
}

impl Logger {
    pub fn log(&self, entry: LogEntry) { ... }
    pub fn debug(&self, module: &str, msg: &str) { ... }
    pub fn info(&self, module: &str, msg: &str) { ... }
    pub fn warn(&self, module: &str, msg: &str) { ... }
    pub fn error(&self, module: &str, msg: &str) { ... }
}
```

**日志文件格式**（NDJSON，每行一个 JSON）：
```json
{"ts":"2026-08-07T21:30:15.123Z","level":"INFO","module":"pdf.overlay","msg":"开始处理","op":"batch_overlay_pdf_text:42","ctx":{"items":50}}
{"ts":"2026-08-07T21:30:15.456Z","level":"INFO","module":"pdf.overlay","msg":"处理完成","op":"batch_overlay_pdf_text:42","ctx":{"item":1,"duration_ms":333,"size_bytes":1048576}}
{"ts":"2026-08-07T21:30:16.789Z","level":"ERROR","module":"pdf.overlay","msg":"处理失败","op":"batch_overlay_pdf_text:42","ctx":{"item":37,"error":"qpdf exit code 2"}}
```

### 3.3 操作追踪

```rust
// 订阅 OperationManager 事件，自动记录操作生命周期
pub struct OperationTracker {
    operations: Mutex<HashMap<String, OperationRecord>>,
}

pub struct OperationRecord {
    operation_id: String,
    command: String,
    started_at: Instant,
    finished_at: Option<Instant>,
    status: OperationStatus,  // Running, Completed, Cancelled, Failed
    log_entries: Vec<String>,  // 关联的日志条目 ID
}
```

**与 MDG-001 的接驳**：
- OperationManager.begin() 时发射 `docsy-operation-started`
- OperationManager.finish() 时发射 `docsy-operation-finished`
- Diagnostics 订阅这些事件，自动创建/关闭 OperationRecord

### 3.4 状态快照（替代模块内嵌诊断）

```rust
// 各模块注册自己的状态提供者
pub struct Diagnostics {
    state_providers: Mutex<Vec<Box<dyn StateProvider>>>,
}

pub trait StateProvider: Send + Sync {
    fn name(&self) -> &str;
    fn snapshot(&self) -> serde_json::Value;
}

// 示例：模板模块注册
impl StateProvider for TemplateStateProvider {
    fn name(&self) -> &str { "template" }
    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "templatePath": self.template_path,
            "fieldRowCount": self.field_rows.len(),
            "markCount": self.marks.len(),
            "uncoveredMarks": self.uncovered_marks(),
            // ... 替代 TemplateView.vue 里的 diagnostic 对象
        })
    }
}
```

**前端对应**：
```javascript
// diagnostics.js
export function registerSnapshotProvider(name, providerFn) {
    stateProviders.set(name, providerFn)
}

export async function exportReport() {
    // 收集所有前端状态快照
    const snapshots = {}
    for (const [name, fn] of stateProviders) {
        snapshots[name] = fn()
    }
    // 调用 Rust 端收集后端状态 + 日志 + 系统信息
    const report = await invoke('export_diagnostic_report', { frontendSnapshots: snapshots })
    // 保存到用户可找到的位置
    await writeTextFile(report.path, JSON.stringify(report, null, 2))
    return report.path
}
```

### 3.5 崩溃报告

```rust
// 改进 panic hook
pub fn install_panic_hook(diagnostics: Arc<Diagnostics>) {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let report = CrashReport {
            timestamp: Utc::now(),
            message: info.to_string(),
            backtrace: Backtrace::capture().to_string(),
            active_operations: diagnostics.tracker.active_operations(),
            recent_logs: diagnostics.logger.recent_logs(100),
            system_info: collect_system_info(),
        };
        let path = diagnostics.log_dir.join(format!(
            "crash-report-{}.json",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let _ = std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap());
        default_hook(info);
    }));
}
```

### 3.6 用户导出

设置页新增"导出诊断报告"按钮：

```
诊断报告内容：
├── system_info.json        # 系统 + 工具版本
├── recent_logs.ndjson      # 最近 1000 条日志
├── operation_history.json  # 最近 50 个操作记录
├── crash_reports/          # 崩溃报告（如有）
└── frontend_snapshots/     # 各模块状态快照
    ├── template.json
    ├── pdf.json
    └── ...
```

打包为单个 JSON 文件，用户可以发给开发者分析。

---

## 4. 需要清理的现有代码

| 文件 | 现有代码 | 替换为 |
|------|----------|--------|
| `TemplateView.vue:1380-1434` | 模块内嵌诊断 dump | `diagnostics.snapshot('template', ...)` |
| `app_log.rs` | 简单 panic hook | 改进的 CrashReporter |
| `commands/system.rs:24` | `write_frontend_log` | 统一 Logger 接口 |
| `SettingsView.vue:376` | `get_diagnostic_info` | 保留，改为从 Diagnostics 读取 |

---

## 5. 分阶段实施

### Phase A: 日志基础设施
- [ ] 新建 `diagnostics.rs`，实现 Logger + LogEntry + 日志轮转
- [ ] 替换 `app_log.rs` 的简单实现
- [ ] 前端 `diagnostics.js`：log.debug/info/warn/error
- [ ] Rust 各模块开始使用 `diagnostics.log.info()` 替代 `eprintln!`

### Phase B: 操作追踪 + MDG-001 接驳
- [ ] OperationTracker 订阅 OperationManager 事件
- [ ] 日志自动关联 operationId
- [ ] 操作耗时记录

### Phase C: 状态快照 + 导出
- [ ] StateProvider trait + 注册机制
- [ ] 前端 registerSnapshotProvider
- [ ] 模板模块迁移到 snapshot（删除内嵌诊断代码）
- [ ] 设置页"导出诊断报告"按钮

### Phase D: 崩溃报告 + 性能指标
- [ ] 改进 panic hook
- [ ] Metrics 记录操作耗时/文件大小
- [ ] 性能统计面板（可选）

---

## 6. 变更日志

- 2026-08-07 — 初稿
