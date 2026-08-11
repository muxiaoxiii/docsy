// src-tauri/src/operations.rs
//
// MDG-001: 第四通用层 — 操作生命周期管理
//
// 异步任务取消管理（CancellationToken）。
// 与 SubprocessRegistry（PID-based 子进程取消）互补，由 cancel_operation 统一入口。
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

struct OperationEntry {
    token: CancellationToken,
    started_at: Instant,
    command: String,
}

/// 操作生命周期事件（Rust → 前端）。
/// 前端 Doclet 动画系统后续订阅这些事件来驱动动画。
/// MDG-011: 增加结束原因，供诊断系统使用。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub operation_id: String,
    pub command: String,
}

/// 操作结束事件（带结束原因）。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationFinishedEvent {
    pub operation_id: String,
    pub command: String,
    pub outcome: OperationOutcome,
    pub elapsed_ms: u64,
}

/// 长任务阶段更新事件。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgressEvent {
    pub operation_id: String,
    pub label: String,
}

/// 操作结束原因。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationOutcome {
    Completed,
    Cancelled,
    Failed,
}

/// 活跃操作摘要（供前端查询和崩溃报告）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveOperation {
    pub operation_id: String,
    pub command: String,
    pub elapsed_ms: u64,
}

/// 操作生命周期管理器。
///
/// CancellationToken-based 异步任务管理。
/// 与 SubprocessRegistry（PID-based 子进程取消）互补。
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
            started_at: Instant::now(),
            command: command.to_string(),
        };
        if let Ok(mut map) = self.operations.lock() {
            map.insert(operation_id.to_string(), entry);
        }

        // 发射 started 事件（供 Doclet 动画接驳）
        self.emit_event("docsy-operation-started", operation_id, command);

        // 诊断日志
        crate::app_log::info_with_op(
            "operations",
            "操作开始",
            serde_json::json!({ "command": command }),
            operation_id,
        );

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

    /// 取消所有操作。用于窗口关闭时清理。
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
    /// 通过检查 token 是否已取消来判断结束原因。
    pub fn finish(&self, operation_id: &str, failed: bool) {
        // 先查询取消状态（使用 is_cancelled 方法），再移除条目
        let was_cancelled = self.is_cancelled(operation_id);
        let (command, started_at) = if let Ok(mut map) = self.operations.lock() {
            if let Some(entry) = map.remove(operation_id) {
                (Some(entry.command), Some(entry.started_at))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        if let (Some(cmd), Some(start)) = (command, started_at) {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let outcome = if was_cancelled {
                OperationOutcome::Cancelled
            } else if failed {
                OperationOutcome::Failed
            } else {
                OperationOutcome::Completed
            };

            // 诊断日志：按结果分级
            let ctx = serde_json::json!({ "command": &cmd, "elapsed_ms": elapsed_ms });
            if was_cancelled {
                crate::app_log::warn("operations", "操作取消", ctx);
            } else if failed {
                crate::app_log::error_with_op("operations", "操作失败", ctx, operation_id);
            } else {
                crate::app_log::info_with_op("operations", "操作完成", ctx, operation_id);
            }

            // 发射 enriched finished 事件（Doclet 动画 + 诊断系统）
            if let Ok(h) = self.app_handle.lock() {
                if let Some(app) = h.as_ref() {
                    let _ = app.emit(
                        "docsy-operation-finished",
                        OperationFinishedEvent {
                            operation_id: operation_id.to_string(),
                            command: cmd,
                            outcome,
                            elapsed_ms,
                        },
                    );
                }
            }
        }
    }

    /// 更新前端显示的阶段文字，不改变操作生命周期。
    pub fn update(&self, operation_id: &str, label: impl Into<String>) {
        let label = label.into();
        let exists = if let Ok(map) = self.operations.lock() {
            map.contains_key(operation_id)
        } else {
            false
        };
        if !exists {
            return;
        }
        if let Ok(h) = self.app_handle.lock() {
            if let Some(app) = h.as_ref() {
                let _ = app.emit(
                    "docsy-operation-progress",
                    OperationProgressEvent {
                        operation_id: operation_id.to_string(),
                        label,
                    },
                );
            }
        }
    }

    /// 获取活跃操作摘要（用于前端查询、取消确认、崩溃报告）。
    pub fn list_active(&self) -> Vec<ActiveOperation> {
        if let Ok(map) = self.operations.lock() {
            map.iter()
                .map(|(id, entry)| ActiveOperation {
                    operation_id: id.clone(),
                    command: entry.command.clone(),
                    elapsed_ms: entry.started_at.elapsed().as_millis() as u64,
                })
                .collect()
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

        manager.finish("op-1", false);
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
        let ids: Vec<&str> = active.iter().map(|a| a.operation_id.as_str()).collect();
        assert!(ids.contains(&"op-1"));
        assert!(ids.contains(&"op-2"));
    }

    #[test]
    fn test_cancel_is_idempotent() {
        let manager = OperationManager::new();
        manager.begin("op-1", "cmd1");

        assert!(manager.cancel("op-1"));
        assert!(manager.cancel("op-1")); // 第二次也返回 true（entry 还在）
    }
}
