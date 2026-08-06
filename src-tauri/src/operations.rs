// src-tauri/src/operations.rs
//
// MDG-001: 第四通用层 — 操作生命周期管理
//
// 替代 SubprocessRegistry，统一管理异步任务和外部子进程的取消。
// 设计原则：
//   - 不设固定超时自动 kill（大文件慢就慢）
//   - 只支持用户主动取消（通过 CancellationToken）
//   - 与 run_blocking 并存，新命令用 run_managed，旧命令不改
//   - 发射生命周期事件（docsy-operation-started/finished），供前端 Doclet 动画接驳

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tauri::Emitter;
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

/// 操作生命周期事件（Rust → 前端）。
/// 前端 Doclet 动画系统后续订阅这些事件来驱动动画。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub operation_id: String,
    pub command: String,
}

/// 操作生命周期管理器。
///
/// 替代 SubprocessRegistry，统一管理异步任务和外部子进程。
/// 通过 CancellationToken 传递取消信号，执行层自行检查并退出。
pub struct OperationManager {
    operations: Mutex<HashMap<String, OperationEntry>>,
    app_handle: Mutex<Option<tauri::AppHandle>>,
}

impl Default for OperationManager {
    fn default() -> Self {
        Self {
            operations: Mutex::new(HashMap::new()),
            app_handle: Mutex::new(None),
        }
    }
}

impl OperationManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 AppHandle，用于发射生命周期事件。
    /// 在 Tauri setup 阶段调用一次。
    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        if let Ok(mut h) = self.app_handle.lock() {
            *h = Some(handle);
        }
    }

    /// 注册新操作，返回 CancellationToken。
    ///
    /// 自动发射 `docsy-operation-started` 事件到前端。
    /// 长时间任务应定期检查 token.is_cancelled()，
    /// 发现取消后尽快清理资源并返回错误。
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

        // 发射 started 事件（供 Doclet 动画接驳）
        self.emit_event("docsy-operation-started", operation_id, command);

        token
    }

    /// 取消指定操作。触发 CancellationToken，执行层自行检查并退出。
    ///
    /// 返回 true 表示找到并取消了该操作，false 表示操作不存在。
    pub fn cancel(&self, operation_id: &str) -> bool {
        if let Ok(map) = self.operations.lock() {
            if let Some(entry) = map.get(operation_id) {
                entry.token.cancel();
                return true;
            }
        }
        false
    }

    /// 取消所有操作。仅用于 app shutdown 清理。
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
    ///
    /// 自动发射 `docsy-operation-finished` 事件到前端。
    pub fn finish(&self, operation_id: &str) {
        let command = if let Ok(mut map) = self.operations.lock() {
            map.remove(operation_id).map(|e| e.command)
        } else {
            None
        };

        // 发射 finished 事件（供 Doclet 动画接驳）
        if let Some(cmd) = command {
            self.emit_event("docsy-operation-finished", operation_id, &cmd);
        }
    }

    /// 获取活跃操作列表（用于前端查询和调试）。
    pub fn list_active(&self) -> Vec<String> {
        if let Ok(map) = self.operations.lock() {
            map.keys().cloned().collect()
        } else {
            vec![]
        }
    }

    /// 内部：发射生命周期事件到前端。
    fn emit_event(&self, event_name: &str, operation_id: &str, command: &str) {
        if let Ok(h) = self.app_handle.lock() {
            if let Some(app) = h.as_ref() {
                let _ = app.emit(
                    event_name,
                    OperationEvent {
                        operation_id: operation_id.to_string(),
                        command: command.to_string(),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_begin_and_cancel() {
        let manager = OperationManager::new();
        let token = manager.begin("op-1", "test_command");

        assert!(!token.is_cancelled());
        assert!(!manager.is_cancelled("op-1"));
        assert!(manager.cancel("op-1"));
        assert!(token.is_cancelled());
        assert!(manager.is_cancelled("op-1"));
    }

    #[test]
    fn test_cancel_nonexistent() {
        let manager = OperationManager::new();
        assert!(!manager.cancel("nonexistent"));
    }

    #[test]
    fn test_finish_removes_entry() {
        let manager = OperationManager::new();
        manager.begin("op-1", "test_command");
        assert_eq!(manager.list_active().len(), 1);

        manager.finish("op-1");
        assert_eq!(manager.list_active().len(), 0);
        assert!(!manager.is_cancelled("op-1")); // 已移除，返回 false
    }

    #[test]
    fn test_cancel_all() {
        let manager = OperationManager::new();
        let token1 = manager.begin("op-1", "cmd1");
        let token2 = manager.begin("op-2", "cmd2");

        manager.cancel_all();
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_multiple_operations_independent() {
        let manager = OperationManager::new();
        let token1 = manager.begin("op-1", "cmd1");
        let token2 = manager.begin("op-2", "cmd2");

        manager.cancel("op-1");
        assert!(token1.is_cancelled());
        assert!(!token2.is_cancelled()); // op-2 不受影响
    }

    #[test]
    fn test_list_active() {
        let manager = OperationManager::new();
        assert!(manager.list_active().is_empty());

        manager.begin("op-1", "cmd1");
        manager.begin("op-2", "cmd2");
        let active = manager.list_active();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"op-1".to_string()));
        assert!(active.contains(&"op-2".to_string()));
    }

    #[test]
    fn test_cancel_is_idempotent() {
        let manager = OperationManager::new();
        manager.begin("op-1", "cmd1");

        assert!(manager.cancel("op-1"));
        assert!(manager.cancel("op-1")); // 第二次也返回 true（entry 还在）
    }
}
