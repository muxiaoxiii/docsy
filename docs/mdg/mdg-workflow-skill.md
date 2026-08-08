---
name: mdg-workflow
description: "MDG (Make Docsy Great) — Docsy 项目修复工作的标准流程。变更管理、影响分析、测试验证、进度追踪。"
version: 2.0.0
author: Jade Zhang
platforms: [linux, macos]
metadata:
  hermes:
    tags: [docsy, mdg, code-review, bugfix, refactoring]
---

# MDG — Make Docsy Great

## 项目概况

- **项目路径**：`/Users/only/Documents/PythonProgram/Docsy`
- **文档目录**：`/Users/only/Documents/PythonProgram/Docsy/docs/mdg/`
- **技术栈**：Tauri 2 + Rust + Vue 3 + Element Plus
- **审阅报告**：`docs/review-summary.md`、`docs/bug-report.md`、`docs/deep-review-*.md`、`docs/improvement-plan.md`

## 参考资料

- `references/tauri-operation-management.md` — Tauri 2 操作管理架构模式（Channel、CancellationToken、run_managed、参数注入限制）
- `references/change-document-template.md` — 变更单写作范例和结构要点
- `references/code-review-methodology.md` — 大项目三阶段并行审阅方法
- `references/docsy-bug-inventory.md` — Docsy 全部 Bug 总表（含状态追踪）
- `references/template-module-pitfalls.md` — 模板模块陷阱：标黄空白 mark、诊断写入、sourceDocx 为空、autocomplete 丢失、Word 域结构
- `references/template-preview-pitfalls.md` — 模板预览与选区陷阱：补选提取失败、图例冗余、统一预览模块设计
- `references/filename-token-input.md` — FilenameTokenInput 组件设计与 token 模式
- `references/tauri-capabilities-scope.md` — Tauri v2 fs 权限 scope 配置（forbidden path 修复）
- `references/evidence-header-footer-module.md` — 证据处理模块页眉页脚系统代码结构与设计分析
- `references/glm-audit-to-mdg-flow.md` — GLM 审计 → MDG 实施的完整流程（含 MDG-017 实际执行记录）

## 变更管理协议（必须遵守）

### 核心原则

**每一个修改点，必须完成以下六步：**

```
① 定位 → ② 影响分析 → ③ 记录 → ④ 修改 → ⑤ 验证 → ⑥ 提交
```

### ① 定位
- 精确到文件路径 + 行号 + 函数名
- 读取当前代码，理解上下文

### ② 影响分析（最关键）
- 用 `search_files` / `grep` 找出该函数/变量/类型的**全部调用位置**
- 包括：直接调用、间接引用、测试文件中的使用、类型依赖
- 每个调用点记录：文件、行号、调用方式、是否兼容
- **修改前必须确认所有调用点兼容，否则不能改**

**架构级变更的额外要求：**
- 读取现有设计文档（docs/*.md），理解原始设计意图
- 搜索框架官方文档和社区最佳实践（如 Tauri docs.rs、Vue RFC）
- 评估是否有更通用的方案（不只是修一个点，而是建立通用层）
- 画出完整的调用链（前端 → IPC → Rust 命令 → 业务逻辑 → 文件系统）
- 确认新方案是否能替代多个相关问题（如 OperationManager 同时解决 cancel + progress + timeout）
- 对比多个候选方案，列出 tradeoff 表

### ③ 记录变更单
写入 `docs/mdg/changes/` 目录：

```markdown
# MDG-XXX: 变更名

## 状态：🔴 未开始 / 🟡 进行中 / 🟢 已完成 / ❌ 已取消
## 优先级：P0 / P1 / P2 / P3
## 关联 Bug：bug-report.md BUG-XXX

## 修改目标
一句话说明为什么要改

## 修改前代码
完整贴出当前代码（含文件路径和行号）

## 修改后代码
完整贴出计划修改的代码

## 影响分析
| 调用位置 | 文件:行号 | 当前行为 | 修改后兼容 | 风险 |
|----------|-----------|----------|------------|------|
| ...      | ...       | ...      | ✅/⚠️/❌   | ...  |

## 测试计划
- [ ] 单元测试：...
- [ ] 集成验证：...
- [ ] 手动验证：...

## 回退方案
如果出问题，如何恢复

## 变更日志
- YYYY-MM-DD HH:MM — 状态变更说明
```

### ④ 修改
- 在 git 分支上执行：`git checkout -b mdg/MDG-XXX-变更名`
- 用 `patch` 精确修改，不用 `write_file` 重写整个文件
- 每次修改后 `git diff` 确认只改了该改的地方

### ⑤ 验证
- 运行相关测试：`cargo test` / `npm run test`
- 检查编译警告新增
- 手动验证（如需要）

### ⑥ 提交
- `git commit -m "MDG-XXX: 变更说明"`
- 更新变更单状态为 🟢 已完成

## 目录结构

```
docsy/docs/mdg/
├── README.md                    # 项目总览 + 进度看板
├── changes/                     # 变更单目录
│   ├── MDG-001-cancel-operation.md
│   ├── MDG-002-xxx.md
│   └── ...
└── logs/                        # 作业日志
    ├── 2026-08-07.md
    └── ...
```

## 进度追踪

### README.md 中维护进度看板

```markdown
| ID | 变更名 | 优先级 | 状态 | 分支 | 变更单 |
|----|--------|--------|------|------|--------|
| MDG-001 | cancel_operation 重构 | P0 | 🟡 进行中 | mdg/MDG-001 | [链接] |
```

### 每次作业记录到 logs/

```markdown
# MDG 作业日志 — YYYY-MM-DD

## 今日目标
- ...

## 完成的工作
- MDG-XXX: ...

## 遇到的问题
- ...

## 明日计划
- ...
```

## P0 Bug 清单（修复顺序） — 全部完成 2026-08-07

1. ✅ **MDG-001**: 操作管理层重构（第四通用层，含 ConversionState Condvar 修复）— 已合并
2. ✅ **MDG-002**: `Mutex::unwrap()` 安全模式 — 已合并
3. ✅ **MDG-003**: 代码重复消除（same_path 4→0, temp_named_path 7→0, fnv1a_hash 3→0, 前端 3 函数去重）— 已合并
4. ✅ **MDG-004**: 书签 + 路径安全（import 覆盖保护 + bookmarkRemoveExisting bug 修复 + 死字段清理）— 已合并
5. ✅ **MDG-005**: 生产代码清理（删除 module_registry.rs dead code，eprintln!/mime 已由 MDG-008 清理）— 已完成
6. ✅ **MDG-006**: 输出路径碰撞（已有 unique_output_path + same_path 保护）

## 全部 MDG 状态（截至 2026-08-08）

| ID | 名称 | 状态 | 说明 |
|----|------|------|------|
| MDG-001 | 操作管理层重构 | ✅ | 已合并 |
| MDG-002 | Mutex::unwrap() | ✅ | 已合并 |
| MDG-003 | 代码重复消除 | ✅ | 已合并 |
| MDG-004 | 书签+路径安全 | ✅ | 已合并 |
| MDG-005 | 生产代码清理 | ✅ | 删除 module_registry.rs（dead code） |
| MDG-006 | 输出路径碰撞 | ✅ | 已有保护 |
| MDG-007 | same_path 统一 | ✅ | 合入 MDG-003 |
| MDG-008 | 生产代码清理 | ✅ | eprintln!→log::debug! + mime 死代码 |
| MDG-009 | TextOverlay 命名 | ✅ | 命名已一致 |
| MDG-010 | 前后端默认值统一 | ✅ | margin_mm 15→12 + filename_without_ext false→true |
| MDG-011 | 诊断系统 | ✅ | 日志基础设施+操作追踪+快照导出+前端服务 |
| MDG-012 | Word 域保护 | ✅ | 域深度跟踪 |
| MDG-013 | 模板文件名 | ✅ | 全部 6 Phase 完成 |
| MDG-014 | 预览+编辑 UI | ✅ | 全部 4 Phase 完成（DocumentPreview + 图例精简） |
| MDG-015 | 编译警告清零 | ✅ | 诊断日志接入 |
| MDG-016 | 证据处理页眉页脚优化 | ✅ | 160 tests passed |
| MDG-017 | GLM 全项目审计修复 | ✅ | P0+P1+P2 全部完成，19 个 commit |
| MDG-018 | GLM 审计剩余 + 测试回归修复 | ✅ | Phase 1+2+3 全部完成，13 个 commit |

## 架构诊断优先

**修复 bug 前，先诊断架构。** 表面 bug 往往是架构缺陷的症状。例如 `cancel_operation` 杀所有进程，表面是参数没用上，实际是前端/命令层/执行层三层断裂——只改参数是治标不治本。

诊断流程：
1. 追踪完整调用链（前端 → IPC → 命令层 → 执行层）
2. 找出断裂点（ID 不匹配、机制缺失、注册空洞）
3. 判断：是单点修复还是需要重构？
4. 如果需要重构，在变更单中给出分阶段实施方案

**Jade 不是程序员，需要你做专业判断。** 不要只报告 metrics（行数、文件数），要给出架构级的分析和建议。不要盲目建议"拆分大组件"——先检查是否已经拆过（子组件、composables），协调层 2000-3000 行可能是合理的。

**新架构方案需要先做技术调研。** 不要凭空设计——先读现有设计文档理解原始意图，搜索框架官方文档和社区最佳实践，评估多个候选方案的 tradeoff。例如 MDG-001 设计 v1 想注入 operationId 到 Tauri args，调研后发现 serde 会拒绝未知字段，改为 Rust 侧生成 ID。

## 代码审阅方法

**逐模块深度审阅优于一次性全量审阅。** 大项目（4 万行+）不能一次性塞给 Claude Code，context window 会爆导致产出肤浅。

正确做法：
1. 按模块拆分（如 template+core、pdf-tools、其他模块）
2. 每模块一次 Claude Code 调用，max-turns 50，逐文件读透
3. 产出是逐函数级分析（文件路径+行号+函数签名+依赖+bug）
4. 最后统稿合并

**产出质量检查：** 如果审阅报告只有概述级（模块名+行数），没有逐函数分析，说明没读透——重做。

### 功能级全链路审查（Jade 明确要求）

**审查一个功能时，不是"检查代码是否正确"，而是"从设计目的出发，追踪完整数据链路"。**

Jade 原话：「从插入书签命令开始检查吧」「深入到设计目的考虑整体书签功能」

错误做法：只看 Rust 端的函数实现，确认逻辑正确就结束。这会漏掉前端→后端的断裂。

正确做法：
1. **先理解设计目的**：这个功能要解决什么用户场景？（如：合并 PDF 后标记每个文件起始页，方便法官跳转）
2. **从 UI 入口开始追踪**：用户点击什么 → 前端组装什么数据 → 通过什么命令发送 → 后端哪个函数接收 → 中间经过哪些转换 → 最终写入什么
3. **逐环节检查一致性**：
   - UI 的 checkbox/radio 绑定的 ref 值是否真的传到了 composable？
   - composable 是否读取了 rules 中的值，还是硬编码了？
   - 发送到后端的 JSON 字段名是否与 Rust struct 的 serde rename 匹配？
   - 中间是否有"数据被组装了但从未被读取"的死字段？
4. **检查设计断裂**：UI 存在但值未传递、字段定义了但从未读取、注释说"已移至X"但X没有实现

**案例**：书签功能审查。只看 Rust 端 → `collect_merge_bookmarks` 偏移正确，`apply_bookmarks` 逻辑正确，结论"没问题"。但全链路审查发现：
- `bookmarkRemoveExisting` 在 composable 中硬编码为 `false`，用户勾选"删除已有书签"完全无效
- `HeaderFooterJob.bookmark`（单数）和 `BookmarkConfig.remove_existing` 是死字段，从未被读取
- 只有从 UI → composable → JSON → Rust 全链路追踪才能发现这些问题

### 审查时的死代码检测模式

struct 字段级别的死代码比函数级别更难发现，因为字段在定义、初始化、测试中都会出现，但可能从未被**读取**。检测方法：

```bash
# 找字段定义
grep -n "bookmark:" header_footer.rs
# 找所有引用（排除定义和初始化）
grep -n "\.bookmark\b" header_footer.rs | grep -v "bookmark_remove_existing\|bookmarks"
# 如果只有定义(行53)和初始化(行2303)，没有读取 → 死字段
```

### 硬编码值绕过用户设置的模式

这是一个常见的 bug 模式：UI 正确绑定 → rules 正确传递 → composable 硬编码忽略。检测方法：

```bash
# 检查 composable 中是否硬编码了用户设置的值
grep -n "RemoveExisting: false\|Enabled: false\|Mode: 'none'" composables/*.js
# 对比前端 rules 中是否有对应的设置
grep -n "RemoveExisting\|Enabled\|Mode" views/*.vue
```

### PDF 领域知识

- **qpdf `--pages` merge 不保留源 PDF 的书签**（Outlines）。合并后的 PDF 是干净的，只有后续 `apply_bookmarks` 写入的书签。所以"删除已有书签"功能只在**重新处理**场景有意义（对已有书签的合并版重新跑处理）。

## 代码实施委托规则（必须遵守）

**Hermes（我）只负责需求文档和变更管理，代码实施全部委托给 Claude Code。**

### 职责分工

| 角色 | 职责 |
|------|------|
| **Hermes** | ① 定位问题 ② 影响分析 ③ 写变更单 ④ 验证结果 ⑤ 提交 |
| **Claude Code** | ④ 读代码 + 写代码 + 运行测试 |

### 流程

1. **Hermes 写变更单**：变更单必须是实施级（不是摘要），包含：
   - 每个修改点的精确文件路径 + 行号 + 函数名
   - 修改前代码（完整贴出）
   - 修改后代码（完整贴出，或清晰的伪代码描述）
   - 影响分析表（所有调用点 + 兼容性判断）
   - 测试计划
   - 回退方案

2. **写入临时任务文件**：变更单可能很长，直接传入 shell 会截断。写入 `/tmp/mdgXXX-tasks.md`：
   ```python
   write_file(path="/tmp/mdg016-remaining-tasks.md", content=task_description)
   ```
   任务文件应精简：只包含 Claude Code 需要执行的具体操作（文件路径、修改前后代码、验证命令），不包含背景故事和分析过程。

3. **Claude Code 实施**：
   ```
   claude -p "$(cat /tmp/mdgXXX-tasks.md)" --max-turns 20 --dangerously-skip-permissions
   ```
   - `--max-turns 20` 足够大多数任务。大任务拆成多个小任务文件分别 dispatch
   - Claude Code 达到 max-turns 时会退出，检查 `git log` + `git diff` 看完成了多少，剩余的重新 dispatch
   - 用 `background=true, notify_on_complete=true` 异步执行，不阻塞 Hermes

4. **Hermes 验证**：
   - `git diff --stat HEAD` 检查改了哪些文件
   - `git log --oneline -5` 检查是否提交
   - `cargo test 2>&1 | grep "test result"` 确认测试通过
   - **检查 Claude Code 是否用了 `#[allow(dead_code)]`** — Claude Code 倾向于用 suppress 而不是真正修复。用 `grep -n "#[allow(dead_code)]" src/` 扫描，发现后按 pitfall #59 处理
   - 更新变更单状态

### 为什么这样分工

- **Hermes 用 patch 工具改代码容易出错**（缩进错误、函数嵌套、模糊匹配替换错误目标）
- **Claude Code 读完整文件上下文后改代码更准确**
- **Hermes 擅长需求分析、影响分析、变更管理**
- **Claude Code 擅长读代码、写代码、运行测试**

### 变更单质量要求

变更单是 Claude Code 的唯一输入。如果变更单不够详细，Claude Code 会做出错误的修改。

**必须包含**：
- 精确的文件路径和行号
- 修改前的完整代码片段（不是摘要）
- 修改后的完整代码片段（或非常具体的描述）
- 所有需要修改的位置（不能只说"修改 xxx 函数"，要列出所有调用点）

**不要包含**：
- 模糊的描述（"优化一下"、"改好"）
- 未验证的假设（"可能需要改 xxx"）
- 多余的背景故事

## 实施模式（已验证）

### 分阶段提交

大变更（如架构重构）拆成 A/B/C/D 四个 Phase，每 Phase 独立 commit：
- **Phase A**：基础设施（新增文件/依赖，不改已有代码）→ cargo test 验证无回归
- **Phase B**：改造命令层（修改已有命令/前端）→ cargo test + npm test
- **Phase C**：迁移业务命令（逐个把 run_blocking 改为 run_managed）→ 含一个示范迁移
- **Phase D**：清理 + 文档更新

好处：任何 Phase 失败都可以回退到上一个成功的 commit，不影响已验证的基础设施。

### _cancellable 包装模式

修改底层函数签名影响面大。正确做法：**新建 `_cancellable` 包装函数**，保留原函数不动：

```rust
// 保留原函数（向后兼容）
pub fn batch_overlay(args: &serde_json::Value) -> Result<serde_json::Value> { ... }

// 新建包装（加取消支持）
pub fn batch_overlay_cancellable(
    args: &serde_json::Value,
    token: &CancellationToken,
) -> Result<serde_json::Value> {
    for item in items {
        if token.is_cancelled() { return partial_result(); }  // 返回部分结果
        // ... 调用原函数的逻辑
    }
}
```

### 取消时返回部分结果

**不要丢弃已完成的工作。** 取消时返回 `{ cancelled: true, processed: N, total: M, results: [...partial] }`。前端可以告诉用户"已处理 5/10 个文件"。

## 关键设计原则（Jade 明确要求）

### 超时 ≠ 无响应
处理大文件超时很正常，超时就 kill 是错误的。两者必须区分：
- **超时**：还在工作，只是慢 → 让它继续跑，用户可以主动取消
- **无响应**：卡死了，没有进展 → 检测长时间无输出后提示用户（不自动 kill）
- 固定超时自动 kill 只用于下载等网络操作，不用于文件处理

### 通用层优于点修复
架构级问题应该建立通用层，而非逐命令修补。例如 OperationManager 作为第四通用层（与 tauriBridge/moduleRegistry/run_blocking 同级），新命令只需用 `run_managed` 替代 `run_blocking` 就自动获得取消能力。

### 不设固定超时自动 kill
外部子进程的取消通过 CancellationToken 在轮询循环中检查，用户主动取消才 kill。保留 `command_output_with_timeout` 仅用于 managed tool 下载等确实需要的场景。

## 跨 MDG 接口审查

**当一个 MDG 依赖另一个 MDG 的接口时，必须先全面审查已建接口是否够用。**

流程：
1. 列出新 MDG 需要从已有 MDG 获取的所有数据/事件/方法
2. 逐项检查已有 MDG 是否暴露了这些接口
3. 发现缺口 → 在已有 MDG 的分支上补充接口，确认测试通过后再开始新 MDG
4. 接口补充应作为独立 commit（如 "MDG-001 接口补充：为 MDG-011 扩展..."）

**案例**：MDG-011（诊断系统）需要操作耗时和结束原因，但 MDG-001 的 OperationManager 只暴露了 `list_active() -> Vec<String>`。审查后补充了 `ActiveOperation`（带 elapsed_ms + command）和 `OperationFinishedEvent`（带 outcome + elapsed_ms），然后才开始 MDG-011。

## 模块诊断 → 全局诊断模式

**局部的、模块内嵌的诊断应该被全局的、深入的诊断系统替代。**

错误模式：每个模块自己写诊断 dump（如 TemplateView.vue 内嵌 60 行 diagnostic JSON dump + 写文件逻辑）。

正确模式：
- 全局 `Diagnostics` 服务提供 Logger / Metrics / StateProvider / CrashReporter
- 各模块只调用 `diagnostics.log.info(module, msg, ctx)` 和 `diagnostics.snapshot(module, data)`
- 操作追踪通过订阅 OperationManager 事件自动完成，无需模块自己管
- 导出报告统一收集所有模块快照 + 日志 + 系统信息

## 全面递归审查（Jade 明确要求）

**每次 MDG 变更提交前，必须对全部已修改文件做递归审查。** 不只是审查最新 commit，而是审查整个分支的 diff（`git diff main..branch`）。

审查清单：
1. **错误路径泄漏（Rust `?` 操作符）** — `?` 提前返回时，后续的 `finish()` / `drop()` / cleanup 是否被跳过？用 `match` 替代 `?` 或 RAII guard。这是 MDG-001 发现的 critical bug
2. **死代码** — 被替代的枚举/字段/函数是否还有调用者？用 `grep -rn` 确认。特别注意 struct 字段级别的死代码（定义+初始化存在但从未读取）
3. **返回类型一致性** — 改了 `list_active()` 的返回类型，所有调用方（前端+Rust）是否都适配了？
4. **事件结构一致性** — 发射的事件 payload 变了，所有监听方是否兼容？
5. **接口完整性** — 下游 MDG 需要的数据，当前接口是否都暴露了？
6. **全链路数据流一致性（功能级审查）** — 从 UI → composable → IPC → Rust 命令 → 处理逻辑 → 输出，每个环节的数据是否真的传递了？特别检查"UI 有 checkbox 但 composable 硬编码忽略"的断裂
7. **用户设置是否真的生效** — grep 所有 composable/service 中的硬编码值（如 `RemoveExisting: false`），对比前端 rules 是否有对应设置

**案例**：MDG-001 审查发现 `run_managed` 的 `?` 操作符在 spawn_blocking panic 时跳过 `finish()`，导致操作永远留在 map 里。修复：用 `match` 替代 `?`，确保 `finish()` 永远被调用。MDG-004 审查发现变更单中 4 个问题只有 1 个仍存在，其余已修或需推迟。

## 诊断系统实施模式（MDG-011 验证）

**增强现有模块优于新建模块。** MDG-011 设计文档建议新建 `diagnostics.rs`，但实际上 `app_log.rs` 已有 NDJSON + 日期轮转 + panic hook。正确做法是增强现有模块：

1. **Phase A — 日志增强**：给 `app_log.rs` 加 `debug`/`warn` 级别 + `operation_id` 可选字段。不需要新模块
2. **Phase B — 操作追踪**：在 `OperationManager.begin()`/`finish()` 中加 `app_log::info_with_op()` 调用，自动记录操作生命周期
3. **Phase C — 状态快照**：前端 `diagnostics.js` 提供 `registerSnapshotProvider(name, fn)` + `exportReport()`。各模块在 `onMounted` 注册快照
4. **Phase D — 崩溃报告**：增强 panic hook 收集活跃操作 + 最近日志。需要 OperationManager 全局注入，改动较大时可推迟

**前端诊断模块结构**：
```javascript
// shared/diagnostics.js
export const log = { debug, info, warn, error }  // 写后端日志文件
export function registerSnapshotProvider(name, fn)  // 注册模块快照
export function exportReport()  // 调 Rust export_diagnostic_report
```

**Rust 端新增命令**：`export_diagnostic_report(frontend_snapshots)` — 收集系统信息 + 最近日志 + 前端快照，写入 `$APPCONFIG/logs/diagnostic-YYYYMMDD-HHMMSS.json`

## 批量完成 MDG 条目的工作模式

当用户要求"把现有的 MDG 条目全部完成"时：

1. **先读 README + 变更单**，列出所有未完成项
2. **按复杂度排序**：简单修复 → 中等改动 → 大型重构
3. **每项完成后立即**：test → commit → 更新 README 状态 → 更新变更单 Phase 标记
4. **标记"部分完成"是可接受的**：当某个 Phase 需要较大 UI 重构或全局注入时，先完成可独立提交的部分，标记剩余为待做
5. **不要问"继续下一个？"** — 有剩余就做下一个

## `git revert` 双向恢复模式

`git revert HEAD --no-edit` 是幂等的第二次操作：第一次撤销 commit，第二次重新应用。用于：
- 用户说"别改，恢复" → `git revert HEAD --no-edit`
- 用户说"你看看能不能用" → 再次 `git revert HEAD --no-edit` 重新应用

## Pitfalls

1. **不要用 `write_file` 重写整个文件** — 用 `patch` 精确修改
2. **不要跳过影响分析** — 每个修改必须先搜索全部调用点
3. **不要在主分支直接改** — 必须建 mdg/ 分支
4. **不要一次改多个 bug** — 每个 MDG-XXX 独立一个分支和变更单
5. **Rust 代码修改后必须 `cargo test`** — 编译通过不等于逻辑正确
6. **前端代码修改后必须 `npm run test`** — 有测试的跑测试，没测试的手动验证
7. **变更单必须先写再改** — 不允许直接改代码后补记录
8. **不要只报告 metrics** — Jade 需要架构级判断，不是文件数和行数统计
9. **不要盲目建议"拆分大组件"** — 先检查子组件和 composables 是否已经合理拆分
10. **不要一次性大 prompt 审阅整个项目** — context 爆了产出会很肤浅，分模块逐文件审阅
11. **不要混淆超时和无响应** — 大文件慢是正常的，不自动 kill
12. **架构变更先读设计文档** — 理解原始设计意图再提方案，搜索框架官方文档
13. **Tauri 命令参数不能随意注入额外字段** — serde 默认拒绝未知字段。前端的 operationId 用于 UI 动画追踪，Rust 的 OperationManager 生成自己的 ID，两套 ID 独立运作。取消时前端通过 `list_active_operations` 查询 Rust 侧活跃操作
14. **Tauri `.manage()` 消耗 Arc 所有权** — 如果 `.setup()` 闭包也需要访问同一 Arc，必须先 clone：`let x_for_setup = x.clone();` 然后 `.setup(move |app| { x_for_setup.xxx() })`
15. **Rust `app.emit()` 需要 `use tauri::Emitter`** — 不导入这个 trait，`.emit()` 方法找不到，编译错误不会提示缺少 import
16. **UI/UX 不是"后面再做"** — 架构变更时必须预留 UI 接驳点。用户会问"这里需要考虑 XXX 动画吗"——如果答案是"应该但推迟"，至少建好事件接口（如 `docsy-operation-started/finished`），不要等架构定型后再硬塞
17. **接口只为当前消费者设计会返工** — 设计接口时问"下游 MDG 会需要什么数据"。MDG-001 只为前端动画设计了 `list_active() -> Vec<String>`，MDG-011 诊断系统需要 elapsed_ms 和 command，被迫返工补充 `ActiveOperation` 和 `OperationFinishedEvent`。宁可接口丰富一点，也不要后面回来改
18. **Rust `?` 操作符跳过清理代码** — 如果 `finish()` / `drop()` / cleanup 在 `?` 之后，错误路径会跳过它们，导致资源泄漏。正确做法：用 `match` 替代 `?`，或使用 RAII guard（impl Drop）。案例：`run_managed` 中 `spawn_blocking().await.map_err()?` 在 JoinError 时跳过 `finish()`，操作永远留在 OperationManager map 里。审查时专门检查"错误路径是否跳过清理"
19. **合并前做跨 MDG 接口扫描** — 不仅在"开始新 MDG"时审查依赖接口，合并前也要扫描其他变更单是否引用了本次变更的接口。流程：`grep -l "关键词" docs/mdg/changes/MDG-00[2-9]*.md docs/mdg/changes/MDG-01[0-9]*.md` 检查所有未合并的变更单。MDG-001 合并前确认 MDG-011 的接口需求已满足，其余变更单不涉及
20. **变更单行号可能过时** — 变更单中的行号是写单时的快照，实际修的时候代码可能已变（中间有其他 MDG 合并了）。先 grep -n 定位当前行号，不要盲信变更单。MDG-004 变更单写 header_footer.rs:324，实际代码已移到 :381
21. **变更单中的问题可能已修过** — 执行前必须验证当前代码状态。变更单中的 4 个问题，实际审查发现 2 个已修、1 个推迟、只剩 1 个需要修。不要假设变更单中列出的问题仍然存在
22. **MDG 之间不要问"继续下一个？"** — 有剩余任务就做下一个，没有就声明完成并停止。Jade 反感无意义的确认提问
23. **全库扫描替代变更单调用点** — 变更单的影响分析表只列出了写单时找到的调用点。实际修复时必须做全库扫描（grep -rn），可能发现更多调用点，也可能发现原来的问题已经不存在了
24. **不要问"继续下一个？"** — 有剩余任务就做下一个，没有就声明完成并停止。MDG 之间更不要问，直接做。Jade 的 "？" 回复就是对这类无意义确认的批评
25. **patch 工具的模糊匹配会替换错误目标** — 用 `patch` 删一个 Vue/JS 函数时，如果 old_string 包含的"下一个函数名"与实际代码不匹配，patch 的模糊匹配可能把另一个同名函数替换掉。MDG-003 前端去重时 patch 把 `openSplitDialog` 替换成了 `allFieldSuggestionItems`。**防御：patch 后立即用 `sed -n 'Xp'` 验证目标行附近内容是否正确**
26. **不要用 `git add -A`** — 子任务可能引入无关文件（如 docs/*.md），`git add -A` 会把它们全部混入代码 commit。始终用 `git add <具体文件>` 指定
27. **子任务结果必须逐文件验证** — 委派机械性任务（如批量去重）后，必须逐文件检查子任务是否完整完成。MDG-003 子任务处理了部分文件但遗漏了其他文件，且 evidence.rs 加了 import 但没删本地定义。用 `grep -rn "fn xxx"` 扫描确认零残留
28. **不要凭用户口头判断下结论** — 用户说"偏移是对的吧"，不能直接接受，必须自己用代码验证。追踪 `.nth(page_index)` 的 0-based 索引 + `page_offset` 累加逻辑，确认后再给出结论。验证要具体（".nth(page_index) 是 0-based，累加逻辑 X+Y=Z 正确"），不能笼统（"没问题"）
29. **启发式检测优于强制新类型** — 当发现新的 Word 特性（如 FORMCHECKBOX 域）时，不要急于引入新的字段类型。先看能否自动检测并匹配到已有类型（如 checkbox），保留原有结构，用户可在 UI 中覆盖。Jade 原话："如果选中的文件中有，就自动匹配上...可以不做强制性的，但可以检测并匹配"。设计原则：自动推断 + 用户可覆盖，而非强制分类
31. **Word 标黄可能包含无意义空白** — 用户在 Word 里标黄时经常不小心多选了空格/标点/换行。mark 生成端（Rust scan）必须过滤 `text.trim().is_empty()` 的 mark，不能把过滤责任推给前端。详见 `references/template-module-pitfalls.md`
30. **Tauri 桌面应用没有浏览器 console** — `console.log` 输出看不到。诊断信息必须写文件（用 `@tauri-apps/plugin-fs` 的 `writeTextFile`，不是 `invoke('write_text_file')`）。需要在 capabilities/default.json 加 `fs:allow-write-text-file` 权限
31. **Word FORMCHECKBOX 域结构被 SDT 破坏** — Word 勾选框由 7 个 `w:r` 组成（fldChar begin → instrText → fldChar end），扫描器正确只索引含 `w:t` 的 run，但保存器可能把所有 run 都包裹 SDT，导致渲染时文本重复 7 次。调试：解压 .docsytpl 看 document.xml 中 SDT 包裹了哪些 run。详见 `references/template-module-pitfalls.md` 的 MDG-012 章节
32. **模板问题调试 = 解压对比 XML** — docx 和 .docsytpl 都是 zip。用 Python zipfile 解压后对比 `word/document.xml`，关注 `<w:r>` 级结构差异（不只是文本内容），检查 SDT 包裹是否正确
33. **分析文件格式 bug 必须先理解格式规范** — 用户批评"你没研究啊，假装研究呢？"。分析 docx 相关 bug 时，不能只看代码层面的 `w:r` 处理，必须先理解 Word 的域（Field）机制（fldChar/instrText/separate/end 结构、域类型、嵌套规则）。同样适用于其他结构化格式（PDF object tree、OOXML 核心部件等）。**先理解格式规范，再分析代码实现**。扫描现有文件 ≠ 研究格式规范
34. **不要过度工程化预览** — Jade 说"纯文本预览倒是没有问题，纯文本预览的好处就是会比较快"。当用户指出某个功能"丑又不好用"时，先确认具体哪些部分需要改，不要全盘重构。纯文本 `<pre>` 预览在速度上有优势，不必急于改成 HTML 渲染。**先问清楚用户想改什么，再动**
35. **一个按钮自动推断优于多个类型按钮** — 用户说选区添加功能不需要 8 个类型按钮（文本/日期/列表/引用/勾选/前缀/后缀/保留原文），改为一个"添加为字段"按钮，默认文本，自动推断 checkbox/date，用户可在表格中修改。复用已有的 `inferTemplateField` 逻辑
36. **预览模块应该做成通用组件** — 用户设想三层预览（原文/字段标注/填写），共用同一个 `DocumentPreview` 组件，接受 `runs[]` + `overlays[]`。其他模块（证据 PDF、文书对比）可复用。不要在每个模块内各写一套预览逻辑。详见 `references/filename-token-input.md`
37. **Token 输入组件必须允许自由文本** — FilenameTokenInput 这类组件，输入框必须用 `v-model` 双向绑定允许用户输入任意文本。token 解析只在 blur/Enter 时执行。**绝对不能让 token 同步覆盖用户正在输入的文本**——用 `inputFocused` flag 阻止 watcher 在输入时同步
38. **描述文字做成 tooltip，不占 UI 空间** — Jade 明确要求"各个区域的提示词可以做成悬浮的气泡显示，不要占据UI空间"。面板标题旁加 `<el-tooltip>` + `<el-icon class="help-icon">`（QuestionFilled），不加 `<p>` 描述标签
39. **横向紧凑布局优先** — 用户多次纠正"做成两排了，我觉得一排就够了"。工具栏、token 条、输入框等控件应横向排列（`display: flex; flex-wrap: nowrap`），不纵向堆叠。空间不够时用 `overflow-x: auto` 横向滚动
40. **长下拉列表必须限制高度** — Element Plus dropdown 超过 10 项时加 `max-height: 280px` + `overflow-y: auto`。用 `:deep(.menu-class)` 样式穿透
41. **外部操作前先 commit 输入状态** — 当输入框和按钮共存时，点击按钮（如添加预设/字段）必须先 commit 输入框当前文本，再执行按钮动作。否则用户输入的内容丢失
42. **UI 改动必须严格按六步协议** — Jade 批评\"最近的修改你好像没有按照mdg的标准来，才导致我改了这么多次还没改好\"。UI 组件改动也是代码改动，必须：①定位精确代码 ②分析影响范围 ③记录变更单 ④一次实现到位 ⑤验证 ⑥提交。不要反复迭代原型——每次迭代都是浪费用户时间。先确认设计方案再动手，不要边做边改
43. **按钮可拆分：主区域+下三角** — 当一个按钮有默认操作和高级选项时，用拆分布局：点击主区域=默认操作，点击右侧小三角 ▾=弹出选项 popover。下三角宽度约为图标一半，整个方框保持和谐。适用于序号格式、日期格式等场景
44. **CSS 变量名必须先验证存在** — Jade 要求用的 CSS 变量（如 `--docsy-surface-base`）可能不存在。用之前先 `grep -r "变量名" src/styles.css` 确认。Docsy 实际变量：`--docsy-surface`（#fcf5ea）、`--docsy-surface-elevated`（#fff9f0）、`--docsy-surface-muted`（#f3eadf）。不存在的变量解析为空值→背景透明→UI 看不见。MDG-013 中 `.fn-popover` 用了 `--docsy-surface-base`（不存在）导致 popover 菜单透明，修了 3 次才发现
45. **Popover 定位需要 position:relative 父容器** — `position: absolute` 的 popover 相对于最近的 `position: relative` 祖先定位。如果按钮在 `display: flex` 容器中没有 `position: relative`，popover 会相对于页面定位，出现在错误位置。MDG-013 中日期/序号菜单弹出在整行最右边而非按钮下方。修复：每个 split 按钮用 `.fn-btn-group`（`position: relative; display: inline-flex`）包裹，popover 定位 `left: 0`（相对于按钮组）
46. **颜色体系统一** — 当 UI 有预览区和操作区（按钮）时，同类元素必须颜色一致。MDG-013：模板名=绿(#d1fae5/#065f46)、日期=琥珀(#fef3c7/#92400e)、序号=紫(#ede9fe/#5b21b6)、字段=蓝(primary-soft)、literal=灰(muted)。预览色块和右侧按钮用同一套 class，用户一眼看出对应关系
47. **el-dropdown 菜单项防横向溢出** — Element Plus dropdown 菜单项文字可能很长（如字段名"律师事务所名称"）。CSS：`:deep(.menu-class) { overflow-x: hidden; max-width: 200px; }` + `:deep(.el-dropdown-menu__item) { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }`。MDG-013 字段列表反复出现横向拖拽条，根因是没加 `overflow-x: hidden`
48. **Tauri v2 fs 权限需要 scope 配置** — `fs:allow-write-text-file` 和 `fs:allow-read-file` 在 Tauri v2 中默认不允许写任意路径。必须在 `capabilities/default.json` 中配置 scope：`{ "identifier": "fs:allow-write-text-file", "allow": [{ "path": "$APPCONFIG/**" }, { "path": "$APPLOCALDATA/**" }, ...] }`。缺少 scope 会导致 "forbidden path" 错误。常用 scope 变量：`$APPCONFIG`（应用配置目录）、`$APPLOCALDATA`（本地数据）、`$DOCUMENT`、`$DOWNLOAD`、`$DESKTOP`、`$TEMP`。详见 `references/tauri-capabilities-scope.md`
49. **Rust serde struct 加 optional 字段要全量扫描构造点** — 给 serde struct 加 `#[serde(default)] pub new_field: Option<T>` 后，**所有显式构造该 struct 的地方**都必须加 `new_field: None`（或对应值）。否则编译报 `missing field`。搜索方法：`grep -rn "StructName {" src/ --include='*.rs'`。MDG-013 加 `filename_template` 时漏了 5 个构造点（engine.rs 测试×2 + template_history.rs 测试×2 + render.rs 测试×1）
50. **不要在用户确认前做 UI 改动** — 用户描述了一个功能（如"关系列加按钮"），不能直接实现并提交。先确认方案再动手。MDG-003 时我直接加了按钮，用户说"别改，恢复"，然后说"好像是被 revert 了你看看能不能用"——说明用户自己在 Claude Code 里做过类似改动，需要先检查现有状态。**流程：描述方案 → 用户确认 → 实现 → 验证 → 提交**
51. **开始前检查其他会话的改动** — 用户可能在 Claude Code 或其他工具中做了改动。开始新任务前先 `git log --oneline -10` + `git status --short` 检查当前状态，可能发现已有实现、未提交的改动、或需要衔接的工作。MDG-013 用户在 Claude Code 中做了 15 个 commit，我开始时没有检查
52. **Rust struct 加 optional 字段后编译报错的修复模式** — `cargo test` 报 `missing field 'xxx' in initializer` 时，用 `grep -rn "StructName {" src/ --include='*.rs'` 找所有构造点，逐个加 `xxx: None`。常见遗漏：测试文件中的构造（如 `engine.rs:518`、`engine.rs:704`、`template_history.rs:879`、`template_history.rs:928`、`render.rs:1049`）。**一次修完所有构造点再编译**，不要修一个编译一次
53. **`log::debug!` 不需要 `use log`** — Rust 中 `log` crate 的宏可以直接用 `log::debug!("...")` 而不需要 `use log;`，因为宏是按路径解析的。但 `eprintln!` 在 test 代码中可保留（开发者可能想看输出）
54. **`match` 空分支用于验证** — 当 match 只用于验证（如检查文件扩展名是否合法）而不需要返回值时，用空分支 `=> {}` 替代返回字符串的分支。简化 `let mime = match ext { "jpg" => "image/jpeg", _ => bail!() }` 为 `match ext { "jpg" | "png" | ... => {} _ => bail!() }`，同时删除未使用的 `mime` 变量和 `let _ = mime;` 压制
55. **中文序号转换函数** — 批量文件名需要中文序号（一、二、三...）时，用 `to_chinese_number(n: usize)` 实现。注意：`一十` 应简化为 `十`，`>=10000` fallback 到阿拉伯数字。实现参考 `batch.rs` 中的 `to_chinese_number` 函数
56. **README 标题与变更单不同步** — README 中的 MDG 名称可能与变更单文件名/内容不匹配。MDG-005 README 写"书签写入原子操作"，但变更单 `MDG-005-prod-cleanup.md` 内容是"生产代码清理"。**开始任何 MDG 前，先验证 README 标题与变更单内容一致**。用 `head -1 docs/mdg/changes/MDG-XXX-*.md` 对比 README 对应行
57. **"部分完成"可能掩盖可完成项** — README 标记"✅ 部分完成"的条目，剩余项可能只需几分钟就能完成。MDG-010 标记"部分完成"但只剩一个 `unwrap_or(false)` → `unwrap_or(true)` 的默认值对齐。MDG-011 标记"基础完成"但 Phase D 的活跃操作收集只是 nice-to-have。**遇到"部分完成"时，先检查剩余项的实际工作量，不要直接跳过**
58. **完成全部 MDG 后做跨模块审查** — 所有 MDG 条目标记完成后，做一次系统性审查：(1) 每个新增/修改的 pub fn 用 `grep -rn` 找全部调用方 (2) 每个新增的前端组件/props 用 `grep -rn` 找全部使用方 (3) 检查删除的文件是否有残留引用 (4) 检查默认值变更是否前后端一致 (5) 最终 `cargo test` + `npm test` 确认无回归。审查应覆盖：Rust struct 字段变更的所有构造点、前端 props 传递链（定义→传递→使用）、共享模块的所有消费者
59. **禁止 `#[allow(dead_code)]` suppress warnings** — Jade 明确拒绝这种做法："这种保留的你加上 Allow dead code，就相当于你还是不用它，但是不让它警告了，这有什么意义呢？" 处理 unused warning 的三种正确做法：(1) **找到使用场景并接入**（如 `debug` 在 batch.rs generate_filename 中接入）、(2) **删除函数**（如 `cancel_all`/`is_cancelled` 无生产调用方就删除）、(3) **标注 `#[cfg(test)]`**（仅当函数确实是测试辅助，不是 suppress，是声明用途）。判断顺序：先问"这个函数应该在哪用？"→ 有场景就接入 → 无场景就删除 → 仅测试用才 `#[cfg(test)]`
60. **构建 API 必须立即接入调用方** — Jade 批评 MDG-011 新增了 `warn`/`debug`/`error_with_op` 但无人调用："Debug不是在MDG里有相关的内容吗？为什么还会让它悬置？那现在不做啥时候做？" 正确做法：新增公共 API 时，必须在同一 commit 中至少接入一个生产调用方。`debug` → batch.rs generate_filename 入口；`warn` → operations.rs cancelled 路径；`error_with_op` → operations.rs failed 路径。不允许"先建 API 后接入"
62. **分析结论和行动必须一致** — Jade 批评"你说了很多次有必要存在，但又不用，你觉得合理吗"。如果你分析一个函数为"有用的公共 API"或"OperationManager 的完整 API"，就必须找到接入点并接入，不能分析完说"有必要"然后又删掉。正确流程：分析 → 找接入点 → 接入 → 验证。如果找不到接入点，说明分析有误——重新评估是否真的有必要
63. **死代码审查必须检查重复实现** — `apply_bookmark`（单数）和 `apply_bookmarks`（复数）是两套独立实现，`apply_bookmarks` 没有调用 `apply_bookmark`。这是早期写的单书签版本，后来写了批量版本但没有复用。发现 dead function 时，先检查是否有另一个函数做了同样的事——如果有，删除重复的那个，测试改用统一版本。搜索方法：`grep -n "fn xxx\|fn xxxs" src/` 看是否有单复数版本
64. **Tauri `on_window_event` 是 Builder 方法** — `tauri::Builder::default().on_window_event(|window, event| { ... })` 链式调用，不是 `app.on_window_event()`。签名：`Fn(&Window<R>, &WindowEvent) + Send + Sync + 'static`。用于窗口关闭时清理资源（如 `cancel_all`）。注意：`.on_window_event()` 在 `.manage()` 之后、`.setup()` 之前调用
65. **先查询状态再移除条目** — `finish()` 中检查 `is_cancelled` 再 `map.remove()`，而不是在同一个 lock 内做两件事。好处：复用 `is_cancelled` 方法（消除 warning），逻辑更清晰。注意两次 lock 之间的时间窗口——在此场景中安全，因为 `cancel` 只设置 token flag（已检查过），`finish` 移除条目（下一步才做）

61. **设计文档必须是实施级，不是摘要** — Jade 批评"你的这个总结写得太简单了，如果只依据这个处理的话后续处理的时候可能会出现误解"。设计文档不是给用户看的总结，是给实施者看的详细方案。必须包含：(1) 每个 Bug 的完整调用链追踪（从 UI 到后端，每一步都写清楚）(2) 每个需求的代码级方案（具体改哪个函数、改什么参数）(3) 数据结构变更（Rust struct + 前端 props 的完整字段列表）(4) 竞品调研的具体功能点（不是"Acrobat 有奇偶页"，而是"Acrobat 通过页码范围 '1,3,5,7...' 实现奇偶页"）。变更单是实施的蓝图，不是会议纪要

62. **Bug 调查必须追踪完整技术链路，不能中途下结论** — Jade 批评"我不知道你说的这个不能删除的结论是怎么做出来的"。当发现"检测成功但操作失败"时，不能只看表面就说"技术路径不一致"。必须追踪完整链路：(1) 检测用了什么技术 (2) 操作用了什么技术 (3) 两者是否有共享的能力（如 bbox 匹配）(4) 失败的具体原因是什么。案例：页眉页脚删除失败。我最初说"检测用 pdftotext，删除用内容流解析，路径不一致"。但深入追踪发现：plain text 删除路径**已经有** `target_bbox_matches` 基于 bbox 坐标匹配内容流中文本的能力。实际失败原因可能是 bbox 未传递、坐标转换误差、或 zone 设置问题——不是"路径不一致"。详见 `references/evidence-header-footer-module.md`

63. **Warning 清零的系统性方法** — `cargo test 2>&1 | grep "^warning" | sort | uniq -c | sort -rn` 收集所有 warning，逐个分析：(1) `grep -rn` 找全部引用判断是真死代码还是漏接入 (2) 区分 unused import（删除）、never-called function（接入或删除）、test-only function（`#[cfg(test)]`） (3) 每修一个立即编译验证 (4) 最终确认 0 warnings。不要批量 suppress
64. **Jade 不能自己读文件，必须在聊天中详细总结** — Jade 明确说"你还是把文档尽可能详细地总结给我，因为我不能自己去看"。写完设计文档/变更单后，必须在聊天中给出完整详细的内容摘要（不是一句话概述），包含：每个 Bug 的根因调用链、每个需求的具体方案、竞品调研结论。变更单是实施蓝图，聊天中的摘要是 Jade 的决策依据。两者都要详细
65. **重大模块用多 AI 交叉审计** — MDG-016 用 Claude + GLM 5.2 两个模型独立审计，再交叉验证。结果：GLM 发现了 Claude 遗漏的 CID 字体编码根因，Claude 纠正了 GLM 的 dingbat 格式错误。两个模型各自有盲区，交叉验证能显著提高准确性。详见 `cross-audit-code-review` skill
66. **审计简报不要预消化代码** — 给审计者的简报应包含：项目背景、问题现象（不含分析）、代码文件清单（绝对路径+行数+职责）、具体问题清单。**不要**包含代码片段或你的根因分析——审计者应从代码中独立得出结论。MDG-016 第一版简报包含代码片段，GLM 的结论被引导；第二版去掉代码片段后，GLM 得出了更独立的分析
67. **竞品调研要落实到具体方案** — 调研 Acrobat/WPS/Foxit 后，不能只说"它们有奇偶页支持"。要写清楚：Acrobat 通过页码范围 "1,3,5,7..." 实现、WPS 用复选框、Foxit 用 Bates 编号。然后结合自身设计给出具体 UI 方案（卡片化/Tab 分层/专用设置区）。调研的价值在于可执行的方案，不是功能清单
68. **不要用 patch 工具改复杂代码** — Hermes 的 patch 工具容易出错（缩进错误、函数嵌套、模糊匹配替换错误目标）。MDG-016 实施时 patch 把 `read_form_plan` 函数嵌套进了 `object_number` 的 match arm 里。正确做法：Hermes 写完整变更单，Claude Code 负责改代码。**例外：1-3 行的简单改动（如 `||` → `??`、import 添加、参数重命名）可以直接用 patch**，见 pitfall #88。详见「代码实施委托规则」
69. **Hermes 只写需求文档，不写代码** — Jade 明确要求"写代码的任务都放进 Claude 去，你只写完整的需求文档"。Hermes 的价值在于需求分析、影响分析、变更管理，不在于写代码。用 `claude -p` print mode 传入变更单，Claude Code 读代码+写代码+跑测试，Hermes 验证结果+提交
70. **Claude Code 倾向于 suppress warnings 而非修复** — Claude Code 处理 unused warning 时经常加 `#[allow(dead_code)]` 而不是真正修复（删除/接入/`#[cfg(test)]`）。MDG-016 中 Claude Code 给 `DeleteDiagnostic`、`DeleteSkipReason`、`matches_any_target` 都加了 `#[allow(dead_code)]`。**验证步骤必须包含** `grep -n "#[allow(dead_code)]" src/` 扫描，发现后按 pitfall #59 处理。同理，JS/TS 中 Claude Code 可能用 `// @ts-ignore` 或 `eslint-disable` suppress 类型错误
71. **Claude Code max-turns 不够时拆分任务** — Claude Code `--max-turns 30` 对大任务不够（MDG-016 的 4 个 Rust 任务 + 3 个 JS 任务 = 30 轮不够）。解决：(1) 先 dispatch JS 任务（较快），commit 后再 dispatch Rust 任务 (2) 每次只给 2-3 个任务 (3) 用 `--max-turns 20` 而非 30，留余量给意外 (4) Claude Code 退出后检查 `git log` + `git diff` 看完成了多少，剩余的重新 dispatch
72. **Claude Code 几乎每次都会留下编译错误** — MDG-017 中每次 Claude Code dispatch 后 `cargo test` 都编译失败，需要手动修复。常见模式：(1) 改了函数签名但没更新所有调用点（特别是测试中的）(2) `CancellationToken` 等类型用了错误的 import 路径（`crate::operations::CancellationToken` 是 private 的，应该用 `tokio_util::sync::CancellationToken`）(3) 闭包借用外部变量需要 `move` 但 Claude Code 漏加 (4) 参数引用/值类型不匹配（`token` vs `&token`）(5) 统一后的函数回退值与原测试期望不一致（如 `safe_file_stem("")` 从 `"split"` 变为 `"output"`）。**验证流程必须包含编译+测试**，不能只看 `git diff`
73. **大 MDG 批量 dispatch 模式** — 当 MDG 有 20+ 个小任务时，不要一次性全给 Claude Code（context 会爆）。正确做法：(1) 按优先级和依赖分批，每批 3-4 个任务 (2) 每批写一个独立的 `/tmp/mdgXXX-batchN.md` 任务文件 (3) dispatch → 等完成 → 修复编译错误 → commit → 下一批 (4) 每批任务文件只包含具体操作（文件路径+修改前后代码+验证命令），不包含背景故事。MDG-017 用这个模式在 5 轮 dispatch 中完成了 15 个任务
74. **并行测试的临时文件竞争条件** — `temp_named_path` 用 PID+毫秒时间戳生成文件名，但并行测试在同一进程中 PID 相同，时间戳可能相同。修复：用 `output_path.with_extension("pdf.tmp")` 替代 `temp_named_path`，确保临时文件与目标文件在同一目录（同一文件系统，rename 可用）且文件名基于目标文件名（不会冲突）。**不要用 `std::env::temp_dir()` 创建临时文件再 rename 到目标目录**——可能跨文件系统导致 rename 失败
75. **用户等待时主动汇报进度** — 用户说"你还在工作吗？怎么感觉你卡住了"。长时间等待 Claude Code 时（>30 秒），应该主动发消息说明当前状态，而不是默默 polling。`process(action='wait')` 的 60 秒超时限制意味着需要多次 wait，在每次 wait 之间告诉用户进展
76. **Rust `safe_file_stem` 必须处理全角字符** — 统一路径函数时，原来的实现可能处理了全角括号（`（）`）等字符，新实现遗漏会导致测试失败。统一函数前先对比所有实现的字符处理列表，取并集
77. **合并重复函数时必须检查所有测试的期望值** — 不同文件中的同名函数可能有不同的默认值（如 `safe_file_stem` 空字符串回退值：evidence.rs 用 `"output"`，split.rs 用 `"split"`）。统一实现后，必须 `cargo test` 检查所有测试，逐个修复期望值不匹配的测试。不能只改实现不改测试
78. **path deduplication 搜索策略** — 合并重复函数时，用 `grep -rn "fn 函数名" src/` 找所有定义，再用 `grep -rn "函数名(" src/` 找所有调用。注意：(1) 调用点可能用 `super::函数名` 或 `crate::模块::函数名` (2) 测试中的调用也要更新 (3) 删除本地定义后要更新 import（`use super::函数名` 或 `use crate::模块::函数名`）(4) 函数签名可能在不同文件中略有不同（参数数量、类型），统一时要取最通用的版本
72. **Claude Code 原子写入实现的常见 bug** — Claude Code 实现原子写入时，用 `output_path.with_file_name(...)` 创建临时文件。但如果 `output_path` 的目录不存在（如测试中的临时路径），`std::fs::rename` 会报 "No such file or directory"。**正确做法**：临时文件必须和目标文件在同一目录（rename 的前提），且要确保目录存在。修复任务文件中应明确写："临时文件用 `output_path.parent()` 构建，确保目录存在"。MDG-016 测试因此失败
73. **GLM 审计 → MDG 设计的完整流程** — 当 GLM 做完整项目审计后，正确流程是：(1) Hermes 读完所有审计报告 (2) 委托 Claude 背靠背验证 GLM 的发现（`delegate_task` 读代码确认） (3) Hermes 基于验证结果写变更单 (4) 委托 Claude Code 实施。不要跳过第 2 步——GLM 的发现可能有误（如 MDG-016 中 dingbat 格式错误是假警报）
74. **后台任务管理策略** — 多个 Claude Code 任务用 `background=true, notify_on_complete=true` 异步执行。Hermes 在等待期间可以做其他工作（如写变更单、设计下一个 MDG）。Claude Code 完成后用 `git log` + `git diff` 检查结果。如果 Claude Code 达到 max-turns 退出，检查完成了多少，剩余的重新 dispatch
75. **原子写入临时文件的三个陷阱** — 实现原子写入（先写临时文件再 rename）时：(a) `output_path.with_file_name(temp_name)` 保留目标目录，如果目标目录不存在会失败 (b) `temp_named_path` 用 `std::env::temp_dir()`，可能与目标不在同一文件系统，`std::fs::rename` 不支持跨文件系统 (c) 并行测试中 `temp_named_path` 同 PID + 同毫秒时间戳会冲突。**正确做法**：`output_path.with_extension("pdf.tmp")` — 临时文件与目标同目录（同一文件系统），文件名基于目标文件名（不会冲突）。MDG-016 修了 3 次才修好
76. **dispatch Claude Code 后的验证清单** — Claude Code 完成后，必须执行以下验证：(1) `git log --oneline -3` 检查是否提交 (2) `git diff --stat HEAD` 检查改了哪些文件 (3) `cargo test 2>&1 | grep "test result"` 确认测试通过 (4) `grep -n "#[allow(dead_code)]" src/` 扫描 suppress (5) 如果有测试失败，看错误信息判断是 Claude Code 引入的 bug 还是预存问题 (6) 修复后重新 dispatch 时，任务文件要包含具体的错误信息和修复方向，不要只说"修好它"

79. **`run_managed` 闭包需要 `move` 关键字** — 当 `commands/video.rs` 等文件中的 `run_managed` 闭包引用了外部变量（如 `args`），必须用 `move |token| { ... }` 而不是 `|token| { ... }`。否则编译报 `closure may outlive the current function, but it borrows 'args'`。Claude Code 经常漏加 `move`，需要手动修复

80. **`CancellationToken` 的正确 import 路径** — `crate::operations::CancellationToken` 是 private 的，不能直接用。正确路径是 `tokio_util::sync::CancellationToken`。Claude Code 经常用错误的 import 路径，编译报 `struct import 'CancellationToken' is private`

81. **`run_managed` 闭包中 `token` 是值不是引用** — `run_managed` 的闭包签名是 `FnOnce(CancellationToken) -> ...`，所以 `token` 是所有权转移。在调用 `extract(&args, &token)` 时需要传引用 `&token`，不能直接传 `token`

82. **大 MDG 的批量 dispatch 策略** — 当 MDG 有 15+ 个小任务时：(1) 按优先级分批，每批 3-4 个任务 (2) 每批写一个 `/tmp/mdgXXX-batchN.md` 任务文件 (3) dispatch → 等完成 → 检查编译 → 修复错误 → commit → 下一批 (4) 每批只包含具体操作（文件路径+修改前后代码+验证命令），不包含背景故事 (5) `--max-turns 20` 对大多数批次足够。MDG-017 用这个模式在 5 轮 dispatch 中完成了 15 个任务

83. **MDG 完成标准：P0+P1 即可宣布核心完成** — 大型 MDG（如全项目审计修复）可能有 20+ 个问题。P0+P1 全部完成后即可宣布核心完成，P2 标记为"待后续 MDG"。不要试图一次性做完所有 P2/P3 — 那些是持续改进，不是阻塞性问题。用户说"做完"时，理解为"P0+P1 做完"而非"所有 45 个问题都做完"

84. **用户等 Claude Code 时要主动汇报** — 当 Claude Code 跑超过 30 秒时，用户会问"你还在工作吗？怎么感觉你卡住了"。用 `process(action='wait')` 等待时，60 秒超时限制意味着需要多次 wait。在每次 wait 之间应该告诉用户当前进展（"Claude Code 在执行 Phase X，预计还需要 Y 分钟"），不要默默 polling
85. **Claude Code 可能达到 max-turns 但零输出** — pitfall #72 说 Claude Code 留下编译错误。但更糟的情况是 Claude Code 达到 max-turns 退出后 `git diff --stat HEAD` 显示零改动——它消耗了所有 turns 在"思考"而没有实际修改代码。MDG-018 中 Claude Code 用 20 turns 做了 0 个文件改动。**验证流程必须检查 git diff**，不能只看 Claude Code 的自述输出。如果零输出，直接用 `patch` 工具手动作修复（对于简单改动如 `||` → `??`）
86. **JavaScript `||` vs `??` 空字符串陷阱** — `||` 把 `''`（空字符串）当 falsy，`??` 只把 `null`/`undefined` 当 nullish。当用户明确清空字段（设为 `''`）时，`||` 会错误触发 fallback。案例：`file.header || '证据1'` 在 `file.header = ''` 时返回 `'证据1'` 而非 `''`。**规则：用户可编辑的文本字段一律用 `??` 不用 `||`**。MDG-018 修了 4 个测试回归，根因全是这个问题
87. **重构 API 时保留 legacy fallback 路径** — 当从旧 API（如 rules-based）重构到新 API（如 group-based）时，新 API 的入口函数必须处理"旧调用方没有 group"的情况。不能假设所有调用方都已迁移到新 API。案例：`buildHeaderText` 依赖 `selectedGroupFor` 返回 group，但测试文件通过旧 API 传入 `rules.headerText`，没有 group。**规则：新 API 入口函数必须 fallback 到旧 API 的解析逻辑**，用 `if (!group) { /* legacy path */ }` 包裹
88. **简单修复不要 dispatch Claude Code** — 当修复只有 1-3 行改动（如 `||` → `??`、参数重命名）时，直接用 `patch` 工具完成。dispatch Claude Code 的开销（启动 + 读代码 + max-turns 消耗）远大于直接 patch。**判断标准：如果任务描述不超过 3 行，直接 patch；超过 3 行或涉及多文件协调，才 dispatch Claude Code**
89. **六步流程不允许跳步** — 用户批评"你没有完整运行MDG workflow的6步"。即使改动很小（如 `||` → `??`），也必须完成全部六步：①定位（精确到文件+行号+函数名）→ ②影响分析（`grep -rn` 搜索全部调用方，不能只看当前文件）→ ③记录变更单（更新实施记录，不只是初始版本）→ ④修改 → ⑤验证（测试 + `#[allow(dead_code)]` 扫描 + 编译警告检查）→ ⑥提交。**常见跳步**：(a) "改动很小不用分析" — 错，任何改动都需要搜索调用方 (b) "测试通过就行" — 错，还要检查编译警告和 dead_code (c) "变更单以后再更新" — 错，实施后立即更新变更单记录实际修改
90. **Tauri 命令错误类型变更必须检查前端兼容性** — 将 `Result<T, String>` 改为 `Result<T, DocsyError>` 时，前端 `tauriCallSafe` 的 `catch(err)` 收到的不再是字符串而是序列化的 JSON 对象 `{ kind, message, reason }`。`String(err)` 会变成 `"[object Object]"`。必须同步修改前端错误处理：(1) 检查 `typeof err === 'object'` 并提取 `err.message || err.reason` (2) 如果 message 是 JSON 字符串（来自 `anyhow_to_json_string`），`JSON.parse` 后提取 `message` 字段。MDG-018 发现此问题并修复了 `tauriBridge.js`
91. **"双轨机制"不一定是技术债** — 发现两个类似机制时（如 OperationManager + SubprocessRegistry），先分析各自服务的场景再判断是否需要统一。MDG-018 中 GLM-A4 标记"取消机制双轨"为问题，但分析后发现 OperationManager（CancellationToken）和 SubprocessRegistry（PID kill）服务于不同场景：前者是 Rust 异步任务，后者是外部子进程。两者互补而非重复。**正确做法：修正误导性注释，确认架构合理，而不是强行合并**
92. **CSS 硬编码颜色迁移需要多轮扫描** — 第一轮替换后必须 `grep -rn 'color:.*#[0-9a-fA-F]' src/ --include='*.vue' --include='*.css' | grep -v 'var(--' | grep -v 'styles.css'` 全量扫描。MDG-018 第一轮替换了 15 处，第二轮发现还有 13 处遗漏（分布在 TemplateRenderTab、VideoExtractView、HeaderFooterRuleFields、PdfToolsView 等文件）。同时注意区分 CSS 颜色和 JS 配置值（如 `color: '#000000'` 传给后端渲染 PDF 的默认值不是 CSS 颜色，不要改）
93. **不要以"大规模重构"为由推迟任务** — 用户批评"大规模重构就重构啊，一直在往后推"。判断标准不是代码量大小，而是：(1) 是否有清晰的设计文档 (2) 是否有中间检查点 (3) 是否按六步流程走。如果三个条件满足，再大的重构也可以做。MDG-018 在一个 session 内完成了 ImagePaddlerView 拆分（1060 行）、video-extract 进度+取消重写（FFmpeg 子进程管理）、DocsyError 全覆盖（30+ 函数）、28 处硬编码颜色迁移——全部按六步流程执行
94. **FFmpeg 子进程进度+取消的实现模式** — 当需要用 CancellationToken 取消外部子进程时：(1) `cmd.stdout(Stdio::piped()).stderr(Stdio::piped())` 捕获输出 (2) 单独线程 `BufReader::new(stderr).lines()` 逐行读取 (3) 主循环 `rx.try_recv()` 非阻塞接收 + `token.is_cancelled()` 检查 (4) 取消时 `child.kill()` + 清理临时文件 (5) 返回 `{ cancelled: true, count: 0 }` 部分结果。复用已有 `parse_time_text()` 解析 FFmpeg 的 `time=HH:MM:SS.ss` 进度格式。MDG-018 Phase 3b 实现
95. **Rust `loop`-as-expression 消除 unused variable 警告** — 当变量只在循环内赋值、循环后使用时，`let mut x = None; loop { x = Some(val); break; }` 会产生 "value assigned to `x` is never read" 警告（初始 `None` 从未被读取）。重构为 `let x = loop { break val; };`，变量绑定直接捕获循环返回值，消除 mutable + unused 初始值。适用于 FFmpeg 子进程 exit status、retry 循环最终结果等场景
96. **Git 删除文件前必须检查残留引用** — `git status` 显示 `D`（deleted）时，不能直接 `git add` 提交。必须先 `grep -rn "import.*FileName\|from.*FileName\|use.*FileName" src/` 检查是否还有文件引用被删文件。案例：EvidencePdfWorkbench.vue 被从工作树删除（非 commit），但 EvidencePdfView.vue 仍 `import` 它。提交删除会破坏编译。**恢复命令**：`git checkout -- <path>`。此检查对 `.vue`、`.rs`、`.js` 模块文件尤为重要——模块注册表可能自动发现文件，但显式 import 不会自动更新 — 当需要用 CancellationToken 取消外部子进程时：(1) `cmd.stdout(Stdio::piped()).stderr(Stdio::piped())` 捕获输出 (2) 单独线程 `BufReader::new(stderr).lines()` 逐行读取 (3) 主循环 `rx.try_recv()` 非阻塞接收 + `token.is_cancelled()` 检查 (4) 取消时 `child.kill()` + 清理临时文件 (5) 返回 `{ cancelled: true, count: 0 }` 部分结果。复用已有 `parse_time_text()` 解析 FFmpeg 的 `time=HH:MM:SS.ss` 进度格式。MDG-018 Phase 3b 实现
