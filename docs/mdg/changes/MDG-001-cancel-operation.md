# MDG-001: 操作管理层重构

## 状态：🟢 基础设施完成（5 commits，其余命令按需迁移）
## 优先级：P0
## 关联：BUG-001, BUG-017, BUG-003, STYLE-001, 架构问题
## 版本：v2（2026-08-07 重写，基于架构分析 + Tauri 官方文档 + 用户反馈）

---

## 1. 问题诊断

### 1.1 设计意图（来自代码注释和文档）

`SubprocessRegistry` 的注释写得很清楚：
> "Registry of active subprocess PIDs for cancellation support. Frontend can call `cancel_operation` to kill a subprocess by its operation ID."

原设计目标：防止正在处理的子进程超时无响应导致整个软件不能工作，提供取消能力。

### 1.2 三层断裂（v1 诊断）

```
前端 operationId（字符串 "render_docx_template:1"）
    ↕ 断裂点1：ID 不匹配
Rust cancel_operation（忽略 operation_id，杀所有进程）
    ↕ 断裂点2：几乎没有东西注册
SubprocessRegistry（只有 qpdf.rs 一个调用点）
```

### 1.3 更深层的问题：超时 ≠ 无响应（v2 诊断）

当前代码 `external/mod.rs` 中的 `command_output_with_timeout` 把超时和无响应混为一谈：

```rust
// 当前逻辑：固定超时就 kill
if start.elapsed() >= timeout {
    child.kill().ok();  // ← 大文件正在正常工作就被杀了
    anyhow::bail!("命令执行超时");
}
```

**问题**：处理 500 页 PDF 合并需要 2 分钟，设 60 秒超时，它正在正常工作就被杀了。

| | 超时 | 无响应 |
|--|------|--------|
| **状态** | 还在工作，只是慢 | 卡死了，没有进展 |
| **例子** | 处理 500 页 PDF | Word 转 PDF 时 Word 弹了对话框 |
| **正确处理** | 让它继续跑，用户可以主动取消 | 需要检测 + 干预 |

### 1.4 缺失的第四通用层

Docsy 已有三个通用层：

| 通用层 | 位置 | 解决的问题 |
|--------|------|------------|
| `tauriBridge.js` | 前端 IPC | 统一错误处理 + 动画触发 |
| `moduleRegistry.js` | 前端模块注册 | 新模块自动发现 |
| `run_blocking` | Rust 异步封装 | 线程池调度 + 错误转换 |

**缺失的第四层**：操作生命周期管理。当前每个长时间操作自己管状态——ConversionState 管 Word 转 PDF 的超时等待，SubprocessRegistry 管 qpdf 的 PID，其他操作什么都不管。

---

## 2. 设计目标

### 2.1 核心目标

**建立第四通用层：操作生命周期管理**，和 `run_blocking` 同级，所有命令自动享有。

### 2.2 具体目标

1. **按 ID 取消单个操作**，不影响其他操作
2. **区分超时和无响应**——不自动 kill 正在工作的进程
3. **统一异步任务和外部子进程的管理接口**
4. **新命令自动获得取消能力**——只需用 `run_managed` 替代 `run_blocking`
5. **进度推送**——长时间操作可以向前端推送"已处理 3/10 个文件"
6. **ConversionState 统一进 OperationManager**

### 2.3 非目标

- 不要求所有命令都支持取消——快速命令（inspect、get_page_count）用 `run_blocking` 就够了
- 不要求统一外部工具的调用方式——qpdf、ffmpeg、Word 的调用方式各有不同，只需要统一**生命周期管理**

---

## 3. 架构设计

### 3.1 总体架构

```
┌─ 前端 ──────────────────────────────────────────────────────┐
│  tauriBridge.js:                                             │
│    tauriCall(command, args)                                   │
│      → 生成 operationId（保持现有逻辑）                        │
│      → 注入 args.operationId 传给 Rust（新增）                │
│      → emitOperationEvent('start', command, operationId)     │
│      → invoke(command, { ...args, operationId })             │
│      → emitOperationEvent('finish', command, operationId)    │
│                                                              │
│  App.vue:                                                    │
│    cancelCurrentOperation()                                   │
│      → tauriCallSafe('cancel_operation', { operationId })    │
│      → pendingOperations.delete(firstId)  // 只删被取消的     │
│                                                              │
│  取消按钮：保持现有 30 秒延迟显示逻辑                          │
└──────────────────────────────────────────────────────────────┘
                    ↕ operationId 通过 args 传递
┌─ Rust 操作管理层（第四通用层）────────────────────────────────┐
│                                                              │
│  OperationManager (替代 SubprocessRegistry)                  │
│    operations: Mutex<HashMap<String, OperationEntry>>        │
│                                                              │
│    OperationEntry {                                          │
│      token: CancellationToken,     // 取消信号               │
│      status: OperationStatus,      // pending/running/done   │
│      started_at: Instant,          // 启动时间               │
│      command: String,              // 命令名（用于日志）      │
│    }                                                         │
│                                                              │
│  方法：                                                      │
│    begin(id, command) → CancellationToken                    │
│    cancel(id) → bool                                         │
│    cancel_all() → void           // 仅用于 app shutdown      │
│    is_cancelled(id) → bool                                   │
│    finish(id) → void                                         │
│    list_active() → Vec<String>   // 前端查询活跃操作          │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  run_managed(task, operation_id, command)                    │
│    // 替代 run_blocking，自动注册/注销操作                     │
│    // 内部：                                                 │
│    //   1. manager.begin(operation_id, command)              │
│    //   2. spawn_blocking(move || task(token))               │
│    //   3. manager.finish(operation_id)                      │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  command_output_cancellable(command, token)                  │
│    // 替代 command_output_with_timeout 的取消能力             │
│    // 去掉固定超时自动 kill，改为只支持用户主动取消             │
│    // 保留 idle_timeout 用于检测无响应（见 §3.4）             │
│                                                              │
└──────────────────────────────────────────────────────────────┘
                    ↕ token 传给执行函数
┌─ 执行层 ──────────────────────────────────────────────────────┐
│                                                              │
│  异步任务（spawn_blocking）：                                  │
│    fn my_task(token: &CancellationToken) -> Result<T> {      │
│      for item in items {                                     │
│        if token.is_cancelled() { return Err(Cancelled) }     │
│        process(item);                                        │
│      }                                                       │
│    }                                                         │
│                                                              │
│  外部子进程（qpdf/ffmpeg/Word）：                              │
│    loop {                                                    │
│      if child.try_wait()?.is_some() { break; }  // 正常结束  │
│      if token.is_cancelled() {                    // 用户取消 │
│        child.kill(); break;                                  │
│      }                                                       │
│      if idle_timeout_exceeded() {                // 无响应    │
│        // 不自动 kill，通知前端提示用户                        │
│      }                                                       │
│      sleep(30ms);                                            │
│    }                                                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 run_managed — 第四通用层

```rust
/// 替代 run_blocking 的通用操作管理封装。
///
/// 自动注册操作到 OperationManager，将 CancellationToken 传给 task。
/// 操作完成（正常或异常）后自动注销。
///
/// # 用法
/// ```rust
/// #[tauri::command]
/// pub async fn my_long_operation(
///     manager: tauri::State<'_, Arc<OperationManager>>,
///     args: MyArgs,
/// ) -> Result<String, String> {
///     run_managed(&manager, "my_long_operation", move |token| {
///         for item in items {
///             if token.is_cancelled() {
///                 return Err(anyhow!("操作已取消"));
///             }
///             process(item)?;
///         }
///         Ok(result)
///     }).await
/// }
/// ```
pub async fn run_managed<T, F>(
    manager: &OperationManager,
    command: &str,
    operation_id: Option<String>,
    task: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(CancellationToken) -> anyhow::Result<T> + Send + 'static,
{
    let op_id = operation_id.unwrap_or_else(|| format!("{}:{}", command, uuid::Uuid::new_v4()));
    let token = manager.begin(&op_id, command);

    let result = tauri::async_runtime::spawn_blocking(move || task(token))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string());

    manager.finish(&op_id);
    result
}
```

**关键设计**：
- `operation_id` 可选——前端传了就用前端的，没传就自动生成
- `task` 接收 `CancellationToken`，可以检查也可以不检查
- 注册/注销全自动，命令层无感知
- 已有命令迁移：只需把 `run_blocking` 换成 `run_managed`

### 3.3 command_output_cancellable — 外部子进程取消

```rust
/// 替代 command_output_with_timeout，支持用户主动取消。
///
/// 不设固定超时——大文件慢就慢，不自动 kill。
/// 通过 CancellationToken 支持用户主动取消。
/// 通过 idle_timeout 检测无响应（可选）。
pub fn command_output_cancellable(
    command: &mut Command,
    token: &CancellationToken,
    idle_timeout: Option<Duration>,
) -> anyhow::Result<Output> {
    hide_command_window(command);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut last_activity = Instant::now();
    let mut last_stdout_len = 0usize;
    let mut last_stderr_len = 0usize;

    loop {
        // 正常结束
        if child.try_wait()?.is_some() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout).ok();
            }
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr).ok();
            }
            let status = child.wait()?;
            return Ok(Output { status, stdout, stderr });
        }

        // 用户主动取消
        if token.is_cancelled() {
            child.kill().ok();
            child.wait().ok();
            anyhow::bail!("用户取消操作");
        }

        // 无响应检测（可选）
        if let Some(idle) = idle_timeout {
            // 检查子进程是否有新的 stdout/stderr 输出
            // 如果长时间无输出，可能无响应
            // 但不自动 kill，只返回超时错误让前端提示用户
            if last_activity.elapsed() >= idle {
                // 不 kill，让用户决定
                anyhow::bail!("进程可能无响应（{}秒无输出）", idle.as_secs());
            }
        }

        std::thread::sleep(Duration::from_millis(30));
    }
}
```

**关键设计**：
- **不设固定超时**——大文件慢就慢，不会被误杀
- **用户取消**——通过 CancellationToken，kill 子进程
- **无响应检测**——可选的 idle_timeout，检测长时间无输出，但**不自动 kill**，只报错让前端提示用户
- **保留原有超时函数**——`command_output_with_timeout` 不删除，保留给 managed tool 下载等确实需要超时的场景

### 3.4 ConversionState 统一

当前 ConversionState 是独立的超时等待机制（Word → PDF 转换时询问用户是否继续等待）。可以统一进 OperationManager：

```rust
// 当前：独立的 ConversionState
pub struct ConversionState {
    response: Arc<(Mutex<u8>, Condvar)>,
}
// 使用：conversion_state.wait_for_user_response(app)

// 统一后：复用 OperationManager 的 CancellationToken
// Word 转 PDF 也是一个 operation，有自己的 token
// 超时时不自动 kill，而是通知前端弹窗
// 用户点"继续"→ 操作继续
// 用户点"取消"→ token.cancel()
```

但 ConversionState 的 Condvar 等待模式（BUG-003 的忙等待修复）可以独立先行，不依赖 OperationManager 的完整重构。

### 3.5 Tauri Channel（进度推送）

Tauri 原生的 `Channel<T>` 可用于 Rust → 前端的进度推送：

```rust
#[tauri::command]
pub async fn batch_render_from_xlsx(
    manager: tauri::State<'_, Arc<OperationManager>>,
    args: BatchRenderArgs,
    on_progress: tauri::ipc::Channel<ProgressEvent>,  // Tauri Channel
) -> Result<BatchResult, String> {
    run_managed(&manager, "batch_render", args.operation_id.clone(), move |token| {
        let total = args.rows.len();
        for (i, row) in args.rows.iter().enumerate() {
            if token.is_cancelled() {
                return Err(anyhow!("用户取消"));
            }
            render_row(row)?;
            // 推送进度
            on_progress.send(ProgressEvent {
                current: i + 1,
                total,
                message: format!("已处理 {}/{}", i + 1, total),
            }).ok();
        }
        Ok(result)
    }).await
}
```

**前端接收进度**：
```javascript
const onProgress = new Channel()
onProgress.onmessage = (event) => {
    // event = { current: 3, total: 10, message: "已处理 3/10" }
    updateProgressBar(event)
}
await invoke('batch_render_from_xlsx', { args, onProgress })
```

**但进度推送不是 MDG-001 的范围**，MDG-001 只做操作管理基础设施。进度推送是后续优化。

---

## 4. 修改前代码（完整）

### 4.1 Rust: SubprocessRegistry（lib.rs:17-117）

```rust
#[derive(Default)]
pub struct SubprocessRegistry {
    pids: Mutex<HashMap<String, u32>>,
}

impl SubprocessRegistry {
    pub fn register(&self, operation_id: &str, pid: u32) { ... }
    pub fn unregister(&self, operation_id: &str) { ... }
    pub fn cancel(&self, operation_id: &str) -> bool { ... }  // kill PID
    pub fn is_cancelled(&self, operation_id: &str) -> bool { ... }
    pub fn spawn_and_wait(&self, operation_id: &str, mut cmd: Command) -> io::Result<Output> { ... }
    pub fn cancel_all(&self) { ... }  // 杀所有
}
```

### 4.2 Rust: cancel_operation（commands/system.rs:137-145）

```rust
#[tauri::command]
pub fn cancel_operation(
    registry: tauri::State<'_, std::sync::Arc<crate::SubprocessRegistry>>,
    _operation_id: String,  // ← 忽略
) -> Result<bool, String> {
    registry.cancel_all();  // ← 杀所有
    Ok(true)
}
```

### 4.3 Rust: ConversionState 忙等待（lib.rs:131-180）

```rust
pub fn wait_for_user_response(&self, app: &tauri::AppHandle) -> bool {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));  // ← 忙等待
        match self.response.load(Ordering::SeqCst) {
            1 => return true,
            2 => return false,
            _ => continue,
        }
    }
}
```

### 4.4 Rust: command_output_with_timeout（external/mod.rs:121-155）

```rust
pub fn command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,  // ← 固定超时
) -> anyhow::Result<Output> {
    let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() { return Ok(output); }
        if start.elapsed() >= timeout {
            child.kill().ok();  // ← 超时就 kill（大文件会被误杀）
            child.wait().ok();
            anyhow::bail!("命令执行超时");
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}
```

### 4.5 前端: tauriBridge.js 调用方式

```javascript
// operationId 从未传给 Rust
export async function tauriCall(command, args = {}) {
    const operationId = nextOperationId(command)
    emitOperationEvent('start', command, operationId)
    try {
        return await invoke(command, args)  // ← args 中没有 operationId
    } finally {
        emitOperationEvent('finish', command, operationId)
    }
}
```

### 4.6 前端: cancelCurrentOperation（App.vue:167-185）

```javascript
async function cancelCurrentOperation() {
    const firstId = Array.from(pendingOperations.keys()).at(0)
    if (firstId) {
        await tauriCallSafe('cancel_operation', { operationId: firstId })
    }
    pendingOperations.clear()  // ← 清除所有
}
```

---

## 5. 修改后代码

### 5.1 新增文件: operations.rs

```rust
// src-tauri/src/operations.rs

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Running,
    Done,
}

struct OperationEntry {
    token: CancellationToken,
    status: OperationStatus,
    started_at: Instant,
    command: String,
}

#[derive(Default)]
pub struct OperationManager {
    operations: Mutex<HashMap<String, OperationEntry>>,
}

impl OperationManager {
    pub fn new() -> Self {
        Self {
            operations: Mutex::new(HashMap::new()),
        }
    }

    /// 注册新操作，返回 CancellationToken。
    pub fn begin(&self, operation_id: &str, command: &str) -> CancellationToken {
        let token = CancellationToken::new();
        let entry = OperationEntry {
            token: token.clone(),
            status: OperationStatus::Running,
            started_at: Instant::now(),
            command: command.to_string(),
        };
        if let Ok(mut map) = self.operations.lock() {
            map.insert(operation_id.to_string(), entry);
        }
        token
    }

    /// 取消指定操作。触发 CancellationToken，执行层自行检查并退出。
    pub fn cancel(&self, operation_id: &str) -> bool {
        if let Ok(map) = self.operations.lock() {
            if let Some(entry) = map.get(operation_id) {
                entry.token.cancel();
                return true;
            }
        }
        false
    }

    /// 取消所有操作（仅用于 app shutdown）。
    pub fn cancel_all(&self) {
        if let Ok(map) = self.operations.lock() {
            for entry in map.values() {
                entry.token.cancel();
            }
        }
    }

    /// 检查操作是否已取消。
    pub fn is_cancelled(&self, operation_id: &str) -> bool {
        if let Ok(map) = self.operations.lock() {
            if let Some(entry) = map.get(operation_id) {
                return entry.token.is_cancelled();
            }
        }
        false
    }

    /// 标记操作完成，移除注册。
    pub fn finish(&self, operation_id: &str) {
        if let Ok(mut map) = self.operations.lock() {
            map.remove(operation_id);
        }
    }

    /// 获取活跃操作列表。
    pub fn list_active(&self) -> Vec<String> {
        if let Ok(map) = self.operations.lock() {
            map.keys().cloned().collect()
        } else {
            vec![]
        }
    }

    /// 从前端 args 中提取 operation_id。
    /// 如果 args 中没有 operation_id，自动生成。
    pub fn extract_operation_id(&self, args: &serde_json::Value, command: &str) -> String {
        args.get("operationId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}:{}", command, uuid::Uuid::new_v4()))
    }
}
```

### 5.2 修改: run_managed（commands/mod.rs）

```rust
// 新增 run_managed，与 run_blocking 并存

use tokio_util::sync::CancellationToken;

/// 通用操作管理封装。替代 run_blocking，自动注册/注销操作。
pub async fn run_managed<T, F>(
    manager: &crate::operations::OperationManager,
    command: &str,
    operation_id: Option<String>,
    task: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(CancellationToken) -> anyhow::Result<T> + Send + 'static,
{
    let op_id = operation_id.unwrap_or_else(|| {
        format!("{}:{}", command, uuid::Uuid::new_v4())
    });
    let token = manager.begin(&op_id, command);

    let result = tauri::async_runtime::spawn_blocking(move || task(token))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string());

    manager.finish(&op_id);
    result
}

/// 旧的 run_blocking 保留不动，快速命令继续使用。
pub async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
```

### 5.3 修改: cancel_operation（commands/system.rs）

```rust
#[tauri::command]
pub fn cancel_operation(
    manager: tauri::State<'_, std::sync::Arc<crate::operations::OperationManager>>,
    operation_id: String,  // ← 使用这个参数
) -> Result<bool, String> {
    Ok(manager.cancel(&operation_id))  // ← 只取消指定操作
}
```

### 5.4 修改: cancelCurrentOperation（App.vue）

```javascript
async function cancelCurrentOperation() {
    const firstId = Array.from(pendingOperations.keys()).at(0)
    if (firstId) {
        const result = await tauriCallSafe('cancel_operation', { operationId: firstId })
        if (result.ok) {
            pendingOperations.delete(firstId)  // ← 只删除被取消的那一个
        }
    }
}
```

### 5.5 修改: tauriBridge.js 注入 operationId

```javascript
export async function tauriCall(command, args = {}) {
    const operationId = nextOperationId(command)
    emitOperationEvent('start', command, operationId)
    try {
        return await invoke(command, { ...args, operationId })  // ← 注入
    } catch (err) {
        void logError('tauri.bridge', `${command} failed`, { error: err })
        throw err
    } finally {
        emitOperationEvent('finish', command, operationId)
    }
}

export async function tauriCallSafe(command, args = {}) {
    const operationId = nextOperationId(command)
    emitOperationEvent('start', command, operationId)
    try {
        const result = await invoke(command, { ...args, operationId })  // ← 注入
        return { ok: true, data: result }
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        void logError('tauri.bridge', `${command} failed`, { message, stack: err?.stack })
        return { ok: false, error: message }
    } finally {
        emitOperationEvent('finish', command, operationId)
    }
}
```

### 5.6 修改: command_output_cancellable（external/mod.rs）

```rust
/// 支持用户主动取消的子进程执行。
///
/// 不设固定超时——大文件慢就慢，不会被误杀。
/// 通过 CancellationToken 支持用户主动取消。
/// 通过 idle_timeout 可选检测无响应（不自动 kill）。
pub fn command_output_cancellable(
    command: &mut Command,
    token: &CancellationToken,
    idle_timeout: Option<Duration>,
) -> anyhow::Result<Output> {
    hide_command_window(command);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    loop {
        // 正常结束
        if child.try_wait()?.is_some() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout).ok();
            }
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr).ok();
            }
            let status = child.wait()?;
            return Ok(Output { status, stdout, stderr });
        }

        // 用户主动取消 → kill 子进程
        if token.is_cancelled() {
            child.kill().ok();
            child.wait().ok();
            anyhow::bail!("用户取消操作");
        }

        // 无响应检测（可选，不自动 kill）
        if let Some(_idle) = idle_timeout {
            // TODO: 检测子进程是否有新的 stdout/stderr 输出
            // 如果长时间无输出，返回错误让前端提示用户
            // 但不自动 kill
        }

        std::thread::sleep(Duration::from_millis(30));
    }
}

/// 旧的 command_output_with_timeout 保留不动。
/// 仅用于 managed tool 下载等确实需要固定超时的场景。
pub fn command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> anyhow::Result<Output> {
    // 保持原有实现不变
    ...
}
```

### 5.7 修改: ConversionState 忙等待 → Condvar

```rust
// lib.rs

pub struct ConversionState {
    response: Arc<(Mutex<u8>, std::sync::Condvar)>,
}

impl ConversionState {
    pub fn new() -> Self {
        Self {
            response: Arc::new((Mutex::new(0), std::sync::Condvar::new())),
        }
    }

    pub fn set_response(&self, value: u8) {
        let (lock, cvar) = &*self.response;
        let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
        *state = value;
        cvar.notify_all();
    }

    /// 等待用户响应。不再忙等待，使用 Condvar 阻塞。
    pub fn wait_for_user_response(&self, _app: &tauri::AppHandle) -> bool {
        let (lock, cvar) = &*self.response;
        let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
        while *state == 0 {
            state = cvar.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        *state == 1
    }
}
```

同时修改 `respond_conversion_timeout` 命令：
```rust
#[tauri::command]
pub fn respond_conversion_timeout(
    state: tauri::State<'_, std::sync::Arc<crate::ConversionState>>,
    continue_waiting: bool,
) -> Result<(), String> {
    state.set_response(if continue_waiting { 1 } else { 2 });
    Ok(())
}
```

---

## 6. 影响分析

### 6.1 需要修改的文件

| 文件 | 修改内容 | 兼容性 | 风险 | 阶段 |
|------|----------|--------|------|------|
| `Cargo.toml` | 添加 `tokio-util` 依赖 | ✅ | 低 | A |
| `lib.rs` | 新增 `operations` mod；注册 OperationManager；修改 ConversionState | ✅ | 中 | A |
| `operations.rs` | **新增文件** | ✅ | 低 | A |
| `commands/mod.rs` | 新增 `run_managed` | ✅ | 低 | A |
| `commands/system.rs` | 修改 `cancel_operation` 使用 OperationManager | ✅ | 低 | B |
| `external/mod.rs` | 新增 `command_output_cancellable` | ✅ | 低 | B |
| `tauriBridge.js` | tauriCall/tauriCallSafe 注入 operationId | ✅ | 低 | B |
| `App.vue` | cancelCurrentOperation 只删除被取消的操作 | ✅ | 低 | B |
| `commands/pdf.rs` | 长时间命令改用 run_managed | ⚠️ | 中 | C |
| `commands/template.rs` | batch_render 改用 run_managed | ⚠️ | 中 | C |
| `commands/video.rs` | extract_frames 改用 run_managed | ⚠️ | 低 | C |
| `commands/image_paddler.rs` | run_image_paddler 改用 run_managed | ⚠️ | 低 | C |
| `pdf/qpdf.rs` | 改用 command_output_cancellable | ✅ | 低 | C |
| `pdf/evidence.rs` | 改用 command_output_cancellable | ⚠️ | 中 | C |

### 6.2 不需要修改的文件

- 所有快速命令（inspect、get_page_count、open_path 等）——继续用 `run_blocking`，无需改动
- 所有前端模块的业务逻辑——operationId 由 tauriBridge 自动注入，模块无感知
- SubprocessRegistry——Phase D 才删除，Phase A-C 与 OperationManager 并存

### 6.3 调用链追踪

**正常操作流程**：
```
前端 tauriCall('batch_overlay_pdf_text', args)
  → args.operationId = 'batch_overlay_pdf_text:42'
  → invoke('batch_overlay_pdf_text', { ...args, operationId })
  → Rust commands/pdf.rs::batch_overlay_pdf_text
    → run_managed(&manager, "batch_overlay_pdf_text", Some("batch_overlay_pdf_text:42"), |token| {
        for item in items {
          if token.is_cancelled() { return Err("用户取消"); }
          process_item(item)?;
        }
        Ok(result)
      })
    → manager.begin("batch_overlay_pdf_text:42", "batch_overlay_pdf_text")
    → spawn_blocking(task)
    → [task 执行中]
    → manager.finish("batch_overlay_pdf_text:42")
  → 返回结果
```

**取消流程**：
```
前端 cancelCurrentOperation()
  → firstId = 'batch_overlay_pdf_text:42'
  → tauriCallSafe('cancel_operation', { operationId: firstId })
  → Rust commands/system.rs::cancel_operation
    → manager.cancel('batch_overlay_pdf_text:42')
      → entry.token.cancel()  // 触发取消信号
      → return true
  → 前端 pendingOperations.delete(firstId)

[task 内部]
  → token.is_cancelled() == true
  → return Err("用户取消")
  → manager.finish()
```

---

## 7. 测试计划

### 7.1 单元测试
- [ ] OperationManager.begin/cancel/is_cancelled/finish
- [ ] OperationManager.cancel_all（验证所有 token 被取消）
- [ ] OperationManager.list_active
- [ ] OperationManager.extract_operation_id（有/无 operationId）
- [ ] ConversionState Condvar 等待和通知
- [ ] command_output_cancellable 用户取消场景

### 7.2 集成测试
- [ ] 启动两个并发操作（batch_render + PDF preview），取消其中一个，验证另一个继续运行
- [ ] 取消不存在的 operationId，验证返回 false
- [ ] 快速连续取消同一操作，验证幂等性
- [ ] 大文件操作不被误杀（验证不设固定超时）

### 7.3 手动验证
- [ ] Doclet 动画在操作取消后正确消失
- [ ] 30 秒取消按钮正确显示和隐藏
- [ ] 取消批量渲染后，可以立即开始新操作
- [ ] Word → PDF 转换超时弹窗仍然正常工作（ConversionState 改造后）

---

## 8. 回退方案

1. Phase A 完全无侵入——OperationManager 与 SubprocessRegistry 并存
2. Phase B 只改 cancel_operation 命令和前端——旧的 cancel_all 逻辑可以通过 SubprocessRegistry 调用
3. `git stash` / `git checkout main` 随时回退
4. 如果新方案有问题，可以用 `#[cfg(feature = "legacy-operations")]` 条件编译保留旧逻辑

---

## 9. 分阶段实施

### Phase A: 基础设施（不影响现有功能）
- [ ] `Cargo.toml` 添加 `tokio-util` 依赖
- [ ] 新建 `operations.rs`，实现 OperationManager
- [ ] 在 `lib.rs` 中注册 OperationManager（与 SubprocessRegistry 并存）
- [ ] `commands/mod.rs` 新增 `run_managed`（与 run_blocking 并存）
- [ ] 写单元测试
- [ ] **验证**：cargo test 通过，现有功能不受影响

### Phase B: 前端 + cancel_operation
- [ ] 修改 `tauriBridge.js`，注入 operationId 到 args
- [ ] 修改 `cancel_operation` 使用 OperationManager
- [ ] 修改 `App.vue` cancelCurrentOperation 只删除被取消的操作
- [ ] `external/mod.rs` 新增 `command_output_cancellable`
- [ ] 修改 ConversionState 忙等待 → Condvar
- [ ] **验证**：cargo test + npm test 通过，手动验证取消按钮

### Phase C: 迁移关键命令
- [ ] `batch_overlay_pdf_text` 改用 run_managed
- [ ] `batch_render_from_xlsx` 改用 run_managed
- [ ] `extract_frames` 改用 run_managed
- [ ] `run_image_paddler` 改用 run_managed
- [ ] `qpdf.rs` 改用 command_output_cancellable
- [ ] `evidence.rs` 改用 command_output_cancellable
- [ ] **验证**：批量渲染可取消，PDF 处理可取消

### Phase D: 清理
- [ ] 删除 SubprocessRegistry（如所有调用已迁移）
- [ ] 删除 ConversionState（如已统一进 OperationManager）
- [ ] 删除旧的 command_output_with_timeout 中的超时 kill 逻辑（保留给 managed tool 下载）
- [ ] 更新 architecture.md 文档

---

## 10. 变更日志

- 2026-08-07 22:00 — v1 初稿
- 2026-08-07 23:30 — v2 重写：引入第四通用层 run_managed；区分超时和无响应；统一异步任务和外部子进程管理；去掉固定超时自动 kill
