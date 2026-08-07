# MDG-005: 生产代码清理

## 状态：✅ 已完成
## 优先级：P1

## 清理项

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 1 | eprintln! 调试输出 | header_footer.rs:238 | ✅ MDG-008 已清理 |
| 2 | 未使用 mime 变量 | commands/system.rs:67 | ✅ 已不存在 |
| 3 | all_descriptors dead code | services/module_registry.rs | ✅ 已删除 |
| 4 | #[allow(dead_code)] | docx_template/index.rs | ✅ 有注释说明，保留 |

## 变更日志
- 2026-08-07 — 初稿
- 2026-08-07 — ✅ 完成（删除 module_registry.rs，其他项已由 MDG-008 清理）
