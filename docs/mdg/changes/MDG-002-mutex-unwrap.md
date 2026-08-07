# MDG-002: Mutex::unwrap() 安全模式

## 状态：🔴 未开始
## 优先级：P0
## 关联：BUG-002

## 修改目标
将 `MANIFEST_CACHE.lock().unwrap()` 替换为安全模式，防止 Mutex 中毒时 panic。

## 影响分析
| 调用位置 | 文件:行号 | 当前行为 | 修改后兼容 |
|----------|-----------|----------|------------|
| cached_manifest 读取 | commands/template.rs:16 | unwrap() | unwrap_or_else |
| cached_manifest 写入 | commands/template.rs:32 | unwrap() | unwrap_or_else |

## 修改方案
```rust
// 修改前：
MANIFEST_CACHE.lock().unwrap()
// 修改后：
MANIFEST_CACHE.lock().unwrap_or_else(|e| e.into_inner())
```

## 测试计划
- [ ] cargo test 通过
- [ ] 手动触发 Mutex 中毒场景验证不 panic

## 变更日志
- 2026-08-07 — 🔴 未开始
