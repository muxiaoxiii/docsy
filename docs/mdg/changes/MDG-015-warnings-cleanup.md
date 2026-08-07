# MDG-015: 编译警告清零 + 诊断日志接入

## 状态：✅ 已完成
## 优先级：P1

## 背景
MDG-011 新增了诊断 API（`warn`/`debug`/`error_with_op`）但未接入任何调用方，
导致编译警告。同时存在其他遗留警告。

## 警告清单

| # | Warning | 位置 | 处理方式 | 状态 |
|---|---------|------|----------|------|
| W1 | `batch_overlay` unused import | overlay.rs:7 | 删除 re-export | ✅ |
| W2 | `AtomicU8` unused import | lib.rs:14 | 删除导入 | ✅ |
| W3 | `warn` never used | app_log.rs:82 | operations.rs finish 中 cancelled 路径使用 | ✅ |
| W4 | `debug` never used | app_log.rs:90 | batch.rs generate_filename 接入 | ✅ |
| W5 | `error_with_op` never used | app_log.rs:98 | operations.rs finish 中 failed 路径使用 | ✅ |
| W6 | `command_output_cancellable` never used | external/mod.rs:262 | 删除函数（功能已被 run_managed 覆盖） | ✅ |
| W7 | `cancel_all`/`is_cancelled` never used | operations.rs:137 | 删除方法，测试改用 cancel | ✅ |
| W8 | `apply_bookmark` never used | header_footer.rs:378 | 标注 #[cfg(test)] | ✅ |

## 变更日志
- 2026-08-08 — ✅ 完成（0 warnings，159 tests passed）
