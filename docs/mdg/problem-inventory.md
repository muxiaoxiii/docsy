# MDG 问题总表

> 更新日期：2026-08-07
> 来源：bug-report.md + deep-review-*.md 合并去重

---

## 分组原则

**可一并解决的问题归为同一个 MDG**，减少分支数量和重复工作。分组依据：
- 代码区域重叠的归一组（如同一个文件的多个 bug）
- 修改相互依赖的归一组（如 cancel_operation 前后端必须同时改）
- 纯清理工作归一组（如删除 eprintln + 删除死代码）

---

## MDG-001：操作管理层重构（第四通用层）🔴 P0

**本质问题**：缺少操作生命周期管理层。前端 operationId 和后端 PID 是两套体系，SubprocessRegistry 几乎是空的，异步任务没有取消通道，超时和无响应被混为一谈。

**设计方案 v2**：建立第四通用层 `run_managed`（与 `run_blocking` 同级），自动注册/注销操作，统一管理异步任务和外部子进程的取消。不设固定超时自动 kill，只支持用户主动取消。

**合并的问题：**
| 原编号 | 问题 | 文件 |
|--------|------|------|
| BUG-001 | `cancel_operation` 忽略 operation_id，杀死所有进程 | `commands/system.rs:138` |
| BUG-017 | `cancelCurrentOperation` 清除所有 pendingOperations | `App.vue:178` |
| BUG-003 | `ConversionState::wait_for_user_response` 忙等待 | `lib.rs:158-180` |
| 架构问题 | SubprocessRegistry 只有 qpdf.rs 一个调用点 | `pdf/qpdf.rs:12` |
| 架构问题 | 异步任务无法取消 | 多处 |
| 架构问题 | 超时和无响应混为一谈（大文件被误杀） | `external/mod.rs:121` |
| STYLE-001 | `SeqCst` 内存序过度使用 | `lib.rs:132-154` |

**变更单**：[MDG-001-cancel-operation.md](changes/MDG-001-cancel-operation.md)（34KB，含完整架构设计）

---

## MDG-002：Mutex 安全模式 🔴 P0

**本质问题**：`Mutex::unwrap()` 在中毒时 panic。

**合并的问题：**
| 原编号 | 问题 | 文件 |
|--------|------|------|
| BUG-002 | `MANIFEST_CACHE` 使用 `Mutex::unwrap()` | `commands/template.rs:16,32` |

**变更单**：[MDG-002-mutex-unwrap.md](changes/MDG-002-mutex-unwrap.md)

---

## MDG-003：代码重复消除 🟠 P1

**本质问题**：3 组函数在多处重复定义，行为微妙不同。

**合并的问题：**
| 原编号 | 问题 | 文件 | 重复次数 |
|--------|------|------|----------|
| BUG-004 | `same_path` 路径比较函数 | `annotations.rs` / `artifacts.rs` / `evidence_session.rs` | 3 |
| BUG-005 | `temp_named_path` 临时文件路径 | `preview.rs` / `annotations.rs` / `split.rs` / `normalize.rs` | 4+ |
| BUG-008 | `fnv1a_hash` 哈希函数 | `docx_template/mod.rs` / `pdf/evidence.rs` | 2 |
| 前端重复 | `ensureExtension` / `splitPartyLabelSegments` / `effectiveFieldType` / `fieldFormKey` | `TemplateView.vue` / `fieldRowUtils.js` / `TemplateRenderTab.vue` | 2-3 |

**变更单**：[MDG-003-dedup.md](changes/MDG-003-dedup.md)

---

## MDG-004：书签 + 路径安全 🔴 P0

**本质问题**：书签操作和输出路径管理存在数据安全风险。

**合并的问题：**
| 原编号 | 问题 | 文件 |
|--------|------|------|
| deep-review | 书签页码偏移错位 | `evidence_session.rs:295` |
| deep-review | 书签写入非原子操作，可能丢失原文件 | `header_footer.rs:324` |
| deep-review | 输出路径碰撞导致静默覆盖 | `header_footer.rs:726` |
| BUG-013/018 | `import_template_to_library` 静默覆盖已有模板 | `commands/template.rs:295-306` |

**变更单**：[MDG-004-bookmark-path-safety.md](changes/MDG-004-bookmark-path-safety.md)

---

## MDG-005：生产代码清理 🟠 P1

**本质问题**：调试代码残留和无用变量。

**合并的问题：**
| 原编号 | 问题 | 文件 |
|--------|------|------|
| BUG-006 | `eprintln!` 调试输出 | `header_footer.rs:238-244` |
| BUG-010 | `batch_overlay` 首项 debug 输出 | `header_footer.rs:238-244`（同上） |
| BUG-007 | `mime` 变量计算后被忽略 | `commands/system.rs:67-88` |
| STYLE-008 | `all_descriptors` dead code | `services/module_registry.rs:3` |
| STYLE-009 | `#[allow(dead_code)]` 多处 | `docx_template/index.rs` |

**变更单**：[MDG-005-prod-cleanup.md](changes/MDG-005-prod-cleanup.md)

---

## MDG-006：模板系统健壮性 🟡 P2

**合并的问题：**
| 原编号 | 问题 | 文件 |
|--------|------|------|
| BUG-011 | 损坏模板静默跳过 | `docx_template/mod.rs:383` |
| BUG-012 | label sanitization 误替换 | `docx_template/mod.rs:299-321` |
| BUG-013 | import_template 覆盖已有模板 | `commands/template.rs:295-306` |
| deep-review | `batch.rs:586` build_row_values 重复调用 | `docx_template/batch.rs:586` |

**变更单**：[MDG-006-template-robustness.md](changes/MDG-006-template-robustness.md)

---

## MDG-007：防复制模块修正 🟡 P2

| 原编号 | 问题 | 文件 |
|--------|------|------|
| BUG-009 | `TextOverlay` 方法名误导（实际做 CmapScramble） | `anti_ocr.rs:125-141` |
| deep-review | PUA 篡改映射空间仅 4096 值可暴力逆向 | `anti_ocr.rs:358` |

**变更单**：[MDG-007-anti-copy.md](changes/MDG-007-anti-copy.md)

---

## MDG-008：资源管理 + 文件名 🟡 P2

| 原编号 | 问题 | 文件 |
|--------|------|------|
| BUG-014 | 临时文件泄漏（无 RAII guard） | `header_footer.rs:291-311` |
| deep-review | `services/history.rs` 文件名误导（实际是 AppSettings） | `services/history.rs` |
| deep-review | `useWindowFileDrop` 的 `onEnter` 命名不精确 | `useWindowFileDrop.js:12` |

**变更单**：[MDG-008-resource-cleanup.md](changes/MDG-008-resource-cleanup.md)

---

## MDG-009：前后端默认值统一 🟠 P1

| 原编号 | 问题 | 文件 |
|--------|------|------|
| deep-review | `margin_mm` 默认值 15.0 vs 前端推荐 12.0 | `image_paddler.rs:522` |
| deep-review | `filename_without_ext` 前后端默认值不同 | `image_paddler.rs:525` |
| deep-review | `list_system_fonts` 不递归扫描，Linux 遗漏字体 | `detect.rs:4-31` |
| deep-review | `managed.rs` macOS/Linux sha256 永远为空 | `managed.rs:212-229` |

**变更单**：[MDG-009-defaults-sync.md](changes/MDG-009-defaults-sync.md)

---

## MDG-012：Word 域（Field）启发式检测与保护 🔴 P0

**本质问题**：docx_template 模块对 Word 域零感知。scan 碰巧正确，save 把域代码区 run 也包裹 SDT，render 给无 `w:t` 的 run 注入文本。

**设计方案**：启发式检测——FORMCHECKBOX 自动推断为已有 checkbox 类型（复用 `checkboxLike` 路径），不引入新字段类型。域深度跟踪保护域结构：scan 识别域边界，save 跳过域代码区，render 不注入 `w:t`。用户可在 UI 里看到并修改。四阶段实施：域边界感知 → 域结构保护 → 渲染保护 → 前端自动推断。

**变更单**：[MDG-012-formcheckbox-field-structure.md](changes/MDG-012-formcheckbox-field-structure.md)

---

## MDG-013：模板驱动文件名生成 🟠 P1

**本质问题**：批量生成文件时无法根据字段值自动拼接文件名。

**设计方案**：模板 manifest 新增 `filenameTemplate` 字段，用 `{字段名}` 语法引用。含特殊变量（日期、序号、原文件名）、非法字符处理、冲突处理。

**变更单**：[MDG-013-template-filename.md](changes/MDG-013-template-filename.md)

---

## MDG-014：统一文档预览模块 + 编辑 UI 优化 🟠 P1

**本质问题**：预览逻辑分散在多处，补选提取不到文本（渲染预览的 `<button>` 没有 `data-run-id`），底部图例冗余。

**设计方案**：提取通用 `DocumentPreview` 组件，接受 `runs[]` + `overlays[]`，支持三种模式（原文/字段标注/填写）。所有 overlay span 统一带 `data-run-id`，选区解析统一处理。底部图例精简为一个"添加为字段"按钮（默认文本，自动推断 checkbox/date）。其他模块（证据 PDF、文书对比）可复用。

**变更单**：[MDG-014-template-ui-redesign.md](changes/MDG-014-template-ui-redesign.md)

| MDG | 优先级 | 问题数 | 复杂度 |
|-----|--------|--------|--------|
| MDG-001 | P0 | 6 | 🔴 高（架构重构） |
| MDG-002 | P0 | 1 | 🟢 低（简单替换） |
| MDG-003 | P1 | 4 组 | 🟡 中（提取公共函数） |
| MDG-004 | P0 | 4 | 🟡 中（文件操作安全） |
| MDG-005 | P1 | 5 | 🟢 低（删除/替换） |
| MDG-006 | P2 | 4 | 🟡 中（逻辑修改） |
| MDG-007 | P2 | 2 | 🟡 中（需决策） |
| MDG-008 | P2 | 3 | 🟢 低（重命名+RAII） |
| MDG-009 | P1 | 4 | 🟢 低（值替换） |
| MDG-012 | P0 | 1 | 🔴 高（域机制重构） |
| MDG-013 | P1 | 1 | 🟡 中（新增功能） |
| MDG-014 | P1 | 3 | 🟡 中（组件提取 + 选区修复） |
