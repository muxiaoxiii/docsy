# MDG-015: 编译警告清理 + 诊断日志接入

## 状态：🟡 进行中
## 优先级：P1

## 背景
MDG-011 新增了诊断 API（`warn`/`debug`/`error_with_op`）但未接入任何调用方，
导致编译警告。同时存在其他遗留警告。

## 警告清单

| # | Warning | 位置 | 处理方式 |
|---|---------|------|----------|
| W1 | `batch_overlay` unused import | overlay.rs:7 | 删除 re-export |
| W2 | `AtomicU8` unused import | lib.rs:14 | 删除导入 |
| W3 | `warn` never used | app_log.rs:82 | 在 operations.rs finish 中 cancelled 路径使用 |
| W4 | `debug` never used | app_log.rs:90 | 在关键路径接入 debug 日志 |
| W5 | `error_with_op` never used | app_log.rs:98 | 在 operations.rs finish 中 failed 路径使用 |
| W6 | `command_output_cancellable` never used | external/mod.rs:262 | 删除函数 |
| W7 | `cancel_all`/`is_cancelled` never used | operations.rs:137 | 标注 `#[cfg(test)]`（仅测试用） |
| W8 | `apply_bookmark` never used | header_footer.rs:378 | 标注 `#[cfg(test)]`（仅测试用） |

## 变更日志
- 2026-08-08 — 🟡 进行中
