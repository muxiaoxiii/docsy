# MDG-018: GLM 审计剩余问题 + MDG-017 测试回归修复

## 状态：📋 待实施
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

## 五、变更日志
- 2026-08-08 — 基于 MDG-017 实施后代码审计创建
