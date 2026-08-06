# MDG — Make Docsy Great

> 启动日期：2026-08-07
> 项目：`/Users/only/Documents/PythonProgram/Docsy`
> 技术栈：Tauri 2 + Rust + Vue 3 + Element Plus
> 流程规范：`mdg-workflow` skill（变更管理六步协议）

---

## 进度看板

| ID | 变更名 | 优先级 | 状态 | 分支 | 变更单 |
|----|--------|--------|------|------|--------|
| MDG-001 | cancel_operation 第四通用层重构 | P0 | 🟢 基础设施完成 | mdg/MDG-001-operation-manager | [changes/MDG-001](changes/MDG-001-cancel-operation.md) |
| MDG-002 | Mutex::unwrap() 安全模式 | P0 | 🔴 未开始 | — | [changes/MDG-002](changes/MDG-002-mutex-unwrap.md) |
| MDG-003 | 代码重复消除 | P1 | 🔴 未开始 | — | [changes/MDG-003](changes/MDG-003-dedup.md) |
| MDG-004 | 书签页码偏移修复 | P0 | 🔴 未开始 | — | [changes/MDG-004](changes/MDG-004-bookmark-offset.md) |
| MDG-005 | 书签写入原子操作 | P0 | 🔴 未开始 | — | [changes/MDG-005](changes/MDG-005-bookmark-atomic.md) |
| MDG-006 | 输出路径碰撞检查 | P0 | 🔴 未开始 | — | [changes/MDG-006](changes/MDG-006-output-collision.md) |
| MDG-007 | same_path 统一为公共函数 | P1 | 🔴 未开始 | — | — |
| MDG-008 | 生产代码 eprintln! 清理 | P1 | 🔴 未开始 | — | — |
| MDG-009 | TextOverlay 方法命名修正 | P1 | 🔴 未开始 | — | — |
| MDG-010 | 前后端默认值统一 | P1 | 🔴 未开始 | — | — |

---

## 文档索引

| 文档 | 说明 |
|------|------|
| **[problem-inventory.md](problem-inventory.md)** | 全部问题分组总表（9 个 MDG，32 个问题） |
| **[changes/](changes/)** | 变更单目录（含影响分析、代码方案、回退方案） |

## 变更管理协议

**六步流程**：① 定位 → ② 影响分析 → ③ 记录变更单 → ④ 修改 → ⑤ 验证 → ⑥ 提交

详见 `mdg-workflow` skill 或变更单模板。

---

## 审阅报告索引

| 文档 | 内容 |
|------|------|
| `review-summary.md` | 综合审阅报告 |
| `bug-report.md` | Bug 清单（P0×6, P1×12, P2×19, P3×17） |
| `deep-review-template-core.md` | template + core 深度审阅 |
| `deep-review-pdf.md` | PDF 模块深度审阅 |
| `deep-review-other.md` | 其他模块深度审阅 |
| `ui-review.md` | UI/UX 审查 |
| `design-vs-implementation.md` | 设计 vs 代码差异 |
| `improvement-plan.md` | 完善方案（含代码片段） |

---

## 日志

- [2026-08-07](logs/2026-08-07.md) — 项目启动，完成全部审阅，建立 MDG 流程
