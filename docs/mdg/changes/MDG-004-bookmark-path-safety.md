# MDG-004: 书签 + 路径安全

## 状态：🔴 未开始
## 优先级：P0
## 4 个问题：
1. evidence_session.rs:295 — 书签页码偏移错位
2. header_footer.rs:324 — 书签写入非原子
3. header_footer.rs:726 — 输出路径碰撞静默覆盖
4. commands/template.rs:295 — import_template 覆盖已有模板

## 变更日志
- 2026-08-07 — 🔴 未开始
