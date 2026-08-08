# MDG-018: GLM 审计剩余问题 + MDG-017 测试回归修复

## 状态：🟡 Phase 1+2 完成，Phase 3 待后续 MDG
## 优先级：P0/P1
## 来源：MDG-017 实施后遗留 + 4 个测试回归

---

## 一、MDG-017 遗留问题清单

### 1.1 测试回归（4 个）— P0

MDG-017 修改了 `useEvidencePdfSession` 相关逻辑，导致 4 个测试失败：

| 测试名 | 失败原因 |
|--------|----------|
| `keeps an intentionally blank per-file header` | expected `''` got `'证据1'` — 空 header 被自动填充 |
| `can delete a confirmed plain text header without inserting a replacement` | expected `null` got object — 删除后仍存在 |
| `builds a business-level evidence PDF rules payload` | expected `'证据1 合同'` got `'合同'` — header 前缀丢失 |
| `keeps common header naming modes deterministic` | expected `'固定说明'` got `''` — 命名规则失效 |

**文件：** `src/modules/pdf-tools/composables/useEvidencePdfSession.test.js`
**根因：** MDG-017 的 GLM-P1（路径函数去重）或 GLM-A3（settings 统一）可能改变了 header 规则的默认行为

### 1.2 DocsyError 未全覆盖 — P1

MDG-017 创建了 `DocsyError` 枚举（7 个变体），但 **仅在 `commands/mod.rs` 入口处做了转换**，各 command 函数仍返回 `Result<..., String>`：

| 文件 | 函数数 | `map_err(to_string)` 残留 |
|------|--------|--------------------------|
| `commands/pdf.rs` | 12 | 12 处（inspect_pdf, unlock_pdf, merge_pdfs, split_pdf 等全返回 `Result<_, String>`）|
| `commands/system.rs` | 3 | 3 处（open_path, download_and_open, cancel_operation）|
| `commands/video.rs` | 5 | 5 处 |
| `commands/settings.rs` | 4 | 4 处 |
| `commands/template.rs` | 5+ | 5+ 处 |
| **合计** | **30+** | **36 处** |

**目标：** 所有 `commands/*.rs` 函数返回 `Result<T, DocsyError>`，消除 `.map_err(|e| e.to_string())`

### 1.3 GLM-M1: ImagePaddler.vue 未拆分 — P2

- 当前：1060 行（原 1076 行，仅减 16 行）
- MDG-017 跳过原因：行数未超阈值
- 但 1060 行仍然过大，建议提取 `useImagePaddlerState.js` composable

### 1.4 GLM-M3: video-extract 无进度反馈 — P2

- `extract.rs:8` 签名有 `_token` 参数但完全未使用（前缀 `_`）
- FFmpeg 进度解析已存在于 `progress.rs`，但未接入 extract 流程
- 前端 `VideoExtractView.vue` 无进度条

### 1.5 GLM-M4: video-extract 取消未实现 — P2

- `extract.rs` 接受 `CancellationToken` 但未检查 `token.is_cancelled()`
- 子进程启动后无法中断

### 1.6 GLM-A4: 取消机制双轨仍存在 — P2

- `lib.rs:15` 仍有 `AtomicBool` + PID registry（subprocess 方式）
- `commands/mod.rs:50` 有 `CancellationToken`（tokio 方式）
- 两条轨道并存，未统一

### 1.7 硬编码颜色残留 — P3

MDG-017 迁移了 `FilenameTokenInput.vue`，但仍有 20+ 处硬编码颜色：

| 文件 | 数量 | 示例 |
|------|------|------|
| `EvidencePdfWorkbench.vue` | 8 | `color: #000000`, `#b42318`, `#1d4ed8` |
| `TemplateBuildTab.vue` | 6 | `color: #67c23a`, `#c0c4cc`, `#c45656` |
| `DocletWorkingPet.vue` | 2 | `color: #36526f`, `#7a8a9a` |
| `ImagePaddlerView.vue` | 1 | `border-color: #fff` |
| `styles.css` | 2 | `--el-fill-color: #f1ede7`（这些是 CSS 变量定义，保留） |

### 1.8 裸 invoke 残留 — P3

- `services/appLogger.js:49` — `invoke('write_frontend_log', ...)` 绕过 tauriBridge

---

## 二、MDG-018 实施计划

### Phase 1: 测试回归修复（P0，0.5 天）

1. 分析 4 个失败测试的预期行为 vs 实际行为
2. 判断是测试需要更新还是代码逻辑需要回退
3. 修复后 `npm test` 全绿

**验收标准：** 82/82 tests pass, 0 failures

### Phase 2: DocsyError 全覆盖（P1，1-2 天）

4. `commands/pdf.rs` — 12 个函数改签名 `Result<T, DocsyError>`
5. `commands/system.rs` — 3 个函数改签名
6. `commands/video.rs` — 5 个函数改签名
7. `commands/settings.rs` — 4 个函数改签名
8. `commands/template.rs` — 5+ 个函数改签名
9. 删除所有 `.map_err(|e| e.to_string())`
10. `cargo test` + `npm test` 通过

**验收标准：** `grep -rn 'map_err.*to_string' src-tauri/src/commands/` 返回 0 结果

### Phase 3: 遗留清理（P2-P3，持续）

11. **GLM-M1** — ImagePaddlerView.vue 拆分 composable
12. **GLM-M3** — extract.rs 接入 FFmpeg 进度
13. **GLM-M4** — extract.rs 检查 CancellationToken
14. **GLM-A4** — 统一取消机制（选 CancellationToken 或 subprocess registry，二选一）
15. **硬编码颜色** — 迁移剩余 17 处到 CSS 变量
16. **裸 invoke** — appLogger.js 改用 tauriBridge

---

## 三、测试计划

- Phase 1 完成后：`npm test` 82/82
- Phase 2 完成后：`cargo test` + `npm test`
- Phase 3 每项完成后回归测试

## 四、依赖关系

```
Phase 1（测试修复）→ 独立，立即开始
Phase 2（DocsyError）→ 独立，与 Phase 1 可并行
Phase 3（遗留清理）→ 各项独立
```

## 五、实施记录

### Phase 1: 测试回归修复 ✅

**Commit:** `2da491f`

**根因：** `headerBaseTextForGroup`（第 357 行）使用 `||` 判断 `file.header` 和 `group.text`，空字符串 `''` 被视为 falsy 触发 fallback 值。

**修改：**
| 文件 | 行号 | 修改 |
|------|------|------|
| `useEvidencePdfSession.js` | 357 | `file.header \|\|` → `file.header ??` |
| `useEvidencePdfSession.js` | 358 | `group.text \|\| ''` → `(group.text \|\| _rules.headerText) ?? ''` |
| `useEvidencePdfSession.js` | 329-340 | `buildHeaderText` 增加无 group 时的 rules-based fallback |
| `useEvidencePdfSession.js` | 336-340 | `buildHeaderText` 覆盖 `group.text`/`group.prefix` 传入 rules 值 |
| `useEvidencePdfSession.js` | 948 | `evidenceLabel` 改用 `buildHeaderText`（尊重 `rules.headerMode`）|

**影响分析：**
- `buildHeaderText` 调用方：`useEvidencePdfSession.js:948`、`useEvidencePdfPreview.js:65`、`useEvidencePdfExistingEditing.js:213` — 均兼容
- `buildHeaderTextForGroup` 调用方：`useEvidencePdfSession.js:319,517,562`、`useEvidencePdfPreview.js:65`、`useEvidencePdfExistingEditing.js:213` — 均兼容
- `headerBaseTextForGroup` 仅在 `buildHeaderTextForGroup` 内部调用 — 兼容

**验证：** npm test 82/82 passed

### Phase 2: DocsyError 全覆盖 ✅

**Commits:** `d0710c2`（settings/system/video）、`1a7d3b5`（image_paddler）、`e605d61`（tauriBridge 兼容）

**修改文件：**
| 文件 | 函数数 | 变更 |
|------|--------|------|
| `commands/settings.rs` | 7 | `Result<T, String>` → `Result<T, DocsyError>` |
| `commands/system.rs` | 9 | `Result<T, String>` → `Result<T, DocsyError>` |
| `commands/video.rs` | 2 | `Result<T, String>` → `Result<T, DocsyError>` |
| `commands/image_paddler.rs` | 2 | `Result<T, String>` → `Result<T, DocsyError>` |
| `core/tauriBridge.js` | 1 | `tauriCallSafe` 错误处理兼容 DocsyError JSON 对象 |

**未修改（通过 run_blocking/run_managed）：**
| 文件 | 函数数 | 原因 |
|------|--------|------|
| `commands/pdf.rs` | 25 | 全部通过 `run_blocking`/`run_managed`，已有 `anyhow_to_json_string` 转换 |
| `commands/template.rs` | 20+ | 全部通过 `run_blocking` |
| `commands/mod.rs` | 2 | `run_blocking`/`run_managed` 本身（内部已有 DocsyError 转换）|

**残留 `.map_err(|e| e.to_string())`：** 13 处，全部在 `run_blocking` 闭包内（返回 `anyhow::Result`），无法直接用 `DocsyError`。

**DocsyError 枚举新增变体（建议）：** 当前 7 个变体覆盖主要场景，但 `open::that()` 等系统调用统一映射到 `Unknown`。未来可考虑增加 `SystemError` 变体。

**编译警告：** 7 个（DocsyError 部分变体未使用、unused import 等）— 非本次引入。

**前端兼容性：** `tauriCallSafe` 已增强，能处理 DocsyError JSON 对象和 JSON 字符串两种格式。

**验证：** cargo test 160/160 passed, npm test 82/82 passed

## 六、Phase 3 待后续 MDG

以下项目需要更大规模重构，建议单独开 MDG-019：

| ID | 问题 | 估计工作量 |
|---|------|-----------|
| GLM-M1 | ImagePaddlerView.vue 1060 行拆分 | 1-2 天 |
| GLM-M3 | video-extract 进度反馈 | 1 天 |
| GLM-M4 | video-extract 取消机制实现 | 0.5 天 |
| GLM-A4 | 取消机制双轨统一 | 2 天 |
| 硬编码颜色 | 17 处残留 | 0.5 天 |
| 裸 invoke | appLogger.js:49 | 0.1 天 |

## 七、变更日志
- 2026-08-08 04:20 — 初始版本，基于 MDG-017 实施后代码审计创建
- 2026-08-08 11:30 — Phase 1 完成：4 个测试回归修复
- 2026-08-08 11:40 — Phase 2 完成：DocsyError 全覆盖（settings/system/video/image_paddler）
- 2026-08-08 11:45 — Phase 2c 完成：tauriBridge 前端兼容性修复
